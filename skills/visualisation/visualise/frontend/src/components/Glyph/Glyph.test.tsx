import { describe, it, expect, vi, afterEach } from 'vitest'
import { render } from '@testing-library/react'
import { Glyph, GLYPH_DOC_TYPE_KEYS, isGlyphDocTypeKey, type GlyphDocTypeKey } from './Glyph'
import GlyphSource from './Glyph.tsx?raw'
import { DOC_TYPE_KEYS } from '../../api/types'

// Compile-time type-rejection guards. The @ts-expect-error directives fire
// when `typecheck` (tsc --noEmit) runs; `npm test` alone does not enforce
// them. Exported so `noUnusedLocals` doesn't elide the function — never
// called at runtime.
export function _typeContractGuards(): void {
  // @ts-expect-error — 'templates' is excluded from GlyphDocTypeKey.
  void (<Glyph docType="templates" size={24} />)
  // @ts-expect-error — size 20 is not 16 | 24 | 32.
  void (<Glyph docType="decisions" size={20} />)
}

describe('Glyph: type-level helpers', () => {
  it('GLYPH_DOC_TYPE_KEYS has 12 entries', () => {
    expect(GLYPH_DOC_TYPE_KEYS.length).toBe(12)
  })

  it('GLYPH_DOC_TYPE_KEYS excludes the virtual templates key', () => {
    expect(GLYPH_DOC_TYPE_KEYS).not.toContain('templates' as GlyphDocTypeKey)
  })

  it('isGlyphDocTypeKey accepts every non-virtual key', () => {
    for (const k of GLYPH_DOC_TYPE_KEYS) {
      expect(isGlyphDocTypeKey(k)).toBe(true)
    }
  })

  it('isGlyphDocTypeKey rejects the virtual templates key', () => {
    expect(isGlyphDocTypeKey('templates')).toBe(false)
  })

  it('GLYPH_DOC_TYPE_KEYS is the set of DOC_TYPE_KEYS minus virtuals', () => {
    const expected = DOC_TYPE_KEYS.filter(k => k !== 'templates')
    expect([...GLYPH_DOC_TYPE_KEYS].sort()).toEqual([...expected].sort())
  })
})

describe('Glyph: runtime DOM shape', () => {
  it('root element is <svg> with viewBox 0 0 24 24', () => {
    const { container } = render(<Glyph docType="decisions" size={24} />)
    const svg = container.querySelector('svg')
    expect(svg).not.toBeNull()
    expect(svg!.getAttribute('viewBox')).toBe('0 0 24 24')
  })

  it('width and height attributes match the requested size', () => {
    const { container } = render(<Glyph docType="decisions" size={32} />)
    const svg = container.querySelector('svg')!
    expect(svg.getAttribute('width')).toBe('32')
    expect(svg.getAttribute('height')).toBe('32')
  })

  it('inline style.color resolves to var(--ac-doc-<key>)', () => {
    const { container } = render(<Glyph docType="research" size={24} />)
    const svg = container.querySelector('svg') as SVGElement
    expect(svg.style.color).toBe('var(--ac-doc-research)')
  })

  it('carries data-doc-type attribute matching docType', () => {
    const { container } = render(<Glyph docType="plans" size={16} />)
    const svg = container.querySelector('svg')!
    expect(svg.getAttribute('data-doc-type')).toBe('plans')
  })

  it('every descendant fill is "currentColor" or "none" — never a hex (deep walk)', () => {
    // `"none"` is permitted for stroke-only shapes; only hex literals would
    // break the theme contract. Walk via querySelectorAll('*') (not children)
    // so paths nested inside <g> groups are inspected too.
    for (const docType of GLYPH_DOC_TYPE_KEYS) {
      const { container, unmount } = render(<Glyph docType={docType} size={24} />)
      const svg = container.querySelector('svg')!
      for (const node of Array.from(svg.querySelectorAll('*'))) {
        const fill = node.getAttribute('fill')
        if (fill !== null) {
          expect(
            fill === 'currentColor' || fill === 'none',
            `${docType}: descendant <${node.tagName}> has fill="${fill}"`,
          ).toBe(true)
        }
      }
      unmount()
    }
  })

  it('every descendant stroke is "currentColor" when present (deep walk)', () => {
    for (const docType of GLYPH_DOC_TYPE_KEYS) {
      const { container, unmount } = render(<Glyph docType={docType} size={24} />)
      const svg = container.querySelector('svg')!
      for (const node of Array.from(svg.querySelectorAll('*'))) {
        const stroke = node.getAttribute('stroke')
        if (stroke !== null) {
          expect(stroke, `${docType}: descendant <${node.tagName}> has stroke="${stroke}"`).toBe('currentColor')
        }
      }
      unmount()
    }
  })
})

