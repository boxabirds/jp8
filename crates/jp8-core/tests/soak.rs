/// Soak tests — simulate 60 minutes of audio time.
/// Run with: cargo test --test soak -- --ignored
///
/// These render at CPU speed, not real-time.
/// 60 min @ 44.1kHz = 158,760,000 samples = ~1,240,312 blocks of 128 frames.
/// At ~10M samples/sec, this takes ~16 seconds wall clock.

use jp8_core::engine::Engine;
use jp8_core::params::PARAM_COUNT;

const SR: f32 = 44100.0;
const BLOCK: usize = 128;
const STEREO_BLOCK: usize = BLOCK * 2;

const SIXTY_MINUTES_BLOCKS: usize = (SR as usize * 60 * 60) / BLOCK;
const CHECK_INTERVAL: usize = (1_000_000 / STEREO_BLOCK) + 1; // ~every 1M samples

#[test]
#[ignore]
fn continuous_playback_60min() {
    let mut engine = Engine::new(SR);
    engine.note_on(60, 100);

    let mut buf = [0.0f32; STEREO_BLOCK];
    let mut max_sample = 0.0f32;
    let mut min_sample = 0.0f32;
    let mut silent_streak = 0usize;
    const MAX_SILENT_STREAK: usize = 44100 / BLOCK; // 1 second of silence

    for block in 0..SIXTY_MINUTES_BLOCKS {
        engine.render(&mut buf);

        if block % CHECK_INTERVAL == 0 {
            // Check for NaN/Inf
            for &s in &buf {
                assert!(!s.is_nan(), "NaN at block {block}");
                assert!(!s.is_infinite(), "Inf at block {block}");
            }

            // Track bounds
            for &s in &buf {
                max_sample = max_sample.max(s);
                min_sample = min_sample.min(s);
            }
            assert!(max_sample < 2.0 && min_sample > -2.0,
                "Output unbounded at block {block}: [{min_sample}, {max_sample}]");
        }

        // Check for unexpected silence while note is playing
        let block_energy: f32 = buf.iter().map(|s| s * s).sum();
        if block_energy < 1e-12 {
            silent_streak += 1;
        } else {
            silent_streak = 0;
        }
        // Allow initial silence but not sustained silence
        if block > 100 {
            assert!(silent_streak < MAX_SILENT_STREAK,
                "Unexpected silence streak at block {block}");
        }
    }
}

#[test]
#[ignore]
fn param_automation_loop_60min() {
    let mut engine = Engine::new(SR);
    engine.note_on(60, 100);

    let mut buf = [0.0f32; STEREO_BLOCK];
    let mut base_raw = [0.0f32; PARAM_COUNT];
    // Start with reasonable defaults
    base_raw[0] = 1.0; base_raw[3] = 0.8; base_raw[4] = 1.0; base_raw[7] = 0.8;
    base_raw[12] = 8000.0; base_raw[13] = 0.0; base_raw[19] = 0.6; base_raw[24] = 0.7;
    base_raw[33] = 0.7;
    base_raw[17] = 0.01; base_raw[18] = 0.3; base_raw[20] = 0.5;
    base_raw[22] = 0.01; base_raw[23] = 0.3; base_raw[25] = 0.5;

    for block in 0..SIXTY_MINUTES_BLOCKS {
        // Sweep params with sine automation
        let phase = block as f32 / 1000.0;

        let mut raw = base_raw;
        raw[12] = 1000.0 + 9000.0 * (0.5 + 0.5 * (phase * 0.1).sin()); // filter cutoff
        raw[13] = 0.5 + 0.4 * (phase * 0.07).sin(); // resonance
        raw[14] = (phase * 0.05).sin(); // env depth (bipolar)
        raw[26] = 5.0 + 10.0 * (0.5 + 0.5 * (phase * 0.03).sin()); // lfo rate
        raw[28] = 0.5 + 0.4 * (phase * 0.02).sin(); // lfo pitch depth
        raw[30] = 0.3 + 0.3 * (phase * 0.04).sin(); // lfo pwm depth
        raw[32] = ((phase * 0.01).sin() * 2.0 + 1.5).floor().clamp(0.0, 3.0); // chorus mode

        engine.apply_params(&raw);
        engine.render(&mut buf);

        if block % CHECK_INTERVAL == 0 {
            for &s in &buf {
                assert!(!s.is_nan(), "NaN at block {block}");
                assert!(!s.is_infinite(), "Inf at block {block}");
                assert!(s.abs() < 5.0, "Unbounded output at block {block}: {s}");
            }
        }
    }
}
