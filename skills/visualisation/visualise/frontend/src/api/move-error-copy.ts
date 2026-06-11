import { ConflictError, FetchError } from "./fetch";

/**
 * User-facing copy for a failed work-item status move, rendered in the
 * (assertive, persistent) error toast. Lives in `src/api/` alongside the
 * fetch/error types because it is derived purely from the error class.
 *
 * This is the single error-class → copy mapper for the move path (it
 * supersedes the deleted inline conflict banner's `conflictMessageFor`), so the
 * `ConflictError`/`FetchError` discrimination lives in exactly one place and
 * only the strings differ per branch.
 */
export function errorToastMessageFor(error: unknown): string {
  if (error instanceof ConflictError) {
    return "This work item was updated by another editor. Your change was not saved, so the card has returned to its original column.";
  }
  if (error instanceof FetchError) {
    return "The work item could not be saved. Try again in a moment.";
  }
  return "An unexpected error occurred while saving. Try again.";
}
