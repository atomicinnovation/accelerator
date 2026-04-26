import { describe, it, expect } from 'vitest'
import { screen, within } from '@testing-library/react'
import { DndContext } from '@dnd-kit/core'
import { KanbanColumn } from './KanbanColumn'
import { makeIndexEntry } from '../../api/test-fixtures'
import { renderWithRouterAt } from '../../test/router-helpers'

function renderColumn(ui: React.ReactNode) {
  return renderWithRouterAt(<DndContext>{ui}</DndContext>)
}

describe('KanbanColumn', () => {
  it('renders the column heading and one card per entry', async () => {
    const a = makeIndexEntry({
      type: 'tickets', relPath: 'meta/tickets/0001-a.md', title: 'Alpha',
      frontmatter: { type: 'adr-creation-task', status: 'todo' },
    })
    const b = makeIndexEntry({
      type: 'tickets', relPath: 'meta/tickets/0002-b.md', title: 'Beta',
      frontmatter: { type: 'adr-creation-task', status: 'todo' },
    })
    renderColumn(<KanbanColumn columnKey="todo" label="Todo" entries={[a, b]} />)
    const region = await screen.findByRole('region', { name: /todo/i })
    expect(within(region).getByRole('heading', { name: /todo/i, level: 2 })).toBeInTheDocument()
    expect(within(region).getByText('Alpha')).toBeInTheDocument()
    expect(within(region).getByText('Beta')).toBeInTheDocument()
  })

  it('renders an empty-state message when entries is empty, marked aria-hidden', async () => {
    renderColumn(<KanbanColumn columnKey="in-progress" label="In progress" entries={[]} />)
    const region = await screen.findByRole('region', { name: /in progress/i })
    const empty = within(region).getByText(/no tickets/i)
    expect(empty).toBeInTheDocument()
    expect(empty.getAttribute('aria-hidden')).toBe('true')
  })

  it('exposes the count via aria-label without duplicating the column name', async () => {
    const a = makeIndexEntry({ type: 'tickets', relPath: 'meta/tickets/0001-a.md', frontmatter: { status: 'done' } })
    const b = makeIndexEntry({ type: 'tickets', relPath: 'meta/tickets/0002-b.md', frontmatter: { status: 'done' } })
    renderColumn(<KanbanColumn columnKey="done" label="Done" entries={[a, b]} />)
    await screen.findByRole('region', { name: /done/i })
    expect(screen.getByLabelText(/^2 tickets$/i)).toBeInTheDocument()
  })

  it('uses singular wording for one ticket and plural for zero or many', async () => {
    const a = makeIndexEntry({ type: 'tickets', relPath: 'meta/tickets/0001-a.md', frontmatter: { status: 'todo' } })
    const { unmount } = renderColumn(<KanbanColumn columnKey="todo" label="Todo" entries={[a]} />)
    await screen.findByRole('region', { name: /todo/i })
    expect(screen.getByLabelText(/^1 ticket$/i)).toBeInTheDocument()
    unmount()
    renderColumn(<KanbanColumn columnKey="todo" label="Todo" entries={[]} />)
    await screen.findByRole('region', { name: /todo/i })
    expect(screen.getByLabelText(/^0 tickets$/i)).toBeInTheDocument()
  })

  it('renders the "Other" column variant with a distinct heading and explanation', async () => {
    const x = makeIndexEntry({
      type: 'tickets', relPath: 'meta/tickets/0007-x.md', title: 'Exotic',
      frontmatter: { type: 'adr-creation-task', status: 'blocked' },
    })
    renderColumn(
      <KanbanColumn
        columnKey="other"
        label="Other"
        entries={[x]}
        description="Tickets whose status is missing or not one of: todo, in-progress, done."
      />
    )
    expect(await screen.findByRole('heading', { name: /other/i, level: 2 })).toBeInTheDocument()
    expect(screen.getByText('Exotic')).toBeInTheDocument()
    expect(screen.getByText(/missing or not one of/i)).toBeInTheDocument()
  })
})
