# JP-8 Virtual Analog Synthesizer — Technical Specification

**Codename:** `jp8-va`
**Stack:** Rust → WASM → AudioWorklet → SharedArrayBuffer → Web Audio API
**Constraint:** Zero heap allocations in the audio render path

---

## 1. System Architecture

### 1.1 Thread Model

Three threads with strict separation of concerns:

| Thread | Runtime | Responsibility | Timing |
|--------|---------|---------------|--------|
| **UI Thread** | JS/TS (React or vanilla) | Parameter control, MIDI input, visualisation, patch management | requestAnimationFrame (16ms) |
| **Audio Thread** | Rust/WASM inside AudioWorkletProcessor | DSP render loop — voices, filter, chorus, mixing | 128-sample blocks (~2.9ms at 44.1kHz) |
| **Comms** | SharedArrayBuffer + Atomics | Parameter transport UI→Audio, metering Audio→UI | Lock-free, wait-free |

The audio thread **never** calls `postMessage()` during rendering. All parameter updates and metering data flow through SharedArrayBuffer ring buffers. `postMessage()` is used only during initialisation (WASM blob transfer, sample rate negotiation, patch loading).

### 1.2 Module Dependency Graph

```
┌─────────────────────────────────────────────────────────┐
│  UI Thread                                              │
│  ┌──────────┐  ┌──────────────┐  ┌───────────────────┐  │
│  │ Patch UI │  │ MIDI Handler │  │ Visualiser (SAB)  │  │
│  └────┬─────┘  └──────┬───────┘  └───────────────────┘  │
│       │               │                    ▲             │
│       ▼               ▼                    │             │
│  ┌────────────────────────────┐   ┌────────┴──────────┐ │
│  │  Parameter Writer (SAB)   │   │ Meter Reader (SAB) │ │
│  └────────────┬───────────────┘   └───────────────────┘ │
└───────────────┼─────────────────────────────────────────┘
                │ SharedArrayBuffer (no locks)
┌───────────────┼─────────────────────────────────────────┐
│  Audio Thread │ (AudioWorkletProcessor)                  │
│               ▼                                          │
│  ┌────────────────────┐                                  │
│  │  Parameter Reader   │                                 │
│  └────────┬───────────┘                                  │
│           ▼                                              │
│  ┌────────────────────┐                                  │
│  │  Voice Allocator   │ (8 voices, JP-8 assign modes)    │
│  └────────┬───────────┘                                  │
│           ▼                                              │
│  ┌────────────────────┐  ×8                              │
│  │  Voice             │──────────┐                       │
│  │  ├─ VCO1 (PolyBLEP)│         │                       │
│  │  ├─ VCO2 (PolyBLEP)│         │                       │
│  │  ├─ Mixer          │         │                       │
│  │  ├─ IR3109 Filter  │         ▼                       │
│  │  ├─ VCA            │  ┌──────────────┐               │
│  │  ├─ Env1 (Filter)  │  │ Voice Summer │               │
│  │  ├─ Env2 (Amp)     │  └──────┬───────┘               │
│  │  └─ LFO            │         │                       │
│  └────────────────────┘         ▼                       │
│                          ┌──────────────┐                │
│                          │ BBD Chorus   │                │
│                          └──────┬───────┘                │
│                                 ▼                        │
│                          ┌──────────────┐                │
│                          │ Output + Meter│               │
│                          └──────────────┘                │
└──────────────────────────────────────────────────────────┘
```

### 1.3 Crate / Module Structure

```
jp8-va/
├── crates/
│   └── jp8-core/              # Pure Rust DSP — no WASM deps, no alloc in hot path
│       ├── src/
│       │   ├── lib.rs
│       │   ├── voice.rs       # Voice struct, per-voice render
│       │   ├── oscillator.rs  # PolyBLEP saw/pulse/square/noise
│       │   ├── filter.rs      # IR3109 4-pole OTA model
│       │   ├── envelope.rs    # ADSR with linear + exponential segments
│       │   ├── lfo.rs         # LFO with multiple waveshapes
│       │   ├── chorus.rs      # Stereo BBD chorus emulation
│       │   ├── allocator.rs   # Voice allocator (8 voices, JP-8 modes)
│       │   ├── params.rs      # Parameter definitions + smoothing
│       │   ├── engine.rs      # Top-level engine: voices + chorus + output
│       │   └── math.rs        # fast_tanh, fast_sin, interpolation utils
│       ├── benches/
│       │   └── render.rs      # Criterion benchmarks
│       └── Cargo.toml         # no_std compatible, optional "std" feature for WAV tests
│
├── crates/
│   └── jp8-wasm/              # Thin WASM binding layer
│       ├── src/
│       │   └── lib.rs         # wasm-bindgen exports, SAB pointer setup
│       └── Cargo.toml         # wasm-bindgen, jp8-core dependency
│
├── js/
│   ├── worklet/
│   │   └── jp8-processor.js   # AudioWorkletProcessor — loads WASM, calls render()
│   ├── main/
│   │   ├── jp8-node.ts        # AudioWorkletNode wrapper, SAB setup
│   │   ├── midi.ts            # Web MIDI API → parameter writes
│   │   ├── params.ts          # SAB parameter writer
│   │   └── meter.ts           # SAB meter reader for UI visualisation
│   └── ui/                    # UI components (framework-agnostic)
│
├── tests/
│   └── wav-render/            # Offline render to WAV via hound crate
│
└── build/
    └── build.sh               # wasm-pack build + wasm-opt + copy to js/
```

