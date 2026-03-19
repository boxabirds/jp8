import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  testMatch: 'soak.spec.ts',
  timeout: 3700000, // 61+ minutes
  retries: 0,
  use: {
    baseURL: 'http://localhost:5173',
    headless: false,
  },
  webServer: {
    command: 'lsof -ti :5173 | xargs kill -9 2>/dev/null; bunx vite',
    port: 5173,
    reuseExistingServer: true,
    timeout: 15000,
  },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
  ],
});
