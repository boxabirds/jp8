/// Stress tests for jp8-core Engine.
/// These are marked #[ignore] — run with: cargo test --test stress -- --ignored

use jp8_core::engine::Engine;
use jp8_core::params::PARAM_COUNT;

const SR: f32 = 44100.0;
const BLOCK: usize = 128;
const STEREO_BLOCK: usize = BLOCK * 2;

fn has_nan_or_inf(samples: &[f32]) -> bool {
    samples.iter().any(|s| s.is_nan() || s.is_infinite())
}

fn is_bounded(samples: &[f32], limit: f32) -> bool {
    samples.iter().all(|s| s.abs() <= limit)
}

/// Simple LCG for deterministic pseudo-random values in tests.
struct Rng(u32);
impl Rng {
    fn new(seed: u32) -> Self { Self(seed) }
    fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1103515245).wrapping_add(12345);
        (self.0 >> 16) as f32 / 32768.0
    }
}

#[test]
#[ignore]
fn rapid_param_changes() {
    let mut engine = Engine::new(SR);
    engine.note_on(60, 100);
    let mut rng = Rng::new(42);
    let mut buf = [0.0f32; STEREO_BLOCK];

    for block in 0..10_000 {
        // Randomize params every block
        let mut raw = [0.0f32; PARAM_COUNT];
        raw[0] = if rng.next_f32() > 0.5 { 1.0 } else { 2.0 };
        raw[1] = (rng.next_f32() * 4.0 - 2.0).round();
        raw[2] = 0.05 + rng.next_f32() * 0.9;
        raw[3] = rng.next_f32();
        raw[4] = if rng.next_f32() > 0.5 { 1.0 } else { 2.0 };
        raw[5] = (rng.next_f32() * 4.0 - 2.0).round();
        raw[6] = 0.05 + rng.next_f32() * 0.9;
        raw[7] = rng.next_f32();
        raw[8] = rng.next_f32() * 2.0 - 1.0;
        raw[9] = rng.next_f32();
        raw[10] = rng.next_f32();
        raw[11] = rng.next_f32();
        raw[12] = 20.0 + rng.next_f32() * 19980.0;
        raw[13] = rng.next_f32();
        raw[14] = rng.next_f32() * 2.0 - 1.0;
        raw[15] = rng.next_f32();
        raw[16] = 20.0 + rng.next_f32() * 19980.0;
        raw[17] = 0.001 + rng.next_f32() * 2.0;
        raw[18] = 0.001 + rng.next_f32() * 2.0;
        raw[19] = rng.next_f32();
        raw[20] = 0.001 + rng.next_f32() * 2.0;
        raw[21] = if rng.next_f32() > 0.5 { 1.0 } else { 0.0 };
        raw[22] = 0.001 + rng.next_f32() * 2.0;
        raw[23] = 0.001 + rng.next_f32() * 2.0;
        raw[24] = rng.next_f32();
        raw[25] = 0.001 + rng.next_f32() * 2.0;
        raw[26] = 0.1 + rng.next_f32() * 29.9;
        raw[27] = (rng.next_f32() * 5.0).floor();
        raw[28] = rng.next_f32();
        raw[29] = rng.next_f32();
        raw[30] = rng.next_f32();
        raw[31] = rng.next_f32() * 5.0;
        raw[32] = (rng.next_f32() * 4.0).floor();
        raw[33] = rng.next_f32();
        raw[34] = 0.0;
        raw[35] = rng.next_f32() * 5.0;
        raw[36] = 0.0;
        raw[37] = 1.0;
        raw[38] = 120.0;
        raw[39] = 0.0;

        engine.apply_params(&raw);
        engine.render(&mut buf);
        assert!(!has_nan_or_inf(&buf), "NaN/Inf at block {block}");
    }
}

#[test]
#[ignore]
fn note_spam() {
    let mut engine = Engine::new(SR);
    let mut buf = [0.0f32; STEREO_BLOCK];

    for block in 0..5_000 {
        // Rapid note on/off
        for note in 0..100u8 {
            engine.note_on(note % 128, 100);
        }
        for note in 0..100u8 {
            engine.note_off(note % 128);
        }
        engine.render(&mut buf);
        assert!(!has_nan_or_inf(&buf), "NaN/Inf at block {block}");
    }
}

#[test]
#[ignore]
fn all_eight_voices_sustained() {
    let mut engine = Engine::new(SR);
    for note in 60..68 {
        engine.note_on(note, 100);
    }

    let mut buf = [0.0f32; STEREO_BLOCK];
    for block in 0..10_000 {
        engine.render(&mut buf);
        assert!(!has_nan_or_inf(&buf), "NaN/Inf at block {block}");
        assert!(is_bounded(&buf, 10.0), "Unbounded output at block {block}");
    }
}

