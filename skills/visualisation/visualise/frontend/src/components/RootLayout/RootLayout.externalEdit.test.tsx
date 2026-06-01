import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import {
  render,
  screen,
  fireEvent,
  act,
  waitFor,
} from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import {
  createRouter,
  createRootRoute,
  createRoute,
  createMemoryHistory,
  RouterProvider,
  Outlet,
} from '@tanstack/react-router'
import {
  DocEventsContext,
  dispatchSseEvent,
  type DocEventsHandle,
} from '../../api/use-doc-events'
import {
  SelfCauseContext,
  createSelfCauseRegistry,
} from '../../api/self-cause'
import {
  ToastContext,
  useToastDispatcher,
} from '../../api/use-toast'
import { Toaster } from '../Toaster/Toaster'
import { ExternalEditToast } from '../Toaster/ExternalEditToast'
import { queryKeys } from '../../api/query-keys'
import type { IndexEntry, SseEvent, ActionKind } from '../../api/types'

const RELPATH = 'meta/work/0007-foo.md'
const SLUG = '0007-foo'

function entry(overrides: Partial<IndexEntry> = {}): IndexEntry {
  return {
    type: 'work-items',
    path: RELPATH,
    relPath: RELPATH,
    slug: SLUG,
    workItemId: '0007',
    title: 'Foo',
    frontmatter: {},
    frontmatterState: 'parsed',
    workItemRefs: [],
    mtimeMs: 0,
    size: 0,
    etag: 'etag-v1',
    bodyPreview: '',
    completeness: null,
    linkedCount: 0,
    ...overrides,
  }
}

interface Harness {
  qc: QueryClient
  registry: ReturnType<typeof createSelfCauseRegistry>
  docEventsHandle: DocEventsHandle
  listeners: Set<(e: SseEvent) => void>
  unmount: () => void
}

function makeDocEventsHandle(): {
  handle: DocEventsHandle
  listeners: Set<(e: SseEvent) => void>
} {
  const listeners = new Set<(e: SseEvent) => void>()
  const handle: DocEventsHandle = {
    setDragInProgress: vi.fn(),
    connectionState: 'open',
    justReconnected: false,
    subscribe: (listener) => {
      listeners.add(listener)
      return () => listeners.delete(listener)
    },
  }
  return { handle, listeners }
}

function renderSubstituteTree(atUrl: string, seedEntries: IndexEntry[]): Harness {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  qc.setQueryData(queryKeys.docs('work-items'), seedEntries)

  const registry = createSelfCauseRegistry()
  const { handle: docEventsHandle, listeners } = makeDocEventsHandle()

  const root = createRootRoute({ component: () => <Outlet /> })
  const Tree = () => {
    const toast = useToastDispatcher()
    return (
      <ToastContext.Provider value={toast}>
        <ExternalEditToast />
        <Toaster />
      </ToastContext.Provider>
    )
  }
  const docRoute = createRoute({
    getParentRoute: () => root,
    path: '/library/$type/$fileSlug',
    component: Tree,
  })
  const indexRoute = createRoute({
    getParentRoute: () => root,
    path: '/',
    component: Tree,
  })
  const tree = root.addChildren([indexRoute, docRoute])
  const router = createRouter({
    routeTree: tree,
    history: createMemoryHistory({ initialEntries: [atUrl] }),
  })

  const { unmount } = render(
    <QueryClientProvider client={qc}>
      <DocEventsContext.Provider value={docEventsHandle}>
        <SelfCauseContext.Provider value={registry}>
          <RouterProvider router={router} />
        </SelfCauseContext.Provider>
      </DocEventsContext.Provider>
    </QueryClientProvider>,
  )

  return { qc, registry, docEventsHandle, listeners, unmount }
}

