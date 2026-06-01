import type { ReactNode } from 'react'
import { useMemo, useState } from 'react'
import { Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { fetchLifecycleClusters, FetchError } from '../../api/fetch'
import { formatMtime } from '../../api/format'
import { queryKeys } from '../../api/query-keys'
import {
  WORKFLOW_PIPELINE_STEPS,
  type IndexEntry,
  type LifecycleCluster,
} from '../../api/types'
import { Pipeline } from '../../components/Pipeline/Pipeline'
import { Page } from '../../components/Page/Page'
import { Chip } from '../../components/Chip/Chip'
import { statusToVariant } from '../../api/status-variant'
import { ClockIcon, LifecycleEyebrowIcon } from './icons'
import styles from './LifecycleIndex.module.css'

type SortMode = 'recent' | 'completeness'

function completenessScore(c: LifecycleCluster): number {
  return WORKFLOW_PIPELINE_STEPS.reduce(
    (n, step) => (c.completeness[step.key] ? n + 1 : n),
    0,
  )
}

function sortClusters(clusters: LifecycleCluster[], mode: SortMode): LifecycleCluster[] {
  const sorted = [...clusters]
  if (mode === 'recent') {
    sorted.sort((a, b) => b.lastChangedMs - a.lastChangedMs)
  } else {
    sorted.sort((a, b) => {
      const diff = completenessScore(b) - completenessScore(a)
      return diff !== 0 ? diff : b.lastChangedMs - a.lastChangedMs
    })
  }
  return sorted
}

function workItemEntry(cluster: LifecycleCluster): IndexEntry | undefined {
  return cluster.entries.find(e => e.type === 'work-items')
}

function statusOf(cluster: LifecycleCluster): string | null {
  const entry = workItemEntry(cluster)
  if (!entry || entry.frontmatterState !== 'parsed') return null
  const raw = entry.frontmatter['status']
  return typeof raw === 'string' && raw.length > 0 ? raw : null
}

function pluralise(n: number, singular: string, plural = `${singular}s`): string {
  return `${n} ${n === 1 ? singular : plural}`
}

export function LifecycleIndex() {
  const [sortMode, setSortMode] = useState<SortMode>('recent')

  const { data: clusters = [], isPending, isError, error } = useQuery({
    queryKey: queryKeys.lifecycle(),
    queryFn: fetchLifecycleClusters,
  })

  const visible = useMemo(
    () => sortClusters(clusters, sortMode),
    [clusters, sortMode],
  )

  let content: ReactNode
  if (isPending) {
    content = <p>Loading…</p>
  } else if (isError) {
    content = (
      <p role="alert" className={styles.error}>
        {error instanceof FetchError
          ? 'Could not load lifecycle clusters. Try again later.'
          : 'Something went wrong loading lifecycle clusters. Try again later.'}
      </p>
    )
  } else if (clusters.length === 0) {
    content = <p className={styles.empty}>No lifecycle clusters found.</p>
  } else {
    content = (
      <ul className={styles.cardList}>
        {visible.map(cluster => {
          const status = statusOf(cluster)
          const artifactCount = cluster.entries.length
          const stagesComplete = cluster.completeness.present.filter(p =>
            WORKFLOW_PIPELINE_STEPS.some(s => s.docType === p),
          ).length
          return (
            <li key={cluster.slug} className={styles.card}>
              <Link
                to="/lifecycle/$slug"
                params={{ slug: cluster.slug }}
                className={styles.cardLink}
              >
                <div className={styles.cardHeading}>
                  <h3 className={styles.cardTitle}>{cluster.title}</h3>
                  <span className={styles.cardSlug}>{cluster.slug}</span>
                </div>
                <div className={styles.cardMeta}>
                  {status !== null && (
                    <Chip variant={statusToVariant(status)} size="sm">
                      {status}
                    </Chip>
                  )}
                  <span className={styles.cardMetaTime}>
                    <ClockIcon size={11} />
                    {formatMtime(cluster.lastChangedMs)}
                  </span>
                  <span className={styles.cardMetaArtifacts}>
                    {pluralise(artifactCount, 'artifact')}
                  </span>
                </div>
                <div className={`${styles.cardPipe} ac-lcard__pipe`}>
                  <Pipeline completeness={cluster.completeness} variant="card" />
                  <span className={styles.cardPipeCount}>{stagesComplete}/8</span>
                </div>
              </Link>
            </li>
          )
        })}
      </ul>
    )
  }

  const showSort = !isPending && !isError && clusters.length > 0
  return (
    <Page
      eyebrow={<><LifecycleEyebrowIcon /> Lifecycle</>}
      title="Work units, from idea to shipped"
      subtitle="Each row is a slug-clustered work unit. Filled tiles mark the stages present on disk. Missing stages are where the workflow has gaps."
      actions={showSort
        ? <SortSegment value={sortMode} onChange={setSortMode} />
        : undefined}
    >
      {content}
    </Page>
  )
}

interface SortSegmentProps {
  value: SortMode
  onChange: (m: SortMode) => void
}

function SortSegment({ value, onChange }: SortSegmentProps) {
  return (
    <div className={styles.sortSegment} role="group" aria-label="Sort clusters">
      <SortPill current={value} value="recent" label="Updated" onChange={onChange} />
      <SortPill current={value} value="completeness" label="Completeness" onChange={onChange} />
    </div>
  )
}

function SortPill({
  current, value, label, onChange,
}: {
  current: SortMode
  value: SortMode
  label: string
  onChange: (m: SortMode) => void
}) {
  const active = current === value
  return (
    <button
      type="button"
      className={`${styles.sortPill} ${active ? styles.sortPillActive : ''}`}
      aria-pressed={active}
      onClick={() => onChange(value)}
    >
      {label}
    </button>
  )
}

