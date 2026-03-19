/// Parameter definitions.
/// Expanded from spec §5.1 to include HPF, sub-osc, LFO delay, ENV1→VCA.
/// wave_flags: bit0=saw, bit1=pulse (additive mixing, not selector).

#[derive(Clone)]
pub struct EngineParams {
    // VCO-1
    pub vco1_wave_flags: u8,    // bit0=saw, bit1=pulse
    pub vco1_range: i8,         // -2..+2 octaves (16', 8', 4', 2')
    pub vco1_pw: f32,
    pub vco1_level: f32,
    // VCO-2
    pub vco2_wave_flags: u8,
    pub vco2_range: i8,
    pub vco2_pw: f32,
    pub vco2_level: f32,
    pub vco2_detune: f32,
    // Cross-mod: VCO2 → VCO1 (correct JP-8 direction)
    pub cross_mod: f32,
    pub noise_level: f32,
    // Sub-oscillator (VCO1, one octave below, square)
    pub sub_osc_level: f32,
    // Filter
    pub filter_cutoff: f32,
    pub filter_resonance: f32,
    pub filter_env_depth: f32,  // bipolar [-1, 1]
    pub filter_key_track: f32,
    pub hpf_cutoff: f32,        // highpass filter after VCF
    // ENV-1 (filter, optionally VCA)
    pub env1_attack: f32,
    pub env1_decay: f32,
    pub env1_sustain: f32,
    pub env1_release: f32,
    pub env1_to_vca: bool,      // route ENV-1 to VCA as well
    // ENV-2 (amp)
    pub env2_attack: f32,
    pub env2_decay: f32,
    pub env2_sustain: f32,
    pub env2_release: f32,
    // LFO (global)
    pub lfo_rate: f32,
    pub lfo_waveform: u8,
    pub lfo_pitch_depth: f32,
    pub lfo_filter_depth: f32,
    pub lfo_pwm_depth: f32,
    pub lfo_delay: f32,         // seconds to fade in LFO
    // Output
    pub chorus_mode: u8,
    pub master_volume: f32,
    pub assign_mode: u8,
    pub portamento: f32,
    // Arpeggiator
    pub arp_mode: u8,       // 0=Off, 1=Up, 2=Down, 3=UpDown
    pub arp_range: u8,      // 1–4 octaves
    pub arp_tempo: f32,     // BPM
}

impl EngineParams {
    pub fn default_patch() -> Self {
        Self {
            vco1_wave_flags: 1,   // saw on
            vco1_range: 0,
            vco1_pw: 0.5,
            vco1_level: 0.8,
            vco2_wave_flags: 1,
            vco2_range: 0,
            vco2_pw: 0.5,
            vco2_level: 0.8,
            vco2_detune: 0.0,
            cross_mod: 0.0,
            noise_level: 0.0,
            sub_osc_level: 0.0,
            filter_cutoff: 8000.0,
            filter_resonance: 0.0,
            filter_env_depth: 0.5,
            filter_key_track: 0.5,
            hpf_cutoff: 20.0,
            env1_attack: 0.01,
            env1_decay: 0.3,
            env1_sustain: 0.6,
            env1_release: 0.5,
            env1_to_vca: false,
            env2_attack: 0.01,
            env2_decay: 0.3,
            env2_sustain: 0.7,
            env2_release: 0.5,
            lfo_rate: 5.0,
            lfo_waveform: 0,
            lfo_pitch_depth: 0.0,
            lfo_filter_depth: 0.0,
            lfo_pwm_depth: 0.0,
            lfo_delay: 0.0,
            chorus_mode: 3,
            master_volume: 0.7,
            assign_mode: 0,
            portamento: 0.0,
            arp_mode: 0,
            arp_range: 1,
            arp_tempo: 120.0,
        }
    }
}

/// SAB parameter count. Expanded from 32 to 40 for new controls.
pub const PARAM_COUNT: usize = 40;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_patch_in_range() {
        let p = EngineParams::default_patch();
        assert!(p.vco1_pw >= 0.05 && p.vco1_pw <= 0.95);
        assert!(p.vco1_level >= 0.0 && p.vco1_level <= 1.0);
        assert!(p.vco2_pw >= 0.05 && p.vco2_pw <= 0.95);
        assert!(p.vco2_level >= 0.0 && p.vco2_level <= 1.0);
        assert!(p.vco2_detune >= -1.0 && p.vco2_detune <= 1.0);
        assert!(p.cross_mod >= 0.0 && p.cross_mod <= 1.0);
        assert!(p.noise_level >= 0.0 && p.noise_level <= 1.0);
        assert!(p.sub_osc_level >= 0.0 && p.sub_osc_level <= 1.0);
        assert!(p.filter_cutoff >= 20.0 && p.filter_cutoff <= 20000.0);
        assert!(p.filter_resonance >= 0.0 && p.filter_resonance <= 1.0);
        assert!(p.filter_env_depth >= -1.0 && p.filter_env_depth <= 1.0);
        assert!(p.filter_key_track >= 0.0 && p.filter_key_track <= 1.0);
        assert!(p.hpf_cutoff >= 20.0 && p.hpf_cutoff <= 20000.0);
        assert!(p.env1_attack >= 0.001);
        assert!(p.env1_decay >= 0.001);
        assert!(p.env1_sustain >= 0.0 && p.env1_sustain <= 1.0);
        assert!(p.env1_release >= 0.001);
        assert!(p.env2_attack >= 0.001);
        assert!(p.env2_decay >= 0.001);
        assert!(p.env2_sustain >= 0.0 && p.env2_sustain <= 1.0);
        assert!(p.env2_release >= 0.001);
        assert!(p.lfo_rate >= 0.1 && p.lfo_rate <= 30.0);
        assert!(p.master_volume >= 0.0 && p.master_volume <= 1.0);
        assert!(p.portamento >= 0.0 && p.portamento <= 5.0);
        assert!(p.arp_range >= 1 && p.arp_range <= 4);
        assert!(p.arp_tempo >= 30.0 && p.arp_tempo <= 300.0);
    }

    #[test]
    fn param_count_is_40() {
        assert_eq!(PARAM_COUNT, 40);
    }
}
