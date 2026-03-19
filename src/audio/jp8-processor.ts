/**
 * JP-8 AudioWorklet Processor.
 * Follows POC10 pattern: WASM engine runs entirely inside the worklet.
 * SAB for parameter transport + telemetry. postMessage for notes only.
 */

import init, {
  init_engine,
  render_block,
  note_on,
  note_off,
  all_notes_off,
  apply_params,
  get_active_voice_count,
} from '@jp8-wasm/jp8_wasm.js';

const BLOCK_FRAMES = 128;
const CHANNELS = 2;
const PARAM_COUNT = 32;

// SAB telemetry layout
const SAB_ACTIVE_VOICES = 0;
const SAB_SLOTS = 4;

type Command =
  | { type: 'note-on'; note: number; velocity: number }
  | { type: 'note-off'; note: number }
  | { type: 'all-notes-off' };

class JP8Processor extends AudioWorkletProcessor {
  private engineReady = false;
  private pendingCommands: Command[] = [];
  private blockBuffer: Float32Array;
  private sabInt32: Int32Array | null = null;
  private paramView: Float32Array | null = null;
  private paramSnapshot: Float32Array;

  constructor(options: AudioWorkletProcessorOptions) {
    super();

    const { wasmModule, telemetrySab, paramSab } = options.processorOptions as {
      wasmModule: WebAssembly.Module;
      telemetrySab: SharedArrayBuffer;
      paramSab: SharedArrayBuffer;
    };

    this.blockBuffer = new Float32Array(BLOCK_FRAMES * CHANNELS);
    this.paramSnapshot = new Float32Array(PARAM_COUNT);

    if (telemetrySab) {
      this.sabInt32 = new Int32Array(telemetrySab, 0, SAB_SLOTS);
    }
    if (paramSab) {
      this.paramView = new Float32Array(paramSab);
    }

    this.initEngine(wasmModule);

    this.port.onmessage = (event: MessageEvent) => {
      const cmd = event.data as Command;
      if (!this.engineReady) {
        this.pendingCommands.push(cmd);
        return;
      }
      this.handleCommand(cmd);
    };
  }

  private async initEngine(wasmModule: WebAssembly.Module): Promise<void> {
    try {
      await init({ module_or_path: wasmModule });
      init_engine(sampleRate);
      this.engineReady = true;

      for (const cmd of this.pendingCommands) {
        this.handleCommand(cmd);
      }
      this.pendingCommands = [];

      this.port.postMessage({ type: 'ready', sampleRate, blockFrames: BLOCK_FRAMES });
    } catch (err) {
      this.port.postMessage({ type: 'error', message: String(err) });
    }
  }

  private handleCommand(cmd: Command): void {
    switch (cmd.type) {
      case 'note-on':
        note_on(cmd.note, cmd.velocity);
        break;
      case 'note-off':
        note_off(cmd.note);
        break;
      case 'all-notes-off':
        all_notes_off();
        break;
    }
  }

  process(_inputs: Float32Array[][], outputs: Float32Array[][]): boolean {
    if (!this.engineReady) {
      const out = outputs[0];
      if (out) for (const ch of out) ch.fill(0);
      return true;
    }

    // Read params from SAB and apply to engine
    if (this.paramView) {
      this.paramSnapshot.set(this.paramView);
      apply_params(this.paramSnapshot);
    }

    // Render interleaved stereo
    render_block(this.blockBuffer);

    // Deinterleave to output channels
    const out = outputs[0];
    if (out && out.length >= CHANNELS) {
      const left = out[0];
      const right = out[1];
      for (let i = 0; i < BLOCK_FRAMES; i++) {
        left[i] = this.blockBuffer[i * CHANNELS];
        right[i] = this.blockBuffer[i * CHANNELS + 1];
      }
    }

    // Telemetry to SAB
    if (this.sabInt32) {
      Atomics.store(this.sabInt32, SAB_ACTIVE_VOICES, get_active_voice_count());
    }

    return true;
  }
}

registerProcessor('jp8-processor', JP8Processor);
