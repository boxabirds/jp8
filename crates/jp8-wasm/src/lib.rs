use wasm_bindgen::prelude::*;

use jp8_core::engine::Engine;
use jp8_core::params::PARAM_COUNT;

const MAX_ENGINES: usize = 8;
const BLOCK_FRAMES: usize = 128;
const CHANNELS: usize = 2;
const OUTPUT_BUF_LEN: usize = BLOCK_FRAMES * CHANNELS;

/// Multiple engines, each with its own output and param buffers.
/// All pre-allocated — zero heap allocs after init.
static mut ENGINES: [Option<Engine>; MAX_ENGINES] = [
    None, None, None, None, None, None, None, None,
];
static mut OUTPUT_BUFS: [[f32; OUTPUT_BUF_LEN]; MAX_ENGINES] = [[0.0; OUTPUT_BUF_LEN]; MAX_ENGINES];
static mut PARAM_BUFS: [[f32; PARAM_COUNT]; MAX_ENGINES] = [[0.0; PARAM_COUNT]; MAX_ENGINES];

fn with_engine<R>(id: usize, f: impl FnOnce(&mut Engine) -> R) -> Option<R> {
    if id >= MAX_ENGINES { return None; }
    let engine = unsafe { ENGINES[id].as_mut()? };
    Some(f(engine))
}

#[wasm_bindgen]
pub fn create_engine(id: usize, sample_rate: f32) {
    if id < MAX_ENGINES {
        unsafe { ENGINES[id] = Some(Engine::new(sample_rate)); }
    }
}

#[wasm_bindgen]
pub fn destroy_engine(id: usize) {
    if id < MAX_ENGINES {
        unsafe { ENGINES[id] = None; }
    }
}

/// Returns pointer to engine #id's output buffer.
#[wasm_bindgen]
pub fn get_output_ptr(id: usize) -> *const f32 {
    if id >= MAX_ENGINES { return core::ptr::null(); }
    unsafe { OUTPUT_BUFS[id].as_ptr() }
}

#[wasm_bindgen]
pub fn get_output_len() -> usize {
    OUTPUT_BUF_LEN
}

/// Render engine #id into its own output buffer.
#[wasm_bindgen]
pub fn render(id: usize) {
    if id >= MAX_ENGINES { return; }
    let buf = unsafe { &mut OUTPUT_BUFS[id] };
    if let Some(engine) = unsafe { ENGINES[id].as_mut() } {
        engine.render(buf);
    }
}

/// Returns pointer to engine #id's param buffer.
#[wasm_bindgen]
pub fn get_param_ptr(id: usize) -> *mut f32 {
    if id >= MAX_ENGINES { return core::ptr::null_mut(); }
    unsafe { PARAM_BUFS[id].as_mut_ptr() }
}

/// Apply params from engine #id's param buffer.
#[wasm_bindgen]
pub fn apply_params_from_buf(id: usize) {
    if id >= MAX_ENGINES { return; }
    let buf = unsafe { &PARAM_BUFS[id] };
    let mut arr = [0.0f32; PARAM_COUNT];
    arr.copy_from_slice(buf);
    with_engine(id, |e| e.apply_params(&arr));
}

#[wasm_bindgen]
pub fn note_on(id: usize, note: u8, velocity: u8) {
    with_engine(id, |e| e.note_on(note, velocity));
}

#[wasm_bindgen]
pub fn note_off(id: usize, note: u8) {
    with_engine(id, |e| e.note_off(note));
}

#[wasm_bindgen]
pub fn all_notes_off(id: usize) {
    with_engine(id, |e| e.all_notes_off());
}

#[wasm_bindgen]
pub fn get_active_voice_count(id: usize) -> u32 {
    with_engine(id, |e| e.voices_active_count()).unwrap_or(0)
}
