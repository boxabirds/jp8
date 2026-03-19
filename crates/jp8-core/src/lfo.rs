/// LFO with multiple waveshapes.
/// Per the JP-8 spec §2.5.

#[derive(Clone, Copy)]
pub enum LfoWave {
    Sine,
    Triangle,
    Saw,
    Square,
    SampleAndHold,
}

pub struct Lfo {
    phase: f32,
    pub rate: f32,
    pub waveform: LfoWave,
    pub depth: f32,
    sh_value: f32,
    sh_state: u32,
}

impl Lfo {
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            rate: 5.0,
            waveform: LfoWave::Sine,
            depth: 0.0,
            sh_value: 0.0,
            sh_state: 12345,
        }
    }

    /// Generate one LFO sample in [-1, 1].
    #[inline(always)]
    pub fn tick(&mut self, inv_sr: f32) -> f32 {
        let prev_phase = self.phase;
        self.phase += self.rate * inv_sr;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        let raw = match self.waveform {
            LfoWave::Sine => {
                (self.phase * core::f32::consts::TAU).sin()
            }
            LfoWave::Triangle => {
                4.0 * (self.phase - 0.5).abs() - 1.0
            }
            LfoWave::Saw => {
                2.0 * self.phase - 1.0
            }
            LfoWave::Square => {
                if self.phase < 0.5 { 1.0 } else { -1.0 }
            }
            LfoWave::SampleAndHold => {
                // New random value on each cycle reset
                if self.phase < prev_phase {
                    self.sh_state ^= self.sh_state << 13;
                    self.sh_state ^= self.sh_state >> 17;
                    self.sh_state ^= self.sh_state << 5;
                    self.sh_value =
                        (self.sh_state as f32 / u32::MAX as f32) * 2.0 - 1.0;
                }
                self.sh_value
            }
        };

        raw * self.depth
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INV_SR: f32 = 1.0 / 44100.0;

    fn make_lfo(wave: LfoWave, rate: f32, depth: f32) -> Lfo {
        let mut lfo = Lfo::new();
        lfo.waveform = wave;
        lfo.rate = rate;
        lfo.depth = depth;
        lfo
    }

    #[test]
    fn sine_bounded() {
        let mut lfo = make_lfo(LfoWave::Sine, 5.0, 1.0);
        for _ in 0..44100 {
            let s = lfo.tick(INV_SR);
            assert!(s >= -1.0 && s <= 1.0, "sine out of bounds: {s}");
        }
    }

    #[test]
    fn triangle_bounded() {
        let mut lfo = make_lfo(LfoWave::Triangle, 5.0, 1.0);
        for _ in 0..44100 {
            let s = lfo.tick(INV_SR);
            assert!(s >= -1.0 && s <= 1.0, "triangle out of bounds: {s}");
        }
    }

    #[test]
    fn saw_bounded() {
        let mut lfo = make_lfo(LfoWave::Saw, 5.0, 1.0);
        for _ in 0..44100 {
            let s = lfo.tick(INV_SR);
            assert!(s >= -1.0 && s <= 1.0, "saw out of bounds: {s}");
        }
    }

    #[test]
    fn square_only_two_values() {
        let mut lfo = make_lfo(LfoWave::Square, 5.0, 1.0);
        for _ in 0..44100 {
            let s = lfo.tick(INV_SR);
            assert!(s == 1.0 || s == -1.0, "square should be ±1, got {s}");
        }
    }

    #[test]
    fn sample_and_hold_changes_on_wrap() {
        let mut lfo = make_lfo(LfoWave::SampleAndHold, 1.0, 1.0);
        // Run for one full cycle
        let samples_per_cycle = (44100.0 / 1.0) as usize;
        let first_val = lfo.tick(INV_SR);
        // Check value stays constant until phase wrap
        for _ in 1..samples_per_cycle - 10 {
            let s = lfo.tick(INV_SR);
            assert_eq!(s, first_val, "S&H should hold between wraps");
        }
        // After phase wrap, value should change (run past the wrap)
        let mut changed = false;
        for _ in 0..100 {
            let s = lfo.tick(INV_SR);
            if s != first_val {
                changed = true;
                break;
            }
        }
        assert!(changed, "S&H should change value after phase wrap");
    }

    #[test]
    fn sample_and_hold_holds_between_wraps() {
        let mut lfo = make_lfo(LfoWave::SampleAndHold, 1.0, 1.0);
        let first = lfo.tick(INV_SR);
        // For half a cycle, value should be constant
        for _ in 1..22000 {
            assert_eq!(lfo.tick(INV_SR), first);
        }
    }

    #[test]
    fn depth_zero_silence() {
        let mut lfo = make_lfo(LfoWave::Sine, 5.0, 0.0);
        for _ in 0..1000 {
            assert_eq!(lfo.tick(INV_SR), 0.0);
        }
    }

    #[test]
    fn rate_affects_period() {
        let mut lfo = make_lfo(LfoWave::Saw, 1.0, 1.0);
        // At 1Hz, after exactly 44100 samples the phase should wrap back near 0
        // Due to floating-point accumulation, phase may be slightly below 1.0 (not yet wrapped)
        for _ in 0..44100 {
            lfo.tick(INV_SR);
        }
        // Phase should be very close to 0 or very close to 1 (just before wrap)
        let near_zero_or_one = lfo.phase < 0.01 || lfo.phase > 0.99;
        assert!(near_zero_or_one, "After 1 cycle at 1Hz, phase should be near 0 or 1, got {}", lfo.phase);
    }

    #[test]
    fn reset_zeroes_phase() {
        let mut lfo = make_lfo(LfoWave::Sine, 5.0, 1.0);
        for _ in 0..1000 {
            lfo.tick(INV_SR);
        }
        assert!(lfo.phase > 0.0);
        lfo.reset();
        assert_eq!(lfo.phase, 0.0);
    }

    #[test]
    fn all_waveforms_no_nan() {
        for wave in [LfoWave::Sine, LfoWave::Triangle, LfoWave::Saw, LfoWave::Square, LfoWave::SampleAndHold] {
            let mut lfo = make_lfo(wave, 5.0, 1.0);
            for _ in 0..100_000 {
                let s = lfo.tick(INV_SR);
                assert!(!s.is_nan(), "NaN from LFO waveform");
                assert!(!s.is_infinite(), "Inf from LFO waveform");
            }
        }
    }
}
