import { Link } from '@tanstack/react-router'
import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { fileSlugFromRelPath } from '../../api/path-utils'
import { WorkItemCardPresentation } from './WorkItemCardPresentation'
import type { IndexEntry } from '../../api/types'
import styles from './WorkItemCard.module.css'

export interface WorkItemCardProps {
  entry: IndexEntry
  now?: number
}

export function WorkItemCard({ entry, now }: WorkItemCardProps) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id: entry.relPath })

  // Strip role="button" so the anchor keeps its native role="link" — the card
  // is still a navigation target and must be discoverable via link navigation.
  // aria-roledescription="sortable" is kept so screen readers announce the
  // drag affordance without misrepresenting the element as a button.
  const { role: _role, ...sortableAttributes } = attributes

  const fileSlug = fileSlugFromRelPath(entry.relPath)

  return (
    <li className={styles.cardItem} data-relpath={entry.relPath}>
      <Link
        ref={setNodeRef}
        to="/library/$type/$fileSlug"
        params={{ type: 'work-items', fileSlug }}
        className={styles.cardLink}
        // While dragging, the lifted DragOverlay clone follows the cursor and
        // the source card stays in its slot (showing the dragging style), so the
        // sortable translate transform is suppressed for the active card.
        style={{
          transform: isDragging ? undefined : CSS.Transform.toString(transform),
          transition,
        }}
        {...sortableAttributes}
        {...listeners}
      >
        <WorkItemCardPresentation entry={entry} now={now} dragging={isDragging} />
      </Link>
    </li>
  )
}
