import { defineConfig } from '@playwright/test'

const HEALTH_PORT = Number(process.env.E2E_HEALTH_PORT ?? 19087)

export default defineConfig({
  testDir: './e2e',
  timeout: 30_000,
  retries: 1,
  // Force sequential execution across all projects so that the chromium
  // kanban drag tests cannot run concurrently with each other or with the
  // visual-regression snapshot tests (both touch shared fixture files or
  // compare screenshots of fixture-driven content).
  workers: 1,
  globalSetup: './e2e/global-setup.ts',
  globalTeardown: './e2e/global-teardown.ts',
  use: {
    // BASE_URL is populated by globalSetup (which runs after webServer).
    baseURL: process.env.BASE_URL ?? 'http://127.0.0.1',
    trace: 'on-first-retry',
  },
  projects: [
    // visual-regression runs first so the kanban snapshot captures the clean
    // fixture state before the chromium project's drag tests modify it.
    {
      name: 'visual-regression',
      testDir: './tests/visual-regression',
      snapshotDir: './tests/visual-regression/__screenshots__',
      use: { browserName: 'chromium' },
    },
    {
      name: 'chromium',
      // Depends on visual-regression so the kanban snapshot is captured
      // against clean fixtures before the drag tests modify work item statuses.
      dependencies: ['visual-regression'],
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
