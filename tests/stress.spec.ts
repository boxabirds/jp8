import { test, expect } from '@playwright/test';
import { startAndInjectAnalyser, isAudioPresent, playNote, releaseNote } from './helpers/audio-analyser';

test.describe('Stress Tests', () => {
  test.setTimeout(120000);

  test('rapid note spam', async ({ page }) => {
    await startAndInjectAnalyser(page);

    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    // 100 rapid note on/off events
    for (let i = 0; i < 100; i++) {
      await playNote(page, 48 + (i % 40));
      await releaseNote(page, 48 + (i % 40));
    }
    await page.waitForTimeout(500);

    // Engine should survive — verify audio still works
    await playNote(page, 60);
    await page.waitForTimeout(300);
    expect(await isAudioPresent(page)).toBe(true);
    await releaseNote(page, 60);

    expect(errors).toHaveLength(0);
  });

  test('rapid patch switching', async ({ page }) => {
    await startAndInjectAnalyser(page);

    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    const patchButtons = page.locator('[data-testid="patch-button"]');
    const count = await patchButtons.count();

    // Hold a note and cycle through all patches 3 times
    await playNote(page, 60);
    for (let round = 0; round < 3; round++) {
      for (let i = 0; i < count; i++) {
        await patchButtons.nth(i).click();
        await page.waitForTimeout(30);
      }
    }
    await releaseNote(page, 60);
    await page.waitForTimeout(100);

    // After rapid switching, load a known-good patch and verify engine works
    // (last rapid-switched patch may be "Noise Hit" which has zero-length envelope)
    await patchButtons.nth(0).click(); // Brass Ensemble
    await page.waitForTimeout(200);
    await playNote(page, 60);
    await page.waitForTimeout(500);
    expect(await isAudioPresent(page)).toBe(true);
    await releaseNote(page, 60);

    expect(errors).toHaveLength(0);
  });

  test('instance add remove cycle', async ({ page }) => {
    await startAndInjectAnalyser(page);

    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    // Add 4 more instances (total 6)
    for (let i = 0; i < 4; i++) {
      await page.getByTestId('add-instance').click();
      await page.waitForTimeout(300);
    }

    // Remove 3
    for (let i = 0; i < 3; i++) {
      const tabs = page.locator('[data-testid^="tab-"]');
      const lastTab = tabs.last();
      const closeBtn = lastTab.locator('span');
      await closeBtn.click();
      await page.waitForTimeout(200);
    }

    // Verify audio still works
    await playNote(page, 60);
    await page.waitForTimeout(300);
    expect(await isAudioPresent(page)).toBe(true);
    await releaseNote(page, 60);

    expect(errors).toHaveLength(0);
  });
});
