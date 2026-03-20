/**
 * JP-8 AudioWorklet Processor.
 *
 * Multiple processor instances share one WASM module (same AudioContext).
 * Each gets a unique engineId → separate WASM engine, output buffer, param buffer.
 * Zero-copy rendering via Float32Array views into WASM linear memory.
 */

import init, {
  create_engine,
  render,
  get_output_ptr,
  get_output_len,
  get_param_ptr,
  get_wavetable_ptr,
  store_wavetable,
  debug_wavetable_peak,
  debug_wavetable_len,
  apply_params_from_buf,
  note_on,
  note_off,
  all_notes_off,
  get_active_voice_count,
} from '@jp8-wasm/jp8_wasm.js';

let wasmMemory: WebAssembly.Memory;
let wasmReady = false;
let wasmInitPromise: Promise<void> | null = null;
let nextEngineId = 0;

const BLOCK_FRAMES = 128;
const CHANNELS = 2;
const PARAM_COUNT = 68;
const SAB_ACTIVE_VOICES = 0;
const SAB_SLOTS = 4;

type Command =
  | { type: 'note-on'; note: number; velocity: number }
  | { type: 'note-off'; note: number }
  | { type: 'all-notes-off' }
  | { type: 'upload-wavetable'; excIdx: number; bodyIdx: number; data: Float32Array };

/** Ensure WASM is initialized exactly once across all processor instances. */
function ensureWasmInit(wasmModule: WebAssembly.Module): Promise<void> {
  if (wasmReady) return Promise.resolve();
  if (wasmInitPromise) return wasmInitPromise;

  wasmInitPromise = (async () => {
    const wasm = await init({ module_or_path: wasmModule });
    wasmMemory = wasm.memory;
    wasmReady = true;
  })();

  return wasmInitPromise;
}

class JP8Processor extends AudioWorkletProcessor {
  private engineReady = false;
  private engineId: number;
  private pendingCommands: Command[] = [];
  private outputView: Float32Array | null = null;
  private paramWasmView: Float32Array | null = null;
  private sabInt32: Int32Array | null = null;
  private paramSabView: Float32Array | null = null;

  constructor(options: AudioWorkletProcessorOptions) {
    super();

    this.engineId = nextEngineId++;

    const { wasmModule, telemetrySab, paramSab } = options.processorOptions as {
      wasmModule: WebAssembly.Module;
      telemetrySab: SharedArrayBuffer;
      paramSab: SharedArrayBuffer;
    };

    if (telemetrySab) this.sabInt32 = new Int32Array(telemetrySab, 0, SAB_SLOTS);
    if (paramSab) this.paramSabView = new Float32Array(paramSab);

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
      await ensureWasmInit(wasmModule);

      // Create this instance's engine (wavetables uploaded later from JS)
      create_engine(this.engineId, sampleRate);

      // Get pointers to this engine's pre-allocated buffers
      const outputPtr = get_output_ptr(this.engineId) as unknown as number;
      const outputLen = get_output_len();
      this.outputView = new Float32Array(wasmMemory.buffer, outputPtr, outputLen);

      const paramPtr = get_param_ptr(this.engineId) as unknown as number;
      this.paramWasmView = new Float32Array(wasmMemory.buffer, paramPtr, PARAM_COUNT);

      this.engineReady = true;

      for (const cmd of this.pendingCommands) {
        this.handleCommand(cmd);
      }
      this.pendingCommands = [];

      this.port.postMessage({ type: 'ready', sampleRate, blockFrames: BLOCK_FRAMES, engineId: this.engineId });
    } catch (err) {
      this.port.postMessage({ type: 'error', message: String(err) });
    }
  }

  private handleCommand(cmd: Command): void {
    switch (cmd.type) {
      case 'note-on':
        note_on(this.engineId, cmd.note, cmd.velocity);
        break;
      case 'note-off':
        note_off(this.engineId, cmd.note);
        break;
      case 'all-notes-off':
        all_notes_off(this.engineId);
        break;
      case 'upload-wavetable': {
        // Copy Float32Array data into WASM memory, then store in engine cache
        const ptr = get_wavetable_ptr(this.engineId) as unknown as number;
        if (ptr && wasmMemory) {
          // Must create fresh view — WASM memory may have grown
          const view = new Float32Array(wasmMemory.buffer, ptr, cmd.data.length);
          view.set(cmd.data);
          store_wavetable(this.engineId, cmd.excIdx, cmd.bodyIdx, cmd.data.length);
          // Verify round-trip: check what Rust actually stored
          const rustPeak = debug_wavetable_peak(this.engineId, cmd.excIdx, cmd.bodyIdx);
          const rustLen = debug_wavetable_len(this.engineId, cmd.excIdx, cmd.bodyIdx);
          const jsPeak = cmd.data.reduce((max: number, s: number) => Math.max(max, Math.abs(s)), 0);
          console.log(`[JP8] Uploaded exc=${cmd.excIdx} body=${cmd.bodyIdx} jsLen=${cmd.data.length} jsPeak=${jsPeak.toFixed(4)} rustLen=${rustLen} rustPeak=${rustPeak.toFixed(4)}`);
        } else {
          console.warn(`[JP8] Failed to upload wavetable: ptr=${ptr} memory=${!!wasmMemory}`);
        }
        break;
      }
    }
  }

  process(_inputs: Float32Array[][], outputs: Float32Array[][]): boolean {
    if (!this.engineReady || !this.outputView || !this.paramWasmView) {
      const out = outputs[0];
      if (out) for (const ch of out) ch.fill(0);
      return true;
    }

    // Refresh views if WASM memory grew
    if (this.outputView.buffer !== wasmMemory.buffer) {
      const outputPtr = get_output_ptr(this.engineId) as unknown as number;
      this.outputView = new Float32Array(wasmMemory.buffer, outputPtr, get_output_len());
      const paramPtr = get_param_ptr(this.engineId) as unknown as number;
      this.paramWasmView = new Float32Array(wasmMemory.buffer, paramPtr, PARAM_COUNT);
    }

    // SAB params → WASM param buffer → apply
    if (this.paramSabView) {
      this.paramWasmView.set(this.paramSabView);
      apply_params_from_buf(this.engineId);
    }

    // Render this engine's block (zero copy — writes to engine's own buffer)
    render(this.engineId);

    // Deinterleave from WASM memory to Web Audio output
    const out = outputs[0];
    if (out && out.length >= CHANNELS) {
      const left = out[0];
      const right = out[1];
      const buf = this.outputView;
      for (let i = 0; i < BLOCK_FRAMES; i++) {
        left[i] = buf[i * CHANNELS];
        right[i] = buf[i * CHANNELS + 1];
      }
    }

    if (this.sabInt32) {
      Atomics.store(this.sabInt32, SAB_ACTIVE_VOICES, get_active_voice_count(this.engineId));
    }

    return true;
  }
}

registerProcessor('jp8-processor', JP8Processor);
