import type { ReactNode } from 'react'
import { useEffect, useMemo, useRef, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  DndContext, DragOverlay, PointerSensor, KeyboardSensor,
  useSensor, useSensors, closestCorners,
  type DragEndEvent, type DragStartEvent,
} from '@dnd-kit/core'
import { sortableKeyboardCoordinates } from '@dnd-kit/sortable'
import { fetchDocs, FetchError } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import { groupWorkItemsByStatus } from '../../api/work-item'
import { useKanbanConfig } from '../../api/use-kanban-config'
import { OTHER_COLUMN, OTHER_COLUMN_KEY } from '../../api/types'
import type { IndexEntry } from '../../api/types'
import { useDocEventsContext } from '../../api/use-doc-events'
import { useMoveWorkItem } from '../../api/use-move-work-item'
import { useToast } from '../../api/use-toast'
import { resolveDropOutcome } from './resolve-drop-outcome'
import { buildKanbanAnnouncements } from './announcements'
import { moveToastFor } from './move-toast'
import { KanbanColumn } from './KanbanColumn'
import { WorkItemCardPresentation } from './WorkItemCardPresentation'
import { KanbanFocusContext, createKanbanFocusRegistry } from './kanban-focus-registry'
import { Page } from '../../components/Page/Page'
import { Chip } from '../../components/Chip/Chip'
import styles from './KanbanBoard.module.css'

function errorMessageFor(error: unknown): string {
  if (error instanceof FetchError && error.status >= 500) {
    return 'The visualiser server returned an error. Try again in a moment.'
  }
  return 'Something went wrong loading the work items.'
}

