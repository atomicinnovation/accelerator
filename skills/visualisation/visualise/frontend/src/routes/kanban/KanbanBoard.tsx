import { useEffect, useMemo, useRef, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  DndContext, PointerSensor, KeyboardSensor,
  useSensor, useSensors, closestCorners,
  type DragEndEvent,
} from '@dnd-kit/core'
import { sortableKeyboardCoordinates } from '@dnd-kit/sortable'
import { fetchDocs, FetchError, ConflictError } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import { groupWorkItemsByStatus } from '../../api/work-item'
import { STATUS_COLUMNS, OTHER_COLUMN, OTHER_COLUMN_KEY } from '../../api/types'
import type { IndexEntry } from '../../api/types'
import { useDocEventsContext } from '../../api/use-doc-events'
import { useMoveWorkItem } from '../../api/use-move-work-item'
import { resolveDropOutcome } from './resolve-drop-outcome'
import { buildKanbanAnnouncements } from './announcements'
import { KanbanColumn } from './KanbanColumn'
import styles from './KanbanBoard.module.css'

const OTHER_DESCRIPTION =
  'Work items whose status is missing or not one of: todo, in-progress, done.'

function errorMessageFor(error: unknown): string {
  if (error instanceof FetchError && error.status >= 500) {
    return 'The visualiser server returned an error. Try again in a moment.'
  }
  return 'Something went wrong loading the work items.'
}

function conflictMessageFor(error: unknown): string {
  if (error instanceof ConflictError) {
    return 'This work item was updated by another editor. Your change was not saved — the card has been returned to its original column.'
  }
  if (error instanceof FetchError) {
    return 'The work item could not be saved. Try again in a moment.'
  }
  return 'An unexpected error occurred while saving. Try again.'
}

export function KanbanBoard() {
  const queryClient = useQueryClient()
  const [conflict, setConflict] = useState<string | null>(null)
  const [announcement, setAnnouncement] = useState('')
  const conflictTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const announcementTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  )

  const { data: entries = [], isPending, isError, error } = useQuery({
    queryKey: queryKeys.docs('work-items'),
    queryFn: () => fetchDocs('work-items'),
  })

  const groups = useMemo(() => groupWorkItemsByStatus(entries), [entries])
  const otherEntries = groups.get(OTHER_COLUMN_KEY) ?? []
  const entriesByRelPath = useMemo(
    () => new Map<string, IndexEntry>(entries.map(e => [e.relPath, e])),
    [entries],
  )

  const entriesRef = useRef(entriesByRelPath)
  useEffect(() => { entriesRef.current = entriesByRelPath }, [entriesByRelPath])

  const announcements = useMemo(
    () => buildKanbanAnnouncements({ entries: () => entriesRef.current }),
    [],
  )

  const docEvents = useDocEventsContext()
  const move = useMoveWorkItem()

  function showConflict(msg: string) {
    setConflict(msg)
    if (conflictTimerRef.current !== null) clearTimeout(conflictTimerRef.current)
    conflictTimerRef.current = setTimeout(() => setConflict(null), 30_000)
  }

  function showAnnouncement(msg: string) {
    setAnnouncement(msg)
    if (announcementTimerRef.current !== null) clearTimeout(announcementTimerRef.current)
    announcementTimerRef.current = setTimeout(() => setAnnouncement(''), 15_000)
  }

  function handleDragStart() {
    docEvents.setDragInProgress(true)
    if (conflictTimerRef.current !== null) { clearTimeout(conflictTimerRef.current); conflictTimerRef.current = null }
    setConflict(null)
    if (announcementTimerRef.current !== null) { clearTimeout(announcementTimerRef.current); announcementTimerRef.current = null }
    setAnnouncement('')
  }

  function handleDragEnd({ active, over }: DragEndEvent) {
    docEvents.setDragInProgress(false)
    const outcome = resolveDropOutcome(active, over, entriesByRelPath)
    const cardId = active.id as string
    const source = entriesByRelPath.get(cardId)

    switch (outcome.kind) {
      case 'move':
        move.mutate(
          { entry: source!, toStatus: outcome.toStatus },
          {
            onError: (err) => {
              showConflict(conflictMessageFor(err))
              requestAnimationFrame(() =>
                document.querySelector<HTMLElement>(
                  `[data-relpath="${CSS.escape(cardId)}"]`,
                )?.focus()
              )
            },
            onSuccess: () => setConflict(null),
          },
        )
        return
      case 'no-op-other-rejected':
        showAnnouncement('The Other column is read-only; drops are ignored.')
        return
      case 'no-op-same-column': {
        const sourceStatus = String(source?.frontmatter['status'] ?? '')
        const columnLabel = STATUS_COLUMNS.find(c => c.key === sourceStatus)?.label ?? sourceStatus
        showAnnouncement(`Card returned to ${columnLabel}.`)
        return
      }
      case 'no-op-unknown':
        return
    }
  }

  if (isPending) {
    return (
      <div className={styles.board}>
        <h1 className={styles.title}>Kanban</h1>
        <p role="status" className={styles.status}>Loading…</p>
      </div>
    )
  }

  if (isError) {
    return (
      <div className={styles.board}>
        <h1 className={styles.title}>Kanban</h1>
        <div role="alert" className={styles.alert}>
          <p className={styles.alertMessage}>{errorMessageFor(error)}</p>
          <button
            type="button"
            className={styles.retry}
            onClick={() => {
              queryClient.invalidateQueries({ queryKey: queryKeys.docs('work-items') })
            }}
          >
            Retry
          </button>
        </div>
      </div>
    )
  }

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCorners}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
      accessibility={{ announcements }}
    >
      <div className={styles.board} data-sse-state={docEvents.connectionState}>
        <h1 className={styles.title}>Kanban</h1>
        {conflict !== null && (
          <div role="alert" aria-atomic="true" className={styles.conflictBanner}>
            <span className={styles.conflictMessage}>{conflict}</span>
            <button
              type="button"
              aria-label="Dismiss conflict notice"
              className={styles.conflictDismiss}
              onClick={() => setConflict(null)}
            >
              ×
            </button>
          </div>
        )}
        <div role="status" aria-live="polite" className={styles.announcement}>
          {announcement}
        </div>
        <div className={styles.columns}>
          {STATUS_COLUMNS.map(col => (
            <KanbanColumn
              key={col.key}
              columnKey={col.key}
              label={col.label}
              entries={groups.get(col.key) ?? []}
            />
          ))}
        </div>
        {otherEntries.length > 0 && (
          <div className={styles.otherSwimlane}>
            <KanbanColumn
              columnKey={OTHER_COLUMN.key}
              label={OTHER_COLUMN.label}
              entries={otherEntries}
              description={OTHER_DESCRIPTION}
            />
          </div>
        )}
      </div>
    </DndContext>
  )
}