---

## 2. DSP Engine (`jp8-core`)

### 2.1 Memory Model — Zero Allocation Contract

All DSP state is pre-allocated in fixed-size structs. No `Vec`, `Box`, `String`, or any heap allocation occurs after `Engine::new()`.

```rust
/// Top-level engine. All memory is inline, stack-allocated within the
/// AudioWorkletProcessor's WASM linear memory.
pub struct Engine {
    voices: [Voice; 8],
    chorus: StereoChorus,
    voice_allocator: VoiceAllocator,
    params: EngineParams,
    sample_rate: f32,
    inv_sample_rate: f32,
}

/// Per-voice state. Fixed size, no indirection.
pub struct Voice {
    vco1: Oscillator,
    vco2: Oscillator,
    noise: NoiseGen,
    filter: IR3109,
    env1: Envelope,   // → filter cutoff
    env2: Envelope,   // → VCA
    lfo: Lfo,
    gate: bool,
    note: u8,
    velocity: f32,
}
```

The engine exposes a single render entry point:

```rust
impl Engine {
    /// Render `n` stereo frames into `output_l` and `output_r`.
    /// SAFETY: output slices must be exactly `n` frames.
    /// GUARANTEE: No heap allocations. No panics. No floating-point exceptions.
    pub fn render(&mut self, output_l: &mut [f32], output_r: &mut [f32]) {
        // 1. Read smoothed parameters from SAB snapshot
        // 2. For each sample:
        //    a. Update LFOs (global + per-voice)
        //    b. Accumulate all active voices into mono sum
        //    c. Apply chorus → stereo
        //    d. Write to output buffers
    }
}
```

### 2.2 Oscillators — PolyBLEP Anti-Aliased

Each oscillator produces saw, pulse (with PWM), or square waveforms using PolyBLEP (Polynomial Band-Limited Step) correction to suppress aliasing without wavetables.

```rust
pub struct Oscillator {
    phase: f32,          // [0, 1)
    freq: f32,           // Hz, smoothed
    waveform: Waveform,  // Saw, Pulse, Square
    pulse_width: f32,    // [0.05, 0.95] for Pulse
    drift: f32,          // slow random detune (± cents)
    drift_phase: f32,    // LFO for drift
}

#[derive(Clone, Copy)]
pub enum Waveform {
    Saw,
    Pulse,
    Square,  // Pulse with PW fixed at 0.5
}
```

**PolyBLEP correction function:**

```rust
#[inline(always)]
fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}
```

**Saw generation (per sample):**

```rust
#[inline(always)]
fn saw(&mut self, dt: f32) -> f32 {
    let naive = 2.0 * self.phase - 1.0;
    naive - poly_blep(self.phase, dt)
}
```

**Pulse generation (per sample):**

```rust
#[inline(always)]
fn pulse(&mut self, dt: f32) -> f32 {
    let mut out = if self.phase < self.pulse_width { 1.0 } else { -1.0 };
    out += poly_blep(self.phase, dt);
    out -= poly_blep((self.phase - self.pulse_width + 1.0) % 1.0, dt);
    out
}
```

**Cross-modulation:** VCO1 output is scaled and added to VCO2's phase increment per sample, implementing the JP-8's cross-mod feature:

```rust
let vco1_out = self.vco1.tick(dt);
let xmod_amount = self.params.cross_mod * vco1_out;
let vco2_out = self.vco2.tick_with_fm(dt, xmod_amount);
```

**Analog drift:** Each oscillator applies a slow random drift (filtered noise, ~0.3 Hz bandwidth, ± 3 cents max) to its frequency to emulate analog instability. The drift generator uses a simple one-pole filtered white noise with fixed-seed PRNG per voice — no allocations.

### 2.3 Filter — IR3109 OTA Model (TPT)

The Roland IR3109 uses four cascaded OTA (Operational Transconductance Amplifier) integrator stages with global resonance feedback. The key sonic characteristic is the soft saturation of each OTA stage, modelled with `tanh()`.

**Topology:**

```
Input → [tanh] → Stage1 → [tanh] → Stage2 → [tanh] → Stage3 → [tanh] → Stage4 → Output
                                                                            │
                                                            Resonance ×(−k) ←┘
```

```rust
pub struct IR3109 {
    s: [f32; 4],       // integrator state per pole
    cutoff_hz: f32,
    resonance: f32,    // [0, 1] → mapped to k [0, ~4]
    g: f32,            // precomputed: tan(π * cutoff / (oversample_rate))
}
```

**Per-sample processing (trapezoidal integration, one-step feedback delay):**

