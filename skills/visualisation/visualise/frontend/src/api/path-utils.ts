/** Extract the URL-friendly fileSlug from an IndexEntry's relPath —
 *  the last path segment with its `.md` extension stripped. Server
 *  indexer only admits `.md` today, so stripping a single extension is
 *  sufficient; update both this helper and the indexer contract if
 *  other extensions are ever admitted. */
export function fileSlugFromRelPath(relPath: string): string {
  return relPath.split('/').at(-1)?.replace(/\.md$/, '') ?? relPath
}
