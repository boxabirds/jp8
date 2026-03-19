use std::cell::UnsafeCell;
use wasm_bindgen::prelude::*;

use jp8_core::engine::Engine;
use jp8_core::params::PARAM_COUNT;

/// Global engine state. Single-thread safety guaranteed by AudioWorklet.
struct WorkerCell(UnsafeCell<Option<Engine>>);
unsafe impl Sync for WorkerCell {}

static ENGINE: WorkerCell = WorkerCell(UnsafeCell::new(None));

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

#[wasm_bindgen]
pub fn render_block(output: &mut [f32]) {
    with_engine(|e| e.render(output));
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

/// Apply all 32 parameters from a flat f32 array (SAB snapshot).
#[wasm_bindgen]
pub fn apply_params(raw: &[f32]) {
    if raw.len() < PARAM_COUNT {
        return;
    }
    let mut arr = [0.0f32; PARAM_COUNT];
    arr.copy_from_slice(&raw[..PARAM_COUNT]);
    with_engine(|e| e.apply_params(&arr));
}

#[wasm_bindgen]
pub fn get_active_voice_count() -> u32 {
    with_engine(|e| {
        e.voices_active_count()
    })
}
