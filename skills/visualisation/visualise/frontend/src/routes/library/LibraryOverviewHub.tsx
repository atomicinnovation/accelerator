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
        <section className={styles.phaseSection}>
          <h2 className={styles.phaseHeading}>META</h2>
          <div className={styles.hubGrid}>
            <HubCard docType={data.templates} />
          </div>
        </section>
      </>
    )
  }

  return (
    <Page
      eyebrow={<>LIBRARY</>}
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
      <div className={styles.cardTopRow}>
        <span className={styles.cardLabel}>
          {isGlyphDocTypeKey(docType.id) && (
            <Glyph docType={docType.id} size={24} framed />
          )}
          {docType.label}
        </span>
        <span className={styles.cardCount}>{docType.count}</span>
      </div>
      {docType.latest ? (
        <span className={styles.cardLatest}>latest · {docType.latest.title}</span>
      ) : (
        <span className={styles.cardEmpty}>no documents yet</span>
      )}
    </>
  )
  if (isEmpty) {
    return (
      <div className={`${styles.card} ${styles.cardDisabled}`} aria-disabled="true">
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
