/**
 * JP-8 Engine — Single synth instance.
 * Can own its AudioContext or use an external one (for rack mode).
 * SAB for params (40 × f32), postMessage for notes.
 */

import { loadJP8Wasm } from './wasm-loader';

export type JP8EngineStatus = 'idle' | 'loading' | 'ready' | 'error';

export const PARAM_COUNT = 68;
const TELEMETRY_SAB_BYTES = 16;
const SAB_ACTIVE_VOICES = 0;

type StatusCallback = (status: JP8EngineStatus) => void;

export class JP8Engine {
  private audioCtx: AudioContext | null = null;
  private ownsContext = false; // true if we created the context ourselves
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

  /**
   * Start the engine. If externalCtx is provided, uses it instead of
   * creating a new AudioContext (for rack mode where multiple engines
   * share one context or each gets their own).
   */
  async start(externalCtx?: AudioContext): Promise<void> {
    if (this.status === 'ready') return;
    if (this.initPromise) return this.initPromise;
    this.initPromise = this._start(externalCtx);
    return this.initPromise;
  }

  private async _start(externalCtx?: AudioContext): Promise<void> {
    this.setStatus('loading');
    try {
      const wasmModule = await loadJP8Wasm();

      if (externalCtx) {
        this.audioCtx = externalCtx;
        this.ownsContext = false;
      } else {
        this.audioCtx = new AudioContext({ sampleRate: 44100, latencyHint: 'interactive' });
        this.ownsContext = true;
      }

      this.paramSab = new SharedArrayBuffer(PARAM_COUNT * 4);
      this.paramView = new Float32Array(this.paramSab);
      this.telemetrySab = new SharedArrayBuffer(TELEMETRY_SAB_BYTES);
      this.telemetryInt32 = new Int32Array(this.telemetrySab, 0, 4);

      this.writeDefaults();

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

      // Don't auto-connect to destination — caller (rack or standalone) handles routing
      if (this.ownsContext) {
        this.workletNode.connect(this.audioCtx.destination);
      }

      await new Promise<void>((resolve, reject) => {
        const timeout = setTimeout(() => reject(new Error('JP8 init timeout')), 5000);
        this.workletNode!.port.onmessage = (ev: MessageEvent) => {
          if (ev.data.type === 'ready') { clearTimeout(timeout); resolve(); }
          else if (ev.data.type === 'error') { clearTimeout(timeout); reject(new Error(ev.data.message)); }
        };
      });

      if (this.ownsContext) {
        await this.audioCtx.resume();
      }

      this.setStatus('ready');

      // Pre-convolve waveguide wavetables in background (non-blocking)
      this.loadWaveguideWavetables();
    } catch (err) {
      console.error('JP8 start failed:', err);
      this.setStatus('error');
    }
  }

  private async loadWaveguideWavetables(): Promise<void> {
    try {
      const { loadWaveguideSamples, getConvolvedWavetable, EXCITATION_COUNT, BODY_COUNT } = await import('./waveguide-loader');
      const presets = await loadWaveguideSamples();

      for (let exc = 0; exc < EXCITATION_COUNT; exc++) {
        for (let body = 0; body < BODY_COUNT; body++) {
          const wavetable = await getConvolvedWavetable(presets, exc, body);
          this.workletNode?.port.postMessage({
            type: 'upload-wavetable',
            excIdx: exc,
            bodyIdx: body,
            data: wavetable,
          });
        }
      }
    } catch (err) {
      console.warn('Waveguide wavetable loading failed:', err);
    }
  }

