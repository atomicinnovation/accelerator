import { describe, it, expect, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { LibraryDocView } from './LibraryDocView'
import * as fetchModule from '../../api/fetch'
import { dispatchSseEvent } from '../../api/use-doc-events'
import type { IndexEntry } from '../../api/types'
import { MemoryRouter } from '../../test/router-helpers'

const mockEntry: IndexEntry = {
  type: 'plans', path: '/p/meta/plans/2026-01-01-foo.md',
  relPath: 'meta/plans/2026-01-01-foo.md',
  slug: 'foo', title: 'Foo Plan',
  frontmatter: { status: 'draft' }, frontmatterState: 'parsed', workItemRefs: [],
  mtimeMs: 1_700_000_000_000, size: 100, etag: 'sha256-a',
  bodyPreview: '',
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
    expect(await screen.findByText('draft')).toBeInTheDocument()
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
    expect(await screen.findByText(/Document not found/i)).toBeInTheDocument()
  })

  it('renders Loading while the content fetch is pending', async () => {
    // fetchDocs resolves so the entry is found, but fetchDocContent never
    // resolves — keeps the component in isLoading state indefinitely.
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockImplementation(
      () => new Promise(() => { /* pending forever */ }),
    )
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    // findByText waits for RouterProvider to settle and fetchDocs to resolve
    expect(await screen.findByText(/Loading…/i)).toBeInTheDocument()
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
  it('fetches related on mount and renders the inbound group when present', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: 'Body.',
      etag: '"sha256-a"',
    })
    const review: IndexEntry = {
      ...mockEntry,
      type: 'plan-reviews',
      relPath: 'meta/reviews/plans/2026-01-01-foo-review-1.md',
      title: 'Foo review',
    }
    vi.spyOn(fetchModule, 'fetchRelated').mockResolvedValue({
      inferredCluster: [],
      declaredOutbound: [],
      declaredInbound: [review],
    })
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    expect(
      await screen.findByRole('heading', { level: 4, name: 'Inbound reviews' }),
    ).toBeInTheDocument()
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
})
