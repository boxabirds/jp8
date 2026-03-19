# JP-8 Synthesizer Test Hardening Plan

## Context

The JP-8 is a virtual analog synthesizer with a Rust/WASM DSP engine (~1,500 lines), TypeScript/React UI (~1,800 lines), and AudioWorklet-based zero-copy rendering. It has 5 Playwright E2E tests covering rack lifecycle but **zero unit tests** on either the Rust or TypeScript side. The goal is MECE test coverage that makes the engine rock-solid for expansion: catch DSP bugs at the source, verify full-stack audio correctness in-browser, and prove long-term stability under stress.

---

## Layer 1: Rust Unit Tests (`cargo test`)

- [ ] Add `#[cfg(test)] mod tests` at the bottom of each module in `crates/jp8-core/src/`. No new crate dependencies needed — use built-in `#[test]` with `assert!` and float epsilon comparisons.

### 1.1 `math.rs` (4 tests)
- [ ] `fast_tanh_at_zero` — returns 0.0
- [ ] `fast_tanh_saturation` — close to real tanh at ±3, clamped beyond
- [ ] `fast_tanh_symmetry` — `fast_tanh(-x) == -fast_tanh(x)`
- [ ] `fast_tan_accuracy` — within 0.01 of `f32::tan()` for inputs 0–0.5

### 1.2 `oscillator.rs` (12 tests)
- [ ] `saw_output_bounded` — all samples in [-1.1, 1.1] over 1000 ticks at 440Hz
- [ ] `pulse_output_bounded` — all samples in [-1.2, 1.2]
- [ ] `saw_and_pulse_additive` — wave_flags=3 output louder than single waveform
- [ ] `wave_flags_zero_silence` — wave_flags=0 produces zeros
- [ ] `phase_stays_in_range` — phase always [0, 1) after extended run
- [ ] `portamento_converges` — freq within 1Hz of target after porta time
- [ ] `portamento_instant_below_threshold` — porta < 0.001s is immediate
- [ ] `sub_oscillator_square` — output is exactly ±1.0
- [ ] `sub_oscillator_half_rate` — completes 1 cycle per 2 VCO1 cycles
- [ ] `fm_zero_amount_unchanged` — `tick_with_fm(0.0)` matches `tick()`
- [ ] `fm_negative_dt_clamped` — large negative FM doesn't produce NaN
- [ ] `noise_bounded` — 10000 samples all in [-1, 1], no NaN

### 1.3 `filter.rs` (10 tests)
- [ ] `lpf_passes_dc` — constant input converges to ~1.0 output
- [ ] `lpf_attenuates_nyquist` — alternating ±1 input attenuated to <0.01 at cutoff=200Hz
- [ ] `resonance_zero_no_ring` — impulse response decays monotonically
- [ ] `resonance_high_peaks` — impulse at res=0.95 exceeds input at some point
- [ ] `cutoff_clamp_low` — set_cutoff(5) clamps to 20Hz
- [ ] `cutoff_clamp_high` — set_cutoff(30000) clamps to sr*0.45
- [ ] `no_nan_extreme_input` — feed 1e6, -1e6, 0. No NaN
- [ ] `reset_clears_state` — filter state zeroed after reset
- [ ] `hpf_blocks_dc` — constant input → output converges to 0
- [ ] `hpf_passes_high_freq` — alternating ±1 passes through at cutoff=20Hz

### 1.4 `envelope.rs` (12 tests)
- [ ] `idle_outputs_zero` — 100 ticks in Idle, all 0.0
- [ ] `attack_reaches_one` — level ≥ 1.0 triggers Decay transition
- [ ] `attack_linear` — at midpoint of attack, level ≈ 0.5
- [ ] `attack_instant_below_threshold` — attack < 0.001s: 1 tick to peak
- [ ] `decay_reaches_sustain` — converges within 0.002 of sustain level
- [ ] `decay_exponential_shape` — curve bows above linear midpoint
- [ ] `sustain_holds_level` — constant for 1000 ticks
- [ ] `release_reaches_idle` — level drops below 0.001, stage = Idle
- [ ] `release_from_sustain` — starts from sustain level after gate_off
- [ ] `retrigger_during_decay` — Attack from current level, no reset to 0
- [ ] `retrigger_during_release` — Attack from current level
- [ ] State machine: cover all 8 valid transitions (Idle→Attack, Attack→Decay, Decay→Sustain, Sustain→Release, Release→Idle, Attack→Release, Decay→Release, Idle→Idle on gate_off)

