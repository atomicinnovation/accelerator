import { test, expect } from '@playwright/test'
import { LIGHT_COLOR_TOKENS, DARK_COLOR_TOKENS } from '../../src/styles/tokens'

function hexToRgb(hex: string): string {
  const v = hex.replace('#', '')
  const r = parseInt(v.slice(0, 2), 16)
  const g = parseInt(v.slice(2, 4), 16)
  const b = parseInt(v.slice(4, 6), 16)
  return `rgb(${r}, ${g}, ${b})`
}

function parseRgb(rgb: string): [number, number, number] {
  const m = rgb.match(/rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/)
  if (!m) throw new Error(`Cannot parse colour: ${rgb}`)
  return [Number(m[1]), Number(m[2]), Number(m[3])]
}

function expectChannelsBetween(actual: string, a: string, b: string) {
  const [ar, ag, ab] = parseRgb(actual)
  const [xr, xg, xb] = parseRgb(a)
  const [yr, yg, yb] = parseRgb(b)
  expect(ar).toBeGreaterThanOrEqual(Math.min(xr, yr))
  expect(ar).toBeLessThanOrEqual(Math.max(xr, yr))
  expect(ag).toBeGreaterThanOrEqual(Math.min(xg, yg))
  expect(ag).toBeLessThanOrEqual(Math.max(xg, yg))
  expect(ab).toBeGreaterThanOrEqual(Math.min(xb, yb))
  expect(ab).toBeLessThanOrEqual(Math.max(xb, yb))
}

const COLOUR_CODED = ['indigo', 'green', 'amber', 'red', 'violet'] as const

const EXPECTED_FG_LIGHT: Record<(typeof COLOUR_CODED)[number], string> = {
  indigo: hexToRgb(LIGHT_COLOR_TOKENS['ac-accent']),
  green:  hexToRgb(LIGHT_COLOR_TOKENS['ac-ok']),
  amber:  hexToRgb(LIGHT_COLOR_TOKENS['ac-warn']),
  red:    hexToRgb(LIGHT_COLOR_TOKENS['ac-err']),
  violet: hexToRgb(LIGHT_COLOR_TOKENS['ac-violet']),
}

const EXPECTED_FG_DARK: Record<(typeof COLOUR_CODED)[number], string> = {
  indigo: hexToRgb(DARK_COLOR_TOKENS['ac-accent']),
  green:  hexToRgb(DARK_COLOR_TOKENS['ac-ok']),
  amber:  hexToRgb(DARK_COLOR_TOKENS['ac-warn']),
  red:    hexToRgb(DARK_COLOR_TOKENS['ac-err']),
  // --ac-violet has no dark override; remains theme-invariant.
  violet: hexToRgb(LIGHT_COLOR_TOKENS['ac-violet']),
}

for (const theme of ['light', 'dark'] as const) {
  test.describe(`chip-resolved-colours (${theme})`, () => {
    test.beforeEach(async ({ page }) => {
      await page.goto('/chip-showcase')
      if (theme === 'dark') {
        await page.evaluate(() => {
          document.documentElement.dataset.theme = 'dark'
        })
        await page.waitForFunction(() => document.documentElement.dataset.theme === 'dark')
      }
    })

    for (const variant of COLOUR_CODED) {
      test(`${variant}: foreground matches the resolved token hex`, async ({ page }) => {
        const chip = page.locator(
          `[data-testid="chip-cell-${variant}-sm"] [data-variant="${variant}"]`,
        )
        const fg = await chip.evaluate((el) => getComputedStyle(el).color)
        const expected = theme === 'light' ? EXPECTED_FG_LIGHT[variant] : EXPECTED_FG_DARK[variant]
        expect(fg).toBe(expected)
      })

      test(`${variant}: background sits between --ac-bg and the semantic colour`, async ({ page }) => {
        const chip = page.locator(
          `[data-testid="chip-cell-${variant}-sm"] [data-variant="${variant}"]`,
        )
        const bg = await chip.evaluate((el) => getComputedStyle(el).backgroundColor)
        const acBg = await page.evaluate(() =>
          getComputedStyle(document.documentElement).getPropertyValue('--ac-bg').trim(),
        )
        const expectedFg = theme === 'light' ? EXPECTED_FG_LIGHT[variant] : EXPECTED_FG_DARK[variant]
        expectChannelsBetween(bg, expectedFg, hexToRgb(acBg))
      })
    }
  })
}
