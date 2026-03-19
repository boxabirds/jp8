use std::cell::UnsafeCell;
use wasm_bindgen::prelude::*;

use jp8_core::engine::Engine;
use jp8_core::params::PARAM_COUNT;

const BLOCK_FRAMES: usize = 128;
const CHANNELS: usize = 2;
const OUTPUT_BUF_LEN: usize = BLOCK_FRAMES * CHANNELS;

/// Global engine state. Single-thread safety guaranteed by AudioWorklet.
struct WorkerCell(UnsafeCell<Option<Engine>>);
unsafe impl Sync for WorkerCell {}
static ENGINE: WorkerCell = WorkerCell(UnsafeCell::new(None));

/// Pre-allocated output buffer in WASM linear memory.
/// JS reads directly from here — zero copies.
static mut OUTPUT_BUF: [f32; OUTPUT_BUF_LEN] = [0.0; OUTPUT_BUF_LEN];

/// Pre-allocated param buffer for apply_params.
static mut PARAM_BUF: [f32; PARAM_COUNT] = [0.0; PARAM_COUNT];

fn with_engine<R>(f: impl FnOnce(&mut Engine) -> R) -> R {
    let cell = unsafe { &mut *ENGINE.0.get() };
    let engine = cell.as_mut().expect("engine not initialized");
    f(engine)
}

#[wasm_bindgen]
pub fn init_engine(sample_rate: f32) {
    let cell = unsafe { &mut *ENGINE.0.get() };
    *cell = Some(Engine::new(sample_rate));
}

/// Returns pointer to the output buffer in WASM linear memory.
/// JS creates a Float32Array view at this address — zero copy.
#[wasm_bindgen]
pub fn get_output_ptr() -> *const f32 {
    unsafe { OUTPUT_BUF.as_ptr() }
}

/// Returns the output buffer length (in f32 elements).
#[wasm_bindgen]
pub fn get_output_len() -> usize {
    OUTPUT_BUF_LEN
}

/// Render one block into the pre-allocated output buffer.
/// After calling this, read samples from get_output_ptr().
#[wasm_bindgen]
pub fn render() {
    with_engine(|e| {
        let buf = unsafe { &mut OUTPUT_BUF };
        e.render(buf);
    });
}

/// Returns pointer to the param buffer in WASM linear memory.
/// JS writes param values here, then calls apply_params_from_buf().
#[wasm_bindgen]
pub fn get_param_ptr() -> *mut f32 {
    unsafe { PARAM_BUF.as_mut_ptr() }
}

/// Apply params from the pre-allocated param buffer.
#[wasm_bindgen]
pub fn apply_params_from_buf() {
    let buf = unsafe { &PARAM_BUF };
    let mut arr = [0.0f32; PARAM_COUNT];
    arr.copy_from_slice(buf);
    with_engine(|e| e.apply_params(&arr));
}

#[wasm_bindgen]
pub fn note_on(note: u8, velocity: u8) {
    with_engine(|e| e.note_on(note, velocity));
}

#[wasm_bindgen]
pub fn note_off(note: u8) {
    with_engine(|e| e.note_off(note));
}

#[wasm_bindgen]
pub fn all_notes_off() {
    with_engine(|e| e.all_notes_off());
}

#[wasm_bindgen]
pub fn get_active_voice_count() -> u32 {
    with_engine(|e| e.voices_active_count())
}
