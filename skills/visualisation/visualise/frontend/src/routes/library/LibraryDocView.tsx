import { useParams } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { fetchDocs } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import { useDocPageData } from '../../api/use-doc-page-data'
import { useWikiLinkResolver } from '../../api/use-wiki-link-resolver'
import { useDeferredFetchingHint } from '../../api/use-deferred-fetching-hint'
import { FrontmatterChips } from '../../components/FrontmatterChips/FrontmatterChips'
import { MarkdownRenderer } from '../../components/MarkdownRenderer/MarkdownRenderer'
import { RelatedArtifacts } from '../../components/RelatedArtifacts/RelatedArtifacts'
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
    queryKey: type ? queryKeys.docs(type) : queryKeys.disabled('docs'),
    queryFn: () => fetchDocs(type!),
    enabled: type !== undefined,
  })

  const entry = entries.find(
    e => e.slug === fileSlug || fileSlugFromRelPath(e.relPath) === fileSlug,
  )

  // The doc-view's read path: content + related, both gated on
  // `entry?.relPath`. Body rendering is *not* gated on the wiki-link
  // resolver — pending markers handle the cache-warming window.
  const { content, related } = useDocPageData(entry?.relPath)
  const { resolver: resolveWikiLink, pattern: wikiLinkPattern } = useWikiLinkResolver()
  const showUpdatingHint = useDeferredFetchingHint(related)

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
  if (content.isError) {
    return (
      <p role="alert" className={styles.error}>
        Failed to load document content: {content.error instanceof Error ? content.error.message : String(content.error)}
      </p>
    )
  }
  if (!entry && entries.length > 0) return <p>Document not found.</p>
  if (content.isPending || !content.data) return <p>Loading…</p>

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
          {related.isError && (
            <p role="alert" className={styles.error}>
              Failed to load related artifacts:{' '}
              {related.error instanceof Error ? related.error.message : String(related.error)}
            </p>
          )}
          {related.isPending && !related.isError && (
            <p>Loading…</p>
          )}
          {related.data && (
            <RelatedArtifacts
              related={related.data}
              showUpdatingHint={showUpdatingHint}
            />
          )}
        </section>
        <section>
          <h3>File</h3>
          <p className={styles.meta}>{entry!.relPath}</p>
        </section>
      </div>

      {entry!.frontmatterState === 'malformed' && (
        <div className={styles.malformedBanner} aria-label="Document metadata header notice">
          <strong className={styles.malformedPrefix}>Warning:</strong>{' '}
          We couldn&rsquo;t read this document&rsquo;s metadata header; showing the file as-is.
        </div>
      )}

      <div className={styles.body}>
        <MarkdownRenderer content={content.data.content} resolveWikiLink={resolveWikiLink} wikiLinkPattern={wikiLinkPattern} />
      </div>
    </article>
  )
}