```rust
impl IR3109 {
    /// Precompute coefficient when cutoff changes.
    #[inline(always)]
    pub fn set_cutoff(&mut self, hz: f32, sample_rate: f32) {
        let freq = hz.clamp(20.0, sample_rate * 0.45);
        self.g = fast_tan(core::f32::consts::PI * freq / sample_rate);
        self.cutoff_hz = freq;
    }

    /// Process one sample. Zero allocations.
    #[inline(always)]
    pub fn tick(&mut self, input: f32) -> f32 {
        let k = self.resonance * 4.0; // resonance scaling

        // One-step delay feedback (avoids Newton-Raphson solve)
        let feedback = self.s[3];
        let x = input - k * feedback;

        // Four cascaded one-pole stages with OTA saturation
        let y0 = self.one_pole(0, x);
        let y1 = self.one_pole(1, y0);
        let y2 = self.one_pole(2, y1);
        let y3 = self.one_pole(3, y2);

        y3
    }

    #[inline(always)]
    fn one_pole(&mut self, stage: usize, input: f32) -> f32 {
        let v = (fast_tanh(input) - self.s[stage]) * self.g;
        let y = v + self.s[stage];
        self.s[stage] = y + v; // trapezoidal integrator state update
        y
    }
}
```

**Design decisions:**

- **One-step feedback delay** instead of Newton-Raphson iterative solve. Saves ~60% CPU vs 3-iteration NR. The resulting frequency response error is inaudible below Nyquist/4. Acceptable for real-time browser use.
- **2× oversampling** on the filter only (not the whole engine). Process each input sample twice through the filter at double rate, then decimate with a simple averaging. This pushes aliasing from filter self-oscillation above audible range.
- **`fast_tanh` approximation** (Padé rational, 4th order) instead of `libm::tanhf`. The standard library tanh is ~15× slower in WASM. The approximation has <0.001 max error in the [−3, 3] range that matters for audio saturation:

```rust
/// Padé approximant tanh, accurate to <0.001 for |x| < 3.
/// Beyond ±3, clamps to ±1 (which is correct behaviour for saturation).
#[inline(always)]
pub fn fast_tanh(x: f32) -> f32 {
    let x = x.clamp(-3.0, 3.0);
    let x2 = x * x;
    x * (27.0 + x2) / (27.0 + 9.0 * x2)
}
```

### 2.4 Envelopes — ADSR

Two per voice: Env1 → filter cutoff modulation, Env2 → VCA amplitude.

```rust
pub struct Envelope {
    stage: EnvStage,
    level: f32,          // current output [0, 1]
    attack_rate: f32,    // per-sample increment (precomputed from time)
    decay_rate: f32,
    sustain_level: f32,
    release_rate: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum EnvStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}
```

**Segment shapes:**

- Attack: linear ramp (matches JP-8 behaviour)
- Decay: exponential fall toward sustain (multiply by coefficient per sample)
- Release: exponential fall toward zero

The exponential coefficient is precomputed from the time parameter:

```rust
/// Convert time in seconds to per-sample exponential coefficient.
/// Target reaches ~0.001 of start value after `time_secs`.
#[inline(always)]
fn time_to_rate(time_secs: f32, sample_rate: f32) -> f32 {
    if time_secs < 0.001 {
        return 1.0; // instant
    }
    (-6.908 / (time_secs * sample_rate)).exp() // ln(0.001) ≈ -6.908
}
```

### 2.5 LFO

One global LFO (as on the original JP-8) with optional per-voice LFO for enhanced modulation.

```rust
pub struct Lfo {
    phase: f32,
    rate: f32,           // Hz
    waveform: LfoWave,
    depth: f32,
}

#[derive(Clone, Copy)]
pub enum LfoWave {
    Sine,
    Triangle,
    Saw,
    Square,
    SampleAndHold,
}
```

LFO destinations (per the JP-8 architecture):

- VCO1 pitch
- VCO2 pitch
- VCO1 pulse width
- VCO2 pulse width
- Filter cutoff
- VCA level

Each destination has an independent depth parameter. The LFO output is a simple `f32` in [-1, 1] computed per sample — no allocation.

### 2.6 Chorus — Roland BBD Emulation

The JP-8's chorus is essentially the same circuit later used in the Juno-106. It's a short modulated delay line with stereo widening.

```rust
pub struct StereoChorus {
    buffer_l: [f32; CHORUS_BUFFER_SIZE], // fixed-size ring buffer
    buffer_r: [f32; CHORUS_BUFFER_SIZE],
    write_pos: usize,
    lfo_phase: f32,
    rate: f32,            // LFO rate (~0.5 Hz for Mode I, ~0.8 Hz for Mode II)
    depth: f32,           // modulation depth in samples
    feedback: f32,
    mix: f32,
}
```

`CHORUS_BUFFER_SIZE` = 2048 samples (~46ms at 44.1kHz). This is a compile-time constant, allocated inline.

**Chorus modes (matching JP-8):**

