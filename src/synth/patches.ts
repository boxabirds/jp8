/**
 * JP-8 Factory Patch Bank
 * Per spec §7.1–7.2. Each patch is a flat 32-element array matching the SAB layout (§5.1).
 *
 * Index map:
 *  0  VCO1 Wave (0=Saw,1=Pulse,2=Square)  16 Env1 Decay (s)
 *  1  VCO1 Range (-2..+2)                  17 Env1 Sustain (0..1)
 *  2  VCO1 PW (0.05..0.95)                 18 Env1 Release (s)
 *  3  VCO1 Level (0..1)                     19 Env2 Attack (s)
 *  4  VCO2 Wave                             20 Env2 Decay (s)
 *  5  VCO2 Range                            21 Env2 Sustain (0..1)
 *  6  VCO2 PW                               22 Env2 Release (s)
 *  7  VCO2 Level                            23 LFO Rate (Hz)
 *  8  VCO2 Detune (-1..+1 semitones)        24 LFO Wave (0=Sin,1=Tri,2=Saw,3=Sq,4=S&H)
 *  9  Cross Mod (0..1)                      25 LFO→Pitch (0..1)
 * 10  Noise (0..1)                          26 LFO→Filter (0..1)
 * 11  Filter Cutoff (20..20000 Hz)          27 LFO→PWM (0..1)
 * 12  Filter Resonance (0..1)               28 Chorus (0=Off,1=I,2=II,3=I+II)
 * 13  Filter Env Depth (-1..+1)             29 Master Volume (0..1)
 * 14  Filter Key Track (0..1)               30 Assign (0=Poly8,2=Unison,3=Solo)
 * 15  Env1 Attack (s)                       31 Portamento (0..5 s)
 */

export interface Patch {
  name: string;
  params: number[];
}

//                             VCO1                   VCO2                    MIX         FILTER                   ENV1 (Filter)            ENV2 (Amp)               LFO                      OUTPUT
//                          W  Rng PW  Lvl          W  Rng PW  Lvl  Det  XMod Noi  Cut    Res  Env  Key    A     D    S    R      A     D    S    R     Rate Wav Ptch Flt  PWM  Chr  Vol  Asgn Port

const BRASS_ENSEMBLE =     [0, 0, 0.5, 0.8,        0, 0, 0.5, 0.7, 0.08, 0,  0,  2500,  0.2, 0.6, 0.5,  0.05, 0.4, 0.3, 0.3,  0.05, 0.2, 0.8, 0.3,  6.0, 0, 0,   0.1, 0,   3,   0.7, 0, 0  ];
const WARM_PAD =           [0, 0, 0.5, 0.7,        0, 0, 0.5, 0.7, 0.15, 0,  0,  1800,  0.1, 0.4, 0.3,  0.8,  1.2, 0.4, 1.5,  0.6,  0.8, 0.85,1.2,  0.3, 0, 0,   0.15,0,   3,   0.65,0, 0  ];
const BASS =               [0, -1,0.5, 0.9,        2, -1,0.5, 0.6, 0.0,  0,  0,  800,   0.3, 0.7, 0.2,  0.01, 0.25,0.1, 0.12, 0.005,0.15,0.6, 0.1,  5.0, 0, 0,   0,   0,   0,   0.8, 3, 0  ];
const STRINGS =            [0, 0, 0.5, 0.6,        0, 0, 0.5, 0.6, 0.1,  0,  0,  3500,  0.05,0.3, 0.5,  0.4,  0.5, 0.5, 0.6,  0.5,  0.6, 0.9, 0.8,  5.5, 0, 0,   0.05,0,   2,   0.6, 0, 0  ];
const LEAD =               [1, 0, 0.35,0.8,        0, 0, 0.5, 0.7, 0.05, 0,  0,  3000,  0.4, 0.5, 0.6,  0.02, 0.35,0.2, 0.2,  0.01, 0.3, 0.6, 0.15, 5.0, 0, 0,   0,   0,   0,   0.7, 2, 0.08];
const SYNC_LEAD =          [0, 0, 0.5, 0.8,        0, 1, 0.5, 0.6, 0.0,  0.6,0,  4000,  0.5, 0.7, 0.5,  0.01, 0.3, 0.15,0.15, 0.005,0.2, 0.5, 0.1,  4.0, 0, 0,   0,   0,   0,   0.7, 3, 0  ];
const KEYS =               [1, 0, 0.45,0.7,        1, 0, 0.55,0.5, 0.0,  0,  0,  5000,  0.15,0.4, 0.5,  0.01, 0.5, 0.2, 0.3,  0.005,0.4, 0.4, 0.3,  5.0, 0, 0,   0,   0,   0,   0.7, 0, 0  ];
const AMBIENT_SWEEP =      [0, 0, 0.5, 0.6,        0, 0, 0.5, 0.6, 0.12, 0,  0,  1200,  0.2, 0.5, 0.3,  1.0,  2.0, 0.3, 3.0,  0.8,  2.0, 0.7, 3.0,  0.15,0, 0,   0.35,0,   3,   0.6, 0, 0  ];

