import { describe, test, expect } from 'vitest';
import { FACTORY_PATCHES } from '../patches';

const PARAM_COUNT = 40;

describe('Factory Patches', () => {
  test('all 16 patches have exactly 40 params', () => {
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
