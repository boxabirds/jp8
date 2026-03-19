/// IR3109 OTA 4-pole ladder filter — TPT implementation.
/// Per the JP-8 spec §2.3.

use crate::math::{fast_tan, fast_tanh};

pub struct IR3109 {
    s: [f32; 4],
    pub cutoff_hz: f32,
    pub resonance: f32,
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

    #[inline(always)]
    pub fn set_cutoff(&mut self, hz: f32) {
        let freq = hz.clamp(20.0, self.sample_rate * 0.45);
        self.g = fast_tan(core::f32::consts::PI * freq / (self.sample_rate * 2.0));
        self.cutoff_hz = freq;
    }

    #[inline(always)]
    pub fn tick(&mut self, input: f32) -> f32 {
        let y1 = self.tick_inner(input);
        let y2 = self.tick_inner(input);
        (y1 + y2) * 0.5
    }

    #[inline(always)]
    fn tick_inner(&mut self, input: f32) -> f32 {
        let k = self.resonance * 4.0;
        let feedback = self.s[3];
        let x = input - k * feedback;
        let y0 = self.one_pole(0, x);
        let y1 = self.one_pole(1, y0);
        let y2 = self.one_pole(2, y1);
        self.one_pole(3, y2)
    }

    #[inline(always)]
    fn one_pole(&mut self, stage: usize, input: f32) -> f32 {
        let v = (fast_tanh(input) - self.s[stage]) * self.g;
        let y = v + self.s[stage];
        self.s[stage] = y + v;
        y
    }

    pub fn reset(&mut self) { self.s = [0.0; 4]; }
}

/// Simple 1-pole highpass filter.
/// Sits after the main VCF in the JP-8 signal chain.
pub struct HighPass {
    prev_input: f32,
    prev_output: f32,
    coeff: f32,
    sample_rate: f32,
}

impl HighPass {
    pub fn new(sample_rate: f32) -> Self {
        let mut hp = Self {
            prev_input: 0.0,
            prev_output: 0.0,
            coeff: 0.0,
            sample_rate,
        };
        hp.set_cutoff(20.0);
        hp
    }

    pub fn set_cutoff(&mut self, hz: f32) {
        let freq = hz.clamp(20.0, self.sample_rate * 0.45);
        let rc = 1.0 / (core::f32::consts::TAU * freq);
        let dt = 1.0 / self.sample_rate;
        self.coeff = rc / (rc + dt);
    }

    #[inline(always)]
    pub fn tick(&mut self, input: f32) -> f32 {
        let y = self.coeff * (self.prev_output + input - self.prev_input);
        self.prev_input = input;
        self.prev_output = y;
        y
    }

    pub fn reset(&mut self) {
        self.prev_input = 0.0;
        self.prev_output = 0.0;
    }
}