  private writeDefaults() {
    const p = this.paramView!;
    p[P.VCO1_WAVE] = 1;
    p[P.VCO1_RANGE] = 0;
    p[P.VCO1_PW] = 0.5;
    p[P.VCO1_LEVEL] = 0.8;
    p[P.VCO2_WAVE] = 1;
    p[P.VCO2_RANGE] = 0;
    p[P.VCO2_PW] = 0.5;
    p[P.VCO2_LEVEL] = 0.8;
    p[P.VCO2_DETUNE] = 0;
    p[P.CROSS_MOD] = 0;
    p[P.NOISE] = 0;
    p[P.SUB_OSC] = 0;
    p[P.FILTER_CUTOFF] = 8000;
    p[P.FILTER_RESO] = 0;
    p[P.FILTER_ENV] = 0.5;
    p[P.FILTER_KEY] = 0.5;
    p[P.HPF_CUTOFF] = 20;
    p[P.ENV1_A] = 0.01;
    p[P.ENV1_D] = 0.3;
    p[P.ENV1_S] = 0.6;
    p[P.ENV1_R] = 0.5;
    p[P.ENV1_VCA] = 0;
    p[P.ENV2_A] = 0.01;
    p[P.ENV2_D] = 0.3;
    p[P.ENV2_S] = 0.7;
    p[P.ENV2_R] = 0.5;
    p[P.LFO_RATE] = 5;
    p[P.LFO_WAVE] = 0;
    p[P.LFO_PITCH] = 0;
    p[P.LFO_FILTER] = 0;
    p[P.LFO_PWM] = 0;
    p[P.LFO_DELAY] = 0;
    p[P.CHORUS] = 0;  // off by default — enable per patch
    p[P.VOLUME] = 0.7;
    p[P.ASSIGN] = 0;
    p[P.PORTAMENTO] = 0;
    p[P.ARP_MODE] = 0;
    p[P.ARP_RANGE] = 1;
    p[P.ARP_TEMPO] = 120;
    // Extended modules — all off/bypass by default
    p[P.SOURCE_MODE] = 0;
    p[P.SPECTRAL_TILT] = 0;
    p[P.SPECTRAL_PARTIALS] = 32;
    p[P.SPECTRAL_NOISE] = 0;
    p[P.SPECTRAL_MORPH] = 0;
    p[P.SPECTRAL_TARGET] = 0;
    p[P.WG_EXCITATION] = 0;
    p[P.WG_BODY] = 0;
    p[P.WG_BRIGHTNESS] = 0.5;
    p[P.WG_BODY_MIX] = 0.5;
    p[P.MODAL_MIX] = 0;          // 0 = full bypass
    p[P.MODAL_MATERIAL] = 0.5;
    p[P.MODAL_BODY] = 0;
    p[P.MODAL_MODES] = 16;
    p[P.MODAL_INHARMONICITY] = 0;
    p[P.CHAOS_ENABLE] = 0;
    p[P.CHAOS_RATE1] = 5;
    p[P.CHAOS_RATE2] = 7;
    p[P.CHAOS_DEPTH] = 0;
    p[P.CHAOS_TO_PITCH] = 0;
    p[P.CHAOS_TO_FILTER] = 0;
    p[P.CHAOS_TO_PWM] = 0;
    p[P.BUBBLE_ENABLE] = 0;
    p[P.BUBBLE_RATE] = 5;
    p[P.BUBBLE_MIN_SIZE] = 0.003;
    p[P.BUBBLE_MAX_SIZE] = 0.01;
    p[P.BUBBLE_LEVEL] = 0;
  }

  // --- Parameter access ---

  setParam(index: number, value: number) {
    if (this.paramView && index >= 0 && index < PARAM_COUNT) {
      this.paramView[index] = value;
    }
  }

  /** Read current params from SAB. Used when switching tabs to restore UI state. */
  getParams(): number[] {
    if (!this.paramView) return new Array(PARAM_COUNT).fill(0);
    return Array.from(this.paramView);
  }

  /** Get the AudioWorkletNode for connecting to external mixer routing. */
  getAudioNode(): AudioNode | null {
    return this.workletNode;
  }

  // --- Notes ---

