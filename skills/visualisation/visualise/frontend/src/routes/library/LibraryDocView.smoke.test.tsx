import { describe, it, expect, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { LibraryDocView } from './LibraryDocView'
import * as fetchModule from '../../api/fetch'
import { dispatchSseEvent } from '../../api/use-doc-events'
import { MemoryRouter } from '../../test/router-helpers'
import type { IndexEntry } from '../../api/types'

const planEntry: IndexEntry = {
  type: 'plans',
  path: '/p/meta/plans/2026-01-01-foo.md',
  relPath: 'meta/plans/2026-01-01-foo.md',
  slug: 'foo',
  title: 'Foo Plan',
  frontmatter: {},
  frontmatterState: 'parsed',
  ticket: null,
  mtimeMs: 0,
  size: 0,
  etag: 'sha256-a',
  bodyPreview: '',
}

const reviewEntry: IndexEntry = {
  ...planEntry,
  type: 'plan-reviews',
  relPath: 'meta/reviews/plans/2026-01-01-foo-review-1.md',
  title: 'Foo review',
}

function buildQueryClient(): QueryClient {
  // Disable retries so rejected fetches surface fast; default
  // gcTime keeps inactive queries cached — load-bearing for Test B.
  return new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
}

function makeWrapper(qc: QueryClient) {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return (
      <QueryClientProvider client={qc}>
        <MemoryRouter>{children}</MemoryRouter>
      </QueryClientProvider>
    )
  }
}

describe('LibraryDocView smoke', () => {
  // ── Step 7.2a ────────────────────────────────────────────────────────
  it('active refetch re-renders the DOM after SSE doc-changed', async () => {
    const qc = buildQueryClient()
    vi.spyOn(fetchModule, 'fetchDocs').mockImplementation((type) => {
      if (type === 'plans') return Promise.resolve([planEntry])
      return Promise.resolve([])
    })
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: 'Body.',
      etag: '"sha256-a"',
    })
    const fetchRelated = vi.spyOn(fetchModule, 'fetchRelated')
    fetchRelated.mockResolvedValueOnce({
      inferredCluster: [],
      declaredOutbound: [],
      declaredInbound: [reviewEntry],
    })
    fetchRelated.mockResolvedValueOnce({
      inferredCluster: [],
      declaredOutbound: [],
      declaredInbound: [],
    })

    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, {
      wrapper: makeWrapper(qc),
    })
    expect(
      await screen.findByRole('heading', { level: 4, name: 'Inbound reviews' }),
    ).toBeInTheDocument()
    expect(fetchRelated).toHaveBeenCalledTimes(1)

    // Dispatch a doc-changed event while the view is still mounted.
    dispatchSseEvent(
      {
        type: 'doc-changed',
        docType: 'plan-reviews',
        path: 'meta/reviews/plans/2026-01-01-foo-review-1.md',
        etag: 'sha256-new',
      },
      qc,
    )

    await waitFor(() => expect(fetchRelated).toHaveBeenCalledTimes(2))
    await waitFor(() => {
      expect(
        screen.queryByRole('heading', { level: 4, name: 'Inbound reviews' }),
      ).toBeNull()
    })
  })

  // ── Step 7.2b ────────────────────────────────────────────────────────
  it('inactive cached query refetches on remount after SSE invalidation', async () => {
    const qc = buildQueryClient()
    vi.spyOn(fetchModule, 'fetchDocs').mockImplementation((type) => {
      if (type === 'plans') return Promise.resolve([planEntry])
      return Promise.resolve([])
    })
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: 'Body.',
      etag: '"sha256-a"',
    })
    const fetchRelated = vi.spyOn(fetchModule, 'fetchRelated')
    fetchRelated.mockResolvedValueOnce({
      inferredCluster: [],
      declaredOutbound: [],
      declaredInbound: [reviewEntry],
    })
    fetchRelated.mockResolvedValueOnce({
      inferredCluster: [],
      declaredOutbound: [],
      declaredInbound: [],
    })

    // First mount — observe inbound review present.
    const first = render(
      <LibraryDocView type="plans" fileSlug="2026-01-01-foo" />,
      { wrapper: makeWrapper(qc) },
    )
    expect(
      await screen.findByRole('heading', { level: 4, name: 'Inbound reviews' }),
    ).toBeInTheDocument()

    // Unmount — query becomes inactive but stays cached per gcTime.
    first.unmount()

    // Dispatch SSE event while unmounted — refetchType: 'all' triggers
    // a refetch on the inactive cached query.
    dispatchSseEvent(
      {
        type: 'doc-changed',
        docType: 'plan-reviews',
        path: 'meta/reviews/plans/2026-01-01-foo-review-1.md',
        etag: 'sha256-new',
      },
      qc,
    )

    await waitFor(() => expect(fetchRelated).toHaveBeenCalledTimes(2))

    // Remount — DOM must reflect the *new* (empty) response on first
    // paint. Without refetchType: 'all', the cache would still hold
    // the original response and the heading would briefly reappear.
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, {
      wrapper: makeWrapper(qc),
    })

    // Wait for any settled "Loading…" to clear, then assert the
    // Inbound reviews group never appears.
    await screen.findByText('Foo Plan')
    await waitFor(() => {
      expect(
        screen.queryByRole('heading', { level: 4, name: 'Inbound reviews' }),
      ).toBeNull()
    })
  })
})
