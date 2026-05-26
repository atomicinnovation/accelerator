import { describe, it, expect } from 'vitest'
import { render } from '@testing-library/react'
import { EyebrowLabel } from './EyebrowLabel'
import { DOC_TYPE_LABELS } from '../../api/types'

describe('EyebrowLabel', () => {
  it('renders a tinted Glyph and uppercase label for a non-virtual key', () => {
    const { container } = render(<EyebrowLabel type="decisions" />)
    const wrapper = container.querySelector('[data-testid="eyebrow-label"]')!
    expect(wrapper).not.toBeNull()
    const svg = wrapper.querySelector('svg')!
    expect(svg.getAttribute('data-doc-type')).toBe('decisions')
    expect(svg.style.color).toBe('var(--ac-doc-decisions)')
    expect(wrapper.textContent).toContain(DOC_TYPE_LABELS['decisions'].toUpperCase())
  })

  it('renders a Glyph for the virtual templates key resolving to --ac-fg-muted', () => {
    const { container } = render(<EyebrowLabel type="templates" />)
    const wrapper = container.querySelector('[data-testid="eyebrow-label"]')!
    const svg = wrapper.querySelector('svg')!
    expect(svg.getAttribute('data-doc-type')).toBe('templates')
    expect(svg.style.color).toBe('var(--ac-fg-muted)')
    expect(wrapper.textContent).toContain('TEMPLATES')
  })
})