describe('Glyph: accessibility branches', () => {
  it('default render carries aria-hidden="true" and neither role nor aria-label', () => {
    const { container } = render(<Glyph docType="decisions" size={24} />)
    const svg = container.querySelector('svg')!
    expect(svg.getAttribute('aria-hidden')).toBe('true')
    expect(svg.getAttribute('role')).toBeNull()
    expect(svg.getAttribute('aria-label')).toBeNull()
  })

  it('with ariaLabel="Decision" carries role="img" and aria-label and no aria-hidden', () => {
    const { container } = render(<Glyph docType="decisions" size={24} ariaLabel="Decision" />)
    const svg = container.querySelector('svg')!
    expect(svg.getAttribute('role')).toBe('img')
    expect(svg.getAttribute('aria-label')).toBe('Decision')
    expect(svg.getAttribute('aria-hidden')).toBeNull()
  })

  it('with ariaLabel="" (empty string) treats Glyph as labelled, not decorative', () => {
    const { container } = render(<Glyph docType="decisions" size={24} ariaLabel="" />)
    const svg = container.querySelector('svg')!
    expect(svg.getAttribute('role')).toBe('img')
    expect(svg.getAttribute('aria-label')).toBe('')
    expect(svg.getAttribute('aria-hidden')).toBeNull()
  })
})

describe('Glyph: runtime guard', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('renders null and warns once for an unknown docType in dev', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    // Force an unknown key past the type system to exercise the dev guard.
    const docType = 'banana' as unknown as GlyphDocTypeKey
    const { container } = render(<Glyph docType={docType} size={24} />)
    expect(container.querySelector('svg')).toBeNull()
    expect(warn).toHaveBeenCalledTimes(1)
    expect(warn.mock.calls[0][0]).toMatch(/Unknown docType: banana/)
  })
})

describe('Glyph: source-level no-state-hooks guard', () => {
  it('Glyph.tsx contains no React state, effect, or context hooks', () => {
    // AC #4's "no React render occurred" invariant. Structural enforcement —
    // a future refactor introducing a state hook must consciously update or
    // remove this guard.
    expect(GlyphSource).not.toMatch(/\buse(State|Effect|Reducer|Context|LayoutEffect)\b/)
  })
})

// AC #4 ("getComputedStyle(svg).fill resolves to a hex") is verified end-to-end
// by Playwright (tests/visual-regression/glyph-resolved-fill.spec.ts). JSDOM
// does not reliably substitute `var()` in SVG presentation attributes — see
// the Resolved Decision in meta/work/0037-glyph-component.md.

// AC line 74: explicit 12 × 3 = 36 combination matrix. Each (docType, size)
// case is named so a regression points directly at the failing combination.
const SIZES = [16, 24, 32] as const

describe.each(GLYPH_DOC_TYPE_KEYS)('Glyph: %s', (docType) => {
  describe.each(SIZES)('size %s', (size) => {
    it('renders an <svg> with correct dimensions, viewBox, and color var', () => {
      const { container } = render(<Glyph docType={docType} size={size} />)
      const svg = container.querySelector('svg') as SVGElement | null
      expect(svg).not.toBeNull()
      expect(svg!.getAttribute('width')).toBe(String(size))
      expect(svg!.getAttribute('height')).toBe(String(size))
      expect(svg!.getAttribute('viewBox')).toBeTruthy()
      expect(svg!.style.color).toBe(`var(--ac-doc-${docType})`)
    })
  })
})

// Replace attribute-literal a11y assertions with Testing Library's
// accessible-name resolution so the test reflects what assistive tech
// actually sees, not the raw attribute spelling.
describe('Glyph: accessible-name semantics', () => {
  it('default render is not exposed as an image to assistive tech', () => {
    const { queryByRole } = render(<Glyph docType="decisions" size={24} />)
    expect(queryByRole('img')).toBeNull()
  })

  it('with ariaLabel, render exposes role=img with the given name', () => {
    const { getByRole } = render(
      <Glyph docType="decisions" size={24} ariaLabel="Decision" />,
    )
    expect(getByRole('img', { name: 'Decision' })).toBeTruthy()
  })
})
