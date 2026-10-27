import { defineConfig } from '@playwright/test';
// Regenerates README screenshots: `pnpm screenshots`. Not part of the test suite.
export default defineConfig({
  testDir: 'scripts', testMatch: 'screenshots.spec.ts', timeout: 120000, fullyParallel: false,
  use: { channel: 'chrome', baseURL: 'http://127.0.0.1:1420', viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 },
  webServer: { command: 'pnpm dev', url: 'http://127.0.0.1:1420', reuseExistingServer: true },
  reporter: [['list']],
});
