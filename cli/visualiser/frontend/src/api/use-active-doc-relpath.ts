import { useQuery } from "@tanstack/react-query";
import { useParams } from "@tanstack/react-router";
import { fetchDocs } from "./fetch";
import { fileSlugFromRelPath } from "./path-utils";
import { queryKeys } from "./query-keys";
import { isDocTypeKey } from "./types";

/**
 * Resolves the relPath of the document currently being viewed, purely from
 * the URL params (no prop overrides, no error surfacing). Returns `undefined`
 * whenever not on a doc route or the docs list is unloaded/unmatched — which
 * is exactly the "no toast off-route" behaviour the subscriber relies on.
 *
 * Intentionally NOT a shared resolver with `LibraryDocView` — that view
 * supports prop overrides and surfaces query errors; unifying them would
 * change its behaviour.
 */
export function useActiveDocRelPath(): string | undefined {
  const params = useParams({ strict: false }) as {
    type?: string;
    fileSlug?: string;
  };
  const type =
    params.type && isDocTypeKey(params.type) ? params.type : undefined;
  const fileSlug = params.fileSlug ?? "";
  const { data: entries = [] } = useQuery({
    queryKey: type ? queryKeys.docs(type) : queryKeys.disabled("docs"),
    // biome-ignore lint/style/noNonNullAssertion: queryFn only runs while `enabled: type !== undefined`, so `type` is guaranteed defined here
    queryFn: () => fetchDocs(type!),
    enabled: type !== undefined,
  });
  if (!fileSlug) return undefined;
  return entries.find(
    (e) => e.slug === fileSlug || fileSlugFromRelPath(e.relPath) === fileSlug,
  )?.relPath;
}
