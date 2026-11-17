import { availableParallelism } from "node:os";
import { defineConfig } from "@playwright/test";
import { allocateWorkers } from "./scripts/worker-allocation";

const ALLOCATED_HEALTH_PORT = process.env.E2E_HEALTH_PORT;
const HEALTH_PORT = Number(ALLOCATED_HEALTH_PORT ?? 19087);
const allocation = allocateWorkers(availableParallelism());

export default defineConfig({
  testDir: "./e2e",
  timeout: 30_000,
  retries: 0,
  workers: allocation.total,
  globalSetup: "./e2e/global-setup.ts",
  globalTeardown: "./e2e/global-teardown.ts",
  use: {
    // BASE_URL is populated by globalSetup (which runs after webServer).
    baseURL: process.env.BASE_URL ?? "http://127.0.0.1",
    trace: "retain-on-failure",
  },
  projects: [
    {
      // Runs concurrently with chromium, so no probe here may measure an
      // element that comes from a fixture a chromium spec writes.
      name: "resolved-styles",
      testDir: "./tests/resolved-styles",
      workers: allocation.resolvedStyles,
      use: { browserName: "chromium" },
    },
    {
      // One worker, because its drag specs mutate shared fixtures in place;
      // globalSetup and globalTeardown restore them.
      name: "chromium",
      workers: allocation.chromium,
      use: { browserName: "chromium" },
    },
  ],
  webServer: {
    command: "node e2e/start-server.mjs",
    // Health server returns 200 once the real server is up.
    url: `http://127.0.0.1:${HEALTH_PORT}`,
    // A port allocated for this run has nothing legitimate to reuse; refusing
    // reuse makes a port stolen since allocation fail loudly instead of
    // pointing these specs at another checkout's server.
    reuseExistingServer: !process.env.CI && ALLOCATED_HEALTH_PORT === undefined,
    timeout: 60_000,
    stdout: "pipe",
    stderr: "pipe",
  },
});
