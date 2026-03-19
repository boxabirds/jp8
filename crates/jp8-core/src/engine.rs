/// Top-level JP-8 engine.
/// Global LFO (single phase shared across all voices, per the real JP-8).

use crate::allocator::VoiceAllocator;
use crate::arpeggiator::{ArpMode, Arpeggiator};
use crate::chorus::{ChorusMode, StereoChorus};
use crate::lfo::{Lfo, LfoWave};
use crate::params::{EngineParams, PARAM_COUNT};
use crate::voice::Voice;

const NUM_VOICES: usize = 8;

pub struct Engine {
    voices: [Voice; NUM_VOICES],
    chorus: StereoChorus,
    voice_allocator: VoiceAllocator,
    arp: Arpeggiator,
    pub params: EngineParams,
    lfo: Lfo,
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
            arp: Arpeggiator::new(sample_rate),
            params: EngineParams::default_patch(),
            lfo: Lfo::new(),
            sample_rate,
            inv_sample_rate: 1.0 / sample_rate,
        }
    }

    pub fn note_on(&mut self, note: u8, velocity: u8) {
        if self.arp.note_on(note) {
            // Arp is handling this note — don't trigger voice directly
            return;
        }
        self.trigger_voice(note, velocity);
    }

    pub fn note_off(&mut self, note: u8) {
        if self.arp.note_off(note) {
            return;
        }
        self.release_voice(note);
    }

    pub fn all_notes_off(&mut self) {
        self.arp.all_off();
        self.voice_allocator.all_off();
        for voice in &mut self.voices {
            voice.note_off();
        }
    }

    fn trigger_voice(&mut self, note: u8, velocity: u8) {
        if self.params.assign_mode == 2 {
            for (i, voice) in self.voices.iter_mut().enumerate() {
                voice.note_on(note, velocity, &self.params);
                let detune = (i as f32 - 3.5) * self.voice_allocator.unison_detune * 0.1;
                let factor = 2.0f32.powf(detune / 12.0);
                voice.vco1.target_freq *= factor;
                voice.vco2.target_freq *= factor;
            }
            for i in 0..NUM_VOICES {
                self.voice_allocator.mark_active(i, note);
            }
        } else {
            let idx = self.voice_allocator.note_on(note);
            self.voices[idx].note_on(note, velocity, &self.params);
        }
    }

    fn release_voice(&mut self, note: u8) {
        let mut released = [0usize; 8];
        let count = self.voice_allocator.note_off(note, &mut released);
        for i in 0..count {
            self.voices[released[i]].note_off();
        }
    }

    pub fn voices_active_count(&self) -> u32 {
        self.voices.iter().filter(|v| v.is_active()).count() as u32
    }

    /// Apply parameters from SAB float array (40 slots).
    pub fn apply_params(&mut self, raw: &[f32; PARAM_COUNT]) {
        self.params.vco1_wave_flags = raw[0] as u8;
        self.params.vco1_range = (raw[1] as i8).clamp(-2, 2);
        self.params.vco1_pw = raw[2].clamp(0.05, 0.95);
        self.params.vco1_level = raw[3].clamp(0.0, 1.0);
        self.params.vco2_wave_flags = raw[4] as u8;
        self.params.vco2_range = (raw[5] as i8).clamp(-2, 2);
        self.params.vco2_pw = raw[6].clamp(0.05, 0.95);
        self.params.vco2_level = raw[7].clamp(0.0, 1.0);
        self.params.vco2_detune = raw[8].clamp(-1.0, 1.0);
        self.params.cross_mod = raw[9].clamp(0.0, 1.0);
        self.params.noise_level = raw[10].clamp(0.0, 1.0);
        self.params.sub_osc_level = raw[11].clamp(0.0, 1.0);
        self.params.filter_cutoff = raw[12].clamp(20.0, 20000.0);
        self.params.filter_resonance = raw[13].clamp(0.0, 1.0);
        self.params.filter_env_depth = raw[14].clamp(-1.0, 1.0);
        self.params.filter_key_track = raw[15].clamp(0.0, 1.0);
        self.params.hpf_cutoff = raw[16].clamp(20.0, 20000.0);
        self.params.env1_attack = raw[17].max(0.001);
        self.params.env1_decay = raw[18].max(0.001);
        self.params.env1_sustain = raw[19].clamp(0.0, 1.0);
        self.params.env1_release = raw[20].max(0.001);
        self.params.env1_to_vca = raw[21] > 0.5;
        self.params.env2_attack = raw[22].max(0.001);
        self.params.env2_decay = raw[23].max(0.001);
        self.params.env2_sustain = raw[24].clamp(0.0, 1.0);
        self.params.env2_release = raw[25].max(0.001);
        self.params.lfo_rate = raw[26].clamp(0.1, 30.0);
        self.params.lfo_waveform = raw[27] as u8;
        self.params.lfo_pitch_depth = raw[28].clamp(0.0, 1.0);
        self.params.lfo_filter_depth = raw[29].clamp(0.0, 1.0);
        self.params.lfo_pwm_depth = raw[30].clamp(0.0, 1.0);
        self.params.lfo_delay = raw[31].clamp(0.0, 5.0);
        self.params.chorus_mode = raw[32] as u8;
        self.params.master_volume = raw[33].clamp(0.0, 1.0);
        self.params.assign_mode = raw[34] as u8;
        self.params.portamento = raw[35].clamp(0.0, 5.0);
        self.params.arp_mode = raw[36] as u8;
        self.params.arp_range = (raw[37] as u8).clamp(1, 4);
        self.params.arp_tempo = raw[38].clamp(30.0, 300.0);
        // 39 reserved

        // Update envelope parameters on all voices
        for voice in &mut self.voices {
            voice.env1.set_adsr(
                self.params.env1_attack, self.params.env1_decay,
                self.params.env1_sustain, self.params.env1_release,
            );
            voice.env2.set_adsr(
                self.params.env2_attack, self.params.env2_decay,
                self.params.env2_sustain, self.params.env2_release,
            );
        }

        // Global LFO config
        self.lfo.rate = self.params.lfo_rate;
        self.lfo.waveform = match self.params.lfo_waveform {
            0 => LfoWave::Sine,
            1 => LfoWave::Triangle,
            2 => LfoWave::Saw,
            3 => LfoWave::Square,
            4 => LfoWave::SampleAndHold,
            _ => LfoWave::Sine,
        };
        self.lfo.depth = 1.0; // depth applied per-destination in voice

        // Arpeggiator
        self.arp.mode = match self.params.arp_mode {
            0 => ArpMode::Off,
            1 => ArpMode::Up,
            2 => ArpMode::Down,
            3 => ArpMode::UpDown,
            _ => ArpMode::Off,
        };
        self.arp.range_octaves = self.params.arp_range;
        self.arp.set_tempo(self.params.arp_tempo);

        // Chorus mode
        self.chorus.mode = match self.params.chorus_mode {
            0 => ChorusMode::Off,
            1 => ChorusMode::Mode1,
            2 => ChorusMode::Mode2,
            _ => ChorusMode::Mode12,
        };
    }

    /// Render stereo interleaved frames.
    pub fn render(&mut self, output: &mut [f32]) {
        let frames = output.len() / 2;
        let inv_sr = self.inv_sample_rate;

        // Collect arp events for entire block first (avoids borrow conflicts)
        const MAX_ARP_EVENTS: usize = 8;
        let mut arp_events: [(u8, u8); MAX_ARP_EVENTS] = [(0, 0); MAX_ARP_EVENTS];
        let mut arp_event_count = 0;

        if self.arp.is_active() {
            for frame in 0..frames {
                let (on, off) = self.arp.tick();
                if (on > 0 || off > 0) && arp_event_count < MAX_ARP_EVENTS {
                    arp_events[arp_event_count] = (on, off);
                    arp_event_count += 1;
                }
            }
        }

        for i in 0..arp_event_count {
            let (on, off) = arp_events[i];
            if off > 0 { self.release_voice(off); }
            if on > 0 { self.trigger_voice(on, 100); }
        }

        let params = &self.params;

        for frame in 0..frames {
            let lfo_out = self.lfo.tick(inv_sr);
            let mut mono_sum: f32 = 0.0;

            for (i, voice) in self.voices.iter_mut().enumerate() {
                if voice.is_active() {
                    let sample = voice.render_sample(inv_sr, params, lfo_out);
                    mono_sum += sample;
                    self.voice_allocator.update_env_level(i, voice.env2.level);
                }
            }

            let (left, right) = self.chorus.tick(mono_sum);

            let vol = params.master_volume;
            let idx = frame * 2;
            output[idx] = left * vol;
            output[idx + 1] = right * vol;
        }
    }

    /// Test tone: pure 440 Hz sine, bypasses all DSP. For debugging audio path.
    pub fn render_test_tone(&mut self, output: &mut [f32]) {
        let frames = output.len() / 2;
        let inv_sr = self.inv_sample_rate;
        static mut TEST_PHASE: f32 = 0.0;

        for frame in 0..frames {
            let phase = unsafe { TEST_PHASE };
            let sample = (phase * core::f32::consts::TAU).sin() * 0.3;
            let idx = frame * 2;
            output[idx] = sample;
            output[idx + 1] = sample;
            unsafe {
                TEST_PHASE += 440.0 * inv_sr;
                if TEST_PHASE >= 1.0 { TEST_PHASE -= 1.0; }
            }
        }
    }
}
