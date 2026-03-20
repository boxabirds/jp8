/// Commuted waveguide synthesis — Karplus-Strong delay line with loop filter.
/// 6 excitation presets × 5 body presets = 30 instruments.
///
/// Julius Smith's commuted synthesis: excitation waveforms from real recordings
/// are convolved with body IRs and fed into a delay line. Per-voice cost: ~10 ops/sample.

use crate::sample_data;

const MAX_DELAY: usize = 2048; // supports down to ~21 Hz at 44.1kHz
const EXCITATION_LEN: usize = sample_data::SAMPLE_LEN; // 512 samples from real recordings

/// Body preset characteristics for the loop filter.
/// (damping, brightness_offset)
const BODY_PARAMS: [(f32, f32); 5] = [
    (0.15, 0.1),    // 0: Glass — moderate damping, bright
    (0.08, 0.0),    // 1: Metal bar — low damping, neutral
    (0.10, -0.1),   // 2: Metal tube — low damping, slightly dark
    (0.06, 0.1),    // 3: Tubular bell — very resonant, bright
    (0.20, 0.2),    // 4: Wine glass — moderate damping, very bright
];

pub struct Waveguide {
    delay_line: [f32; MAX_DELAY],
    write_pos: usize,
    delay_len: usize,
    loop_filter_state: f32,
    loop_coeff: f32,
    decay: f32,
    excitation_buf: [f32; EXCITATION_LEN],
    excitation_pos: usize,
    sample_rate: f32,
}

