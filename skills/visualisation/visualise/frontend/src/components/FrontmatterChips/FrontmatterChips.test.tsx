import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { FrontmatterChips } from './FrontmatterChips'
import css from './FrontmatterChips.module.css?raw'
import styles from './FrontmatterChips.module.css'
import { DOC_TYPE_KEYS, type DocTypeKey } from '../../api/types'

const CHIP_SELECTOR = '[data-testid="status-badge"],[data-testid="frontmatter-chip"]'

// Fixed clock so the `date` chip's relative-time output is deterministic.
// Fixtures use `date: '2026-04-05'`, three days before this instant, which
// `formatChipDate` renders as `3d ago`.
const NOW = new Date('2026-04-08T00:00:00Z')

beforeEach(() => {
  vi.useFakeTimers()
  vi.setSystemTime(NOW)
})

afterEach(() => {
  vi.useRealTimers()
})

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
      // `Record<string, unknown>` frontmatter. The `date` chip routes
      // through `formatChipDate` (relative time); `author` routes through
      // `FrontmatterChip.formatChipValue`'s `, ` join for arrays — pin
      // both so future changes don't regress chip rendering silently.
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
        'date: 3d ago',
        'author: Alice, Bob',
      ])
    })
  })

  describe('absent state', () => {
    it('renders the empty chip-strip container so the subtitle slot keeps its height', () => {
      const { container } = render(<FrontmatterChips state="absent" />)
      const strip = container.querySelector('[data-testid="frontmatter-chips"]')
      expect(strip).not.toBeNull()
      expect(strip).toBeInstanceOf(HTMLElement)
      expect((strip as HTMLElement).children.length).toBe(0)
      expect(strip).toHaveClass(styles.chips)
      expect(strip).toHaveAttribute('aria-hidden', 'true')
    })
  })

  describe('zero qualifying keys (parsed state)', () => {
    it('renders the empty chip-strip container when no canonical keys qualify', () => {
      const { container } = render(
        <FrontmatterChips state="parsed" frontmatter={{ priority: 'medium', tags: ['x'] }} />,
      )
      expect(container.querySelectorAll('[data-testid]').length).toBe(1)
      const strip = container.querySelector('[data-testid="frontmatter-chips"]')
      expect(strip).not.toBeNull()
      expect((strip as HTMLElement).children.length).toBe(0)
      expect(strip).toHaveAttribute('aria-hidden', 'true')
    })

    it('renders the empty chip-strip container when frontmatter is entirely empty', () => {
      const { container } = render(
        <FrontmatterChips state="parsed" frontmatter={{}} />,
      )
      const strip = container.querySelector('[data-testid="frontmatter-chips"]')
      expect(strip).not.toBeNull()
      expect((strip as HTMLElement).children.length).toBe(0)
      expect(strip).toHaveAttribute('aria-hidden', 'true')
    })

    it('omits aria-hidden when at least one chip qualifies', () => {
      const { container } = render(
        <FrontmatterChips state="parsed" frontmatter={{ status: 'draft' }} />,
      )
      const strip = container.querySelector('[data-testid="frontmatter-chips"]')
      expect(strip).not.toBeNull()
      expect(strip).not.toHaveAttribute('aria-hidden')
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
    it('declares a min-height on .chips so the empty container preserves one-chip height', () => {
      expect(css).toMatch(/\.chips\s*\{[^}]*min-height:\s*1lh/)
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
        'status: Draft', 'date: 3d ago', 'author: Toby Clemson',
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
        'status: Draft', 'date: 3d ago', 'author: Toby Clemson',
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
      [{ status: 'draft', date: '2026-04-05' }, ['status: Draft', 'date: 3d ago']],
      [{ status: 'draft', author: 'Toby Clemson' }, ['status: Draft', 'author: Toby Clemson']],
      [{ date: '2026-04-05', author: 'Toby Clemson' }, ['date: 3d ago', 'author: Toby Clemson']],
      [{ author: 'Toby Clemson', date: '2026-04-05' }, ['date: 3d ago', 'author: Toby Clemson']],
      [{ status: 'draft' }, ['status: Draft']],
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
      expect(labels).toEqual(['status: Draft'])
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
      expect(labels).toEqual(['status: First'])
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
      expect(labels).toEqual(['date: 3d ago'])
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

describe('12-doc-kind corpus verification', () => {
  // One extra non-canonical key per kind, plus the three canonical
  // keys, ensures the whitelist excludes anything outside the set
  // regardless of which kind's fixture shape is used. Per-kind
  // extras are deliberately illustrative — they need not match
  // ADR-0033's per-artifact-type extras list exactly; the contract
  // being verified is "any non-canonical key is excluded", not "this
  // specific key shape per kind". Typed as Record<DocTypeKey, ...>
  // so a future doc-kind addition fails typecheck rather than
  // runtime-destructuring undefined.
  const NON_CANONICAL_PER_KIND: Record<DocTypeKey, [string, unknown]> = {
    'decisions':           ['adr', '0033'],
    'work-items':          ['priority', 'medium'],
    'plans':               ['type', 'plan'],
    'research':            ['git_commit', 'abc123def456'],
    'plan-reviews':        ['verdict', 'APPROVE'],
    'pr-reviews':          ['verdict', 'APPROVE'],
    'work-item-reviews':   ['verdict', 'APPROVE'],
    'validations':         ['result', 'pass'],
    'notes':               ['tags', ['note']],
    'pr-descriptions':     ['pr_number', 123],
    'design-gaps':         ['file_path', 'some/path.md'],
    'design-inventories':  ['last_updated_by', 'Someone'],
    'templates':           ['template_for', 'work-items'],
  }

  it.each(DOC_TYPE_KEYS)('kind "%s" never renders the extra non-canonical key as a chip', (kind) => {
    const [extraKey, extraValue] = NON_CANONICAL_PER_KIND[kind]
    const { container } = render(
      <FrontmatterChips
        state="parsed"
        frontmatter={{
          status: 'draft',
          date: '2026-04-05',
          author: 'Toby Clemson',
          [extraKey]: extraValue,
        } as Record<string, unknown>}
      />,
    )
    const labels = Array.from(container.querySelectorAll('[aria-label]'))
      .map((el) => el.getAttribute('aria-label'))
    // Exactly three canonical chips, no chip carrying the extra key.
    expect(labels).toEqual([
      'status: Draft', 'date: 3d ago', 'author: Toby Clemson',
    ])
    expect(labels.some((l) => l?.startsWith(`${extraKey}:`))).toBe(false)
  })
})
