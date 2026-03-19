/**
 * JP-8 Rack — Multi-instance orchestrator.
 *
 * Manages N JP-8 engines with per-instance volume/pan/mute/solo.
 *
 * Modes:
 *   single-context: all WorkletNodes on one AudioContext (default, simpler)
 *   multi-context:  each instance gets its own AudioContext + OS thread,
 *                   routed via MediaStreamDestination to master context
 */

import { JP8Engine, type JP8EngineStatus } from './jp8-engine';

export type RackMode = 'single-context' | 'multi-context';

const MAX_INSTANCES_SINGLE = 8;
const MAX_INSTANCES_MULTI = 6; // Chrome AudioContext limit

export interface ChannelConfig {
  volume: number;   // 0–1
  pan: number;      // -1 to 1
  mute: boolean;
  solo: boolean;
  midiChannel: number; // 0 = omni, 1–16 = specific
}

export interface RackInstance {
  id: number;
  name: string;
  engine: JP8Engine;
  channel: ChannelConfig;
}

interface MixerChannel {
  gainNode: GainNode;
  panNode: StereoPannerNode;
  // Multi-context only:
  ownContext?: AudioContext;
  mediaStreamDest?: MediaStreamAudioDestinationNode;
  sourceNode?: MediaStreamAudioSourceNode;
}

export type RackStatusCallback = (status: JP8EngineStatus) => void;

export class JP8Rack {
  private mode: RackMode;
  private masterCtx: AudioContext | null = null;
  private masterGain: GainNode | null = null;
  private instances: Map<number, RackInstance> = new Map();
  private mixerChannels: Map<number, MixerChannel> = new Map();
  private nextId = 1;
  private activeId = 0;
  private status: JP8EngineStatus = 'idle';
  private onStatusChange: RackStatusCallback | null = null;

  constructor(mode: RackMode = 'single-context') {
    this.mode = mode;
  }

  setStatusCallback(cb: RackStatusCallback) { this.onStatusChange = cb; }
  getStatus() { return this.status; }
  private setStatus(s: JP8EngineStatus) { this.status = s; this.onStatusChange?.(s); }

  getInstances(): RackInstance[] {
    return Array.from(this.instances.values());
  }

  getActiveInstance(): RackInstance | undefined {
    return this.instances.get(this.activeId);
  }

  getActiveId() { return this.activeId; }

  setActiveId(id: number) {
    if (this.instances.has(id)) {
      this.activeId = id;
    }
  }

  getMaxInstances(): number {
    return this.mode === 'multi-context' ? MAX_INSTANCES_MULTI : MAX_INSTANCES_SINGLE;
  }

  /**
   * Start the rack — creates the master AudioContext and initializes
   * all existing instances.
   */
  async start(): Promise<void> {
    this.setStatus('loading');

    try {
      this.masterCtx = new AudioContext({ sampleRate: 44100, latencyHint: 'interactive' });
      this.masterGain = this.masterCtx.createGain();
      this.masterGain.connect(this.masterCtx.destination);

      // Start all existing instances
      for (const inst of this.instances.values()) {
        await this.startInstance(inst);
      }

      await this.masterCtx.resume();
      this.setStatus('ready');
    } catch (err) {
      console.error('JP8 Rack start failed:', err);
      this.setStatus('error');
    }
  }

  /**
   * Add a new instance to the rack. If rack is already started,
   * the instance is started immediately.
   */
  async addInstance(name?: string): Promise<RackInstance> {
    const max = this.getMaxInstances();
    if (this.instances.size >= max) {
      throw new Error(`Maximum ${max} instances in ${this.mode} mode`);
    }

    const id = this.nextId++;
    const inst: RackInstance = {
      id,
      name: name ?? `JP-8 #${id}`,
      engine: new JP8Engine(),
      channel: { volume: 0.7, pan: 0, mute: false, solo: false, midiChannel: 0 },
    };

    this.instances.set(id, inst);
    if (this.instances.size === 1) {
      this.activeId = id;
    }

    // If rack is already running, start this instance now
    if (this.masterCtx && this.status === 'ready') {
      await this.startInstance(inst);
    }

    return inst;
  }

  async removeInstance(id: number): Promise<void> {
    const inst = this.instances.get(id);
    if (!inst) return;

    // Disconnect mixer chain
    const ch = this.mixerChannels.get(id);
    if (ch) {
      ch.gainNode.disconnect();
      ch.panNode.disconnect();
      ch.sourceNode?.disconnect();
      if (ch.ownContext) {
        await ch.ownContext.close();
      }
      this.mixerChannels.delete(id);
    }

    await inst.engine.stop();
    this.instances.delete(id);

    // Move active to another instance
    if (this.activeId === id) {
      const remaining = this.instances.keys().next();
      this.activeId = remaining.done ? 0 : remaining.value;
    }
  }

