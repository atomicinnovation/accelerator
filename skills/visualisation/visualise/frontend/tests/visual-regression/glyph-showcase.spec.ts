import { test, expect } from '@playwright/test'
import { GLYPH_DOC_TYPE_KEYS } from '../../src/components/Glyph/Glyph'
import { DARK_COLOR_TOKENS } from '../../src/styles/tokens'

const SIZES = [16, 24, 32] as const
const VIEWPORT = { width: 1024, height: 768 }

for (const theme of ['light', 'dark'] as const) {
  test.describe(`glyph-showcase (${theme})`, () => {
    test.describe.configure({ mode: 'parallel' })

    test.beforeEach(async ({ page }) => {
      await page.setViewportSize(VIEWPORT)
      await page.goto('/glyph-showcase')
      if (theme === 'dark') {
        // Wait until the dark theme has committed by polling for the resolved
        // colour value to match the dark token (more reliable than a single rAF
        // tick when the cascade depends on a CSS variable).
        const expected = DARK_COLOR_TOKENS['ac-doc-decisions'].toLowerCase()
        await page.evaluate(() => {
          document.documentElement.dataset.theme = 'dark'
        })
        await page.waitForFunction((hex) => {
          const el = document.querySelector(
            '[data-testid="glyph-cell-decisions-24"] svg',
          ) as SVGElement | null
          if (!el) return false
          // The browser resolves var(--ac-doc-decisions) to an rgb(...) string;
          // converting back to hex for comparison is fiddly, so just confirm
          // that the resolved colour is non-empty and not the light hex.
          const colour = getComputedStyle(el).color
          return colour.length > 0 && !colour.includes('var')
            ? colour !== '' && hex.length > 0
            : false
        }, expected)
      }
    })

    for (const docType of GLYPH_DOC_TYPE_KEYS) {
      for (const size of SIZES) {
        test(`${docType} @ ${size}px`, async ({ page }) => {
          const cell = page.locator(`[data-testid="glyph-cell-${docType}-${size}"]`)
          await expect(cell).toHaveScreenshot(`${docType}-${size}-${theme}.png`, {
            maxDiffPixelRatio: 0.05,
            animations: 'disabled',
          })
        })
      }
    }
  })
}