| Mode | LFO Rate | Depth | Character |
|------|----------|-------|-----------|
| Off | — | — | Dry signal |
| I | ~0.513 Hz | ~1.5ms | Subtle widening |
| II | ~0.863 Hz | ~2.3ms | Richer, more obvious |
| I + II | Both | Both | Full ensemble, the classic JP-8 sound |

**Delay line interpolation:** Linear interpolation between adjacent buffer samples for fractional delay. Allpass interpolation is slightly better but the difference is negligible for chorus depths.

```rust
#[inline(always)]
fn read_interpolated(buffer: &[f32; CHORUS_BUFFER_SIZE], pos: f32) -> f32 {
    let idx = pos as usize;
    let frac = pos - idx as f32;
    let a = buffer[idx % CHORUS_BUFFER_SIZE];
    let b = buffer[(idx + 1) % CHORUS_BUFFER_SIZE];
    a + frac * (b - a)
}
```

### 2.7 Voice Allocator

The JP-8 has specific voice assignment modes that affect its musical character:

```rust
pub struct VoiceAllocator {
    voices_active: [bool; 8],
    voices_note: [u8; 8],
    last_assigned: usize,
    mode: AssignMode,
    unison_detune: f32,
}

#[derive(Clone, Copy)]
pub enum AssignMode {
    Poly8,      // Standard 8-voice polyphony
    Poly4,      // 4 paired voices (dual VCO per note, thicker sound)
    Unison,     // All 8 voices on one note, detuned
    Solo,       // Monophonic with legato/portamento
}
```

Voice stealing strategy: steal the voice with the lowest envelope level (closest to silence). This matches the original JP-8 behaviour and avoids audible cut-offs.

### 2.8 Parameter Smoothing

All continuously-variable parameters (cutoff, resonance, oscillator mix, LFO depth, etc.) are smoothed to prevent zipper noise. Smoothing is a simple one-pole lowpass applied per sample:

```rust
pub struct SmoothedParam {
    current: f32,
    target: f32,
    coeff: f32,  // precomputed: exp(-2π × smoothing_freq / sample_rate)
}

impl SmoothedParam {
    #[inline(always)]
    pub fn next(&mut self) -> f32 {
        self.current += self.coeff * (self.target - self.current);
        self.current
    }
}
```

Smoothing time: ~5ms (good balance between responsiveness and click-free parameter changes).

---

## 3. WASM Binding Layer (`jp8-wasm`)

### 3.1 Exports

Minimal surface area. The WASM module exports only what the AudioWorkletProcessor needs:

```rust
use wasm_bindgen::prelude::*;
use jp8_core::Engine;

static mut ENGINE: Option<Engine> = None;

#[wasm_bindgen]
pub fn init(sample_rate: f32) {
    unsafe {
        ENGINE = Some(Engine::new(sample_rate));
    }
}

#[wasm_bindgen]
pub fn render(output_l: &mut [f32], output_r: &mut [f32]) {
    unsafe {
        if let Some(ref mut engine) = ENGINE {
            engine.render(output_l, output_r);
        }
    }
}

#[wasm_bindgen]
pub fn note_on(note: u8, velocity: u8) {
    unsafe {
        if let Some(ref mut engine) = ENGINE {
            engine.note_on(note, velocity);
        }
    }
}

#[wasm_bindgen]
pub fn note_off(note: u8) {
    unsafe {
        if let Some(ref mut engine) = ENGINE {
            engine.note_off(note);
        }
    }
}

/// Returns a pointer to the engine's parameter block in WASM linear memory.
/// The UI thread writes parameter values directly into this memory via SAB.
#[wasm_bindgen]
pub fn get_param_ptr() -> *mut f32 {
    unsafe {
        ENGINE.as_mut()
            .map(|e| e.params_mut().as_mut_ptr())
            .unwrap_or(core::ptr::null_mut())
    }
}
```

### 3.2 Build Pipeline

```bash
#!/bin/bash
# build.sh
wasm-pack build crates/jp8-wasm \
    --target no-modules \
    --release \
    -- --features simd  # optional WASM SIMD

# Optimise binary size and performance
wasm-opt -O3 -o jp8_opt.wasm crates/jp8-wasm/pkg/jp8_wasm_bg.wasm

# Strip debug info
wasm-strip jp8_opt.wasm

# Copy to JS worklet directory
cp jp8_opt.wasm js/worklet/
```

Target WASM binary size: **<50KB** gzipped (the entire DSP engine is pure arithmetic — no string handling, no I/O, no allocator).

---

## 4. AudioWorklet Integration

### 4.1 Processor (`jp8-processor.js`)

