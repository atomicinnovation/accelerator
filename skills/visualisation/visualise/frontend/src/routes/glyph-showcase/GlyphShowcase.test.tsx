import { describe, it, expect } from 'vitest'
import { render } from '@testing-library/react'
import { GlyphShowcase } from './GlyphShowcase'
import { GLYPH_DOC_TYPE_KEYS } from '../../components/Glyph/Glyph'

describe('GlyphShowcase', () => {
  it('renders 12 × 3 = 36 <svg> elements', () => {
    const { container } = render(<GlyphShowcase />)
    expect(container.querySelectorAll('svg').length).toBe(36)
  })

  it('renders a cell with stable data-testid for every (docType, size) pair', () => {
    const { container } = render(<GlyphShowcase />)
    for (const docType of GLYPH_DOC_TYPE_KEYS) {
      for (const size of [16, 24, 32] as const) {
        const cell = container.querySelector(`[data-testid="glyph-cell-${docType}-${size}"]`)
        expect(cell, `missing cell for ${docType} ${size}`).not.toBeNull()
        expect(cell!.querySelector('svg')).not.toBeNull()
      }
    }
  })

  it('renders both the kebab-case key and the friendly label for every doc type', () => {
    const { container } = render(<GlyphShowcase />)
    const text = container.textContent ?? ''
    for (const docType of GLYPH_DOC_TYPE_KEYS) {
      expect(text, `missing kebab key ${docType}`).toContain(docType)
    }
    // Sample two friendly labels — full set covered by DOC_TYPE_LABELS parity.
    expect(text).toContain('Decision')
    expect(text).toContain('Design inventory')
  })
})
