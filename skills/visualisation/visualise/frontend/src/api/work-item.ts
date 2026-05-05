import type { IndexEntry, KanbanColumn } from './types'
import { OTHER_COLUMN_KEY } from './types'

export function parseWorkItemId(relPath: string): number | null {
  const filename = relPath.split('/').at(-1) ?? ''
  const match = filename.match(/^(\d+)-/)
  if (!match) return null
  const n = Number.parseInt(match[1], 10)
  return Number.isFinite(n) ? n : null
}

export function groupWorkItemsByStatus(
  entries: IndexEntry[],
  columns: ReadonlyArray<KanbanColumn>,
): Map<string, IndexEntry[]> {
  const knownKeys = new Set(columns.map(c => c.key))
  const groups = new Map<string, IndexEntry[]>()
  for (const c of columns) groups.set(c.key, [])
  for (const entry of entries) {
    const raw = entry.frontmatterState === 'parsed'
      ? entry.frontmatter['status']
      : undefined
    const status = typeof raw === 'string' ? raw : null
    const key = status && knownKeys.has(status) ? status : OTHER_COLUMN_KEY
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
