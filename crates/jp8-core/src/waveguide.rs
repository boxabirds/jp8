/// Commuted waveguide synthesis — Karplus-Strong delay line with loop filter.
/// 6 excitation presets × 5 body presets = 30 instruments.
///
/// Julius Smith's commuted synthesis: pre-convolve excitation with body IR,
/// feed into delay line. Per-voice cost: ~10 ops/sample.

const MAX_DELAY: usize = 2048; // supports down to ~21 Hz at 44.1kHz
const EXCITATION_LEN: usize = 128; // short procedural impulse

/// Excitation preset: short procedural impulse characteristics.
/// (attack_sharpness, brightness, noise_amount)
const EXCITATIONS: [(f32, f32, f32); 6] = [
    (1.0, 0.8, 0.1),   // 0: Anvil strike — sharp, bright, minimal noise
    (0.3, 0.3, 0.9),   // 1: Air hiss — soft, dark, mostly noise
    (0.8, 0.5, 0.3),   // 2: Bubble pop — medium attack, moderate noise
    (0.6, 0.9, 0.2),   // 3: Door-stop twang — medium, very bright
    (1.0, 0.7, 0.15),  // 4: Hammer hit — sharp, bright
    (0.9, 0.4, 0.05),  // 5: Relay click — very sharp, dark, clean
];

/// Body preset: resonance characteristics applied to the loop filter.
/// (damping, brightness_offset, body_color)
/// damping: how quickly energy is absorbed (0=resonant, 1=dead)
/// brightness_offset: shifts the loop filter cutoff
/// body_color: additional spectral tilt
const BODIES: [(f32, f32, f32); 5] = [
    (0.15, 0.0, 0.3),   // 0: Metal bar — low damping, neutral, some color
    (0.10, -0.1, 0.5),  // 1: Metal tube — very resonant, slightly dark, hollow
    (0.25, 0.2, 0.1),   // 2: Glass — moderate damping, bright, clear
    (0.08, 0.1, 0.4),   // 3: Tubular bell — very resonant, bright, colored
    (0.20, 0.3, 0.05),  // 4: Wine glass — moderate damping, very bright, pure
];

pub struct Waveguide {
    delay_line: [f32; MAX_DELAY],
    write_pos: usize,
    delay_len: usize,      // in samples, determines pitch
    loop_filter_state: f32, // one-pole lowpass state
    loop_coeff: f32,        // filter coefficient (brightness)
    decay: f32,             // per-sample decay (0.99-0.9999)
    excitation_buf: [f32; EXCITATION_LEN],
    excitation_pos: usize,  // playback position (EXCITATION_LEN = done)
    sample_rate: f32,
    rng_state: u32,
}

