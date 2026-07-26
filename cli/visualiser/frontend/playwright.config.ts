import { defineConfig } from "@playwright/test";

const HEALTH_PORT = Number(process.env.E2E_HEALTH_PORT ?? 19087);

export default defineConfig({
  testDir: "./e2e",
  timeout: 30_000,
  retries: 1,
  // Force sequential execution across all projects so that the chromium
  // kanban drag tests cannot run concurrently with each other or with the
  // visual-regression snapshot tests (both touch shared fixture files or
  // compare screenshots of fixture-driven content).
  workers: 1,
  globalSetup: "./e2e/global-setup.ts",
  globalTeardown: "./e2e/global-teardown.ts",
  use: {
    // BASE_URL is populated by globalSetup (which runs after webServer).
    baseURL: process.env.BASE_URL ?? "http://127.0.0.1",
    trace: "on-first-retry",
  },
  projects: [
    {
      // Computed-style specs only — no screenshots. The screenshot-emitting
      // visual-regression specs live in playwright.docker.config.ts and run
      // only in the pinned Docker image. These hit static showcase routes, so
      // they must stay independent of the chromium drag tests' fixture
      // mutation/restore order (project order is unspecified under workers:1);
      // do not add a resolved-styles spec that reads live work-item status.
      name: "resolved-styles",
      testDir: "./tests/resolved-styles",
      use: { browserName: "chromium" },
    },
    {
      // No dependency on a snapshot project: the clean-fixture-before-drag
      // guarantee is now provided by globalSetup/globalTeardown restore, not by
      // an inter-project dependency. Do not add a native spec that relies on
      // observing pre-drag clean fixture state.
      name: "chromium",
      use: { browserName: "chromium" },
    },
  ],
  webServer: {
    command: "node e2e/start-server.mjs",
    // Health server (fixed port) returns 200 once the real server is up.
    url: `http://127.0.0.1:${HEALTH_PORT}`,
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
    stdout: "pipe",
    stderr: "pipe",
  },
});
