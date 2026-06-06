import type { IndexEntry, RelatedArtifactsResponse } from '../../api/types'
import { fileSlugFromRelPath } from '../../api/path-utils'
import { formatMtime } from '../../api/format'
import { Glyph } from '../Glyph/Glyph'
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

type Kind = 'declared' | 'inferred'

/** Visible tag copy kept separate from the discriminant so copy and the
 *  CSS class can diverge later (e.g. an `aria-label` distinct from the
 *  class name). */
const TAG_TEXT: Record<Kind, string> = {
  declared: '(declared)',
  inferred: '(inferred)',
}

function tagClass(kind: Kind): string {
  return kind === 'declared' ? styles.tagDeclared : styles.tagInferred
}

/** Trailing affordance chevron (prototype `Icon name="chevron-right"`). */
function Chevron() {
  return (
    <svg
      className={styles.chevron}
      width={14}
      height={14}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="m9 6 6 6-6 6" />
    </svg>
  )
}

export function RelatedArtifacts({ related, showUpdatingHint }: Props) {
  // Concatenate the two declared arrays and dedupe by `path`. The server
  // builds declaredOutbound/declaredInbound independently and does NOT
  // dedup across them (`related.rs` dedups only within each list), so a
  // bidirectional declared relation (A targets B; B targets A) appears in
  // both arrays. The old grouped UI tolerated this under distinct
  // `Targets` / `Referenced by` headings; the flat Option B list would
  // otherwise render it twice with a colliding `key={entry.path}`. Keep
  // the first occurrence; directionality is intentionally collapsed at the
  // render boundary (the server contract is untouched).
  const declaredAll = [...related.declaredOutbound, ...related.declaredInbound]
  const seen = new Set<string>()
  const declared = declaredAll.filter(
    (e) => !seen.has(e.path) && seen.add(e.path),
  )
  const inferred = related.inferredCluster
  const rows: { entry: IndexEntry; kind: Kind }[] = [
    ...declared.map((entry) => ({ entry, kind: 'declared' as const })),
    ...inferred.map((entry) => ({ entry, kind: 'inferred' as const })),
  ]

  if (rows.length === 0) {
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
      <ul className={styles.list} data-testid="related-list">
        {rows.map(({ entry, kind }) => {
          const title = entry.title || entry.relPath
          return (
            <li
              key={entry.path}
              className={styles.item}
              data-testid="related-row"
              data-kind={kind}
            >
              {/* Whole-row link (prototype `.ac-related__item`). aria-label
                  pins the accessible name to the title so the supplementary
                  mtime/tag metadata isn't announced as part of the link. */}
              <a
                className={styles.row}
                href={`/library/${entry.type}/${fileSlugFromRelPath(entry.relPath)}`}
                aria-label={title}
              >
                <Glyph docType={entry.type} size={16} framed />
                <span className={styles.content}>
                  <span className={styles.title}>{title}</span>
                  <span className={styles.metaRow}>
                    <time>{formatMtime(entry.mtimeMs)}</time>
                    <span
                      className={`${styles.tag} ${tagClass(kind)}`}
                      data-testid="related-tag"
                    >
                      {TAG_TEXT[kind]}
                    </span>
                  </span>
                </span>
                <Chevron />
              </a>
            </li>
          )
        })}
      </ul>
    </>
  )
}
