import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { dispatchSseEvent, makeUseDocEvents } from './use-doc-events'
import { queryKeys } from './query-keys'

// ── Pure dispatch tests ──────────────────────────────────────────────────
// No hooks, no EventSource, no async — just exercise the invalidation
// rules directly.
describe('dispatchSseEvent', () => {
  let queryClient: QueryClient

  beforeEach(() => {
    queryClient = new QueryClient()
    vi.spyOn(queryClient, 'invalidateQueries')
  })

  it('invalidates docs query on doc-changed event', () => {
    dispatchSseEvent(
      { type: 'doc-changed', docType: 'plans', path: 'meta/plans/foo.md', etag: 'sha256-abc' },
      queryClient,
    )
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith(
      expect.objectContaining({ queryKey: queryKeys.docs('plans') }),
    )
  })

  it('invalidates doc content for the changed file', () => {
    dispatchSseEvent(
      { type: 'doc-changed', docType: 'plans', path: 'meta/plans/foo.md', etag: 'sha256-abc' },
      queryClient,
    )
    // Refreshes the markdown body when the open detail view's file changes.
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith(
      expect.objectContaining({ queryKey: queryKeys.docContent('meta/plans/foo.md') }),
    )
  })

  it('invalidates kanban on ticket doc-changed event', () => {
    dispatchSseEvent(
      { type: 'doc-changed', docType: 'tickets', path: 'meta/tickets/0001-foo.md', etag: 'sha256-abc' },
      queryClient,
    )
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith(
      expect.objectContaining({ queryKey: queryKeys.kanban() }),
    )
  })

  it('invalidates the lifecycle-cluster prefix on doc-changed event', () => {
    queryClient.setQueryData(queryKeys.lifecycleCluster('foo'), null)
    queryClient.setQueryData(queryKeys.lifecycleCluster('bar'), null)

    dispatchSseEvent(
      { type: 'doc-changed', docType: 'plans', path: 'meta/plans/x.md', etag: 'sha256-x' },
      queryClient,
    )

    expect(queryClient.getQueryState(queryKeys.lifecycleCluster('foo'))?.isInvalidated).toBe(true)
    expect(queryClient.getQueryState(queryKeys.lifecycleCluster('bar'))?.isInvalidated).toBe(true)
  })

  it('also invalidates the lifecycle-cluster prefix on doc-invalid event', () => {
    queryClient.setQueryData(queryKeys.lifecycleCluster('foo'), null)

    dispatchSseEvent(
      { type: 'doc-invalid', docType: 'plans', path: 'meta/plans/x.md' },
      queryClient,
    )

    expect(
      queryClient.getQueryState(queryKeys.lifecycleCluster('foo'))?.isInvalidated,
    ).toBe(true)
  })
})

// ── Wiring tests via the factory ─────────────────────────────────────────
// Construct an isolated hook with a fake EventSource factory. Instance
// capture happens via a test-local closure — no global-stub coordination.
describe('makeUseDocEvents wiring', () => {
  let queryClient: QueryClient

  class FakeEventSource {
    onmessage: ((e: MessageEvent) => void) | null = null
    onerror: ((e: Event) => void) | null = null
    close = vi.fn()
    constructor(public url: string) {}
  }

  beforeEach(() => { queryClient = new QueryClient() })

  function wrapper({ children }: { children: React.ReactNode }) {
    return React.createElement(QueryClientProvider, { client: queryClient }, children)
  }

  it('opens an EventSource to /api/events', () => {
    const factory = vi.fn(
      (url: string) => new FakeEventSource(url) as unknown as EventSource,
    )
    const useDocEvents = makeUseDocEvents(factory)
    renderHook(() => useDocEvents(), { wrapper })
    expect(factory).toHaveBeenCalledWith('/api/events')
  })

  it('closes the EventSource on unmount', () => {
    let captured: FakeEventSource | null = null
    const useDocEvents = makeUseDocEvents((url) => {
      captured = new FakeEventSource(url)
      return captured as unknown as EventSource
    })
    const { unmount } = renderHook(() => useDocEvents(), { wrapper })
    unmount()
    expect(captured!.close).toHaveBeenCalled()
  })

  it('ignores malformed JSON without throwing or invalidating', () => {
    vi.spyOn(queryClient, 'invalidateQueries')
    vi.spyOn(console, 'warn').mockImplementation(() => {})
    let captured: FakeEventSource | null = null
    const useDocEvents = makeUseDocEvents((url) => {
      captured = new FakeEventSource(url)
      return captured as unknown as EventSource
    })
    renderHook(() => useDocEvents(), { wrapper })

    expect(() => {
      captured!.onmessage?.(new MessageEvent('message', { data: 'not json' }))
    }).not.toThrow()
    expect(queryClient.invalidateQueries).not.toHaveBeenCalled()
  })

  it('invalidates all docs queries via prefix match on EventSource error', () => {
    // Seed two populated docs queries so the prefix invalidation can be
    // observed by state change, not just by call shape. This locks in
    // TanStack Query's default partial-match semantics (`exact: false`)
    // — a future global `exact: true` would break reconcile-on-reconnect
    // and this test would catch it.
    queryClient.setQueryData(queryKeys.docs('plans'), [])
    queryClient.setQueryData(queryKeys.docs('tickets'), [])
    vi.spyOn(console, 'warn').mockImplementation(() => {})

    let captured: FakeEventSource | null = null
    const useDocEvents = makeUseDocEvents((url) => {
      captured = new FakeEventSource(url)
      return captured as unknown as EventSource
    })
    renderHook(() => useDocEvents(), { wrapper })

    captured!.onerror?.(new Event('error'))

    // Both child queries are marked stale.
    expect(queryClient.getQueryState(queryKeys.docs('plans'))?.isInvalidated).toBe(true)
    expect(queryClient.getQueryState(queryKeys.docs('tickets'))?.isInvalidated).toBe(true)
  })
})
