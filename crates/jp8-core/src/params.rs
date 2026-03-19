/// Parameter definitions + smoothing.
/// Per the JP-8 spec §2.8 and §5.1.

/// One-pole smoothed parameter.
pub struct SmoothedParam {
    current: f32,
    target: f32,
    coeff: f32,
}

impl SmoothedParam {
    pub fn new(value: f32, smoothing_hz: f32, sample_rate: f32) -> Self {
        Self {
            current: value,
            target: value,
            coeff: (-core::f32::consts::TAU * smoothing_hz / sample_rate).exp(),
        }
    }

    #[inline(always)]
    pub fn set(&mut self, target: f32) {
        self.target = target;
    }

    #[inline(always)]
    pub fn next(&mut self) -> f32 {
        self.current += (1.0 - self.coeff) * (self.target - self.current);
        self.current
    }

    pub fn get(&self) -> f32 {
        self.current
    }

    pub fn snap(&mut self) {
        self.current = self.target;
    }
}

/// All engine parameters. Matches spec §5.1 parameter layout.
pub struct EngineParams {
    pub vco1_waveform: u8,
    pub vco1_range: i8,
    pub vco1_pw: f32,
    pub vco1_level: f32,
    pub vco2_waveform: u8,
    pub vco2_range: i8,
    pub vco2_pw: f32,
    pub vco2_level: f32,
    pub vco2_detune: f32,
    pub cross_mod: f32,
    pub noise_level: f32,
    pub filter_cutoff: f32,
    pub filter_resonance: f32,
    pub filter_env_depth: f32,
    pub filter_key_track: f32,
    pub env1_attack: f32,
    pub env1_decay: f32,
    pub env1_sustain: f32,
    pub env1_release: f32,
    pub env2_attack: f32,
    pub env2_decay: f32,
    pub env2_sustain: f32,
    pub env2_release: f32,
    pub lfo_rate: f32,
    pub lfo_waveform: u8,
    pub lfo_pitch_depth: f32,
    pub lfo_filter_depth: f32,
    pub lfo_pwm_depth: f32,
    pub chorus_mode: u8,
    pub master_volume: f32,
    pub assign_mode: u8,
    pub portamento: f32,
}

impl EngineParams {
    pub fn default_patch() -> Self {
        Self {
            vco1_waveform: 0,
            vco1_range: 0,
            vco1_pw: 0.5,
            vco1_level: 0.8,
            vco2_waveform: 0,
            vco2_range: 0,
            vco2_pw: 0.5,
            vco2_level: 0.8,
            vco2_detune: 0.0,
            cross_mod: 0.0,
            noise_level: 0.0,
            filter_cutoff: 8000.0,
            filter_resonance: 0.0,
            filter_env_depth: 0.5,
            filter_key_track: 0.5,
            env1_attack: 0.01,
            env1_decay: 0.3,
            env1_sustain: 0.6,
            env1_release: 0.5,
            env2_attack: 0.01,
            env2_decay: 0.3,
            env2_sustain: 0.7,
            env2_release: 0.5,
            lfo_rate: 5.0,
            lfo_waveform: 0,
            lfo_pitch_depth: 0.0,
            lfo_filter_depth: 0.0,
            lfo_pwm_depth: 0.0,
            chorus_mode: 3,
            master_volume: 0.7,
            assign_mode: 0,
            portamento: 0.0,
        }
    }
}

/// Parameter count for SAB layout.
pub const PARAM_COUNT: usize = 32;
