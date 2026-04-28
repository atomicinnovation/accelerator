import { useQuery } from '@tanstack/react-query'
import { fetchRelated } from './fetch'
import { queryKeys } from './query-keys'

/** TanStack Query hook around `fetchRelated`. Gates the query on
 *  `relPath` so the consumer can render the route shell while the
 *  selected entry resolves; the query never fires until a defined
 *  relPath is in hand. */
export function useRelated(relPath: string | undefined) {
  return useQuery({
    queryKey: relPath ? queryKeys.related(relPath) : queryKeys.disabled('related'),
    queryFn: () => fetchRelated(relPath!),
    enabled: !!relPath,
  })
}
