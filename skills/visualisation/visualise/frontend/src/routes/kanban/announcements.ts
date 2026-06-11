import type { Announcements } from "@dnd-kit/core";
import type { IndexEntry, KanbanColumn } from "../../api/types";
import { OTHER_COLUMN } from "../../api/types";

interface Deps {
  entries: () => Map<string, IndexEntry>;
  columns: () => ReadonlyArray<KanbanColumn>;
}

export function workItemIdFromRelPath(relPath: string): string | null {
  const m = /(\d{4})-/.exec(relPath.split("/").pop() ?? "");
  return m ? m[1] : null;
}

function labelFor(
  columnId: unknown,
  columns: ReadonlyArray<KanbanColumn>,
): string {
  const id = String(columnId);
  return (
    columns.find((c) => c.key === id)?.label ??
    (OTHER_COLUMN.key === id ? OTHER_COLUMN.label : id)
  );
}

/**
 * Names a work item for screen-reader announcements AND the move-confirmation
 * toast — the single source of truth so the two can never drift. Operates on a
 * resolved entry; an `undefined` entry (e.g. deleted mid-drag) yields the bare
 * "work item" fallback. The number is relPath-derived (matching the historical
 * announcement wording) rather than `entry.workItemId`.
 */
export function describeEntry(entry: IndexEntry | undefined): string {
  if (!entry) return "work item";
  const num = workItemIdFromRelPath(entry.relPath);
  const title = entry.title;
  if (num && title) return `work item ${num}: ${title}`;
  if (title) return `work item ${title}`;
  return `work item ${entry.relPath}`;
}

function describe(id: unknown, entries: Map<string, IndexEntry>): string {
  return describeEntry(typeof id === "string" ? entries.get(id) : undefined);
}

export function buildKanbanAnnouncements({
  entries,
  columns,
}: Deps): Announcements {
  return {
    onDragStart({ active }): string {
      return `Picked up ${describe(active.id, entries())}.`;
    },
    onDragOver({ active, over }): string | undefined {
      if (!over) return undefined;
      return `${describe(active.id, entries())} is over ${labelFor(over.id, columns())}.`;
    },
    onDragEnd({ active, over }): string {
      if (!over)
        return `Drop of ${describe(active.id, entries())} cancelled, no target.`;
      return `Moved ${describe(active.id, entries())} to ${labelFor(over.id, columns())}.`;
    },
    onDragCancel({ active }): string {
      return `Drag of ${describe(active.id, entries())} cancelled.`;
    },
  };
}
