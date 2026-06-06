import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { LibraryDocView } from './LibraryDocView'
import * as fetchModule from '../../api/fetch'
import { dispatchSseEvent } from '../../api/use-doc-events'
import type { IndexEntry } from '../../api/types'
import { makeIndexEntry, makeLifecycleCluster } from '../../api/test-fixtures'
import { MemoryRouter } from '../../test/router-helpers'

const mockEntry: IndexEntry = {
  type: 'plans', path: '/p/meta/plans/2026-01-01-foo.md',
  relPath: 'meta/plans/2026-01-01-foo.md',
  slug: 'foo', workItemId: null, title: 'Foo Plan',
  frontmatter: { status: 'draft' }, frontmatterState: 'parsed', workItemRefs: [],
  mtimeMs: 1_700_000_000_000, size: 100, etag: 'sha256-a',
  bodyPreview: '',
  completeness: null, linkedCount: 0, clusterKey: null,
}

function Wrapper({ children }: { children: React.ReactNode }) {
  // Disable retries in tests so rejected fetches surface as error state
  // immediately instead of re-firing the mock and slowing the suite.
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>{children}</MemoryRouter>
    </QueryClientProvider>
  )
}

describe('LibraryDocView', () => {
  // Wiring useDocCluster into the view makes every resolved-document test
  // call fetchLifecycleClusters. Default it to an empty list (no matching
  // cluster) so the suite never hits a real /api/lifecycle fetch; the
  // cluster-present/error tests override this inside the test body.
  // (restoreMocks: true resets spies per test, so this must re-run here.)
  beforeEach(() => {
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue([])
  })

  it('renders the doc title', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    // Use content that doesn't repeat the title to avoid duplicate h1s in DOM
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: 'Body text.',
      etag: '"sha256-a"',
    })
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    expect(await screen.findByText('Foo Plan')).toBeInTheDocument()
  })

  it('renders frontmatter chips', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: '# Title', etag: '"sha256-a"',
    })
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    // 'draft' appears both in the chip strip and in the FrontmatterTable.
    expect((await screen.findAllByText('draft')).length).toBeGreaterThan(0)
  })

  it('renders the markdown body', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: '# Foo Plan\nBody text here.', etag: '"sha256-a"',
    })
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    expect(await screen.findByText('Body text here.')).toBeInTheDocument()
  })

  it('shows Document not found when the slug does not match any entry', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    render(<LibraryDocView type="plans" fileSlug="nonexistent" />, { wrapper: Wrapper })
    expect(
      await screen.findByRole('heading', { level: 1, name: /Document not found/i }),
    ).toBeInTheDocument()
  })

  it('renders Loading while the content fetch is pending', async () => {
    // fetchDocs resolves so the entry is found, but fetchDocContent never
    // resolves — keeps the component in isLoading state indefinitely.
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockImplementation(
      () => new Promise(() => { /* pending forever */ }),
    )
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    expect(
      (await screen.findAllByText(/Loading…/i)).length,
    ).toBeGreaterThan(0)
  })

  it('renders an error alert when fetchDocs rejects', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockRejectedValue(new Error('list-boom'))
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    expect(await screen.findByRole('alert')).toHaveTextContent(/Failed to load document list/i)
  })

  it('renders an error alert when fetchDocContent rejects', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockRejectedValue(new Error('content-boom'))
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    expect(await screen.findByRole('alert')).toHaveTextContent(/Failed to load document content/i)
  })

  // ── Step 6.6 ──────────────────────────────────────────────────────────
  it('fetches related on mount and renders an inbound declared row when present', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: 'Body.',
      etag: '"sha256-a"',
    })
    const review: IndexEntry = {
      ...mockEntry,
      type: 'plan-reviews',
      path: '/p/meta/reviews/plans/2026-01-01-foo-review-1.md',
      relPath: 'meta/reviews/plans/2026-01-01-foo-review-1.md',
      title: 'Foo review',
    }
    vi.spyOn(fetchModule, 'fetchRelated').mockResolvedValue({
      inferredCluster: [],
      declaredOutbound: [],
      declaredInbound: [review],
    })
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    // Option B: the inbound entry renders as a single declared-tagged row.
    const link = await screen.findByRole('link', { name: 'Foo review' })
    expect(link).toBeInTheDocument()
    expect(screen.getByText('(declared)')).toBeInTheDocument()
    expect(screen.queryByRole('heading', { level: 4 })).toBeNull()
  })

  // ── Step 6.6b ─────────────────────────────────────────────────────────
  it('renders error path with role=alert when fetchRelated fails', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: 'Body.',
      etag: '"sha256-a"',
    })
    vi.spyOn(fetchModule, 'fetchRelated').mockRejectedValue(new Error('related-boom'))
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    expect(
      await screen.findByText(/Failed to load related artifacts/i),
    ).toBeInTheDocument()
  })

  // ── Step 6.7 ──────────────────────────────────────────────────────────
  it('real wiring resolves wiki-link to anchor when ADR is in cache', async () => {
    const adrEntry: IndexEntry = {
      ...mockEntry,
      type: 'decisions',
      relPath: 'meta/decisions/ADR-0001-example.md',
      title: 'Example decision',
      frontmatter: { adr_id: 'ADR-0001' },
    }
    vi.spyOn(fetchModule, 'fetchDocs').mockImplementation((type) => {
      if (type === 'decisions') return Promise.resolve([adrEntry])
      if (type === 'plans') return Promise.resolve([mockEntry])
      return Promise.resolve([])
    })
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: 'Reference: [[ADR-0001]] in body.',
      etag: '"sha256-a"',
    })
    vi.spyOn(fetchModule, 'fetchRelated').mockResolvedValue({
      inferredCluster: [],
      declaredOutbound: [],
      declaredInbound: [],
    })
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    const link = await screen.findByRole('link', { name: 'Example decision' })
    expect(link.getAttribute('href')).toBe('/library/decisions/ADR-0001-example')
    expect(link.getAttribute('title')).toBe('[[ADR-0001]]')
  })

  // ── Step 6.8 ──────────────────────────────────────────────────────────
  it('renders unresolved-wiki-link span when ADR is not in cache after settle', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockImplementation((type) => {
      // Both decisions and work items settle as empty arrays — resolver
      // moves from kind=pending to kind=unresolved.
      if (type === 'plans') return Promise.resolve([mockEntry])
      return Promise.resolve([])
    })
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: 'Missing ref: [[ADR-9999]].',
      etag: '"sha256-a"',
    })
    vi.spyOn(fetchModule, 'fetchRelated').mockResolvedValue({
      inferredCluster: [],
      declaredOutbound: [],
      declaredInbound: [],
    })
    const { container } = render(
      <LibraryDocView type="plans" fileSlug="2026-01-01-foo" />,
      { wrapper: Wrapper },
    )
    // Wait for the pending → unresolved flip after both fetchDocs
    // queries settle. The unresolved span appears once the resolver
    // rotates and MarkdownRenderer re-runs the plugin pipeline.
    const span = await waitFor(() => {
      const el = container.querySelector('span.unresolved-wiki-link')
      if (!el) throw new Error('not yet')
      return el
    })
    expect(span.textContent).toBe('[[ADR-9999]]')
    expect(span.getAttribute('title')).toBe('No matching ADR found for ID 9999')
  })

  // ── Phase 10.6: Malformed-frontmatter banner ─────────────────────────
  it('renders a malformed-frontmatter banner when entry.frontmatterState is malformed', async () => {
    const malformedEntry: IndexEntry = {
      ...mockEntry,
      frontmatterState: 'malformed',
    }
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([malformedEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: '# body', etag: '"sha256-a"',
    })
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    const banner = await screen.findByLabelText(/metadata header/i)
    expect(banner).toHaveTextContent(/We couldn.t read this document.s metadata header/i)
    expect(banner).not.toHaveAttribute('role', 'status')
  })

  it('does not render the malformed banner for parsed or absent state', async () => {
    for (const state of ['parsed', 'absent'] as const) {
      const entry: IndexEntry = {
        ...mockEntry,
        relPath: `meta/plans/2026-01-01-${state}.md`,
        frontmatterState: state,
      }
      vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([entry])
      vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
        content: '# body', etag: '"sha256-a"',
      })
      const { unmount } = render(
        <LibraryDocView type="plans" fileSlug={`2026-01-01-${state}`} />,
        { wrapper: Wrapper },
      )
      await screen.findByRole('article')
      expect(screen.queryByLabelText(/metadata header/i)).toBeNull()
      unmount()
    }
  })

  // ── Phase 0078: FrontmatterTable integration ─────────────────────────
  it('renders the FrontmatterTable when frontmatterState is parsed', async () => {
    const entry: IndexEntry = {
      ...mockEntry,
      frontmatter: { kind: 'story', status: 'ready', parent: '' },
      frontmatterState: 'parsed',
    }
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([entry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: '# Body', etag: '"sha256-a"',
    })
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    expect(
      await screen.findByLabelText('Document metadata'),
    ).toBeInTheDocument()
  })

  it('does NOT render the FrontmatterTable when frontmatterState is malformed', async () => {
    const entry: IndexEntry = {
      ...mockEntry,
      frontmatter: {},
      frontmatterState: 'malformed',
    }
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([entry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: '# Body', etag: '"sha256-a"',
    })
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    await screen.findByLabelText(/metadata header/i)
    expect(screen.queryByLabelText('Document metadata')).toBeNull()
  })

  it('does NOT render the FrontmatterTable when frontmatterState is absent', async () => {
    const entry: IndexEntry = {
      ...mockEntry,
      frontmatter: {},
      frontmatterState: 'absent',
    }
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([entry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: '# Body', etag: '"sha256-a"',
    })
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    await screen.findByRole('article')
    expect(screen.queryByLabelText('Document metadata')).toBeNull()
  })

  it('strips a leading YAML frontmatter block before rendering the markdown body', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: '---\ntitle: Foo\nstatus: draft\n---\n\n# Body heading\n\nBody text here.',
      etag: '"sha256-a"',
    })
    const { container } = render(
      <LibraryDocView type="plans" fileSlug="2026-01-01-foo" />,
      { wrapper: Wrapper },
    )
    // Body still renders.
    expect(await screen.findByText('Body text here.')).toBeInTheDocument()
    // No HR from the trailing `---` of the frontmatter block.
    expect(container.querySelector('article hr')).toBeNull()
    // The setext-H1 collision (title line followed by `---`) is gone — the
    // body markdown contains exactly one h1, sourced from `# Body heading`.
    const h1s = container.querySelectorAll('article h1')
    expect(h1s.length).toBe(1)
    expect(h1s[0].textContent).toBe('Body heading')
  })

  it('linkifies a WORK-ITEM scalar value via the shared resolver (end-to-end)', async () => {
    const referenced: IndexEntry = {
      ...mockEntry,
      type: 'work-items',
      relPath: 'meta/work/0041-page-wrapper.md',
      slug: '0041-page-wrapper',
      workItemId: '0041',
      title: 'Page wrapper',
    }
    const subject: IndexEntry = {
      ...mockEntry,
      type: 'work-items',
      relPath: 'meta/work/0078-detail-page-frontmatter-table.md',
      slug: '0078-detail-page-frontmatter-table',
      workItemId: '0078',
      frontmatter: { parent: 'WORK-ITEM-0041' },
      frontmatterState: 'parsed',
    }
    vi.spyOn(fetchModule, 'fetchDocs').mockImplementation((type) => {
      if (type === 'work-items') return Promise.resolve([subject, referenced])
      return Promise.resolve([])
    })
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: '# Body', etag: '"sha256-a"',
    })
    vi.spyOn(fetchModule, 'fetchRelated').mockResolvedValue({
      inferredCluster: [],
      declaredOutbound: [],
      declaredInbound: [],
    })

    const { container } = render(
      <LibraryDocView type="work-items" fileSlug="0078-detail-page-frontmatter-table" />,
      { wrapper: Wrapper },
    )

    await screen.findByLabelText('Document metadata')
    const link = await waitFor(() => {
      const el = container.querySelector(
        'a[href$="/library/work-items/0041-page-wrapper"]',
      )
      if (!el) throw new Error('not yet')
      return el
    })
    expect(link.textContent).toBe('WORK-ITEM-0041')
  })

  it('shows the malformed banner mid-session when docs query refetches with malformed state', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: '# body', etag: '"sha256-a"',
    })
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    function WrapperWithQC({ children }: { children: React.ReactNode }) {
      return (
        <QueryClientProvider client={qc}>
          <MemoryRouter>{children}</MemoryRouter>
        </QueryClientProvider>
      )
    }
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: WrapperWithQC })
    await screen.findByRole('article')
    expect(screen.queryByLabelText(/metadata header/i)).toBeNull()

    // Simulate: SSE doc-invalid flips entry to malformed on refetch
    const malformedEntry: IndexEntry = { ...mockEntry, frontmatterState: 'malformed' }
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([malformedEntry])
    dispatchSseEvent(
      { type: 'doc-invalid', docType: 'plans', path: mockEntry.relPath },
      qc,
    )
    await screen.findByLabelText(/metadata header/i)
  })

  // ── 0079: Cluster block ───────────────────────────────────────────────
  it('renders a Cluster section after File when the doc is in a cluster', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: 'Body.',
      etag: '"sha256-a"',
    })
    vi.spyOn(fetchModule, 'fetchRelated').mockResolvedValue({
      inferredCluster: [],
      declaredOutbound: [],
      declaredInbound: [],
    })
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue([
      makeLifecycleCluster({
        slug: '0001',
        title: 'Foo cluster',
        entries: [makeIndexEntry({ path: mockEntry.path })],
      }),
    ])
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })

    const clusterLink = await screen.findByRole('link', { name: /Foo cluster/ })
    expect(clusterLink.getAttribute('href')).toBe('/lifecycle/0001')

    // DOM order: Related artifacts → File → Cluster.
    const headings = screen
      .getAllByRole('heading', { level: 3 })
      .map((h) => h.textContent)
    expect(headings).toEqual(['Related artifacts', 'File', 'Cluster'])
  })

  it('shows the Cluster heading + Loading while the cluster query is pending', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: 'Body.',
      etag: '"sha256-a"',
    })
    vi.spyOn(fetchModule, 'fetchRelated').mockResolvedValue({
      inferredCluster: [],
      declaredOutbound: [],
      declaredInbound: [],
    })
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockImplementation(
      () => new Promise(() => { /* pending forever */ }),
    )
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })

    // The section shell does not vanish: heading + a loading state show.
    const clusterHeading = await screen.findByRole('heading', { name: 'Cluster' })
    const section = clusterHeading.closest('section')!
    expect(within(section).getByText('Loading…')).toBeInTheDocument()
  })

  it('shows a role=alert in the Cluster section when fetchLifecycleClusters rejects', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: 'Body.',
      etag: '"sha256-a"',
    })
    vi.spyOn(fetchModule, 'fetchRelated').mockResolvedValue({
      inferredCluster: [],
      declaredOutbound: [],
      declaredInbound: [],
    })
    vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockRejectedValue(
      new Error('lifecycle-boom'),
    )
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })

    expect(await screen.findByText(/Could not load cluster/i)).toBeInTheDocument()
    const clusterHeading = screen.getByRole('heading', { name: 'Cluster' })
    const section = clusterHeading.closest('section')!
    expect(within(section).getByRole('alert')).toBeInTheDocument()
  })

  it('renders no Cluster section once the query settles with no matching cluster', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: 'Body.',
      etag: '"sha256-a"',
    })
    vi.spyOn(fetchModule, 'fetchRelated').mockResolvedValue({
      inferredCluster: [],
      declaredOutbound: [],
      declaredInbound: [],
    })
    // Default beforeEach stub resolves []; the doc is in no cluster.
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })

    await screen.findByText('Foo Plan')
    // Anchor on the settled signal: the would-be Cluster section's Loading…
    // clears and the heading is absent (it must not pass against the
    // still-loading shell).
    await waitFor(() => {
      expect(screen.queryByRole('heading', { name: 'Cluster' })).toBeNull()
    })
  })
})
