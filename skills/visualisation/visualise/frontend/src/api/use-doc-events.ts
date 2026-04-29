import { createContext, useCallback, useContext, useEffect, useRef, useState } from 'react'
import { useQueryClient, type QueryClient } from '@tanstack/react-query'
import { queryKeys, SESSION_STABLE_QUERY_ROOTS } from './query-keys'
import type { SseEvent } from './types'
import { type SelfCauseRegistry, defaultSelfCauseRegistry } from './self-cause'
import {
  ReconnectingEventSource,
  type ConnectionState,
} from './reconnecting-event-source'

export type { ConnectionState } from './reconnecting-event-source'

export type EventSourceFactory = (url: string) => EventSource

export interface DocEventsHandle {
  setDragInProgress(v: boolean): void
  connectionState: ConnectionState
  justReconnected: boolean
}

function queryKeysForEvent(event: SseEvent): ReadonlyArray<readonly unknown[]> {
  if (event.type !== 'doc-changed' && event.type !== 'doc-invalid') return []
  const keys: Array<readonly unknown[]> = [
    queryKeys.docs(event.docType),
    queryKeys.docContent(event.path),
    queryKeys.lifecycle(),
    queryKeys.lifecycleClusterPrefix(),
    queryKeys.relatedPrefix(),
  ]
  if (event.docType === 'tickets') {
    keys.push(queryKeys.kanban())
  }
  return keys
}

/**
 * Pure dispatch: given an SSE event, invalidate the appropriate query
 * caches. Exported so unit tests can exercise the invalidation logic
 * without any hook / EventSource / async machinery.
 *
 * `event.path` matches the `relPath` used by `fetchDocContent`, so
 * invalidating `docContent(event.path)` refreshes the rendered markdown
 * body when the currently-open detail view's file changes on disk.
 *
 * When `registry` is supplied, `doc-changed` events whose etag matches a
 * locally-registered mutation are silently dropped (self-cause filter).
 */
export function dispatchSseEvent(
  event: SseEvent,
  queryClient: QueryClient,
  registry?: SelfCauseRegistry,
): void {
  if (event.type === 'doc-changed' && registry?.has(event.etag)) {
    return
  }
  if (event.type === 'doc-changed' || event.type === 'doc-invalid') {
    void queryClient.invalidateQueries({ queryKey: queryKeys.docs(event.docType) })
    void queryClient.invalidateQueries({ queryKey: queryKeys.docContent(event.path) })
    void queryClient.invalidateQueries({ queryKey: queryKeys.lifecycle() })
    void queryClient.invalidateQueries({ queryKey: queryKeys.lifecycleClusterPrefix() })
    // Prefix-invalidate the related namespace. The set of related-of
    // pages that depend on a given doc is unbounded (every doc whose
    // cluster contains it; every plan if it's a review's target;
    // transitively…), and the lists are tiny — prefix-invalidate is
    // the simplest correct behaviour. `refetchType: 'all'` revalidates
    // unmounted-but-cached queries too, so navigating to a target
    // plan after deleting one of its reviews shows fresh data on
    // mount rather than serving stale cached data until the next
    // event.
    void queryClient.invalidateQueries({
      queryKey: queryKeys.relatedPrefix(),
      refetchType: 'all',
    })
    if (event.docType === 'tickets') {
      void queryClient.invalidateQueries({ queryKey: queryKeys.kanban() })
    }
  }
}

/**
 * Build a `useDocEvents` hook bound to a specific EventSource factory and
 * self-cause registry. Production wires the real `EventSource` once (see
 * `useDocEvents` below); tests construct isolated hooks with fake factories
 * so they can capture the EventSource instance via a test-local closure.
 */
export function makeUseDocEvents(
  createSource: EventSourceFactory,
  registry: SelfCauseRegistry = defaultSelfCauseRegistry,
) {
  return function useDocEvents(): DocEventsHandle {
    const queryClient = useQueryClient()
    const isDraggingRef = useRef(false)
    const pendingRef = useRef(new Set<string>())
    const [connectionState, setConnectionState] = useState<ConnectionState>('connecting')
    const [justReconnected, setJustReconnected] = useState(false)

    const setDragInProgress = useCallback(
      (v: boolean) => {
        isDraggingRef.current = v
        if (!v) {
          for (const qkStr of pendingRef.current) {
            void queryClient.invalidateQueries({ queryKey: JSON.parse(qkStr) as unknown[] })
          }
          pendingRef.current.clear()
        }
      },
      [queryClient],
    )

    useEffect(() => {
      let reconnectedTimer: ReturnType<typeof setTimeout> | null = null
      const reconnecting = new ReconnectingEventSource('/api/events', {
        factory: createSource,
        onStateChange: setConnectionState,
        onerror: () => {
          console.warn('useDocEvents: SSE error — reconnecting')
        },
        onReconnect: () => {
          registry.reset()
          for (const qkStr of pendingRef.current) {
            void queryClient.invalidateQueries({ queryKey: JSON.parse(qkStr) as unknown[] })
          }
          pendingRef.current.clear()
          void queryClient.invalidateQueries({
            predicate: (q) => !SESSION_STABLE_QUERY_ROOTS.has(q.queryKey[0]),
          })
          if (reconnectedTimer !== null) clearTimeout(reconnectedTimer)
          setJustReconnected(true)
          reconnectedTimer = setTimeout(() => {
            setJustReconnected(false)
            reconnectedTimer = null
          }, 3_000)
        },
      })

      reconnecting.onmessage = (e: MessageEvent) => {
        try {
          const event = JSON.parse(e.data as string) as SseEvent
          if (event.type === 'doc-changed' && registry.has(event.etag)) return
          if (isDraggingRef.current) {
            for (const k of queryKeysForEvent(event)) {
              pendingRef.current.add(JSON.stringify(k))
            }
          } else {
            dispatchSseEvent(event, queryClient)
          }
        } catch (err) {
          console.warn('useDocEvents: failed to parse SSE message', { data: e.data, err })
        }
      }

      return () => {
        if (reconnectedTimer !== null) clearTimeout(reconnectedTimer)
        reconnecting.close()
      }
    }, [queryClient, createSource, registry])

    return { setDragInProgress, connectionState, justReconnected }
  }
}

/** Production hook. Wired once at module load. */
export const useDocEvents = makeUseDocEvents((url) => new EventSource(url))

const _defaultHandle: DocEventsHandle = {
  setDragInProgress: () => {},
  connectionState: 'connecting',
  justReconnected: false,
}

export const DocEventsContext = createContext<DocEventsHandle>(_defaultHandle)

export function useDocEventsContext(): DocEventsHandle {
  return useContext(DocEventsContext)
}
