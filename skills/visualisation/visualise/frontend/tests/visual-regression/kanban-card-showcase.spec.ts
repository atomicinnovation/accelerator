import { test, expect } from '@playwright/test'

// Drag-state visual-regression oracle (A1). Captured from the static showcase
// surface (not a live drag) so the frame is reproducible, constrained to the
// card element to minimise the rotated-text region under pixel comparison, with
// animations disabled. The grab/grabbing cursor is OS-rendered and never
// appears in a screenshot — it is verified only in kanban-card-resolved-styles.
const STATES = ['resting', 'dragging', 'overlay'] as const
const VIEWPORT = { width: 1024, height: 768 }

for (const theme of ['light', 'dark'] as const) {
  test.describe(`kanban-card-showcase (${theme})`, () => {
    test.describe.configure({ mode: 'parallel' })

    test.beforeEach(async ({ page }) => {
      await page.setViewportSize(VIEWPORT)
      await page.goto('/kanban-card-showcase')
      if (theme === 'dark') {
        await page.evaluate(() => {
          document.documentElement.dataset.theme = 'dark'
        })
        await page.waitForFunction(
          () => document.documentElement.dataset.theme === 'dark',
        )
      }
      // Wait for the real glyphs so wrapped/measured text is stable across the
      // swap fallback.
      await page.evaluate(() => document.fonts.ready)
    })

    for (const state of STATES) {
      test(`${state}`, async ({ page }) => {
        const card = page.locator(
          `[data-testid="kanban-card-cell-${state}"] .ac-kcard`,
        )
        await expect(card).toHaveScreenshot(`drag-${state}-${theme}.png`, {
          maxDiffPixelRatio: 0.05,
          animations: 'disabled',
        })
      })
    }
  })
}