impl Waveguide {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            delay_line: [0.0; MAX_DELAY],
            write_pos: 0,
            delay_len: 100,
            loop_filter_state: 0.0,
            loop_coeff: 0.5,
            decay: 0.9999,
            excitation_buf: [0.0; EXCITATION_LEN],
            excitation_pos: EXCITATION_LEN,
            sample_rate,
        }
    }

    pub fn set_pitch(&mut self, freq_hz: f32) {
        let len = (self.sample_rate / freq_hz.max(20.0)) as usize;
        self.delay_len = len.clamp(2, MAX_DELAY - 1);
    }

    pub fn set_params(&mut self, body_preset: u8, brightness: f32, body_mix: f32) {
        let body_idx = (body_preset as usize).min(BODY_PARAMS.len() - 1);
        let (damping, bright_offset) = BODY_PARAMS[body_idx];

        let coeff = (brightness + bright_offset).clamp(0.0, 1.0);
        self.loop_coeff = 0.3 + coeff * 0.65;

        let effective_damping = damping * body_mix + (1.0 - body_mix) * 0.01;
        self.decay = 1.0 - effective_damping * 0.00002;
    }

    /// Excite the waveguide with real sample data (called on note_on).
    pub fn excite(&mut self, excitation_preset: u8, velocity: f32) {
        let exc_idx = (excitation_preset as usize).min(sample_data::EXCITATIONS.len() - 1);
        let body_idx = 0; // body applied via loop filter params, but we can also
                          // use body IR for initial coloring

        let exc_data = sample_data::EXCITATIONS[exc_idx];
        let vel_scale = 0.3 + velocity * 0.7;

        // Load real excitation samples into buffer, scaled by velocity
        for i in 0..EXCITATION_LEN {
            self.excitation_buf[i] = exc_data[i] * vel_scale;
        }

        self.excitation_pos = 0;
        self.delay_line = [0.0; MAX_DELAY];
        self.loop_filter_state = 0.0;
    }

    /// Excite with convolution of excitation and body IR (commuted synthesis).
    pub fn excite_commuted(&mut self, excitation_preset: u8, body_preset: u8, velocity: f32) {
        let exc_idx = (excitation_preset as usize).min(sample_data::EXCITATIONS.len() - 1);
        let body_idx = (body_preset as usize).min(sample_data::BODIES.len() - 1);

        let exc_data = sample_data::EXCITATIONS[exc_idx];
        let body_data = sample_data::BODIES[body_idx];
        let vel_scale = 0.3 + velocity * 0.7;

        // Simple commuted synthesis: multiply excitation with body IR
        // (short-time approximation of convolution — works well for transients)
        for i in 0..EXCITATION_LEN {
            self.excitation_buf[i] = (exc_data[i] * 0.6 + exc_data[i] * body_data[i] * 0.4) * vel_scale;
        }

        self.excitation_pos = 0;
        self.delay_line = [0.0; MAX_DELAY];
        self.loop_filter_state = 0.0;
    }

    #[inline(always)]
    pub fn tick(&mut self) -> f32 {
        let read_pos = (self.write_pos + MAX_DELAY - self.delay_len) % MAX_DELAY;
        let delayed = self.delay_line[read_pos];

        let excitation = if self.excitation_pos < EXCITATION_LEN {
            let e = self.excitation_buf[self.excitation_pos];
            self.excitation_pos += 1;
            e
        } else {
            0.0
        };

        // One-pole loop filter
        let filtered = self.loop_coeff * (delayed + excitation)
            + (1.0 - self.loop_coeff) * self.loop_filter_state;
        self.loop_filter_state = filtered;

        let output = filtered * self.decay;

        self.delay_line[self.write_pos] = output;
        self.write_pos = (self.write_pos + 1) % MAX_DELAY;

        output
    }

    pub fn reset(&mut self) {
        self.delay_line = [0.0; MAX_DELAY];
        self.write_pos = 0;
        self.loop_filter_state = 0.0;
        self.excitation_pos = EXCITATION_LEN;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 44100.0;

    #[test]
    fn produces_output_after_excite() {
        let mut wg = Waveguide::new(SR);
        wg.set_pitch(440.0);
        wg.set_params(0, 0.5, 0.5);
        wg.excite(0, 1.0);
        let mut has_output = false;
        for _ in 0..4410 {
            if wg.tick().abs() > 0.01 { has_output = true; break; }
        }
        assert!(has_output, "Waveguide should produce output after excitation");
    }

    #[test]
    fn commuted_produces_output() {
        let mut wg = Waveguide::new(SR);
        wg.set_pitch(440.0);
        wg.set_params(0, 0.5, 0.5);
        wg.excite_commuted(0, 0, 1.0);
        let mut has_output = false;
        for _ in 0..4410 {
            if wg.tick().abs() > 0.01 { has_output = true; break; }
        }
        assert!(has_output, "Commuted excitation should produce output");
    }

    #[test]
    fn pitch_tracks_frequency() {
        let mut wg = Waveguide::new(SR);
        wg.set_pitch(440.0);
        assert_eq!(wg.delay_len, 100);
        wg.set_pitch(220.0);
        assert_eq!(wg.delay_len, 200);
    }

    #[test]
    fn no_nan_all_presets() {
        for exc in 0..6u8 {
            for body in 0..5u8 {
                let mut wg = Waveguide::new(SR);
                wg.set_pitch(440.0);
                wg.set_params(body, 0.5, 0.5);
                wg.excite_commuted(exc, body, 1.0);
                for _ in 0..44100 {
                    let out = wg.tick();
                    assert!(!out.is_nan(), "NaN from exc={exc} body={body}");
                    assert!(!out.is_infinite(), "Inf from exc={exc} body={body}");
                }
            }
        }
    }

    #[test]
    fn output_bounded() {
        let mut wg = Waveguide::new(SR);
        wg.set_pitch(440.0);
        wg.set_params(0, 1.0, 1.0);
        wg.excite(0, 1.0);
        let mut max_out = 0.0f32;
        for _ in 0..44100 {
            max_out = max_out.max(wg.tick().abs());
        }
        assert!(max_out < 5.0, "Output should be bounded, got {max_out}");
    }

    #[test]
    fn different_excitations_different_sound() {
        let mut outputs: Vec<Vec<f32>> = Vec::new();
        for exc in 0..6u8 {
            let mut wg = Waveguide::new(SR);
            wg.set_pitch(440.0);
            wg.set_params(0, 0.5, 0.5);
            wg.excite(exc, 1.0);
            let mut samples = Vec::new();
            for _ in 0..4410 { samples.push(wg.tick()); }
            outputs.push(samples);
        }
        for i in 0..5 {
            let differ = outputs[i].iter().zip(&outputs[i + 1])
                .any(|(a, b)| (a - b).abs() > 0.001);
            assert!(differ, "Excitation {i} and {} should differ", i + 1);
        }
    }

    #[test]
    fn different_bodies_different_sound() {
        let mut outputs: Vec<Vec<f32>> = Vec::new();
        for body in 0..5u8 {
            let mut wg = Waveguide::new(SR);
            wg.set_pitch(440.0);
            wg.set_params(body, 0.5, 0.5);
            wg.excite_commuted(0, body, 1.0);
            let mut samples = Vec::new();
            for _ in 0..4410 { samples.push(wg.tick()); }
            outputs.push(samples);
        }
        for i in 0..4 {
            let differ = outputs[i].iter().zip(&outputs[i + 1])
                .any(|(a, b)| (a - b).abs() > 0.001);
            assert!(differ, "Body {i} and {} should differ", i + 1);
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut wg = Waveguide::new(SR);
        wg.set_pitch(440.0);
        wg.set_params(0, 0.5, 0.5);
        wg.excite(0, 1.0);
        for _ in 0..1000 { wg.tick(); }
        wg.reset();
        let out = wg.tick();
        assert_eq!(out, 0.0, "Reset should produce silence");
    }

    #[test]
    fn low_and_high_pitch() {
        let mut wg = Waveguide::new(SR);
        wg.set_pitch(32.7);
        wg.excite(0, 1.0);
        for _ in 0..4410 { assert!(!wg.tick().is_nan()); }

        wg.set_pitch(8372.0);
        wg.excite(0, 1.0);
        for _ in 0..4410 { assert!(!wg.tick().is_nan()); }
    }

    #[test]
    fn sample_data_integrity() {
        // Verify all sample arrays have correct length and are not all zeros
        for (i, exc) in sample_data::EXCITATIONS.iter().enumerate() {
            assert_eq!(exc.len(), sample_data::SAMPLE_LEN);
            let has_nonzero = exc.iter().any(|s| s.abs() > 0.001);
            assert!(has_nonzero, "Excitation {i} should not be all zeros");
            assert!(exc.iter().all(|s| *s >= -1.0 && *s <= 1.0), "Excitation {i} should be normalized");
        }
        for (i, body) in sample_data::BODIES.iter().enumerate() {
            assert_eq!(body.len(), sample_data::SAMPLE_LEN);
            let has_nonzero = body.iter().any(|s| s.abs() > 0.001);
            assert!(has_nonzero, "Body {i} should not be all zeros");
            assert!(body.iter().all(|s| *s >= -1.0 && *s <= 1.0), "Body {i} should be normalized");
        }
    }
}
