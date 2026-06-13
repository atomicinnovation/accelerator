import { defineConfig } from "@playwright/test";

function required(name: string): string {
  const value = process.env[name];
  if (!value) {
    throw new Error(
      `${name} must be set by the docker invoke task — refusing to run ` +
        `against an unconfigured origin/channel/locale.`,
    );
  }
  return value;
}

// Visual-regression screenshot specs run only here, inside the pinned Playwright
// Linux image. The Rust server runs on the host; this config reaches it via
// BASE_URL. channel and locale come from the single shared source
// (tasks/shared/playwright.py) via env, never hardcoded here — see that module.
export default defineConfig({
  testDir: "./tests/visual-regression",
  snapshotDir: "./tests/visual-regression/__screenshots__",
  // Single canonical baseline — no {platform} token; rendering is always the
  // pinned container.
  snapshotPathTemplate:
    "{snapshotDir}/{testFileName}-snapshots/{arg}{-projectName}{ext}",
  timeout: 30_000,
  retries: 1,
  workers: 1,
  use: {
    baseURL: required("BASE_URL"),
    channel: required("CHROMIUM_CHANNEL"),
    locale: required("PLAYWRIGHT_LOCALE"),
    trace: "on-first-retry",
  },
  projects: [
    {
      name: "visual-regression",
      use: { browserName: "chromium" },
    },
  ],
});
