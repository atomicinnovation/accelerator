import type { CSSProperties, ReactNode } from 'react'
import { Link, useParams } from '@tanstack/react-router'
import { lifecycleClusterRoute } from '../../router'
import { useQuery } from '@tanstack/react-query'
import { fetchLifecycleCluster, FetchError } from '../../api/fetch'
import { formatMtime } from '../../api/format'
import { queryKeys } from '../../api/query-keys'
import { fileSlugFromRelPath } from '../../api/path-utils'
import {
  LIFECYCLE_PIPELINE_STEPS,
  WORKFLOW_PIPELINE_STEPS,
  LONG_TAIL_PIPELINE_STEPS,
  type IndexEntry,
  type LifecycleCluster,
} from '../../api/types'
import { Chip } from '../../components/Chip/Chip'
import { Page } from '../../components/Page/Page'
import { Pipeline } from '../../components/Pipeline/Pipeline'
import { Glyph } from '../../components/Glyph/Glyph'
import { statusToVariant } from '../../api/status-variant'
import { formatDocId } from '../library/doc-type-id'
import { ClockIcon, LifecycleEyebrowIcon } from './icons'
import { clusterViaLabel } from './cluster-via-label'
import styles from './LifecycleClusterView.module.css'

type Step = (typeof LIFECYCLE_PIPELINE_STEPS)[number]

function statusOf(cluster: LifecycleCluster): string | null {
  const wi = cluster.entries.find(e => e.type === 'work-items')
  if (!wi || wi.frontmatterState !== 'parsed') return null
  const raw = wi.frontmatter['status']
  return typeof raw === 'string' && raw.length > 0 ? raw : null
}

/** Pure renderer. Tests render this directly with a literal slug. */
export function LifecycleClusterContent({ slug }: { slug: string }) {
  const { data: cluster, isPending, isError, error } = useQuery({
    queryKey: queryKeys.lifecycleCluster(slug),
    queryFn: () => fetchLifecycleCluster(slug),
  })

  const eyebrow = (
    <>
      <LifecycleEyebrowIcon />
      <Link to="/lifecycle" className={styles.eyebrowLink}>Lifecycle</Link>
    </>
  )

  if (isPending) {
    return (
      <Page eyebrow={eyebrow} title="Loading…">
        <p>Loading…</p>
      </Page>
    )
  }

  if (isError || !cluster) {
    const isNotFound = error instanceof FetchError && error.status === 404
    return (
      <Page
        eyebrow={eyebrow}
        title={isNotFound ? 'Cluster not found' : 'Cluster unavailable'}
      >
        <p role="alert" className={styles.error}>
          {isNotFound
            ? <>No cluster called <code>{slug}</code> exists.</>
            : 'Something went wrong loading this cluster. Try again later.'}
        </p>
      </Page>
    )
  }

  const status = statusOf(cluster)
  const subtitle = (
    <div className={styles.subRow}>
      {status !== null && (
        <Chip variant={statusToVariant(status)} size="sm">{status}</Chip>
      )}
      <span className={styles.subMeta}>
        <ClockIcon size={11} />
        updated {formatMtime(cluster.lastChangedMs)}
      </span>
    </div>
  )

  return (
    <Page eyebrow={eyebrow} title={cluster.title} subtitle={subtitle}>
      <section className={`${styles.pipelinePanel} ac-lcluster__pipeline`}>
        <div className={`${styles.pipelineEyebrow} ac-lcluster__pipeline-eyebrow`}>
          Pipeline
        </div>
        <Pipeline completeness={cluster.completeness} variant="panel" />
      </section>

      <ol className={styles.timeline}>
        {buildTimeline(cluster).map((slot, i) =>
          slot.entry ? (
            <TimelineStep
              key={`${slot.step.docType}-${slot.entry.relPath}`}
              step={slot.step}
              entry={slot.entry}
              cluster={cluster}
            />
          ) : (
            <MissingStep key={`missing-${slot.step.docType}-${i}`} step={slot.step} />
          ),
        )}
      </ol>
    </Page>
  )
}

interface TimelineSlot {
  step: Step
  entry: IndexEntry | null
}

