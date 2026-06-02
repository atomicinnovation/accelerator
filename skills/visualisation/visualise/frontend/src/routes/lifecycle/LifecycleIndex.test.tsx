import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent, within } from '@testing-library/react'
import React from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from '../../test/router-helpers'
import { LifecycleIndex } from './LifecycleIndex'
import * as fetchModule from '../../api/fetch'
import type { LifecycleCluster } from '../../api/types'
import { makeCompleteness, makeIndexEntry } from '../../api/test-fixtures'

const clusters: LifecycleCluster[] = [
  {
    slug: 'older',
    title: 'Older Cluster',
    entries: [
      makeIndexEntry({
        type: 'plans',
        relPath: 'meta/plans/older.md',
        slug: 'older',
        title: 'Older plan',
      }),
    ],
    completeness: makeCompleteness({ hasPlan: true, present: ['plans'] }),
    lastChangedMs: 1_700_000_000_000,
    clusterKey: null,
  },
  {
    slug: 'newer',
    title: 'Newer Cluster',
    entries: [
      makeIndexEntry({
        type: 'work-items',
        relPath: 'meta/work/0001-newer.md',
        slug: 'newer',
        title: 'Newer work item',
        workItemId: '0001',
        frontmatter: { status: 'in-progress' },
      }),
      makeIndexEntry({
        type: 'plans',
        relPath: 'meta/plans/newer.md',
        slug: 'newer',
        title: 'Newer plan',
      }),
      makeIndexEntry({
        type: 'plan-reviews',
        relPath: 'meta/reviews/plans/newer-review-1.md',
        slug: 'newer',
        title: 'Newer plan review',
      }),
      makeIndexEntry({
        type: 'decisions',
        relPath: 'meta/decisions/ADR-0001-newer.md',
        slug: 'newer',
        title: 'Newer ADR',
      }),
    ],
    completeness: makeCompleteness({
      hasWorkItem: true,
      hasPlan: true,
      hasPlanReview: true,
      hasDecision: true,
      present: ['work-items', 'plans', 'plan-reviews', 'decisions'],
    }),
    lastChangedMs: 1_700_500_000_000,
    clusterKey: null,
  },
]

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

describe('LifecycleIndex', () => {
  it('renders the prototype-spec page header (eyebrow, title, subtitle)', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Older Cluster')
    expect(screen.getByText(/^Lifecycle$/i)).toBeInTheDocument()
    expect(
      screen.getByRole('heading', { name: /work units, from idea to shipped/i }),
    ).toBeInTheDocument()
    expect(
      screen.getByText(/each row is a slug-clustered work unit/i),
    ).toBeInTheDocument()
  })

  it('renders a card per cluster with title and slug', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    expect(await screen.findByText('Older Cluster')).toBeInTheDocument()
    expect(screen.getByText('Newer Cluster')).toBeInTheDocument()
  })

  it('renders a Pipeline + N/8 counter inside .ac-lcard__pipe per cluster', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    const { container } = render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Older Cluster')

    const pipes = container.querySelectorAll('.ac-lcard__pipe')
    expect(pipes.length).toBe(clusters.length)

    // Older cluster (after default 'recent' sort, sits second): present = ['plans'] → 1/8
    const older = within(pipes[1] as HTMLElement)
    expect(older.getByText('1/8')).toBeInTheDocument()
    expect(
      (pipes[1] as HTMLElement)
        .querySelector('[data-stage="plans"]')!
        .getAttribute('data-active'),
    ).toBe('true')

    // Newer cluster: present has 4 entries, 4 of which are workflow → 4/8
    const newer = within(pipes[0] as HTMLElement)
    expect(newer.getByText('4/8')).toBeInTheDocument()
    expect(
      (pipes[0] as HTMLElement)
        .querySelector('[data-stage="decisions"]')!
        .getAttribute('data-active'),
    ).toBe('true')
  })

  it('renders a status chip when the cluster has a work-item with a status', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Newer Cluster')
    // 'in-progress' from the newer cluster's work-item frontmatter.
    expect(screen.getByText('in-progress')).toBeInTheDocument()
  })

  it('omits the status chip when the cluster has no work-item entry', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    const { container } = render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Older Cluster')
    // Find the older card by title and assert it has no Chip (Chips render
    // a <span data-variant=...> with neutral/indigo/green/.. values).
    const cards = container.querySelectorAll('li')
    const olderCard = Array.from(cards).find(
      c => c.querySelector('h3')?.textContent === 'Older Cluster',
    )!
    expect(olderCard.querySelector('span[data-variant]')).toBeNull()
  })

  it('renders the per-cluster artifact count with correct pluralisation', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Older Cluster')
    expect(screen.getByText('1 artifact')).toBeInTheDocument()
    expect(screen.getByText('4 artifacts')).toBeInTheDocument()
  })

  it('orders by most-recently-changed by default (Updated)', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Newer Cluster')
    const titles = screen.getAllByRole('heading', { level: 3 }).map(h => h.textContent)
    expect(titles[0]).toBe('Newer Cluster')
    expect(titles[1]).toBe('Older Cluster')
  })

  it('sorts by completeness when the Completeness pill is pressed', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Newer Cluster')
    fireEvent.click(screen.getByRole('button', { name: /completeness/i }))
    const titles = screen.getAllByRole('heading', { level: 3 }).map(h => h.textContent)
    expect(titles[0]).toBe('Newer Cluster')
    expect(titles[1]).toBe('Older Cluster')
  })

  it('exposes the sort group as a segmented control (Updated + Completeness)', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Older Cluster')
    const group = screen.getByRole('group', { name: /sort clusters/i })
    expect(within(group).getByRole('button', { name: /updated/i })).toBeInTheDocument()
    expect(
      within(group).getByRole('button', { name: /completeness/i }),
    ).toBeInTheDocument()
    // No "Oldest" button — dropped to match the prototype.
    expect(
      within(group).queryByRole('button', { name: /oldest/i }),
    ).toBeNull()
  })

  it('renders 8 pipeline tiles per card', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Newer Cluster')
    const pipelines = screen.getAllByRole('list', { name: /lifecycle pipeline/i })
    expect(pipelines).toHaveLength(2)
    pipelines.forEach(p =>
      expect(within(p).getAllByRole('listitem')).toHaveLength(8),
    )
  })

  it('shows empty state when no clusters', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue([])
    render(<LifecycleIndex />, { wrapper: Wrapper })
    expect(await screen.findByText(/no lifecycle clusters found/i)).toBeInTheDocument()
  })

  it('shows loading state while fetching', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockImplementation(
      () => new Promise(() => { /* pending forever */ }),
    )
    render(<LifecycleIndex />, { wrapper: Wrapper })
    expect(await screen.findByText(/loading/i)).toBeInTheDocument()
  })

  it('shows a generic alert on FetchError without leaking the URL', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockRejectedValue(
      new fetchModule.FetchError(500, 'GET /api/lifecycle: 500'),
    )
    render(<LifecycleIndex />, { wrapper: Wrapper })
    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent(/could not load lifecycle clusters/i)
    expect(alert.textContent).not.toMatch(/\/api\//)
  })

  it('shows a generic alert on non-FetchError rejections', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockRejectedValue(new Error('boom'))
    render(<LifecycleIndex />, { wrapper: Wrapper })
    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent(/something went wrong/i)
    expect(alert.textContent).not.toMatch(/boom/)
  })
})
