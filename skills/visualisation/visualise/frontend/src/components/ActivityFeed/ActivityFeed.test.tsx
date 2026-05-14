import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor, act } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ActivityFeed } from './ActivityFeed'
import * as fetchModule from '../../api/fetch'
import * as docEventsModule from '../../api/use-doc-events'
import type { ActivityEvent, SseEvent } from '../../api/types'
import type { ConnectionState } from '../../api/reconnecting-event-source'
import type { DocEventsHandle } from '../../api/use-doc-events'

// AC11 (self-caused events reach the feed) is covered at the hook layer
// in `use-doc-events.test.ts` via the `subscribe + self-caused` test —
// the ActivityFeed never consults the self-cause registry, so duplicating
// the assertion here would not add incremental coverage.

vi.mock('../../api/use-doc-events', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../api/use-doc-events')>()
  return {
    ...actual,
    useDocEventsContext: vi.fn(),
  }
})
vi.mock('../../api/fetch', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../api/fetch')>()
  return {
    ...actual,
    fetchActivity: vi.fn(),
  }
})

interface MountResult {
  getListener: () => ((e: SseEvent) => void) | undefined
  queryClient: QueryClient
}

function mockHandle(overrides: Partial<DocEventsHandle> = {}): DocEventsHandle {
  return {
    setDragInProgress: vi.fn(),
    connectionState: 'open',
    justReconnected: false,
    subscribe: () => () => {},
    ...overrides,
  }
}

function mountWith({
  initial,
  connectionState = 'open' as const,
}: {
  initial: ActivityEvent[] | Promise<ActivityEvent[]>
  connectionState?: ConnectionState
}): MountResult {
  let captured: ((e: SseEvent) => void) | undefined
  vi.mocked(fetchModule.fetchActivity).mockReturnValue(
    initial instanceof Promise ? initial : Promise.resolve(initial),
  )
  vi.mocked(docEventsModule.useDocEventsContext).mockReturnValue(
    mockHandle({
      connectionState,
      subscribe: (listener) => {
        captured = listener
        return () => {}
      },
    }),
  )
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  render(
    <QueryClientProvider client={queryClient}>
      <ActivityFeed />
    </QueryClientProvider>,
  )
  return { getListener: () => captured, queryClient }
}

function makeEvent(overrides: Partial<ActivityEvent> = {}): ActivityEvent {
  return {
    action: 'edited',
    docType: 'plans',
    path: 'meta/plans/foo.md',
    timestamp: '2026-05-13T12:00:00Z',
    ...overrides,
  }
}

function makeSseEvent(overrides: Partial<Extract<SseEvent, { type: 'doc-changed' }>> = {}): SseEvent {
  return {
    type: 'doc-changed',
    action: 'edited',
    docType: 'plans',
    path: 'meta/plans/foo.md',
    etag: 'sha256-abc',
    timestamp: '2026-05-13T12:00:00Z',
    ...overrides,
  }
}

