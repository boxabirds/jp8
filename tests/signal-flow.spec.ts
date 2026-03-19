import { test, expect } from '@playwright/test';
import { startAndInjectAnalyser, isAudioPresent, playNote, releaseNote } from './helpers/audio-analyser';

test.describe('Signal Flow Bar', () => {

  test('signal flow bar is visible after start', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('start-audio').click();
    await expect(page.getByTestId('signal-flow-bar')).toBeVisible();
  });

  test('all expected blocks are present', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('start-audio').click();
    await expect(page.getByTestId('sfb-source')).toBeVisible();
    await expect(page.getByTestId('sfb-bubble')).toBeVisible();
    await expect(page.getByTestId('sfb-vcf')).toBeVisible();
    await expect(page.getByTestId('sfb-modal')).toBeVisible();
    await expect(page.getByTestId('sfb-output')).toBeVisible();
    await expect(page.getByTestId('sfb-chaos')).toBeVisible();
  });

  test('modal toggle changes visual state', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('start-audio').click();

    const toggle = page.getByTestId('sfb-modal-toggle');
    await expect(toggle).toHaveText('OFF');

    await toggle.click();
    await expect(toggle).toHaveText('ON');

    await toggle.click();
    await expect(toggle).toHaveText('OFF');
  });

  test('clicking block opens expansion tray', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('start-audio').click();

    // Tray should not exist initially
    await expect(page.getByTestId('module-tray')).not.toBeVisible();

    // Click modal expand
    await page.getByTestId('sfb-modal-expand').click();
    await expect(page.getByTestId('module-tray')).toBeVisible();
  });

  test('clicking different block swaps tray', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('start-audio').click();

    // Open modal tray
    await page.getByTestId('sfb-modal-expand').click();
    await expect(page.getByTestId('module-tray')).toBeVisible();

    // Switch to chaos — tray should still be visible (different content)
    await page.getByTestId('sfb-chaos-expand').click();
    await expect(page.getByTestId('module-tray')).toBeVisible();
  });

  test('clicking same block closes tray', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('start-audio').click();

    await page.getByTestId('sfb-modal-expand').click();
    await expect(page.getByTestId('module-tray')).toBeVisible();

    await page.getByTestId('sfb-modal-expand').click();
    await expect(page.getByTestId('module-tray')).not.toBeVisible();
  });

  test('source selector WG dims VCO sections', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('start-audio').click();

    // Select WG source
    const wgBtn = page.getByTestId('sfb-source').getByText('WG');
    await wgBtn.click();
    await page.waitForTimeout(300);

    // Module tray should open with waveguide controls
    await expect(page.getByTestId('module-tray')).toBeVisible();

    // Switch back to BLEP
    const blepBtn = page.getByTestId('sfb-source').getByText('BLEP');
    await blepBtn.click();
    await page.waitForTimeout(300);

    // Tray should close
    await expect(page.getByTestId('module-tray')).not.toBeVisible();
  });

  test('existing patches still produce audio with signal flow bar', async ({ page }) => {
    await startAndInjectAnalyser(page);

    // Play note with default patch (BLEP source, no modules)
    await playNote(page, 60);
    await page.waitForTimeout(300);
    expect(await isAudioPresent(page)).toBe(true);
    await releaseNote(page, 60);
  });
});
