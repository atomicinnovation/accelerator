import { useQuery } from '@tanstack/react-query'
import { queryKeys } from './query-keys'

export interface ServerInfo {
  name?: string
  version?: string
}

async function fetchServerInfo(): Promise<ServerInfo> {
  const resp = await fetch('/api/info')
  if (!resp.ok) {
    throw new Error(`/api/info returned ${resp.status}`)
  }
  return resp.json() as Promise<ServerInfo>
}

export function useServerInfo() {
  return useQuery({
    queryKey: queryKeys.serverInfo(),
    queryFn: fetchServerInfo,
    staleTime: Infinity,
  })
}