impl Waveguide {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            delay_line: [0.0; MAX_DELAY],
            write_pos: 0,
            delay_len: 100,
            loop_filter_state: 0.0,
            loop_coeff: 0.5,
            decay: 0.998,
            excitation_buf: [0.0; EXCITATION_LEN],
            excitation_pos: EXCITATION_LEN, // mark as done
            sample_rate,
            rng_state: 31337,
        }
    }

    /// Set pitch from frequency.
    pub fn set_pitch(&mut self, freq_hz: f32) {
        let len = (self.sample_rate / freq_hz.max(20.0)) as usize;
        self.delay_len = len.clamp(2, MAX_DELAY - 1);
    }

    /// Configure loop filter from body preset and brightness param.
    pub fn set_params(&mut self, body_preset: u8, brightness: f32, body_mix: f32) {
        let body_idx = (body_preset as usize).min(BODIES.len() - 1);
        let (damping, bright_offset, body_color) = BODIES[body_idx];

        // Loop filter coefficient: higher = brighter (less filtering per pass)
        let coeff = (brightness + bright_offset).clamp(0.0, 1.0);
        self.loop_coeff = 0.3 + coeff * 0.65; // range [0.3, 0.95]

        // Per-sample decay: must account for the fact that at 440Hz the signal
        // passes through the loop 441 times/sec (44100/100 samples per loop).
        // We want 1-5 second audible sustain.
        // decay^(SR) = target_level after sustain_seconds.
        // So decay = target^(1/(SR * sustain_seconds))
        let effective_damping = damping * body_mix + (1.0 - body_mix) * 0.02;
        let sustain_secs = 4.0 - effective_damping * 3.0; // 1-4 seconds
        self.decay = (-6.908 / (self.sample_rate * sustain_secs)).exp();
    }

    /// Excite the waveguide (called on note_on).
    pub fn excite(&mut self, excitation_preset: u8, velocity: f32) {
        let exc_idx = (excitation_preset as usize).min(EXCITATIONS.len() - 1);
        let (sharpness, brightness, noise_amt) = EXCITATIONS[exc_idx];

        // Generate procedural excitation impulse
        let vel_scale = 0.3 + velocity * 0.7;
        for i in 0..EXCITATION_LEN {
            let t = i as f32 / EXCITATION_LEN as f32;

            // Attack envelope: sharper = faster rise
            let env = if t < 0.1 * (2.0 - sharpness) {
                t / (0.1 * (2.0 - sharpness))
            } else {
                (1.0 - t).max(0.0).powf(1.0 + sharpness)
            };

            // Tonal component: wideband impulse
            let tonal = ((t * 8.0 * core::f32::consts::TAU).sin()
                + (t * 13.0 * core::f32::consts::TAU).sin() * 0.5)
                * brightness;

            // Noise component
            let noise = (self.rand_f32() * 2.0 - 1.0) * noise_amt;

            self.excitation_buf[i] = (tonal + noise + env * 0.5) * env * vel_scale;
        }

        self.excitation_pos = 0;
        // Clear delay line for clean attack
        self.delay_line = [0.0; MAX_DELAY];
        self.loop_filter_state = 0.0;
    }

    /// Generate one sample.
    #[inline(always)]
    pub fn tick(&mut self) -> f32 {
        // Read from delay line
        let read_pos = (self.write_pos + MAX_DELAY - self.delay_len) % MAX_DELAY;
        let delayed = self.delay_line[read_pos];

        // Add excitation if still playing
        let excitation = if self.excitation_pos < EXCITATION_LEN {
            let e = self.excitation_buf[self.excitation_pos];
            self.excitation_pos += 1;
            e
        } else {
            0.0
        };

        // One-pole loop filter: y = coeff * x + (1-coeff) * y_prev
        let filtered = self.loop_coeff * (delayed + excitation)
            + (1.0 - self.loop_coeff) * self.loop_filter_state;
        self.loop_filter_state = filtered;

        // Apply decay
        let output = filtered * self.decay;

        // Write back to delay line
        self.delay_line[self.write_pos] = output;
        self.write_pos = (self.write_pos + 1) % MAX_DELAY;

        output
    }

    fn rand_f32(&mut self) -> f32 {
        self.rng_state = self.rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        ((self.rng_state >> 16) & 0x7FFF) as f32 / 32768.0
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
    fn decays_to_silence() {
        let mut wg = Waveguide::new(SR);
        wg.set_pitch(440.0);
        wg.set_params(0, 0.5, 0.5);
        wg.excite(0, 1.0);
        // Run for 10 seconds — should decay to silence
        for _ in 0..441000 { wg.tick(); }
        let mut max_out = 0.0f32;
        for _ in 0..1000 {
            max_out = max_out.max(wg.tick().abs());
        }
        assert!(max_out < 0.2, "Should decay significantly after 10s, got {max_out}");
    }

    #[test]
    fn pitch_tracks_frequency() {
        let mut wg = Waveguide::new(SR);
        wg.set_pitch(440.0);
        assert_eq!(wg.delay_len, 100); // 44100/440 = 100.2 → 100

        wg.set_pitch(220.0);
        assert_eq!(wg.delay_len, 200); // 44100/220 = 200.4 → 200
    }

    #[test]
    fn no_nan_all_presets() {
        for exc in 0..6u8 {
            for body in 0..5u8 {
                let mut wg = Waveguide::new(SR);
                wg.set_pitch(440.0);
                wg.set_params(body, 0.5, 0.5);
                wg.excite(exc, 1.0);
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
    fn different_bodies_different_sound() {
        let mut outputs: Vec<Vec<f32>> = Vec::new();
        for body in 0..5u8 {
            let mut wg = Waveguide::new(SR);
            wg.set_pitch(440.0);
            wg.set_params(body, 0.5, 0.5);
            wg.excite(0, 1.0);
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
        // MIDI 24 (~32 Hz) — near max delay
        let mut wg = Waveguide::new(SR);
        wg.set_pitch(32.7);
        wg.excite(0, 1.0);
        for _ in 0..4410 { assert!(!wg.tick().is_nan()); }

        // MIDI 96 (~8372 Hz) — very short delay
        wg.set_pitch(8372.0);
        wg.excite(0, 1.0);
        for _ in 0..4410 { assert!(!wg.tick().is_nan()); }
    }
}