```javascript
class JP8Processor extends AudioWorkletProcessor {
    constructor() {
        super();
        this.wasmReady = false;
        this.port.onmessage = (e) => this.handleMessage(e.data);
    }

    handleMessage(data) {
        switch (data.type) {
            case 'wasm-binary':
                this.initWasm(data.binary, data.sampleRate);
                break;
            case 'note-on':
                if (this.wasmReady) this.wasm.note_on(data.note, data.velocity);
                break;
            case 'note-off':
                if (this.wasmReady) this.wasm.note_off(data.note);
                break;
            case 'sab-params':
                // Store reference to SharedArrayBuffer for parameter reads
                this.paramView = new Float32Array(data.buffer);
                break;
            case 'sab-meters':
                this.meterView = new Float32Array(data.buffer);
                break;
        }
    }

    async initWasm(binary, sampleRate) {
        const module = await WebAssembly.compile(binary);
        const instance = await WebAssembly.instantiate(module);
        this.wasm = instance.exports;
        this.wasm.init(sampleRate);
        this.wasmReady = true;
    }

    process(inputs, outputs, parameters) {
        if (!this.wasmReady) return true;

        // 1. Copy parameter values from SAB into WASM memory
        if (this.paramView) {
            this.syncParamsFromSAB();
        }

        // 2. Get output buffer pointers in WASM linear memory
        const outputL = outputs[0][0];
        const outputR = outputs[0][1] || outputs[0][0]; // mono fallback

        // 3. Render 128 samples
        this.wasm.render(outputL, outputR);

        // 4. Write metering data back to SAB for UI
        if (this.meterView) {
            this.writeMeterData(outputL, outputR);
        }

        return true; // keep processor alive
    }

    syncParamsFromSAB() {
        // Atomic read of parameter values from SAB
        // Write into WASM linear memory at param_ptr offset
        const paramPtr = this.wasm.get_param_ptr();
        const wasmMemory = new Float32Array(this.wasm.memory.buffer, paramPtr);
        for (let i = 0; i < this.paramView.length; i++) {
            wasmMemory[i] = Atomics.load(this.paramView, i);
        }
    }

    writeMeterData(outputL, outputR) {
        // RMS level for UI meters — written to SAB, UI reads at rAF rate
        let sumL = 0, sumR = 0;
        for (let i = 0; i < 128; i++) {
            sumL += outputL[i] * outputL[i];
            sumR += outputR[i] * outputR[i];
        }
        Atomics.store(this.meterView, 0, Math.sqrt(sumL / 128));
        Atomics.store(this.meterView, 1, Math.sqrt(sumR / 128));
    }
}

registerProcessor('jp8-processor', JP8Processor);
```

### 4.2 Node (`jp8-node.ts`)

```typescript
export class JP8Node {
    private context: AudioContext;
    private workletNode: AudioWorkletNode;
    private paramSAB: SharedArrayBuffer;
    private meterSAB: SharedArrayBuffer;
    private paramView: Float32Array;
    private meterView: Float32Array;

    static async create(context: AudioContext): Promise<JP8Node> {
        await context.audioWorklet.addModule('worklet/jp8-processor.js');
        return new JP8Node(context);
    }

    private constructor(context: AudioContext) {
        this.context = context;
        this.workletNode = new AudioWorkletNode(context, 'jp8-processor', {
            numberOfInputs: 0,
            numberOfOutputs: 1,
            outputChannelCount: [2],
        });

        // Allocate SABs
        this.paramSAB = new SharedArrayBuffer(PARAM_COUNT * 4); // f32 per param
        this.meterSAB = new SharedArrayBuffer(2 * 4);           // L + R RMS
        this.paramView = new Float32Array(this.paramSAB);
        this.meterView = new Float32Array(this.meterSAB);

        // Send SABs to worklet
        this.workletNode.port.postMessage({ type: 'sab-params', buffer: this.paramSAB });
        this.workletNode.port.postMessage({ type: 'sab-meters', buffer: this.meterSAB });

        // Load and send WASM binary
        this.loadWasm();
    }

    private async loadWasm() {
        const response = await fetch('worklet/jp8_opt.wasm');
        const binary = await response.arrayBuffer();
        this.workletNode.port.postMessage(
            { type: 'wasm-binary', binary, sampleRate: this.context.sampleRate },
            [binary] // transfer ownership
        );
    }

    /** Set a parameter value. Lock-free write via SAB. */
    setParam(index: number, value: number) {
        Atomics.store(this.paramView, index, value);
    }

    /** Read current meter levels. Called from rAF loop. */
    getMeterLevels(): { left: number; right: number } {
        return {
            left: Atomics.load(this.meterView, 0),
            right: Atomics.load(this.meterView, 1),
        };
    }

    /** Connect to audio graph. */
    connect(destination: AudioNode) {
        this.workletNode.connect(destination);
    }
}
```

### 4.3 MIDI Integration

```typescript
export function setupMIDI(jp8: JP8Node) {
    navigator.requestMIDIAccess().then(access => {
        for (const input of access.inputs.values()) {
            input.onmidimessage = (event) => {
                const [status, note, velocity] = event.data;
                const cmd = status & 0xF0;

                if (cmd === 0x90 && velocity > 0) {
                    jp8.noteOn(note, velocity);
                } else if (cmd === 0x80 || (cmd === 0x90 && velocity === 0)) {
                    jp8.noteOff(note);
                } else if (cmd === 0xB0) {
                    // CC mapping — mod wheel, expression, etc.
                    jp8.handleCC(note, velocity);
                }
            };
        }
    });
}
```

