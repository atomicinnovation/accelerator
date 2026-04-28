import { useDocContent } from './use-doc-content'
import { useRelated } from './use-related'

/** Compose the doc-view's read-path queries: content + related. The
 *  composition layer keeps `LibraryDocView` thin and is the join point
 *  for any future read-side fanout (backlinks-graph,
 *  suggested-next-action, etc.). */
export function useDocPageData(relPath: string | undefined) {
  const content = useDocContent(relPath)
  const related = useRelated(relPath)
  return { content, related }
}
