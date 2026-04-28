import { useEffect, useState } from 'react'

interface QueryStatus {
  isFetching: boolean
  isPending: boolean
}

/** Gate the "Updating…" hint on two conditions:
 *  1. The refetch has been in flight for at least `delayMs` (default
 *     ~250ms), so transient sub-second refetches — the common case
 *     for unrelated `doc-changed` invalidations under
 *     `refetchType: 'all'` — never surface the hint.
 *  2. The query is `isFetching && !isPending` — a refresh of
 *     already-rendered data, not the initial load. The initial-load
 *     case is covered by the caller's separate `Loading…` branch. */
export function useDeferredFetchingHint(
  query: QueryStatus,
  delayMs = 250,
): boolean {
  const [show, setShow] = useState(false)
  const isRefetch = query.isFetching && !query.isPending
  useEffect(() => {
    if (!isRefetch) {
      setShow(false)
      return
    }
    const id = setTimeout(() => setShow(true), delayMs)
    return () => clearTimeout(id)
  }, [isRefetch, delayMs])
  return show
}
