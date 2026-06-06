import { useEffect, useRef } from 'react'
import type { MouseEvent } from 'react'
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

/**
 * Pure suppress/allow decision for the card's click guard (A2): a click is
 * swallowed iff a drag is currently active OR one just ended (the synthetic
 * post-drag click). The ref toggling and one-tick-late clear timing are the
 * E2E test's job; this captures only the decision.
 */
export function shouldSuppressClick(isDragging: boolean, justDragged: boolean): boolean {
  return isDragging || justDragged
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

  // A2: suppress the synthetic post-drag click so a drag never navigates, while
  // a genuine click still does. Card-local (not a board-passed signal) so the
  // card stays independently unit-testable. This guard is deliberately SEPARATE
  // from the board's setDragInProgress (cleared synchronously, gates SSE) and
  // from activeId — it is the only one whose clear is intentionally one
  // interaction late.
  const draggedRef = useRef(false)

  // Keyed off a real drag having started (the activation threshold crossed /
  // isDragging gone true), NOT a bare press — so a sub-threshold genuine click
  // is never swallowed.
  useEffect(() => {
    if (isDragging) draggedRef.current = true
  }, [isDragging])

  // Cleared on the NEXT interaction's pointer-down — a boundary provably later
  // than the suppressing synthetic click (which fires in the same pointerup
  // task). Preferred over setTimeout(…, 0): a rapid second drag's pointerdown
  // naturally supersedes the prior guard window, with no pending timer that
  // could clear the guard mid-second-drag.
  const handlePointerDownCapture = () => {
    draggedRef.current = false
  }

  const handleClickCapture = (e: MouseEvent) => {
    if (shouldSuppressClick(isDragging, draggedRef.current)) {
      e.preventDefault()
      e.stopPropagation()
    }
  }

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
        onPointerDownCapture={handlePointerDownCapture}
        onClickCapture={handleClickCapture}
        {...sortableAttributes}
        {...listeners}
      >
        <WorkItemCardPresentation entry={entry} now={now} dragging={isDragging} />
      </Link>
    </li>
  )
}
