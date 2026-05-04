import type { IndexEntry } from './types'
import {
  STATUS_COLUMNS,
  OTHER_COLUMN_KEY,
  type KanbanColumnKey,
  type KanbanGroupKey,
} from './types'

export function parseWorkItemId(relPath: string): number | null {
  const filename = relPath.split('/').at(-1) ?? ''
  const match = filename.match(/^(\d+)-/)
  if (!match) return null
  const n = Number.parseInt(match[1], 10)
  return Number.isFinite(n) ? n : null
}

const KNOWN_KEYS: ReadonlySet<KanbanColumnKey> = new Set(
  STATUS_COLUMNS.map(c => c.key),
)

function statusGroupOf(entry: IndexEntry): KanbanGroupKey {
  if (entry.frontmatterState !== 'parsed') return OTHER_COLUMN_KEY
  const raw = entry.frontmatter['status']
  if (typeof raw !== 'string') return OTHER_COLUMN_KEY
  return KNOWN_KEYS.has(raw as KanbanColumnKey)
    ? (raw as KanbanColumnKey)
    : OTHER_COLUMN_KEY
}

export function groupWorkItemsByStatus(
  entries: IndexEntry[],
): Map<KanbanGroupKey, IndexEntry[]> {
  const groups = new Map<KanbanGroupKey, IndexEntry[]>()
  for (const c of STATUS_COLUMNS) groups.set(c.key, [])
  for (const entry of entries) {
    const key = statusGroupOf(entry)
    let list = groups.get(key)
    if (!list) {
      list = []
      groups.set(key, list)
    }
    list.push(entry)
  }
  for (const list of groups.values()) {
    list.sort((a, b) => {
      if (b.mtimeMs !== a.mtimeMs) return b.mtimeMs - a.mtimeMs
      return a.relPath.localeCompare(b.relPath)
    })
  }
  return groups
}
