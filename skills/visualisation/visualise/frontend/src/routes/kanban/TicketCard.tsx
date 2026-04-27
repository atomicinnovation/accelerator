import { Link } from '@tanstack/react-router'
import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { formatMtime } from '../../api/format'
import { fileSlugFromRelPath } from '../../api/path-utils'
import { parseTicketNumber } from '../../api/ticket'
import type { IndexEntry } from '../../api/types'
import styles from './TicketCard.module.css'

export interface TicketCardProps {
  entry: IndexEntry
  now?: number
}

export function TicketCard({ entry, now }: TicketCardProps) {
  const { attributes, listeners, setNodeRef, transform, transition } = useSortable({
    id: entry.relPath,
  })

  // Strip role="button" so the anchor keeps its native role="link" — the card
  // is still a navigation target and must be discoverable via link navigation.
  // aria-roledescription="sortable" is kept so screen readers announce the
  // drag affordance without misrepresenting the element as a button.
  const { role: _role, ...sortableAttributes } = attributes

  const number = parseTicketNumber(entry.relPath)
  const fmType = entry.frontmatter['type']
  const typeLabel = typeof fmType === 'string' && fmType.length > 0 ? fmType : null
  const fileSlug = fileSlugFromRelPath(entry.relPath)
  const idChip = number !== null
    ? `#${String(number).padStart(4, '0')}`
    : fileSlug

  return (
    <li className={styles.card} data-relpath={entry.relPath}>
      <Link
        ref={setNodeRef}
        to="/library/$type/$fileSlug"
        params={{ type: 'tickets', fileSlug }}
        className={styles.cardLink}
        style={{
          transform: CSS.Transform.toString(transform),
          transition,
        }}
        {...sortableAttributes}
        {...listeners}
      >
        <div className={styles.cardHeader}>
          <span className={number !== null ? styles.cardNumber : styles.cardSlug}>
            {idChip}
          </span>
          <span className={styles.cardMtime}>{formatMtime(entry.mtimeMs, now)}</span>
        </div>
        <p className={styles.cardTitle}>{entry.title}</p>
        {typeLabel !== null && <p className={styles.cardType}>{typeLabel}</p>}
      </Link>
    </li>
  )
}