describe('ActivityFeed', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders rows from initial history with new SSE events prepended', async () => {
    const initial = [makeEvent({ path: 'meta/plans/old.md', timestamp: '2026-05-13T10:00:00Z' })]
    const { getListener } = mountWith({ initial })

    // Wait for initial fetch to resolve.
    await screen.findByText(/old\.md/)

    act(() => {
      getListener()!(makeSseEvent({ action: 'created', path: 'meta/plans/new.md', timestamp: '2026-05-13T11:00:00Z' }))
    })

    const items = screen.getAllByRole('listitem')
    expect(items[0].textContent).toMatch(/created/)
    expect(items[0].textContent).toMatch(/new\.md/)
    expect(items[1].textContent).toMatch(/edited/)
    expect(items[1].textContent).toMatch(/old\.md/)
  })

  it('shows LIVE badge when connectionState is open', async () => {
    mountWith({ initial: [makeEvent()] })
    await screen.findByText(/foo\.md/)
    expect(screen.getByTestId('activity-live-badge')).toHaveTextContent('LIVE')
  })

  it('hides LIVE badge when connectionState is closed', async () => {
    mountWith({ initial: [makeEvent()], connectionState: 'closed' })
    await screen.findByText(/foo\.md/)
    expect(screen.queryByTestId('activity-live-badge')).toBeNull()
  })

  it('refreshes relative-times on each 60s tick', async () => {
    const NOW = new Date('2026-05-13T12:00:30Z').getTime()
    vi.useFakeTimers()
    vi.setSystemTime(NOW)
    try {
      const ev = makeEvent({ timestamp: '2026-05-13T12:00:30Z' })
      mountWith({ initial: [ev] })
      // Flush React Query's microtasks under fake timers.
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0)
      })
      expect(screen.getByText(/0s ago/)).toBeTruthy()

      await act(async () => {
        await vi.advanceTimersByTimeAsync(60_000)
      })
      expect(screen.getByText(/1m ago/)).toBeTruthy()

      await act(async () => {
        await vi.advanceTimersByTimeAsync(60_000)
      })
      expect(screen.getByText(/2m ago/)).toBeTruthy()
    } finally {
      vi.useRealTimers()
    }
  })

  it('fetches the initial history with limit=5', async () => {
    mountWith({ initial: [makeEvent()] })
    await screen.findByText(/foo\.md/)
    expect(fetchModule.fetchActivity).toHaveBeenCalledWith(5)
  })

  it('renders an empty state when history is empty', async () => {
    mountWith({ initial: [] })
    await screen.findByTestId('activity-empty')
    expect(screen.getByTestId('activity-empty')).toHaveTextContent('No recent activity')
    expect(screen.queryByRole('listitem')).toBeNull()
  })

  it('shows neither empty state nor rows while loading', () => {
    // Never-resolving fetch — render synchronously and assert the
    // intermediate state has no flashing empty placeholder.
    mountWith({ initial: new Promise<ActivityEvent[]>(() => {}) })
    expect(screen.queryByTestId('activity-empty')).toBeNull()
    expect(screen.queryByRole('listitem')).toBeNull()
  })

  it('dedupes when a live event has the same identity as an initial entry', async () => {
    const a = makeEvent({ path: 'meta/plans/a.md', timestamp: '2026-05-13T11:00:00Z' })
    const b = makeEvent({ path: 'meta/plans/b.md', timestamp: '2026-05-13T10:00:00Z' })
    const { getListener } = mountWith({ initial: [a, b] })

    await screen.findByText(/a\.md/)

    // Replay event `a` via the listener — same activityRowId.
    act(() => {
      getListener()!(makeSseEvent({
        action: a.action,
        docType: a.docType,
        path: a.path,
        timestamp: a.timestamp,
      }))
    })

    expect(screen.getAllByRole('listitem').length).toBe(2)
  })

  it('drops doc-changed events with empty action or timestamp', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const { getListener } = mountWith({ initial: [] })
    await screen.findByTestId('activity-empty')

    act(() => {
      getListener()!({
        type: 'doc-changed',
        action: '' as 'created',
        docType: 'plans',
        path: 'meta/plans/bad.md',
        timestamp: '2026-05-13T12:00:00Z',
      })
    })

    // Empty state still shown — bad event was dropped.
    expect(screen.queryByText(/bad\.md/)).toBeNull()
    expect(warn).toHaveBeenCalled()
  })

  it('orders newer initial entries above older live entries via sort step', async () => {
    // Live event arrives first with older timestamp; initial fetch
    // resolves later with a newer entry. Sort must place the newer one
    // on top.
    let resolveInitial!: (value: ActivityEvent[]) => void
    const pending = new Promise<ActivityEvent[]>(r => { resolveInitial = r })
    const { getListener } = mountWith({ initial: pending })

    // Live event (older).
    await waitFor(() => expect(getListener()).toBeDefined())
    act(() => {
      getListener()!(makeSseEvent({ path: 'meta/plans/older-live.md', timestamp: '2026-05-13T10:00:00Z' }))
    })

    // Initial fetch resolves (newer).
    await act(async () => {
      resolveInitial([makeEvent({ path: 'meta/plans/newer-initial.md', timestamp: '2026-05-13T11:00:00Z' })])
    })

    await screen.findByText(/newer-initial\.md/)
    const items = screen.getAllByRole('listitem')
    expect(items[0].textContent).toMatch(/newer-initial\.md/)
    expect(items[1].textContent).toMatch(/older-live\.md/)
  })
})
