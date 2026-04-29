import { describe, it, expect } from 'vitest'
import globalCss from './global.css?raw'
import { contrastRatio } from './contrast'
import { COLOR_TOKENS } from './tokens'

function readCssVar(name: string): string | null {
  const re = new RegExp(`--${name}:\\s*([^;]+);`)
  const m = re.exec(globalCss)
  return m ? m[1].trim() : null
}

describe('tokens.ts is the single source of truth for :root colour values', () => {
  for (const [name, value] of Object.entries(COLOR_TOKENS)) {
    it(`global.css :root --${name} matches COLOR_TOKENS.${name}`, () => {
      expect(readCssVar(name)).toBe(value)
    })
  }
})

describe('design-token contrast (WCAG 2.2 AA)', () => {
  it('muted-text on white passes AA for normal text (4.5:1)', () => {
    expect(
      contrastRatio(COLOR_TOKENS['color-muted-text'], '#ffffff'),
    ).toBeGreaterThanOrEqual(4.5)
  })
  it('warning-text on warning-bg passes AA for normal text (4.5:1)', () => {
    expect(
      contrastRatio(COLOR_TOKENS['color-warning-text'], COLOR_TOKENS['color-warning-bg']),
    ).toBeGreaterThanOrEqual(4.5)
  })
  it('focus-ring on white passes AA for UI components (3:1)', () => {
    expect(
      contrastRatio(COLOR_TOKENS['color-focus-ring'], '#ffffff'),
    ).toBeGreaterThanOrEqual(3)
  })
})