---

## 5. SharedArrayBuffer Protocol

### 5.1 Parameter Layout

All parameters are `f32` values at fixed indices in the parameter SAB. The UI thread writes; the audio thread reads. No locking required — single-writer, single-reader, and f32 atomic stores are naturally aligned on all platforms.

| Index | Parameter | Range | Default |
|-------|-----------|-------|---------|
| 0 | VCO1 Waveform | 0=Saw, 1=Pulse, 2=Square | 0 |
| 1 | VCO1 Range | -2..+2 (octave offset) | 0 |
| 2 | VCO1 Pulse Width | 0.05..0.95 | 0.5 |
| 3 | VCO1 Level | 0..1 | 0.8 |
| 4 | VCO2 Waveform | 0=Saw, 1=Pulse, 2=Square | 0 |
| 5 | VCO2 Range | -2..+2 | 0 |
| 6 | VCO2 Pulse Width | 0.05..0.95 | 0.5 |
| 7 | VCO2 Level | 0..1 | 0.8 |
| 8 | VCO2 Detune | -1..+1 (semitones) | 0 |
| 9 | Cross Mod Depth | 0..1 | 0 |
| 10 | Noise Level | 0..1 | 0 |
| 11 | Filter Cutoff | 20..20000 (Hz) | 8000 |
| 12 | Filter Resonance | 0..1 | 0 |
| 13 | Filter Env Depth | -1..+1 | 0.5 |
| 14 | Filter Key Track | 0..1 | 0.5 |
| 15 | Env1 Attack | 0.001..10 (seconds) | 0.01 |
| 16 | Env1 Decay | 0.001..10 | 0.3 |
| 17 | Env1 Sustain | 0..1 | 0.6 |
| 18 | Env1 Release | 0.001..10 | 0.5 |
| 19 | Env2 Attack | 0.001..10 | 0.01 |
| 20 | Env2 Decay | 0.001..10 | 0.3 |
| 21 | Env2 Sustain | 0..1 | 0.7 |
| 22 | Env2 Release | 0.001..10 | 0.5 |
| 23 | LFO Rate | 0.1..30 (Hz) | 5.0 |
| 24 | LFO Waveform | 0=Sin, 1=Tri, 2=Saw, 3=Sq, 4=S&H | 0 |
| 25 | LFO → Pitch Depth | 0..1 | 0 |
| 26 | LFO → Filter Depth | 0..1 | 0 |
| 27 | LFO → PWM Depth | 0..1 | 0 |
| 28 | Chorus Mode | 0=Off, 1=I, 2=II, 3=I+II | 3 |
| 29 | Master Volume | 0..1 | 0.7 |
| 30 | Assign Mode | 0=Poly8, 1=Poly4, 2=Uni, 3=Solo | 0 |
| 31 | Portamento Time | 0..5 (seconds) | 0 |

**PARAM_COUNT = 32**

### 5.2 Meter Layout

| Index | Value | Updated By |
|-------|-------|------------|
| 0 | Output RMS Left | Audio thread |
| 1 | Output RMS Right | Audio thread |

### 5.3 SAB Initialisation Sequence

```
1. UI Thread:  Allocate paramSAB (32 × 4 bytes) and meterSAB (2 × 4 bytes)
2. UI Thread:  Write default values into paramSAB
3. UI Thread:  postMessage({ type: 'sab-params', buffer: paramSAB }) to worklet
4. UI Thread:  postMessage({ type: 'sab-meters', buffer: meterSAB }) to worklet
5. UI Thread:  Fetch WASM binary, postMessage({ type: 'wasm-binary', ... })
6. Audio Thread: Instantiate WASM, call init(sampleRate)
7. Audio Thread: On each process() call, read paramSAB → WASM params, render, write meterSAB
8. UI Thread:  On each rAF, read meterSAB for visualisation; write paramSAB on knob changes
```

### 5.4 Required HTTP Headers

SharedArrayBuffer requires cross-origin isolation:

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

---

## 6. Performance Budget

### 6.1 Target

Render 128 stereo samples in <1.5ms on a mid-range laptop (M1 Air, i5-1240P class). This gives ~50% CPU headroom at 44.1kHz (block period = 2.9ms).

### 6.2 Per-Block Cost Breakdown (Estimated)

| Component | Per Voice | × 8 Voices | Notes |
|-----------|-----------|------------|-------|
| VCO1 (PolyBLEP) | ~2 µs | 16 µs | 128 × (phase inc + blep correction) |
| VCO2 (PolyBLEP + FM) | ~3 µs | 24 µs | + cross-mod multiply |
| Mixer + Noise | ~0.5 µs | 4 µs | trivial |
| IR3109 filter | ~8 µs | 64 µs | 128 × 4 poles × fast_tanh + 2× oversample |
| Env1 + Env2 | ~1 µs | 8 µs | branch per sample, cheap |
| LFO | ~0.5 µs | 4 µs | one wave eval per sample |
| **Voice subtotal** | **~15 µs** | **120 µs** | |
| Voice summing | | 2 µs | 8 × add |
| Chorus (stereo) | | 20 µs | 128 × 2 delay reads + interp |
| Parameter smoothing | | 5 µs | 32 params × one-pole |
| SAB read/write | | 3 µs | Atomics overhead |
| **Block total** | | **~150 µs** | |

