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

/// Pad a short param slice to full PARAM_COUNT with trailing zeros.
fn pad_patch(base: &[f32]) -> [f32; PARAM_COUNT] {
    let mut out = [0.0f32; PARAM_COUNT];
    for (i, &v) in base.iter().enumerate() {
        out[i] = v;
    }
    out
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
    // Representative subset of factory patches — padded from 39 elements to 68
    let patches: Vec<[f32; PARAM_COUNT]> = [
        &[1.0, 0.0, 0.5, 0.8, 1.0, 0.0, 0.5, 0.8, 0.08, 0.0, 0.0, 0.0, 2000.0, 0.15, 0.6, 0.5, 20.0, 0.05, 0.2, 0.7, 0.3, 0.0, 0.01, 0.1, 0.8, 0.3, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 3.0, 0.7, 0.0, 0.0, 0.0, 1.0, 120.0][..],
        &[1.0, -1.0, 0.5, 0.9, 2.0, -1.0, 0.5, 0.7, 0.05, 0.0, 0.0, 0.5, 800.0, 0.3, 0.7, 0.2, 20.0, 0.005, 0.15, 0.3, 0.15, 0.0, 0.005, 0.1, 0.7, 0.2, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0, 0.0, 1.0, 120.0],
        &[0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.9, 0.0, 2000.0, 0.5, 0.8, 0.0, 20.0, 0.001, 0.15, 0.0, 0.1, 0.0, 0.001, 0.1, 0.0, 0.08, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0, 0.0, 1.0, 120.0],
    ].iter().map(|b| pad_patch(b)).collect();

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

// --- Extended param integration tests ---

#[test]
fn extended_params_default_no_nan() {
    // 68-element default array should produce same output as before (all modules off)
    let mut engine = Engine::new(SR);
    let raw = [0.0f32; PARAM_COUNT]; // all zeros — source_mode=0, all enables=0
    // Set minimum viable params for sound
    let mut raw = raw;
    raw[0] = 1.0; // vco1 saw
    raw[3] = 0.8; // vco1 level
    raw[4] = 1.0; // vco2 saw
    raw[7] = 0.8; // vco2 level
    raw[12] = 8000.0; // filter cutoff
    raw[24] = 0.7; // env2 sustain
    raw[33] = 0.7; // volume
    engine.apply_params(&raw);
    engine.note_on(60, 100);
    let out = render_blocks(&mut engine, 50);
    assert!(!has_nan_or_inf(&out), "Extended default params should produce no NaN/Inf");
    assert!(has_nonzero(&out), "Should produce audio with basic params set");
}

#[test]
fn extended_params_extreme_no_nan() {
    // All new params at extreme values — modules don't produce DSP yet,
    // but apply_params must not crash or produce NaN
    let mut engine = Engine::new(SR);
    let mut raw = [0.0f32; PARAM_COUNT];
    // Basic sound params
    raw[0] = 1.0; raw[3] = 0.8; raw[4] = 1.0; raw[7] = 0.8;
    raw[12] = 8000.0; raw[24] = 0.7; raw[33] = 0.7;
    // Extended params at extremes
    raw[39] = 2.0;  // source_mode = WG (max)
    raw[40] = -1.0;  // spectral_tilt min
    raw[41] = 64.0;  // spectral_partials max
    raw[42] = 1.0;   // spectral_noise max
    raw[49] = 1.0;   // modal_mix max
    raw[50] = 1.0;   // modal_material max (metal)
    raw[52] = 32.0;  // modal_modes max
    raw[53] = 1.0;   // modal_inharmonicity max
    raw[54] = 1.0;   // chaos_enable on
    raw[55] = 30.0;  // chaos_rate1 max
    raw[56] = 30.0;  // chaos_rate2 max
    raw[57] = 1.0;   // chaos_depth max
    raw[58] = 1.0;   // chaos_to_pitch max
    raw[59] = 1.0;   // chaos_to_filter max
    raw[60] = 1.0;   // chaos_to_pwm max
    raw[61] = 1.0;   // bubble_enable on
    raw[62] = 60.0;  // bubble_rate max
    raw[65] = 1.0;   // bubble_level max
    engine.apply_params(&raw);
    engine.note_on(60, 100);
    let out = render_blocks(&mut engine, 50);
    assert!(!has_nan_or_inf(&out), "Extreme extended params should produce no NaN/Inf");
}

#[test]
fn extended_params_clamping_roundtrip() {
    let mut engine = Engine::new(SR);
    let mut raw = [0.0f32; PARAM_COUNT];
    // Out-of-range values
    raw[39] = 99.0;   // source_mode → clamp to 2
    raw[41] = 999.0;  // spectral_partials → clamp to 64
    raw[52] = -100.0;  // modal_modes → clamp to 4
    raw[55] = 999.0;  // chaos_rate1 → clamp to 30
    engine.apply_params(&raw);
    assert!(engine.params.source_mode <= 2);
    assert!(engine.params.spectral_partials <= 64);
    assert!(engine.params.modal_modes >= 4);
    assert!(engine.params.chaos_rate1 <= 30.0);
}

#[test]
fn waveguide_plays_uploaded_wavetable() {
    // Reproduce: waveguide should play back the uploaded wavetable data,
    // not produce ring-mod/buzz from an empty buffer.
    let mut engine = Engine::new(SR);

    // Create a known wavetable: a decaying burst (like a convolved excitation)
    let mut wavetable = vec![0.0f32; 4096];
    for i in 0..4096 {
        let t = i as f32 / 4096.0;
        wavetable[i] = (t * 20.0 * std::f32::consts::TAU).sin() * (1.0 - t).max(0.0);
    }
    // Normalize
    let peak = wavetable.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    for s in &mut wavetable { *s *= 0.5 / peak; }

    // Store it as excitation 0, body 0
    engine.store_wavetable(0, 0, &wavetable);

    // Verify it's stored
    let stored = engine.get_wavetable_for_test(0, 0);
    assert_eq!(stored.len(), 4096, "Wavetable should be stored with correct length");
    assert!(stored[10].abs() > 0.001, "Wavetable should have nonzero data at index 10, got {}", stored[10]);

    // Set up waveguide mode
    let mut raw = [0.0f32; PARAM_COUNT];
    raw[0] = 1.0; raw[3] = 0.8; // VCO defaults (unused in WG mode)
    raw[24] = 1.0; // env2 sustain = 1.0
    raw[33] = 1.0; // master volume
    raw[39] = 2.0; // source_mode = waveguide
    raw[45] = 0.0; // wg_excitation = 0
    raw[46] = 0.0; // wg_body = 0
    raw[47] = 0.5; // wg_brightness
    raw[48] = 0.5; // wg_body_mix
    engine.apply_params(&raw);

    // Play a note
    engine.note_on(60, 100);

    // Render a few blocks
    let out = render_blocks(&mut engine, 10);

    // The output should contain the wavetable content, not just buzz
    assert!(has_nonzero(&out), "Waveguide should produce output");
    assert!(!has_nan_or_inf(&out), "No NaN/Inf");

    // Check that the output resembles the wavetable (first few hundred samples
    // should have the decaying burst character, not a constant buzz)
    // Note: first ~100 samples silent (delay line propagation at 440Hz).
    // Check after that.
    let early_peak = out[200..500].iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let late_peak = out[2000..2256].iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    println!("Early peak (after propagation): {early_peak}, Late peak: {late_peak}");

    assert!(early_peak > 0.01, "Waveguide should produce output after delay propagation, got {early_peak}");
    // Early burst should be louder than late sustain (wavetable decays)
    assert!(early_peak > late_peak, "Early burst ({early_peak}) should exceed late sustain ({late_peak})");
}

#[test]
fn waveguide_output_shape_diagnostic() {
    let mut engine = Engine::new(SR);

    // Simple decaying burst wavetable
    let mut wavetable = vec![0.0f32; 2000];
    for i in 0..2000 {
        let t = i as f32 / 2000.0;
        wavetable[i] = (t * 15.0 * std::f32::consts::TAU).sin() * (1.0 - t).powf(0.5) * 0.5;
    }
    engine.store_wavetable(0, 0, &wavetable);

    let mut raw = [0.0f32; PARAM_COUNT];
    raw[22] = 0.001; // env2 attack instant
    raw[23] = 0.001; // env2 decay instant
    raw[24] = 1.0;   // env2 sustain = full
    raw[25] = 5.0;   // env2 release long
    raw[33] = 1.0;   // master volume
    raw[39] = 2.0;   // source_mode = waveguide
    raw[45] = 0.0;   // excitation 0
    raw[46] = 0.0;   // body 0
    raw[47] = 0.5;   // brightness
    raw[48] = 0.5;   // body mix
    engine.apply_params(&raw);

    engine.note_on(60, 127); // C4, full velocity

    let mut buf = [0.0f32; 256]; // 128 stereo frames
    let mut all_samples = Vec::new();

    for block in 0..20 {
        engine.render(&mut buf);
        // Take left channel only
        for frame in 0..128 {
            all_samples.push(buf[frame * 2]);
        }
    }

    // Print first 300 samples
    println!("=== Waveguide output (first 300 mono samples) ===");
    for (i, &s) in all_samples[..300].iter().enumerate() {
        if i % 20 == 0 {
            let chunk: Vec<String> = all_samples[i..std::cmp::min(i+20, 300)].iter().map(|s| format!("{:.4}", s)).collect();
            println!("  [{:3}..{:3}]: {}", i, i+19, chunk.join(", "));
        }
    }

    let peak = all_samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let first_nonzero = all_samples.iter().position(|s| s.abs() > 0.001);
    println!("Peak: {peak:.4}, First nonzero: {:?}", first_nonzero);
}

#[test]
fn waveguide_through_voice_diagnostic() {
    // Test the FULL path: engine.note_on -> voice -> waveguide -> output
    // with all default params to see what the user actually hears.
    let mut engine = Engine::new(SR);

    let mut wavetable = vec![0.0f32; 2000];
    for i in 0..2000 {
        let t = i as f32 / 2000.0;
        wavetable[i] = (t * 15.0 * std::f32::consts::TAU).sin() * (1.0 - t).powf(0.5) * 0.5;
    }
    engine.store_wavetable(0, 0, &wavetable);

    // Use the ACTUAL default params (what the UI sends)
    let mut raw = [0.0f32; PARAM_COUNT];
    // Copy defaults from EngineParams::default_patch()
    raw[0] = 1.0; raw[2] = 0.5; raw[3] = 0.8; // VCO1
    raw[4] = 1.0; raw[6] = 0.5; raw[7] = 0.8; // VCO2
    raw[12] = 8000.0; // filter cutoff
    raw[14] = 0.5; raw[15] = 0.5; // filter env, key
    raw[16] = 20.0; // HPF
    raw[17] = 0.01; raw[18] = 0.3; raw[19] = 0.6; raw[20] = 0.5; // ENV1
    raw[22] = 0.01; raw[23] = 0.3; raw[24] = 0.7; raw[25] = 0.5; // ENV2
    raw[26] = 5.0; // LFO rate
    raw[32] = 3.0; // chorus mode12
    raw[33] = 0.7; // volume
    // Now override for waveguide
    raw[39] = 2.0; // source_mode = WG
    raw[47] = 0.5; raw[48] = 0.5;
    engine.apply_params(&raw);

    engine.note_on(60, 100);

    let mut buf = [0.0f32; 256];
    let mut all = Vec::new();
    for _ in 0..20 {
        engine.render(&mut buf);
        for f in 0..128 { all.push(buf[f * 2]); }
    }

    let peak = all.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let first_nz = all.iter().position(|s| s.abs() > 0.001);
    println!("Full path: peak={peak:.4}, first_nonzero={first_nz:?}");
    println!("Samples 160..200: {:?}", all[160..200].iter().map(|s| format!("{:.4}", s)).collect::<Vec<_>>());
}
