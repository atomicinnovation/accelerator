import { defineConfig } from '@playwright/test'

// Standalone Playwright config used to generate / verify the Glyph showcase
// baselines. The default config (playwright.config.ts) brings up the Rust
// visualiser server because most specs need /api/* data — those need
// platform-specific Rust artefacts that are awkward to produce inside a
// Playwright Docker image. The Glyph specs only need the frontend, so this
// config serves the built `dist/` via `vite preview` and shares the same
// snapshot directory as the default config so the linux baselines land
// alongside the darwin ones.
const PORT = Number(process.env.GLYPH_PREVIEW_PORT ?? 4173)

export default defineConfig({
  testDir: './tests/visual-regression',
  // Only the Glyph specs — other visual-regression specs (tokens.spec.ts)
  // depend on the visualiser server and must run via playwright.config.ts.
  testMatch: ['glyph-showcase.spec.ts', 'glyph-resolved-fill.spec.ts'],
  snapshotDir: './tests/visual-regression/__screenshots__',
  timeout: 30_000,
  retries: 0,
  workers: 1,
  use: {
    baseURL: `http://127.0.0.1:${PORT}`,
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'visual-regression',
      use: { browserName: 'chromium' },
    },
  ],
  webServer: {
    command: `npx vite preview --host 127.0.0.1 --port ${PORT} --strictPort`,
    url: `http://127.0.0.1:${PORT}/glyph-showcase`,
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
    stdout: 'pipe',
    stderr: 'pipe',
  },
})
