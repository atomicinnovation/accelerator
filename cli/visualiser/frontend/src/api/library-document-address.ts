import { interpolatePath } from "@tanstack/react-router";
import type { DocTypeKey, IndexEntry } from "./types";

export type LibraryDocument = Pick<IndexEntry, "type" | "id" | "relPath">;

export interface LibraryLink {
  to: "/library/$type/$documentId";
  params: { type: DocTypeKey; documentId: string };
}

function filenameStemOf(relPath: string): string {
  return relPath.split("/").at(-1)?.replace(/\.md$/, "") ?? relPath;
}

export function documentIdOf(document: LibraryDocument): string {
  return document.id ?? filenameStemOf(document.relPath);
}

export function libraryLink(document: LibraryDocument): LibraryLink {
  return {
    to: "/library/$type/$documentId",
    params: { type: document.type, documentId: documentIdOf(document) },
  };
}

export function libraryHref(link: LibraryLink): string {
  return interpolatePath({ path: link.to, params: link.params })
    .interpolatedPath;
}

export function descriptiveSlugOf(
  entry: Pick<IndexEntry, "slug" | "relPath">,
): string {
  return entry.slug ?? filenameStemOf(entry.relPath);
}

export function resolveLibraryDocument(
  entries: readonly IndexEntry[],
  documentId: string,
): IndexEntry | undefined {
  return (
    entries.find((e) => e.id === documentId) ??
    entries.find((e) => filenameStemOf(e.relPath) === documentId) ??
    entries.find((e) => e.slug === documentId)
  );
}