### 1.5 `lfo.rs` (10 tests)
- [ ] `sine_bounded` — output in [-depth, depth] over full run
- [ ] `triangle_bounded` — bounded, symmetric around zero
- [ ] `saw_bounded` — bounded
- [ ] `square_only_two_values` — exactly ±depth
- [ ] `sample_and_hold_changes_on_wrap` — value changes at phase wrap
- [ ] `sample_and_hold_holds_between_wraps` — constant between wraps
- [ ] `depth_zero_silence` — all output is 0
- [ ] `rate_affects_period` — at 1Hz, 44100 ticks completes 1 cycle
- [ ] `reset_zeroes_phase` — phase = 0 after reset
- [ ] `all_waveforms_no_nan` — 100K ticks per waveform, no NaN

### 1.6 `allocator.rs` (10 tests)
- [ ] `poly8_allocates_sequentially` — 8 note_ons return unique indices 0–7
- [ ] `poly8_steals_lowest_env` — 9th note steals voice with min env level
- [ ] `poly4_only_four_voices` — indices 0–3 only
- [ ] `unison_all_voices_same_note` — note_on activates all 8
- [ ] `solo_same_as_unison` — identical behavior
- [ ] `note_off_releases_correct` — releases voice matching the note
- [ ] `note_off_unison_releases_all` — all 8 released
- [ ] `all_off_clears_all` — all voices inactive
- [ ] `round_robin_wraps` — wraps after 8+
- [ ] `env_level_tracking` — update_env_level reflected in steal decisions

### 1.7 `arpeggiator.rs` (14 tests)
- [ ] `off_mode_passthrough` — note_on/off return false
- [ ] `up_ascending` — C4,E4,G4 → sequence C4,E4,G4,C4...
- [ ] `down_descending` — G4,E4,C4,G4...
- [ ] `updown_bounces` — ascending then descending with direction toggle
- [ ] `range_2_octaves` — single note expands across octaves
- [ ] `tempo_step_rate` — 120 BPM → samples_per_step ≈ 5512
- [ ] `add_note_sorted` — held array maintained in order
- [ ] `remove_note_shifts` — removal compacts array
- [ ] `duplicate_ignored` — held_count unchanged on duplicate
- [ ] `max_16_held_notes` — 17th rejected
- [ ] `all_off_resets` — held_count=0, sequence reset
- [ ] `is_active_requires_notes_and_mode` — both conditions needed
- [ ] `tick_inactive_returns_zero` — (0,0) when no notes
- [ ] `note_saturating_add` — octave range doesn't exceed MIDI 127

### 1.8 `chorus.rs` (8 tests)
- [ ] `off_passthrough` — input unchanged as mono L=R
- [ ] `mode1_stereo_spread` — L ≠ R after settling
- [ ] `mode2_wider_than_mode1` — greater L-R difference
- [ ] `mode12_combined` — output differs from mode1 and mode2 alone
- [ ] `silence_in_silence_out` — zero input → near-zero output
- [ ] `no_nan_long_run` — 100K ticks, no NaN
- [ ] `reset_clears_buffers` — all zeros after reset
- [ ] `output_bounded` — never exceeds 2x input peak

### 1.9 `voice.rs` (8 tests)
- [ ] `silent_when_inactive` — render_sample returns 0.0
- [ ] `produces_audio_after_note_on` — nonzero output
- [ ] `note_off_eventually_silences` — decays below 0.001 within release + margin
- [ ] `velocity_scales_output` — vel=127 louder than vel=30
- [ ] `env1_to_vca_reduces_output` — lower sustained level when enabled with env1 sustain < 1
- [ ] `cross_mod_changes_timbre` — RMS differs with cross_mod=0.5 vs 0.0
- [ ] `lfo_delay_ramps` — LFO effect = 0 at sample 0, full at sample SR
- [ ] `no_nan_full_chain` — 44100 ticks, various params, no NaN/Inf

