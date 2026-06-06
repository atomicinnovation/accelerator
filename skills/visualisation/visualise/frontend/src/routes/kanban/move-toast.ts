import type { ShowToastInput } from '../../api/use-toast'
import type { IndexEntry } from '../../api/types'
import { errorToastMessageFor } from '../../api/move-error-copy'
import { describeEntry } from './announcements'
import type { DropOutcome } from './resolve-drop-outcome'

/** Whether the move's PATCH settled successfully, or the error it failed with. */
export type MoveResult = { ok: true } | { ok: false; error: unknown }

/**
 * Pure mapping from a resolved drop outcome + the already-resolved human target
 * label + success/error into a toast input — or `null` for any non-`move`
 * outcome (same-column, rejected, unknown) and for a missing source entry,
 * which raise no toast. The caller resolves `targetLabel` (asserting it is a
 * real column label, never the raw status key) and `entry` before calling, so
 * this stays a board/config-free pure function the board can unit-test
 * exhaustively without faking the `DndContext`.
 */
export function moveToastFor(
  outcome: DropOutcome,
  entry: IndexEntry | undefined,
  targetLabel: string,
  result: MoveResult,
): ShowToastInput | null {
  if (outcome.kind !== 'move') return null
  if (entry === undefined) return null
  if (result.ok) {
    // Heading-only success toast (the heading carries the confirmation; the
    // Toaster omits an empty body so it is not mis-padded).
    return {
      kind: 'ok',
      heading: `${describeEntry(entry)} moved to ${targetLabel}`,
      message: '',
    }
  }
  return {
    kind: 'error',
    heading: 'Move failed',
    message: errorToastMessageFor(result.error),
  }
}
