import { describe, it, expect } from 'vitest'
import { screen } from '@testing-library/react'
import { DndContext } from '@dnd-kit/core'
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable'
import { WorkItemCard } from './WorkItemCard'
import { makeIndexEntry } from '../../api/test-fixtures'
import { renderWithRouterAt } from '../../test/router-helpers'

const FROZEN_NOW = 1_700_000_000_000

function renderCard(entry: ReturnType<typeof makeIndexEntry>, now = FROZEN_NOW) {
  return renderWithRouterAt(
    <DndContext>
      <SortableContext items={[entry.relPath]} strategy={verticalListSortingStrategy}>
        <WorkItemCard entry={entry} now={now} />
      </SortableContext>
    </DndContext>,
  )
}

describe('WorkItemCard', () => {
  it('renders the work item number with four-digit zero-padding', async () => {
    const entry = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/0001-three-layer.md',
      title: 'Three-layer review system architecture',
      frontmatter: { type: 'adr-creation-task', status: 'done' },
      mtimeMs: FROZEN_NOW - 90_000,
    })
    renderCard(entry)
    expect(await screen.findByText('#0001')).toBeInTheDocument()
    expect(screen.getByText('Three-layer review system architecture')).toBeInTheDocument()
    expect(screen.getByText('adr-creation-task')).toBeInTheDocument()
    expect(screen.getByText('1m ago')).toBeInTheDocument()
  })

  it('renders larger work item numbers verbatim (no truncation)', async () => {
    const entry = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/0029-template-management.md',
      title: 'Template management',
      frontmatter: { type: 'adr-creation-task', status: 'done' },
      mtimeMs: FROZEN_NOW - 90_000,
    })
    renderCard(entry)
    expect(await screen.findByText('#0029')).toBeInTheDocument()
  })

  it('links to the library detail page using the canonical typed-route form', async () => {
    const entry = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/0001-three-layer-review-system-architecture.md',
      title: 'Three-layer review system architecture',
      frontmatter: { type: 'adr-creation-task', status: 'todo' },
    })
    renderCard(entry)
    const link = await screen.findByRole('link', {
      name: /three-layer review system architecture/i,
    })
    expect(link.getAttribute('href')).toBe(
      '/library/work-items/0001-three-layer-review-system-architecture',
    )
  })

  it('renders gracefully when frontmatter.type is missing', async () => {
    const entry = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/0042-no-type.md',
      title: 'No type',
      frontmatter: { status: 'todo' },
    })
    renderCard(entry)
    expect(await screen.findByText('#0042')).toBeInTheDocument()
    expect(screen.getByText('No type')).toBeInTheDocument()
    expect(screen.queryByText(/undefined/)).toBeNull()
  })

  it('falls back to the file slug when the relPath has no numeric prefix', async () => {
    const entry = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/foo-without-number.md',
      title: 'No number',
      frontmatter: { type: 'adr-creation-task', status: 'todo' },
    })
    renderCard(entry)
    expect(await screen.findByText('No number')).toBeInTheDocument()
    expect(screen.queryByText(/^#\d/)).toBeNull()
    expect(screen.getByText('foo-without-number')).toBeInTheDocument()
  })

  it('announces_sortable_role_description_when_enabled', async () => {
    const entry = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/0001-a.md',
      title: 'Some work item',
      frontmatter: { type: 'adr-creation-task', status: 'todo' },
    })
    renderCard(entry)
    const link = await screen.findByRole('link', { name: /some work item/i })
    expect(link.getAttribute('aria-roledescription')).toBe('sortable')
  })

  it('card_carries_data_relpath_for_focus_restore', async () => {
    const entry = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/0001-a.md',
      title: 'Some work item',
      frontmatter: { type: 'adr-creation-task', status: 'todo' },
    })
    renderCard(entry)
    await screen.findByRole('link', { name: /some work item/i })
    const li = document.querySelector('[data-relpath="meta/work/0001-a.md"]')
    expect(li).not.toBeNull()
  })
})
