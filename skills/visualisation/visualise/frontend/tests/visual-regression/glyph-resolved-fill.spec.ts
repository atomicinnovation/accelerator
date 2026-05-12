import { test, expect } from '@playwright/test'
import { LIGHT_COLOR_TOKENS, DARK_COLOR_TOKENS } from '../../src/styles/tokens'

function hexToRgb(hex: string): string {
  const v = hex.replace('#', '')
  const r = parseInt(v.slice(0, 2), 16)
  const g = parseInt(v.slice(2, 4), 16)
  const b = parseInt(v.slice(4, 6), 16)
  return `rgb(${r}, ${g}, ${b})`
}

// Verifies AC #4's resolved-hex contract using a real Chromium engine: that
// `getComputedStyle(svg).color` substitutes `var(--ac-doc-decisions)` to the
// canonical hex. JSDOM does not reliably substitute `var()` in SVG inline
// styles, hence this is a Playwright spec rather than a Vitest one.
test('Glyph resolves --ac-doc-decisions to the light hex', async ({ page }) => {
  await page.goto('/glyph-showcase')
  const colour = await page
    .locator('[data-testid="glyph-cell-decisions-24"] svg')
    .evaluate((el) => getComputedStyle(el).color)
  expect(colour).toBe(hexToRgb(LIGHT_COLOR_TOKENS['ac-doc-decisions']))
})

test('Glyph resolves --ac-doc-decisions to the dark hex after theme swap', async ({ page }) => {
  await page.goto('/glyph-showcase')
  await page.evaluate(() => {
    document.documentElement.dataset.theme = 'dark'
  })
  await page.waitForFunction(() => document.documentElement.dataset.theme === 'dark')
  const colour = await page
    .locator('[data-testid="glyph-cell-decisions-24"] svg')
    .evaluate((el) => getComputedStyle(el).color)
  expect(colour).toBe(hexToRgb(DARK_COLOR_TOKENS['ac-doc-decisions']))
})
