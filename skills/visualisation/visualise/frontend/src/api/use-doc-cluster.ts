import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { fetchLifecycleClusters } from './fetch'
import { queryKeys } from './query-keys'
import type { IndexEntry, LifecycleCluster } from './types'

/** Resolves the lifecycle cluster a document belongs to by fetching the
 *  full cluster list and matching on `path`.
 *
 *  Membership is matched by `path` — NOT by a per-cluster
 *  `fetchLifecycleCluster(slug)` / `queryKeys.lifecycleCluster(slug)` — because
 *  a cluster's `slug` is a server-chosen *representative* slug that is not
 *  reliably derivable from an arbitrary member entry. The list-plus-path-match
 *  is the robust path and also shares the `queryKeys.lifecycle()` cache with
 *  `LifecycleIndex`.
 *
 *  Returns the bare `UseQueryResult` spread plus the derived `cluster`, matching
 *  the sibling read-side hooks (`useRelated` / `useDocContent`). Spreading the
 *  query keeps `.isPending` / `.isError` / `.data` / `.isFetching` / `.refetch`
 *  available so the caller can distinguish loading / error from "genuinely no
 *  cluster" — all three otherwise collapse to `cluster === null`. */
export function useDocCluster(entry: IndexEntry | undefined) {
  const query = useQuery({
    queryKey: queryKeys.lifecycle(),
    queryFn: fetchLifecycleClusters,
    enabled: !!entry,
  })
  const cluster = useMemo<LifecycleCluster | null>(
    () =>
      entry
        ? (query.data?.find((c) => c.entries.some((e) => e.path === entry.path)) ??
          null)
        : null,
    [query.data, entry?.path],
  )
  return { ...query, cluster }
}
