import type { IndexEntry } from './types'
import { fileSlugFromRelPath } from './path-utils'

export interface WikiLinkIndex {
  adrById: Map<number, IndexEntry>
  workItemById: Map<number, IndexEntry>
}

export interface ResolvedWikiLink {
  href: string
  title: string
}

/** Match `[[ADR-NNNN]]` and `[[WORK-ITEM-NNNN]]` only. Bare `[[NNNN]]` is
 *  intentionally excluded so the prefix namespace stays free for
 *  future ID kinds (`[[EPIC-NNNN]]`, etc.). The digit count is bounded
 *  to 1..6 to comfortably exceed any realistic ID space while
 *  preventing pathological inputs from producing values that overflow
 *  `Number.MAX_SAFE_INTEGER`. */
export const WIKI_LINK_PATTERN = /\[\[(ADR|WORK-ITEM)-(\d{1,6})\]\]/g

type WikiLinkKind = 'ADR' | 'WORK-ITEM'

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

/** Parse a work item id from the leading numeric prefix of the
 *  filename, e.g. `0001-foo.md` → 1. Leading zeros are stripped via
 *  `parseInt(_, 10)`. */
function workItemIdOf(entry: IndexEntry): number | null {
  const filename = entry.relPath.split('/').at(-1) ?? ''
  const dash = filename.indexOf('-')
  if (dash <= 0) return null
  return parsePositiveInt(filename.slice(0, dash))
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
 *  `frontmatter.adr_id` *or* the filename prefix; work items are keyed by
 *  the filename's leading numeric prefix. Defensively filters by
 *  `entry.type` so a misuse — accidentally passing plans as work items —
 *  cannot route `[[WORK-ITEM-N]]` to a non-work-item. On duplicate keys the
 *  entry with the lexically-earliest `relPath` wins (deterministic
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
    insertWithEarliestRelPathTieBreak(adrById, id, entry)
  }
  const workItemById = new Map<number, IndexEntry>()
  for (const entry of workItemEntries) {
    if (entry.type !== 'work-items') continue
    const n = workItemIdOf(entry)
    if (n === null) continue
    insertWithEarliestRelPathTieBreak(workItemById, n, entry)
  }
  return { adrById, workItemById }
}

function insertWithEarliestRelPathTieBreak(
  map: Map<number, IndexEntry>,
  key: number,
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
 *  preserved exactly. */
export function resolveWikiLink(
  prefix: WikiLinkKind,
  n: number,
  idx: WikiLinkIndex,
): ResolvedWikiLink | null {
  const entry = prefix === 'ADR' ? idx.adrById.get(n) : idx.workItemById.get(n)
  if (!entry) return null
  const fileSlug = fileSlugFromRelPath(entry.relPath)
  return {
    href: `/library/${entry.type}/${fileSlug}`,
    title: entry.title,
  }
}
