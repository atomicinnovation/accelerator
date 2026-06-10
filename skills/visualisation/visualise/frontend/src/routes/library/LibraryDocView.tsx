import type { ReactNode } from 'react'
import { useParams } from '@tanstack/react-router'
import { Page } from '../../components/Page/Page'
import { useQuery } from '@tanstack/react-query'
import { fetchDocs } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import { useDocPageData } from '../../api/use-doc-page-data'
import { useWikiLinkResolver } from '../../api/use-wiki-link-resolver'
import { useDeferredFetchingHint } from '../../api/use-deferred-fetching-hint'
import { FrontmatterChips } from '../../components/FrontmatterChips/FrontmatterChips'
import { FrontmatterTable } from '../../components/FrontmatterTable/FrontmatterTable'
import { MarkdownRenderer } from '../../components/MarkdownRenderer/MarkdownRenderer'
import { RelatedArtifacts } from '../../components/RelatedArtifacts/RelatedArtifacts'
import { RelatedCluster } from '../../components/RelatedCluster/RelatedCluster'
import { useDocCluster } from '../../api/use-doc-cluster'
import { EyebrowLabel } from '../../components/EyebrowLabel/EyebrowLabel'
import { CopyPathButton } from '../../components/DetailHeaderActions/CopyPathButton'
import type { DocTypeKey } from '../../api/types'
import { isDocTypeKey } from '../../api/types'
import { fileSlugFromRelPath } from '../../api/path-utils'
import { formatBytes, formatEtagShort } from '../../api/format'
import styles from './LibraryDocView.module.css'

/** Strip a leading YAML frontmatter block (`---\n…\n---\n`) from the
 *  raw document content. The server returns the file verbatim, so
 *  markdown parsers would otherwise render the closing `---` as a
 *  horizontal rule and the title line as a setext heading. The
 *  frontmatter is already surfaced via the FrontmatterTable above the
 *  body. */
const FRONTMATTER_RE = /^---\r?\n[\s\S]*?\r?\n---\r?\n?/
function stripFrontmatter(content: string): string {
  return content.replace(FRONTMATTER_RE, '')
}

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
  const {
    cluster,
    isPending: clusterPending,
    isError: clusterError,
  } = useDocCluster(entry)
  const {
    resolver: resolveWikiLink,
    pattern: wikiLinkPattern,
    bareIdPattern,
  } = useWikiLinkResolver()
  const showUpdatingHint = useDeferredFetchingHint(related)

  if (type === undefined) {
    return <p role="alert">Unknown doc type: {String(rawType)}</p>
  }
  if (!fileSlug) {
    return <p role="alert">Missing file slug.</p>
  }

  let title: ReactNode = 'Loading…'
  let subtitle: ReactNode | undefined = undefined
  let body: ReactNode = <p>Loading…</p>

  if (listError) {
    title = 'Document not found'
    body = (
      <p role="alert" className={styles.error}>
        Failed to load document list: {listErr instanceof Error ? listErr.message : String(listErr)}
      </p>
    )
  } else if (content.isError) {
    title = 'Document not found'
    body = (
      <p role="alert" className={styles.error}>
        Failed to load document content: {content.error instanceof Error ? content.error.message : String(content.error)}
      </p>
    )
  } else if (!entry && entries.length > 0) {
    title = 'Document not found'
    body = <p>Document not found.</p>
  } else if (entry && content.data) {
    title = entry.title
    subtitle = (
      <FrontmatterChips
        frontmatter={entry.frontmatter as Record<string, unknown>}
        state={entry.frontmatterState}
      />
    )
    body = (
      <article className={styles.article}>
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
            <p className={styles.meta}>{entry.relPath}</p>
            <p className={styles.fileDetail} title={entry.etag}>
              etag · {formatEtagShort(entry.etag)}
            </p>
            <p className={styles.fileDetail}>size · {formatBytes(entry.size)}</p>
          </section>
          {/* Cluster section degrades visibly, mirroring the Related
              artifacts section above: render the shell while the cluster
              query is pending/errored so the heading + state shows rather
              than the block silently vanishing; suppress the section
              entirely only once the query has settled with no match. */}
          {(clusterPending || clusterError || cluster) && (
            <section>
              <h3>Cluster</h3>
              {clusterError ? (
                <p role="alert" className={styles.error}>
                  Could not load cluster. Try again later.
                </p>
              ) : clusterPending ? (
                <p>Loading…</p>
              ) : cluster ? (
                <RelatedCluster cluster={cluster} />
              ) : null}
            </section>
          )}
        </div>

        <div className={styles.bodyColumn}>
          {entry.frontmatterState === 'malformed' && (
            <div className={styles.malformedBanner} aria-label="Document metadata header notice">
              <strong className={styles.malformedPrefix}>Warning:</strong>{' '}
              We couldn&rsquo;t read this document&rsquo;s metadata header; showing the file as-is.
            </div>
          )}

          {entry.frontmatterState === 'parsed' && (
            <FrontmatterTable
              frontmatter={entry.frontmatter as Record<string, unknown>}
              resolveWikiLink={resolveWikiLink}
              bareIdPattern={bareIdPattern}
            />
          )}

          <div className={styles.body}>
            <MarkdownRenderer content={stripFrontmatter(content.data.content)} resolveWikiLink={resolveWikiLink} wikiLinkPattern={wikiLinkPattern} />
          </div>
        </div>
      </article>
    )
  }

  // Only show the per-doc-type eyebrow once the document has resolved —
  // not on the loading or "Document not found" branches.
  const hasResolvedDocument = Boolean(entry && content.data)

  return (
    <Page
      eyebrow={hasResolvedDocument ? <EyebrowLabel type={type} /> : undefined}
      title={title}
      subtitle={subtitle}
      actions={
        hasResolvedDocument && entry ? (
          <CopyPathButton relPath={entry.relPath} />
        ) : undefined
      }
    >
      {body}
    </Page>
  )
}
