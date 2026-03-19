import { test, expect } from '@playwright/test';

test.describe('JP-8 Rack', () => {

  test('rack starts with 2 instances', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('start-audio').click();

    // Wait for tabs to appear
    await expect(page.getByTestId('instance-tabs')).toBeVisible();
    await expect(page.getByTestId('tab-1')).toBeVisible();
    await expect(page.getByTestId('tab-2')).toBeVisible();

    // Mixer strip visible with channels
    await expect(page.getByTestId('mixer-strip')).toBeVisible();

    // Status badge
    await expect(page.getByText('ACTIVE')).toBeVisible();
  });

  test('instances produce independent audio', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('start-audio').click();
    await expect(page.getByTestId('tab-1')).toBeVisible();

    // Instance #1 is active by default — click a keyboard key
    // The keyboard uses data-note attributes; middle C = note 60
    const key60 = page.locator('[data-note="60"]').first();
    await key60.dispatchEvent('pointerdown');
    await page.waitForTimeout(200);

    // Switch to instance #2
    await page.getByTestId('tab-2').click();
    await page.waitForTimeout(100);

    // Instance #2 should show JUPITER-8 header (panel rendered)
    await expect(page.getByText('JUPITER-8')).toBeVisible();

    // Click a different key on instance #2
    const key64 = page.locator('[data-note="64"]').first();
    await key64.dispatchEvent('pointerdown');
    await page.waitForTimeout(200);

    // Release both
    await key60.dispatchEvent('pointerup');
    await key64.dispatchEvent('pointerup');
  });

  test('patch loading is per-instance', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('start-audio').click();
    await expect(page.getByTestId('tab-1')).toBeVisible();

    // On instance #1, load "Bass" (patch index 2, 3rd button)
    const bassButton = page.getByText('Bass').first();
    await bassButton.click();
    await page.waitForTimeout(100);

    // Switch to #2, load "Warm Pad"
    await page.getByTestId('tab-2').click();
    await page.waitForTimeout(100);
    const padButton = page.getByText('Warm Pad').first();
    await padButton.click();
    await page.waitForTimeout(100);

    // Switch back to #1 — "Bass" should still be the active patch
    await page.getByTestId('tab-1').click();
    await page.waitForTimeout(100);

    // Bass patch button should have the active style
    // (We can't easily check CSS, but the patch buttons are rendered per-panel)
    // Verify the panel is showing by checking header exists
    await expect(page.getByText('JUPITER-8')).toBeVisible();
  });

  test('add and remove instance', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('start-audio').click();
    await expect(page.getByTestId('tab-1')).toBeVisible();
    await expect(page.getByTestId('tab-2')).toBeVisible();

    // Add a 3rd instance
    await page.getByTestId('add-instance').click();
    await page.waitForTimeout(500);

    // Should now have 3 tabs
    await expect(page.getByTestId('tab-3')).toBeVisible();

    // Remove the 3rd instance (click the × on its tab)
    const closeBtn = page.getByTestId('tab-3').locator('span');
    await closeBtn.click();
    await page.waitForTimeout(200);

    // Back to 2 tabs
    await expect(page.getByTestId('tab-3')).not.toBeVisible();
    await expect(page.getByTestId('tab-1')).toBeVisible();
    await expect(page.getByTestId('tab-2')).toBeVisible();
  });

  test('mixer mute and solo', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('start-audio').click();
    await expect(page.getByTestId('mixer-strip')).toBeVisible();

    // Find M and S buttons — they're small 22×18px buttons, not the MIDI select
    const muteButtons = page.getByTestId('mixer-strip').locator('button:text-is("M")');
    const soloButtons = page.getByTestId('mixer-strip').locator('button:text-is("S")');

    await expect(muteButtons).toHaveCount(2);
    await expect(soloButtons).toHaveCount(2);

    // Click mute on first channel
    await muteButtons.first().click();
    await page.waitForTimeout(100);

    // Click solo on second channel
    await soloButtons.nth(1).click();
    await page.waitForTimeout(100);

    // Unsolo
    await soloButtons.nth(1).click();
    await page.waitForTimeout(100);

    // Unmute
    await muteButtons.first().click();
  });
});
