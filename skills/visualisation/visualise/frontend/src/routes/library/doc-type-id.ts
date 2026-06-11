/**
 * Formats a work-item id by zero-padding the numeric tail to four digits.
 * Pass-through for already-formatted ids (≥4 digits) and any string that
 * does not match `<prefix>-<digits>`.
 */
export function formatDocId(workItemId: string | null | undefined): string {
  if (!workItemId) return "";
  const match = workItemId.match(/^([^-]+)-(\d+)$/);
  if (!match) return workItemId;
  const [, prefix, digits] = match;
  return `${prefix}-${digits.padStart(4, "0")}`;
}
