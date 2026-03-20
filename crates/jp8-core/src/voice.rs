/// Per-voice state.
/// Fixes from audit:
///  - Cross-mod direction: VCO2→VCO1 (not VCO1→VCO2)
///  - LFO→PWM applied
///  - HPF post-VCF
///  - Sub-oscillator on VCO1
///  - Portamento
///  - ENV-1→VCA optional routing
///  - Global LFO output passed in from engine (not per-voice)

use crate::bubble::BubbleOscillator;
use crate::envelope::Envelope;
use crate::filter::{HighPass, IR3109};
use crate::oscillator::{NoiseGen, Oscillator};
use crate::params::EngineParams;
use crate::waveguide::Waveguide;

pub struct Voice {
    pub vco1: Oscillator,
    pub vco2: Oscillator,
    pub noise: NoiseGen,
    pub bubble: BubbleOscillator,
    pub waveguide: Waveguide,
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
            bubble: BubbleOscillator::new(sample_rate),
            waveguide: Waveguide::new(sample_rate),
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

        // Waveguide source: set pitch and excite
        if params.source_mode == 2 {
            self.waveguide.set_pitch(freq);
            self.waveguide.set_params(params.wg_body, params.wg_brightness, params.wg_body_mix);
            self.waveguide.excite(params.wg_excitation, self.velocity);
        }

        // Trigger note-based bubbles if enabled
        if params.bubble_enable > 0 && params.bubble_level > 0.0 {
            self.bubble.set_params(params.bubble_rate, params.bubble_min_size, params.bubble_max_size);
            self.bubble.trigger_note(freq, self.velocity);
        }

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

    /// Render one sample. `global_lfo` and `chaos_out` are global modulation signals in [-1, 1].
    #[inline(always)]
    pub fn render_sample(&mut self, inv_sr: f32, params: &EngineParams, global_lfo: f32, chaos_out: f32) -> f32 {
        if !self.env2.is_active() {
            return 0.0;
        }

        // LFO delay fade-in
        if self.lfo_delay_level < 1.0 {
            self.lfo_delay_level = (self.lfo_delay_level + self.lfo_delay_rate).min(1.0);
        }
        let lfo = global_lfo * self.lfo_delay_level;

        // LFO + Chaos → pitch modulation
        let pitch_mod = lfo * params.lfo_pitch_depth
            + chaos_out * params.chaos_to_pitch;

        // Bubble oscillator (ticks continuously for ambient, plus note-triggered)
        let bubble_out = if params.bubble_enable > 0 && params.bubble_level > 0.0 {
            self.bubble.tick() * params.bubble_level
        } else {
            0.0
        };

        let noise_out = self.noise.next();

        // Source dispatch: BLEP oscillators or waveguide
        let mix = if params.source_mode == 2 {
            // Waveguide source — replaces VCO1+VCO2
            // Boost to match VCO output levels (~0.8 per VCO)
            let wg_out = self.waveguide.tick() * 3.0;
            wg_out + noise_out * params.noise_level + bubble_out
        } else {
            // BLEP oscillators (default, source_mode == 0)
            // LFO + Chaos → PWM modulation
            let pwm_mod = lfo * params.lfo_pwm_depth * 0.4
                + chaos_out * params.chaos_to_pwm * 0.4;
            self.vco1.pulse_width = (params.vco1_pw + pwm_mod).clamp(0.05, 0.95);
            self.vco2.pulse_width = (params.vco2_pw + pwm_mod).clamp(0.05, 0.95);

            // VCO2 first (needed for cross-mod into VCO1)
            let base_freq2 = self.vco2.target_freq;
            self.vco2.target_freq = base_freq2 * (1.0 + pitch_mod * 0.1);
            let vco2_out = self.vco2.tick(inv_sr, params.vco2_wave_flags);
            self.vco2.target_freq = base_freq2;

            // VCO1 with cross-mod FROM VCO2
            let xmod = params.cross_mod * vco2_out;
            let base_freq1 = self.vco1.target_freq;
            self.vco1.target_freq = base_freq1 * (1.0 + pitch_mod * 0.1);
            let vco1_out = self.vco1.tick_with_fm(inv_sr, params.vco1_wave_flags, xmod * inv_sr);
            self.vco1.target_freq = base_freq1;

            // Sub-oscillator
            let sub_out = self.vco1.sub_oscillator() * params.sub_osc_level;

            vco1_out * params.vco1_level
                + vco2_out * params.vco2_level
                + sub_out
                + noise_out * params.noise_level
                + bubble_out
        };

        // ENV-1 → filter cutoff modulation
        let env1_out = self.env1.tick();
        let base_cutoff = params.filter_cutoff;
        let key_track = params.filter_key_track * (self.note as f32 - 60.0) * 50.0;
        let lfo_filter_mod = lfo * params.lfo_filter_depth * base_cutoff;
        let chaos_filter_mod = chaos_out * params.chaos_to_filter * base_cutoff;
        let env_mod = env1_out * params.filter_env_depth * base_cutoff;
        let cutoff = (base_cutoff + env_mod + key_track + lfo_filter_mod + chaos_filter_mod).clamp(20.0, 20000.0);

        self.filter.set_cutoff(cutoff);
        self.filter.resonance = params.filter_resonance;
        let filtered = self.filter.tick(mix);

        // HPF post-VCF
        self.hpf.set_cutoff(params.hpf_cutoff);
        let hpf_out = self.hpf.tick(filtered);

        // VCA: waveguide handles its own decay, so bypass ENV-2
        if params.source_mode == 2 {
            // Still tick ENV-2 so is_active() eventually returns false (voice cleanup)
            self.env2.tick();
            hpf_out * self.velocity
        } else {
            let env2_out = self.env2.tick();
            let vca_env = if params.env1_to_vca {
                env2_out * env1_out
            } else {
                env2_out
            };
            hpf_out * vca_env * self.velocity
        }
    }

