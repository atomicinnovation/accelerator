import { test, expect } from '@playwright/test'
import { DOC_TYPE_KEYS } from '../../src/api/types'
import { EXPECTED_COLOR, hexToRgb, setTheme } from './lib/expected-colours'
import { DETAIL_ROUTE_SLUGS } from './lib/detail-route-slugs'
import { LIGHT_COLOR_TOKENS, DARK_COLOR_TOKENS } from '../../src/styles/tokens'

const THEMES = ['light', 'dark'] as const

for (const theme of THEMES) {
  test.describe(`eyebrow icon colour — ${theme}`, () => {
    for (const docType of DOC_TYPE_KEYS) {
      test(`${docType}`, async ({ page }) => {
        await page.goto(`/library/${docType}/${DETAIL_ROUTE_SLUGS[docType]}`)
        await setTheme(page, theme)

        // Target the SVG directly — data-doc-type is on the svg for both
        // framed and unframed Glyphs. Scope to the eyebrow region.
        const icon = page.locator(
          `[data-slot="eyebrow"] svg[data-doc-type="${docType}"]`,
        )
        await expect(icon).toBeVisible()
        const color = await icon.evaluate((el) => getComputedStyle(el).color)
        expect(color).toBe(hexToRgb(EXPECTED_COLOR[docType][theme]))
      })
    }
  })

  // Single theme-level assertion: the eyebrow LABEL TEXT colour resolves
  // to --ac-fg-faint (theme-invariant across all 13 doc types). The text
  // node inherits `color` from the wrapping eyebrow-label span (which
  // inherits from .eyebrow's --ac-fg-faint rule); the Glyph's inline
  // `color` is set on the inner <svg>, so it does NOT propagate upward.
  test(`eyebrow label text colour is --ac-fg-faint — ${theme}`, async ({
    page,
  }) => {
    await page.goto(`/library/decisions/${DETAIL_ROUTE_SLUGS.decisions}`)
    await setTheme(page, theme)
    const eyebrowText = page.locator(
      '[data-slot="eyebrow"] [data-testid="eyebrow-label"]',
    )
    const color = await eyebrowText.evaluate(
      (el) => getComputedStyle(el).color,
    )
    const tokens = theme === 'light' ? LIGHT_COLOR_TOKENS : DARK_COLOR_TOKENS
    expect(color).toBe(hexToRgb(tokens['ac-fg-faint']))
  })
}
