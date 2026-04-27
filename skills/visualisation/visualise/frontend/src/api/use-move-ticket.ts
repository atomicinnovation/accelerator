import { useMutation, useQueryClient } from '@tanstack/react-query'
import { patchTicketFrontmatter, type FetchError } from './fetch'
import { useSelfCauseRegistry } from './self-cause'
import { queryKeys } from './query-keys'
import type { IndexEntry, KanbanColumnKey } from './types'
import type { PatchResult } from './fetch'

export interface MoveTicketVars {
  entry: IndexEntry
  toStatus: KanbanColumnKey
}

type MoveTicketContext = { previous?: IndexEntry[] }

export function useMoveTicket() {
  const qc = useQueryClient()
  const registry = useSelfCauseRegistry()

  return useMutation<PatchResult, FetchError, MoveTicketVars, MoveTicketContext>({
    mutationFn: ({ entry, toStatus }) =>
      patchTicketFrontmatter(entry.relPath, { status: toStatus }, entry.etag),

    onMutate: async ({ entry, toStatus }) => {
      await qc.cancelQueries({ queryKey: queryKeys.docs('tickets') })
      const previous = qc.getQueryData<IndexEntry[]>(queryKeys.docs('tickets'))
      qc.setQueryData<IndexEntry[]>(queryKeys.docs('tickets'), (old) =>
        old?.map((e) =>
          e.relPath === entry.relPath
            ? { ...e, frontmatter: { ...e.frontmatter, status: toStatus } }
            : e,
        ),
      )
      return { previous }
    },

    onSuccess: (result) => {
      registry.register(result.etag)
    },

    onError: (_err, _vars, ctx) => {
      qc.setQueryData(queryKeys.docs('tickets'), ctx?.previous)
    },

    onSettled: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.docs('tickets') })
    },
  })
}
