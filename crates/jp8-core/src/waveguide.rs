/// Commuted waveguide synthesis — matches audio-1 CommutedMatrixSynth.
///
/// The wavetable (pre-convolved excitation × body IR) is computed in JS
/// using OfflineAudioContext + ConvolverNode and uploaded to WASM.
/// The waveguide just plays it back through a delay line with loop filter.
///
/// Per-voice cost: ~5 ops/sample.

const MAX_DELAY: usize = 2048;
const MAX_WAVETABLE: usize = 16384;
const FEEDBACK_MIN: f32 = 0.995;
const FEEDBACK_RANGE: f32 = 0.0049;
const DENORMAL_BIAS: f32 = 1.0e-25;
const FADE_SAMPLES: usize = 128; // ~2.7ms crossfade on retrigger

pub struct Waveguide {
    wavetable: Vec<f32>,
    wavetable_len: usize,
    wavetable_pos: usize,
    wavetable_playing: bool,
    delay_buf: Vec<f32>,
    delay_write: usize,
    delay_length: usize,
    loop_filter_state: f32,
    filter_coeff: f32,
    feedback: f32,
    fade_remaining: usize, // crossfade counter on retrigger
    sample_rate: f32,
}

impl Waveguide {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            wavetable: vec![0.0; MAX_WAVETABLE],
            wavetable_len: 0,
            wavetable_pos: 0,
            wavetable_playing: false,
            delay_buf: vec![0.0; MAX_DELAY],
            delay_write: 0,
            delay_length: 200,
            loop_filter_state: 0.0,
            filter_coeff: 0.5,
            feedback: 0.997,
            fade_remaining: 0,
            sample_rate,
        }
    }

    /// Load a pre-convolved wavetable (from JS-side FFT convolution).
    pub fn set_wavetable(&mut self, data: &[f32]) {
        let len = data.len().min(MAX_WAVETABLE);
        self.wavetable[..len].copy_from_slice(&data[..len]);
        self.wavetable_len = len;
    }

    pub fn set_pitch(&mut self, freq_hz: f32) {
        let len = (self.sample_rate / freq_hz.max(20.0)) as usize;
        self.delay_length = len.clamp(2, MAX_DELAY - 1);
    }

    pub fn set_params(&mut self, _body_preset: u8, brightness: f32, body_mix: f32) {
        // filter_coeff: 0 = bright, 1 = dark (matches audio-1)
        self.filter_coeff = 1.0 - brightness * 0.95;
        // Feedback: 0.995 - 0.9999 (body_mix controls sustain)
        self.feedback = FEEDBACK_MIN + body_mix * FEEDBACK_RANGE;
    }

    /// Trigger playback — crossfade with existing content (matches audio-1).
    /// Old resonance fades out over FADE_SAMPLES while new excitation ramps in.
    pub fn trigger(&mut self) {
        self.wavetable_pos = 0;
        self.wavetable_playing = self.wavetable_len > 0;
        self.fade_remaining = FADE_SAMPLES;
    }

    /// Render one sample — matches audio-1 CommutedMatrixSynth with crossfade.
    #[inline(always)]
    pub fn tick(&mut self) -> f32 {
        // Read raw excitation from wavetable
        let raw_excitation = if self.wavetable_playing && self.wavetable_pos < self.wavetable_len {
            let val = self.wavetable[self.wavetable_pos];
            self.wavetable_pos += 1;
            if self.wavetable_pos >= self.wavetable_len {
                self.wavetable_playing = false;
            }
            val
        } else {
            0.0
        };

        // Crossfade on retrigger: ramp excitation in, reduce feedback to let old die
        let (excitation, feedback_scale) = if self.fade_remaining > 0 {
            let t = self.fade_remaining as f32 / FADE_SAMPLES as f32;
            self.fade_remaining -= 1;
            (raw_excitation * (1.0 - t), t * 0.3) // ramp 0→1, feedback 1→0.3
        } else {
            (raw_excitation, 1.0)
        };

        // Read from delay line
        let read_pos = (self.delay_write + MAX_DELAY - self.delay_length) % MAX_DELAY;
        let delayed = self.delay_buf[read_pos];

        // One-pole loop filter on delayed signal
        self.loop_filter_state = delayed * (1.0 - self.filter_coeff)
            + self.loop_filter_state * self.filter_coeff;

        // Write: excitation + filtered feedback (scaled during crossfade)
        self.delay_buf[self.delay_write] =
            excitation + self.loop_filter_state * self.feedback * feedback_scale + DENORMAL_BIAS;
        self.delay_write = (self.delay_write + 1) % MAX_DELAY;

        delayed
    }

    /// Legacy excite for Rust-only tests (no JS convolution available).
    pub fn excite_simple(&mut self, data: &[f32], velocity: f32) {
        let len = data.len().min(MAX_WAVETABLE);
        for i in 0..len {
            self.wavetable[i] = data[i] * velocity;
        }
        self.wavetable_len = len;
        self.trigger();
    }

    pub fn reset(&mut self) {
        for s in self.delay_buf.iter_mut() { *s = 0.0; }
        self.delay_write = 0;
        self.loop_filter_state = 0.0;
        self.wavetable_pos = 0;
        self.wavetable_playing = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 44100.0;

    fn make_impulse() -> Vec<f32> {
        let mut data = vec![0.0f32; 512];
        // Simple click + decay for testing
        data[0] = 1.0;
        data[1] = 0.5;
        data[2] = -0.3;
        for i in 3..512 {
            data[i] = data[i - 1] * 0.95 * (i as f32 * 0.1).sin();
        }
        data
    }

    #[test]
    fn produces_output_after_excite() {
        let mut wg = Waveguide::new(SR);
        wg.set_pitch(440.0);
        wg.set_params(0, 0.5, 0.5);
        wg.excite_simple(&make_impulse(), 1.0);
        let mut has_output = false;
        for _ in 0..4410 {
            if wg.tick().abs() > 0.001 { has_output = true; break; }
        }
        assert!(has_output);
    }

    #[test]
    fn pitch_tracks_frequency() {
        let mut wg = Waveguide::new(SR);
        wg.set_pitch(440.0);
        assert_eq!(wg.delay_length, 100);
        wg.set_pitch(220.0);
        assert_eq!(wg.delay_length, 200);
    }

    #[test]
    fn no_nan() {
        let mut wg = Waveguide::new(SR);
        wg.set_pitch(440.0);
        wg.set_params(0, 0.5, 0.5);
        wg.excite_simple(&make_impulse(), 1.0);
        for _ in 0..100_000 {
            let out = wg.tick();
            assert!(!out.is_nan());
            assert!(!out.is_infinite());
        }
    }

    #[test]
    fn output_bounded() {
        let mut wg = Waveguide::new(SR);
        wg.set_pitch(440.0);
        wg.set_params(0, 1.0, 1.0);
        wg.excite_simple(&make_impulse(), 1.0);
        let mut max = 0.0f32;
        for _ in 0..44100 {
            max = max.max(wg.tick().abs());
        }
        assert!(max < 5.0, "output bounded, got {max}");
    }

    #[test]
    fn reset_clears() {
        let mut wg = Waveguide::new(SR);
        wg.set_pitch(440.0);
        wg.excite_simple(&make_impulse(), 1.0);
        for _ in 0..1000 { wg.tick(); }
        wg.reset();
        assert_eq!(wg.tick(), 0.0);
    }

    #[test]
    fn set_wavetable_works() {
        let mut wg = Waveguide::new(SR);
        let data = make_impulse();
        wg.set_wavetable(&data);
        assert_eq!(wg.wavetable_len, 512);
        wg.set_pitch(440.0);
        wg.trigger();
        let mut has_output = false;
        for _ in 0..4410 {
            if wg.tick().abs() > 0.001 { has_output = true; break; }
        }
        assert!(has_output);
    }
}
