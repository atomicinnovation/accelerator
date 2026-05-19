import type { ReactNode } from 'react'
import { Link } from '@tanstack/react-router'
import { Page } from '../../components/Page/Page'
import { useQuery } from '@tanstack/react-query'
import { fetchTemplates } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import type { TemplateSummary, TemplateTier } from '../../api/types'
import { Chip, type ChipVariant } from '../../components/Chip/Chip'
import { TIER_ORDER, TIER_SHORT_LABELS } from './template-tier'
import styles from './LibraryTemplatesIndex.module.css'

function chipVariantForTier(present: boolean, active: boolean): ChipVariant {
  if (!present) return 'neutral'
  if (active) return 'green'
  return 'indigo'
}

function TierPresenceRow({ tiers }: { tiers: TemplateTier[] }) {
  const byKey = new Map(tiers.map((t) => [t.source, t]))
  return (
    <span className={styles.tierPresenceRow}>
      {TIER_ORDER.map((source) => {
        const t = byKey.get(source)
        const present = t?.present ?? false
        const active = t?.active ?? false
        return (
          <Chip key={source} variant={chipVariantForTier(present, active)}>
            {TIER_SHORT_LABELS[source]}
          </Chip>
        )
      })}
    </span>
  )
}

export function LibraryTemplatesIndex() {
  const { data, isLoading, isError, error } = useQuery({
    queryKey: queryKeys.templates(),
    queryFn: fetchTemplates,
  })

  let content: ReactNode = <p>Loading…</p>
  if (isError) {
    content = (
      <p role="alert" className={styles.error}>
        Failed to load templates: {error instanceof Error ? error.message : String(error)}
      </p>
    )
  } else if (!isLoading && data) {
    content = (
      <ul className={styles.list}>
        {data.templates.map((t: TemplateSummary) => (
          <li key={t.name}>
            <Link to="/library/templates/$name" params={{ name: t.name }}>
              {t.name}
            </Link>
            <TierPresenceRow tiers={t.tiers} />
          </li>
        ))}
      </ul>
    )
  }

  return (
    <Page title="Templates">
      {content}
    </Page>
  )
}
