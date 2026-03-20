import { test, expect } from '@playwright/test';
import { startAndInjectAnalyser, isAudioPresent, playNote, releaseNote } from './helpers/audio-analyser';

test.describe('Signal Flow Bar', () => {

  test('signal flow bar is visible after start', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('start-audio').click();
    await expect(page.getByTestId('signal-flow-bar')).toBeVisible();
  });

  test('module blocks are present', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('start-audio').click();
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

    await expect(page.getByTestId('module-tray')).not.toBeVisible();

    await page.getByTestId('sfb-modal-expand').click();
    await expect(page.getByTestId('module-tray')).toBeVisible();
  });

  test('clicking different block swaps tray', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('start-audio').click();

    await page.getByTestId('sfb-modal-expand').click();
    await expect(page.getByTestId('module-tray')).toBeVisible();

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

  test('source selector switches VCO and waveguide sections', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('start-audio').click();

    // Default: VCO source, VCO-1 section visible
    await expect(page.getByText('VCO-1')).toBeVisible();

    // Select WAVEGUIDE
    await page.getByText('WAVEGUIDE').click();
    await page.waitForTimeout(200);

    // VCO-1 should be replaced by WAVEGUIDE section
    await expect(page.getByText('VCO-1')).not.toBeVisible();

    // Switch back to VCO
    await page.getByText('VCO').first().click();
    await page.waitForTimeout(200);

    // VCO-1 should be back
    await expect(page.getByText('VCO-1')).toBeVisible();
  });

  test('existing patches still produce audio with signal flow bar', async ({ page }) => {
    await startAndInjectAnalyser(page);

    await playNote(page, 60);
    await page.waitForTimeout(300);
    expect(await isAudioPresent(page)).toBe(true);
    await releaseNote(page, 60);
  });
});
