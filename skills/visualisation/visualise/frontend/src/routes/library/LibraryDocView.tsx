import { useParams } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { fetchDocs, fetchDocContent } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import { FrontmatterChips } from '../../components/FrontmatterChips/FrontmatterChips'
import { MarkdownRenderer } from '../../components/MarkdownRenderer/MarkdownRenderer'
import type { DocTypeKey } from '../../api/types'
import { isDocTypeKey } from '../../api/types'
import { fileSlugFromRelPath } from '../../api/path-utils'
import styles from './LibraryDocView.module.css'

interface Props {
  type?: DocTypeKey
  fileSlug?: string
}

export function LibraryDocView({ type: propType, fileSlug: propSlug }: Props) {
  // See LibraryTypeView for the prop-or-param + narrowing rationale.
  const params = useParams({ strict: false }) as { type?: string; fileSlug?: string }
  const rawType = propType ?? params.type
  const fileSlug = propSlug ?? params.fileSlug ?? ''

  const type: DocTypeKey | undefined =
    rawType && isDocTypeKey(rawType) ? rawType : undefined

  const { data: entries = [], isError: listError, error: listErr } = useQuery({
    queryKey: type ? queryKeys.docs(type) : ['docs', '__invalid__'] as const,
    queryFn: () => fetchDocs(type!),
    enabled: type !== undefined,
  })

  const entry = entries.find(e => fileSlugFromRelPath(e.relPath) === fileSlug)

  const { data: docContent, isLoading, isError: contentError, error: contentErr } = useQuery({
    queryKey: queryKeys.docContent(entry?.relPath ?? ''),
    queryFn: () => fetchDocContent(entry!.relPath),
    enabled: !!entry,
  })

  if (type === undefined) {
    return <p role="alert">Unknown doc type: {String(rawType)}</p>
  }
  if (!fileSlug) {
    return <p role="alert">Missing file slug.</p>
  }
  if (listError) {
    return (
      <p role="alert" className={styles.error}>
        Failed to load document list: {listErr instanceof Error ? listErr.message : String(listErr)}
      </p>
    )
  }
  if (contentError) {
    return (
      <p role="alert" className={styles.error}>
        Failed to load document content: {contentErr instanceof Error ? contentErr.message : String(contentErr)}
      </p>
    )
  }
  if (!entry && entries.length > 0) return <p>Document not found.</p>
  if (isLoading || !docContent) return <p>Loading…</p>

  return (
    <article className={styles.article}>
      <header className={styles.header}>
        <h1 className={styles.title}>{entry!.title}</h1>
        <FrontmatterChips
          frontmatter={entry!.frontmatter as Record<string, unknown>}
          state={entry!.frontmatterState}
        />
      </header>

      <div className={styles.aside}>
        <section>
          <h3>Related artifacts</h3>
          <p className={styles.empty}>No related artifacts yet.</p>
        </section>
        <section>
          <h3>File</h3>
          <p className={styles.meta}>{entry!.relPath}</p>
        </section>
      </div>

      <div className={styles.body}>
        <MarkdownRenderer content={docContent.content} />
      </div>
    </article>
  )
}
