import type { DocTypeKey, LibrarySelection, LibrarySelectionPerType } from './types'

/**
 * Produces a canonical form of a `LibrarySelection`:
 *   - drops facets with empty / undefined option arrays;
 *   - drops empty / undefined per-type slices;
 *   - sorts option arrays ascending (since the server treats them as a set);
 *   - sorts facet keys and doc-type keys ascending so the cache key is
 *     order-independent.
 * Returns `{}` when nothing remains. Two semantically-equivalent selections
 * normalise to deep-equal objects, so they hash to identical cache keys.
 */
export function normaliseSelection(
  selection?: LibrarySelection,
): Record<string, LibrarySelectionPerType> {
  if (!selection) return {}
  const docTypeKeys = (Object.keys(selection) as DocTypeKey[]).filter(
    (k) => selection[k] !== undefined,
  )
  const result: Record<string, LibrarySelectionPerType> = {}
  for (const docType of [...docTypeKeys].sort()) {
    const perType = selection[docType]
    if (!perType) continue
    const facetEntries: [string, string[]][] = []
    for (const facetId of Object.keys(perType).sort()) {
      const options = perType[facetId]
      if (!options || options.length === 0) continue
      facetEntries.push([facetId, [...options].sort()])
    }
    if (facetEntries.length === 0) continue
    const sortedPerType: LibrarySelectionPerType = {}
    for (const [facetId, options] of facetEntries) {
      sortedPerType[facetId] = options
    }
    result[docType] = sortedPerType
  }
  return result
}

export const queryKeys = {
  serverInfo: () => ['server-info'] as const,
  workItemConfig: () => ['work-item-config'] as const,
  types: () => ['types'] as const,
  libraryStructure: (selection?: LibrarySelection) =>
    ['library-structure', normaliseSelection(selection)] as const,
  docs: (type: DocTypeKey) => ['docs', type] as const,
  docContent: (relPath: string) => ['doc-content', relPath] as const,
  templates: () => ['templates'] as const,
  templateDetail: (name: string) => ['template-detail', name] as const,
  // v2 marker: cluster representative slugs change after the lifecycle
  // composite-key migration (Phase 4). Bumping the segment forces a
  // cache miss for any developer with a long-running tab open across
  // the deploy, sidestepping the slug-stale window described in the
  // plan's Migration Notes.
  lifecycle: () => ['lifecycle', 'v2'] as const,
  lifecycleClusterPrefix: () => ['lifecycle-cluster', 'v2'] as const,
  lifecycleCluster: (slug: string) => ['lifecycle-cluster', 'v2', slug] as const,
  kanban: () => ['kanban'] as const,
  editor: () => ['editor'] as const,
  related: (relPath: string) => ['related', relPath] as const,
  relatedPrefix: () => ['related'] as const,
  activity: (limit: number) => ['activity', limit] as const,
  search: (q: string) => ['search', q] as const,
  /** Sentinel for queries gated on a still-undefined dependency. The
   *  `__disabled__` token cannot collide with any real value-keyed
   *  query because the real keys take a typed parameter (e.g. a
   *  relPath) the caller had to provide. */
  disabled: (prefix: string) => [prefix, '__disabled__'] as const,
} as const

export const SESSION_STABLE_QUERY_ROOTS: ReadonlySet<unknown> = new Set([
  'server-info',
  'work-item-config',
])
