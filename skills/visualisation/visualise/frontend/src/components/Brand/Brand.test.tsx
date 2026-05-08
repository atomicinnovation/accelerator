import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { Brand } from './Brand'

describe('Brand', () => {
  it('renders an aria-hidden SVG with a gradient and accent-2 dot', () => {
    render(<Brand />)
    const svg = document.querySelector('svg')
    expect(svg).not.toBeNull()
    expect(svg?.getAttribute('aria-hidden')).toBe('true')

    const stops = document.querySelectorAll('stop')
    const stopColors = Array.from(stops).map(s => s.getAttribute('stop-color'))
    expect(stopColors).toContain('var(--ac-accent)')

    // Center dot uses the secondary accent (orange/coral)
    const circles = document.querySelectorAll('circle')
    const fills = Array.from(circles).map(c => c.getAttribute('fill')).filter(Boolean)
    expect(fills).toContain('var(--ac-accent-2)')
  })

  it('renders "Accelerator" text', () => {
    render(<Brand />)
    expect(screen.getByText('Accelerator')).toBeInTheDocument()
  })

  it('renders "VISUALISER" text', () => {
    render(<Brand />)
    expect(screen.getByText('VISUALISER')).toBeInTheDocument()
  })
})
