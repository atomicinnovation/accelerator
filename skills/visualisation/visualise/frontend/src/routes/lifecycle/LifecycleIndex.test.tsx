import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent, within } from '@testing-library/react'
import React from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from '../../components/Sidebar/test-helpers'
import { LifecycleIndex } from './LifecycleIndex'
import * as fetchModule from '../../api/fetch'
import type { LifecycleCluster, Completeness } from '../../api/types'

const empty: Completeness = {
  hasTicket: false, hasResearch: false, hasPlan: false,
  hasPlanReview: false, hasValidation: false, hasPr: false,
  hasPrReview: false, hasDecision: false, hasNotes: false,
}

const clusters: LifecycleCluster[] = [
  {
    slug: 'older',
    title: 'Older Cluster',
    entries: [],
    completeness: { ...empty, hasPlan: true },
    lastChangedMs: 1_700_000_000_000,
  },
  {
    slug: 'newer',
    title: 'Newer Cluster',
    entries: [],
    completeness: { ...empty, hasPlan: true, hasPlanReview: true, hasDecision: true },
    lastChangedMs: 1_700_500_000_000,
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
  it('renders a card per cluster with title and slug', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    expect(await screen.findByText('Older Cluster')).toBeInTheDocument()
    expect(screen.getByText('Newer Cluster')).toBeInTheDocument()
  })

  it('orders by most-recently-changed by default', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Newer Cluster')
    const titles = screen.getAllByRole('heading', { level: 3 }).map(h => h.textContent)
    expect(titles[0]).toBe('Newer Cluster')
    expect(titles[1]).toBe('Older Cluster')
  })

  it('sorts by completeness when "Completeness" sort is chosen', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Newer Cluster')
    fireEvent.click(screen.getByRole('button', { name: /completeness/i }))
    const titles = screen.getAllByRole('heading', { level: 3 }).map(h => h.textContent)
    expect(titles[0]).toBe('Newer Cluster')
    expect(titles[1]).toBe('Older Cluster')
  })

  it('breaks completeness ties by most-recently-changed', async () => {
    const tied: LifecycleCluster[] = [
      {
        slug: 'older-equal',
        title: 'Older Equal',
        entries: [],
        completeness: { ...empty, hasPlan: true, hasDecision: true },
        lastChangedMs: 1_700_000_000_000,
      },
      {
        slug: 'newer-equal',
        title: 'Newer Equal',
        entries: [],
        completeness: { ...empty, hasPlan: true, hasDecision: true },
        lastChangedMs: 1_700_500_000_000,
      },
    ]
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(tied)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Newer Equal')
    fireEvent.click(screen.getByRole('button', { name: /completeness/i }))
    const titles = screen.getAllByRole('heading', { level: 3 }).map(h => h.textContent)
    expect(titles[0]).toBe('Newer Equal')
    expect(titles[1]).toBe('Older Equal')
  })

  it('sorts by oldest when "Oldest" sort is chosen', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Newer Cluster')
    fireEvent.click(screen.getByRole('button', { name: /oldest/i }))
    const titles = screen.getAllByRole('heading', { level: 3 }).map(h => h.textContent)
    expect(titles[0]).toBe('Older Cluster')
    expect(titles[1]).toBe('Newer Cluster')
  })

  it('renders 8 pipeline dots per card', async () => {
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
    // RouterProvider initialises asynchronously, so we wait for the loading
    // text to appear rather than asserting synchronously.
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

  it('filters clusters by title or slug substring (case-insensitive)', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Newer Cluster')

    const input = screen.getByRole('searchbox', { name: /filter clusters/i })

    fireEvent.change(input, { target: { value: 'NEWER' } })
    expect(screen.getByText('Newer Cluster')).toBeInTheDocument()
    expect(screen.queryByText('Older Cluster')).not.toBeInTheDocument()

    fireEvent.change(input, { target: { value: 'older' } })
    expect(screen.getByText('Older Cluster')).toBeInTheDocument()
    expect(screen.queryByText('Newer Cluster')).not.toBeInTheDocument()

    fireEvent.change(input, { target: { value: '' } })
    expect(screen.getByText('Newer Cluster')).toBeInTheDocument()
    expect(screen.getByText('Older Cluster')).toBeInTheDocument()
  })

  it('shows a no-match message when the filter excludes every cluster', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
    render(<LifecycleIndex />, { wrapper: Wrapper })
    await screen.findByText('Newer Cluster')
    fireEvent.change(
      screen.getByRole('searchbox', { name: /filter clusters/i }),
      { target: { value: 'zzz-no-match' } },
    )
    expect(screen.getByText(/no clusters match "zzz-no-match"/i)).toBeInTheDocument()
  })
})