### 1.10 `engine.rs` (10 tests)
- [ ] `silence_without_notes` — render produces zeros
- [ ] `note_on_produces_audio` — nonzero output after note_on
- [ ] `note_off_decays` — converges to zero after release
- [ ] `apply_params_clamps` — out-of-range values clamped correctly
- [ ] `all_notes_off_silences` — output zeros after sufficient release
- [ ] `voices_active_count` — tracks correctly on note_on/off
- [ ] `unison_activates_all` — assign_mode=2 → 8 active voices
- [ ] `master_volume_scales` — vol=1.0 peak > vol=0.5 peak
- [ ] `no_nan_all_factory_patches` — all 16 patches: apply, note_on, render 10 blocks, scan for NaN/Inf. **Highest single-test coverage of entire DSP chain.**
- [ ] `render_stereo_interleaved` — buf[0]=L, buf[1]=R, buf[2]=L...

### 1.11 `params.rs` (2 tests)
- [ ] `default_patch_in_range` — all defaults within documented ranges
- [ ] `param_count_is_40` — PARAM_COUNT == 40

**Total Layer 1: ~100 tests**

---

## Layer 2: Rust Integration Tests

**File:** `crates/jp8-core/tests/integration.rs`

Tests the `Engine` as a black box, verifying cross-module behavior:

- [ ] `create_render_destroy_lifecycle` — no panic
- [ ] `eight_engines_independent` — 8 engines, different notes, different outputs
- [ ] `param_buffer_roundtrip` — apply_params → verify engine.params fields
- [ ] `note_routing_isolation` — engine A note 60, engine B note 72, outputs differ
- [ ] `render_output_256_samples` — 128 frames × 2 channels
- [ ] `rapid_create_destroy_100x` — no crash
- [ ] `all_factory_patches_no_nan` — 16 patches × note_on × 50 blocks, zero NaN/Inf
- [ ] `param_change_mid_render` — apply_params between renders, no crash/NaN

**Total Layer 2: ~8 tests**

---

## Layer 3: TypeScript Unit Tests (vitest)

**Setup:**
- [ ] Add `vitest` devDependency
- [ ] Add `"test:unit": "bunx vitest run"` to package.json
- [ ] vitest auto-discovers `*.test.ts` files, uses existing vite.config.ts

### 3.1 `src/audio/__tests__/jp8-rack.test.ts` (~12 tests)

Mock `AudioContext`, `GainNode`, `StereoPannerNode` with `vi.fn()`. Test pure logic only (no real audio):

- [ ] Constructor defaults (mode, status, empty instances)
- [ ] addInstance creates instance with default channel config
- [ ] First instance becomes activeId
- [ ] Max instance limit throws
- [ ] removeInstance cleans up and moves activeId
- [ ] setActiveId rejects unknown IDs
- [ ] setChannelVolume/Pan/Mute/Solo update config
- [ ] Solo logic: solo one channel → others gain=0
- [ ] Unsolo restores volumes
- [ ] shouldReceiveMidi: OMNI → active only, channel match

### 3.2 `src/synth/__tests__/midi.test.ts` (~6 tests)

Mock `navigator.requestMIDIAccess`. Test MIDI message parsing:

- [ ] 0x90 vel>0 → routeNoteOn
- [ ] 0x80 → routeNoteOff
- [ ] 0x90 vel=0 → routeNoteOff
- [ ] 0xB0 → routeCC with correct channel/cc/value
- [ ] Short messages ignored
- [ ] Missing MIDI API returns noop

### 3.3 `src/synth/__tests__/patches.test.ts` (~4 tests)

- [ ] All 16 patches have exactly 40 params
- [ ] All patches have non-empty names
- [ ] No NaN in any param array
- [ ] Spot-check ranges: idx 12 in [20,20000], idx 13 in [0,1], etc.

**Total Layer 3: ~22 tests**

---

## Layer 4: E2E Tests (Playwright, headed)

### Test hook requirement

