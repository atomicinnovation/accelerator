import { describe, it, expect, vi, afterEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { RouterProvider } from '@tanstack/react-router'
import { QueryClientProvider } from '@tanstack/react-query'
import * as fetchModule from './api/fetch'
import {
  setupRouterFixtures, buildRouter, waitForPath,
} from './test/router-fixtures'

setupRouterFixtures()

function renderAt(url: string) {
  const { router, queryClient } = buildRouter(url)
  render(
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  )
  return router
}

function stubKanbanConfigFetch(columns = [
  { key: 'draft', label: 'Draft' },
  { key: 'ready', label: 'Ready' },
  { key: 'in-progress', label: 'In progress' },
]) {
  vi.stubGlobal('fetch', vi.fn((url: string) => {
    if (url === '/api/kanban/config') {
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ columns }) })
    }
    return Promise.reject(new Error(`unexpected fetch: ${url}`))
  }))
}

describe('router', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
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
        hasDesignInventory: false, hasDesignGap: false,
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

  it('routes /kanban to the kanban board and renders configured columns', async () => {
    stubKanbanConfigFetch()
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([])
    const router = renderAt('/kanban')
    await waitForPath(router, '/kanban')
    expect(await screen.findByRole('region', { name: /draft/i })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: /ready/i })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: /in progress/i })).toBeInTheDocument()
    expect(screen.queryByRole('region', { name: /other/i })).toBeNull()
  })

  it('does not render the legacy "coming in Phase 7" stub copy at /kanban', async () => {
    stubKanbanConfigFetch()
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([])
    const router = renderAt('/kanban')
    await waitForPath(router, '/kanban')
    expect(screen.queryByText(/coming in phase 7/i)).toBeNull()
  })
})

describe('loader crumbs', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('/library/templates → Templates crumb with Library ancestor', async () => {
    const router = renderAt('/library/templates')
    await waitForPath(router, '/library/templates')
    const matches = router.state.matches
    const templatesMatch = matches.find(m => m.routeId.includes('templates') && !m.routeId.includes('$name'))
    const libraryMatch = matches.find(m => m.routeId === '/library')
    expect((templatesMatch?.loaderData as any)?.crumb).toBe('Templates')
    expect((libraryMatch?.loaderData as any)?.crumb).toBe('Library')
  })

  it('/library/templates/adr → adr crumb', async () => {
    const router = renderAt('/library/templates/adr')
    await waitForPath(router, '/library/templates/adr')
    const matches = router.state.matches
    const detailMatch = matches.find(m => m.routeId.includes('$name'))
    expect((detailMatch?.loaderData as any)?.crumb).toBe('adr')
  })

  it('/library/decisions → decisions crumb with Library ancestor', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([])
    const router = renderAt('/library/decisions')
    await waitForPath(router, '/library/decisions')
    const matches = router.state.matches
    const typeMatch = matches.find(m => m.routeId.includes('$type'))
    const libraryMatch = matches.find(m => m.routeId === '/library')
    expect((typeMatch?.loaderData as any)?.crumb).toBe('decisions')
    expect((libraryMatch?.loaderData as any)?.crumb).toBe('Library')
  })

  it('/library/decisions/some-slug → some-slug crumb with Library and decisions ancestors', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([
      { slug: 'some-slug', title: 'Some', docType: 'decisions', tags: [], lastModifiedMs: 0 } as any,
    ])
    const router = renderAt('/library/decisions/some-slug')
    await waitForPath(router, '/library/decisions/some-slug')
    const matches = router.state.matches
    const docMatch = matches.find(m => m.routeId.includes('$fileSlug'))
    const typeMatch = matches.find(m => m.routeId.includes('$type'))
    const libraryMatch = matches.find(m => m.routeId === '/library')
    expect((docMatch?.loaderData as any)?.crumb).toBe('some-slug')
    expect((typeMatch?.loaderData as any)?.crumb).toBe('decisions')
    expect((libraryMatch?.loaderData as any)?.crumb).toBe('Library')
  })

  it('/lifecycle → Lifecycle crumb', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue([])
    const router = renderAt('/lifecycle')
    await waitForPath(router, '/lifecycle')
    const matches = router.state.matches
    const lifecycleMatch = matches.find(m => m.routeId === '/lifecycle')
    expect((lifecycleMatch?.loaderData as any)?.crumb).toBe('Lifecycle')
  })

  it('/lifecycle/some-cluster → some-cluster crumb with Lifecycle ancestor', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue({
      slug: 'some-cluster', title: 'Some Cluster', entries: [],
      completeness: {
        hasWorkItem: false, hasResearch: false, hasPlan: false,
        hasPlanReview: false, hasValidation: false, hasPr: false,
        hasPrReview: false, hasDecision: false, hasNotes: false,
        hasDesignInventory: false, hasDesignGap: false,
      },
      lastChangedMs: 0,
    })
    const router = renderAt('/lifecycle/some-cluster')
    await waitForPath(router, '/lifecycle/some-cluster')
    const matches = router.state.matches
    const clusterMatch = matches.find(m => m.routeId.includes('$slug'))
    const lifecycleMatch = matches.find(m => m.routeId === '/lifecycle')
    expect((clusterMatch?.loaderData as any)?.crumb).toBe('some-cluster')
    expect((lifecycleMatch?.loaderData as any)?.crumb).toBe('Lifecycle')
  })

  it('/kanban → Kanban crumb', async () => {
    stubKanbanConfigFetch()
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([])
    const router = renderAt('/kanban')
    await waitForPath(router, '/kanban')
    const matches = router.state.matches
    const kanbanMatch = matches.find(m => m.routeId === '/kanban')
    expect((kanbanMatch?.loaderData as any)?.crumb).toBe('Kanban')
  })
})
