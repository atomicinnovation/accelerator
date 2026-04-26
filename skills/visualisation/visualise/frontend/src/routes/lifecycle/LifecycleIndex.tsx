import { useMemo, useState } from 'react'
import { Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { fetchLifecycleClusters, FetchError } from '../../api/fetch'
import { formatMtime } from '../../api/format'
import { queryKeys } from '../../api/query-keys'
import { WORKFLOW_PIPELINE_STEPS, type LifecycleCluster } from '../../api/types'
import { PipelineDots } from '../../components/PipelineDots/PipelineDots'
import styles from './LifecycleIndex.module.css'

type SortMode = 'recent' | 'oldest' | 'completeness'

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
  } else if (mode === 'oldest') {
    sorted.sort((a, b) => a.lastChangedMs - b.lastChangedMs)
  } else {
    sorted.sort((a, b) => {
      const diff = completenessScore(b) - completenessScore(a)
      return diff !== 0 ? diff : b.lastChangedMs - a.lastChangedMs
    })
  }
  return sorted
}

function filterClusters(clusters: LifecycleCluster[], filter: string): LifecycleCluster[] {
  const needle = filter.trim().toLowerCase()
  if (!needle) return clusters
  return clusters.filter(c =>
    c.title.toLowerCase().includes(needle) ||
    c.slug.toLowerCase().includes(needle),
  )
}

export function LifecycleIndex() {
  const [sortMode, setSortMode] = useState<SortMode>('recent')
  const [filter, setFilter] = useState<string>('')

  const { data: clusters = [], isPending, isError, error } = useQuery({
    queryKey: queryKeys.lifecycle(),
    queryFn: fetchLifecycleClusters,
  })

  const visible = useMemo(
    () => sortClusters(filterClusters(clusters, filter), sortMode),
    [clusters, filter, sortMode],
  )

  if (isPending) return <p>Loading…</p>
  if (isError) {
    return (
      <p role="alert" className={styles.error}>
        {error instanceof FetchError
          ? 'Could not load lifecycle clusters. Try again later.'
          : 'Something went wrong loading lifecycle clusters. Try again later.'}
      </p>
    )
  }
  if (clusters.length === 0) {
    return <p className={styles.empty}>No lifecycle clusters found.</p>
  }

  return (
    <div className={styles.container}>
      <div className={styles.toolbar}>
        <input
          type="search"
          aria-label="Filter clusters"
          placeholder="Filter…"
          className={styles.filterInput}
          value={filter}
          onChange={e => setFilter(e.target.value)}
        />
        <span className={styles.toolbarLabel}>Sort:</span>
        <SortButton current={sortMode} value="recent"       label="Recent"       onChange={setSortMode} />
        <SortButton current={sortMode} value="oldest"       label="Oldest"       onChange={setSortMode} />
        <SortButton current={sortMode} value="completeness" label="Completeness" onChange={setSortMode} />
      </div>

      {visible.length === 0 && (
        <p role="status" className={styles.empty}>
          No clusters match &quot;{filter}&quot;.
        </p>
      )}

      <ul className={styles.cardList}>
        {visible.map(cluster => {
          const score = completenessScore(cluster)
          return (
            <li key={cluster.slug} className={styles.card}>
              <Link
                to="/lifecycle/$slug"
                params={{ slug: cluster.slug }}
                className={styles.cardLink}
              >
                <div className={styles.cardHeader}>
                  <h3 className={styles.cardTitle}>{cluster.title}</h3>
                  <span className={styles.cardSlug}>{cluster.slug}</span>
                </div>
                <PipelineDots completeness={cluster.completeness} />
                <div className={styles.cardMeta}>
                  <span>{score} of {WORKFLOW_PIPELINE_STEPS.length} stages</span>
                  <span>{formatMtime(cluster.lastChangedMs)}</span>
                </div>
              </Link>
            </li>
          )
        })}
      </ul>
    </div>
  )
}

function SortButton({
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
      className={`${styles.sortButton} ${active ? styles.sortButtonActive : ''}`}
      aria-pressed={active}
      onClick={() => onChange(value)}
    >
      {label}
    </button>
  )
}
