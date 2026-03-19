/**
 * JP-8 Factory Patch Bank — 40-param layout.
 *
 * Index map (matches engine.rs apply_params):
 *  0  VCO1 Wave flags (1=saw,2=pulse,3=both)  20 Env1 Release (s)
 *  1  VCO1 Range (-2..+2)                      21 Env1→VCA (0/1)
 *  2  VCO1 PW                                  22 Env2 Attack (s)
 *  3  VCO1 Level                               23 Env2 Decay (s)
 *  4  VCO2 Wave flags                          24 Env2 Sustain
 *  5  VCO2 Range                               25 Env2 Release (s)
 *  6  VCO2 PW                                  26 LFO Rate (Hz)
 *  7  VCO2 Level                               27 LFO Wave (0-4)
 *  8  VCO2 Detune (semitones)                  28 LFO→Pitch
 *  9  Cross Mod (VCO2→VCO1)                    29 LFO→Filter
 * 10  Noise Level                              30 LFO→PWM
 * 11  Sub Osc Level                            31 LFO Delay (s)
 * 12  Filter Cutoff (Hz)                       32 Chorus (0-3)
 * 13  Filter Resonance                         33 Volume
 * 14  Filter Env Depth (bipolar)               34 Assign (0=Poly,2=Uni,3=Solo)
 * 15  Filter Key Track                         35 Portamento (s)
 * 16  HPF Cutoff (Hz)                          36-39 reserved
 * 17  Env1 Attack (s)
 * 18  Env1 Decay (s)
 * 19  Env1 Sustain
 */

export interface Patch {
  name: string;
  params: number[];
}

// Helper: build 40-element array with trailing zeros for reserved slots
function p(values: number[]): number[] {
  const out = new Array(40).fill(0);
  for (let i = 0; i < values.length && i < 40; i++) out[i] = values[i];
  return out;
}

//                              0     1   2     3     4     5   6     7     8     9    10   11   12     13    14    15   16    17     18    19   20    21  22     23    24   25    26   27  28   29    30   31   32  33   34  35
//                             VW1  Rng  PW1  Lvl1  VW2  Rng  PW2  Lvl2  Det  XMod  Noi  Sub  FCut  Res  FEnv  Key  HPF  E1A   E1D  E1S  E1R  E1V  E2A   E2D  E2S  E2R  LRate LW  LPt  LFlt LPWM LDly Chr  Vol  Asgn Port

