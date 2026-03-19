import { test, expect } from '@playwright/test';
import { startAndInjectAnalyser, getAudioEnergy, getHighFreqEnergy, playNote, releaseNote } from './helpers/audio-analyser';

test.describe('Parameters', () => {

  test('filter cutoff affects spectrum', async ({ page }) => {
    await startAndInjectAnalyser(page);

    // Set filter cutoff low
    await page.evaluate(() => {
      const rack = (window as any).__jp8_rack;
      const inst = rack.getActiveInstance();
      if (inst) inst.engine.setParam(12, 200);
    });
    await playNote(page, 60);
    await page.waitForTimeout(400);
    const lowCutoffHf = await getHighFreqEnergy(page, 2000);
    await releaseNote(page, 60);
    await page.waitForTimeout(300);

    // Set filter cutoff high
    await page.evaluate(() => {
      const rack = (window as any).__jp8_rack;
      const inst = rack.getActiveInstance();
      if (inst) inst.engine.setParam(12, 15000);
    });
    await playNote(page, 60);
    await page.waitForTimeout(400);
    const highCutoffHf = await getHighFreqEnergy(page, 2000);
    await releaseNote(page, 60);

    expect(highCutoffHf).toBeGreaterThan(lowCutoffHf);
  });

  test('volume slider affects level', async ({ page }) => {
    await startAndInjectAnalyser(page);

    // Full volume
    await page.evaluate(() => {
      const rack = (window as any).__jp8_rack;
      const inst = rack.getActiveInstance();
      if (inst) inst.engine.setParam(33, 1.0);
    });
    await playNote(page, 60);
    await page.waitForTimeout(400);
    const loudEnergy = await getAudioEnergy(page);
    await releaseNote(page, 60);
    await page.waitForTimeout(300);

    // Low volume
    await page.evaluate(() => {
      const rack = (window as any).__jp8_rack;
      const inst = rack.getActiveInstance();
      if (inst) inst.engine.setParam(33, 0.1);
    });
    await playNote(page, 60);
    await page.waitForTimeout(400);
    const quietEnergy = await getAudioEnergy(page);
    await releaseNote(page, 60);

    expect(loudEnergy).toBeGreaterThan(quietEnergy);
  });
});
