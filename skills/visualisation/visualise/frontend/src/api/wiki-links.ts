import type { IndexEntry } from './types'
import { fileSlugFromRelPath } from './path-utils'

export interface WikiLinkIndex {
  adrById: Map<number, IndexEntry>
  workItemById: Map<string, IndexEntry>
}

export interface ResolvedWikiLink {
  href: string
  title: string
}

type WikiLinkKind = 'ADR' | 'WORK-ITEM'

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

/** Build the wiki-link match pattern from the server-supplied project code
 *  (or `null` when no project code is configured / default pattern).
 *
 *  - Default (`null`): matches `[[ADR-NNNN]]` and `[[WORK-ITEM-NNNN]]`.
 *  - Project-prefixed: also matches `[[WORK-ITEM-<PROJECT>-NNNN]]` via the
 *    `<PROJECT>-\d+` alternative, with bare-numeric `\d+` as fallback for
 *    legacy files. Multi-segment project codes (e.g. `ACME-CORE`) are out of
 *    scope: the compiler grammar forbids hyphens in project codes. */
export function buildWikiLinkPattern(projectCode: string | null): RegExp {
  const innerWorkItem = projectCode
    ? `${escapeRegExp(projectCode)}-\\d+|\\d+`
    : `\\d+`
  return new RegExp(`\\[\\[(ADR|WORK-ITEM)-(${innerWorkItem})\\]\\]`, 'g')
}

/** Parse an ADR id that may live on `frontmatter.adr_id` (e.g. `"ADR-0017"`)
 *  or in the filename prefix (`ADR-0017-foo.md`). Frontmatter wins over
 *  filename — mirrors the server's `parse_adr_id` precedence. */
function adrIdOf(entry: IndexEntry): number | null {
  const fmId = entry.frontmatter['adr_id']
  if (typeof fmId === 'string') {
    const rest = fmId.startsWith('ADR-') ? fmId.slice(4) : fmId
    const n = parsePositiveInt(rest)
    if (n !== null) return n
  }
  const filename = entry.relPath.split('/').at(-1) ?? ''
  if (!filename.startsWith('ADR-')) return null
  const rest = filename.slice(4)
  const dash = rest.indexOf('-')
  if (dash < 0) return null
  return parsePositiveInt(rest.slice(0, dash))
}

/** Single helper for all numeric extraction in this module. Explicit
 *  radix 10 (no octal/hex inference); rejects leading-`+`, leading-`-`,
 *  empty, and trailing-non-digits. */
function parsePositiveInt(s: string): number | null {
  if (!/^\d+$/.test(s)) return null
  const n = parseInt(s, 10)
  return Number.isFinite(n) && n >= 0 ? n : null
}

/** Build the resolver maps from the docs caches. ADRs are keyed by
 *  `frontmatter.adr_id` *or* the filename prefix. Work items are keyed by
 *  `entry.workItemId` (filename-derived, supplied by the server). Defensively
 *  filters by `entry.type` so a misuse — accidentally passing plans as work
 *  items — cannot route `[[WORK-ITEM-N]]` to a non-work-item. On duplicate
 *  keys the entry with the lexically-earliest `relPath` wins (deterministic
 *  across reloads). */
export function buildWikiLinkIndex(
  adrEntries: IndexEntry[],
  workItemEntries: IndexEntry[],
): WikiLinkIndex {
  const adrById = new Map<number, IndexEntry>()
  for (const entry of adrEntries) {
    if (entry.type !== 'decisions') continue
    const id = adrIdOf(entry)
    if (id === null) continue
    insertWithEarliestRelPath(adrById, id, entry)
  }
  const workItemById = new Map<string, IndexEntry>()
  for (const entry of workItemEntries) {
    if (entry.type !== 'work-items') continue
    const id = entry.workItemId
    if (id === null) continue
    insertWithEarliestRelPath(workItemById, id, entry)
  }
  return { adrById, workItemById }
}

function insertWithEarliestRelPath<K>(
  map: Map<K, IndexEntry>,
  key: K,
  candidate: IndexEntry,
): void {
  const existing = map.get(key)
  if (!existing || candidate.relPath < existing.relPath) {
    map.set(key, candidate)
  }
}

/** Resolve one wiki-link target. Returns `{ href, title }` on hit,
 *  `null` on miss. The href follows the existing
 *  `/library/:type/:fileSlug` shape; `fileSlug` is derived from the
 *  entry's `relPath` so the filename's date or numeric prefix is
 *  preserved exactly.
 *
 *  `id` is the raw string captured from the wiki-link pattern (group 2):
 *  e.g. `"0017"` for `[[ADR-0017]]`, `"PROJ-0042"` for
 *  `[[WORK-ITEM-PROJ-0042]]`, `"0042"` for `[[WORK-ITEM-0042]]`. */
export function resolveWikiLink(
  prefix: WikiLinkKind,
  id: string,
  idx: WikiLinkIndex,
): ResolvedWikiLink | null {
  let entry: IndexEntry | undefined
  if (prefix === 'ADR') {
    const n = parsePositiveInt(id)
    if (n === null) return null
    entry = idx.adrById.get(n)
  } else {
    entry = idx.workItemById.get(id)
  }
  if (!entry) return null
  const fileSlug = fileSlugFromRelPath(entry.relPath)
  return {
    href: `/library/${entry.type}/${fileSlug}`,
    title: entry.title,
  }
}
