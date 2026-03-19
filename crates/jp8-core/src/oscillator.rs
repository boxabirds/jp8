/// PolyBLEP anti-aliased oscillator — saw, pulse, square.
/// Per the JP-8 spec §2.2.

#[derive(Clone, Copy, PartialEq)]
pub enum Waveform {
    Saw,
    Pulse,
    Square, // Pulse with PW fixed at 0.5
}

pub struct Oscillator {
    pub phase: f32,
    pub freq: f32,
    pub waveform: Waveform,
    pub pulse_width: f32,
    drift: f32,
    drift_phase: f32,
    drift_state: f32,
}

impl Oscillator {
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            freq: 440.0,
            waveform: Waveform::Saw,
            pulse_width: 0.5,
            drift: 0.0,
            drift_phase: 0.0,
            drift_state: 0.0,
        }
    }

    /// Set frequency with optional analog drift (± cents).
    pub fn set_freq(&mut self, hz: f32, drift_amount: f32) {
        self.freq = hz;
        self.drift = drift_amount;
    }

    /// Generate one sample and advance phase.
    #[inline(always)]
    pub fn tick(&mut self, inv_sr: f32) -> f32 {
        let drift_hz = self.update_drift(inv_sr);
        let freq = self.freq + drift_hz;
        let dt = freq * inv_sr;

        let out = match self.waveform {
            Waveform::Saw => self.saw(dt),
            Waveform::Pulse => self.pulse(dt),
            Waveform::Square => {
                let saved = self.pulse_width;
                self.pulse_width = 0.5;
                let out = self.pulse(dt);
                self.pulse_width = saved;
                out
            }
        };

        self.phase += dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        out
    }

    /// Tick with frequency modulation (cross-mod).
    #[inline(always)]
    pub fn tick_with_fm(&mut self, inv_sr: f32, fm_amount: f32) -> f32 {
        let drift_hz = self.update_drift(inv_sr);
        let freq = self.freq + drift_hz;
        let dt = (freq * inv_sr) + fm_amount;
        let dt = dt.max(0.0);

        let out = match self.waveform {
            Waveform::Saw => self.saw(dt),
            Waveform::Pulse => self.pulse(dt),
            Waveform::Square => {
                let saved = self.pulse_width;
                self.pulse_width = 0.5;
                let out = self.pulse(dt);
                self.pulse_width = saved;
                out
            }
        };

        self.phase += dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        if self.phase < 0.0 {
            self.phase += 1.0;
        }

        out
    }

    #[inline(always)]
    fn saw(&self, dt: f32) -> f32 {
        let naive = 2.0 * self.phase - 1.0;
        naive - poly_blep(self.phase, dt)
    }

    #[inline(always)]
    fn pulse(&self, dt: f32) -> f32 {
        let mut out = if self.phase < self.pulse_width {
            1.0
        } else {
            -1.0
        };
        out += poly_blep(self.phase, dt);
        out -= poly_blep((self.phase - self.pulse_width + 1.0) % 1.0, dt);
        out
    }

    /// Slow random drift (~0.3 Hz bandwidth, ± 3 cents max).
    #[inline(always)]
    fn update_drift(&mut self, inv_sr: f32) -> f32 {
        if self.drift <= 0.0 {
            return 0.0;
        }
        // Simple one-pole filtered noise for drift
        self.drift_phase += 0.3 * inv_sr;
        if self.drift_phase >= 1.0 {
            self.drift_phase -= 1.0;
            // Cheap pseudo-random using phase bits
            let bits = (self.phase * 1000.0) as u32;
            let rand = ((bits.wrapping_mul(1103515245).wrapping_add(12345)) >> 16) as f32 / 32768.0 - 1.0;
            self.drift_state = rand;
        }
        // Convert cents to Hz: drift_amount cents at current freq
        let cents = self.drift_state * self.drift * 3.0;
        self.freq * (2.0f32.powf(cents / 1200.0) - 1.0)
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
    }
}

/// PolyBLEP correction function (spec §2.2).
#[inline(always)]
fn poly_blep(t: f32, dt: f32) -> f32 {
    if dt <= 0.0 {
        return 0.0;
    }
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

/// White noise generator (no allocation).
pub struct NoiseGen {
    state: u32,
}

impl NoiseGen {
    pub fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    #[inline(always)]
    pub fn next(&mut self) -> f32 {
        // xorshift32
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        (self.state as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}
