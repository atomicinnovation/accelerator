import { test, expect } from '@playwright/test'
import { LIGHT_COLOR_TOKENS, DARK_COLOR_TOKENS } from '../../src/styles/tokens'
import { DOC_TYPE_KEYS } from '../../src/api/types'
import { DOC_TYPE_TOKEN_KEY } from '../../src/components/Glyph/Glyph.constants'

function hexToRgb(hex: string): string {
  const v = hex.replace('#', '')
  const r = parseInt(v.slice(0, 2), 16)
  const g = parseInt(v.slice(2, 4), 16)
  const b = parseInt(v.slice(4, 6), 16)
  return `rgb(${r}, ${g}, ${b})`
}

const TOKEN_TABLE = {
  light: LIGHT_COLOR_TOKENS,
  dark: DARK_COLOR_TOKENS,
} as const

// Verifies AC #4's resolved-hex contract using a real Chromium engine: that
// `getComputedStyle(svg).color` substitutes each Glyph's colour var to the
// canonical hex. JSDOM does not reliably substitute `var()` in SVG inline
// styles, hence this is a Playwright spec rather than a Vitest one.
//
// Parametrised over all 13 doc types × {light, dark}; expected hex looked up
// via the typed DOC_TYPE_TOKEN_KEY map (templates → ac-fg-muted, the other
// 12 → ac-doc-<key>).
for (const theme of ['light', 'dark'] as const) {
  test.describe(`glyph resolved fill — ${theme}`, () => {
    for (const docType of DOC_TYPE_KEYS) {
      test(`${docType}`, async ({ page }) => {
        await page.goto('/glyph-showcase')
        if (theme === 'dark') {
          await page.evaluate(() => {
            document.documentElement.dataset.theme = 'dark'
          })
          await page.waitForFunction(
            () => document.documentElement.dataset.theme === 'dark',
          )
        }
        const colour = await page
          .locator(`[data-testid="glyph-cell-${docType}-24"] svg`)
          .evaluate((el) => getComputedStyle(el).color)
        const tokenKey = DOC_TYPE_TOKEN_KEY[docType]
        expect(colour).toBe(hexToRgb(TOKEN_TABLE[theme][tokenKey]))
      })
    }
  })
}
