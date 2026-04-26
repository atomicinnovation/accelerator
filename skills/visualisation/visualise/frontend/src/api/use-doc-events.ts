import { useEffect } from 'react'
import { useQueryClient, type QueryClient } from '@tanstack/react-query'
import { queryKeys } from './query-keys'
import type { SseEvent } from './types'

export type EventSourceFactory = (url: string) => EventSource

/**
 * Pure dispatch: given an SSE event, invalidate the appropriate query
 * caches. Exported so unit tests can exercise the invalidation logic
 * without any hook / EventSource / async machinery.
 *
 * `event.path` matches the `relPath` used by `fetchDocContent`, so
 * invalidating `docContent(event.path)` refreshes the rendered markdown
 * body when the currently-open detail view's file changes on disk.
 */
export function dispatchSseEvent(
  event: SseEvent,
  queryClient: QueryClient,
): void {
  if (event.type === 'doc-changed' || event.type === 'doc-invalid') {
    void queryClient.invalidateQueries({ queryKey: queryKeys.docs(event.docType) })
    void queryClient.invalidateQueries({ queryKey: queryKeys.docContent(event.path) })
    void queryClient.invalidateQueries({ queryKey: queryKeys.lifecycle() })
    void queryClient.invalidateQueries({ queryKey: queryKeys.lifecycleClusterPrefix() })
    if (event.docType === 'tickets') {
      void queryClient.invalidateQueries({ queryKey: queryKeys.kanban() })
    }
  }
}

/**
 * Build a `useDocEvents` hook bound to a specific EventSource factory.
 * Production wires the real `EventSource` once (see `useDocEvents` below);
 * tests construct isolated hooks with fake factories so they can capture
 * the EventSource instance via a test-local closure — no global stub
 * coordination required.
 */
export function makeUseDocEvents(createSource: EventSourceFactory) {
  return function useDocEvents(): void {
    const queryClient = useQueryClient()

    useEffect(() => {
      const source = createSource('/api/events')

      source.onmessage = (e: MessageEvent) => {
        try {
          const event = JSON.parse(e.data as string) as SseEvent
          dispatchSseEvent(event, queryClient)
        } catch (err) {
          // Malformed SSE data — don't crash the hook, but surface the
          // problem so server/client schema drift is debuggable.
          console.warn('useDocEvents: failed to parse SSE message', { data: e.data, err })
        }
      }

      // Native EventSource auto-reconnects on network errors; events
      // fired during the outage are lost. Invalidate the top-level docs
      // prefix so reconnecting clients refetch and reconcile. Full
      // reconnect UX (banner + backoff) is Phase 10.
      source.onerror = () => {
        console.warn('useDocEvents: EventSource error — invalidating docs cache')
        void queryClient.invalidateQueries({ queryKey: ['docs'] })
      }

      return () => source.close()
    }, [queryClient])
  }
}

/** Production hook. Wired once at module load. */
export const useDocEvents = makeUseDocEvents((url) => new EventSource(url))
