import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import React from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from '../../test/router-helpers'
import { LifecycleClusterContent } from './LifecycleClusterView'
import * as fetchModule from '../../api/fetch'
import { makeIndexEntry } from '../../api/test-fixtures'
import type { LifecycleCluster, Completeness, IndexEntry } from '../../api/types'

const empty: Completeness = {
  hasWorkItem: false, hasResearch: false, hasPlan: false,
  hasPlanReview: false, hasValidation: false, hasPr: false,
  hasPrReview: false, hasDecision: false, hasNotes: false,
  hasDesignInventory: false, hasDesignGap: false,
}

function entry(
  type: IndexEntry['type'],
  rel: string,
  title: string,
  mtime: number,
  bodyPreview = '',
): IndexEntry {
  return makeIndexEntry({
    type,
    path: `/x/${rel}`,
    relPath: rel,
    title,
    frontmatter: { date: '2026-04-18' },
    mtimeMs: mtime,
    bodyPreview,
  })
}

const cluster: LifecycleCluster = {
  slug: 'foo', title: 'Foo Cluster',
  entries: [
    entry(
      'plans', 'meta/plans/2026-04-18-foo.md', 'The Foo Plan', 100,
      'A short summary of what the plan covers.',
    ),
    entry('decisions', 'meta/decisions/ADR-0007-foo.md', 'ADR Foo', 200),
  ],
  completeness: { ...empty, hasPlan: true, hasDecision: true },
  lastChangedMs: 200,
}

function Wrapper({ children }: { children: React.ReactNode }) {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>{children}</MemoryRouter>
    </QueryClientProvider>
  )
}

describe('LifecycleClusterContent', () => {
  it('renders the cluster title and slug', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    expect(await screen.findByRole('heading', { name: 'Foo Cluster' })).toBeInTheDocument()
    expect(screen.getByText('foo')).toBeInTheDocument()
  })

  it('renders a back-link to the lifecycle index', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    const back = await screen.findByRole('link', { name: /all clusters/i })
    expect(back.getAttribute('href')).toBe('/lifecycle')
  })

  it('renders one card per present entry', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    expect(await screen.findByText('The Foo Plan')).toBeInTheDocument()
    expect(screen.getByText('ADR Foo')).toBeInTheDocument()
  })

  it('renders a faded placeholder for each absent stage', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    expect(await screen.findByText('The Foo Plan')).toBeInTheDocument()
    expect(screen.getByText(/no work item yet/i)).toBeInTheDocument()
    expect(screen.getByText(/no research yet/i)).toBeInTheDocument()
    expect(screen.getByText(/no plan review yet/i)).toBeInTheDocument()
    expect(screen.getByText(/no validation yet/i)).toBeInTheDocument()
    expect(screen.getByText(/no pr yet/i)).toBeInTheDocument()
    expect(screen.getByText(/no pr review yet/i)).toBeInTheDocument()
  })

  it('present-entry cards link to the library page', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    const link = await screen.findByRole('link', { name: /the foo plan/i })
    expect(link.getAttribute('href')).toBe('/library/plans/2026-04-18-foo')
  })

  it('renders bodyPreview on cards that have one and omits the element when empty', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    expect(
      await screen.findByText(/short summary of what the plan covers/i),
    ).toBeInTheDocument()
    const allPreviews = screen.queryAllByText(/short summary/i)
    expect(allPreviews).toHaveLength(1)
  })

  it('shows loading state while fetching', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockImplementation(
      () => new Promise(() => { /* pending forever */ }),
    )
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    expect(await screen.findByText(/loading/i)).toBeInTheDocument()
  })

  it('shows a "no such cluster" message and a back-link on 404', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockRejectedValue(
      new fetchModule.FetchError(404, 'GET /api/lifecycle/foo: 404'),
    )
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    expect(await screen.findByRole('alert')).toHaveTextContent(/no cluster called/i)
    expect(screen.getByRole('link', { name: /all clusters/i })).toBeInTheDocument()
  })

  it('shows a generic error message on 5xx without leaking the URL', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockRejectedValue(
      new fetchModule.FetchError(500, 'GET /api/lifecycle/foo: 500'),
    )
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent(/something went wrong/i)
    expect(alert.textContent).not.toMatch(/\/api\//)
  })

  it('renders Notes entries in a separate "Other" long-tail section', async () => {
    const withNotes: LifecycleCluster = {
      ...cluster,
      entries: [
        ...cluster.entries,
        entry('notes', 'meta/notes/2026-04-20-foo.md', 'A scratch note', 150),
      ],
      completeness: { ...cluster.completeness, hasNotes: true },
    }
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(withNotes)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    expect(await screen.findByRole('region', { name: /other artifacts/i }))
      .toBeInTheDocument()
    expect(screen.getByText('A scratch note')).toBeInTheDocument()
    expect(screen.queryByText(/no notes yet/i)).not.toBeInTheDocument()
  })

  it('shows design inventory and design gap in the long-tail section when present', async () => {
    const withDesignDocs: LifecycleCluster = {
      ...cluster,
      entries: [
        ...cluster.entries,
        entry('design-inventories', 'meta/design-inventories/2026-05-01-foo.md', 'Foo Design Inventory', 150),
        entry('design-gaps', 'meta/design-gaps/2026-05-02-foo.md', 'Foo Design Gap', 160),
      ],
      completeness: { ...cluster.completeness, hasDesignInventory: true, hasDesignGap: true },
    }
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(withDesignDocs)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    expect(await screen.findByRole('region', { name: /other artifacts/i }))
      .toBeInTheDocument()
    expect(screen.getByText('Foo Design Inventory')).toBeInTheDocument()
    expect(screen.getByText('Foo Design Gap')).toBeInTheDocument()
    expect(screen.queryByText(/no design inventory yet/i)).not.toBeInTheDocument()
    expect(screen.queryByText(/no design gap yet/i)).not.toBeInTheDocument()
  })

  it('hides the "Other" long-tail section when no long-tail entries exist', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    await screen.findByText('The Foo Plan')
    expect(screen.queryByRole('region', { name: /other artifacts/i }))
      .not.toBeInTheDocument()
  })

  it('renders multiple entries within a single stage', async () => {
    const multi: LifecycleCluster = {
      ...cluster,
      entries: [
        ...cluster.entries,
        entry('plan-reviews', 'meta/reviews/plans/foo-review-1.md', 'Foo plan review 1', 110),
        entry('plan-reviews', 'meta/reviews/plans/foo-review-2.md', 'Foo plan review 2', 130),
      ],
      completeness: { ...cluster.completeness, hasPlanReview: true },
    }
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(multi)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    expect(await screen.findByText('Foo plan review 1')).toBeInTheDocument()
    expect(screen.getByText('Foo plan review 2')).toBeInTheDocument()
  })
})
