/// ADSR envelope generator.
/// Per the JP-8 spec §2.4.

#[derive(Clone, Copy, PartialEq)]
pub enum EnvStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

pub struct Envelope {
    pub stage: EnvStage,
    pub level: f32,
    attack_rate: f32,
    decay_coeff: f32,
    pub sustain_level: f32,
    release_coeff: f32,
    sample_rate: f32,
}

impl Envelope {
    pub fn new(sample_rate: f32) -> Self {
        let mut e = Self {
            stage: EnvStage::Idle,
            level: 0.0,
            attack_rate: 0.0,
            decay_coeff: 0.0,
            sustain_level: 0.7,
            release_coeff: 0.0,
            sample_rate,
        };
        e.set_adsr(0.01, 0.3, 0.7, 0.5);
        e
    }

    /// Set ADSR times in seconds.
    pub fn set_adsr(&mut self, attack: f32, decay: f32, sustain: f32, release: f32) {
        self.attack_rate = if attack < 0.001 {
            1.0
        } else {
            1.0 / (attack * self.sample_rate)
        };
        self.decay_coeff = time_to_rate(decay, self.sample_rate);
        self.sustain_level = sustain.clamp(0.0, 1.0);
        self.release_coeff = time_to_rate(release, self.sample_rate);
    }

    pub fn gate_on(&mut self) {
        self.stage = EnvStage::Attack;
    }

    pub fn gate_off(&mut self) {
        if self.stage != EnvStage::Idle {
            self.stage = EnvStage::Release;
        }
    }

    /// Generate one envelope sample.
    #[inline(always)]
    pub fn tick(&mut self) -> f32 {
        match self.stage {
            EnvStage::Idle => {}
            EnvStage::Attack => {
                // Linear ramp (matches JP-8 behaviour)
                self.level += self.attack_rate;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = EnvStage::Decay;
                }
            }
            EnvStage::Decay => {
                // Exponential fall toward sustain
                self.level = self.sustain_level
                    + (self.level - self.sustain_level) * self.decay_coeff;
                if (self.level - self.sustain_level).abs() < 0.001 {
                    self.level = self.sustain_level;
                    self.stage = EnvStage::Sustain;
                }
            }
            EnvStage::Sustain => {
                self.level = self.sustain_level;
            }
            EnvStage::Release => {
                // Exponential fall toward zero
                self.level *= self.release_coeff;
                if self.level < 0.001 {
                    self.level = 0.0;
                    self.stage = EnvStage::Idle;
                }
            }
        }
        self.level
    }

    pub fn is_active(&self) -> bool {
        self.stage != EnvStage::Idle
    }

    pub fn reset(&mut self) {
        self.stage = EnvStage::Idle;
        self.level = 0.0;
    }
}

/// Convert time in seconds to per-sample exponential coefficient.
/// Target reaches ~0.001 of start value after `time_secs`.
#[inline(always)]
fn time_to_rate(time_secs: f32, sample_rate: f32) -> f32 {
    if time_secs < 0.001 {
        return 0.0; // instant
    }
    (-6.908 / (time_secs * sample_rate)).exp() // ln(0.001) ≈ -6.908
}
