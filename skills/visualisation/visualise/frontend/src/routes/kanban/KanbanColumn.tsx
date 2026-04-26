import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable'
import { TicketCard } from './TicketCard'
import type { IndexEntry, KanbanGroupKey } from '../../api/types'
import styles from './KanbanColumn.module.css'

export interface KanbanColumnProps {
  columnKey: KanbanGroupKey
  label: string
  entries: IndexEntry[]
  description?: string
}

export function KanbanColumn({ columnKey, label, entries, description }: KanbanColumnProps) {
  const ids = entries.map(e => e.relPath)
  const count = entries.length
  const headingId = `kanban-col-${columnKey}-heading`
  const ticketWord = count === 1 ? 'ticket' : 'tickets'

  return (
    <section
      className={styles.column}
      aria-labelledby={headingId}
      data-column={columnKey}
    >
      <header className={styles.columnHeader}>
        <h2 id={headingId} className={styles.columnHeading}>
          {label}
        </h2>
        <span className={styles.columnCount} aria-label={`${count} ${ticketWord}`}>
          {count}
        </span>
      </header>
      {description && <p className={styles.columnDescription}>{description}</p>}
      <SortableContext items={ids} strategy={verticalListSortingStrategy}>
        {entries.length === 0 ? (
          <p className={styles.empty} aria-hidden="true">No tickets</p>
        ) : (
          <ul className={styles.cardList}>
            {entries.map(entry => (
              <TicketCard key={entry.relPath} entry={entry} />
            ))}
          </ul>
        )}
      </SortableContext>
    </section>
  )
}
