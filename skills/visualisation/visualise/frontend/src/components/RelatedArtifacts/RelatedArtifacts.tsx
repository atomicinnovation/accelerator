import type { IndexEntry, RelatedArtifactsResponse } from '../../api/types'
import { fileSlugFromRelPath } from '../../api/path-utils'
import styles from './RelatedArtifacts.module.css'

interface Props {
  related: RelatedArtifactsResponse
  /** Optional. Set to true ONLY when a refetch has been in flight for
   *  more than ~250ms AND will likely produce different data. The
   *  caller should drive this through `useDeferredFetchingHint` —
   *  passing `query.isFetching` directly causes the hint to flash on
   *  every background refetch (`refetchType: 'all'` invalidates the
   *  related prefix on every doc-changed event, including unrelated
   *  edits). */
  showUpdatingHint?: boolean
}

export function RelatedArtifacts({ related, showUpdatingHint }: Props) {
  const isEmpty =
    related.inferredCluster.length === 0 &&
    related.declaredOutbound.length === 0 &&
    related.declaredInbound.length === 0
  if (isEmpty) {
    return (
      <p className={styles.emptyAll}>
        This document has no declared or inferred relations.
      </p>
    )
  }
  return (
    <>
      {showUpdatingHint && (
        <p className={styles.updating} aria-live="polite">
          Updating…
        </p>
      )}
      <Legend />
      {related.declaredOutbound.length > 0 && (
        <RelatedGroup
          label="Targets"
          entries={related.declaredOutbound}
          kind="declared"
        />
      )}
      {related.declaredInbound.length > 0 && (
        <RelatedGroup
          label="Inbound reviews"
          entries={related.declaredInbound}
          kind="declared"
        />
      )}
      {related.inferredCluster.length > 0 && (
        <RelatedGroup
          label="Same lifecycle"
          entries={related.inferredCluster}
          kind="inferred"
        />
      )}
    </>
  )
}

function Legend() {
  return (
    <dl className={styles.legend}>
      <dt>Declared</dt>
      <dd>explicit cross-reference in frontmatter.</dd>
      <dt>Inferred</dt>
      <dd>shares a slug with this document.</dd>
    </dl>
  )
}

interface GroupProps {
  label: string
  entries: IndexEntry[]
  kind: 'declared' | 'inferred'
}

function RelatedGroup({ label, entries, kind }: GroupProps) {
  const groupClass =
    kind === 'declared' ? styles.groupDeclared : styles.groupInferred
  const badgeClass =
    kind === 'declared' ? styles.badgeDeclared : styles.badgeInferred
  return (
    <div className={`${styles.group} ${groupClass}`}>
      <h4 className={styles.groupHeading}>{label}</h4>
      <ul className={styles.groupList}>
        {entries.map((entry) => (
          <li key={entry.path} className={styles.groupItem}>
            <a href={`/library/${entry.type}/${fileSlugFromRelPath(entry.relPath)}`}>
              {entry.title || entry.relPath}
            </a>
            <span className={`${styles.badge} ${badgeClass}`}>{kind}</span>
          </li>
        ))}
      </ul>
    </div>
  )
}
