import { useParams } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { fetchTemplateDetail } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import { MarkdownRenderer } from '../../components/MarkdownRenderer/MarkdownRenderer'
import type { TemplateTier } from '../../api/types'
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
  if (isError) {
    return (
      <p role="alert" className={styles.error}>
        Failed to load template: {error instanceof Error ? error.message : String(error)}
      </p>
    )
  }
  if (isLoading || !data) return <p>Loading…</p>

  return (
    <div className={styles.container}>
      <h1 className={styles.title}>{name}</h1>
      <div className={styles.tiers}>
        {data.tiers.map(tier => (
          <TierPanel
            key={tier.source}
            tier={tier}
            isActive={tier.source === data.activeTier}
          />
        ))}
      </div>
    </div>
  )
}

function TierPanel({ tier, isActive }: { tier: TemplateTier; isActive: boolean }) {
  return (
    <section className={`${styles.panel} ${!tier.present ? styles.absent : ''}`}>
      <header className={styles.panelHeader}>
        <span className={styles.tierLabel}>{TIER_LABELS[tier.source] ?? tier.source}</span>
        {isActive && <span className={styles.activeBadge}>active</span>}
        <code className={styles.path}>{tier.path}</code>
      </header>
      {tier.present && tier.content != null ? (
        <MarkdownRenderer content={tier.content} />
      ) : (
        <p className={styles.absentNote}>Not currently configured.</p>
      )}
    </section>
  )
}
