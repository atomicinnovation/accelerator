import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { dispatchSseEvent, makeUseDocEvents } from './use-doc-events'
import { createSelfCauseRegistry } from './self-cause'
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
    onopen: ((e: Event) => void) | null = null
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

// ── Self-cause filter + drag-suppress ────────────────────────────────────
describe('makeUseDocEvents self-cause + drag-suppress', () => {
  let queryClient: QueryClient

  class FakeEventSource {
    onmessage: ((e: MessageEvent) => void) | null = null
    onerror: ((e: Event) => void) | null = null
    onopen: ((e: Event) => void) | null = null
    close = vi.fn()
    constructor(public url: string) {}
  }

  function makeFactory() {
    let captured: FakeEventSource | null = null
    const factory = (url: string) => {
      captured = new FakeEventSource(url)
      return captured as unknown as EventSource
    }
    return { factory, get source() { return captured! } }
  }

  beforeEach(() => { queryClient = new QueryClient() })

  function wrapper({ children }: { children: React.ReactNode }) {
    return React.createElement(QueryClientProvider, { client: queryClient }, children)
  }

  it('skips invalidation when etag was self-caused', () => {
    vi.spyOn(queryClient, 'invalidateQueries')
    const registry = createSelfCauseRegistry()
    const ctx = makeFactory()
    const useDocEvents = makeUseDocEvents(ctx.factory, registry)
    renderHook(() => useDocEvents(), { wrapper })

    registry.register('sha256-X')
    ctx.source.onmessage?.(new MessageEvent('message', {
      data: JSON.stringify({ type: 'doc-changed', docType: 'tickets', path: 'meta/tickets/foo.md', etag: 'sha256-X' }),
    }))

    expect(queryClient.invalidateQueries).not.toHaveBeenCalled()
  })

  it('still invalidates for unknown etags', () => {
    vi.spyOn(queryClient, 'invalidateQueries')
    const registry = createSelfCauseRegistry()
    const ctx = makeFactory()
    const useDocEvents = makeUseDocEvents(ctx.factory, registry)
    renderHook(() => useDocEvents(), { wrapper })

    ctx.source.onmessage?.(new MessageEvent('message', {
      data: JSON.stringify({ type: 'doc-changed', docType: 'tickets', path: 'meta/tickets/foo.md', etag: 'sha256-FOREIGN' }),
    }))

    expect(queryClient.invalidateQueries).toHaveBeenCalled()
  })

  it('suppresses duplicate self-caused events (non-consuming)', () => {
    vi.spyOn(queryClient, 'invalidateQueries')
    const registry = createSelfCauseRegistry()
    const ctx = makeFactory()
    const useDocEvents = makeUseDocEvents(ctx.factory, registry)
    renderHook(() => useDocEvents(), { wrapper })

    registry.register('sha256-X')
    const msg = new MessageEvent('message', {
      data: JSON.stringify({ type: 'doc-changed', docType: 'tickets', path: 'meta/tickets/foo.md', etag: 'sha256-X' }),
    })
    ctx.source.onmessage?.(msg)
    ctx.source.onmessage?.(msg)

    expect(queryClient.invalidateQueries).not.toHaveBeenCalled()
  })

  it('queues invalidation during drag and flushes on drop', () => {
    vi.spyOn(queryClient, 'invalidateQueries')
    const registry = createSelfCauseRegistry()
    const ctx = makeFactory()
    const useDocEvents = makeUseDocEvents(ctx.factory, registry)
    const { result } = renderHook(() => useDocEvents(), { wrapper })

    result.current.setDragInProgress(true)
    ctx.source.onmessage?.(new MessageEvent('message', {
      data: JSON.stringify({ type: 'doc-changed', docType: 'tickets', path: 'meta/tickets/foo.md', etag: 'sha256-FOREIGN' }),
    }))
    expect(queryClient.invalidateQueries).not.toHaveBeenCalled()

    result.current.setDragInProgress(false)
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith(
      expect.objectContaining({ queryKey: queryKeys.docs('tickets') }),
    )
  })

  it('coalesces multiple invalidations for same key during drag', () => {
    const spy = vi.spyOn(queryClient, 'invalidateQueries')
    const registry = createSelfCauseRegistry()
    const ctx = makeFactory()
    const useDocEvents = makeUseDocEvents(ctx.factory, registry)
    const { result } = renderHook(() => useDocEvents(), { wrapper })

    result.current.setDragInProgress(true)
    const makeMsg = (path: string) => new MessageEvent('message', {
      data: JSON.stringify({ type: 'doc-changed', docType: 'tickets', path, etag: `sha256-${path}` }),
    })
    ctx.source.onmessage?.(makeMsg('meta/tickets/a.md'))
    ctx.source.onmessage?.(makeMsg('meta/tickets/b.md'))
    ctx.source.onmessage?.(makeMsg('meta/tickets/c.md'))

    result.current.setDragInProgress(false)

    const docsTicketsCalls = spy.mock.calls.filter(
      ([arg]) => JSON.stringify((arg as { queryKey: unknown }).queryKey) === JSON.stringify(queryKeys.docs('tickets'))
    )
    expect(docsTicketsCalls).toHaveLength(1)
  })

  it('resets registry and flushes pending invalidations on SSE reconnect', () => {
    const spy = vi.spyOn(queryClient, 'invalidateQueries')
    vi.spyOn(console, 'warn').mockImplementation(() => {})
    const registry = createSelfCauseRegistry()
    const ctx = makeFactory()
    const useDocEvents = makeUseDocEvents(ctx.factory, registry)
    const { result } = renderHook(() => useDocEvents(), { wrapper })

    registry.register('sha256-X')

    // Queue an invalidation during drag, then flush (clear the pending set)
    result.current.setDragInProgress(true)
    ctx.source.onmessage?.(new MessageEvent('message', {
      data: JSON.stringify({ type: 'doc-changed', docType: 'tickets', path: 'meta/tickets/foo.md', etag: 'sha256-FOREIGN' }),
    }))
    result.current.setDragInProgress(false)
    spy.mockClear()

    // Re-register and simulate disconnect → reconnect
    registry.register('sha256-X')
    ctx.source.onerror?.(new Event('error'))

    // Re-queue a pending invalidation that the reconnect flush should dispatch
    result.current.setDragInProgress(true)
    ctx.source.onmessage?.(new MessageEvent('message', {
      data: JSON.stringify({ type: 'doc-changed', docType: 'plans', path: 'meta/plans/foo.md', etag: 'sha256-PLAN' }),
    }))
    spy.mockClear()

    // Reconnect: should reset registry and flush pending invalidations
    ctx.source.onopen?.(new Event('open'))

    expect(registry.has('sha256-X')).toBe(false)
    expect(queryClient.invalidateQueries).toHaveBeenCalled()
  })
})
