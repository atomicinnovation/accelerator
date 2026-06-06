import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { WorkItemCardPresentation } from './WorkItemCardPresentation'
import { makeIndexEntry } from '../../api/test-fixtures'

const entry = makeIndexEntry({
  type: 'work-items',
  relPath: 'meta/work/0086-kanban.md',
  workItemId: '0086',
  title: 'Kanban DnD',
  frontmatter: { kind: 'feature' },
})

function cardOf(container: HTMLElement): HTMLElement {
  return container.querySelector('.ac-kcard') as HTMLElement
}

describe('WorkItemCardPresentation', () => {
  it('renders the card visuals from the entry with no sortable/navigation wiring', () => {
    const { container } = render(<WorkItemCardPresentation entry={entry} />)
    expect(screen.getByText('Kanban DnD')).toBeInTheDocument()
    expect(container.querySelector('.ac-kcard__id')!.textContent).toBe('0086')
    // No navigation link, no sortable role description in the bare presentation.
    expect(container.querySelector('a')).toBeNull()
    expect(container.querySelector('[aria-roledescription="sortable"]')).toBeNull()
  })

  it('applies the dragging state when the dragging prop is set', () => {
    const { container } = render(<WorkItemCardPresentation entry={entry} dragging />)
    const card = cardOf(container)
    expect(card.hasAttribute('data-dragging')).toBe(true)
    expect(card.hasAttribute('data-overlay')).toBe(false)
  })

  it('applies the overlay (clone) state when the overlay prop is set', () => {
    const { container } = render(<WorkItemCardPresentation entry={entry} overlay />)
    const card = cardOf(container)
    expect(card.hasAttribute('data-overlay')).toBe(true)
    expect(card.hasAttribute('data-dragging')).toBe(false)
    // Still no navigation in the overlay clone.
    expect(container.querySelector('a')).toBeNull()
  })

  it('is in neither state by default (resting card)', () => {
    const { container } = render(<WorkItemCardPresentation entry={entry} />)
    const card = cardOf(container)
    expect(card.hasAttribute('data-dragging')).toBe(false)
    expect(card.hasAttribute('data-overlay')).toBe(false)
  })
})
