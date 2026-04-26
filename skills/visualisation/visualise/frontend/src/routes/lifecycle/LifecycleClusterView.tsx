import { Link, useParams } from '@tanstack/react-router'
import { lifecycleClusterRoute } from '../../router'
import { useQuery } from '@tanstack/react-query'
import { fetchLifecycleCluster, FetchError } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import { fileSlugFromRelPath } from '../../api/path-utils'
import {
  WORKFLOW_PIPELINE_STEPS, LONG_TAIL_PIPELINE_STEPS,
  type IndexEntry,
} from '../../api/types'
import styles from './LifecycleClusterView.module.css'

/** Pure renderer. Tests render this directly with a literal slug. */
export function LifecycleClusterContent({ slug }: { slug: string }) {
  const { data: cluster, isPending, isError, error } = useQuery({
    queryKey: queryKeys.lifecycleCluster(slug),
    queryFn: () => fetchLifecycleCluster(slug),
  })

  if (isPending) return <p>Loading…</p>
  if (isError || !cluster) {
    const isNotFound = error instanceof FetchError && error.status === 404
    return (
      <div className={styles.container}>
        <Link to="/lifecycle" className={styles.backLink}>
          ← All clusters
        </Link>
        <p role="alert" className={styles.error}>
          {isNotFound
            ? <>No cluster called <code>{slug}</code> exists.</>
            : 'Something went wrong loading this cluster. Try again later.'}
        </p>
      </div>
    )
  }

  return (
    <div className={styles.container}>
      <Link to="/lifecycle" className={styles.backLink}>
        ← All clusters
      </Link>

      <header className={styles.header}>
        <h2 className={styles.title}>{cluster.title}</h2>
        <span className={styles.slug}>{cluster.slug}</span>
      </header>

      <ol className={styles.timeline}>
        {WORKFLOW_PIPELINE_STEPS.map(step => renderStage(step, cluster.entries))}
      </ol>

      {LONG_TAIL_PIPELINE_STEPS.some(
        step => cluster.entries.some(e => e.type === step.docType),
      ) && (
        <section
          className={styles.longTail}
          aria-labelledby="lifecycle-other-artifacts"
        >
          <h3
            id="lifecycle-other-artifacts"
            className={styles.longTailHeading}
          >
            Other artifacts
          </h3>
          <ol className={styles.timeline}>
            {LONG_TAIL_PIPELINE_STEPS.map(step => renderStage(step, cluster.entries))}
          </ol>
        </section>
      )}
    </div>
  )
}

type Step = (typeof WORKFLOW_PIPELINE_STEPS)[number] | (typeof LONG_TAIL_PIPELINE_STEPS)[number]

function renderStage(step: Step, entries: IndexEntry[]) {
  const stageEntries = entries.filter(e => e.type === step.docType)
  if (stageEntries.length === 0) {
    if (step.longTail) return null
    return (
      <li
        key={step.key}
        className={`${styles.stage} ${styles.absent}`}
        data-stage={step.key}
        data-present="false"
      >
        <span className={styles.stageLabel}>{step.label}</span>
        <span className={styles.placeholder}>{step.placeholder}</span>
      </li>
    )
  }
  return (
    <li
      key={step.key}
      className={styles.stage}
      data-stage={step.key}
      data-present="true"
    >
      <span className={styles.stageLabel}>{step.label}</span>
      <ul className={styles.entryList}>
        {stageEntries.map(e => (
          <EntryCard key={e.relPath} entry={e} />
        ))}
      </ul>
    </li>
  )
}

function EntryCard({ entry }: { entry: IndexEntry }) {
  const fileSlug = fileSlugFromRelPath(entry.relPath)
  const status = (entry.frontmatter as Record<string, unknown>).status
  const date = (entry.frontmatter as Record<string, unknown>).date
  return (
    <li className={styles.entryCard}>
      <Link
        to="/library/$type/$fileSlug"
        params={{ type: entry.type, fileSlug }}
        className={styles.entryLink}
      >
        <span className={styles.entryTitle}>{entry.title}</span>
      </Link>
      <div className={styles.entryMeta}>
        {typeof date === 'string' && <span>{date}</span>}
        {typeof status === 'string' && (
          <span className={styles.statusBadge}>{status}</span>
        )}
      </div>
      {entry.bodyPreview && (
        <p className={styles.bodyPreview}>{entry.bodyPreview}</p>
      )}
    </li>
  )
}

/** Router-bound shell. Reads the strictly-typed `slug` from the
 *  cluster route and forwards it to the pure renderer. */
export function LifecycleClusterView() {
  const { slug } = useParams({ from: lifecycleClusterRoute.id })
  return <LifecycleClusterContent slug={slug} />
}
