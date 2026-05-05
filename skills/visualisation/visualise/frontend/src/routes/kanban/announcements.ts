import type { Announcements } from '@dnd-kit/core'
import type { IndexEntry, KanbanColumn } from '../../api/types'
import { OTHER_COLUMN } from '../../api/types'

interface Deps {
  entries: () => Map<string, IndexEntry>
  columns: () => ReadonlyArray<KanbanColumn>
}

export function workItemIdFromRelPath(relPath: string): string | null {
  const m = /(\d{4})-/.exec(relPath.split('/').pop() ?? '')
  return m ? m[1] : null
}

function labelFor(columnId: unknown, columns: ReadonlyArray<KanbanColumn>): string {
  const id = String(columnId)
  return columns.find(c => c.key === id)?.label
    ?? (OTHER_COLUMN.key === id ? OTHER_COLUMN.label : id)
}

function describe(id: unknown, entries: Map<string, IndexEntry>): string {
  if (typeof id !== 'string') return `work item ${String(id)}`
  const entry = entries.get(id)
  const num = workItemIdFromRelPath(id)
  const title = entry?.title
  if (num && title) return `work item ${num}: ${title}`
  if (title) return `work item ${title}`
  return `work item ${id}`
}

export function buildKanbanAnnouncements(
  { entries, columns }: Deps,
): Announcements {
  return {
    onDragStart({ active }): string {
      return `Picked up ${describe(active.id, entries())}.`
    },
    onDragOver({ active, over }): string | undefined {
      if (!over) return undefined
      return `${describe(active.id, entries())} is over ${labelFor(over.id, columns())}.`
    },
    onDragEnd({ active, over }): string {
      if (!over) return `Drop of ${describe(active.id, entries())} cancelled, no target.`
      return `Moved ${describe(active.id, entries())} to ${labelFor(over.id, columns())}.`
    },
    onDragCancel({ active }): string {
      return `Drag of ${describe(active.id, entries())} cancelled.`
    },
  }
}
