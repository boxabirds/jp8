/// Parameter definitions.
/// Expanded from spec §5.1 to include HPF, sub-osc, LFO delay, ENV1→VCA.
/// Extended with synthesis modules: spectral, waveguide, modal, chaos, bubble.
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

    // --- Extended synthesis modules (indices 39-67) ---

    // Voice source selection
    pub source_mode: u8,        // 0=BLEP, 1=Spectral, 2=Waveguide

    // IFFT Spectral engine
    pub spectral_tilt: f32,     // -1..+1 (dark..bright)
    pub spectral_partials: u8,  // 2-64
    pub spectral_noise: f32,    // 0-1
    pub spectral_morph: f32,    // 0-1
    pub spectral_target: u8,    // 0-N preset

    // Commuted waveguide
    pub wg_excitation: u8,      // 0-5 (seed sample selector)
    pub wg_body: u8,            // 0-4 (body IR selector)
    pub wg_brightness: f32,     // 0-1
    pub wg_body_mix: f32,       // 0-1

    // Modal resonator (post-filter effect)
    pub modal_mix: f32,         // 0-1 (0=bypass)
    pub modal_material: f32,    // 0-1 (rubber..metal)
    pub modal_body: u8,         // 0-4 preset
    pub modal_modes: u8,        // 4-32
    pub modal_inharmonicity: f32, // 0-1

    // Benjolin chaos modulator
    pub chaos_enable: u8,       // 0/1
    pub chaos_rate1: f32,       // 0.1-30 Hz
    pub chaos_rate2: f32,       // 0.1-30 Hz
    pub chaos_depth: f32,       // 0-1
    pub chaos_to_pitch: f32,    // 0-1
    pub chaos_to_filter: f32,   // 0-1
    pub chaos_to_pwm: f32,      // 0-1

    // Bubble oscillator
    pub bubble_enable: u8,      // 0/1
    pub bubble_rate: f32,       // 0-60
    pub bubble_min_size: f32,   // 0.001-0.01
    pub bubble_max_size: f32,   // 0.005-0.03
    pub bubble_level: f32,      // 0-1
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
            // Extended modules — all off/bypass by default
            source_mode: 0,
            spectral_tilt: 0.0,
            spectral_partials: 32,
            spectral_noise: 0.0,
            spectral_morph: 0.0,
            spectral_target: 0,
            wg_excitation: 0,
            wg_body: 0,
            wg_brightness: 0.5,
            wg_body_mix: 0.5,
            modal_mix: 0.0,
            modal_material: 0.5,
            modal_body: 0,
            modal_modes: 16,
            modal_inharmonicity: 0.0,
            chaos_enable: 0,
            chaos_rate1: 5.0,
            chaos_rate2: 7.0,
            chaos_depth: 0.0,
            chaos_to_pitch: 0.0,
            chaos_to_filter: 0.0,
            chaos_to_pwm: 0.0,
            bubble_enable: 0,
            bubble_rate: 5.0,
            bubble_min_size: 0.003,
            bubble_max_size: 0.01,
            bubble_level: 0.0,
        }
    }
}

/// SAB parameter count. Extended from 40 to 68 for synthesis modules.
pub const PARAM_COUNT: usize = 68;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_patch_in_range() {
        let p = EngineParams::default_patch();
        // Original params
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
        // Extended params
        assert!(p.source_mode <= 2);
        assert!(p.spectral_tilt >= -1.0 && p.spectral_tilt <= 1.0);
        assert!(p.spectral_partials >= 2 && p.spectral_partials <= 64);
        assert!(p.spectral_noise >= 0.0 && p.spectral_noise <= 1.0);
        assert!(p.spectral_morph >= 0.0 && p.spectral_morph <= 1.0);
        assert!(p.wg_brightness >= 0.0 && p.wg_brightness <= 1.0);
        assert!(p.wg_body_mix >= 0.0 && p.wg_body_mix <= 1.0);
        assert!(p.modal_mix >= 0.0 && p.modal_mix <= 1.0);
        assert!(p.modal_material >= 0.0 && p.modal_material <= 1.0);
        assert!(p.modal_modes >= 4 && p.modal_modes <= 32);
        assert!(p.modal_inharmonicity >= 0.0 && p.modal_inharmonicity <= 1.0);
        assert!(p.chaos_enable <= 1);
        assert!(p.chaos_rate1 >= 0.1 && p.chaos_rate1 <= 30.0);
        assert!(p.chaos_rate2 >= 0.1 && p.chaos_rate2 <= 30.0);
        assert!(p.chaos_depth >= 0.0 && p.chaos_depth <= 1.0);
        assert!(p.bubble_enable <= 1);
        assert!(p.bubble_rate >= 0.0 && p.bubble_rate <= 60.0);
        assert!(p.bubble_level >= 0.0 && p.bubble_level <= 1.0);
    }

    #[test]
    fn param_count_is_68() {
        assert_eq!(PARAM_COUNT, 68);
    }
}
