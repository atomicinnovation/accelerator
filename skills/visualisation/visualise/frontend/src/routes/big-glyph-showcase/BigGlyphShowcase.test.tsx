import { describe, it, expect } from 'vitest'
import { render } from '@testing-library/react'
import { BigGlyphShowcase } from './BigGlyphShowcase'
import { DOC_TYPE_KEYS } from '../../api/types'

describe('BigGlyphShowcase', () => {
  it('renders exactly 13 <svg> elements', () => {
    const { container } = render(<BigGlyphShowcase />)
    expect(container.querySelectorAll('svg').length).toBe(13)
  })

  it('renders a cell with stable data-testid containing an <svg> for every doc type', () => {
    const { container } = render(<BigGlyphShowcase />)
    for (const docType of DOC_TYPE_KEYS) {
      const cell = container.querySelector(
        `[data-testid="big-glyph-cell-${docType}"]`,
      )
      expect(cell, `missing cell for ${docType}`).not.toBeNull()
      expect(cell!.querySelector('svg')).not.toBeNull()
    }
  })
})
