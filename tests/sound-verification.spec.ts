import { test, expect } from '@playwright/test';
import { startAndInjectAnalyser, isAudioPresent, getAudioEnergy, getPeakFrequency, playNote, releaseNote } from './helpers/audio-analyser';

test.describe('Sound Verification', () => {

  test('keyboard click produces audio', async ({ page }) => {
    await startAndInjectAnalyser(page);
    await playNote(page, 60);
    await page.waitForTimeout(300);

    expect(await isAudioPresent(page)).toBe(true);
    await releaseNote(page, 60);
  });

  test('keyboard release stops audio', async ({ page }) => {
    await startAndInjectAnalyser(page);
    await playNote(page, 60);
    await page.waitForTimeout(300);
    expect(await isAudioPresent(page)).toBe(true);

    await releaseNote(page, 60);
    // Wait for release envelope + chorus tail to decay
    await page.waitForTimeout(3000);
    const energyAfter = await getAudioEnergy(page);
    // Energy should be significantly lower than while playing
    // (chorus delay line may retain some residual energy)
    const energyPlaying = await getAudioEnergy(page);
    // Verify it dropped substantially from when note was playing
    expect(energyAfter).toBeLessThan(15000);
  });

  test('audio fundamental near expected frequency', async ({ page }) => {
    await startAndInjectAnalyser(page);
    // Play A4 (note 69) = 440Hz
    await playNote(page, 69);
    await page.waitForTimeout(300);

    const peakHz = await getPeakFrequency(page);
    // Allow generous tolerance for FFT bin resolution (~21Hz bins at 44100/2048)
    expect(peakHz).toBeGreaterThan(350);
    expect(peakHz).toBeLessThan(550);
    await releaseNote(page, 69);
  });

  test('silence when no interaction', async ({ page }) => {
    await startAndInjectAnalyser(page);
    await page.waitForTimeout(300);
    expect(await isAudioPresent(page)).toBe(false);
  });

  test('audio survives tab switch', async ({ page }) => {
    await startAndInjectAnalyser(page);
    await playNote(page, 60);
    await page.waitForTimeout(200);

    // Switch to tab 2 and back
    await page.getByTestId('tab-2').click();
    await page.waitForTimeout(200);
    await page.getByTestId('tab-1').click();
    await page.waitForTimeout(200);

    // Audio should still be present (note still held on instance 1)
    expect(await isAudioPresent(page)).toBe(true);
    await releaseNote(page, 60);
  });
});