- [ ] Add to `src/ui/RackView.tsx` (1 line): expose `window.__jp8_rack` in dev mode so E2E tests can access the `AudioContext` and attach an `AnalyserNode` for audio verification.

### Shared E2E helper: `tests/helpers/audio-analyser.ts`

- [ ] Utility that injects an `AnalyserNode` via `page.evaluate()`, connects to the rack's master AudioContext destination, and provides:
  - `getAudioEnergy()` — sum of FFT magnitudes (silence threshold ~= 0)
  - `getPeakFrequency()` — bin with highest magnitude
  - `isAudioPresent()` — energy > threshold

### 4.1 `tests/sound-verification.spec.ts` (6 tests)
- [ ] `keyboard_click_produces_audio` — click note 60, verify AnalyserNode energy > threshold
- [ ] `keyboard_release_stops_audio` — release, wait for envelope, energy drops
- [ ] `audio_fundamental_near_expected` — play A4 (note 69), FFT peak near 440Hz bin
- [ ] `stereo_output_present` — both L and R channels have energy
- [ ] `silence_when_no_interaction` — fresh load, no clicks, silence
- [ ] `audio_survives_tab_switch` — play note, switch tab, switch back, still plays

### 4.2 `tests/patches.spec.ts` (3 tests)
- [ ] `all_16_patches_produce_audio` — loop: click patch button, click key, verify energy > threshold
- [ ] `patch_changes_timbre` — "Bass" vs "Brass Ensemble": high-freq energy differs
- [ ] `patch_per_instance_isolation` — load on instance 1, instance 2 unaffected

### 4.3 `tests/arpeggiator.spec.ts` (3 tests)
- [ ] `arp_produces_rhythmic_output` — enable arp, hold key, sample energy at intervals → periodic peaks
- [ ] `arp_stops_on_release` — release key, audio eventually stops
- [ ] `arp_tempo_affects_speed` — 120 BPM vs 240 BPM: peak interval ~halves

### 4.4 `tests/parameters.spec.ts` (4 tests)
- [ ] `filter_cutoff_affects_spectrum` — low cutoff → less HF energy, high cutoff → more
- [ ] `volume_slider_affects_level` — volume 1.0 peak > volume 0.3 peak
- [ ] `chorus_mode_stereo_width` — chorus off: L≈R, chorus on: L≠R
- [ ] `resonance_creates_peak` — energy concentration near cutoff frequency

### 4.5 `tests/multi-instance-audio.spec.ts` (2 tests)
- [ ] `two_instances_simultaneous` — instance 1 note 60, instance 2 note 72, both produce audio
- [ ] `muted_instance_silent` — mute instance 1, its note produces no contribution

**Total Layer 4: 18 new tests (+ 5 existing = 23 E2E tests)**

---

## Layer 5: Stress Tests

### 5.1 Rust Stress Tests — `crates/jp8-core/tests/stress.rs`

All marked `#[ignore]` — run explicitly via `cargo test --test stress -- --ignored`.

- [ ] `rapid_param_changes` — random params every block for 10K blocks, no NaN/Inf
- [ ] `note_spam` — 100 note_on/off pairs per block for 5K blocks, no crash
- [ ] `all_eight_voices_sustained` — 8 simultaneous notes, 10K blocks, bounded output
- [ ] `extreme_param_combos` — ~10 adversarial param sets (all zeros, max resonance+low cutoff, max everything, negative env depth+high reso, etc.), each: note_on, 100 blocks, no NaN
- [ ] `voice_stealing_under_load` — Poly4 mode, 20 notes forcing 16 steals, no crash
- [ ] `cross_mod_extreme` — cross_mod=1.0, both VCOs max level/freq, 10K blocks
- [ ] `rapid_arp_mode_toggle` — switch arp mode every block while notes held
- [ ] `zero_attack_zero_release_spam` — attack=release=0.001s, rapid note_on/off

### 5.2 Playwright Stress Tests — `tests/stress.spec.ts`

Separate config: `playwright-stress.config.ts` with `timeout: 120000`.

