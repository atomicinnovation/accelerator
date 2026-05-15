import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { FrontmatterChips } from './FrontmatterChips'
import css from './FrontmatterChips.module.css?raw'

describe('FrontmatterChips', () => {
  describe('parsed state', () => {
    it('renders a Chip for each non-null frontmatter value', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ status: 'accepted', date: '2026-04-05', author: 'Toby Clemson' }}
        />,
      )
      const chips = container.querySelectorAll('[data-variant]')
      expect(chips.length).toBe(3)
    })

    it('renders the status field with the colour-coded variant', () => {
      const { container } = render(
        <FrontmatterChips state="parsed" frontmatter={{ status: 'accepted' }} />,
      )
      expect(container.querySelector('[data-variant="green"]')).not.toBeNull()
    })

    it('colour-codes status case-insensitively (Status, STATUS)', () => {
      const { container } = render(
        <FrontmatterChips state="parsed" frontmatter={{ Status: 'accepted' }} />,
      )
      expect(container.querySelector('[data-variant="green"]')).not.toBeNull()
    })

    it('renders non-status fields with variant="neutral"', () => {
      const { container } = render(
        <FrontmatterChips state="parsed" frontmatter={{ date: '2026-04-05' }} />,
      )
      expect(container.querySelector('[data-variant="neutral"]')).not.toBeNull()
    })

    it('does not render keys (e.g. the literal text "status:") in visible content', () => {
      render(
        <FrontmatterChips state="parsed" frontmatter={{ status: 'accepted' }} />,
      )
      expect(screen.getByText('accepted')).toBeInTheDocument()
      expect(screen.queryByText(/^status:/i)).toBeNull()
    })

    it('attaches an aria-label of "${key}: ${value}" to each chip', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ status: 'accepted', date: '2026-04-05' }}
        />,
      )
      expect(container.querySelector('[aria-label="status: accepted"]')).not.toBeNull()
      expect(container.querySelector('[aria-label="date: 2026-04-05"]')).not.toBeNull()
    })

    it('renders the value text', () => {
      render(<FrontmatterChips state="parsed" frontmatter={{ status: 'draft' }} />)
      expect(screen.getByText('draft')).toBeInTheDocument()
    })

    it('skips null and undefined values', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ status: 'draft', author: null, date: undefined } as Record<string, unknown>}
        />,
      )
      const chips = container.querySelectorAll('[data-variant]')
      expect(chips.length).toBe(1)
    })

    it('skips empty-string values', () => {
      const { container } = render(
        <FrontmatterChips state="parsed" frontmatter={{ status: 'draft', author: '' }} />,
      )
      const chips = container.querySelectorAll('[data-variant]')
      expect(chips.length).toBe(1)
    })

    it('renders boolean and numeric values as strings', () => {
      render(
        <FrontmatterChips state="parsed" frontmatter={{ archived: false, version: 0 }} />,
      )
      expect(screen.getByText('false')).toBeInTheDocument()
      expect(screen.getByText('0')).toBeInTheDocument()
    })

    it('joins array values with ", " and reflects the joined text in the aria-label', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ tags: ['design', 'frontend'] }}
        />,
      )
      expect(screen.getByText('design, frontend')).toBeInTheDocument()
      expect(container.querySelector('[aria-label="tags: design, frontend"]')).not.toBeNull()
    })
  })

  describe('absent state', () => {
    it('renders nothing', () => {
      const { container } = render(<FrontmatterChips state="absent" />)
      expect(container.firstChild).toBeNull()
    })
  })

  describe('malformed state', () => {
    it('renders the warning banner role="alert"', () => {
      render(<FrontmatterChips state="malformed" />)
      expect(screen.getByRole('alert')).toBeInTheDocument()
    })

    it('preserves the existing banner text verbatim', () => {
      render(<FrontmatterChips state="malformed" />)
      expect(screen.getByText(/Frontmatter unparseable/i)).toBeInTheDocument()
    })
  })

  describe('CSS source assertions', () => {
    it('no longer defines a .chip class (replaced by <Chip>)', () => {
      expect(css).not.toMatch(/\.chip\s*\{/)
    })
    it('still defines the .banner class for the malformed state', () => {
      expect(css).toMatch(/\.banner\s*\{/)
    })
  })
})
