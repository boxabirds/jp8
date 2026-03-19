/// ADSR envelope generator.
/// Per the JP-8 spec §2.4.

#[derive(Clone, Copy, Debug, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 44100.0;

    fn make_env() -> Envelope {
        Envelope::new(SR)
    }

    #[test]
    fn idle_outputs_zero() {
        let mut env = make_env();
        for _ in 0..100 {
            assert_eq!(env.tick(), 0.0);
        }
        assert_eq!(env.stage, EnvStage::Idle);
    }

    #[test]
    fn attack_reaches_one() {
        let mut env = make_env();
        env.set_adsr(0.01, 0.3, 0.7, 0.5);
        env.gate_on();

        let mut reached = false;
        for _ in 0..44100 {
            env.tick();
            if env.stage == EnvStage::Decay {
                reached = true;
                break;
            }
        }
        assert!(reached, "Attack should transition to Decay");
        assert!((env.level - 1.0).abs() < 0.01);
    }

    #[test]
    fn attack_linear() {
        let mut env = make_env();
        let attack_time = 0.01; // 10ms = 441 samples
        env.set_adsr(attack_time, 0.3, 0.7, 0.5);
        env.gate_on();

        let half_samples = (attack_time * SR / 2.0) as usize;
        for _ in 0..half_samples {
            env.tick();
        }
        assert!((env.level - 0.5).abs() < 0.05,
            "At midpoint of attack, level should be ~0.5, got {}", env.level);
    }

    #[test]
    fn attack_instant_below_threshold() {
        let mut env = make_env();
        env.set_adsr(0.0005, 0.3, 0.7, 0.5); // below 0.001
        env.gate_on();
        env.tick();
        assert!(env.level >= 1.0, "Instant attack should reach 1.0 in 1 tick");
        assert_eq!(env.stage, EnvStage::Decay);
    }

    #[test]
    fn decay_reaches_sustain() {
        let mut env = make_env();
        env.set_adsr(0.001, 0.1, 0.6, 0.5);
        env.gate_on();

        // Run through attack + decay
        for _ in 0..44100 {
            env.tick();
            if env.stage == EnvStage::Sustain {
                break;
            }
        }
        assert_eq!(env.stage, EnvStage::Sustain);
        assert!((env.level - 0.6).abs() < 0.002, "Should converge to sustain, got {}", env.level);
    }

    #[test]
    fn decay_exponential_shape() {
        let mut env = make_env();
        env.set_adsr(0.0005, 2.0, 0.0, 0.5); // instant attack, very long decay to 0
        env.gate_on();
        env.tick(); // instant attack → level=1.0, stage=Decay

        // Early in decay, exponential curve should be above where linear would be
        // After 10% of decay time (0.2s), linear would be at 0.9; exponential should be higher
        let early_samples = (0.2 * SR) as usize;
        for _ in 0..early_samples {
            env.tick();
        }
        // With 2s decay time, exponential at 0.2s should still be well above 0.5
        assert!(env.level > 0.5, "Exponential decay should bow above linear at early time, got {}", env.level);
    }

    #[test]
    fn sustain_holds_level() {
        let mut env = make_env();
        env.set_adsr(0.0005, 0.01, 0.7, 0.5);
        env.gate_on();

        // Run through attack + decay to sustain
        for _ in 0..44100 {
            env.tick();
            if env.stage == EnvStage::Sustain { break; }
        }
        assert_eq!(env.stage, EnvStage::Sustain);

        let level = env.level;
        for _ in 0..1000 {
            env.tick();
            assert!((env.level - level).abs() < 1e-6, "Sustain should hold constant");
        }
    }

    #[test]
    fn release_reaches_idle() {
        let mut env = make_env();
        env.set_adsr(0.0005, 0.01, 0.7, 0.1);
        env.gate_on();

        // Reach sustain
        for _ in 0..44100 {
            env.tick();
            if env.stage == EnvStage::Sustain { break; }
        }

        env.gate_off();
        assert_eq!(env.stage, EnvStage::Release);

        for _ in 0..44100 {
            env.tick();
            if env.stage == EnvStage::Idle { break; }
        }
        assert_eq!(env.stage, EnvStage::Idle);
        assert_eq!(env.level, 0.0);
    }

    #[test]
    fn release_from_sustain() {
        let mut env = make_env();
        env.set_adsr(0.0005, 0.01, 0.6, 0.5);
        env.gate_on();

        for _ in 0..44100 {
            env.tick();
            if env.stage == EnvStage::Sustain { break; }
        }
        let level_before = env.level;
        env.gate_off();
        let level_after = env.tick();
        // Release should start from sustain level (close to it)
        assert!((level_before - 0.6).abs() < 0.01);
        assert!(level_after < level_before, "Release should decrease level");
    }

    #[test]
    fn retrigger_during_decay() {
        let mut env = make_env();
        env.set_adsr(0.0005, 0.5, 0.3, 0.5);
        env.gate_on();
        env.tick(); // attack → peak

        // Tick into decay
        for _ in 0..1000 {
            env.tick();
        }
        assert_eq!(env.stage, EnvStage::Decay);
        let level_before = env.level;
        assert!(level_before < 1.0 && level_before > 0.3);

        // Retrigger
        env.gate_on();
        assert_eq!(env.stage, EnvStage::Attack);
        // Level should NOT reset to 0 — carries over
        assert!(env.level > 0.0, "Level should carry over on retrigger, got {}", env.level);
    }

    #[test]
    fn retrigger_during_release() {
        let mut env = make_env();
        env.set_adsr(0.0005, 0.01, 0.7, 0.5);
        env.gate_on();
        for _ in 0..44100 {
            env.tick();
            if env.stage == EnvStage::Sustain { break; }
        }
        env.gate_off();
        // Tick a bit into release
        for _ in 0..100 {
            env.tick();
        }
        assert_eq!(env.stage, EnvStage::Release);
        let level = env.level;
        assert!(level > 0.0);

        env.gate_on();
        assert_eq!(env.stage, EnvStage::Attack);
        assert!(env.level > 0.0, "Level should carry over from release");
    }

    #[test]
    fn state_machine_transitions() {
        let mut env = make_env();
        env.set_adsr(0.0005, 0.01, 0.5, 0.01);

        // Idle → Attack
        assert_eq!(env.stage, EnvStage::Idle);
        env.gate_on();
        assert_eq!(env.stage, EnvStage::Attack);

        // Attack → Decay
        env.tick();
        assert_eq!(env.stage, EnvStage::Decay);

        // Decay → Sustain
        for _ in 0..44100 {
            env.tick();
            if env.stage == EnvStage::Sustain { break; }
        }
        assert_eq!(env.stage, EnvStage::Sustain);

        // Sustain → Release
        env.gate_off();
        assert_eq!(env.stage, EnvStage::Release);

        // Release → Idle
        for _ in 0..44100 {
            env.tick();
            if env.stage == EnvStage::Idle { break; }
        }
        assert_eq!(env.stage, EnvStage::Idle);

        // Attack → Release (gate_off during attack)
        env.set_adsr(1.0, 0.3, 0.7, 0.5); // slow attack
        env.gate_on();
        env.tick();
        assert_eq!(env.stage, EnvStage::Attack);
        env.gate_off();
        assert_eq!(env.stage, EnvStage::Release);

        // Idle → Idle (gate_off when idle is no-op)
        for _ in 0..44100 { env.tick(); if env.stage == EnvStage::Idle { break; } }
        env.gate_off();
        assert_eq!(env.stage, EnvStage::Idle);

        // Decay → Release
        env.set_adsr(0.0005, 1.0, 0.5, 0.01); // slow decay
        env.gate_on();
        env.tick(); // instant attack → decay
        assert_eq!(env.stage, EnvStage::Decay);
        env.gate_off();
        assert_eq!(env.stage, EnvStage::Release);
    }
}