- [ ] `rapid_slider_sweep` — sweep filter cutoff full range 10x in 5s, no freeze
- [ ] `rapid_note_spam` — 100 pointerdown/up events in 2s, engine survives
- [ ] `eight_instances_playing` — 8 instances, different notes, 10s simultaneous, no console errors
- [ ] `rapid_patch_switching` — cycle all 16 patches 5x while note held
- [ ] `instance_add_remove_cycle` — add 6, remove 3, add 3, remove all but 1

**Total Layer 5: ~13 tests**

---

## Layer 6: Soak Tests (60+ minutes)

### 6.1 Rust Soak — `crates/jp8-core/tests/soak.rs`

Marked `#[ignore]`. These simulate 60 minutes of audio time (158M samples) but run at CPU speed (~16s wall clock).

- [ ] `continuous_playback_60min` — render 158M samples in 128-frame blocks. Every 1M samples assert: no NaN, output bounded [-2,2], not all zeros while notes playing. Track max/min for drift detection.
- [ ] `param_automation_loop_60min` — sweep all 40 params through sine-wave automation. Monitor for NaN and unbounded output throughout.

### 6.2 Playwright Soak — `tests/soak.spec.ts`

Separate config: `playwright-soak.config.ts` with `timeout: 3700000` (61+ min).

- [ ] `browser_stability_60min` — 2 instances running. Every 30s: play note for 2s, release. Every 5min: switch patches, switch instances. Every 10min: add then remove an instance. Monitor `performance.memory` for WASM memory growth (<1MB over 60min). Collect all console.error — assert zero at end. Final check: play note, verify AnalyserNode still works.

**Total Layer 6: 3 tests**

---

## Implementation Sequence

- [ ] **Phase 1**: Rust unit tests (Layer 1) — `crates/jp8-core/src/*.rs` — Highest ROI, catches DSP bugs at source, millisecond runtime
- [ ] **Phase 2**: Rust integration + stress (Layers 2, 5.1) — `crates/jp8-core/tests/{integration,stress}.rs` — Cross-module edge cases
- [ ] **Phase 3**: vitest setup + TS unit tests (Layer 3) — `src/**/__tests__/*.test.ts`, `package.json` — Pure logic coverage
- [ ] **Phase 4**: E2E audio verification (Layer 4) — `tests/*.spec.ts`, `tests/helpers/`, `src/ui/RackView.tsx` (1-line hook) — Full-stack proof
- [ ] **Phase 5**: E2E stress + soak (Layers 5.2, 6) — `tests/{stress,soak}.spec.ts`, `playwright-{stress,soak}.config.ts` — Long-term stability

## Package.json Scripts (final)

```json
"test:unit": "bunx vitest run",
"test:unit:watch": "bunx vitest",
"test:e2e": "npx playwright test --headed",
"test:e2e:stress": "npx playwright test --headed --config=playwright-stress.config.ts",
"test:e2e:soak": "npx playwright test --headed --config=playwright-soak.config.ts",
"test:rust": "cd crates/jp8-core && cargo test",
"test:rust:stress": "cd crates/jp8-core && cargo test --test stress -- --ignored",
"test:rust:soak": "cd crates/jp8-core && cargo test --test soak -- --ignored",
"test:all": "npm run test:rust && npm run test:unit && npm run test:e2e"
```

## Verification

- [ ] **Phase 1-2**: `cargo test` in `crates/jp8-core` — all pass, zero warnings
- [ ] **Phase 3**: `bunx vitest run` — all pass
- [ ] **Phase 4**: `npx playwright test --headed` — all pass, visual confirmation of audio
- [ ] **Phase 5**: `npx playwright test --headed --config=playwright-stress.config.ts` + `cargo test --test stress -- --ignored`
- [ ] **Phase 6**: `cargo test --test soak -- --ignored` (~16s wall clock) + `npx playwright test --headed --config=playwright-soak.config.ts` (60+ min real time)

## Test Count Summary

| Layer | Tests | Runtime |
|-------|-------|---------|
| L1: Rust unit | ~100 | <1s |
| L2: Rust integration | ~8 | <1s |
| L3: TypeScript unit | ~22 | <2s |
| L4: E2E (headed) | ~23 | ~60s |
| L5: Stress | ~13 | ~5min |
| L6: Soak | ~3 | ~60min |
| **Total** | **~169** | |
