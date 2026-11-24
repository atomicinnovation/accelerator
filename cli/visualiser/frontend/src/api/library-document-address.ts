import type { IndexEntry } from "./types";

function filenameStemOf(relPath: string): string {
  return relPath.split("/").at(-1)?.replace(/\.md$/, "") ?? relPath;
}

export function resolveLibraryDocument(
  entries: readonly IndexEntry[],
  documentId: string,
): IndexEntry | undefined {
  return (
    entries.find((e) => e.id === documentId) ??
    entries.find((e) => e.slug === documentId) ??
    entries.find((e) => filenameStemOf(e.relPath) === documentId)
  );
}
