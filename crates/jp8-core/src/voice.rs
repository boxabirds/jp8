/// Per-voice state. Fixed size, no indirection.
/// Per the JP-8 spec §2.1.

use crate::envelope::Envelope;
use crate::filter::IR3109;
use crate::lfo::Lfo;
use crate::oscillator::{NoiseGen, Oscillator, Waveform};
use crate::params::EngineParams;

pub struct Voice {
    pub vco1: Oscillator,
    pub vco2: Oscillator,
    pub noise: NoiseGen,
    pub filter: IR3109,
    pub env1: Envelope, // → filter cutoff
    pub env2: Envelope, // → VCA
    pub lfo: Lfo,
    pub gate: bool,
    pub note: u8,
    pub velocity: f32,
}

impl Voice {
    pub fn new(sample_rate: f32, voice_index: usize) -> Self {
        Self {
            vco1: Oscillator::new(),
            vco2: Oscillator::new(),
            noise: NoiseGen::new(42 + voice_index as u32 * 7),
            filter: IR3109::new(sample_rate),
            env1: Envelope::new(sample_rate),
            env2: Envelope::new(sample_rate),
            lfo: Lfo::new(),
            gate: false,
            note: 0,
            velocity: 0.0,
        }
    }

    pub fn note_on(&mut self, note: u8, velocity: u8, params: &EngineParams) {
        self.note = note;
        self.velocity = velocity as f32 / 127.0;
        self.gate = true;

        let freq = midi_to_hz(note);
        let range1 = 2.0f32.powf(params.vco1_range as f32);
        let range2 = 2.0f32.powf(params.vco2_range as f32);
        let detune_factor = 2.0f32.powf(params.vco2_detune / 12.0);

        self.vco1.set_freq(freq * range1, 0.3);
        self.vco1.waveform = int_to_waveform(params.vco1_waveform);
        self.vco1.pulse_width = params.vco1_pw;

        self.vco2.set_freq(freq * range2 * detune_factor, 0.3);
        self.vco2.waveform = int_to_waveform(params.vco2_waveform);
        self.vco2.pulse_width = params.vco2_pw;

        self.env1.gate_on();
        self.env2.gate_on();
        self.filter.reset();
    }

    pub fn note_off(&mut self) {
        self.gate = false;
        self.env1.gate_off();
        self.env2.gate_off();
    }

    /// Render one sample. Returns mono output.
    #[inline(always)]
    pub fn render_sample(&mut self, inv_sr: f32, params: &EngineParams) -> f32 {
        if !self.env2.is_active() {
            return 0.0;
        }

        // LFO
        let lfo_out = self.lfo.tick(inv_sr);

        // VCO1 with LFO pitch mod
        let pitch_mod = lfo_out * params.lfo_pitch_depth;
        let base_freq1 = self.vco1.freq;
        self.vco1.freq = base_freq1 * (1.0 + pitch_mod * 0.1);
        let vco1_out = self.vco1.tick(inv_sr);
        self.vco1.freq = base_freq1;

        // VCO2 with cross-mod from VCO1
        let xmod = params.cross_mod * vco1_out;
        let base_freq2 = self.vco2.freq;
        self.vco2.freq = base_freq2 * (1.0 + pitch_mod * 0.1);
        let vco2_out = self.vco2.tick_with_fm(inv_sr, xmod * inv_sr);
        self.vco2.freq = base_freq2;

        // Mixer
        let noise_out = self.noise.next();
        let mix = vco1_out * params.vco1_level
            + vco2_out * params.vco2_level
            + noise_out * params.noise_level;

        // Filter envelope → cutoff modulation
        let env1_out = self.env1.tick();
        let base_cutoff = params.filter_cutoff;
        let key_track = params.filter_key_track * (self.note as f32 - 60.0) * 50.0;
        let env_mod = env1_out * params.filter_env_depth * base_cutoff;
        let cutoff = (base_cutoff + env_mod + key_track + lfo_out * params.lfo_filter_depth * base_cutoff)
            .clamp(20.0, 20000.0);
        self.filter.set_cutoff(cutoff);
        self.filter.resonance = params.filter_resonance;

        let filtered = self.filter.tick(mix);

        // VCA envelope
        let env2_out = self.env2.tick();
        let vca_out = filtered * env2_out * self.velocity;

        vca_out
    }

    pub fn is_active(&self) -> bool {
        self.env2.is_active()
    }
}

#[inline(always)]
fn midi_to_hz(note: u8) -> f32 {
    440.0 * 2.0f32.powf((note as f32 - 69.0) / 12.0)
}

fn int_to_waveform(v: u8) -> Waveform {
    match v {
        0 => Waveform::Saw,
        1 => Waveform::Pulse,
        2 => Waveform::Square,
        _ => Waveform::Saw,
    }
}
