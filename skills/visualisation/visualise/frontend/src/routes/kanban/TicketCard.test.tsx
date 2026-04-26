import { describe, it, expect } from 'vitest'
import { screen, fireEvent } from '@testing-library/react'
import { DndContext } from '@dnd-kit/core'
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable'
import { TicketCard } from './TicketCard'
import { makeIndexEntry } from '../../api/test-fixtures'
import { renderWithRouterAt } from '../../test/router-helpers'

const FROZEN_NOW = 1_700_000_000_000

function renderCard(entry: ReturnType<typeof makeIndexEntry>, now = FROZEN_NOW) {
  return renderWithRouterAt(
    <DndContext>
      <SortableContext items={[entry.relPath]} strategy={verticalListSortingStrategy}>
        <TicketCard entry={entry} now={now} />
      </SortableContext>
    </DndContext>,
  )
}

describe('TicketCard', () => {
  it('renders the ticket number with four-digit zero-padding', async () => {
    const entry = makeIndexEntry({
      type: 'tickets',
      relPath: 'meta/tickets/0001-three-layer.md',
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

  it('renders larger ticket numbers verbatim (no truncation)', async () => {
    const entry = makeIndexEntry({
      type: 'tickets',
      relPath: 'meta/tickets/0029-template-management.md',
      title: 'Template management',
      frontmatter: { type: 'adr-creation-task', status: 'done' },
      mtimeMs: FROZEN_NOW - 90_000,
    })
    renderCard(entry)
    expect(await screen.findByText('#0029')).toBeInTheDocument()
  })

  it('links to the library detail page using the canonical typed-route form', async () => {
    const entry = makeIndexEntry({
      type: 'tickets',
      relPath: 'meta/tickets/0001-three-layer-review-system-architecture.md',
      title: 'Three-layer review system architecture',
      frontmatter: { type: 'adr-creation-task', status: 'todo' },
    })
    renderCard(entry)
    const link = await screen.findByRole('link', {
      name: /three-layer review system architecture/i,
    })
    expect(link.getAttribute('href')).toBe(
      '/library/tickets/0001-three-layer-review-system-architecture',
    )
  })

  it('renders gracefully when frontmatter.type is missing', async () => {
    const entry = makeIndexEntry({
      type: 'tickets',
      relPath: 'meta/tickets/0042-no-type.md',
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
      type: 'tickets',
      relPath: 'meta/tickets/foo-without-number.md',
      title: 'No number',
      frontmatter: { type: 'adr-creation-task', status: 'todo' },
    })
    renderCard(entry)
    expect(await screen.findByText('No number')).toBeInTheDocument()
    expect(screen.queryByText(/^#\d/)).toBeNull()
    expect(screen.getByText('foo-without-number')).toBeInTheDocument()
  })

  it('does not announce a misleading "sortable" role-description while disabled', async () => {
    const entry = makeIndexEntry({
      type: 'tickets',
      relPath: 'meta/tickets/0001-a.md',
      title: 'Some ticket',
      frontmatter: { type: 'adr-creation-task', status: 'todo' },
    })
    renderCard(entry)
    const link = await screen.findByRole('link', { name: /some ticket/i })
    expect(link.getAttribute('aria-roledescription')).toBeNull()
  })

  it('does not respond to drag interaction while disabled', async () => {
    const entry = makeIndexEntry({
      type: 'tickets',
      relPath: 'meta/tickets/0001-drag.md',
      title: 'Draggy',
      frontmatter: { type: 'adr-creation-task', status: 'todo' },
    })
    renderCard(entry)
    const link = await screen.findByRole('link', { name: /draggy/i })
    const before = link.getAttribute('style') ?? ''
    fireEvent.pointerDown(link)
    fireEvent.pointerMove(link, { clientX: 50, clientY: 50 })
    const after = link.getAttribute('style') ?? ''
    expect(after).toBe(before)
  })
})
