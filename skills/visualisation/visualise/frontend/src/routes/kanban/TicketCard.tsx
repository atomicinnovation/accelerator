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

const PHASE_7_DISABLED = true

export function TicketCard({ entry, now }: TicketCardProps) {
  const { attributes, listeners, setNodeRef, transform, transition } = useSortable({
    id: entry.relPath,
    disabled: PHASE_7_DISABLED,
  })

  // Strip role="button" and aria-roledescription="sortable" while drag is
  // disabled — the anchor's native role="link" must be preserved, and ARIA
  // must not announce a drag affordance that doesn't exist. Phase 8 re-adds
  // both when PHASE_7_DISABLED flips to false.
  const { 'aria-roledescription': _ariaRoleDescription, role: _role, ...phase7Attributes } = attributes

  const number = parseTicketNumber(entry.relPath)
  const fmType = entry.frontmatter['type']
  const typeLabel = typeof fmType === 'string' && fmType.length > 0 ? fmType : null
  const fileSlug = fileSlugFromRelPath(entry.relPath)
  const idChip = number !== null
    ? `#${String(number).padStart(4, '0')}`
    : fileSlug

  return (
    <li className={styles.card}>
      <Link
        ref={setNodeRef}
        to="/library/$type/$fileSlug"
        params={{ type: 'tickets', fileSlug }}
        className={styles.cardLink}
        style={{
          transform: CSS.Transform.toString(transform),
          transition,
        }}
        {...phase7Attributes}
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
