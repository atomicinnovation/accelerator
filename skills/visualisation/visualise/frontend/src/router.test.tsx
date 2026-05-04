import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import {
  RouterProvider, createRouter, createMemoryHistory,
} from '@tanstack/react-router'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { routeTree } from './router'
import * as fetchModule from './api/fetch'

function renderAt(url: string) {
  const router = createRouter({
    routeTree,
    history: createMemoryHistory({ initialEntries: [url] }),
  })
  const qc = new QueryClient()
  render(
    <QueryClientProvider client={qc}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  )
  return router
}

/** Wait for the router to settle at a specific pathname. Multi-hop
 *  redirect chains (e.g. `/` → `/library` → `/library/decisions`) require
 *  multiple re-evaluation passes that `router.load()` does not
 *  single-shot resolve; `waitFor` polls the router state until the
 *  expected destination is reached. */
async function waitForPath(
  router: { state: { location: { pathname: string } } },
  expected: string,
): Promise<void> {
  await waitFor(() => {
    expect(router.state.location.pathname).toBe(expected)
  })
}

describe('router', () => {
  // RootLayout fetches /api/types and useDocEvents opens EventSource; stub
  // network calls so routing logic is what's actually tested.
  beforeEach(() => {
    vi.spyOn(fetchModule, 'fetchTypes').mockResolvedValue([])
    vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: [] })
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue({
      name: 'adr', activeTier: 'plugin-default', tiers: [],
    })
  })

  it('redirects / to /library/decisions (via /library)', async () => {
    // Chain: / → /library → /library/decisions
    const router = renderAt('/')
    await waitForPath(router, '/library/decisions')
  })

  it('redirects bare /library to /library/decisions', async () => {
    const router = renderAt('/library')
    await waitForPath(router, '/library/decisions')
  })

  it('routes /library/templates to the templates index', async () => {
    const router = renderAt('/library/templates')
    await waitForPath(router, '/library/templates')
    // Heading from LibraryTemplatesIndex — matched via the literal
    // /library/templates route, not the generic /library/$type.
    expect(
      await screen.findByRole('heading', { name: 'Templates' }),
    ).toBeInTheDocument()
  })

  it('routes /library/templates/adr to the templates detail view', async () => {
    const router = renderAt('/library/templates/adr')
    await waitForPath(router, '/library/templates/adr')
    // LibraryTemplatesView heading is the template name; matched via the
    // literal /library/templates/$name route.
    expect(
      await screen.findByRole('heading', { name: 'adr' }),
    ).toBeInTheDocument()
  })

  it('redirects /library/bogus to /library/decisions when the type is unknown', async () => {
    // parseParams on libraryTypeRoute throws redirect({ to: '/library' })
    // for any string that is not a DocTypeKey; /library then chains to
    // /library/decisions.
    const router = renderAt('/library/bogus')
    await waitForPath(router, '/library/decisions')
  })

  it('routes /lifecycle to the index view', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue([])
    const router = renderAt('/lifecycle')
    await waitForPath(router, '/lifecycle')
    expect(
      await screen.findByText(/no lifecycle clusters/i),
    ).toBeInTheDocument()
  })

  it('routes /lifecycle/foo to the cluster detail view', async () => {
    const spy = vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue({
      slug: 'foo', title: 'Foo Cluster', entries: [],
      completeness: {
        hasWorkItem: false, hasResearch: false, hasPlan: false,
        hasPlanReview: false, hasValidation: false, hasPr: false,
        hasPrReview: false, hasDecision: false, hasNotes: false,
      },
      lastChangedMs: 0,
    })
    const router = renderAt('/lifecycle/foo')
    await waitForPath(router, '/lifecycle/foo')
    expect(
      await screen.findByRole('heading', { name: 'Foo Cluster' }),
    ).toBeInTheDocument()
    expect(spy).toHaveBeenCalledWith('foo')
  })

  it('routes /kanban to the kanban board with three columns', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([])
    const router = renderAt('/kanban')
    await waitForPath(router, '/kanban')
    expect(await screen.findByRole('region', { name: /todo/i })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: /in progress/i })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: /done/i })).toBeInTheDocument()
    expect(screen.queryByRole('region', { name: /other/i })).toBeNull()
  })

  it('does not render the legacy "coming in Phase 7" stub copy at /kanban', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([])
    const router = renderAt('/kanban')
    await waitForPath(router, '/kanban')
    expect(screen.queryByText(/coming in phase 7/i)).toBeNull()
  })
})
