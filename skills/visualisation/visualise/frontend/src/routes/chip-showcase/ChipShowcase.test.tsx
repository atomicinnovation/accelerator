import { describe, it, expect } from 'vitest'
import { render } from '@testing-library/react'
import { ChipShowcase } from './ChipShowcase'

const VARIANTS = ['neutral', 'indigo', 'green', 'amber', 'red', 'violet'] as const
const SIZES = ['sm', 'md'] as const

describe('ChipShowcase', () => {
  it('renders 6 × 2 = 12 chip cells', () => {
    const { container } = render(<ChipShowcase />)
    expect(container.querySelectorAll('[data-variant]').length).toBe(12)
  })

  it('renders a cell with stable data-testid for every (variant, size) pair', () => {
    const { container } = render(<ChipShowcase />)
    for (const variant of VARIANTS) {
      for (const size of SIZES) {
        const cell = container.querySelector(`[data-testid="chip-cell-${variant}-${size}"]`)
        expect(cell, `missing cell for ${variant} ${size}`).not.toBeNull()
        expect(cell!.querySelector(`[data-variant="${variant}"][data-size="${size}"]`)).not.toBeNull()
      }
    }
  })
})
