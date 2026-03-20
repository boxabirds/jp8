use wasm_bindgen::prelude::*;

use jp8_core::engine::Engine;
use jp8_core::params::PARAM_COUNT;

const MAX_ENGINES: usize = 8;
const BLOCK_FRAMES: usize = 128;
const CHANNELS: usize = 2;
const OUTPUT_BUF_LEN: usize = BLOCK_FRAMES * CHANNELS;
const MAX_WAVETABLE: usize = 16384;

static mut ENGINES: [Option<Engine>; MAX_ENGINES] = [
    None, None, None, None, None, None, None, None,
];
static mut OUTPUT_BUFS: [[f32; OUTPUT_BUF_LEN]; MAX_ENGINES] = [[0.0; OUTPUT_BUF_LEN]; MAX_ENGINES];
static mut PARAM_BUFS: [[f32; PARAM_COUNT]; MAX_ENGINES] = [[0.0; PARAM_COUNT]; MAX_ENGINES];
/// Shared wavetable upload buffer (one per engine, reused for each upload).
static mut WAVETABLE_BUF: [[f32; MAX_WAVETABLE]; MAX_ENGINES] = [[0.0; MAX_WAVETABLE]; MAX_ENGINES];

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

#[wasm_bindgen]
pub fn get_output_ptr(id: usize) -> *const f32 {
    if id >= MAX_ENGINES { return core::ptr::null(); }
    unsafe { OUTPUT_BUFS[id].as_ptr() }
}

#[wasm_bindgen]
pub fn get_output_len() -> usize {
    OUTPUT_BUF_LEN
}

#[wasm_bindgen]
pub fn render(id: usize) {
    if id >= MAX_ENGINES { return; }
    let buf = unsafe { &mut OUTPUT_BUFS[id] };
    if let Some(engine) = unsafe { ENGINES[id].as_mut() } {
        engine.render(buf);
    }
}

#[wasm_bindgen]
pub fn get_param_ptr(id: usize) -> *mut f32 {
    if id >= MAX_ENGINES { return core::ptr::null_mut(); }
    unsafe { PARAM_BUFS[id].as_mut_ptr() }
}

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

/// Returns pointer to engine #id's wavetable upload buffer.
/// JS writes convolved data here, then calls store_wavetable.
#[wasm_bindgen]
pub fn get_wavetable_ptr(id: usize) -> *mut f32 {
    if id >= MAX_ENGINES { return core::ptr::null_mut(); }
    unsafe { WAVETABLE_BUF[id].as_mut_ptr() }
}

/// Store the uploaded wavetable into the engine's cache at (exc, body) index.
/// JS calls this after writing data to the wavetable buffer.
#[wasm_bindgen]
pub fn store_wavetable(id: usize, exc_idx: u8, body_idx: u8, len: usize) {
    if id >= MAX_ENGINES { return; }
    let actual_len = len.min(MAX_WAVETABLE);
    let buf = unsafe { &WAVETABLE_BUF[id][..actual_len] };
    with_engine(id, |e| e.store_wavetable(exc_idx, body_idx, buf));
}
