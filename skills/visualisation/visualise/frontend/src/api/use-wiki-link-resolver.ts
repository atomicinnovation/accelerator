import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { fetchDocs } from './fetch'
import { queryKeys } from './query-keys'
import {
  buildWikiLinkIndex,
  buildWikiLinkPattern,
  resolveWikiLink,
  type WikiLinkIndex,
} from './wiki-links'
import type {
  Resolver,
  ResolverResult,
} from '../components/MarkdownRenderer/wiki-link-plugin'

interface WorkItemConfig {
  defaultProjectCode?: string | null
}

async function fetchWorkItemConfig(): Promise<WorkItemConfig> {
  const resp = await fetch('/api/work-item/config')
  if (!resp.ok) throw new Error(`/api/work-item/config returned ${resp.status}`)
  return resp.json() as Promise<WorkItemConfig>
}

export interface UseWikiLinkResolverResult {
  resolver: Resolver
  pattern: RegExp
}

/** Combine the ADR + work item caches into a memoised `Resolver` suitable
 *  for `MarkdownRenderer`'s `resolveWikiLink` prop.
 *
 *  Pending semantics: `isPending` (TanStack Query v5) is true *only* on
 *  the initial load when no cached data exists. The resolver returns
 *  `kind: 'pending'` while either docs query is in this state. On
 *  background refetches with cached data, the resolver continues to
 *  serve the previous resolved/unresolved verdicts using the still-
 *  present cached data — see "Refetch staleness (deliberate trade)" in
 *  the Phase 9 plan: defining `isWarming` as `isFetching` would flicker
 *  every wiki-link to `pending` on every unrelated `doc-changed` event,
 *  which is strictly worse UX than the sub-second staleness window. */
export function useWikiLinkResolver(): UseWikiLinkResolverResult {
  const adrs = useQuery({
    queryKey: queryKeys.docs('decisions'),
    queryFn: () => fetchDocs('decisions'),
  })
  const workItems = useQuery({
    queryKey: queryKeys.docs('work-items'),
    queryFn: () => fetchDocs('work-items'),
  })
  const workItemConfig = useQuery({
    queryKey: queryKeys.workItemConfig(),
    queryFn: fetchWorkItemConfig,
    staleTime: Infinity,
  })

  const isWarming = adrs.isPending || workItems.isPending

  const pattern = useMemo<RegExp>(
    () => buildWikiLinkPattern(workItemConfig.data?.defaultProjectCode ?? null),
    [workItemConfig.data?.defaultProjectCode],
  )

  const wikiIndex = useMemo<WikiLinkIndex>(
    () => buildWikiLinkIndex(adrs.data ?? [], workItems.data ?? []),
    [adrs.data, workItems.data],
  )

  const resolver = useMemo<Resolver>(
    () => (prefix, id): ResolverResult => {
      if (isWarming) return { kind: 'pending' }
      const hit = resolveWikiLink(prefix, id, wikiIndex)
      return hit ? { kind: 'resolved', ...hit } : { kind: 'unresolved' }
    },
    [isWarming, wikiIndex],
  )

  return { resolver, pattern }
}
