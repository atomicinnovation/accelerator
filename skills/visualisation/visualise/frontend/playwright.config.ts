import { defineConfig } from '@playwright/test'

const HEALTH_PORT = Number(process.env.E2E_HEALTH_PORT ?? 19087)

export default defineConfig({
  testDir: './e2e',
  timeout: 30_000,
  retries: 1,
  globalSetup: './e2e/global-setup.ts',
  globalTeardown: './e2e/global-teardown.ts',
  use: {
    // BASE_URL is populated by globalSetup (which runs after webServer).
    baseURL: process.env.BASE_URL ?? 'http://127.0.0.1',
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'chromium',
      use: { browserName: 'chromium' },
    },
  ],
  webServer: {
    command: 'node e2e/start-server.mjs',
    // Health server (fixed port) returns 200 once the real server is up.
    url: `http://127.0.0.1:${HEALTH_PORT}`,
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
    stdout: 'pipe',
    stderr: 'pipe',
  },
})
