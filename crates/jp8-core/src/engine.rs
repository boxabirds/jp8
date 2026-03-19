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

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 44100.0;
    const BLOCK: usize = 128;
    const STEREO_BLOCK: usize = BLOCK * 2;

    /// Factory patch params (copied from patches.ts for Rust-side testing).
    /// Each is a [f32; 40] matching the SAB layout.
    const FACTORY_PATCHES: &[[f32; PARAM_COUNT]] = &[
        // 0: Brass Ensemble
        [1.0, 0.0, 0.5, 0.8, 1.0, 0.0, 0.5, 0.8, 0.08, 0.0, 0.0, 0.0,
         2000.0, 0.15, 0.6, 0.5, 20.0, 0.05, 0.2, 0.7, 0.3, 0.0,
         0.01, 0.1, 0.8, 0.3, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 3.0, 0.7, 0.0, 0.0, 0.0, 1.0, 120.0, 0.0],
        // 1: Warm Pad
        [1.0, 0.0, 0.5, 0.7, 1.0, 0.0, 0.5, 0.7, 0.1, 0.0, 0.0, 0.0,
         3000.0, 0.2, 0.4, 0.3, 20.0, 0.8, 0.5, 0.8, 1.0, 0.0,
         0.5, 0.3, 0.9, 0.8, 3.0, 0.0, 0.0, 0.15, 0.0, 0.0, 3.0, 0.7, 0.0, 0.0, 0.0, 1.0, 120.0, 0.0],
        // 2: Bass
        [1.0, -1.0, 0.5, 0.9, 2.0, -1.0, 0.5, 0.7, 0.05, 0.0, 0.0, 0.5,
         800.0, 0.3, 0.7, 0.2, 20.0, 0.005, 0.15, 0.3, 0.15, 0.0,
         0.005, 0.1, 0.7, 0.2, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0, 0.0, 1.0, 120.0, 0.0],
        // 3: Strings
        [1.0, 0.0, 0.5, 0.6, 1.0, 0.0, 0.5, 0.6, 0.12, 0.0, 0.0, 0.0,
         4000.0, 0.1, 0.3, 0.4, 20.0, 0.5, 0.3, 0.8, 0.8, 0.0,
         0.4, 0.3, 0.9, 0.6, 4.0, 1.0, 0.0, 0.1, 0.0, 0.0, 3.0, 0.7, 0.0, 0.0, 0.0, 1.0, 120.0, 0.0],
        // 4: Lead
        [1.0, 0.0, 0.5, 0.9, 1.0, 0.0, 0.3, 0.7, 0.0, 0.15, 0.0, 0.0,
         5000.0, 0.25, 0.5, 0.5, 20.0, 0.01, 0.2, 0.6, 0.3, 0.0,
         0.01, 0.15, 0.8, 0.25, 6.0, 0.0, 0.0, 0.0, 0.1, 0.3, 0.0, 0.75, 0.0, 0.1, 0.0, 1.0, 120.0, 0.0],
        // 5: Sync Lead (approx — uses cross-mod since we don't have sync)
        [1.0, 0.0, 0.5, 0.8, 1.0, 1.0, 0.5, 0.6, 0.0, 0.4, 0.0, 0.0,
         6000.0, 0.2, 0.6, 0.5, 20.0, 0.01, 0.2, 0.5, 0.2, 0.0,
         0.01, 0.1, 0.8, 0.3, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.7, 0.0, 0.0, 0.0, 1.0, 120.0, 0.0],
        // 6: Keys
        [2.0, 0.0, 0.5, 0.8, 2.0, 0.0, 0.45, 0.6, 0.05, 0.0, 0.0, 0.0,
         3500.0, 0.1, 0.4, 0.5, 20.0, 0.005, 0.3, 0.5, 0.2, 0.0,
         0.005, 0.25, 0.6, 0.2, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.7, 0.0, 0.0, 0.0, 1.0, 120.0, 0.0],
        // 7: Ambient Sweep
        [1.0, 0.0, 0.5, 0.6, 1.0, 0.0, 0.5, 0.6, 0.15, 0.0, 0.1, 0.0,
         1500.0, 0.4, 0.7, 0.3, 20.0, 1.0, 1.0, 0.7, 2.0, 0.0,
         0.8, 0.5, 0.9, 1.5, 0.3, 0.0, 0.0, 0.5, 0.2, 1.0, 3.0, 0.6, 0.0, 0.0, 0.0, 1.0, 120.0, 0.0],
        // 8: Organ
        [2.0, 0.0, 0.5, 0.7, 2.0, 1.0, 0.5, 0.5, 0.0, 0.0, 0.0, 0.3,
         8000.0, 0.05, 0.2, 0.3, 20.0, 0.005, 0.05, 0.8, 0.1, 0.0,
         0.005, 0.05, 1.0, 0.1, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.7, 0.0, 0.0, 0.0, 1.0, 120.0, 0.0],
        // 9: Pluck
        [1.0, 0.0, 0.5, 0.8, 1.0, 0.0, 0.5, 0.5, 0.03, 0.0, 0.05, 0.0,
         6000.0, 0.15, 0.8, 0.5, 20.0, 0.001, 0.2, 0.0, 0.15, 0.0,
         0.001, 0.15, 0.0, 0.1, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.75, 0.0, 0.0, 0.0, 1.0, 120.0, 0.0],
        // 10: Fat Unison
        [1.0, 0.0, 0.5, 0.7, 1.0, 0.0, 0.5, 0.7, 0.1, 0.0, 0.0, 0.2,
         3000.0, 0.2, 0.5, 0.4, 20.0, 0.01, 0.2, 0.7, 0.3, 0.0,
         0.01, 0.15, 0.8, 0.3, 5.0, 0.0, 0.0, 0.1, 0.0, 0.0, 3.0, 0.5, 2.0, 0.0, 0.0, 1.0, 120.0, 0.0],
        // 11: PWM Strings
        [2.0, 0.0, 0.5, 0.8, 2.0, 0.0, 0.5, 0.8, 0.08, 0.0, 0.0, 0.0,
         5000.0, 0.1, 0.3, 0.4, 20.0, 0.4, 0.3, 0.8, 0.7, 0.0,
         0.3, 0.3, 0.9, 0.5, 2.0, 1.0, 0.0, 0.0, 0.6, 0.0, 3.0, 0.7, 0.0, 0.0, 0.0, 1.0, 120.0, 0.0],
        // 12: Filter Bass
        [1.0, -1.0, 0.5, 0.9, 1.0, -1.0, 0.5, 0.8, 0.03, 0.0, 0.0, 0.4,
         400.0, 0.5, 0.9, 0.1, 20.0, 0.001, 0.3, 0.1, 0.2, 0.0,
         0.001, 0.2, 0.6, 0.15, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0, 0.0, 1.0, 120.0, 0.0],
        // 13: Shimmer Pad
        [1.0, 1.0, 0.5, 0.5, 1.0, 1.0, 0.5, 0.5, 0.2, 0.0, 0.05, 0.0,
         6000.0, 0.15, 0.3, 0.5, 20.0, 1.0, 0.5, 0.9, 1.5, 0.0,
         0.8, 0.4, 0.85, 1.0, 1.5, 1.0, 0.0, 0.2, 0.3, 0.5, 3.0, 0.6, 0.0, 0.0, 0.0, 1.0, 120.0, 0.0],
        // 14: Reso Sweep
        [1.0, 0.0, 0.5, 0.8, 1.0, 0.0, 0.5, 0.6, 0.1, 0.0, 0.0, 0.0,
         500.0, 0.7, 0.9, 0.3, 20.0, 0.01, 0.8, 0.4, 0.5, 0.0,
         0.01, 0.1, 0.8, 0.4, 0.5, 0.0, 0.0, 0.4, 0.0, 0.0, 2.0, 0.7, 0.0, 0.0, 0.0, 1.0, 120.0, 0.0],
        // 15: Noise Hit
        [0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.9, 0.0,
         2000.0, 0.5, 0.8, 0.0, 20.0, 0.001, 0.15, 0.0, 0.1, 0.0,
         0.001, 0.1, 0.0, 0.08, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0, 0.0, 1.0, 120.0, 0.0],
    ];

    fn render_blocks(engine: &mut Engine, blocks: usize) -> Vec<f32> {
        let mut all = Vec::new();
        let mut buf = [0.0f32; STEREO_BLOCK];
        for _ in 0..blocks {
            engine.render(&mut buf);
            all.extend_from_slice(&buf);
        }
        all
    }

    fn has_nan_or_inf(samples: &[f32]) -> bool {
        samples.iter().any(|s| s.is_nan() || s.is_infinite())
    }

    fn has_nonzero(samples: &[f32]) -> bool {
        samples.iter().any(|s| s.abs() > 1e-10)
    }

    #[test]
    fn silence_without_notes() {
        let mut engine = Engine::new(SR);
        let out = render_blocks(&mut engine, 10);
        assert!(!has_nonzero(&out), "Should be silent with no notes");
    }

    #[test]
    fn note_on_produces_audio() {
        let mut engine = Engine::new(SR);
        engine.note_on(60, 100);
        let out = render_blocks(&mut engine, 5);
        assert!(has_nonzero(&out), "Should produce audio after note_on");
    }

    #[test]
    fn note_off_decays() {
        let mut engine = Engine::new(SR);
        engine.params.env2_release = 0.05;
        // Update envelope params on voices
        let raw = params_to_raw(&engine.params);
        engine.apply_params(&raw);

        engine.note_on(60, 100);
        render_blocks(&mut engine, 10);
        engine.note_off(60);
        let out = render_blocks(&mut engine, 100); // ~0.3 seconds
        // Last block should be near-silent
        let last_block = &out[out.len() - STEREO_BLOCK..];
        let max = last_block.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(max < 0.01, "Should decay to silence, max={max}");
    }

    #[test]
    fn apply_params_clamps() {
        let mut engine = Engine::new(SR);
        let mut raw = [0.0f32; PARAM_COUNT];
        raw[2] = -5.0;  // pw should clamp to 0.05
        raw[12] = 0.0;  // cutoff should clamp to 20
        raw[13] = 5.0;  // resonance should clamp to 1.0
        raw[17] = 0.0;  // attack should clamp to 0.001
        engine.apply_params(&raw);
        assert_eq!(engine.params.vco1_pw, 0.05);
        assert_eq!(engine.params.filter_cutoff, 20.0);
        assert_eq!(engine.params.filter_resonance, 1.0);
        assert_eq!(engine.params.env1_attack, 0.001);
    }

    #[test]
    fn all_notes_off_silences() {
        let mut engine = Engine::new(SR);
        engine.params.env2_release = 0.01;
        let raw = params_to_raw(&engine.params);
        engine.apply_params(&raw);

        for note in 60..68 {
            engine.note_on(note, 100);
        }
        render_blocks(&mut engine, 5);
        engine.all_notes_off();
        let out = render_blocks(&mut engine, 50);
        let last_block = &out[out.len() - STEREO_BLOCK..];
        let max = last_block.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(max < 0.001, "Should silence after all_notes_off, max={max}");
    }

    #[test]
    fn voices_active_count() {
        let mut engine = Engine::new(SR);
        assert_eq!(engine.voices_active_count(), 0);
        engine.note_on(60, 100);
        engine.note_on(64, 100);
        engine.note_on(67, 100);
        // Need to render so voices become active (env2 starts ticking)
        render_blocks(&mut engine, 1);
        assert_eq!(engine.voices_active_count(), 3);
    }

    #[test]
    fn unison_activates_all() {
        let mut engine = Engine::new(SR);
        let mut raw = params_to_raw(&engine.params);
        raw[34] = 2.0; // assign_mode = Unison
        engine.apply_params(&raw);
        engine.note_on(60, 100);
        render_blocks(&mut engine, 1);
        assert_eq!(engine.voices_active_count(), 8, "Unison should activate all 8 voices");
    }

    #[test]
    fn master_volume_scales() {
        let mut engine_loud = Engine::new(SR);
        engine_loud.params.master_volume = 1.0;
        engine_loud.note_on(60, 100);
        let out_loud = render_blocks(&mut engine_loud, 5);
        let peak_loud = out_loud.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

        let mut engine_quiet = Engine::new(SR);
        engine_quiet.params.master_volume = 0.3;
        engine_quiet.note_on(60, 100);
        let out_quiet = render_blocks(&mut engine_quiet, 5);
        let peak_quiet = out_quiet.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

        assert!(peak_loud > peak_quiet, "vol=1.0 peak ({peak_loud}) should exceed vol=0.3 ({peak_quiet})");
    }

    #[test]
    fn no_nan_all_factory_patches() {
        for (i, patch) in FACTORY_PATCHES.iter().enumerate() {
            let mut engine = Engine::new(SR);
            engine.apply_params(patch);
            engine.note_on(60, 100);
            let out = render_blocks(&mut engine, 10);
            assert!(!has_nan_or_inf(&out), "NaN/Inf in factory patch {i}");
        }
    }

    #[test]
    fn render_stereo_interleaved() {
        let mut engine = Engine::new(SR);
        engine.note_on(60, 100);
        let mut buf = [0.0f32; STEREO_BLOCK];
        engine.render(&mut buf);
        // With chorus on (default Mode12), L and R should differ
        let mut found_diff = false;
        for frame in 0..BLOCK {
            let l = buf[frame * 2];
            let r = buf[frame * 2 + 1];
            if (l - r).abs() > 1e-6 {
                found_diff = true;
                break;
            }
        }
        assert!(found_diff, "Stereo interleaved: L and R should differ with chorus on");
    }

    /// Helper: convert EngineParams back to raw f32 array for apply_params
    fn params_to_raw(p: &EngineParams) -> [f32; PARAM_COUNT] {
        let mut raw = [0.0f32; PARAM_COUNT];
        raw[0] = p.vco1_wave_flags as f32;
        raw[1] = p.vco1_range as f32;
        raw[2] = p.vco1_pw;
        raw[3] = p.vco1_level;
        raw[4] = p.vco2_wave_flags as f32;
        raw[5] = p.vco2_range as f32;
        raw[6] = p.vco2_pw;
        raw[7] = p.vco2_level;
        raw[8] = p.vco2_detune;
        raw[9] = p.cross_mod;
        raw[10] = p.noise_level;
        raw[11] = p.sub_osc_level;
        raw[12] = p.filter_cutoff;
        raw[13] = p.filter_resonance;
        raw[14] = p.filter_env_depth;
        raw[15] = p.filter_key_track;
        raw[16] = p.hpf_cutoff;
        raw[17] = p.env1_attack;
        raw[18] = p.env1_decay;
        raw[19] = p.env1_sustain;
        raw[20] = p.env1_release;
        raw[21] = if p.env1_to_vca { 1.0 } else { 0.0 };
        raw[22] = p.env2_attack;
        raw[23] = p.env2_decay;
        raw[24] = p.env2_sustain;
        raw[25] = p.env2_release;
        raw[26] = p.lfo_rate;
        raw[27] = p.lfo_waveform as f32;
        raw[28] = p.lfo_pitch_depth;
        raw[29] = p.lfo_filter_depth;
        raw[30] = p.lfo_pwm_depth;
        raw[31] = p.lfo_delay;
        raw[32] = p.chorus_mode as f32;
        raw[33] = p.master_volume;
        raw[34] = p.assign_mode as f32;
        raw[35] = p.portamento;
        raw[36] = p.arp_mode as f32;
        raw[37] = p.arp_range as f32;
        raw[38] = p.arp_tempo;
        raw[39] = 0.0;
        raw
    }
}
