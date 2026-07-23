import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { PatchResult } from "./fetch";
import { type FetchError, patchWorkItemFrontmatter } from "./fetch";
import { queryKeys } from "./query-keys";
import { useSelfCauseRegistry } from "./self-cause";
import type { IndexEntry } from "./types";

export interface MoveWorkItemVars {
  entry: IndexEntry;
  toStatus: string;
}

type MoveWorkItemContext = { previous?: IndexEntry[] };

export function useMoveWorkItem() {
  const qc = useQueryClient();
  const registry = useSelfCauseRegistry();

  return useMutation<
    PatchResult,
    FetchError,
    MoveWorkItemVars,
    MoveWorkItemContext
  >({
    mutationFn: ({ entry, toStatus }) =>
      patchWorkItemFrontmatter(entry.relPath, { status: toStatus }, entry.etag),

    onMutate: async ({ entry, toStatus }) => {
      await qc.cancelQueries({ queryKey: queryKeys.docs("work-items") });
      const previous = qc.getQueryData<IndexEntry[]>(
        queryKeys.docs("work-items"),
      );
      qc.setQueryData<IndexEntry[]>(queryKeys.docs("work-items"), (old) =>
        old?.map((e) =>
          e.relPath === entry.relPath
            ? { ...e, frontmatter: { ...e.frontmatter, status: toStatus } }
            : e,
        ),
      );
      return { previous };
    },

    onSuccess: (result) => {
      registry.register(result.etag);
    },

    onError: (_err, _vars, ctx) => {
      qc.setQueryData(queryKeys.docs("work-items"), ctx?.previous);
    },

    onSettled: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.docs("work-items") });
    },
  });
}
