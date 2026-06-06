import { formatMtime } from '../../api/format'
import { formatDocId } from '../library/doc-type-id'
import { PipelineMini } from '../../components/PipelineMini/PipelineMini'
import type { IndexEntry } from '../../api/types'
import styles from './WorkItemCard.module.css'

export interface WorkItemCardPresentationProps {
  entry: IndexEntry
  now?: number
  /** Render as the lifted DragOverlay clone — 0.6 opacity, `grabbing` cursor. */
  overlay?: boolean
  /** Render the source card's dragging state — rotate/scale lift, accent border. */
  dragging?: boolean
}

/**
 * The shared inner visual layout of a kanban card, rendered purely from `entry`
 * plus presentation flags — NO sortable wiring and NO navigation. `WorkItemCard`
 * wraps it in the sortable `<li>`/`<Link>`; the board's `<DragOverlay>` renders
 * it directly (`overlay`) as the lifted clone. Keeping the visuals here means
 * each entry point stays single-responsibility and the `useSortable` hook in
 * `WorkItemCard` is never conditionally bypassed (no rules-of-hooks hazard).
 */
export function WorkItemCardPresentation({
  entry,
  now,
  overlay = false,
  dragging = false,
}: WorkItemCardPresentationProps) {
  const fmKind = entry.frontmatter['kind']
  const kindLabel = typeof fmKind === 'string' && fmKind.length > 0 ? fmKind : null
  const idLabel = entry.workItemId ? formatDocId(entry.workItemId) : null

  const className = [
    styles.card,
    'ac-kcard',
    dragging ? styles.cardDragging : '',
    overlay ? styles.cardOverlay : '',
  ]
    .filter(Boolean)
    .join(' ')

  return (
    <div
      className={className}
      data-dragging={dragging ? '' : undefined}
      data-overlay={overlay ? '' : undefined}
    >
      <div className={styles.cardBody}>
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
      </div>
    </div>
  )
}
