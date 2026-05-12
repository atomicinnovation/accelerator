import { describe, it, expect } from 'vitest'
import globalCss from './global.css?raw'
import {
  LIGHT_COLOR_TOKENS,
  DARK_COLOR_TOKENS,
  TYPOGRAPHY_TOKENS,
  SPACING_TOKENS,
  RADIUS_TOKENS,
  LIGHT_SHADOW_TOKENS,
  DARK_SHADOW_TOKENS,
  LAYOUT_TOKENS,
  MONO_FONT_TOKENS,
} from './tokens'
import { contrastRatio } from './contrast'
import { DOC_TYPE_KEYS, DOC_TYPE_LABELS, VIRTUAL_DOC_TYPE_KEYS, type DocTypeKey } from '../api/types'

type Scope = 'root' | 'dark'

/**
 * Reads a CSS custom property's declared value from the relevant top-level
 * block in `global.css`. Comparison is case-insensitive on the value side
 * so hex casing differences (e.g. `#FBFCFE` vs `#fbfcfe`) do not break
 * parity.
 *
 * INVARIANT: the captured block must be flat — no nested selectors, no
 * `@media` wrappers, no CSS nesting inside `:root` or `[data-theme="dark"]`.
 * The non-greedy regex would silently truncate at the first inner `}`.
 */
function readCssVar(name: string, scope: Scope = 'root'): string | null {
  const blockRe =
    scope === 'root'
      ? /:root\s*\{([\s\S]*?)\}/
      : /\[data-theme="dark"\]\s*\{([\s\S]*?)\}/
  const block = blockRe.exec(globalCss)?.[1] ?? ''
  // Defensive: token names today are kebab-case alphanumeric, but escape
  // metacharacters so a future contributor passing an unusual name doesn't
  // silently get a corrupted match.
  const escapedName = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  const re = new RegExp(`--${escapedName}:\\s*([^;]+);`)
  return re.exec(block)?.[1].trim().toLowerCase() ?? null
}

function expectMatches(actual: string | null, expected: string): void {
  expect(actual).toBe(expected.toLowerCase())
}

describe('global focus rings', () => {
  it('declares :focus-visible with an outline', () => {
    expect(globalCss).toMatch(/:focus-visible\s*\{[^}]*outline:[^;]+;/)
  })
  it('declares an outline-offset for breathing room', () => {
    expect(globalCss).toMatch(/:focus-visible\s*\{[^}]*outline-offset:[^;]+;/)
  })
  it('overrides the focus-ring colour under forced-colors mode', () => {
    expect(globalCss).toMatch(
      /@media\s*\(forced-colors:\s*active\)\s*\{[^}]*:focus-visible[^}]*outline-color:\s*Highlight/i,
    )
  })
})

describe('tokens.ts ↔ global.css :root parity (light colour)', () => {
  for (const [name, value] of Object.entries(LIGHT_COLOR_TOKENS)) {
    it(`--${name} matches LIGHT_COLOR_TOKENS.${name}`, () => {
      expectMatches(readCssVar(name, 'root'), value)
    })
  }
})

describe('tokens.ts ↔ global.css [data-theme="dark"] parity (dark colour)', () => {
  for (const [name, value] of Object.entries(DARK_COLOR_TOKENS)) {
    it(`--${name} matches DARK_COLOR_TOKENS.${name}`, () => {
      expectMatches(readCssVar(name, 'dark'), value)
    })
  }
})

describe('tokens.ts ↔ global.css [data-theme="dark"] parity (dark shadow)', () => {
  for (const [name, value] of Object.entries(DARK_SHADOW_TOKENS)) {
    it(`--${name} matches DARK_SHADOW_TOKENS.${name}`, () => {
      expectMatches(readCssVar(name, 'dark'), value)
    })
  }
})

describe.each([
  ['typography', TYPOGRAPHY_TOKENS],
  ['spacing', SPACING_TOKENS],
  ['radius', RADIUS_TOKENS],
  ['light shadow', LIGHT_SHADOW_TOKENS],
  ['layout', LAYOUT_TOKENS],
])('tokens.ts ↔ global.css :root parity (%s)', (_label, tokens) => {
  for (const [name, value] of Object.entries(tokens)) {
    it(`--${name} matches`, () => {
      expectMatches(readCssVar(name, 'root'), value)
    })
  }
})

