import { useMemo } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  DndContext, PointerSensor, KeyboardSensor,
  useSensor, useSensors, closestCorners,
} from '@dnd-kit/core'
import { sortableKeyboardCoordinates } from '@dnd-kit/sortable'
import { fetchDocs, FetchError } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import { groupTicketsByStatus } from '../../api/ticket'
import { STATUS_COLUMNS, OTHER_COLUMN, OTHER_COLUMN_KEY } from '../../api/types'
import { KanbanColumn } from './KanbanColumn'
import styles from './KanbanBoard.module.css'

const OTHER_DESCRIPTION =
  'Tickets whose status is missing or not one of: todo, in-progress, done.'

function errorMessageFor(error: unknown): string {
  if (error instanceof FetchError && error.status >= 500) {
    return 'The visualiser server returned an error. Try again in a moment.'
  }
  return 'Something went wrong loading the tickets.'
}

export function KanbanBoard() {
  const queryClient = useQueryClient()

  // Sensors are pre-mounted so Phase 8 can flip TicketCard's
  // PHASE_7_DISABLED to false without re-threading the DndContext.
  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  )

  const { data: entries = [], isPending, isError, error } = useQuery({
    queryKey: queryKeys.docs('tickets'),
    queryFn: () => fetchDocs('tickets'),
  })

  const groups = useMemo(() => groupTicketsByStatus(entries), [entries])
  const otherEntries = groups.get(OTHER_COLUMN_KEY) ?? []

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
              queryClient.invalidateQueries({ queryKey: queryKeys.docs('tickets') })
            }}
          >
            Retry
          </button>
        </div>
      </div>
    )
  }

  return (
    // Drag handlers are deliberately no-ops in Phase 7. Phase 8 wires
    // them to the PATCH mutation; the DndContext structure stays identical —
    // only TicketCard's PHASE_7_DISABLED flag flips.
    <DndContext
      sensors={sensors}
      collisionDetection={closestCorners}
      onDragStart={() => {}}
      onDragOver={() => {}}
      onDragEnd={() => {}}
    >
      <div className={styles.board}>
        <h1 className={styles.title}>Kanban</h1>
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
