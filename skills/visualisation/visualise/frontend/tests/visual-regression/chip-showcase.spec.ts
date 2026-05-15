import { test, expect } from '@playwright/test'

const VARIANTS = ['neutral', 'indigo', 'green', 'amber', 'red', 'violet'] as const
const SIZES = ['sm', 'md'] as const
const VIEWPORT = { width: 1024, height: 768 }

for (const theme of ['light', 'dark'] as const) {
  test.describe(`chip-showcase (${theme})`, () => {
    test.describe.configure({ mode: 'parallel' })

    test.beforeEach(async ({ page }) => {
      await page.setViewportSize(VIEWPORT)
      await page.goto('/chip-showcase')
      if (theme === 'dark') {
        await page.evaluate(() => {
          document.documentElement.dataset.theme = 'dark'
        })
        // Poll until the dark theme has committed by waiting for the resolved
        // color value to change. More reliable than a single rAF tick when the
        // cascade depends on a CSS variable.
        await page.waitForFunction(() => {
          const el = document.querySelector(
            '[data-testid="chip-cell-green-sm"] [data-variant="green"]',
          ) as HTMLElement | null
          if (!el) return false
          const colour = getComputedStyle(el).color
          return colour.length > 0 && !colour.includes('var')
        })
      }
    })

    for (const variant of VARIANTS) {
      for (const size of SIZES) {
        test(`${variant} @ ${size}`, async ({ page }) => {
          const cell = page.locator(`[data-testid="chip-cell-${variant}-${size}"]`)
          await expect(cell).toHaveScreenshot(`${variant}-${size}-${theme}.png`, {
            maxDiffPixelRatio: 0.05,
            animations: 'disabled',
          })
        })
      }
    }
  })
}
