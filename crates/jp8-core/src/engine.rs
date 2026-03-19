/// Top-level JP-8 engine: voices + chorus + output.
/// Per the JP-8 spec §2.1.

use crate::allocator::{AssignMode, VoiceAllocator};
use crate::chorus::{ChorusMode, StereoChorus};
use crate::lfo::LfoWave;
use crate::params::{EngineParams, PARAM_COUNT};
use crate::voice::Voice;

const NUM_VOICES: usize = 8;

pub struct Engine {
    voices: [Voice; NUM_VOICES],
    chorus: StereoChorus,
    voice_allocator: VoiceAllocator,
    pub params: EngineParams,
    sample_rate: f32,
    inv_sample_rate: f32,
}

impl Engine {
    pub fn new(sample_rate: f32) -> Self {
        let voices = core::array::from_fn(|i| Voice::new(sample_rate, i));

        Self {
            voices,
            chorus: StereoChorus::new(sample_rate),
            voice_allocator: VoiceAllocator::new(),
            params: EngineParams::default_patch(),
            sample_rate,
            inv_sample_rate: 1.0 / sample_rate,
        }
    }

    pub fn note_on(&mut self, note: u8, velocity: u8) {
        let voice_idx = self.voice_allocator.note_on(note);
        self.voices[voice_idx].note_on(note, velocity, &self.params);

        // For unison mode, trigger all voices with detuning
        if matches!(self.params.assign_mode, 0) {
            // Poly mode — single voice
        } else if self.params.assign_mode == 2 {
            // Unison — trigger all with detune
            for (i, voice) in self.voices.iter_mut().enumerate() {
                voice.note_on(note, velocity, &self.params);
                let detune = (i as f32 - 3.5) * self.voice_allocator.unison_detune * 0.1;
                voice.vco1.freq *= 2.0f32.powf(detune / 12.0);
                voice.vco2.freq *= 2.0f32.powf(detune / 12.0);
            }
        }
    }

    pub fn note_off(&mut self, note: u8) {
        let mut released = [0usize; 8];
        let count = self.voice_allocator.note_off(note, &mut released);
        for i in 0..count {
            self.voices[released[i]].note_off();
        }
    }

    pub fn all_notes_off(&mut self) {
        self.voice_allocator.all_off();
        for voice in &mut self.voices {
            voice.note_off();
        }
    }

    /// Apply parameters from SAB float array.
    pub fn apply_params(&mut self, raw: &[f32; PARAM_COUNT]) {
        self.params.vco1_waveform = raw[0] as u8;
        self.params.vco1_range = raw[1] as i8;
        self.params.vco1_pw = raw[2].clamp(0.05, 0.95);
        self.params.vco1_level = raw[3].clamp(0.0, 1.0);
        self.params.vco2_waveform = raw[4] as u8;
        self.params.vco2_range = raw[5] as i8;
        self.params.vco2_pw = raw[6].clamp(0.05, 0.95);
        self.params.vco2_level = raw[7].clamp(0.0, 1.0);
        self.params.vco2_detune = raw[8].clamp(-1.0, 1.0);
        self.params.cross_mod = raw[9].clamp(0.0, 1.0);
        self.params.noise_level = raw[10].clamp(0.0, 1.0);
        self.params.filter_cutoff = raw[11].clamp(20.0, 20000.0);
        self.params.filter_resonance = raw[12].clamp(0.0, 1.0);
        self.params.filter_env_depth = raw[13].clamp(-1.0, 1.0);
        self.params.filter_key_track = raw[14].clamp(0.0, 1.0);
        self.params.env1_attack = raw[15].max(0.001);
        self.params.env1_decay = raw[16].max(0.001);
        self.params.env1_sustain = raw[17].clamp(0.0, 1.0);
        self.params.env1_release = raw[18].max(0.001);
        self.params.env2_attack = raw[19].max(0.001);
        self.params.env2_decay = raw[20].max(0.001);
        self.params.env2_sustain = raw[21].clamp(0.0, 1.0);
        self.params.env2_release = raw[22].max(0.001);
        self.params.lfo_rate = raw[23].clamp(0.1, 30.0);
        self.params.lfo_waveform = raw[24] as u8;
        self.params.lfo_pitch_depth = raw[25].clamp(0.0, 1.0);
        self.params.lfo_filter_depth = raw[26].clamp(0.0, 1.0);
        self.params.lfo_pwm_depth = raw[27].clamp(0.0, 1.0);
        self.params.chorus_mode = raw[28] as u8;
        self.params.master_volume = raw[29].clamp(0.0, 1.0);
        self.params.assign_mode = raw[30] as u8;
        self.params.portamento = raw[31].clamp(0.0, 5.0);

        // Update envelope parameters on all voices
        for voice in &mut self.voices {
            voice.env1.set_adsr(
                self.params.env1_attack,
                self.params.env1_decay,
                self.params.env1_sustain,
                self.params.env1_release,
            );
            voice.env2.set_adsr(
                self.params.env2_attack,
                self.params.env2_decay,
                self.params.env2_sustain,
                self.params.env2_release,
            );
            voice.lfo.rate = self.params.lfo_rate;
            voice.lfo.waveform = match self.params.lfo_waveform {
                0 => LfoWave::Sine,
                1 => LfoWave::Triangle,
                2 => LfoWave::Saw,
                3 => LfoWave::Square,
                4 => LfoWave::SampleAndHold,
                _ => LfoWave::Sine,
            };
            voice.lfo.depth = self.params.lfo_filter_depth;
        }

        // Chorus mode
        self.chorus.mode = match self.params.chorus_mode {
            0 => ChorusMode::Off,
            1 => ChorusMode::Mode1,
            2 => ChorusMode::Mode2,
            _ => ChorusMode::Mode12,
        };
    }

    pub fn voices_active_count(&self) -> u32 {
        self.voices.iter().filter(|v| v.is_active()).count() as u32
    }

    /// Render `n` stereo interleaved frames into `output`.
    /// GUARANTEE: No heap allocations. No panics.
    pub fn render(&mut self, output: &mut [f32]) {
        let frames = output.len() / 2;
        let inv_sr = self.inv_sample_rate;
        let params = &self.params;

        for frame in 0..frames {
            // Accumulate all active voices into mono sum
            let mut mono_sum: f32 = 0.0;

            for (i, voice) in self.voices.iter_mut().enumerate() {
                if voice.is_active() {
                    let sample = voice.render_sample(inv_sr, params);
                    mono_sum += sample;
                    self.voice_allocator.update_env_level(i, voice.env2.level);
                }
            }

            // Apply chorus → stereo
            let (left, right) = self.chorus.tick(mono_sum);

            // Master volume and write to output
            let vol = params.master_volume;
            let idx = frame * 2;
            output[idx] = left * vol;
            output[idx + 1] = right * vol;
        }
    }
}