  private async startInstance(inst: RackInstance): Promise<void> {
    if (!this.masterCtx || !this.masterGain) return;

    if (this.mode === 'single-context') {
      // All engines on the master context
      await inst.engine.start(this.masterCtx);

      const gainNode = this.masterCtx.createGain();
      gainNode.gain.value = inst.channel.volume;

      const panNode = this.masterCtx.createStereoPanner();
      panNode.pan.value = inst.channel.pan;

      const audioNode = inst.engine.getAudioNode();
      if (audioNode) {
        audioNode.connect(gainNode);
        gainNode.connect(panNode);
        panNode.connect(this.masterGain);
      }

      this.mixerChannels.set(inst.id, { gainNode, panNode });

    } else {
      // Multi-context: each engine gets its own AudioContext
      const ownContext = new AudioContext({ sampleRate: 44100, latencyHint: 'interactive' });
      await inst.engine.start(ownContext);

      // Capture output via MediaStreamDestination
      const audioNode = inst.engine.getAudioNode();
      const mediaStreamDest = ownContext.createMediaStreamDestination();
      if (audioNode) {
        audioNode.connect(mediaStreamDest);
      }

      await ownContext.resume();

      // Feed into master context via MediaStreamAudioSourceNode
      const sourceNode = this.masterCtx.createMediaStreamSource(mediaStreamDest.stream);
      const gainNode = this.masterCtx.createGain();
      gainNode.gain.value = inst.channel.volume;
      const panNode = this.masterCtx.createStereoPanner();
      panNode.pan.value = inst.channel.pan;

      sourceNode.connect(gainNode);
      gainNode.connect(panNode);
      panNode.connect(this.masterGain);

      this.mixerChannels.set(inst.id, { gainNode, panNode, ownContext, mediaStreamDest, sourceNode });
    }

    this.applySoloLogic();
  }

  // --- Mixer controls ---

  setChannelVolume(id: number, vol: number) {
    const inst = this.instances.get(id);
    const ch = this.mixerChannels.get(id);
    if (inst) inst.channel.volume = vol;
    if (ch) ch.gainNode.gain.value = inst?.channel.mute ? 0 : vol;
  }

  setChannelPan(id: number, pan: number) {
    const inst = this.instances.get(id);
    const ch = this.mixerChannels.get(id);
    if (inst) inst.channel.pan = pan;
    if (ch) ch.panNode.pan.value = pan;
  }

  setChannelMute(id: number, muted: boolean) {
    const inst = this.instances.get(id);
    if (inst) inst.channel.mute = muted;
    this.applySoloLogic();
  }

  setChannelSolo(id: number, solo: boolean) {
    const inst = this.instances.get(id);
    if (inst) inst.channel.solo = solo;
    this.applySoloLogic();
  }

  setChannelMidiChannel(id: number, ch: number) {
    const inst = this.instances.get(id);
    if (inst) inst.channel.midiChannel = ch;
  }

  /** If any channel is solo'd, mute all non-solo'd. Otherwise respect individual mutes. */
  private applySoloLogic() {
    const anySolo = Array.from(this.instances.values()).some(i => i.channel.solo);

    for (const [id, inst] of this.instances) {
      const ch = this.mixerChannels.get(id);
      if (!ch) continue;

      let audible: boolean;
      if (anySolo) {
        audible = inst.channel.solo;
      } else {
        audible = !inst.channel.mute;
      }

      ch.gainNode.gain.value = audible ? inst.channel.volume : 0;
    }
  }

  // --- MIDI routing ---

  /**
   * Route a MIDI note-on to the appropriate instance(s).
   * midiChannel 0 (OMNI) = active instance only.
   * Specific channel (1-16) = always receives that channel regardless of focus.
   */
  routeNoteOn(midiChannel: number, note: number, velocity: number) {
    for (const inst of this.instances.values()) {
      if (this.shouldReceiveMidi(inst, midiChannel)) {
        inst.engine.noteOn(note, velocity);
      }
    }
  }

  routeNoteOff(midiChannel: number, note: number) {
    for (const inst of this.instances.values()) {
      if (this.shouldReceiveMidi(inst, midiChannel)) {
        inst.engine.noteOff(note);
      }
    }
  }

  routeCC(midiChannel: number, cc: number, value: number) {
    for (const inst of this.instances.values()) {
      if (this.shouldReceiveMidi(inst, midiChannel)) {
        const normalized = value / 127;
        switch (cc) {
          case 1: inst.engine.setParam(28, normalized); break;
          case 7: inst.engine.setParam(33, normalized); break;
          case 74: inst.engine.setParam(12, 20 + normalized * 19980); break;
          case 71: inst.engine.setParam(13, normalized); break;
          case 123: inst.engine.allNotesOff(); break;
        }
      }
    }
  }

  /** OMNI (0) = only when this instance is active. 1-16 = always for that channel. */
  private shouldReceiveMidi(inst: RackInstance, incomingChannel: number): boolean {
    if (inst.channel.midiChannel === 0) {
      return inst.id === this.activeId;
    }
    return inst.channel.midiChannel === incomingChannel + 1;
  }

  // --- Lifecycle ---

  async stop() {
    for (const inst of this.instances.values()) {
      inst.engine.allNotesOff();
    }
    for (const [id] of this.mixerChannels) {
      const ch = this.mixerChannels.get(id)!;
      ch.gainNode.disconnect();
      ch.panNode.disconnect();
      ch.sourceNode?.disconnect();
      if (ch.ownContext) await ch.ownContext.close();
    }
    this.mixerChannels.clear();

    if (this.masterCtx) {
      await this.masterCtx.close();
      this.masterCtx = null;
      this.masterGain = null;
    }
    this.setStatus('idle');
  }
}
