/// IR3109 OTA 4-pole ladder filter — TPT implementation.
/// Per the JP-8 spec §2.3.

use crate::math::{fast_tan, fast_tanh};

pub struct IR3109 {
    s: [f32; 4],
    pub cutoff_hz: f32,
    pub resonance: f32, // [0, 1] → mapped to k [0, ~4]
    g: f32,
    sample_rate: f32,
}

impl IR3109 {
    pub fn new(sample_rate: f32) -> Self {
        let mut f = Self {
            s: [0.0; 4],
            cutoff_hz: 8000.0,
            resonance: 0.0,
            g: 0.0,
            sample_rate,
        };
        f.set_cutoff(8000.0);
        f
    }

    /// Precompute coefficient when cutoff changes.
    #[inline(always)]
    pub fn set_cutoff(&mut self, hz: f32) {
        let freq = hz.clamp(20.0, self.sample_rate * 0.45);
        // 2x oversampling: compute g at double rate
        self.g = fast_tan(core::f32::consts::PI * freq / (self.sample_rate * 2.0));
        self.cutoff_hz = freq;
    }

    /// Process one sample with 2x oversampling. Zero allocations.
    #[inline(always)]
    pub fn tick(&mut self, input: f32) -> f32 {
        // 2x oversample: process twice, average
        let y1 = self.tick_inner(input);
        let y2 = self.tick_inner(input);
        (y1 + y2) * 0.5
    }

    #[inline(always)]
    fn tick_inner(&mut self, input: f32) -> f32 {
        let k = self.resonance * 4.0;

        // One-step delay feedback (avoids Newton-Raphson solve)
        let feedback = self.s[3];
        let x = input - k * feedback;

        // Four cascaded one-pole stages with OTA saturation
        let y0 = self.one_pole(0, x);
        let y1 = self.one_pole(1, y0);
        let y2 = self.one_pole(2, y1);
        let y3 = self.one_pole(3, y2);

        y3
    }

    #[inline(always)]
    fn one_pole(&mut self, stage: usize, input: f32) -> f32 {
        let v = (fast_tanh(input) - self.s[stage]) * self.g;
        let y = v + self.s[stage];
        self.s[stage] = y + v; // trapezoidal integrator state update
        y
    }

    pub fn reset(&mut self) {
        self.s = [0.0; 4];
    }
}