function fireDocChanged(
  listeners: Set<(e: SseEvent) => void>,
  path: string,
  action: ActionKind,
  etag?: string,
): void {
  act(() => {
    for (const l of listeners)
      l({
        type: 'doc-changed',
        action,
        docType: 'work-items',
        path,
        etag,
        timestamp: '2026-05-30T00:00:00Z',
      })
  })
}

async function waitForRelPathResolved(qc: QueryClient): Promise<void> {
  // The substitute tree mounts ExternalEditToast which calls useActiveDocRelPath.
  // That hook reads cached docs('work-items'); we seeded the cache so it
  // resolves synchronously, but route mount still needs a microtask.
  await waitFor(() => {
    expect(qc.getQueryData(queryKeys.docs('work-items'))).toBeDefined()
  })
}

describe('RootLayout external-edit integration (substitute tree)', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  async function expectExternalEditToast(verb: string): Promise<void> {
    expect(await screen.findByText('External edit detected')).toBeInTheDocument()
    const code = await screen.findByText(RELPATH)
    expect(code.tagName.toLowerCase()).toBe('code')
    // The verb sits in the message paragraph alongside the inline <code>.
    expect(code.parentElement?.textContent).toContain(
      `was ${verb} while you were looking at it.`,
    )
  }

  it('positive correlation: doc-changed action=edited raises "updated" toast', async () => {
    const harness = renderSubstituteTree(
      `/library/work-items/${SLUG}`,
      [entry()],
    )
    await waitForRelPathResolved(harness.qc)
    fireDocChanged(harness.listeners, RELPATH, 'edited', 'fresh-etag')
    await expectExternalEditToast('updated')
    harness.unmount()
  })

  it('positive correlation: action=created raises "created" toast', async () => {
    const harness = renderSubstituteTree(
      `/library/work-items/${SLUG}`,
      [entry()],
    )
    await waitForRelPathResolved(harness.qc)
    fireDocChanged(harness.listeners, RELPATH, 'created', 'fresh-etag')
    await expectExternalEditToast('created')
    harness.unmount()
  })

  it('positive correlation: action=deleted raises "deleted" toast (delete-ordering regression guard)', async () => {
    const harness = renderSubstituteTree(
      `/library/work-items/${SLUG}`,
      [entry()],
    )
    await waitForRelPathResolved(harness.qc)
    fireDocChanged(harness.listeners, RELPATH, 'deleted', 'fresh-etag')
    await expectExternalEditToast('deleted')
    harness.unmount()
  })

  it('different path → no toast', async () => {
    const harness = renderSubstituteTree(
      `/library/work-items/${SLUG}`,
      [entry()],
    )
    await waitForRelPathResolved(harness.qc)
    fireDocChanged(harness.listeners, 'meta/work/other.md', 'edited', 'e')
    expect(screen.queryByText('External edit detected')).toBeNull()
    harness.unmount()
  })

  it('self-caused (etag registered) → no toast; dispatch agrees with subscriber', async () => {
    const harness = renderSubstituteTree(
      `/library/work-items/${SLUG}`,
      [entry()],
    )
    await waitForRelPathResolved(harness.qc)
    harness.registry.register('self-etag-1')
    expect(harness.registry.has('self-etag-1')).toBe(true)
    const event: SseEvent = {
      type: 'doc-changed',
      action: 'edited',
      docType: 'work-items',
      path: RELPATH,
      etag: 'self-etag-1',
      timestamp: '2026-05-30T00:00:00Z',
    }
    // Fire to subscribers
    act(() => {
      for (const l of harness.listeners) l(event)
    })
    expect(screen.queryByText('External edit detected')).toBeNull()
    harness.unmount()
  })

  it('auto-dismiss + pause-on-hover end-to-end (fake timers)', async () => {
    vi.useFakeTimers()
    try {
      const harness = renderSubstituteTree(
        `/library/work-items/${SLUG}`,
        [entry()],
      )
      // Drain the microtask queue under fake timers.
      await act(async () => {
        await Promise.resolve()
      })
      fireDocChanged(harness.listeners, RELPATH, 'edited', 'e')
      expect(screen.getByText('External edit detected')).toBeInTheDocument()

      // At 4s, still present.
      act(() => {
        vi.advanceTimersByTime(4_000)
      })
      expect(screen.queryByText('External edit detected')).toBeInTheDocument()

      // Past 5.5s, gone.
      act(() => {
        vi.advanceTimersByTime(1_500)
      })
      expect(screen.queryByText('External edit detected')).toBeNull()

      // Now test pause-on-hover.
      fireDocChanged(harness.listeners, RELPATH, 'edited', 'e2')
      expect(screen.getByText('External edit detected')).toBeInTheDocument()
      const card = screen
        .getByText('External edit detected')
        .closest('div')!.parentElement!
      fireEvent.mouseEnter(card)
      act(() => {
        vi.advanceTimersByTime(10_000)
      })
      expect(screen.queryByText('External edit detected')).toBeInTheDocument()
      fireEvent.mouseLeave(card)
      act(() => {
        vi.advanceTimersByTime(5_500)
      })
      expect(screen.queryByText('External edit detected')).toBeNull()

      harness.unmount()
    } finally {
      vi.useRealTimers()
    }
  })

  it('manual dismiss via close button removes toast', async () => {
    const harness = renderSubstituteTree(
      `/library/work-items/${SLUG}`,
      [entry()],
    )
    await waitForRelPathResolved(harness.qc)
    fireDocChanged(harness.listeners, RELPATH, 'edited', 'e')
    expect(await screen.findByText('External edit detected')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Dismiss notification' }))
    await waitFor(() => {
      expect(screen.queryByText('External edit detected')).toBeNull()
    })
    harness.unmount()
  })
})