#[test]
#[ignore]
fn extreme_param_combos() {
    let adversarial: &[[f32; PARAM_COUNT]] = &[
        // All zeros
        [0.0; PARAM_COUNT],
        // Max resonance + low cutoff
        {
            let mut p = [0.0f32; PARAM_COUNT];
            p[0] = 1.0; p[3] = 1.0; p[4] = 1.0; p[7] = 1.0;
            p[12] = 20.0; p[13] = 1.0; p[14] = 1.0; p[24] = 1.0;
            p[33] = 1.0;
            p
        },
        // Max everything
        {
            let mut p = [1.0f32; PARAM_COUNT];
            p[1] = 2.0; p[5] = 2.0; // ranges
            p[12] = 20000.0; p[16] = 20000.0; // cutoffs in Hz
            p[17] = 5.0; p[18] = 5.0; p[20] = 5.0; // env times
            p[22] = 5.0; p[23] = 5.0; p[25] = 5.0;
            p[26] = 30.0; // lfo rate
            p[33] = 1.0; // volume
            p[37] = 4.0; p[38] = 300.0; // arp range/tempo
            p
        },
        // Negative filter env depth + high resonance
        {
            let mut p = [0.0f32; PARAM_COUNT];
            p[0] = 1.0; p[3] = 1.0; p[4] = 1.0; p[7] = 1.0;
            p[12] = 500.0; p[13] = 0.95; p[14] = -1.0;
            p[17] = 0.001; p[18] = 0.001; p[19] = 1.0; p[20] = 0.001;
            p[22] = 0.001; p[23] = 0.001; p[24] = 1.0; p[25] = 0.001;
            p[33] = 1.0;
            p
        },
        // Sub-bass extreme
        {
            let mut p = [0.0f32; PARAM_COUNT];
            p[0] = 1.0; p[1] = -2.0; p[3] = 1.0;
            p[4] = 1.0; p[5] = -2.0; p[7] = 1.0;
            p[11] = 1.0; // sub osc max
            p[12] = 20.0; p[13] = 0.8;
            p[24] = 1.0; p[33] = 1.0;
            p
        },
    ];

    for (i, params) in adversarial.iter().enumerate() {
        let mut engine = Engine::new(SR);
        engine.apply_params(params);
        engine.note_on(60, 127);
        let mut buf = [0.0f32; STEREO_BLOCK];
        for block in 0..100 {
            engine.render(&mut buf);
            assert!(!has_nan_or_inf(&buf), "NaN/Inf in adversarial set {i} at block {block}");
        }
    }
}

#[test]
#[ignore]
fn voice_stealing_under_load() {
    let mut engine = Engine::new(SR);
    // Set to Poly4
    let mut raw = [0.0f32; PARAM_COUNT];
    raw[0] = 1.0; raw[3] = 0.8; raw[4] = 1.0; raw[7] = 0.8;
    raw[12] = 8000.0; raw[24] = 0.7; raw[33] = 0.7;
    raw[34] = 0.0; // Poly8 (Poly4 would need mode value — use Poly8 and spam more)
    engine.apply_params(&raw);

    let mut buf = [0.0f32; STEREO_BLOCK];
    // Play 20 notes forcing voice stealing
    for note in 40..60 {
        engine.note_on(note, 100);
        engine.render(&mut buf);
        assert!(!has_nan_or_inf(&buf), "NaN during voice stealing at note {note}");
    }
}

#[test]
#[ignore]
fn cross_mod_extreme() {
    let mut engine = Engine::new(SR);
    let mut raw = [0.0f32; PARAM_COUNT];
    raw[0] = 1.0; raw[3] = 1.0; // VCO1 saw, max level
    raw[4] = 1.0; raw[7] = 1.0; // VCO2 saw, max level
    raw[9] = 1.0; // cross-mod max
    raw[12] = 8000.0; raw[24] = 0.8; raw[33] = 0.7;
    engine.apply_params(&raw);

    engine.note_on(80, 127); // high note for more FM effect

    let mut buf = [0.0f32; STEREO_BLOCK];
    for block in 0..10_000 {
        engine.render(&mut buf);
        assert!(!has_nan_or_inf(&buf), "NaN at block {block} with extreme cross-mod");
    }
}

#[test]
#[ignore]
fn rapid_arp_mode_toggle() {
    let mut engine = Engine::new(SR);
    let mut raw = [0.0f32; PARAM_COUNT];
    raw[0] = 1.0; raw[3] = 0.8; raw[4] = 1.0; raw[7] = 0.8;
    raw[12] = 8000.0; raw[24] = 0.7; raw[33] = 0.7;
    raw[36] = 1.0; // arp Up
    raw[38] = 120.0;
    engine.apply_params(&raw);

    engine.note_on(60, 100);
    engine.note_on(64, 100);

    let mut buf = [0.0f32; STEREO_BLOCK];
    for block in 0..5_000 {
        raw[36] = (block % 4) as f32; // cycle through Off, Up, Down, UpDown
        engine.apply_params(&raw);
        engine.render(&mut buf);
        assert!(!has_nan_or_inf(&buf), "NaN at block {block} during arp toggle");
    }
}

#[test]
#[ignore]
fn zero_attack_zero_release_spam() {
    let mut engine = Engine::new(SR);
    let mut raw = [0.0f32; PARAM_COUNT];
    raw[0] = 1.0; raw[3] = 0.8; raw[4] = 1.0; raw[7] = 0.8;
    raw[12] = 8000.0; raw[33] = 0.7;
    raw[17] = 0.001; raw[18] = 0.001; raw[19] = 1.0; raw[20] = 0.001; // env1 instant
    raw[22] = 0.001; raw[23] = 0.001; raw[24] = 1.0; raw[25] = 0.001; // env2 instant
    engine.apply_params(&raw);

    let mut buf = [0.0f32; STEREO_BLOCK];
    for block in 0..5_000 {
        engine.note_on(60, 127);
        engine.render(&mut buf);
        engine.note_off(60);
        engine.render(&mut buf);
        assert!(!has_nan_or_inf(&buf), "NaN at block {block} with zero attack/release");
    }
}