function buildTimeline(cluster: LifecycleCluster): TimelineSlot[] {
  const slots: TimelineSlot[] = []
  // Workflow stages: render every present entry; placeholder when none.
  for (const step of WORKFLOW_PIPELINE_STEPS) {
    const matches = cluster.entries.filter(e => e.type === step.docType)
    if (matches.length === 0) {
      slots.push({ step, entry: null })
    } else {
      for (const entry of matches) {
        slots.push({ step, entry })
      }
    }
  }
  // Long-tail: append present entries inline; no placeholders for missing
  // long-tail types (matches the prototype's "notes appended if present").
  for (const step of LONG_TAIL_PIPELINE_STEPS) {
    const matches = cluster.entries.filter(e => e.type === step.docType)
    for (const entry of matches) {
      slots.push({ step, entry })
    }
  }
  return slots
}

interface StageTileProps {
  step: Step
  active: boolean
  dashed?: boolean
  size?: number
}

function StageTile({ step, active, dashed = false, size = 36 }: StageTileProps) {
  const accent = `var(--ac-stage-${step.docType})`
  // The `--stage-color` custom property feeds both the active fill and
  // the inactive bg/border color-mix in the CSS module.
  const tileStyle = { color: accent } as CSSProperties
  // Glyph size must land on the 16/24/32 grid; pick the nearest at-or-below
  // value so the inner icon doesn't overflow the tile.
  const glyphSize: 16 | 24 | 32 = size >= 32 ? 32 : size >= 24 ? 24 : 16
  return (
    <span
      className={styles.timelineTile}
      data-stage={step.docType}
      data-active={String(active)}
      data-dashed={String(dashed)}
      style={{ ...tileStyle, width: size, height: size }}
      aria-hidden="true"
    >
      <Glyph
        docType={step.docType}
        size={glyphSize}
        colorVar={active ? 'var(--atomic-white)' : accent}
      />
    </span>
  )
}

interface TimelineStepProps {
  step: Step
  entry: IndexEntry
  cluster: LifecycleCluster
}

function TimelineStep({ step, entry, cluster }: TimelineStepProps): ReactNode {
  const fileSlug = fileSlugFromRelPath(entry.relPath)
  const fmStatus = entry.frontmatterState === 'parsed'
    ? entry.frontmatter['status']
    : undefined
  const fmVerdict = entry.frontmatterState === 'parsed'
    ? entry.frontmatter['verdict']
    : undefined
  const fmDate = entry.frontmatterState === 'parsed'
    ? entry.frontmatter['date']
    : undefined
  const id = entry.workItemId
    ? formatDocId(entry.workItemId)
    : fileSlugFromRelPath(entry.relPath)
  return (
    <li className={styles.step} data-stage={step.docType}>
      <span className={styles.stepNode}>
        <StageTile step={step} active />
      </span>
      <Link
        to="/library/$type/$fileSlug"
        params={{ type: entry.type, fileSlug }}
        className={styles.tcard}
      >
        <div className={styles.tcardHead}>
          <div className={styles.tcardHeadLeft}>
            <span className={styles.tcardStage}>{step.label.toUpperCase()}</span>
            <span className={styles.tcardTitle}>{entry.title}</span>
          </div>
          <div className={styles.tcardHeadRight}>
            {typeof fmStatus === 'string' && fmStatus.length > 0 && (
              <Chip variant={statusToVariant(fmStatus)} size="sm">{fmStatus}</Chip>
            )}
            {typeof fmVerdict === 'string' && fmVerdict.length > 0 && (
              <Chip variant={statusToVariant(fmVerdict)} size="sm">{fmVerdict}</Chip>
            )}
            {typeof fmDate === 'string' && fmDate.length > 0 && (
              <span className={styles.tcardDate}>{fmDate}</span>
            )}
          </div>
        </div>
        <div className={styles.tcardBody}>
          <span className={styles.tcardId}>{id}</span>
          {' · modified '}
          <time>{formatMtime(entry.mtimeMs)}</time>
        </div>
        <div className={styles.clusteredVia}>
          {clusterViaLabel(
            { type: entry.type, clusterKey: entry.clusterKey },
            { clusterKey: cluster.clusterKey },
          )}
        </div>
      </Link>
    </li>
  )
}

function MissingStep({ step }: { step: Step }): ReactNode {
  return (
    <li className={`${styles.step} ${styles.stepMissing}`} data-stage={step.docType}>
      <span className={styles.stepNode}>
        <StageTile step={step} active={false} dashed />
      </span>
      <div className={`${styles.tcard} ${styles.tcardMissing}`}>
        No {step.label.toLowerCase()} yet
      </div>
    </li>
  )
}

/** Router-bound shell. Reads the strictly-typed `slug` from the
 *  cluster route and forwards it to the pure renderer. */
export function LifecycleClusterView() {
  const { slug } = useParams({ from: lifecycleClusterRoute.id })
  return <LifecycleClusterContent slug={slug} />
}
