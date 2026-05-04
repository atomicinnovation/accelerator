import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { act, fireEvent, render, screen, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import {
  createRouter, createRootRoute, createRoute,
  createMemoryHistory, RouterProvider, Outlet,
} from '@tanstack/react-router'
import { KanbanBoard } from './KanbanBoard'
import * as fetchModule from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import { makeIndexEntry } from '../../api/test-fixtures'

function renderKanbanAt(qc: QueryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })) {
  // Render KanbanBoard directly in a minimal router (the library doc route is
  // registered so <Link to="/library/$type/$fileSlug"> resolves in tests).
  // This avoids a dependency on Step 11's router wiring.
  const root = createRootRoute({ component: () => <Outlet /> })
  const kanbanRoute = createRoute({
    getParentRoute: () => root,
    path: '/',
    component: KanbanBoard,
  })
  const libraryDocRoute = createRoute({
    getParentRoute: () => root,
    path: '/library/$type/$fileSlug',
    component: () => null,
  })
  const router = createRouter({
    routeTree: root.addChildren([kanbanRoute, libraryDocRoute]),
    history: createMemoryHistory({ initialEntries: ['/'] }),
  })
  return {
    ...render(
      <QueryClientProvider client={qc}>
        <RouterProvider router={router} />
      </QueryClientProvider>,
    ),
    queryClient: qc,
  }
}