    pub fn is_active(&self) -> bool {
        self.env2.is_active()
    }
}

#[inline(always)]
fn midi_to_hz(note: u8) -> f32 {
    440.0 * 2.0f32.powf((note as f32 - 69.0) / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 44100.0;
    const INV_SR: f32 = 1.0 / 44100.0;

    fn default_params() -> EngineParams {
        EngineParams::default_patch()
    }

    #[test]
    fn silent_when_inactive() {
        let mut v = Voice::new(SR, 0);
        let params = default_params();
        for _ in 0..100 {
            assert_eq!(v.render_sample(INV_SR, &params, 0.0, 0.0), 0.0);
        }
    }

    #[test]
    fn produces_audio_after_note_on() {
        let mut v = Voice::new(SR, 0);
        let params = default_params();
        v.note_on(60, 100, &params);
        let mut has_nonzero = false;
        for _ in 0..441 {
            if v.render_sample(INV_SR, &params, 0.0, 0.0).abs() > 1e-6 {
                has_nonzero = true;
                break;
            }
        }
        assert!(has_nonzero, "Voice should produce audio after note_on");
    }

    #[test]
    fn note_off_eventually_silences() {
        let mut v = Voice::new(SR, 0);
        let mut params = default_params();
        params.env2_release = 0.05; // short release
        params.chorus_mode = 0; // chorus off to avoid delay tail
        v.note_on(60, 100, &params);
        // Play for a bit
        for _ in 0..4410 {
            v.render_sample(INV_SR, &params, 0.0, 0.0);
        }
        v.note_off();
        // Should silence within release time + generous margin
        // env2 release coeff targets ~0.001 after release_time
        for _ in 0..(SR as usize) {
            v.render_sample(INV_SR, &params, 0.0, 0.0);
        }
        assert!(!v.is_active(), "Voice should be inactive after sufficient release time");
    }

    #[test]
    fn velocity_scales_output() {
        let params = default_params();

        let mut v_loud = Voice::new(SR, 0);
        v_loud.note_on(60, 127, &params);
        let mut rms_loud = 0.0f32;
        for _ in 0..4410 {
            let s = v_loud.render_sample(INV_SR, &params, 0.0, 0.0);
            rms_loud += s * s;
        }

        let mut v_quiet = Voice::new(SR, 1);
        v_quiet.note_on(60, 30, &params);
        let mut rms_quiet = 0.0f32;
        for _ in 0..4410 {
            let s = v_quiet.render_sample(INV_SR, &params, 0.0, 0.0);
            rms_quiet += s * s;
        }

        assert!(rms_loud > rms_quiet, "vel=127 should be louder than vel=30");
    }

    #[test]
    fn env1_to_vca_reduces_output() {
        let mut params_off = default_params();
        params_off.env1_to_vca = false;
        params_off.env1_sustain = 0.3;

        let mut params_on = params_off.clone();
        params_on.env1_to_vca = true;

        let mut v1 = Voice::new(SR, 0);
        v1.note_on(60, 100, &params_off);
        let mut rms_off = 0.0f32;
        // Reach sustain
        for _ in 0..22050 {
            let s = v1.render_sample(INV_SR, &params_off, 0.0, 0.0);
            rms_off += s * s;
        }

        let mut v2 = Voice::new(SR, 1);
        v2.note_on(60, 100, &params_on);
        let mut rms_on = 0.0f32;
        for _ in 0..22050 {
            let s = v2.render_sample(INV_SR, &params_on, 0.0, 0.0);
            rms_on += s * s;
        }

        assert!(rms_on < rms_off, "env1_to_vca with low sustain should reduce output");
    }

    #[test]
    fn cross_mod_changes_timbre() {
        let mut params_no_xmod = default_params();
        params_no_xmod.cross_mod = 0.0;

        let mut params_xmod = default_params();
        params_xmod.cross_mod = 0.5;

        let mut v1 = Voice::new(SR, 0);
        v1.note_on(60, 100, &params_no_xmod);
        let mut rms1 = 0.0f32;
        for _ in 0..4410 {
            let s = v1.render_sample(INV_SR, &params_no_xmod, 0.0, 0.0);
            rms1 += s * s;
        }

        let mut v2 = Voice::new(SR, 1);
        v2.note_on(60, 100, &params_xmod);
        let mut rms2 = 0.0f32;
        for _ in 0..4410 {
            let s = v2.render_sample(INV_SR, &params_xmod, 0.0, 0.0);
            rms2 += s * s;
        }

        assert!((rms1 - rms2).abs() > 0.001, "Cross-mod should change timbre (RMS differs)");
    }

    #[test]
    fn lfo_delay_ramps() {
        let mut params = default_params();
        params.lfo_delay = 1.0;
        params.lfo_pitch_depth = 1.0;

        let mut v = Voice::new(SR, 0);
        v.note_on(60, 100, &params);

        // At sample 0, LFO delay level should be 0 (no LFO effect)
        let _s0 = v.render_sample(INV_SR, &params, 1.0, 0.0);

        // After 1 second worth of samples, LFO delay should be ~1.0
        for _ in 1..44100 {
            v.render_sample(INV_SR, &params, 1.0, 0.0);
        }
        // The lfo_delay_level should be close to 1.0 now
        assert!(v.lfo_delay_level > 0.95, "LFO delay should ramp to ~1.0 after delay time");
    }

    #[test]
    fn no_nan_full_chain() {
        let params = default_params();
        let mut v = Voice::new(SR, 0);
        v.note_on(60, 100, &params);
        for _ in 0..44100 {
            let s = v.render_sample(INV_SR, &params, 0.5, 0.0);
            assert!(!s.is_nan(), "Voice produced NaN");
            assert!(!s.is_infinite(), "Voice produced Inf");
        }
    }
}
