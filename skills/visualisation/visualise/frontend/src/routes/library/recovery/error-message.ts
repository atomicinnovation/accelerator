/** Resolve an arbitrary thrown value to a display string. Replaces the
 *  `err instanceof Error ? err.message : String(err)` ternary that was
 *  repeated in `LibraryDocView`, so the load-error surface stays purely
 *  presentational and cannot throw on a non-Error value (TanStack Query's
 *  `error` is typed `unknown`). */
export function errorMessage(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (e === undefined || e === null) return "An unknown error occurred.";
  return String(e);
}
