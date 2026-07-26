import { useQueries } from "@tanstack/react-query";
import { useMemo } from "react";
import { fetchDocs } from "../../../api/fetch";
import { fileSlugFromRelPath } from "../../../api/path-utils";
import { queryKeys } from "../../../api/query-keys";
import { DOC_TYPE_KEYS, isPhysicalDocTypeKey } from "../../../api/types";
import {
  isSuggestible,
  rankSlugSuggestions,
  type SlugCandidate,
} from "./rank-slug-suggestions";

const PHYSICAL_KEYS = DOC_TYPE_KEYS.filter(isPhysicalDocTypeKey);

export interface NearbySlugSuggestions {
  /** Ranked suggestions. Empty until the fan-out has settled (see `isPending`). */
  suggestions: SlugCandidate[];
  /** True while the surface is enabled and at least one enabled query is still
   *  in flight — the surface shows a loading hint and withholds the list until
   *  this is false, so suggestions appear once in final ranked order rather than
   *  popping in and re-sorting as queries resolve. */
  isPending: boolean;
}

export function useNearbySlugSuggestions(
  missingSlug: string,
): NearbySlugSuggestions {
  const enabled = isSuggestible(missingSlug);

  const results = useQueries({
    queries: PHYSICAL_KEYS.map((type) => ({
      queryKey: queryKeys.docs(type),
      queryFn: () => fetchDocs(type),
      enabled,
    })),
  });

  // Settled = enabled and no enabled query still fetching. `r.isPending` for an
  // `enabled:false` query is irrelevant because `enabled` short-circuits below.
  const isPending = enabled && results.some((r) => r.isPending);

  // `results` is a fresh array identity every render; the meaningful inputs are
  // the per-query `data` references plus the settle flag, which we list
  // explicitly below. Gating the body on `!isPending` means the rank runs once,
  // when all data is ready.
  // biome-ignore lint/correctness/useExhaustiveDependencies: results identity churns every render; we depend on per-query data + the settle flag explicitly
  return useMemo(() => {
    if (!enabled) return { suggestions: [], isPending: false };
    // Withhold the (re-)ranked list until the fan-out settles, so the rendered
    // block doesn't shuffle as individual queries resolve.
    if (isPending) return { suggestions: [], isPending: true };

    const candidates: SlugCandidate[] = [];
    for (const r of results) {
      for (const e of r.data ?? []) {
        candidates.push({
          type: e.type,
          slug: e.slug ?? fileSlugFromRelPath(e.relPath),
          title: e.title,
          mtimeMs: e.mtimeMs,
          relPath: e.relPath,
        });
      }
    }
    return {
      suggestions: rankSlugSuggestions(missingSlug, candidates),
      isPending: false,
    };
  }, [enabled, isPending, missingSlug, ...results.map((r) => r.data)]);
}
