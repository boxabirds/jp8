/// Per-voice state.
/// Fixes from audit:
///  - Cross-mod direction: VCO2→VCO1 (not VCO1→VCO2)
///  - LFO→PWM applied
///  - HPF post-VCF
///  - Sub-oscillator on VCO1
///  - Portamento
///  - ENV-1→VCA optional routing
///  - Global LFO output passed in from engine (not per-voice)

use crate::envelope::Envelope;
use crate::filter::{HighPass, IR3109};
use crate::oscillator::{NoiseGen, Oscillator};
use crate::params::EngineParams;

pub struct Voice {
    pub vco1: Oscillator,
    pub vco2: Oscillator,
    pub noise: NoiseGen,
    pub filter: IR3109,
    pub hpf: HighPass,
    pub env1: Envelope,     // → filter cutoff (optionally VCA)
    pub env2: Envelope,     // → VCA
    pub gate: bool,
    pub note: u8,
    pub velocity: f32,
    /// Per-voice LFO delay envelope: ramps 0→1 over lfo_delay seconds.
    lfo_delay_level: f32,
    lfo_delay_rate: f32,
    sample_rate: f32,
}

impl Voice {
    pub fn new(sample_rate: f32, voice_index: usize) -> Self {
        Self {
            vco1: Oscillator::new(),
            vco2: Oscillator::new(),
            noise: NoiseGen::new(42 + voice_index as u32 * 7),
            filter: IR3109::new(sample_rate),
            hpf: HighPass::new(sample_rate),
            env1: Envelope::new(sample_rate),
            env2: Envelope::new(sample_rate),
            gate: false,
            note: 0,
            velocity: 0.0,
            lfo_delay_level: 0.0,
            lfo_delay_rate: 0.0,
            sample_rate,
        }
    }

    pub fn note_on(&mut self, note: u8, velocity: u8, params: &EngineParams) {
        self.note = note;
        self.velocity = velocity as f32 / 127.0;
        self.gate = true;

        let freq = midi_to_hz(note);
        let range1 = 2.0f32.powi(params.vco1_range as i32);
        let range2 = 2.0f32.powi(params.vco2_range as i32);
        let detune_factor = 2.0f32.powf(params.vco2_detune / 12.0);

        if params.portamento > 0.001 {
            // Glide to new freq
            self.vco1.set_freq(freq * range1, 0.3);
            self.vco1.set_portamento(params.portamento, self.sample_rate);
            self.vco2.set_freq(freq * range2 * detune_factor, 0.3);
            self.vco2.set_portamento(params.portamento, self.sample_rate);
        } else {
            self.vco1.set_freq_immediate(freq * range1, 0.3);
            self.vco1.set_portamento(0.0, self.sample_rate);
            self.vco2.set_freq_immediate(freq * range2 * detune_factor, 0.3);
            self.vco2.set_portamento(0.0, self.sample_rate);
        }

        self.vco1.pulse_width = params.vco1_pw;
        self.vco2.pulse_width = params.vco2_pw;

        self.env1.gate_on();
        self.env2.gate_on();
        self.filter.reset();
        self.hpf.reset();

        // Reset LFO delay — ramps from 0 to 1
        self.lfo_delay_level = 0.0;
        if params.lfo_delay > 0.001 {
            self.lfo_delay_rate = 1.0 / (params.lfo_delay * self.sample_rate);
        } else {
            self.lfo_delay_level = 1.0;
            self.lfo_delay_rate = 0.0;
        }
    }

    pub fn note_off(&mut self) {
        self.gate = false;
        self.env1.gate_off();
        self.env2.gate_off();
    }

    /// Render one sample. `global_lfo` is the current global LFO output [-1, 1].
    #[inline(always)]
    pub fn render_sample(&mut self, inv_sr: f32, params: &EngineParams, global_lfo: f32) -> f32 {
        if !self.env2.is_active() {
            return 0.0;
        }

        // LFO delay fade-in
        if self.lfo_delay_level < 1.0 {
            self.lfo_delay_level = (self.lfo_delay_level + self.lfo_delay_rate).min(1.0);
        }
        let lfo = global_lfo * self.lfo_delay_level;

        // LFO → PWM modulation
        let pwm_mod = lfo * params.lfo_pwm_depth * 0.4; // ±0.4 range
        self.vco1.pulse_width = (params.vco1_pw + pwm_mod).clamp(0.05, 0.95);
        self.vco2.pulse_width = (params.vco2_pw + pwm_mod).clamp(0.05, 0.95);

        // LFO → pitch
        let pitch_mod = lfo * params.lfo_pitch_depth;

        // VCO2 first (needed for cross-mod into VCO1)
        let base_freq2 = self.vco2.target_freq;
        self.vco2.target_freq = base_freq2 * (1.0 + pitch_mod * 0.1);
        let vco2_out = self.vco2.tick(inv_sr, params.vco2_wave_flags);
        self.vco2.target_freq = base_freq2;

        // VCO1 with cross-mod FROM VCO2 (correct JP-8 direction: VCO2→VCO1)
        let xmod = params.cross_mod * vco2_out;
        let base_freq1 = self.vco1.target_freq;
        self.vco1.target_freq = base_freq1 * (1.0 + pitch_mod * 0.1);
        let vco1_out = self.vco1.tick_with_fm(inv_sr, params.vco1_wave_flags, xmod * inv_sr);
        self.vco1.target_freq = base_freq1;

        // Sub-oscillator (VCO1, one octave below, square)
        let sub_out = self.vco1.sub_oscillator() * params.sub_osc_level;

        // Mixer
        let noise_out = self.noise.next();
        let mix = vco1_out * params.vco1_level
            + vco2_out * params.vco2_level
            + sub_out
            + noise_out * params.noise_level;

        // ENV-1 → filter cutoff modulation
        let env1_out = self.env1.tick();
        let base_cutoff = params.filter_cutoff;
        let key_track = params.filter_key_track * (self.note as f32 - 60.0) * 50.0;
        let lfo_filter_mod = lfo * params.lfo_filter_depth * base_cutoff;
        let env_mod = env1_out * params.filter_env_depth * base_cutoff;
        let cutoff = (base_cutoff + env_mod + key_track + lfo_filter_mod).clamp(20.0, 20000.0);

        self.filter.set_cutoff(cutoff);
        self.filter.resonance = params.filter_resonance;
        let filtered = self.filter.tick(mix);

        // HPF post-VCF
        self.hpf.set_cutoff(params.hpf_cutoff);
        let hpf_out = self.hpf.tick(filtered);

        // VCA: ENV-2, optionally mixed with ENV-1
        let env2_out = self.env2.tick();
        let vca_env = if params.env1_to_vca {
            env2_out * env1_out
        } else {
            env2_out
        };

        hpf_out * vca_env * self.velocity
    }

    pub fn is_active(&self) -> bool {
        self.env2.is_active()
    }
}

#[inline(always)]
fn midi_to_hz(note: u8) -> f32 {
    440.0 * 2.0f32.powf((note as f32 - 69.0) / 12.0)
}