/**
 * The `[data-theme="dark"]` block and the `@media (prefers-color-scheme: dark)`
 * mirror block are hand-maintained duplicates of the same dark token values.
 * `readCssVar` cannot read the mirror block (its flat-block invariant
 * forbids `@media` wrappers), so we use a separate two-step extraction
 * here: first capture the `@media` body, then capture the inner
 * `:root:not([data-theme="light"])` block, and compare its declarations
 * against the explicit `[data-theme="dark"]` block.
 */
/** Extract a `{ ... }` block body starting at the first `{` after `index`,
 *  using brace-balanced scanning so nested rules don't truncate. Returns
 *  the body (without the enclosing braces) or `undefined` if no balanced
 *  block exists at that position. Resilient to formatter changes (no
 *  column-0 anchor required). */
function extractBlockBody(source: string, index: number): string | undefined {
  const open = source.indexOf('{', index)
  if (open === -1) return undefined
  let depth = 1
  for (let i = open + 1; i < source.length; i++) {
    if (source[i] === '{') depth++
    else if (source[i] === '}') {
      depth--
      if (depth === 0) return source.slice(open + 1, i)
    }
  }
  return undefined
}

describe('global.css [data-theme="dark"] ↔ @media (prefers-color-scheme: dark) parity', () => {
  it('the two dark blocks declare the same tokens with the same values', () => {
    const explicitMatch = /\[data-theme="dark"\]\s*\{/.exec(globalCss)
    const explicit = explicitMatch
      ? extractBlockBody(globalCss, explicitMatch.index)
      : undefined

    const mediaMatch = /@media\s*\(prefers-color-scheme:\s*dark\)\s*\{/.exec(globalCss)
    const mediaBody = mediaMatch
      ? extractBlockBody(globalCss, mediaMatch.index)
      : undefined
    const innerMatch = mediaBody
      ? /:root:not\(\[data-theme="light"\]\)\s*\{/.exec(mediaBody)
      : null
    const mirror = innerMatch
      ? extractBlockBody(mediaBody!, innerMatch.index)
      : undefined

    expect(explicit, 'failed to extract [data-theme="dark"] body').toBeDefined()
    expect(mirror, 'failed to extract prefers-color-scheme inner block').toBeDefined()

    const normalise = (s: string): Map<string, string> => {
      const map = new Map<string, string>()
      for (const m of s.matchAll(/--([\w-]+):\s*([^;]+);/g)) {
        map.set(m[1], m[2].trim().toLowerCase())
      }
      return map
    }
    const a = normalise(explicit!)
    const b = normalise(mirror!)

    // Sets of declared property names match (catches "added in one, forgot the other")
    expect([...a.keys()].sort()).toEqual([...b.keys()].sort())
    // Each name has the same value in both blocks
    for (const [name, value] of a) {
      expect(b.get(name)).toBe(value)
    }
  })
})

describe('global.css @keyframes ac-pulse', () => {
  it('declares @keyframes ac-pulse', () => {
    expect(globalCss).toMatch(/@keyframes\s+ac-pulse\s*\{/)
  })
  it('has the canonical body (0%/100% opacity:1, 50% opacity:0.4)', () => {
    expect(globalCss).toContain('0%, 100% { opacity: 1; }')
    expect(globalCss).toContain('50% { opacity: 0.4; }')
  })
})

/**
 * Sanity guard: catch the silent-truncation failure mode of the flat-block
 * regex in `readCssVar`. If a future contributor introduces a nested rule
 * inside `:root` or `[data-theme="dark"]`, the non-greedy match terminates
 * at the inner `}` and tokens declared after it return `null`. By asserting
 * one known-last token from each block, the truncation produces a hard
 * failure rather than passing on a partial extraction.
 */
describe('readCssVar truncation guard', () => {
  it(':root block extends past --ac-topbar-h', () => {
    expect(readCssVar('ac-topbar-h', 'root')).not.toBeNull()
  })
  it('[data-theme="dark"] block extends past --ac-shadow-lift', () => {
    expect(readCssVar('ac-shadow-lift', 'dark')).not.toBeNull()
  })
})

function findBlockBodyForSelector(css: string, selector: string): string | null {
  const idx = css.indexOf(selector + ' ')
  if (idx === -1) return null
  return extractBlockBody(css, idx) ?? null
}

function countTopLevelBodyRules(css: string): number {
  const stripped = css.replace(/@[^{]+\{(?:[^{}]|\{[^}]*\})*\}/g, '')
  return (stripped.match(/(^|\s|,)body\s*\{/g) ?? []).length
}

function readMonoVar(name: string): string | null {
  const blockRe = /\[data-font="mono"\]\s*\{([\s\S]*?)\}/
  const block = blockRe.exec(globalCss)?.[1] ?? ''
  const escapedName = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  const re = new RegExp(`--${escapedName}:\\s*([^;]+);`)
  return re.exec(block)?.[1].trim().toLowerCase() ?? null
}

describe('tokens.ts ↔ global.css [data-font="mono"] parity', () => {
  for (const [name, value] of Object.entries(MONO_FONT_TOKENS)) {
    it(`--${name} matches MONO_FONT_TOKENS.${name}`, () => {
      expectMatches(readMonoVar(name), value)
    })
  }
})

describe('DOC_TYPE_LABELS ↔ DOC_TYPE_KEYS parity', () => {
  it('every DocTypeKey has a label', () => {
    expect(Object.keys(DOC_TYPE_LABELS).sort()).toEqual([...DOC_TYPE_KEYS].sort())
  })
})

describe('--ac-doc-* tokens meet WCAG 1.4.11 ≥3:1 contrast vs --ac-bg', () => {
  const BG_LIGHT = LIGHT_COLOR_TOKENS['ac-bg']
  const BG_DARK = DARK_COLOR_TOKENS['ac-bg']
  const glyphKeys = DOC_TYPE_KEYS.filter(
    (k): k is DocTypeKey => !VIRTUAL_DOC_TYPE_KEYS.includes(k),
  )
  for (const key of glyphKeys) {
    const tokenName = `ac-doc-${key}` as const
    it(`light: ${key} contrast >= 3:1 vs --ac-bg`, () => {
      const fg = (LIGHT_COLOR_TOKENS as Record<string, string>)[tokenName]
      expect(fg, `LIGHT_COLOR_TOKENS missing ${tokenName}`).toBeTruthy()
      expect(contrastRatio(fg, BG_LIGHT)).toBeGreaterThanOrEqual(3)
    })
    it(`dark: ${key} contrast >= 3:1 vs --ac-bg`, () => {
      const fg = (DARK_COLOR_TOKENS as Record<string, string>)[tokenName]
      expect(fg, `DARK_COLOR_TOKENS missing ${tokenName}`).toBeTruthy()
      expect(contrastRatio(fg, BG_DARK)).toBeGreaterThanOrEqual(3)
    })
  }
})

describe('global body/html token consumption', () => {
  it('there is exactly one top-level body rule', () => {
    expect(countTopLevelBodyRules(globalCss)).toBe(1)
  })

  it('body declares background-color: var(--ac-bg)', () => {
    const body = findBlockBodyForSelector(globalCss, 'body')
    expect(body).not.toBeNull()
    expect(body!).toMatch(/background-color:\s*var\(--ac-bg\)/)
  })

  it('body declares color: var(--ac-fg)', () => {
    const body = findBlockBodyForSelector(globalCss, 'body')
    expect(body).not.toBeNull()
    expect(body!).toMatch(/(?<!background-)color:\s*var\(--ac-fg\)/)
  })

  it(':root declares color-scheme: light dark', () => {
    const root = findBlockBodyForSelector(globalCss, ':root')
    expect(root).not.toBeNull()
    expect(root!).toMatch(/color-scheme:\s*light\s+dark/)
  })
})
