/**
 * JP-8 Engine — Main thread facade.
 * SAB for params (UI writes, audio reads) and telemetry (audio writes, UI reads).
 * postMessage only for notes.
 */

import { loadJP8Wasm } from './wasm-loader';

export type JP8EngineStatus = 'idle' | 'loading' | 'ready' | 'error';

const PARAM_COUNT = 32;
const TELEMETRY_SAB_BYTES = 16;
const SAB_ACTIVE_VOICES = 0;

type StatusCallback = (status: JP8EngineStatus) => void;

export class JP8Engine {
  private audioCtx: AudioContext | null = null;
  private workletNode: AudioWorkletNode | null = null;
  private paramSab: SharedArrayBuffer | null = null;
  private paramView: Float32Array | null = null;
  private telemetrySab: SharedArrayBuffer | null = null;
  private telemetryInt32: Int32Array | null = null;
  private status: JP8EngineStatus = 'idle';
  private onStatusChange: StatusCallback | null = null;
  private initPromise: Promise<void> | null = null;

  setStatusCallback(cb: StatusCallback) { this.onStatusChange = cb; }
  getStatus() { return this.status; }
  private setStatus(s: JP8EngineStatus) { this.status = s; this.onStatusChange?.(s); }

  async start(): Promise<void> {
    if (this.status === 'ready') return;
    if (this.initPromise) return this.initPromise;
    this.initPromise = this._start();
    return this.initPromise;
  }

  private async _start(): Promise<void> {
    this.setStatus('loading');
    try {
      const wasmModule = await loadJP8Wasm();
      this.audioCtx = new AudioContext({ sampleRate: 44100, latencyHint: 'interactive' });

      // SABs
      this.paramSab = new SharedArrayBuffer(PARAM_COUNT * 4);
      this.paramView = new Float32Array(this.paramSab);
      this.telemetrySab = new SharedArrayBuffer(TELEMETRY_SAB_BYTES);
      this.telemetryInt32 = new Int32Array(this.telemetrySab, 0, 4);

      // Write default params
      this.writeDefaults();

      // Load processor (Vite transforms .ts)
      const processorUrl = new URL('./jp8-processor.ts', import.meta.url).href;
      await this.audioCtx.audioWorklet.addModule(processorUrl);

      this.workletNode = new AudioWorkletNode(this.audioCtx, 'jp8-processor', {
        numberOfOutputs: 1,
        outputChannelCount: [2],
        processorOptions: {
          wasmModule,
          paramSab: this.paramSab,
          telemetrySab: this.telemetrySab,
        },
      });
      this.workletNode.connect(this.audioCtx.destination);

      await new Promise<void>((resolve, reject) => {
        const timeout = setTimeout(() => reject(new Error('JP8 init timeout')), 5000);
        this.workletNode!.port.onmessage = (ev: MessageEvent) => {
          if (ev.data.type === 'ready') { clearTimeout(timeout); resolve(); }
          else if (ev.data.type === 'error') { clearTimeout(timeout); reject(new Error(ev.data.message)); }
        };
      });

      this.setStatus('ready');
    } catch (err) {
      console.error('JP8 start failed:', err);
      this.setStatus('error');
    }
  }

  private writeDefaults() {
    const p = this.paramView!;
    // Matches spec §5.1 defaults
    p[0] = 0;     // VCO1 Waveform (Saw)
    p[1] = 0;     // VCO1 Range
    p[2] = 0.5;   // VCO1 PW
    p[3] = 0.8;   // VCO1 Level
    p[4] = 0;     // VCO2 Waveform
    p[5] = 0;     // VCO2 Range
    p[6] = 0.5;   // VCO2 PW
    p[7] = 0.8;   // VCO2 Level
    p[8] = 0;     // VCO2 Detune
    p[9] = 0;     // Cross Mod
    p[10] = 0;    // Noise
    p[11] = 8000; // Filter Cutoff
    p[12] = 0;    // Resonance
    p[13] = 0.5;  // Filter Env Depth
    p[14] = 0.5;  // Key Track
    p[15] = 0.01; // Env1 Attack
    p[16] = 0.3;  // Env1 Decay
    p[17] = 0.6;  // Env1 Sustain
    p[18] = 0.5;  // Env1 Release
    p[19] = 0.01; // Env2 Attack
    p[20] = 0.3;  // Env2 Decay
    p[21] = 0.7;  // Env2 Sustain
    p[22] = 0.5;  // Env2 Release
    p[23] = 5.0;  // LFO Rate
    p[24] = 0;    // LFO Waveform
    p[25] = 0;    // LFO Pitch
    p[26] = 0;    // LFO Filter
    p[27] = 0;    // LFO PWM
    p[28] = 3;    // Chorus (I+II)
    p[29] = 0.7;  // Master Volume
    p[30] = 0;    // Assign (Poly8)
    p[31] = 0;    // Portamento
  }

  /** Set a parameter by spec index (§5.1). Lock-free write to SAB. */
  setParam(index: number, value: number) {
    if (this.paramView && index >= 0 && index < PARAM_COUNT) {
      this.paramView[index] = value;
    }
  }

  // --- Notes (postMessage) ---
  noteOn(note: number, velocity = 100) {
    this.workletNode?.port.postMessage({ type: 'note-on', note, velocity });
  }
  noteOff(note: number) {
    this.workletNode?.port.postMessage({ type: 'note-off', note });
  }
  allNotesOff() {
    this.workletNode?.port.postMessage({ type: 'all-notes-off' });
  }

  getActiveVoices(): number {
    if (!this.telemetryInt32) return 0;
    return Atomics.load(this.telemetryInt32, SAB_ACTIVE_VOICES);
  }

  async stop() {
    this.allNotesOff();
    this.workletNode?.disconnect();
    await this.audioCtx?.close();
    this.audioCtx = null;
    this.workletNode = null;
    this.setStatus('idle');
    this.initPromise = null;
  }
}

// Spec §5.1 parameter indices — exported for UI binding
export const P = {
  VCO1_WAVE: 0, VCO1_RANGE: 1, VCO1_PW: 2, VCO1_LEVEL: 3,
  VCO2_WAVE: 4, VCO2_RANGE: 5, VCO2_PW: 6, VCO2_LEVEL: 7,
  VCO2_DETUNE: 8, CROSS_MOD: 9, NOISE: 10,
  FILTER_CUTOFF: 11, FILTER_RESO: 12, FILTER_ENV: 13, FILTER_KEY: 14,
  ENV1_A: 15, ENV1_D: 16, ENV1_S: 17, ENV1_R: 18,
  ENV2_A: 19, ENV2_D: 20, ENV2_S: 21, ENV2_R: 22,
  LFO_RATE: 23, LFO_WAVE: 24, LFO_PITCH: 25, LFO_FILTER: 26, LFO_PWM: 27,
  CHORUS: 28, VOLUME: 29, ASSIGN: 30, PORTAMENTO: 31,
} as const;
