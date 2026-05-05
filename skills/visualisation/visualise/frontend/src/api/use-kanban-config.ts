import { useQuery } from '@tanstack/react-query'
import { queryKeys } from './query-keys'
import type { KanbanColumn } from './types'

interface KanbanConfigResponse {
  columns: KanbanColumn[]
}

async function fetchKanbanConfig(): Promise<KanbanConfigResponse> {
  const resp = await fetch('/api/kanban/config')
  if (!resp.ok) throw new Error(`/api/kanban/config returned ${resp.status}`)
  return resp.json() as Promise<KanbanConfigResponse>
}

export function useKanbanConfig() {
  return useQuery({
    queryKey: queryKeys.kanban(),
    queryFn: fetchKanbanConfig,
    staleTime: Infinity,
  })
}
