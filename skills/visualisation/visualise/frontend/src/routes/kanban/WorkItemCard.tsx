import { Link } from '@tanstack/react-router'
import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { formatMtime } from '../../api/format'
import { fileSlugFromRelPath } from '../../api/path-utils'
import { formatDocId } from '../library/doc-type-id'
import { PipelineMini } from '../../components/PipelineMini/PipelineMini'
import type { IndexEntry } from '../../api/types'
import styles from './WorkItemCard.module.css'

export interface WorkItemCardProps {
  entry: IndexEntry
  now?: number
}

export function WorkItemCard({ entry, now }: WorkItemCardProps) {
  const { attributes, listeners, setNodeRef, transform, transition } = useSortable({
    id: entry.relPath,
  })

  // Strip role="button" so the anchor keeps its native role="link" — the card
  // is still a navigation target and must be discoverable via link navigation.
  // aria-roledescription="sortable" is kept so screen readers announce the
  // drag affordance without misrepresenting the element as a button.
  const { role: _role, ...sortableAttributes } = attributes

  const fileSlug = fileSlugFromRelPath(entry.relPath)
  const fmKind = entry.frontmatter['kind']
  const kindLabel = typeof fmKind === 'string' && fmKind.length > 0 ? fmKind : null
  const idLabel = entry.workItemId ? formatDocId(entry.workItemId) : null

  return (
    <li className={`${styles.card} ac-kcard`} data-relpath={entry.relPath}>
      <Link
        ref={setNodeRef}
        to="/library/$type/$fileSlug"
        params={{ type: 'work-items', fileSlug }}
        className={styles.cardLink}
        style={{
          transform: CSS.Transform.toString(transform),
          transition,
        }}
        {...sortableAttributes}
        {...listeners}
      >
        <div className={`${styles.cardTop} ac-kcard__top`}>
          {entry.completeness != null && (
            <PipelineMini completeness={entry.completeness} />
          )}
          {idLabel !== null && (
            <span className={`${styles.cardId} ac-kcard__id`}>{idLabel}</span>
          )}
        </div>
        <p className={`${styles.cardTitle} ac-kcard__title`}>{entry.title}</p>
        {kindLabel !== null && (
          <p className={`${styles.cardKind} ac-kcard__kind`}>{kindLabel}</p>
        )}
        <div className={`${styles.cardFoot} ac-kcard__foot`}>
          <span className={`${styles.cardMtime} ac-kcard__mtime`}>
            {formatMtime(entry.mtimeMs, now)}
          </span>
          {entry.linkedCount > 0 && (
            <span className={`${styles.cardLinks} ac-kcard__links`}>
              {entry.linkedCount} linked
            </span>
          )}
        </div>
      </Link>
    </li>
  )
}
