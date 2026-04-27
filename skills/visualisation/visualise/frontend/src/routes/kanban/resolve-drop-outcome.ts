import type { Active, Over } from '@dnd-kit/core'
import type { IndexEntry, KanbanColumnKey } from '../../api/types'
import { OTHER_COLUMN_KEY, STATUS_COLUMNS } from '../../api/types'

export type DropOutcome =
  | { kind: 'move'; toStatus: KanbanColumnKey }
  | { kind: 'no-op-same-column' }
  | { kind: 'no-op-other-rejected' }
  | { kind: 'no-op-unknown' }

const VALID_COLUMN_KEYS = new Set<string>(STATUS_COLUMNS.map(c => c.key))

export function resolveDropOutcome(
  active: Active,
  over: Over | null,
  entriesByRelPath: Map<string, IndexEntry>,
): DropOutcome {
  if (over === null) return { kind: 'no-op-unknown' }

  const sourceEntry = entriesByRelPath.get(active.id as string)
  if (!sourceEntry) return { kind: 'no-op-unknown' }

  const overId = over.id as string

  if (overId.startsWith('column:')) {
    const columnKey = overId.slice('column:'.length)
    if (columnKey === OTHER_COLUMN_KEY) return { kind: 'no-op-other-rejected' }
    if (!VALID_COLUMN_KEYS.has(columnKey)) return { kind: 'no-op-unknown' }
    const toStatus = columnKey as KanbanColumnKey
    if (toStatus === String(sourceEntry.frontmatter['status'])) return { kind: 'no-op-same-column' }
    return { kind: 'move', toStatus }
  }

  const targetEntry = entriesByRelPath.get(overId)
  if (!targetEntry) return { kind: 'no-op-unknown' }

  const targetStatus = String(targetEntry.frontmatter['status'])
  if (!VALID_COLUMN_KEYS.has(targetStatus)) return { kind: 'no-op-other-rejected' }

  const sourceStatus = String(sourceEntry.frontmatter['status'])
  if (targetStatus === sourceStatus) return { kind: 'no-op-same-column' }

  return { kind: 'move', toStatus: targetStatus as KanbanColumnKey }
}
