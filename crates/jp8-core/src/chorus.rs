/// Stereo BBD chorus emulation.
/// Per the JP-8 spec §2.6.

const CHORUS_BUFFER_SIZE: usize = 2048;

#[derive(Clone, Copy, PartialEq)]
pub enum ChorusMode {
    Off,
    Mode1,   // ~0.513 Hz, ~1.5ms depth
    Mode2,   // ~0.863 Hz, ~2.3ms depth
    Mode12,  // Both — the classic JP-8 sound
}

pub struct StereoChorus {
    buffer_l: [f32; CHORUS_BUFFER_SIZE],
    buffer_r: [f32; CHORUS_BUFFER_SIZE],
    write_pos: usize,
    lfo_phase_1: f32,
    lfo_phase_2: f32,
    pub mode: ChorusMode,
    pub mix: f32,
    sample_rate: f32,
}

impl StereoChorus {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            buffer_l: [0.0; CHORUS_BUFFER_SIZE],
            buffer_r: [0.0; CHORUS_BUFFER_SIZE],
            write_pos: 0,
            lfo_phase_1: 0.0,
            lfo_phase_2: 0.25, // offset for stereo
            mode: ChorusMode::Mode12,
            mix: 0.5,
            sample_rate,
        }
    }

    /// Process one stereo sample pair. Returns (left, right).
    #[inline(always)]
    pub fn tick(&mut self, input: f32) -> (f32, f32) {
        if self.mode == ChorusMode::Off {
            return (input, input);
        }

        // Write input to both delay buffers
        self.buffer_l[self.write_pos] = input;
        self.buffer_r[self.write_pos] = input;

        let inv_sr = 1.0 / self.sample_rate;
        let (delay_l, delay_r) = match self.mode {
            ChorusMode::Off => unreachable!(),
            ChorusMode::Mode1 => {
                let (dl, dr) = self.chorus_1(inv_sr);
                (dl, dr)
            }
            ChorusMode::Mode2 => {
                let (dl, dr) = self.chorus_2(inv_sr);
                (dl, dr)
            }
            ChorusMode::Mode12 => {
                let (dl1, dr1) = self.chorus_1(inv_sr);
                let (dl2, dr2) = self.chorus_2(inv_sr);
                ((dl1 + dl2) * 0.5, (dr1 + dr2) * 0.5)
            }
        };

        self.write_pos = (self.write_pos + 1) % CHORUS_BUFFER_SIZE;

        let wet_l = delay_l;
        let wet_r = delay_r;
        let dry = input;

        (
            dry * (1.0 - self.mix) + wet_l * self.mix,
            dry * (1.0 - self.mix) + wet_r * self.mix,
        )
    }

    fn chorus_1(&mut self, inv_sr: f32) -> (f32, f32) {
        const RATE: f32 = 0.513;
        const DEPTH_MS: f32 = 1.5;

        self.lfo_phase_1 += RATE * inv_sr;
        if self.lfo_phase_1 >= 1.0 {
            self.lfo_phase_1 -= 1.0;
        }

        let lfo_l = (self.lfo_phase_1 * core::f32::consts::TAU).sin();
        let lfo_r = ((self.lfo_phase_1 + 0.5) * core::f32::consts::TAU).sin();

        let base_delay = DEPTH_MS * 0.001 * self.sample_rate;
        let delay_l = base_delay + lfo_l * base_delay * 0.5;
        let delay_r = base_delay + lfo_r * base_delay * 0.5;

        (
            self.read_interpolated(&self.buffer_l, delay_l),
            self.read_interpolated(&self.buffer_r, delay_r),
        )
    }

    fn chorus_2(&mut self, inv_sr: f32) -> (f32, f32) {
        const RATE: f32 = 0.863;
        const DEPTH_MS: f32 = 2.3;

        self.lfo_phase_2 += RATE * inv_sr;
        if self.lfo_phase_2 >= 1.0 {
            self.lfo_phase_2 -= 1.0;
        }

        let lfo_l = (self.lfo_phase_2 * core::f32::consts::TAU).sin();
        let lfo_r = ((self.lfo_phase_2 + 0.5) * core::f32::consts::TAU).sin();

        let base_delay = DEPTH_MS * 0.001 * self.sample_rate;
        let delay_l = base_delay + lfo_l * base_delay * 0.5;
        let delay_r = base_delay + lfo_r * base_delay * 0.5;

        (
            self.read_interpolated(&self.buffer_l, delay_l),
            self.read_interpolated(&self.buffer_r, delay_r),
        )
    }

    #[inline(always)]
    fn read_interpolated(&self, buffer: &[f32; CHORUS_BUFFER_SIZE], delay: f32) -> f32 {
        let pos = self.write_pos as f32 - delay;
        let pos = if pos < 0.0 {
            pos + CHORUS_BUFFER_SIZE as f32
        } else {
            pos
        };
        let idx = pos as usize;
        let frac = pos - idx as f32;
        let a = buffer[idx % CHORUS_BUFFER_SIZE];
        let b = buffer[(idx + 1) % CHORUS_BUFFER_SIZE];
        a + frac * (b - a)
    }

    pub fn reset(&mut self) {
        self.buffer_l = [0.0; CHORUS_BUFFER_SIZE];
        self.buffer_r = [0.0; CHORUS_BUFFER_SIZE];
        self.write_pos = 0;
        self.lfo_phase_1 = 0.0;
        self.lfo_phase_2 = 0.25;
    }
}