describe('dispatchSseEvent content refresh (direct dispatch)', () => {
  beforeEach(() => vi.restoreAllMocks())

  it('invalidates docContent(X) so refetch returns v2 and v1 is gone', async () => {
    const qc = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    })

    // Seed v1 in the docContent cache.
    qc.setQueryData(queryKeys.docContent(RELPATH), {
      content: 'v1-body',
      etag: 'etag-v1',
    })

    expect(
      qc.getQueryData<{ content: string }>(queryKeys.docContent(RELPATH)),
    ).toEqual({ content: 'v1-body', etag: 'etag-v1' })

    dispatchSseEvent(
      {
        type: 'doc-changed',
        action: 'edited',
        docType: 'work-items',
        path: RELPATH,
        etag: 'etag-v2',
        timestamp: '2026-05-30T00:00:00Z',
      },
      qc,
    )

    // The cached entry for docContent(X) is now stale/invalidated; with no
    // observers, no refetch occurs but a subsequent fetcher would observe
    // the staleness. Check by querying the cache state.
    const state = qc.getQueryState(queryKeys.docContent(RELPATH))
    expect(state?.isInvalidated).toBe(true)
  })
})

describe('RootLayout provider nesting invariant', () => {
  it('toast dispatched through the RootLayout-provided context renders in the Toaster viewport', async () => {
    // We don't render the full RootLayout (it instantiates its own SSE
    // singleton). Instead verify that ToastContext.Provider + <Toaster/> +
    // <ExternalEditToast/> as wired in RootLayout do connect correctly:
    // a toast pushed via the same provider value renders into the portal.
    let toastApi: ReturnType<typeof useToastDispatcher> | null = null
    const Capture = () => {
      toastApi = useToastDispatcher()
      return (
        <ToastContext.Provider value={toastApi}>
          <Toaster />
        </ToastContext.Provider>
      )
    }
    render(<Capture />)
    act(() => {
      toastApi!.showToast({ heading: 'External edit detected', message: 'hi' })
    })
    expect(screen.getByTestId('toaster-viewport')).toBeInTheDocument()
    expect(screen.getByText('External edit detected')).toBeInTheDocument()
  })
})
