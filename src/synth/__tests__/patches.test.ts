import { describe, test, expect } from 'vitest';
import { FACTORY_PATCHES } from '../patches';
import { PARAM_COUNT, P } from '../../audio/jp8-engine';

describe('Factory Patches', () => {
  test('all 16 patches have exactly 68 params', () => {
    expect(FACTORY_PATCHES).toHaveLength(16);
    for (const patch of FACTORY_PATCHES) {
      expect(patch.params).toHaveLength(PARAM_COUNT);
    }
  });

  test('all patches have non-empty names', () => {
    for (const patch of FACTORY_PATCHES) {
      expect(patch.name.length).toBeGreaterThan(0);
    }
  });

  test('no NaN in any param array', () => {
    for (const patch of FACTORY_PATCHES) {
      for (let i = 0; i < patch.params.length; i++) {
        expect(Number.isNaN(patch.params[i])).toBe(false);
      }
    }
  });

  test('P enum has all extended module keys', () => {
    expect(P.SOURCE_MODE).toBe(39);
    expect(P.SPECTRAL_TILT).toBe(40);
    expect(P.WG_EXCITATION).toBe(45);
    expect(P.MODAL_MIX).toBe(49);
    expect(P.CHAOS_ENABLE).toBe(54);
    expect(P.BUBBLE_ENABLE).toBe(61);
    expect(P.BUBBLE_LEVEL).toBe(65);
  });

  test('PARAM_COUNT is 68', () => {
    expect(PARAM_COUNT).toBe(68);
  });

  test('params in valid ranges', () => {
    for (const patch of FACTORY_PATCHES) {
      const p = patch.params;
      // Filter cutoff: idx 12, range [0, 20000] (0 gets clamped to 20 in engine)
      expect(p[12]).toBeGreaterThanOrEqual(0);
      expect(p[12]).toBeLessThanOrEqual(20000);
      // Resonance: idx 13, range [0, 1]
      expect(p[13]).toBeGreaterThanOrEqual(0);
      expect(p[13]).toBeLessThanOrEqual(1);
      // Volume: idx 33, range [0, 1]
      expect(p[33]).toBeGreaterThanOrEqual(0);
      expect(p[33]).toBeLessThanOrEqual(1);
      // Env sustain levels: [0, 1]
      expect(p[19]).toBeGreaterThanOrEqual(0);
      expect(p[19]).toBeLessThanOrEqual(1);
      expect(p[24]).toBeGreaterThanOrEqual(0);
      expect(p[24]).toBeLessThanOrEqual(1);
    }
  });
});
