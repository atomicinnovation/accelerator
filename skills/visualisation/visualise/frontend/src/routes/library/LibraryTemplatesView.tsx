import { useParams } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { fetchTemplateDetail } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import { Chip } from '../../components/Chip/Chip'
import type { TemplateDetail, TemplateTier } from '../../api/types'
import { TIER_LABELS } from './template-tier'
import { TemplatesPage } from './LibraryTemplatesIndex'
import { highlightTemplate } from './template-highlight'
import styles from './LibraryTemplatesView.module.css'

interface Props {
  name?: string
}

export function LibraryTemplatesView({ name: propName }: Props) {
  const params = useParams({ strict: false }) as { name?: string }
  const name = propName ?? params.name

  if (!name) {
    return <p role="alert">Missing template name.</p>
  }

  return (
    <TemplatesPage selectedName={name} extraContent={<TemplateDetailSection name={name} />} />
  )
}

function TemplateDetailSection({ name }: { name: string }) {
  const { data, isLoading, isError, error } = useQuery({
    queryKey: queryKeys.templateDetail(name),
    queryFn: () => fetchTemplateDetail(name),
  })

  if (isError) {
    return (
      <section className={styles.detail} aria-labelledby="template-detail-heading">
        <p role="alert" className={styles.error}>
          Failed to load template: {error instanceof Error ? error.message : String(error)}
        </p>
      </section>
    )
  }
  if (isLoading || !data) {
    return (
      <section className={styles.detail} aria-labelledby="template-detail-heading">
        <p>Loading…</p>
      </section>
    )
  }

  return (
    <section className={styles.detail} aria-labelledby="template-detail-heading">
      <h2 id="template-detail-heading" className={styles.detailHeading}>
        THREE TIERS · {name.toUpperCase()}.MD
      </h2>
      <div className={styles.twoColumn} data-testid="templates-detail-layout">
        <div className={styles.tiers}>
          {data.tiers.map((tier, index) => (
            <TierCard
              key={tier.source}
              tier={tier}
              tierIndex={index + 1}
              isActive={tier.source === data.activeTier}
            />
          ))}
        </div>
        <TemplatePreviewPane data={data} />
      </div>
    </section>
  )
}

/** Description text shown under the tier path, modelled on the prototype's
 *  `tierDesc()` function (view-templates.jsx). */
function tierDescription(tier: TemplateTier): string {
  switch (tier.source) {
    case 'config-override':
      // Tier 1 is the highest-priority slot. When the launcher knows which
      // config file declared the override we add it, mirroring the prototype:
      //   highest priority · .accelerator/config.md
      // For the absent case we still surface "highest priority" so the
      // semantic order of the tier stack is unambiguous.
      return tier.configSource
        ? `highest priority · ${tier.configSource}`
        : 'highest priority'
    case 'user-override':
      // Tier 2 — the active-when-no-config-override tier. Mirrors the
      // prototype copy: "<tier-2-path> in this repo".
      return `${tier.path} in this repo`
    case 'plugin-default':
      return 'plugin-default · always present'
    default:
      return ''
  }
}

function TierCard({
  tier,
  tierIndex,
  isActive,
}: {
  tier: TemplateTier
  tierIndex: number
  isActive: boolean
}) {
  return (
    <section
      className={`${styles.panel} ${!tier.present ? styles.absent : ''}`}
      data-active={isActive ? 'true' : undefined}
    >
      <div className={styles.tierEyebrow}>TIER {tierIndex}</div>
      <header className={styles.panelHeader}>
        <span className={styles.tierLabel}>{TIER_LABELS[tier.source] ?? tier.source}</span>
        {isActive && <Chip variant="indigo">active</Chip>}
        {!tier.present && <Chip variant="neutral">absent</Chip>}
      </header>
      <div className={styles.tierPaths}>
        <code className={styles.tierPath}>{tier.path}</code>
        <span className={styles.tierNote}>{tierDescription(tier)}</span>
      </div>
    </section>
  )
}

function findWinningTier(data: TemplateDetail): TemplateTier | undefined {
  return data.tiers.find((t) => t.source === data.activeTier && t.present)
}

/** Truncate a `sha256-<hex>` etag for compact display: keeps the
 *  `sha256-` prefix + 5 hex characters (total 12 chars) followed by an
 *  ellipsis. The untruncated value is surfaced via a `title` attribute. */
function truncateSha256(sha: string): string {
  if (sha.length <= 13) return sha
  return `${sha.slice(0, 12)}…`
}

function TemplatePreviewPane({ data }: { data: TemplateDetail }) {
  const winning = findWinningTier(data)

  if (!winning) {
    return (
      <div className={styles.previewPane} data-testid="template-preview-pane">
        <p className={styles.absentNote}>No winning tier resolved.</p>
      </div>
    )
  }

  return (
    <div className={styles.previewPane} data-testid="template-preview-pane">
      <div className={styles.previewHeader} data-testid="template-preview-header">
        <span className={styles.previewPath}>{winning.path}</span>
        {data.sha256 ? (
          <span
            className={styles.contentHashLabel}
            aria-label="Content hash"
            title={data.sha256}
            data-full-sha={data.sha256}
          >
            {truncateSha256(data.sha256)}
          </span>
        ) : null}
      </div>
      <div className={styles.previewBody}>
        {winning.content != null
          ? highlightTemplate(winning.content)
          : <span className={styles.absentNote}>tier not present</span>}
      </div>
    </div>
  )
}