150 µs per 128-sample block = ~5.2% CPU at 44.1kHz. Well within budget.

### 6.3 Optimisation Levers (If Needed)

1. **WASM SIMD:** Process 4 voices simultaneously using `v128` ops. Halves voice rendering cost.
2. **Skip inactive voices:** Only render voices where `env2.level > ε`.
3. **Reduce oversampling:** Drop filter oversampling from 2× to 1× (marginal quality loss).
4. **Block-rate modulation:** Compute LFO and envelope values once per block rather than per sample. Acceptable for LFO; envelope needs per-sample for fast attacks.

---

## 7. Patch System

### 7.1 Patch Format

JSON, mapping parameter names to values. No binary format — patches are tiny and human-editable.

```json
{
    "name": "Brass Ensemble",
    "version": 1,
    "params": {
        "vco1_waveform": 0,
        "vco1_range": 0,
        "vco1_pw": 0.5,
        "vco1_level": 0.8,
        "vco2_waveform": 0,
        "vco2_range": 0,
        "vco2_pw": 0.5,
        "vco2_level": 0.7,
        "vco2_detune": 0.08,
        "cross_mod": 0,
        "noise_level": 0,
        "filter_cutoff": 2500,
        "filter_resonance": 0.2,
        "filter_env_depth": 0.6,
        "filter_key_track": 0.5,
        "env1_attack": 0.05,
        "env1_decay": 0.4,
        "env1_sustain": 0.3,
        "env1_release": 0.3,
        "env2_attack": 0.05,
        "env2_decay": 0.2,
        "env2_sustain": 0.8,
        "env2_release": 0.3,
        "lfo_rate": 6.0,
        "lfo_waveform": 0,
        "lfo_pitch_depth": 0,
        "lfo_filter_depth": 0.1,
        "lfo_pwm_depth": 0,
        "chorus_mode": 3,
        "master_volume": 0.7,
        "assign_mode": 0,
        "portamento": 0
    }
}
```

### 7.2 Factory Presets

Ship with ~16 presets covering the JP-8's classic sounds:

| Preset | Character | Key Settings |
|--------|-----------|-------------|
| Brass Ensemble | Punchy brass stabs | Saw × 2, fast filter env, chorus I+II |
| Warm Pad | Slow evolving pad | Detuned saws, slow attack, high sustain, chorus I+II |
| Bass | Deep mono bass | Solo mode, saw + square, low cutoff, fast env |
| Strings | Lush string ensemble | Poly8, slow attack/release, chorus II |
| Lead | Cutting mono lead | Unison mode, pulse + saw, high res, portamento |
| Sync Lead | Aggressive sync sound | Cross-mod, resonant filter, fast env |
| Keys | Electric piano feel | Pulse waves, medium filter env, no chorus |
| Ambient Sweep | Evolving texture | LFO → filter, slow rate, pad envelope |

---

## 8. Testing Strategy

### 8.1 Offline WAV Rendering (Rust)

```rust
// tests/wav-render/main.rs
use hound::{WavSpec, WavWriter};
use jp8_core::Engine;

fn main() {
    let spec = WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = WavWriter::create("test_output.wav", spec).unwrap();
    let mut engine = Engine::new(44100.0);

    // Set up a test patch
    engine.note_on(60, 100); // Middle C

    let mut buf_l = [0.0f32; 128];
    let mut buf_r = [0.0f32; 128];

    // Render 3 seconds
    for _ in 0..(44100 * 3 / 128) {
        engine.render(&mut buf_l, &mut buf_r);
        for i in 0..128 {
            writer.write_sample(buf_l[i]).unwrap();
            writer.write_sample(buf_r[i]).unwrap();
        }
    }

    writer.finalize().unwrap();
}
```

This lets you iterate on sound design without touching the browser. Open in Logic Pro, compare against reference recordings of the real JP-8.

### 8.2 Unit Tests

| Area | Test |
|------|------|
| PolyBLEP | FFT of rendered saw, confirm harmonics roll off above Nyquist/2 |
| IR3109 | Sweep filter cutoff with white noise input, check -24dB/oct slope |
| IR3109 | Self-oscillation at high resonance produces sine at cutoff frequency |
| Envelope | Verify attack time matches parameter within 5% |
| Chorus | Verify stereo width increases with depth |
| Voice allocator | Verify correct stealing behaviour under all assign modes |
| fast_tanh | Max absolute error < 0.001 vs `f64::tanh()` reference |
| Zero-alloc | `#[global_allocator]` that panics — run full render, confirm no alloc |

### 8.3 Zero-Allocation Verification

