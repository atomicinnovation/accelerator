import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { EmptyState } from './EmptyState'
import { EMPTY_DESCRIPTIONS, EMPTY_TYPE_PLURALS } from './empty-descriptions'
import { DOC_TYPE_KEYS } from '../../api/types'
import { DOC_TYPE_HUE } from '../../styles/tokens'

// Does any descendant fill/stroke carry an `hsl(<hue> …)` tone for this hue?
function heroHasHue(svg: SVGElement, hue: number): boolean {
  const want = `hsl(${hue} `
  for (const node of Array.from(svg.querySelectorAll('*'))) {
    for (const attr of ['fill', 'stroke'] as const) {
      const v = node.getAttribute(attr)
      if (v && v.toLowerCase().startsWith(want)) return true
    }
  }
  return false
}

describe('EmptyState', () => {
  it('renders the path heading from dirPath', () => {
    render(<EmptyState docType="pr-descriptions" dirPath="meta/prs/" />)
    // dirPath appears twice (standalone path + inline inside footer); just
    // assert at least one occurrence exists.
    expect(screen.getAllByText('meta/prs/').length).toBeGreaterThan(0)
  })

  it('renders the no-{plural}-yet headline', () => {
    render(<EmptyState docType="decisions" dirPath="meta/decisions" />)
    expect(
      screen.getByRole('heading', { name: /no decisions yet/i }),
    ).toBeInTheDocument()
  })

  it('renders the per-doc-type description', () => {
    render(<EmptyState docType="plans" dirPath="meta/plans" />)
    expect(screen.getByText(EMPTY_DESCRIPTIONS['plans'])).toBeInTheDocument()
  })

  it('renders the indexer-aware footer with the dirPath inline', () => {
    const { container } = render(<EmptyState docType="plans" dirPath="meta/plans" />)
    // The dirPath is wrapped in a span inside the footer, so we can't match
    // the full text via a single literal. Look up the footer paragraph and
    // assert its concatenated text content contains the expected phrase.
    const footer = container.querySelector('p:last-of-type')
    expect(footer?.textContent ?? '').toMatch(
      /new files added to meta\/plans are picked up live/i,
    )
  })
})

describe('EmptyState: BigGlyph hero wiring', () => {
  // Two doc types with deliberately distinct hues, so "the tones differ between
  // the two" is genuinely discriminating. Asserted up front rather than assumed.
  const A = 'work-items'
  const B = 'decisions'

  it('the two probe doc types have distinct hues (guards the wiring assertions)', () => {
    expect(DOC_TYPE_HUE[A]).not.toBe(DOC_TYPE_HUE[B])
  })

  it('renders each type-specific hero coloured from its OWN doc-type hue', () => {
    const a = render(<EmptyState docType={A} />)
    const aSvg = a.container.querySelector('svg')!
    expect(heroHasHue(aSvg, DOC_TYPE_HUE[A]), `${A} hero hue`).toBe(true)
    // Cross-check: the work-items hero must NOT be coloured with decisions' hue.
    expect(heroHasHue(aSvg, DOC_TYPE_HUE[B])).toBe(false)
    a.unmount()

    const b = render(<EmptyState docType={B} />)
    const bSvg = b.container.querySelector('svg')!
    expect(heroHasHue(bSvg, DOC_TYPE_HUE[B]), `${B} hero hue`).toBe(true)
    expect(heroHasHue(bSvg, DOC_TYPE_HUE[A])).toBe(false)
  })

  it('renders the hero as a decorative 96px svg (aria-hidden, no role)', () => {
    const { container } = render(<EmptyState docType="plans" />)
    const svg = container.querySelector('svg')!
    expect(svg.getAttribute('aria-hidden')).toBe('true')
    expect(svg.getAttribute('role')).toBeNull()
    expect(svg.getAttribute('width')).toBe('96')
    expect(svg.getAttribute('height')).toBe('96')
    expect(svg.getAttribute('viewBox')).toBe('0 0 80 80')
  })
})

describe('empty-descriptions table completeness', () => {
  it('declares a non-empty description and plural for every DocTypeKey', () => {
    for (const key of DOC_TYPE_KEYS) {
      expect(EMPTY_DESCRIPTIONS[key], `${key} description`).toBeTruthy()
      expect(EMPTY_TYPE_PLURALS[key], `${key} plural`).toBeTruthy()
    }
  })
})