export function KanbanBoard() {
  const queryClient = useQueryClient()
  const [announcement, setAnnouncement] = useState('')
  const [activeId, setActiveId] = useState<string | null>(null)
  const announcementTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // C3 focus contract: cards register their anchor here by relPath; the board
  // focuses the resting card after a move settles.
  const anchorsRef = useRef(new Map<string, HTMLElement>())
  const focusRegistry = useMemo(
    () => createKanbanFocusRegistry(anchorsRef.current),
    [],
  )
  // Single-use token armed on settle (NOT on entries identity) carrying the
  // moved card's relPath. Armed only after success-commit / error-revert have
  // applied, so it is inert for the optimistic render and any unrelated SSE
  // refetch, and fires exactly once on the post-settle render.
  const pendingFocusRef = useRef<string | null>(null)

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  )

  const { data: entries = [], isPending: entriesPending, isError, error } = useQuery({
    queryKey: queryKeys.docs('work-items'),
    queryFn: () => fetchDocs('work-items'),
  })

  const { data: kanbanConfig, isPending: configPending } = useKanbanConfig()
  const columns = kanbanConfig?.columns ?? []

  const validColumnKeys = useMemo(() => new Set(columns.map(c => c.key)), [columns])
  const groups = useMemo(() => groupWorkItemsByStatus(entries, columns), [entries, columns])
  const otherEntries = groups.get(OTHER_COLUMN_KEY) ?? []
  const entriesByRelPath = useMemo(
    () => new Map<string, IndexEntry>(entries.map(e => [e.relPath, e])),
    [entries],
  )

  const entriesRef = useRef(entriesByRelPath)
  useEffect(() => { entriesRef.current = entriesByRelPath }, [entriesByRelPath])

  const columnsRef = useRef(columns)
  useEffect(() => { columnsRef.current = columns }, [columns])

  const announcements = useMemo(
    () => buildKanbanAnnouncements({ entries: () => entriesRef.current, columns: () => columnsRef.current }),
    [],
  )

  const docEvents = useDocEventsContext()
  const move = useMoveWorkItem()
  const { showToast } = useToast()

  // Declarative consumer of the on-settle focus token: when the entries list
  // next renders with a pending focus armed, focus the registered anchor (live
  // after any refetch remount) and clear the token — single-fire. Inert for the
  // optimistic render (token not yet armed) and for unrelated SSE refetches
  // (token null), so it never re-applies focus. No rAF poll loop.
  useEffect(() => {
    const relPath = pendingFocusRef.current
    if (relPath === null) return
    if (focusRegistry.focus(relPath)) {
      pendingFocusRef.current = null
    }
  }, [entries, focusRegistry])

  function showAnnouncement(msg: string) {
    setAnnouncement(msg)
    if (announcementTimerRef.current !== null) clearTimeout(announcementTimerRef.current)
    announcementTimerRef.current = setTimeout(() => setAnnouncement(''), 15_000)
  }

  function handleDragStart({ active }: DragStartEvent) {
    docEvents.setDragInProgress(true)
    setActiveId(active.id as string)
    if (announcementTimerRef.current !== null) { clearTimeout(announcementTimerRef.current); announcementTimerRef.current = null }
    setAnnouncement('')
  }

  // Single teardown for the gate-clearing invariant, shared by drop and cancel.
  // setDragInProgress(false) runs FIRST so it drains any SSE invalidations queued
  // mid-drag BEFORE the optimistic onMutate write (handleDragEnd dispatches
  // move.mutate only after this), and clearing activeId tears down the overlay.
  // dnd-kit calls onDragCancel INSTEAD OF onDragEnd, so without this the gate
  // would stay true and silently freeze live board updates for the session.
  function endDrag() {
    docEvents.setDragInProgress(false)
    setActiveId(null)
  }

  function handleDragEnd({ active, over }: DragEndEvent) {
    endDrag()
    const outcome = resolveDropOutcome(active, over, entriesByRelPath, validColumnKeys)
    const cardId = active.id as string
    const source = entriesByRelPath.get(cardId)

    switch (outcome.kind) {
      case 'move': {
        // Guard the source entry (matches the overlay null-guard convention): if
        // the entry was deleted mid-drag, treat the drop as a no-op rather than
        // asserting non-null.
        if (!source) return
        // toStatus is a valid configured column key by construction
        // (resolveDropOutcome only returns 'move' for keys in validColumnKeys),
        // so its label always resolves — never fall back to the raw key.
        const targetLabel = columns.find(c => c.key === outcome.toStatus)?.label
        if (targetLabel === undefined) return
        move.mutate(
          { entry: source, toStatus: outcome.toStatus },
          {
            onSuccess: () => {
              const toast = moveToastFor(outcome, source, targetLabel, { ok: true })
              if (toast) showToast(toast)
            },
            onError: (err) => {
              // Revert is owned by useMoveWorkItem's onError; the board only
              // surfaces the assertive, persistent error toast.
              const toast = moveToastFor(outcome, source, targetLabel, { ok: false, error: err })
              if (toast) showToast(toast)
            },
            // Arm focus restoration AFTER the success-commit / error-revert has
            // applied and the invalidation refetch is in flight. relPath is
            // stable across the move, so it resolves to the card in its final
            // resting column (target on success, source on revert).
            onSettled: () => {
              pendingFocusRef.current = cardId
            },
          },
        )
        return
      }
      case 'no-op-other-rejected':
        showAnnouncement('The Other column is read-only; drops are ignored.')
        return
      case 'no-op-same-column': {
        const sourceStatus = String(source?.frontmatter['status'] ?? '')
        const columnLabel = columns.find(c => c.key === sourceStatus)?.label ?? sourceStatus
        showAnnouncement(`Card returned to ${columnLabel}.`)
        return
      }
      case 'no-op-unknown':
        return
    }
  }

  let content: ReactNode
  let isSuccess = false
  if (entriesPending || configPending) {
    content = (
      <p role="status" className={styles.status}>Loading…</p>
    )
  } else if (isError) {
    content = (
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
    )
  } else {
    isSuccess = true
    const otherDescription =
      columns.length > 0
        ? `Work items whose status is missing or not one of: ${columns.map(c => c.key).join(', ')}.`
        : 'Work items whose status is missing or does not match any configured column.'
    content = (
      <KanbanFocusContext.Provider value={focusRegistry}>
      <DndContext
        sensors={sensors}
        collisionDetection={closestCorners}
        onDragStart={handleDragStart}
        onDragEnd={handleDragEnd}
        onDragCancel={endDrag}
        accessibility={{ announcements }}
      >
        <div className={styles.board} data-sse-state={docEvents.connectionState}>
          <div role="status" aria-live="polite" className={styles.announcement}>
            {announcement}
          </div>
          <div className={styles.columns}>
            {columns.map(col => (
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
                description={otherDescription}
              />
            </div>
          )}
        </div>
        <DragOverlay>
          {(() => {
            // Real null-check, not `!`: the board is SSE-live, so a concurrent
            // external delete (flushed when endDrag clears the gate) can remove
            // the in-flight entry from the map mid-drag — rendering it as the
            // lifted clone must degrade to nothing rather than crash.
            const active = activeId ? entriesByRelPath.get(activeId) : null
            return active ? (
              <WorkItemCardPresentation entry={active} overlay />
            ) : null
          })()}
        </DragOverlay>
      </DndContext>
      </KanbanFocusContext.Provider>
    )
  }

  return (
    <Page
      title="Kanban"
      actions={isSuccess ? <Chip variant="indigo">live</Chip> : undefined}
    >
      {content}
    </Page>
  )
}
