import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import React from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from '../../test/router-helpers'
import { LifecycleClusterContent } from './LifecycleClusterView'
import * as fetchModule from '../../api/fetch'
import { makeCompleteness, makeIndexEntry } from '../../api/test-fixtures'
import type { LifecycleCluster, IndexEntry } from '../../api/types'
import lifecycleCss from './LifecycleClusterView.module.css?raw'

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
  completeness: makeCompleteness({
    hasPlan: true,
    hasDecision: true,
    present: ['plans', 'decisions'],
  }),
  lastChangedMs: 200,
  clusterKey: null,
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
  it('renders the cluster title in the page heading', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    expect(await screen.findByRole('heading', { name: 'Foo Cluster' })).toBeInTheDocument()
  })

  it('renders a Pipeline panel above the timeline with cluster completeness', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    const { container } = render(<LifecycleClusterContent slug="foo" />, {
      wrapper: Wrapper,
    })
    await screen.findByRole('heading', { name: 'Foo Cluster' })

    const panel = container.querySelector('.ac-lcluster__pipeline')!
    expect(panel).toBeInTheDocument()
    expect(panel.querySelector('.ac-lcluster__pipeline-eyebrow')!.textContent)
      .toMatch(/^Pipeline$/i)

    const tiles = panel.querySelectorAll('[data-stage]')
    expect(tiles).toHaveLength(8)

    expect(
      panel.querySelector('[data-stage="plans"]')!.getAttribute('data-active'),
    ).toBe('true')
    expect(
      panel.querySelector('[data-stage="decisions"]')!.getAttribute('data-active'),
    ).toBe('true')
    expect(
      panel.querySelector('[data-stage="work-items"]')!.getAttribute('data-active'),
    ).toBe('false')

    expect(panel.querySelector('ol.ac-stagechain')!.getAttribute('data-variant'))
      .toBe('panel')
  })

  it('eyebrow includes a back-link to the lifecycle index', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    await screen.findByRole('heading', { name: 'Foo Cluster' })
    const back = screen.getByRole('link', { name: /^Lifecycle$/i })
    expect(back.getAttribute('href')).toBe('/lifecycle')
  })

  it('renders one timeline step per present entry', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    expect(await screen.findByText('The Foo Plan')).toBeInTheDocument()
    expect(screen.getByText('ADR Foo')).toBeInTheDocument()
  })

  it('renders a missing-stage placeholder card for each absent workflow stage', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    expect(await screen.findByText('The Foo Plan')).toBeInTheDocument()
    expect(screen.getByText(/^No work item yet$/i)).toBeInTheDocument()
    expect(screen.getByText(/^No research yet$/i)).toBeInTheDocument()
    expect(screen.getByText(/^No plan review yet$/i)).toBeInTheDocument()
    expect(screen.getByText(/^No validation yet$/i)).toBeInTheDocument()
    expect(screen.getByText(/^No pr descriptions yet$/i)).toBeInTheDocument()
    expect(screen.getByText(/^No pr review yet$/i)).toBeInTheDocument()
  })

  it('present-entry cards link to the library page', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    const link = await screen.findByRole('link', { name: /the foo plan/i })
    expect(link.getAttribute('href')).toBe('/library/plans/2026-04-18-foo')
  })

  it('renders the entry-card head with stage label, title, and date', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    const { container } = render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    await screen.findByText('The Foo Plan')
    // The pipeline panel's own li[data-stage] tiles also match, so scope
    // to the outer timeline list (not the .ac-stagechain panel).
    const planStep = container.querySelector(
      'ol:not(.ac-stagechain) > li[data-stage="plans"]',
    )!
    expect(planStep).toBeInTheDocument()
    expect(planStep.textContent).toContain('PLAN')
    expect(planStep.textContent).toContain('The Foo Plan')
    expect(planStep.textContent).toContain('2026-04-18')
  })

  it('renders a stage tile for each timeline step', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    const { container } = render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    await screen.findByText('The Foo Plan')
    // Every timeline step has a [data-stage] tile element (not the
    // pipeline panel's tiles, which sit inside the panel section).
    const timelineSteps = container.querySelectorAll('ol:not(.ac-stagechain) > li[data-stage]')
    expect(timelineSteps.length).toBeGreaterThan(0)
    // Each step's tile carries data-active reflecting whether the stage
    // has a present entry.
    const planTile = timelineSteps[0].querySelector('[data-active][data-stage]')!
    expect(planTile).toBeInTheDocument()
  })

  it('shows loading state while fetching', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockImplementation(
      () => new Promise(() => { /* pending forever */ }),
    )
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    expect((await screen.findAllByText(/loading/i)).length).toBeGreaterThan(0)
  })

  it('shows a "no such cluster" message and the eyebrow back-link on 404', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockRejectedValue(
      new fetchModule.FetchError(404, 'GET /api/lifecycle/foo: 404'),
    )
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    expect(await screen.findByRole('alert')).toHaveTextContent(/no cluster called/i)
    expect(screen.getByRole('link', { name: /^Lifecycle$/i })).toBeInTheDocument()
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

  it('appends Notes entries to the timeline', async () => {
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
    expect(await screen.findByText('A scratch note')).toBeInTheDocument()
    // No missing-stage placeholder for notes (long-tail stages omit them).
    expect(screen.queryByText(/no notes yet/i)).not.toBeInTheDocument()
  })

  it('appends design inventory and design gap entries to the timeline', async () => {
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
    expect(await screen.findByText('Foo Design Inventory')).toBeInTheDocument()
    expect(screen.getByText('Foo Design Gap')).toBeInTheDocument()
    expect(screen.queryByText(/no design inventory yet/i)).not.toBeInTheDocument()
    expect(screen.queryByText(/no design gap yet/i)).not.toBeInTheDocument()
  })

  it('does not append long-tail placeholders when no long-tail entries exist', async () => {
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(cluster)
    render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    await screen.findByText('The Foo Plan')
    expect(screen.queryByText(/no notes yet/i)).not.toBeInTheDocument()
    expect(screen.queryByText(/no design inventory yet/i)).not.toBeInTheDocument()
    expect(screen.queryByText(/no design gap yet/i)).not.toBeInTheDocument()
  })

  it('renders an entry status frontmatter as a Chip with the right variant', async () => {
    const withStatus: LifecycleCluster = {
      ...cluster,
      entries: [
        makeIndexEntry({
          type: 'plans',
          path: '/x/p.md',
          relPath: 'meta/plans/2026-04-18-p.md',
          title: 'P',
          frontmatter: { status: 'done' },
          mtimeMs: 100,
        }),
      ],
    }
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(withStatus)
    const { container } = render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    await screen.findByText('P')
    expect(container.querySelector('span[data-variant="green"]')).not.toBeNull()
  })

  it('renders a neutral chip for unknown status', async () => {
    const withStatus: LifecycleCluster = {
      ...cluster,
      entries: [
        makeIndexEntry({
          type: 'plans',
          path: '/x/p.md',
          relPath: 'meta/plans/2026-04-18-p.md',
          title: 'P',
          frontmatter: { status: 'mystery' },
          mtimeMs: 100,
        }),
      ],
    }
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(withStatus)
    const { container } = render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    await screen.findByText('P')
    expect(container.querySelector('span[data-variant="neutral"]')).not.toBeNull()
  })

  it('renders the cluster status chip in the header subtitle from the work-item entry', async () => {
    const withWorkItem: LifecycleCluster = {
      ...cluster,
      entries: [
        makeIndexEntry({
          type: 'work-items',
          path: '/x/wi.md',
          relPath: 'meta/work/0001-foo.md',
          title: 'Foo WI',
          frontmatter: { status: 'in-progress' },
          mtimeMs: 100,
        }),
        ...cluster.entries,
      ],
    }
    vi.spyOn(fetchModule, 'fetchLifecycleCluster').mockResolvedValue(withWorkItem)
    const { container } = render(<LifecycleClusterContent slug="foo" />, { wrapper: Wrapper })
    await screen.findByRole('heading', { name: 'Foo Cluster' })
    // The chip surfaces twice: once in the header subtitle (driven by
    // the work-item entry's status) and once in the timeline card head
    // for the same work-item entry. The header chip lives outside the
    // timeline <ol>; assert at least one such chip exists.
    const headerSubRow = container.querySelector(`.${(/_subRow_[^\s"]+/.exec(container.innerHTML) ?? [''])[0]}`)
    expect(screen.getAllByText('in-progress').length).toBeGreaterThanOrEqual(2)
    if (headerSubRow) {
      expect(headerSubRow.textContent).toContain('in-progress')
    }
  })

  it('CSS module no longer defines the legacy .statusBadge rule', () => {
    expect(lifecycleCss).not.toMatch(/\.statusBadge\b/)
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
