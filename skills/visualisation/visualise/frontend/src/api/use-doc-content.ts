import { useQuery } from '@tanstack/react-query'
import { fetchDocContent } from './fetch'
import { queryKeys } from './query-keys'

/** TanStack Query hook around `fetchDocContent`. Mirrors `useRelated`'s
 *  shape so `useDocPageData` can compose them uniformly. */
export function useDocContent(relPath: string | undefined) {
  return useQuery({
    queryKey: relPath
      ? queryKeys.docContent(relPath)
      : queryKeys.disabled('doc-content'),
    queryFn: () => fetchDocContent(relPath!),
    enabled: !!relPath,
  })
}
