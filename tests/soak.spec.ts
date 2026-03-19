import { test, expect } from '@playwright/test';
import { startAndInjectAnalyser, isAudioPresent, playNote, releaseNote } from './helpers/audio-analyser';

test.describe('Soak Tests', () => {

  test('browser stability 60min', async ({ page }) => {
    test.setTimeout(3700000); // 61+ minutes
    await startAndInjectAnalyser(page);

    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    const patchButtons = page.locator('[data-testid="patch-button"]');

    const THIRTY_SECONDS = 30_000;
    const FIVE_MINUTES = 5 * 60_000;
    const TEN_MINUTES = 10 * 60_000;
    const SIXTY_MINUTES = 60 * 60_000;

    // Capture initial memory baseline
    const initialMemory = await page.evaluate(() => {
      return (performance as any).memory?.usedJSHeapSize ?? 0;
    });

    const startTime = Date.now();
    let iteration = 0;
    let maxMemory = initialMemory;

    while (Date.now() - startTime < SIXTY_MINUTES) {
      const elapsed = Date.now() - startTime;
      iteration++;

      // Every 30 seconds: play a note for 2 seconds
      await playNote(page, 60);
      await page.waitForTimeout(2000);
      const hasAudio = await isAudioPresent(page);
      expect(hasAudio).toBe(true);
      await releaseNote(page, 60);

      // Measure memory every iteration
      const currentMemory = await page.evaluate(() => {
        return (performance as any).memory?.usedJSHeapSize ?? 0;
      });
      maxMemory = Math.max(maxMemory, currentMemory);

      // Every 5 minutes: switch patches and instances
      if (elapsed > 0 && elapsed % FIVE_MINUTES < THIRTY_SECONDS) {
        const patchIdx = iteration % 16;
        const count = await patchButtons.count();
        if (count > patchIdx) {
          await patchButtons.nth(patchIdx).click();
          await page.waitForTimeout(100);
        }

        // Switch to tab 2 and back
        const tab2 = page.getByTestId('tab-2');
        if (await tab2.isVisible()) {
          await tab2.click();
          await page.waitForTimeout(200);
          await page.getByTestId('tab-1').click();
          await page.waitForTimeout(200);
        }
      }

      // Every 10 minutes: add then remove an instance
      if (elapsed > 0 && elapsed % TEN_MINUTES < THIRTY_SECONDS && iteration % 20 === 0) {
        const addBtn = page.getByTestId('add-instance');
        if (await addBtn.isVisible()) {
          await addBtn.click();
          await page.waitForTimeout(500);

          // Remove the last instance
          const tabs = page.locator('[data-testid^="tab-"]');
          const tabCount = await tabs.count();
          if (tabCount > 2) {
            const lastTab = tabs.last();
            const closeBtn = lastTab.locator('span');
            await closeBtn.click();
            await page.waitForTimeout(200);
          }
        }
      }

      // Wait until next 30-second mark
      const nextCheck = Math.ceil((Date.now() - startTime) / THIRTY_SECONDS) * THIRTY_SECONDS;
      const sleepMs = Math.max(0, nextCheck - (Date.now() - startTime));
      if (sleepMs > 0 && sleepMs < THIRTY_SECONDS) {
        await page.waitForTimeout(sleepMs);
      }
    }

    // Final check: audio still works
    await playNote(page, 60);
    await page.waitForTimeout(300);
    expect(await isAudioPresent(page)).toBe(true);
    await releaseNote(page, 60);

    // Memory leak check: heap growth should be < 50MB over 60 minutes
    const memoryGrowthMB = (maxMemory - initialMemory) / (1024 * 1024);
    console.log(`Memory: initial=${(initialMemory / 1024 / 1024).toFixed(1)}MB, max=${(maxMemory / 1024 / 1024).toFixed(1)}MB, growth=${memoryGrowthMB.toFixed(1)}MB`);
    if (initialMemory > 0) {
      expect(memoryGrowthMB).toBeLessThan(50);
    }

    // Assert zero console errors
    expect(errors).toHaveLength(0);
  });
});
