import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import { Page } from '../../components/Page/Page'
import { Glyph, isGlyphDocTypeKey } from '../../components/Glyph/Glyph'
import { fetchLibraryStructure } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import type { LibraryDocType } from '../../api/types'
import styles from './LibraryOverviewHub.module.css'

export function LibraryOverviewHub() {
  const { data, isPending, isError } = useQuery({
    queryKey: queryKeys.libraryStructure(),
    queryFn: () => fetchLibraryStructure(),
  })

  let body: React.ReactNode
  if (isPending) {
    body = <p>Loading…</p>
  } else if (isError || !data) {
    body = <p>Could not load library structure.</p>
  } else {
    body = (
      <>
        {data.phases.map((phase) => (
          <section key={phase.id} className={styles.phaseSection}>
            <h2 className={styles.phaseHeading}>{phase.label.toUpperCase()}</h2>
            <div className={styles.hubGrid}>
              {phase.docTypes.map((dt) => (
                <HubCard key={dt.id} docType={dt} />
              ))}
            </div>
          </section>
        ))}
      </>
    )
  }

  return (
    <Page
      eyebrow={
        <>
          <LibraryIcon /> LIBRARY
        </>
      }
      title={
        <>
          All artifacts in{' '}
          <span className={styles.metaToken}>meta/</span>
        </>
      }
      subtitle="Every document grouped by lifecycle phase."
    >
      {body}
    </Page>
  )
}

function HubCard({ docType }: { docType: LibraryDocType }) {
  const isEmpty = docType.count === 0
  const inner = (
    <>
      {isGlyphDocTypeKey(docType.id) && (
        <Glyph docType={docType.id} size={32} framed />
      )}
      <div className={styles.cardBody}>
        <div className={styles.cardTopRow}>
          <span className={styles.cardLabel}>{docType.label}</span>
          <span className={styles.cardCount}>{docType.count}</span>
        </div>
        <p className={styles.cardLatest}>
          {docType.latest
            ? `latest · ${docType.latest.title}`
            : 'no docs yet'}
        </p>
      </div>
    </>
  )
  if (isEmpty) {
    return (
      <div
        className={`${styles.card} ${styles.cardDisabled}`}
        aria-disabled="true"
      >
        {inner}
      </div>
    )
  }
  return (
    <Link
      to="/library/$type"
      params={{ type: docType.id }}
      className={styles.card}
    >
      {inner}
    </Link>
  )
}

function LibraryIcon() {
  return (
    <svg
      width="12"
      height="12"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M4 4h4v16H4z" />
      <path d="M10 4h4v16h-4z" />
      <path d="m17 5 3 1-4 14-3-1z" />
    </svg>
  )
}
