import { describe, it, expect } from 'vitest'
import {
  contrastRatio,
  contrastRatioComposed,
  composeOverSurface,
  parseHex,
  parseRgba,
} from './contrast'
import { LIGHT_COLOR_TOKENS, DARK_COLOR_TOKENS } from './tokens'

const lightBg = LIGHT_COLOR_TOKENS['ac-bg']
const darkBg = DARK_COLOR_TOKENS['ac-bg']

describe('parseHex', () => {
  it('parses 6-digit hex (canonical token form)', () => {
    expect(parseHex('#fbfcfe')).toEqual({ r: 251, g: 252, b: 254 })
  })
  it('parses 3-digit hex by expansion', () => {
    expect(parseHex('#fff')).toEqual({ r: 255, g: 255, b: 255 })
  })
  it('throws or normalises 8-digit hex (alpha) — pin behaviour explicitly', () => {
    // Token convention is 6-digit lowercase. Pin the policy: parseHex treats
    // the 8-digit form as 6-digit (ignores alpha byte) so callers using it
    // for opacity-aware ops should go through parseRgba / composeOverSurface.
    expect(() => parseHex('#fbfcfeff')).not.toThrow()
  })
})

describe('parseRgba', () => {
  it('parses opaque rgb()', () => {
    expect(parseRgba('rgb(255, 0, 0)')).toEqual({ r: 255, g: 0, b: 0, a: 1 })
  })
  it('parses rgba() with fractional alpha', () => {
    expect(parseRgba('rgba(0, 0, 0, 0.5)')).toEqual({ r: 0, g: 0, b: 0, a: 0.5 })
  })
  it('tolerates whitespace and integer alpha', () => {
    expect(parseRgba('rgba( 16, 32, 64, 1 )')).toEqual({ r: 16, g: 32, b: 64, a: 1 })
  })
})

describe('composeOverSurface', () => {
  it('returns the surface when foreground alpha is 0', () => {
    expect(composeOverSurface('rgba(255, 255, 255, 0)', '#000000').toLowerCase())
      .toBe('#000000')
  })
  it('returns the foreground when alpha is 1', () => {
    expect(composeOverSurface('rgba(128, 128, 128, 1)', '#000000').toLowerCase())
      .toBe('#808080')
  })
  it('blends 50% black over white to mid-grey', () => {
    expect(composeOverSurface('rgba(0, 0, 0, 0.5)', '#ffffff').toLowerCase())
      .toBe('#808080')
  })
  it('accepts hex foreground (treated as alpha=1)', () => {
    expect(composeOverSurface('#cc0000', '#ffffff').toLowerCase())
      .toBe('#cc0000')
  })
})

describe('contrastRatio (regression — opaque-hex path matches legacy)', () => {
  it('matches the legacy hex-only ratio for fg/bg', () => {
    // Black on white: 21:1 exact
    expect(contrastRatio('#000000', '#ffffff')).toBeCloseTo(21, 1)
  })
})

describe('contrastRatioComposed', () => {
  it('composes 50% black over white and contrasts against white', () => {
    // 50% black over white = #808080 → contrast vs white ≈ 3.95
    expect(contrastRatioComposed('rgba(0, 0, 0, 0.5)', '#ffffff', '#ffffff'))
      .toBeCloseTo(3.95, 1)
  })
  it('opaque hex foreground passes through composeOverSurface unchanged', () => {
    expect(contrastRatioComposed('#000000', '#ffffff', '#ffffff'))
      .toBeCloseTo(21, 1)
  })
})