export const FACTORY_PATCHES: Patch[] = [
  { name: 'Brass Ensemble', params: p([
    1,   0, 0.5, 0.8,   1,   0, 0.5, 0.7, 0.08, 0,   0,   0,   2500, 0.2, 0.6, 0.5, 20,  0.05, 0.4, 0.3, 0.3, 0,  0.05, 0.2, 0.8, 0.3,  6,   0, 0,   0.1, 0,   0,   3, 0.7, 0, 0
  ])},
  { name: 'Warm Pad', params: p([
    1,   0, 0.5, 0.7,   1,   0, 0.5, 0.7, 0.15, 0,   0,   0,   1800, 0.1, 0.4, 0.3, 20,  0.8,  1.2, 0.4, 1.5, 0,  0.6,  0.8, 0.85,1.2,  0.3, 0, 0,   0.15,0,   0,   3, 0.65,0, 0
  ])},
  { name: 'Bass', params: p([
    1,  -1, 0.5, 0.9,   3,  -1, 0.5, 0.6, 0,    0,   0,   0.4, 800,  0.3, 0.7, 0.2, 20,  0.01, 0.25,0.1, 0.12,0,  0.005,0.15,0.6, 0.1,  5,   0, 0,   0,   0,   0,   0, 0.8, 3, 0
  ])},
  { name: 'Strings', params: p([
    1,   0, 0.5, 0.6,   1,   0, 0.5, 0.6, 0.1,  0,   0,   0,   3500, 0.05,0.3, 0.5, 20,  0.4,  0.5, 0.5, 0.6, 0,  0.5,  0.6, 0.9, 0.8,  5.5, 0, 0,   0.05,0,   0,   2, 0.6, 0, 0
  ])},
  { name: 'Lead', params: p([
    2,   0, 0.35,0.8,   1,   0, 0.5, 0.7, 0.05, 0,   0,   0,   3000, 0.4, 0.5, 0.6, 20,  0.02, 0.35,0.2, 0.2, 0,  0.01, 0.3, 0.6, 0.15, 5,   0, 0,   0,   0,   0.3, 0, 0.7, 2, 0.08
  ])},
  { name: 'Sync Lead', params: p([
    1,   0, 0.5, 0.8,   1,   1, 0.5, 0.6, 0,    0.6, 0,   0,   4000, 0.5, 0.7, 0.5, 20,  0.01, 0.3, 0.15,0.15,0,  0.005,0.2, 0.5, 0.1,  4,   0, 0,   0,   0,   0,   0, 0.7, 3, 0
  ])},
  { name: 'Keys', params: p([
    2,   0, 0.45,0.7,   2,   0, 0.55,0.5, 0,    0,   0,   0,   5000, 0.15,0.4, 0.5, 20,  0.01, 0.5, 0.2, 0.3, 0,  0.005,0.4, 0.4, 0.3,  5,   0, 0,   0,   0,   0,   0, 0.7, 0, 0
  ])},
  { name: 'Ambient Sweep', params: p([
    1,   0, 0.5, 0.6,   1,   0, 0.5, 0.6, 0.12, 0,   0,   0,   1200, 0.2, 0.5, 0.3, 20,  1.0,  2.0, 0.3, 3.0, 0,  0.8,  2.0, 0.7, 3.0,  0.15,0, 0,   0.35,0,   0,   3, 0.6, 0, 0
  ])},
  { name: 'Organ', params: p([
    3,   0, 0.5, 0.5,   3,   1, 0.5, 0.5, 0,    0,   0,   0.3, 12000,0,   0,   0,   20,  0.01, 0.01,1.0, 0.01,0,  0.005,0.01,1.0, 0.01, 5,   0, 0,   0,   0,   0,   1, 0.6, 0, 0
  ])},
  { name: 'Pluck', params: p([
    1,   0, 0.5, 0.8,   1,   0, 0.5, 0.6, 0.03, 0,   0,   0,   6000, 0.1, 0.8, 0.5, 20,  0.001,0.15,0,   0.1, 0,  0.001,0.2, 0,   0.15, 5,   0, 0,   0,   0,   0,   3, 0.7, 0, 0
  ])},
  { name: 'Fat Unison', params: p([
    1,   0, 0.5, 0.9,   1,   0, 0.5, 0.9, 0.1,  0,   0,   0,   4000, 0.2, 0.5, 0.5, 20,  0.05, 0.3, 0.6, 0.4, 0,  0.03, 0.2, 0.7, 0.3,  5,   0, 0,   0,   0,   0,   3, 0.5, 2, 0
  ])},
  { name: 'PWM Strings', params: p([
    2,   0, 0.3, 0.7,   2,   0, 0.7, 0.7, 0.08, 0,   0,   0,   4500, 0.05,0.25,0.5, 20,  0.5,  0.6, 0.6, 0.8, 0,  0.4,  0.5, 0.85,0.7,  3,   0, 0,   0,   0.4, 0,   2, 0.6, 0, 0
  ])},
  { name: 'Filter Bass', params: p([
    1,  -1, 0.5, 0.9,   2,  -1, 0.4, 0.7, 0,    0,   0,   0.3, 500,  0.6, 0.9, 0.3, 20,  0.001,0.3, 0,   0.15,0,  0.001,0.1, 0,   0.1,  5,   0, 0,   0,   0,   0,   0, 0.8, 3, 0
  ])},
  { name: 'Shimmer Pad', params: p([
    2,   1, 0.4, 0.5,   2,   0, 0.6, 0.5, 0.07, 0,   0.05,0,   6000, 0.1, 0.2, 0.5, 20,  1.2,  1.5, 0.5, 2.0, 0,  0.8,  1.0, 0.8, 2.5,  0.4, 1, 0,   0.2, 0.15,1.0, 3, 0.55,0, 0
  ])},
  { name: 'Reso Sweep', params: p([
    1,   0, 0.5, 0.8,   1,   0, 0.5, 0,   0,    0,   0,   0,   400,  0.85,0.9, 0.5, 20,  0.01, 1.5, 0,   1.0, 0,  0.01, 0.3, 0.7, 0.5,  0.2, 0, 0,   0.5, 0,   0,   3, 0.6, 0, 0
  ])},
  { name: 'Noise Hit', params: p([
    1,   0, 0.5, 0,     1,   0, 0.5, 0,   0,    0,   1,   0,   8000, 0,   0.6, 0,   20,  0.001,0.1, 0,   0.05,0,  0.001,0.08,0,   0.05, 5,   0, 0,   0,   0,   0,   0, 0.7, 0, 0
  ])},
];