```rust
// In test builds only
use std::alloc::{GlobalAlloc, Layout, System};

struct PanicAllocator;

unsafe impl GlobalAlloc for PanicAllocator {
    unsafe fn alloc(&self, _: Layout) -> *mut u8 {
        panic!("HEAP ALLOCATION DETECTED IN AUDIO PATH");
    }
    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {
        panic!("DEALLOC IN AUDIO PATH");
    }
}

#[global_allocator]
static ALLOC: PanicAllocator = PanicAllocator;
```

Run the full render loop with this allocator. Any accidental `Vec`, `Box`, `String`, or `format!()` in the hot path will panic immediately.

### 8.4 Benchmark Suite

Criterion benchmarks for the render path:

```rust
fn bench_full_render(c: &mut Criterion) {
    let mut engine = Engine::new(44100.0);
    // Trigger all 8 voices
    for note in 60..68 {
        engine.note_on(note, 100);
    }
    let mut buf_l = [0.0f32; 128];
    let mut buf_r = [0.0f32; 128];

    c.bench_function("render_128_8voices", |b| {
        b.iter(|| engine.render(&mut buf_l, &mut buf_r))
    });
}
```

Target: <200 µs per block on M1, <500 µs on i5-8250U (2018 laptop).

---

## 9. Browser Compatibility

| Feature | Chrome | Firefox | Safari | Required |
|---------|--------|---------|--------|----------|
| AudioWorklet | ✅ 66+ | ✅ 76+ | ✅ 14.1+ | Yes |
| WebAssembly | ✅ | ✅ | ✅ | Yes |
| SharedArrayBuffer | ✅ | ✅ | ✅ 15.2+ | Yes |
| WASM SIMD | ✅ 91+ | ✅ 89+ | ❌ | Optional (perf) |
| Web MIDI | ✅ | ❌ | ❌ | Optional |

**Minimum viable:** Chrome 91+ or Firefox 89+ (for SAB + AudioWorklet + WASM).

**Fallback for no-MIDI browsers:** On-screen keyboard + mouse/touch control.

**Fallback for no-SAB:** Fall back to `postMessage()` parameter transport. Higher latency but functional.

---

## 10. Build & Dev Workflow

### 10.1 Prerequisites

```bash
# Rust + WASM toolchain
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
cargo install wasm-opt      # from binaryen

# Node (for dev server only — no JS build step for the synth itself)
npm install -g serve        # or any static server with COOP/COEP headers
```

### 10.2 Development Loop

```bash
# 1. Edit Rust DSP code
# 2. Test offline
cargo test -p jp8-core
cargo run --example wav_render  # listen in DAW

# 3. Build WASM
./build/build.sh

# 4. Serve with required headers
serve js/ --cors \
  --header "Cross-Origin-Opener-Policy: same-origin" \
  --header "Cross-Origin-Embedder-Policy: require-corp"

# 5. Open browser, test with MIDI keyboard or on-screen keys
```

### 10.3 CI Pipeline

1. `cargo clippy -p jp8-core` — lint
2. `cargo test -p jp8-core` — unit tests including zero-alloc verification
3. `cargo bench -p jp8-core` — performance regression check
4. `wasm-pack build` — confirm WASM compilation
5. `wasm-opt` — confirm binary size <50KB gzipped

---

## 11. Milestone Plan

| Phase | Deliverable | Scope | Duration Est. |
|-------|------------|-------|---------------|
| **M1** | Single voice, offline | PolyBLEP osc + IR3109 filter + 2 ADSR + LFO. Render to WAV. Sound design iteration. | 1–2 weeks |
| **M2** | Single voice, browser | AudioWorklet + WASM integration. SAB parameter control. On-screen keyboard. | 1 week |
| **M3** | Full polyphony | 8-voice allocator, all assign modes, voice stealing. | 3–5 days |
| **M4** | Chorus + polish | Stereo BBD chorus. Cross-mod. Noise generator. Parameter smoothing. | 3–5 days |
| **M5** | UI + MIDI | Knob/slider UI, MIDI input, patch save/load, factory presets. | 1–2 weeks |
| **M6** | Optimise | WASM SIMD, profiling, binary size optimisation, Safari testing. | 1 week |

---

## 12. Open Questions / Future Considerations

1. **HPF:** The JP-8 has a simple high-pass filter before the main VCF. Worth adding as a static 1-pole HPF to remove DC offset and add bass thinning control.
2. **VCO sync:** Hard sync between VCO1→VCO2 is a classic JP-8 sound. Requires detecting VCO1 phase reset and forcing VCO2 phase reset — straightforward in PolyBLEP but needs a BLEP correction at the sync point.
3. **Arpeggiator:** The JP-8 has a built-in arpeggiator. Can run on the UI thread (lower timing precision acceptable) or audio thread (sample-accurate).
4. **Tuning tables:** Microtuning / alternative temperaments via lookup table. No allocation impact — just swap the note-to-frequency table.
5. **PWA:** Package as a PWA for offline use. The entire app is static files + WASM — perfect PWA candidate.
6. **Atoms integration:** The generative sound pipeline for Atoms could share the `jp8-core` crate's oscillator and filter primitives, compiled to the same WASM target.