describe('KanbanBoard', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('renders the page-level heading at the top of the board', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([])
    renderKanbanAt()
    expect(await screen.findByRole('heading', { level: 1, name: /^kanban$/i })).toBeInTheDocument()
  })

  it('shows a loading state while the work items list is pending', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockImplementation(() => new Promise(() => {}))
    renderKanbanAt()
    const loading = await screen.findByText(/loading/i)
    expect(loading).toBeInTheDocument()
    expect(loading.closest('[role="status"]')).not.toBeNull()
  })

  it('renders three labelled columns when there are no work items, no Other swimlane', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([])
    renderKanbanAt()
    expect(await screen.findByRole('region', { name: /todo/i })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: /in progress/i })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: /done/i })).toBeInTheDocument()
    expect(screen.queryByRole('region', { name: /other/i })).toBeNull()
  })

  it('places work items in the column matching their frontmatter.status', async () => {
    const todo = makeIndexEntry({
      type: 'work-items', relPath: 'meta/work/0001-todo.md', title: 'Todo work item',
      frontmatter: { type: 'adr-creation-task', status: 'todo' },
    })
    const inProgress = makeIndexEntry({
      type: 'work-items', relPath: 'meta/work/0002-wip.md', title: 'WIP work item',
      frontmatter: { type: 'adr-creation-task', status: 'in-progress' },
    })
    const done = makeIndexEntry({
      type: 'work-items', relPath: 'meta/work/0003-done.md', title: 'Done work item',
      frontmatter: { type: 'adr-creation-task', status: 'done' },
    })
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([todo, inProgress, done])
    renderKanbanAt()
    const todoCol = await screen.findByRole('region', { name: /todo/i })
    expect(within(todoCol).getByText('Todo work item')).toBeInTheDocument()
    expect(within(screen.getByRole('region', { name: /in progress/i })).getByText('WIP work item')).toBeInTheDocument()
    expect(within(screen.getByRole('region', { name: /done/i })).getByText('Done work item')).toBeInTheDocument()
  })

  it('renders the Other swimlane with non-canonical statuses', async () => {
    const blocked = makeIndexEntry({
      type: 'work-items', relPath: 'meta/work/0007-blocked.md', title: 'Blocked work item',
      frontmatter: { type: 'adr-creation-task', status: 'blocked' },
    })
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([blocked])
    renderKanbanAt()
    const other = await screen.findByRole('region', { name: /other/i })
    expect(within(other).getByText('Blocked work item')).toBeInTheDocument()
  })

  it('sorts cards within a column by mtimeMs descending', async () => {
    const old = makeIndexEntry({
      type: 'work-items', relPath: 'meta/work/0001-old.md', title: 'Old',
      frontmatter: { type: 'adr-creation-task', status: 'todo' }, mtimeMs: 100,
    })
    const newest = makeIndexEntry({
      type: 'work-items', relPath: 'meta/work/0002-new.md', title: 'Newest',
      frontmatter: { type: 'adr-creation-task', status: 'todo' }, mtimeMs: 300,
    })
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([old, newest])
    renderKanbanAt()
    const todoCol = await screen.findByRole('region', { name: /todo/i })
    const titles = within(todoCol).getAllByRole('link').map(l => l.textContent)
    const newestIdx = titles.findIndex(t => t?.includes('Newest'))
    const oldIdx = titles.findIndex(t => t?.includes('Old'))
    expect(newestIdx).toBeGreaterThanOrEqual(0)
    expect(oldIdx).toBeGreaterThan(newestIdx)
  })

  it('renders a typed-aware error message on FetchError(5xx)', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockRejectedValue(
      new fetchModule.FetchError(500, 'GET /api/docs?type=work-items: 500'),
    )
    renderKanbanAt()
    const alert = await screen.findByRole('alert')
    expect(alert.textContent).toMatch(/server returned an error/i)
    expect(alert.textContent).not.toMatch(/500/)
    expect(alert.textContent).not.toMatch(/\/api\//)
  })

  it('renders a generic error message on non-FetchError rejection', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockRejectedValue(new Error('boom'))
    renderKanbanAt()
    const alert = await screen.findByRole('alert')
    expect(alert.textContent).toMatch(/something went wrong loading/i)
    expect(alert.textContent).not.toMatch(/server returned an error/i)
    expect(alert.textContent).not.toMatch(/boom/)
  })

  it('renders a Retry button inside the error alert that invalidates the query', async () => {
    const fetchSpy = vi.spyOn(fetchModule, 'fetchDocs')
      .mockRejectedValueOnce(new fetchModule.FetchError(500, 'fail'))
      .mockResolvedValue([])
    const { queryClient } = renderKanbanAt()
    const alert = await screen.findByRole('alert')
    const retry = within(alert).getByRole('button', { name: /retry|try again/i })
    fireEvent.click(retry)
    expect(await screen.findByRole('region', { name: /todo/i })).toBeInTheDocument()
    expect(fetchSpy).toHaveBeenCalledTimes(2)
    expect(queryClient.getQueryState(queryKeys.docs('work-items'))?.status).toBe('success')
  })

  it('links cards to their library detail pages via the canonical typed-route form', async () => {
    const entry = makeIndexEntry({
      type: 'work-items',
      relPath: 'meta/work/0029-template-management-subcommand-surface.md',
      title: 'Template management',
      frontmatter: { type: 'adr-creation-task', status: 'done' },
    })
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([entry])
    renderKanbanAt()
    const link = await screen.findByRole('link', { name: /template management/i })
    expect(link.getAttribute('href')).toBe(
      '/library/work-items/0029-template-management-subcommand-surface',
    )
  })

  it('renders polite-announcement status region initially empty and no conflict banner', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([])
    renderKanbanAt()
    // Wait for the loaded board (the column regions only appear after data loads)
    await screen.findByRole('region', { name: /^todo$/i })
    // Polite announcement region always present, initially empty
    const statusRegion = document.querySelector('[role="status"][aria-live="polite"]')
    expect(statusRegion).not.toBeNull()
    expect(statusRegion!.textContent).toBe('')
    // No conflict banner initially
    expect(screen.queryByRole('alert')).toBeNull()
  })

  it('moves a card between columns when the work items query is invalidated (SSE-driven update)', async () => {
    const before = makeIndexEntry({
      type: 'work-items', relPath: 'meta/work/0001-x.md', title: 'Movable',
      frontmatter: { type: 'adr-creation-task', status: 'todo' },
    })
    const after = makeIndexEntry({
      type: 'work-items', relPath: 'meta/work/0001-x.md', title: 'Movable',
      frontmatter: { type: 'adr-creation-task', status: 'done' },
    })
    const fetchSpy = vi.spyOn(fetchModule, 'fetchDocs')
      .mockResolvedValueOnce([before])
      .mockResolvedValueOnce([after])

    const { queryClient } = renderKanbanAt()
    const todoCol = await screen.findByRole('region', { name: /todo/i })
    expect(within(todoCol).getByText('Movable')).toBeInTheDocument()

    await act(async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.docs('work-items') })
    })

    const doneCol = await screen.findByRole('region', { name: /done/i })
    expect(within(doneCol).getByText('Movable')).toBeInTheDocument()
    expect(within(todoCol).queryByText('Movable')).toBeNull()
    expect(fetchSpy).toHaveBeenCalledTimes(2)
  })
})