// Extended presets beyond the spec's 8
const ORGAN =              [2, 0, 0.5, 0.5,        2, 1, 0.5, 0.5, 0.0,  0,  0,  12000, 0.0, 0.0, 0.0,  0.01, 0.01,1.0, 0.01, 0.005,0.01,1.0, 0.01, 5.0, 0, 0,   0,   0,   1,   0.6, 0, 0  ];
const PLUCK =              [0, 0, 0.5, 0.8,        0, 0, 0.5, 0.6, 0.03, 0,  0,  6000,  0.1, 0.8, 0.5,  0.001,0.15,0.0, 0.1,  0.001,0.2, 0.0, 0.15, 5.0, 0, 0,   0,   0,   3,   0.7, 0, 0  ];
const FAT_UNISON =         [0, 0, 0.5, 0.9,        0, 0, 0.5, 0.9, 0.1,  0,  0,  4000,  0.2, 0.5, 0.5,  0.05, 0.3, 0.6, 0.4,  0.03, 0.2, 0.7, 0.3,  5.0, 0, 0,   0,   0,   3,   0.5, 2, 0  ];
const PWM_STRINGS =        [1, 0, 0.3, 0.7,        1, 0, 0.7, 0.7, 0.08, 0,  0,  4500,  0.05,0.25,0.5,  0.5,  0.6, 0.6, 0.8,  0.4,  0.5, 0.85,0.7,  3.0, 0, 0,   0,   0.4, 2,   0.6, 0, 0  ];
const FILTER_BASS =        [0, -1,0.5, 0.9,        1, -1,0.4, 0.7, 0.0,  0,  0,  500,   0.6, 0.9, 0.3,  0.001,0.3, 0.0, 0.15, 0.001,0.1, 0.0, 0.1,  5.0, 0, 0,   0,   0,   0,   0.8, 3, 0  ];
const SHIMMER_PAD =        [1, 1, 0.4, 0.5,        1, 0, 0.6, 0.5, 0.07, 0, 0.05,6000,  0.1, 0.2, 0.5,  1.2,  1.5, 0.5, 2.0,  0.8,  1.0, 0.8, 2.5,  0.4, 1, 0,   0.2, 0.15,3,   0.55,0, 0  ];
const RESONANT_SWEEP =     [0, 0, 0.5, 0.8,        0, 0, 0.5, 0.0, 0.0,  0,  0,  400,   0.85,0.9, 0.5,  0.01, 1.5, 0.0, 1.0,  0.01, 0.3, 0.7, 0.5,  0.2, 0, 0,   0.5, 0,   3,   0.6, 0, 0  ];
const NOISE_HIT =          [0, 0, 0.5, 0.0,        0, 0, 0.5, 0.0, 0.0,  0,  1.0,8000,  0.0, 0.6, 0.0,  0.001,0.1, 0.0, 0.05, 0.001,0.08,0.0, 0.05, 5.0, 0, 0,   0,   0,   0,   0.7, 0, 0  ];

export const FACTORY_PATCHES: Patch[] = [
  { name: 'Brass Ensemble', params: BRASS_ENSEMBLE },
  { name: 'Warm Pad',       params: WARM_PAD },
  { name: 'Bass',           params: BASS },
  { name: 'Strings',        params: STRINGS },
  { name: 'Lead',           params: LEAD },
  { name: 'Sync Lead',      params: SYNC_LEAD },
  { name: 'Keys',           params: KEYS },
  { name: 'Ambient Sweep',  params: AMBIENT_SWEEP },
  { name: 'Organ',          params: ORGAN },
  { name: 'Pluck',          params: PLUCK },
  { name: 'Fat Unison',     params: FAT_UNISON },
  { name: 'PWM Strings',    params: PWM_STRINGS },
  { name: 'Filter Bass',    params: FILTER_BASS },
  { name: 'Shimmer Pad',    params: SHIMMER_PAD },
  { name: 'Reso Sweep',     params: RESONANT_SWEEP },
  { name: 'Noise Hit',      params: NOISE_HIT },
];
