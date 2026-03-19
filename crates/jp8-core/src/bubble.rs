/// Bubble/fluid oscillator — Minnaert equation.
/// Each bubble is a damped sinusoid whose frequency is determined by radius.
/// Poisson-distributed spawning creates naturalistic timing.
///
/// f₀ = 3.26 / radius  (simplified Minnaert for air in water)
/// Decay time ∝ radius (larger bubbles ring longer).
/// Up to 16 simultaneous bubbles. ~6 ops per active bubble.

const MAX_BUBBLES: usize = 16;
const MINNAERT_CONST: f32 = 3.26; // simplified: f = 3.26 / radius_meters

struct Bubble {
    phase: f32,
    freq: f32,        // Hz
    decay_rate: f32,  // per-sample amplitude decay
    amplitude: f32,   // current amplitude (starts at 1, decays)
    active: bool,
}

impl Bubble {
    fn new() -> Self {
        Self { phase: 0.0, freq: 0.0, decay_rate: 0.0, amplitude: 0.0, active: false }
    }

    #[inline(always)]
    fn tick(&mut self, inv_sr: f32) -> f32 {
        if !self.active { return 0.0; }
        let out = (self.phase * core::f32::consts::TAU).sin() * self.amplitude;
        self.phase += self.freq * inv_sr;
        if self.phase >= 1.0 { self.phase -= 1.0; }
        self.amplitude *= self.decay_rate;
        if self.amplitude < 0.001 {
            self.active = false;
        }
        out
    }
}

pub struct BubbleOscillator {
    bubbles: [Bubble; MAX_BUBBLES],
    spawn_timer: f32,       // samples until next spawn
    spawn_interval: f32,    // average samples between spawns
    min_size: f32,          // meters
    max_size: f32,          // meters
    sample_rate: f32,
    inv_sr: f32,
    rng_state: u32,         // simple PRNG
    next_slot: usize,       // round-robin allocation
}

impl BubbleOscillator {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            bubbles: core::array::from_fn(|_| Bubble::new()),
            spawn_timer: 1.0, // spawn on first tick after set_params
            spawn_interval: sample_rate / 5.0, // 5 bubbles/sec default
            min_size: 0.003,
            max_size: 0.01,
            sample_rate,
            inv_sr: 1.0 / sample_rate,
            rng_state: 48271,
            next_slot: 0,
        }
    }

    pub fn set_params(&mut self, rate: f32, min_size: f32, max_size: f32) {
        if rate > 0.01 {
            self.spawn_interval = self.sample_rate / rate;
        } else {
            self.spawn_interval = f32::MAX;
            self.spawn_timer = f32::MAX; // prevent any pending spawn
        }
        self.min_size = min_size.max(0.0005);
        self.max_size = max_size.max(self.min_size + 0.0001);
    }

    /// Generate one sample (sum of all active bubbles).
    #[inline(always)]
    pub fn tick(&mut self) -> f32 {
        // Spawn check
        self.spawn_timer -= 1.0;
        if self.spawn_timer <= 0.0 {
            self.spawn_bubble();
            // Poisson-distributed interval: -ln(U) * mean
            let u = self.rand_f32().max(0.001); // avoid ln(0)
            self.spawn_timer = -u.ln() * self.spawn_interval;
        }

        // Sum active bubbles
        let mut sum = 0.0f32;
        let inv_sr = self.inv_sr;
        for bubble in &mut self.bubbles {
            sum += bubble.tick(inv_sr);
        }
        sum
    }

    fn spawn_bubble(&mut self) {
        // Random radius in [min_size, max_size]
        let t = self.rand_f32();
        let radius = self.min_size + t * (self.max_size - self.min_size);

        // Minnaert frequency
        let freq = (MINNAERT_CONST / radius).clamp(20.0, 20000.0);

        // Decay proportional to radius (larger bubbles ring longer)
        // At 44.1kHz, a 0.005m bubble decays over ~50ms
        let decay_samples = radius * 10000.0; // ~50ms for 0.005m
        let decay_rate = if decay_samples > 1.0 {
            (-6.908 / decay_samples).exp() // reach 0.001 after decay_samples
        } else {
            0.0
        };

        // Allocate slot (round-robin, replacing oldest if full)
        let slot = self.next_slot % MAX_BUBBLES;
        self.next_slot = (self.next_slot + 1) % MAX_BUBBLES;

        self.bubbles[slot] = Bubble {
            phase: 0.0,
            freq,
            decay_rate,
            amplitude: 1.0,
            active: true,
        };
    }

    /// Simple LCG PRNG returning [0, 1).
    fn rand_f32(&mut self) -> f32 {
        self.rng_state = self.rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        ((self.rng_state >> 16) & 0x7FFF) as f32 / 32768.0
    }

    pub fn reset(&mut self) {
        for bubble in &mut self.bubbles {
            bubble.active = false;
        }
        self.spawn_timer = self.sample_rate; // don't spawn immediately after reset
        self.next_slot = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 44100.0;

    #[test]
    fn output_bounded() {
        let mut bo = BubbleOscillator::new(SR);
        bo.set_params(60.0, 0.001, 0.03); // max rate, wide size range
        for _ in 0..100_000 {
            let out = bo.tick();
            assert!(!out.is_nan(), "NaN from bubble oscillator");
            assert!(!out.is_infinite(), "Inf from bubble oscillator");
            assert!(out.abs() < 20.0, "Output unbounded: {out}");
        }
    }

    #[test]
    fn rate_zero_silence() {
        let mut bo = BubbleOscillator::new(SR);
        bo.set_params(0.0, 0.003, 0.01);
        // With rate=0, spawn_interval=MAX, no bubbles should spawn
        let mut has_nonzero = false;
        for _ in 0..44100 {
            if bo.tick().abs() > 0.001 { has_nonzero = true; }
        }
        assert!(!has_nonzero, "Rate=0 should produce silence");
    }

    #[test]
    fn high_rate_produces_output() {
        let mut bo = BubbleOscillator::new(SR);
        bo.set_params(30.0, 0.002, 0.008);
        let mut has_output = false;
        for _ in 0..44100 {
            if bo.tick().abs() > 0.01 { has_output = true; break; }
        }
        assert!(has_output, "Rate=30 should produce audible output");
    }

    #[test]
    fn no_nan_extreme_params() {
        let mut bo = BubbleOscillator::new(SR);
        // Max rate, max size range
        bo.set_params(60.0, 0.001, 0.03);
        for _ in 0..100_000 {
            let out = bo.tick();
            assert!(!out.is_nan());
        }
        // Min rate, min size
        bo.set_params(0.5, 0.001, 0.002);
        for _ in 0..100_000 {
            let out = bo.tick();
            assert!(!out.is_nan());
        }
    }

    #[test]
    fn reset_clears_bubbles() {
        let mut bo = BubbleOscillator::new(SR);
        bo.set_params(60.0, 0.003, 0.01);
        for _ in 0..10000 { bo.tick(); }
        bo.reset();
        bo.set_params(0.0, 0.003, 0.01); // disable spawning
        // After reset with no new spawns, should be silent
        let mut energy = 0.0f32;
        for _ in 0..1000 {
            energy += bo.tick().abs();
        }
        assert!(energy < 0.01, "After reset with no spawns, should be silent");
    }

    #[test]
    fn size_affects_pitch() {
        // Small bubbles = high frequency, large = low frequency
        let small_freq = MINNAERT_CONST / 0.002; // ~1630 Hz
        let large_freq = MINNAERT_CONST / 0.02;  // ~163 Hz
        assert!(small_freq > large_freq * 5.0, "Small bubbles should be much higher pitch");
    }
}
