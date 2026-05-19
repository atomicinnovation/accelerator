import type { ReactNode } from 'react'
import { useParams } from '@tanstack/react-router'
import { Page } from '../../components/Page/Page'
import { useQuery } from '@tanstack/react-query'
import { fetchTemplateDetail } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import { MarkdownRenderer } from '../../components/MarkdownRenderer/MarkdownRenderer'
import { Chip } from '../../components/Chip/Chip'
import type { TemplateDetail, TemplateTier } from '../../api/types'
import styles from './LibraryTemplatesView.module.css'

const TIER_LABELS: Record<string, string> = {
  'plugin-default': 'Plugin default',
  'user-override': 'User override',
  'config-override': 'Config override',
}

interface Props { name?: string }

export function LibraryTemplatesView({ name: propName }: Props) {
  const params = useParams({ strict: false }) as { name?: string }
  const name = propName ?? params.name

  const { data, isLoading, isError, error } = useQuery({
    queryKey: name ? queryKeys.templateDetail(name) : ['template-detail', '__invalid__'] as const,
    queryFn: () => fetchTemplateDetail(name!),
    enabled: !!name,
  })

  if (!name) {
    return <p role="alert">Missing template name.</p>
  }

  let title: ReactNode = 'Loading…'
  let content: ReactNode = <p>Loading…</p>

  if (isError) {
    title = 'Template not found'
    content = (
      <p role="alert" className={styles.error}>
        Failed to load template: {error instanceof Error ? error.message : String(error)}
      </p>
    )
  } else if (!isLoading && data) {
    title = name
    content = (
      <div className={styles.twoColumn} data-testid="templates-detail-layout">
        <div className={styles.tiers}>
          {data.tiers.map(tier => (
            <TierPanel
              key={tier.source}
              tier={tier}
              isActive={tier.source === data.activeTier}
            />
          ))}
        </div>
        <TemplatePreviewPane data={data} />
      </div>
    )
  }

  return <Page title={title}>{content}</Page>
}

function getWinningTier(data: TemplateDetail): TemplateTier | undefined {
  return data.tiers.find(t => t.source === data.activeTier && t.present)
}

function TemplatePreviewPane({ data }: { data: TemplateDetail }) {
  const winning = getWinningTier(data)
  if (!winning) return null
  return (
    <div className={styles.previewPane} data-testid="template-preview-pane">
      <div className={styles.previewHeader} data-testid="template-preview-header">
        <span className={styles.previewPath}>{winning.path}</span>
        {data.sha256 ? (
          <span
            className={styles.contentHashLabel}
            aria-label="Content hash"
          >
            {data.sha256}
          </span>
        ) : null}
      </div>
      {winning.content != null ? (
        <MarkdownRenderer content={winning.content} />
      ) : null}
    </div>
  )
}

function TierPanel({ tier, isActive }: { tier: TemplateTier; isActive: boolean }) {
  return (
    <section
      className={`${styles.panel} ${!tier.present ? styles.absent : ''}`}
      data-active={isActive ? 'true' : undefined}
    >
      <header className={styles.panelHeader}>
        <span className={styles.tierLabel}>{TIER_LABELS[tier.source] ?? tier.source}</span>
        {isActive && <Chip variant="indigo">active</Chip>}
        {!tier.present && <Chip variant="neutral">absent</Chip>}
        <code className={styles.path}>{tier.path}</code>
      </header>
      {!tier.present ? (
        <p className={styles.absentNote}>Not currently configured.</p>
      ) : null}
    </section>
  )
}
