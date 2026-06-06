import { useDroppable } from '@dnd-kit/core'
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable'
import { WorkItemCard } from './WorkItemCard'
import type { IndexEntry } from '../../api/types'
import { OTHER_COLUMN_KEY } from '../../api/types'
import styles from './KanbanColumn.module.css'

export interface KanbanColumnProps {
  columnKey: string
  label: string
  entries: IndexEntry[]
  description?: string
}

export function KanbanColumn({ columnKey, label, entries, description }: KanbanColumnProps) {
  const ids = entries.map(e => e.relPath)
  const count = entries.length
  const headingId = `kanban-col-${columnKey}-heading`
  const itemWord = count === 1 ? 'work item' : 'work items'
  const isOtherColumn = columnKey === OTHER_COLUMN_KEY
  const { setNodeRef, isOver } = useDroppable({
    id: `column:${columnKey}`,
    disabled: isOtherColumn,
  })

  return (
    <section
      ref={setNodeRef}
      className={`${styles.column}${isOver ? ` ${styles.columnOver}` : ''}`}
      aria-labelledby={headingId}
      data-column={columnKey}
    >
      <header className={styles.columnHeader}>
        <h2 id={headingId} className={styles.columnHeading}>
          {label}
        </h2>
        <span className={styles.columnCount} aria-label={`${count} ${itemWord}`}>
          {count}
        </span>
      </header>
      {description && <p className={styles.columnDescription}>{description}</p>}
      <SortableContext items={ids} strategy={verticalListSortingStrategy}>
        {entries.length === 0 ? (
          // Static empty panel matching the prototype. aria-hidden because the
          // column header already announces the item count (single source of
          // truth) — the prose would otherwise be redundant/contradictory. Copy
          // is mechanism-neutral ("Move … here") since keyboard users place
          // cards via Space/arrows, not a pointer-only drop.
          <div className={styles.empty} aria-hidden="true">
            <p className={styles.emptyTitle}>Nothing here</p>
            <p className={styles.emptyBody}>
              Move a work item here to set its status to {label}.
            </p>
          </div>
        ) : (
          <ul className={styles.cardList}>
            {entries.map(entry => (
              <WorkItemCard key={entry.relPath} entry={entry} />
            ))}
          </ul>
        )}
      </SortableContext>
    </section>
  )
}
