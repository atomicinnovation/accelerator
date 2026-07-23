import { useQuery } from "@tanstack/react-query";
import { fetchDocContent } from "./fetch";
import { queryKeys } from "./query-keys";

/** TanStack Query hook around `fetchDocContent`. Mirrors `useRelated`'s
 *  shape so `useDocPageData` can compose them uniformly. */
export function useDocContent(relPath: string | undefined) {
  return useQuery({
    queryKey: relPath
      ? queryKeys.docContent(relPath)
      : queryKeys.disabled("doc-content"),
    // biome-ignore lint/style/noNonNullAssertion: queryFn only runs while `enabled: !!relPath`, so `relPath` is guaranteed defined here
    queryFn: () => fetchDocContent(relPath!),
    enabled: !!relPath,
  });
}
