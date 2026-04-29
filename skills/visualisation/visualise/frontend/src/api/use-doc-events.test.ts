import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { dispatchSseEvent, makeUseDocEvents } from './use-doc-events'
import { createSelfCauseRegistry } from './self-cause'
import { queryKeys, SESSION_STABLE_QUERY_ROOTS } from './query-keys'

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

  // ── Step 5.5 ────────────────────────────────────────────────────────
  it('invalidates the related prefix with refetchType: "all" on doc-changed', () => {
    dispatchSseEvent(
      { type: 'doc-changed', docType: 'plans', path: 'meta/plans/foo.md', etag: 'sha256-x' },
      queryClient,
    )
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith(
      expect.objectContaining({
        queryKey: queryKeys.relatedPrefix(),
        refetchType: 'all',
      }),
    )
  })

  it('related-prefix invalidation marks unmounted-but-cached related queries stale', () => {
    queryClient.setQueryData(queryKeys.related('meta/plans/a.md'), null)
    queryClient.setQueryData(queryKeys.related('meta/plans/b.md'), null)
    dispatchSseEvent(
      { type: 'doc-changed', docType: 'plans', path: 'meta/plans/x.md', etag: 'sha256-x' },
      queryClient,
    )
    expect(queryClient.getQueryState(queryKeys.related('meta/plans/a.md'))?.isInvalidated).toBe(true)
    expect(queryClient.getQueryState(queryKeys.related('meta/plans/b.md'))?.isInvalidated).toBe(true)
  })

  // ── Step 5.5b ───────────────────────────────────────────────────────
  it('does not invalidate related-prefix on unknown event kinds', () => {
    queryClient.setQueryData(queryKeys.related('meta/plans/a.md'), null)
    dispatchSseEvent(
      { type: 'connected' } as unknown as Parameters<typeof dispatchSseEvent>[0],
      queryClient,
    )
    expect(queryClient.invalidateQueries).not.toHaveBeenCalledWith(
      expect.objectContaining({ queryKey: queryKeys.relatedPrefix() }),
    )
    expect(queryClient.getQueryState(queryKeys.related('meta/plans/a.md'))?.isInvalidated).toBe(false)
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

  beforeEach(() => {
    vi.useFakeTimers()
    queryClient = new QueryClient()
  })
  afterEach(() => { vi.useRealTimers() })

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
    captured!.onopen?.(new Event('open'))

    expect(() => {
      captured!.onmessage?.(new MessageEvent('message', { data: 'not json' }))
    }).not.toThrow()
    expect(queryClient.invalidateQueries).not.toHaveBeenCalled()
  })

  it('invalidates all queries except session-stable on reconnect', () => {
    queryClient.setQueryData(queryKeys.docs('plans'), [])
    queryClient.setQueryData(queryKeys.docs('tickets'), [])
    queryClient.setQueryData(queryKeys.serverInfo(), { name: 'x', version: '1.0.0' })
    vi.spyOn(console, 'warn').mockImplementation(() => {})

    const fakes: FakeEventSource[] = []
    const useDocEvents = makeUseDocEvents((url) => {
      const fake = new FakeEventSource(url)
      fakes.push(fake)
      return fake as unknown as EventSource
    })
    renderHook(() => useDocEvents(), { wrapper })

    // Establish open connection then error → reconnect
    fakes[0].onopen?.(new Event('open'))
    fakes[0].onerror?.(new Event('error'))
    // Max possible first-attempt delay with +20% jitter = 1200ms
    vi.advanceTimersByTime(1500)
    fakes[1].onopen?.(new Event('open'))

    // Docs queries are invalidated on reconnect
    expect(queryClient.getQueryState(queryKeys.docs('plans'))?.isInvalidated).toBe(true)
    expect(queryClient.getQueryState(queryKeys.docs('tickets'))?.isInvalidated).toBe(true)
    // Session-stable query survives
    expect(queryClient.getQueryState(queryKeys.serverInfo())?.isInvalidated).toBe(false)
  })

  it('exposes connectionState and justReconnected', () => {
    vi.spyOn(console, 'warn').mockImplementation(() => {})
    const fakes: FakeEventSource[] = []
    const useDocEvents = makeUseDocEvents((url) => {
      const fake = new FakeEventSource(url)
      fakes.push(fake)
      return fake as unknown as EventSource
    })
    const { result } = renderHook(() => useDocEvents(), { wrapper })

    expect(result.current.connectionState).toBe('connecting')

    act(() => { fakes[0].onopen?.(new Event('open')) })
    expect(result.current.connectionState).toBe('open')
    expect(result.current.justReconnected).toBe(false)

    act(() => { fakes[0].onerror?.(new Event('error')) })
    expect(result.current.connectionState).toBe('reconnecting')

    // Max possible first-attempt delay with +20% jitter = 1200ms
    act(() => { vi.advanceTimersByTime(1500) })
    act(() => { fakes[1].onopen?.(new Event('open')) })
    expect(result.current.connectionState).toBe('open')
    expect(result.current.justReconnected).toBe(true)

    act(() => { vi.advanceTimersByTime(3000) })
    expect(result.current.justReconnected).toBe(false)
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
    const fakes: FakeEventSource[] = []
    const factory = (url: string) => {
      const fake = new FakeEventSource(url)
      fakes.push(fake)
      return fake as unknown as EventSource
    }
    return { factory, get source() { return fakes[fakes.length - 1]! }, fakes }
  }

  beforeEach(() => {
    vi.useFakeTimers()
    queryClient = new QueryClient()
  })
  afterEach(() => { vi.useRealTimers() })

  function wrapper({ children }: { children: React.ReactNode }) {
    return React.createElement(QueryClientProvider, { client: queryClient }, children)
  }

  it('skips invalidation when etag was self-caused', () => {
    vi.spyOn(queryClient, 'invalidateQueries')
    const registry = createSelfCauseRegistry()
    const ctx = makeFactory()
    const useDocEvents = makeUseDocEvents(ctx.factory, registry)
    renderHook(() => useDocEvents(), { wrapper })
    ctx.source.onopen?.(new Event('open'))

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
    ctx.source.onopen?.(new Event('open'))

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
    ctx.source.onopen?.(new Event('open'))

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
    ctx.source.onopen?.(new Event('open'))

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
    ctx.source.onopen?.(new Event('open'))

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

    ctx.source.onopen?.(new Event('open'))
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

    // Queue a pending invalidation during drag
    result.current.setDragInProgress(true)
    ctx.source.onmessage?.(new MessageEvent('message', {
      data: JSON.stringify({ type: 'doc-changed', docType: 'plans', path: 'meta/plans/foo.md', etag: 'sha256-PLAN' }),
    }))
    spy.mockClear()

    // Trigger error → backoff → reconnect
    ctx.source.onerror?.(new Event('error'))
    vi.advanceTimersByTime(1500)
    ctx.fakes[1].onopen?.(new Event('open'))

    expect(registry.has('sha256-X')).toBe(false)
    expect(queryClient.invalidateQueries).toHaveBeenCalled()
  })
})
