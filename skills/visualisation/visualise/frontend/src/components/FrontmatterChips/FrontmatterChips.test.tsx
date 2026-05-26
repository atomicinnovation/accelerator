import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { FrontmatterChips } from './FrontmatterChips'
import css from './FrontmatterChips.module.css?raw'

const CHIP_SELECTOR = '[data-testid="status-badge"],[data-testid="frontmatter-chip"]'

describe('FrontmatterChips', () => {
  describe('parsed state', () => {
    it('renders a chip for each non-null frontmatter value', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ status: 'accepted', date: '2026-04-05', author: 'Toby Clemson' }}
        />,
      )
      const chips = container.querySelectorAll(CHIP_SELECTOR)
      expect(chips.length).toBe(3)
    })

    it('skips null and undefined values', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ status: 'draft', author: null, date: undefined } as Record<string, unknown>}
        />,
      )
      const chips = container.querySelectorAll(CHIP_SELECTOR)
      expect(chips.length).toBe(1)
    })

    it('skips empty-string values', () => {
      const { container } = render(
        <FrontmatterChips state="parsed" frontmatter={{ status: 'draft', author: '' }} />,
      )
      const chips = container.querySelectorAll(CHIP_SELECTOR)
      expect(chips.length).toBe(1)
    })

    it('skips whitespace-only string values (treated as missing)', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ status: '   ', date: '\t\n', author: 'Toby Clemson' }}
        />,
      )
      const labels = Array.from(container.querySelectorAll('[aria-label]'))
        .map((el) => el.getAttribute('aria-label'))
      expect(labels).toEqual(['author: Toby Clemson'])
    })

    it('renders a chip for a non-string canonical value (smoke test)', () => {
      // YAML parsers may emit Date objects for ISO dates; co-authored
      // documents may use array `author`. These are realistic shapes for
      // `Record<string, unknown>` frontmatter — pin the current
      // `FrontmatterChip.formatChipValue` rules (JSON.stringify for
      // non-array objects including Date, `, ` join for arrays) so
      // future changes there don't regress chip rendering silently.
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{
            date: new Date('2026-04-05T00:00:00Z'),
            author: ['Alice', 'Bob'],
          } as Record<string, unknown>}
        />,
      )
      const labels = Array.from(container.querySelectorAll('[aria-label]'))
        .map((el) => el.getAttribute('aria-label'))
      expect(labels).toEqual([
        // JSON.stringify(Date) yields a quoted ISO string — the literal
        // quotes are part of the rendered chip text. This is surprising
        // but accurate; if FrontmatterChip ever special-cases Date, this
        // assertion catches the change.
        'date: "2026-04-05T00:00:00.000Z"',
        'author: Alice, Bob',
      ])
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

    it('dispatches non-tone keys to FrontmatterChip', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ date: '2026-05-22', author: 'X' }}
        />,
      )
      expect(container.querySelectorAll('[data-testid="frontmatter-chip"]').length).toBe(2)
      expect(container.querySelector('[data-testid="status-badge"]')).toBeNull()
    })

    it.each([
      ['Status', 'status-badge'], ['STATUS', 'status-badge'],
    ])('dispatches case-folded key "%s" to %s', (key, expectedComponent) => {
      const { container } = render(
        <FrontmatterChips state="parsed" frontmatter={{ [key]: 'pass' }} />,
      )
      expect(container.querySelector(`[data-testid="${expectedComponent}"]`)).not.toBeNull()
    })
  })

  describe('whitelist + canonical order', () => {
    // Note: tests select chips by aria-label (preferred) or by the
    // specific badge/chip testids (`status-badge`, `frontmatter-chip`),
    // not by a generic `[data-testid]` query — the wrapper carries
    // `data-testid="frontmatter-chips"` and would otherwise inflate the
    // count.
    it('renders at most three chips from {status, date, author}', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{
            status: 'accepted', date: '2026-04-05', author: 'Toby Clemson',
            verdict: 'APPROVE', result: 'pass', priority: 'medium',
            tags: ['x'], id: '0084', kind: 'story',
          }}
        />,
      )
      const chipTestids = ['status-badge', 'frontmatter-chip'] as const
      const selector = chipTestids.map((t) => `[data-testid="${t}"]`).join(',')
      const chips = container.querySelectorAll(selector)
      expect(chips.length).toBe(3)
      expect(Array.from(chips).map((el) => el.getAttribute('data-testid'))).toEqual([
        'status-badge', 'frontmatter-chip', 'frontmatter-chip',
      ])
    })

    it('renders chips in canonical order regardless of frontmatter source order', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ author: 'Toby Clemson', status: 'draft', date: '2026-04-05' }}
        />,
      )
      const labels = Array.from(container.querySelectorAll('[aria-label]'))
        .map((el) => el.getAttribute('aria-label'))
      expect(labels).toEqual([
        'status: draft', 'date: 2026-04-05', 'author: Toby Clemson',
      ])
    })

    it('renders only canonical keys when all three are present', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ status: 'draft', date: '2026-04-05', author: 'Toby Clemson' }}
        />,
      )
      const labels = Array.from(container.querySelectorAll('[aria-label]'))
        .map((el) => el.getAttribute('aria-label'))
      expect(labels).toEqual([
        'status: draft', 'date: 2026-04-05', 'author: Toby Clemson',
      ])
    })
  })

  describe('whitelist exclusion', () => {
    it.each([
      ['verdict', 'APPROVE'],
      ['result', 'pass'],
      ['priority', 'medium'],
      ['tags', ['design', 'frontend']],
      ['id', '0084'],
      ['kind', 'story'],
      ['type', 'work-item'],
      ['slug', 'detail-page'],
      ['title', 'A title'],
      ['last_updated', '2026-05-26'],
      ['last_updated_by', 'Someone'],
    ])('does not render a chip for non-canonical key "%s"', (key, value) => {
      const { container } = render(
        <FrontmatterChips state="parsed" frontmatter={{ [key]: value } as Record<string, unknown>} />,
      )
      // Wrapper carries data-testid="frontmatter-chips" — assert no
      // child chip testids, not zero testids overall.
      expect(container.querySelector('[data-testid="status-badge"]')).toBeNull()
      expect(container.querySelector('[data-testid="frontmatter-chip"]')).toBeNull()
    })
  })

  describe('subset ordering', () => {
    it.each([
      [{ status: 'draft', date: '2026-04-05' }, ['status: draft', 'date: 2026-04-05']],
      [{ status: 'draft', author: 'Toby Clemson' }, ['status: draft', 'author: Toby Clemson']],
      [{ date: '2026-04-05', author: 'Toby Clemson' }, ['date: 2026-04-05', 'author: Toby Clemson']],
      [{ author: 'Toby Clemson', date: '2026-04-05' }, ['date: 2026-04-05', 'author: Toby Clemson']],
      [{ status: 'draft' }, ['status: draft']],
    ])('renders subset %# in canonical order', (frontmatter, expectedLabels) => {
      const { container } = render(
        <FrontmatterChips state="parsed" frontmatter={frontmatter} />,
      )
      const labels = Array.from(container.querySelectorAll('[aria-label]'))
        .map((el) => el.getAttribute('aria-label'))
      expect(labels).toEqual(expectedLabels)
    })
  })

  describe('case-fold dedup precedence', () => {
    it('skipped (null) case-variant does not block a later valid canonical key', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ Status: null, status: 'draft' } as Record<string, unknown>}
        />,
      )
      const labels = Array.from(container.querySelectorAll('[aria-label]'))
        .map((el) => el.getAttribute('aria-label'))
      expect(labels).toEqual(['status: draft'])
    })

    it('first non-skipped value wins among case-variant duplicates', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ Status: 'first', status: 'second' } as Record<string, unknown>}
        />,
      )
      const labels = Array.from(container.querySelectorAll('[aria-label]'))
        .map((el) => el.getAttribute('aria-label'))
      expect(labels).toEqual(['status: first'])
    })
  })

  describe('date / author precedence over last_updated mirrors', () => {
    it('uses date (creation-anchored), ignoring last_updated', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ date: '2026-04-05', last_updated: '2026-05-26' }}
        />,
      )
      const labels = Array.from(container.querySelectorAll('[aria-label]'))
        .map((el) => el.getAttribute('aria-label'))
      expect(labels).toEqual(['date: 2026-04-05'])
    })

    it('uses author, ignoring last_updated_by', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ author: 'Toby Clemson', last_updated_by: 'Someone Else' }}
        />,
      )
      const labels = Array.from(container.querySelectorAll('[aria-label]'))
        .map((el) => el.getAttribute('aria-label'))
      expect(labels).toEqual(['author: Toby Clemson'])
    })
  })
})
