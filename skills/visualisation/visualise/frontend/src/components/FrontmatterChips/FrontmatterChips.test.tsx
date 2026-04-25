import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { FrontmatterChips } from './FrontmatterChips'

describe('FrontmatterChips', () => {
  it('renders key-value pairs from frontmatter', () => {
    render(
      <FrontmatterChips
        frontmatter={{ status: 'draft', date: '2026-01-01', author: 'Toby' }}
        state="parsed"
      />
    )
    expect(screen.getByText(/status/i)).toBeInTheDocument()
    expect(screen.getByText('draft')).toBeInTheDocument()
  })

  it('renders a warning banner for malformed frontmatter', () => {
    render(<FrontmatterChips frontmatter={{}} state="malformed" />)
    expect(screen.getByRole('alert')).toBeInTheDocument()
  })

  it('renders nothing for absent frontmatter (no error, no chips)', () => {
    const { container } = render(<FrontmatterChips frontmatter={{}} state="absent" />)
    expect(container.firstChild).toBeNull()
  })
})
