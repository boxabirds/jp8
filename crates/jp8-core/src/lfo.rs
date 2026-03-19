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
