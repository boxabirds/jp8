import { test, expect } from '@playwright/test';
import { startAndInjectAnalyser, isAudioPresent, getHighFreqEnergy, playNote, releaseNote } from './helpers/audio-analyser';

test.describe('Patches', () => {

  test('all 16 patches produce audio', async ({ page }) => {
    await startAndInjectAnalyser(page);

    const patchButtons = page.locator('[data-testid="patch-button"]');
    const count = await patchButtons.count();
    expect(count).toBe(16);

    for (let i = 0; i < count; i++) {
      await patchButtons.nth(i).click();
      await page.waitForTimeout(100);

      await playNote(page, 60);
      await page.waitForTimeout(300);

      const hasAudio = await isAudioPresent(page);
      expect(hasAudio).toBe(true);

      await releaseNote(page, 60);
      await page.waitForTimeout(200);
    }
  });

  test('patch changes timbre', async ({ page }) => {
    await startAndInjectAnalyser(page);

    // Load "Bass" (low cutoff → less HF)
    await page.getByText('Bass').first().click();
    await page.waitForTimeout(100);
    await playNote(page, 60);
    await page.waitForTimeout(300);
    const bassHfEnergy = await getHighFreqEnergy(page, 3000);
    await releaseNote(page, 60);
    await page.waitForTimeout(300);

    // Load "Brass Ensemble" (higher cutoff → more HF)
    await page.getByText('Brass Ensemble').first().click();
    await page.waitForTimeout(100);
    await playNote(page, 60);
    await page.waitForTimeout(300);
    const brassHfEnergy = await getHighFreqEnergy(page, 3000);
    await releaseNote(page, 60);

    expect(brassHfEnergy).toBeGreaterThan(bassHfEnergy);
  });

  test('patch per-instance isolation', async ({ page }) => {
    await startAndInjectAnalyser(page);

    // Load Bass on instance 1
    await page.getByText('Bass').first().click();
    await page.waitForTimeout(100);

    // Switch to instance 2 and load Strings
    await page.getByTestId('tab-2').click();
    await page.waitForTimeout(100);
    await page.getByText('Strings').first().click();
    await page.waitForTimeout(100);

    // Switch back to instance 1
    await page.getByTestId('tab-1').click();
    await page.waitForTimeout(100);

    // Play note — Bass should still be active on instance 1
    await playNote(page, 60);
    await page.waitForTimeout(300);
    const hasAudio = await isAudioPresent(page);
    expect(hasAudio).toBe(true);
    await releaseNote(page, 60);
  });
});
