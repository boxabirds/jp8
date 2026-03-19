/// Integration tests for jp8-core Engine as a black box.

use jp8_core::engine::Engine;
use jp8_core::params::PARAM_COUNT;

const SR: f32 = 44100.0;
const BLOCK: usize = 128;
const STEREO_BLOCK: usize = BLOCK * 2;

fn render_blocks(engine: &mut Engine, blocks: usize) -> Vec<f32> {
    let mut all = Vec::new();
    let mut buf = [0.0f32; STEREO_BLOCK];
    for _ in 0..blocks {
        engine.render(&mut buf);
        all.extend_from_slice(&buf);
    }
    all
}

fn has_nan_or_inf(samples: &[f32]) -> bool {
    samples.iter().any(|s| s.is_nan() || s.is_infinite())
}

fn has_nonzero(samples: &[f32]) -> bool {
    samples.iter().any(|s| s.abs() > 1e-10)
}

#[test]
fn create_render_destroy_lifecycle() {
    let mut engine = Engine::new(SR);
    let mut buf = [0.0f32; STEREO_BLOCK];
    engine.render(&mut buf);
    // Engine dropped at end — no panic
}

#[test]
fn eight_engines_independent() {
    let mut engines: Vec<Engine> = (0..8).map(|_| Engine::new(SR)).collect();
    let notes = [48, 52, 55, 60, 64, 67, 72, 76];

    for (engine, &note) in engines.iter_mut().zip(&notes) {
        engine.note_on(note, 100);
    }

    let outputs: Vec<Vec<f32>> = engines.iter_mut()
        .map(|e| render_blocks(e, 5))
        .collect();

    // Each engine should produce different output (different notes)
    for i in 0..7 {
        let differ = outputs[i].iter().zip(&outputs[i + 1])
            .any(|(a, b)| (a - b).abs() > 1e-6);
        assert!(differ, "Engine {i} and {} should produce different output", i + 1);
    }
}

#[test]
fn param_buffer_roundtrip() {
    let mut engine = Engine::new(SR);
    let mut raw = [0.0f32; PARAM_COUNT];
    raw[0] = 1.0;   // vco1_wave_flags = saw
    raw[2] = 0.3;   // vco1_pw
    raw[12] = 5000.0; // filter_cutoff
    raw[13] = 0.5;  // filter_resonance
    raw[33] = 0.8;  // master_volume
    raw[37] = 2.0;  // arp_range

    engine.apply_params(&raw);

    assert_eq!(engine.params.vco1_wave_flags, 1);
    assert!((engine.params.vco1_pw - 0.3).abs() < 1e-6);
    assert!((engine.params.filter_cutoff - 5000.0).abs() < 1e-6);
    assert!((engine.params.filter_resonance - 0.5).abs() < 1e-6);
    assert!((engine.params.master_volume - 0.8).abs() < 1e-6);
    assert_eq!(engine.params.arp_range, 2);
}

#[test]
fn note_routing_isolation() {
    let mut engine_a = Engine::new(SR);
    let mut engine_b = Engine::new(SR);

    engine_a.note_on(60, 100);
    engine_b.note_on(72, 100);

    let out_a = render_blocks(&mut engine_a, 5);
    let out_b = render_blocks(&mut engine_b, 5);

    let differ = out_a.iter().zip(&out_b).any(|(a, b)| (a - b).abs() > 1e-6);
    assert!(differ, "Different notes should produce different output");
}

#[test]
fn render_output_256_samples() {
    let mut engine = Engine::new(SR);
    engine.note_on(60, 100);
    let mut buf = [0.0f32; STEREO_BLOCK];
    engine.render(&mut buf);
    assert_eq!(buf.len(), 256, "128 frames × 2 channels = 256 samples");
    assert!(has_nonzero(&buf));
}

#[test]
fn rapid_create_destroy_100x() {
    for _ in 0..100 {
        let mut engine = Engine::new(SR);
        engine.note_on(60, 100);
        let mut buf = [0.0f32; STEREO_BLOCK];
        engine.render(&mut buf);
    }
}

#[test]
fn all_factory_patches_no_nan() {
    // Representative subset of factory patches covering all categories
    let patches: &[[f32; PARAM_COUNT]] = &[
        // Brass
        [1.0, 0.0, 0.5, 0.8, 1.0, 0.0, 0.5, 0.8, 0.08, 0.0, 0.0, 0.0,
         2000.0, 0.15, 0.6, 0.5, 20.0, 0.05, 0.2, 0.7, 0.3, 0.0,
         0.01, 0.1, 0.8, 0.3, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 3.0, 0.7, 0.0, 0.0, 0.0, 1.0, 120.0, 0.0],
        // Bass
        [1.0, -1.0, 0.5, 0.9, 2.0, -1.0, 0.5, 0.7, 0.05, 0.0, 0.0, 0.5,
         800.0, 0.3, 0.7, 0.2, 20.0, 0.005, 0.15, 0.3, 0.15, 0.0,
         0.005, 0.1, 0.7, 0.2, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0, 0.0, 1.0, 120.0, 0.0],
        // Noise Hit
        [0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.9, 0.0,
         2000.0, 0.5, 0.8, 0.0, 20.0, 0.001, 0.15, 0.0, 0.1, 0.0,
         0.001, 0.1, 0.0, 0.08, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0, 0.0, 1.0, 120.0, 0.0],
        // Reso Sweep (high resonance)
        [1.0, 0.0, 0.5, 0.8, 1.0, 0.0, 0.5, 0.6, 0.1, 0.0, 0.0, 0.0,
         500.0, 0.7, 0.9, 0.3, 20.0, 0.01, 0.8, 0.4, 0.5, 0.0,
         0.01, 0.1, 0.8, 0.4, 0.5, 0.0, 0.0, 0.4, 0.0, 0.0, 2.0, 0.7, 0.0, 0.0, 0.0, 1.0, 120.0, 0.0],
        // Fat Unison
        [1.0, 0.0, 0.5, 0.7, 1.0, 0.0, 0.5, 0.7, 0.1, 0.0, 0.0, 0.2,
         3000.0, 0.2, 0.5, 0.4, 20.0, 0.01, 0.2, 0.7, 0.3, 0.0,
         0.01, 0.15, 0.8, 0.3, 5.0, 0.0, 0.0, 0.1, 0.0, 0.0, 3.0, 0.5, 2.0, 0.0, 0.0, 1.0, 120.0, 0.0],
    ];

    for (i, patch) in patches.iter().enumerate() {
        let mut engine = Engine::new(SR);
        engine.apply_params(patch);
        engine.note_on(60, 100);
        let out = render_blocks(&mut engine, 50);
        assert!(!has_nan_or_inf(&out), "NaN/Inf in patch {i}");
    }
}

#[test]
fn param_change_mid_render() {
    let mut engine = Engine::new(SR);
    engine.note_on(60, 100);

    let mut raw = [0.0f32; PARAM_COUNT];
    raw[0] = 1.0; raw[3] = 0.8; raw[4] = 1.0; raw[7] = 0.8;
    raw[12] = 8000.0; raw[24] = 0.7; raw[33] = 0.7;

    for _ in 0..100 {
        engine.apply_params(&raw);
        let mut buf = [0.0f32; STEREO_BLOCK];
        engine.render(&mut buf);
        assert!(!has_nan_or_inf(&buf));

        // Change params mid-stream
        raw[12] = (raw[12] + 500.0) % 20000.0 + 20.0;
        raw[13] = (raw[13] + 0.1) % 1.0;
    }
}
