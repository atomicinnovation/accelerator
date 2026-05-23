import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { FrontmatterChips } from './FrontmatterChips'
import css from './FrontmatterChips.module.css?raw'

describe('FrontmatterChips', () => {
  describe('parsed state', () => {
    it('renders a chip for each non-null frontmatter value', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ status: 'accepted', date: '2026-04-05', author: 'Toby Clemson' }}
        />,
      )
      const chips = container.querySelectorAll('[data-testid]')
      expect(chips.length).toBe(3)
    })

    it('skips null and undefined values', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ status: 'draft', author: null, date: undefined } as Record<string, unknown>}
        />,
      )
      const chips = container.querySelectorAll('[data-testid]')
      expect(chips.length).toBe(1)
    })

    it('skips empty-string values', () => {
      const { container } = render(
        <FrontmatterChips state="parsed" frontmatter={{ status: 'draft', author: '' }} />,
      )
      const chips = container.querySelectorAll('[data-testid]')
      expect(chips.length).toBe(1)
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

  describe('dispatch', () => {
    it('dispatches the status key to StatusBadge', () => {
      const { container } = render(
        <FrontmatterChips state="parsed" frontmatter={{ status: 'Accepted' }} />,
      )
      expect(container.querySelector('[data-testid="status-badge"]')).not.toBeNull()
    })

    it('dispatches the verdict key to VerdictBadge', () => {
      const { container } = render(
        <FrontmatterChips state="parsed" frontmatter={{ verdict: 'APPROVE' }} />,
      )
      expect(container.querySelector('[data-testid="verdict-badge"]')).not.toBeNull()
    })

    it('dispatches the result key to ResultBadge', () => {
      const { container } = render(
        <FrontmatterChips state="parsed" frontmatter={{ result: 'pass' }} />,
      )
      expect(container.querySelector('[data-testid="result-badge"]')).not.toBeNull()
    })

    it('dispatches non-tone keys to FrontmatterChip', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ priority: 'medium', date: '2026-05-22' }}
        />,
      )
      expect(container.querySelectorAll('[data-testid="frontmatter-chip"]').length).toBe(2)
      expect(container.querySelector('[data-testid="status-badge"]')).toBeNull()
      expect(container.querySelector('[data-testid="verdict-badge"]')).toBeNull()
      expect(container.querySelector('[data-testid="result-badge"]')).toBeNull()
    })

    it.each([
      ['Status', 'status-badge'], ['STATUS', 'status-badge'],
      ['Verdict', 'verdict-badge'], ['VERDICT', 'verdict-badge'],
      ['Result', 'result-badge'], ['RESULT', 'result-badge'],
    ])('dispatches case-folded key "%s" to %s', (key, expectedComponent) => {
      const { container } = render(
        <FrontmatterChips state="parsed" frontmatter={{ [key]: 'pass' }} />,
      )
      expect(container.querySelector(`[data-testid="${expectedComponent}"]`)).not.toBeNull()
    })
  })

  describe('source order', () => {
    it('preserves frontmatter source order in rendered output', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ verdict: 'APPROVE', status: 'Accepted', priority: 'medium' }}
        />,
      )
      const labels = Array.from(container.querySelectorAll('[aria-label]'))
        .map((el) => el.getAttribute('aria-label'))
      expect(labels).toEqual([
        'verdict: APPROVE', 'status: Accepted', 'priority: medium',
      ])
    })
  })

  describe('AC integration fixtures', () => {
    it('plan-review-shaped document: source order, components, variants', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{
            status: 'Accepted',
            verdict: 'APPROVE',
            priority: 'medium',
            tags: ['design', 'frontend'],
          }}
        />,
      )
      const chips = Array.from(container.querySelectorAll('[data-testid]'))
      expect(chips).toHaveLength(4)
      expect(chips.map((el) => el.getAttribute('data-testid'))).toEqual([
        'status-badge', 'verdict-badge', 'frontmatter-chip', 'frontmatter-chip',
      ])
      expect(chips.map((el) => el.getAttribute('data-variant'))).toEqual([
        'green', 'green', 'neutral', 'neutral',
      ])
    })

    it('validation-shaped document: source order, components, variants', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{
            status: 'complete',
            result: 'pass',
            priority: 'medium',
            tags: ['validation'],
          }}
        />,
      )
      const chips = Array.from(container.querySelectorAll('[data-testid]'))
      expect(chips).toHaveLength(4)
      expect(chips.map((el) => el.getAttribute('data-testid'))).toEqual([
        'status-badge', 'result-badge', 'frontmatter-chip', 'frontmatter-chip',
      ])
      expect(chips.map((el) => el.getAttribute('data-variant'))).toEqual([
        'green', 'green', 'neutral', 'neutral',
      ])
    })
  })
})