  noteOn(note: number, velocity = 100) {
    this.workletNode?.port.postMessage({ type: 'note-on', note, velocity });
  }
  noteOff(note: number) {
    this.workletNode?.port.postMessage({ type: 'note-off', note });
  }
  allNotesOff() {
    this.workletNode?.port.postMessage({ type: 'all-notes-off' });
  }

  // --- Telemetry ---

  getActiveVoices(): number {
    if (!this.telemetryInt32) return 0;
    return Atomics.load(this.telemetryInt32, SAB_ACTIVE_VOICES);
  }

  // --- Lifecycle ---

  async stop() {
    this.allNotesOff();
    this.workletNode?.disconnect();
    if (this.ownsContext) {
      await this.audioCtx?.close();
    }
    this.audioCtx = null;
    this.workletNode = null;
    this.setStatus('idle');
    this.initPromise = null;
  }
}

// SAB parameter indices — matches engine.rs apply_params order
export const P = {
  VCO1_WAVE: 0, VCO1_RANGE: 1, VCO1_PW: 2, VCO1_LEVEL: 3,
  VCO2_WAVE: 4, VCO2_RANGE: 5, VCO2_PW: 6, VCO2_LEVEL: 7,
  VCO2_DETUNE: 8, CROSS_MOD: 9, NOISE: 10, SUB_OSC: 11,
  FILTER_CUTOFF: 12, FILTER_RESO: 13, FILTER_ENV: 14, FILTER_KEY: 15,
  HPF_CUTOFF: 16,
  ENV1_A: 17, ENV1_D: 18, ENV1_S: 19, ENV1_R: 20, ENV1_VCA: 21,
  ENV2_A: 22, ENV2_D: 23, ENV2_S: 24, ENV2_R: 25,
  LFO_RATE: 26, LFO_WAVE: 27, LFO_PITCH: 28, LFO_FILTER: 29,
  LFO_PWM: 30, LFO_DELAY: 31,
  CHORUS: 32, VOLUME: 33, ASSIGN: 34, PORTAMENTO: 35,
  ARP_MODE: 36, ARP_RANGE: 37, ARP_TEMPO: 38,
  // Extended synthesis modules
  SOURCE_MODE: 39,        // 0=BLEP, 1=SPECTRAL, 2=WAVEGUIDE
  SPECTRAL_TILT: 40,      // -1..+1
  SPECTRAL_PARTIALS: 41,  // 2-64
  SPECTRAL_NOISE: 42,     // 0-1
  SPECTRAL_MORPH: 43,     // 0-1
  SPECTRAL_TARGET: 44,    // 0-N preset
  WG_EXCITATION: 45,      // 0-5
  WG_BODY: 46,            // 0-4
  WG_BRIGHTNESS: 47,      // 0-1
  WG_BODY_MIX: 48,        // 0-1
  MODAL_MIX: 49,          // 0-1 (0=bypass)
  MODAL_MATERIAL: 50,     // 0-1 (rubber..metal)
  MODAL_BODY: 51,         // 0-4 preset
  MODAL_MODES: 52,        // 4-32
  MODAL_INHARMONICITY: 53,// 0-1
  CHAOS_ENABLE: 54,       // 0/1
  CHAOS_RATE1: 55,        // 0.1-30 Hz
  CHAOS_RATE2: 56,        // 0.1-30 Hz
  CHAOS_DEPTH: 57,        // 0-1
  CHAOS_TO_PITCH: 58,     // 0-1
  CHAOS_TO_FILTER: 59,    // 0-1
  CHAOS_TO_PWM: 60,       // 0-1
  BUBBLE_ENABLE: 61,      // 0/1
  BUBBLE_RATE: 62,        // 0-60
  BUBBLE_MIN_SIZE: 63,    // 0.001-0.01
  BUBBLE_MAX_SIZE: 64,    // 0.005-0.03
  BUBBLE_LEVEL: 65,       // 0-1
} as const;

export const PARAM_TOTAL = PARAM_COUNT;