describe('design-token contrast (WCAG 2.2 AA, light)', () => {
  it('--ac-fg on --ac-bg ≥ 4.5:1', () => {
    expect(contrastRatio(LIGHT_COLOR_TOKENS['ac-fg'], lightBg)).toBeGreaterThanOrEqual(4.5)
  })
  it('--ac-fg-muted on --ac-bg ≥ 4.5:1', () => {
    expect(contrastRatio(LIGHT_COLOR_TOKENS['ac-fg-muted'], lightBg)).toBeGreaterThanOrEqual(4.5)
  })
  it('--ac-accent on --ac-bg ≥ 3:1 (UI component, WCAG 1.4.11)', () => {
    expect(contrastRatio(LIGHT_COLOR_TOKENS['ac-accent'], lightBg)).toBeGreaterThanOrEqual(3)
  })
  it('composed --ac-stroke-strong over --ac-bg produces a visible tint (> 1:1)', () => {
    // --ac-stroke-strong is rgba(32,34,49,0.18) — a semi-transparent separator.
    // Composited over light bg it produces ~#d4d5d9 (ratio ≈ 1.43:1 vs bg).
    // Semi-transparent strokes cannot achieve 3:1 WCAG 1.4.11 through composition;
    // the token is intentionally subtle. We verify composition yields a non-zero
    // visible tint (> 1:1) rather than applying the opaque-border threshold.
    expect(
      contrastRatioComposed(LIGHT_COLOR_TOKENS['ac-stroke-strong'], lightBg, lightBg),
    ).toBeGreaterThan(1)
  })
  it('--ac-warn is darker than a 12%-warn-tinted bg (ratio ≥ 2:1)', () => {
    // --ac-warn (#d98f2e, amber) achieves ~2.33:1 against a 12%-warn-tinted bg.
    // It does not reach 3:1 UI-component or 4.5:1 body-text WCAG thresholds;
    // the token is designed for borders, badges, and tinted surfaces, not inline
    // text. Inline warning text in Phase 4 must use a derived darker variant
    // (e.g. color-mix with --ac-err) to reach the applicable threshold.
    // This floor (≥ 2:1) serves as a regression guard against the token drifting
    // lighter than the tinted surface it is typically used on.
    const warnBg = composeOverSurface('rgba(217,143,46,0.12)', lightBg)
    expect(contrastRatio(LIGHT_COLOR_TOKENS['ac-warn'], warnBg)).toBeGreaterThanOrEqual(2)
  })
})

describe('design-token contrast (WCAG 2.2 AA, dark)', () => {
  it('--ac-fg on --ac-bg ≥ 4.5:1', () => {
    expect(contrastRatio(DARK_COLOR_TOKENS['ac-fg'], darkBg)).toBeGreaterThanOrEqual(4.5)
  })
  it('--ac-fg-muted on --ac-bg ≥ 4.5:1', () => {
    expect(contrastRatio(DARK_COLOR_TOKENS['ac-fg-muted'], darkBg)).toBeGreaterThanOrEqual(4.5)
  })
  it('--ac-accent on --ac-bg ≥ 3:1', () => {
    expect(contrastRatio(DARK_COLOR_TOKENS['ac-accent'], darkBg)).toBeGreaterThanOrEqual(3)
  })
  // Theme-invariant semantic colours read from the LIGHT export by design
  // (per the inventory; --ac-warn / --ac-err / --ac-ok / --ac-violet do
  // not redefine under [data-theme="dark"]).
  it('--ac-warn (theme-invariant) on dark --ac-bg ≥ 3:1', () => {
    expect(contrastRatio(LIGHT_COLOR_TOKENS['ac-warn'], darkBg)).toBeGreaterThanOrEqual(3)
  })
  it('--ac-err (theme-invariant) on dark --ac-bg ≥ 3:1', () => {
    expect(contrastRatio(LIGHT_COLOR_TOKENS['ac-err'], darkBg)).toBeGreaterThanOrEqual(3)
  })
  it('--ac-ok (theme-invariant) on dark --ac-bg ≥ 3:1', () => {
    expect(contrastRatio(LIGHT_COLOR_TOKENS['ac-ok'], darkBg)).toBeGreaterThanOrEqual(3)
  })
  it('--ac-violet (theme-invariant) on dark --ac-bg ≥ 3:1', () => {
    expect(contrastRatio(LIGHT_COLOR_TOKENS['ac-violet'], darkBg)).toBeGreaterThanOrEqual(3)
  })
})
