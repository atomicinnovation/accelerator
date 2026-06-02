import { test, expect } from '@playwright/test'
import { setTheme, resolveToken } from './lib/expected-colours'

test.use({ viewport: { width: 1280, height: 720 } })

// `p > code` (not `:not(pre) > code`) so the prose locator can never resolve
// to a table-cell `<code>` regardless of fixture append order.
const PROSE_CODE = '[class*="markdown"] p > code'

for (const theme of ['light', 'dark'] as const) {
  test(`inline code is a monospace pill (${theme})`, async ({ page }) => {
    await page.goto('/library/plans/first-plan')
    if (theme === 'dark') await setTheme(page, 'dark')

    const el = page.locator(PROSE_CODE).first()
    const s = await el.evaluate((n) => {
      const c = getComputedStyle(n)
      return {
        fontFamily: c.fontFamily, fontSize: c.fontSize,
        backgroundColor: c.backgroundColor,
        borderTopWidth: c.borderTopWidth, borderTopStyle: c.borderTopStyle,
        borderTopColor: c.borderTopColor,
        borderTopLeftRadius: c.borderTopLeftRadius,
        paddingTop: c.paddingTop, paddingLeft: c.paddingLeft,
      }
    })

    // Theme-invariant chrome (AC2 dimensions + AC3 size + AC1 face).
    expect(s.fontFamily).toContain('Fira Code')
    expect(s.fontSize).toBe('11.5px')
    expect(s.borderTopWidth).toBe('1px')
    expect(s.borderTopStyle).toBe('solid')
    expect(s.borderTopLeftRadius).toBe('3px')
    expect(s.paddingTop).toBe('1px')
    expect(s.paddingLeft).toBe('5px')

    // Theme-varying colours (AC2 colour + AC5): resolve the tokens through the
    // cascade and compare exactly. Light bg #f4f6fa / dark #070b12; light
    // stroke rgba(32,34,49,0.06) / dark rgba(255,255,255,0.04).
    expect(s.backgroundColor).toBe(await resolveToken(page, '--ac-bg-sunken'))
    expect(s.borderTopColor).toBe(await resolveToken(page, '--ac-stroke-soft'))

    // AC1 contrast: the prose body must NOT be the mono face. Guard the
    // precondition that the default font mode is active — [data-font="mono"]
    // repoints --ac-font-body to mono and would collapse the contrast.
    const fontMode = await page.evaluate(
      () => document.documentElement.dataset.font ?? 'default',
    )
    expect(fontMode).not.toBe('mono')
    const proseFont = await page
      .locator('[class*="markdown"] p')
      .first()
      .evaluate((n) => getComputedStyle(n).fontFamily)
    expect(proseFont).not.toContain('Fira Code')
  })
}

// AC5 divergence: the pill colours must actually change between themes, not
// merely resolve to *a* token value — otherwise a theme-invariant token would
// pass both per-theme branches above trivially.
test('inline-code pill colours diverge between light and dark', async ({ page }) => {
  await page.goto('/library/plans/first-plan')
  const lightBg = await resolveToken(page, '--ac-bg-sunken')
  const lightBorder = await resolveToken(page, '--ac-stroke-soft')
  await setTheme(page, 'dark')
  expect(await resolveToken(page, '--ac-bg-sunken')).not.toBe(lightBg)
  expect(await resolveToken(page, '--ac-stroke-soft')).not.toBe(lightBorder)
})
