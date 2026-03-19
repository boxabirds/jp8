/// PolyBLEP anti-aliased oscillator with waveform mixing.
/// JP-8 allows saw AND pulse simultaneously per VCO (additive, not selector).
/// wave_flags: bit 0 = saw, bit 1 = pulse.

pub struct Oscillator {
    pub phase: f32,
    pub freq: f32,
    pub target_freq: f32,       // for portamento
    pub pulse_width: f32,
    porta_coeff: f32,           // 0 = instant, close to 1 = slow glide
    drift: f32,
    drift_phase: f32,
    drift_state: f32,
}

impl Oscillator {
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            freq: 440.0,
            target_freq: 440.0,
            pulse_width: 0.5,
            porta_coeff: 0.0,
            drift: 0.0,
            drift_phase: 0.0,
            drift_state: 0.0,
        }
    }

    pub fn set_freq(&mut self, hz: f32, drift_amount: f32) {
        self.target_freq = hz;
        self.drift = drift_amount;
    }

    /// Set freq immediately (no glide).
    pub fn set_freq_immediate(&mut self, hz: f32, drift_amount: f32) {
        self.freq = hz;
        self.target_freq = hz;
        self.drift = drift_amount;
    }

    pub fn set_portamento(&mut self, time_secs: f32, sample_rate: f32) {
        if time_secs < 0.001 {
            self.porta_coeff = 0.0;
        } else {
            self.porta_coeff = (-2.0 / (time_secs * sample_rate)).exp();
        }
    }

    /// Generate one sample with waveform mixing. wave_flags: bit0=saw, bit1=pulse.
    /// Returns the mixed output and advances phase.
    #[inline(always)]
    pub fn tick(&mut self, inv_sr: f32, wave_flags: u8) -> f32 {
        // Portamento glide
        if self.porta_coeff > 0.0 {
            self.freq += (1.0 - self.porta_coeff) * (self.target_freq - self.freq);
        } else {
            self.freq = self.target_freq;
        }

        let drift_hz = self.update_drift(inv_sr);
        let effective_freq = self.freq + drift_hz;
        let dt = effective_freq * inv_sr;

        let mut out = 0.0;
        if wave_flags & 1 != 0 {
            out += self.saw(dt);
        }
        if wave_flags & 2 != 0 {
            out += self.pulse(dt);
        }

        self.phase += dt;
        if self.phase >= 1.0 { self.phase -= 1.0; }
        if self.phase < 0.0 { self.phase += 1.0; }

        out
    }

    /// Tick with frequency modulation (for cross-mod target).
    #[inline(always)]
    pub fn tick_with_fm(&mut self, inv_sr: f32, wave_flags: u8, fm_amount: f32) -> f32 {
        if self.porta_coeff > 0.0 {
            self.freq += (1.0 - self.porta_coeff) * (self.target_freq - self.freq);
        } else {
            self.freq = self.target_freq;
        }

        let drift_hz = self.update_drift(inv_sr);
        let effective_freq = self.freq + drift_hz;
        let dt = (effective_freq * inv_sr + fm_amount).max(0.0);

        let mut out = 0.0;
        if wave_flags & 1 != 0 { out += self.saw(dt); }
        if wave_flags & 2 != 0 { out += self.pulse(dt); }

        self.phase += dt;
        if self.phase >= 1.0 { self.phase -= 1.0; }
        if self.phase < 0.0 { self.phase += 1.0; }

        out
    }

    /// Sub-oscillator: square wave one octave below, using the same phase.
    #[inline(always)]
    pub fn sub_oscillator(&self) -> f32 {
        // Half-rate square: use phase/2 cycle
        let sub_phase = (self.phase * 0.5) % 1.0;
        if sub_phase < 0.5 { 1.0 } else { -1.0 }
    }

    #[inline(always)]
    fn saw(&self, dt: f32) -> f32 {
        let naive = 2.0 * self.phase - 1.0;
        naive - poly_blep(self.phase, dt)
    }

    #[inline(always)]
    fn pulse(&self, dt: f32) -> f32 {
        let mut out = if self.phase < self.pulse_width { 1.0 } else { -1.0 };
        out += poly_blep(self.phase, dt);
        out -= poly_blep((self.phase - self.pulse_width + 1.0) % 1.0, dt);
        out
    }

    #[inline(always)]
    fn update_drift(&mut self, inv_sr: f32) -> f32 {
        if self.drift <= 0.0 { return 0.0; }
        self.drift_phase += 0.3 * inv_sr;
        if self.drift_phase >= 1.0 {
            self.drift_phase -= 1.0;
            let bits = (self.phase * 1000.0) as u32;
            let rand = ((bits.wrapping_mul(1103515245).wrapping_add(12345)) >> 16) as f32 / 32768.0 - 1.0;
            self.drift_state = rand;
        }
        let cents = self.drift_state * self.drift * 3.0;
        self.freq * (2.0f32.powf(cents / 1200.0) - 1.0)
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
    }
}

#[inline(always)]
fn poly_blep(t: f32, dt: f32) -> f32 {
    if dt <= 0.0 { return 0.0; }
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

/// White noise generator.
pub struct NoiseGen {
    state: u32,
}

impl NoiseGen {
    pub fn new(seed: u32) -> Self {
        Self { state: if seed == 0 { 1 } else { seed } }
    }

    #[inline(always)]
    pub fn next(&mut self) -> f32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        (self.state as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}
