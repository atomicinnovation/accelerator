import type { ReactNode } from 'react'
import { Link } from '@tanstack/react-router'
import { Page } from '../../components/Page/Page'
import { useQuery } from '@tanstack/react-query'
import { fetchTemplates } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import type { TemplateSummary, TemplateTier } from '../../api/types'
import { Chip, type ChipVariant } from '../../components/Chip/Chip'
import { Glyph } from '../../components/Glyph/Glyph'
import { TIER_ORDER, TIER_SHORT_LABELS, glyphKeyForTemplate } from './template-tier'
import styles from './LibraryTemplatesIndex.module.css'

function chipVariantForTier(present: boolean, active: boolean): ChipVariant {
  if (!present) return 'neutral'
  if (active) return 'green'
  return 'indigo'
}

interface TierChipsProps {
  tiers: TemplateTier[]
}

export function TierChips({ tiers }: TierChipsProps) {
  const byKey = new Map(tiers.map((t) => [t.source, t]))
  return (
    <span className={styles.tierChain}>
      {TIER_ORDER.map((source, idx) => {
        const t = byKey.get(source)
        const present = t?.present ?? false
        const active = t?.active ?? false
        return (
          <span key={source} className={styles.tierChainItem}>
            {idx > 0 ? (
              <span className={styles.tierArrow} aria-hidden="true">
                →
              </span>
            ) : null}
            <Chip
              variant={chipVariantForTier(present, active)}
              leading={<span className={styles.tierAdd} aria-hidden="true">+</span>}
            >
              {TIER_SHORT_LABELS[source]}
            </Chip>
          </span>
        )
      })}
    </span>
  )
}

interface TemplatesIndexListProps {
  templates: TemplateSummary[]
  selectedName?: string
}

export function TemplatesIndexList({ templates, selectedName }: TemplatesIndexListProps) {
  return (
    <ul className={styles.list}>
      {templates.map((t) => {
        const glyphKey = glyphKeyForTemplate(t.name)
        const isSelected = selectedName === t.name
        return (
          <li key={t.name}>
            <Link
              to="/library/templates/$name"
              params={{ name: t.name }}
              className={styles.row}
              data-selected={isSelected ? 'true' : undefined}
              aria-current={isSelected ? 'page' : undefined}
            >
              <span className={styles.rowGlyph} aria-hidden="true">
                {glyphKey ? (
                  <Glyph docType={glyphKey} size={16} />
                ) : (
                  <span className={styles.rowGlyphFallback} />
                )}
              </span>
              <span className={styles.rowName}>{t.name}.md</span>
              <TierChips tiers={t.tiers} />
              <span className={styles.rowChevron} aria-hidden="true">
                ›
              </span>
            </Link>
          </li>
        )
      })}
    </ul>
  )
}

interface PageProps {
  selectedName?: string
  extraContent?: ReactNode
}

export function TemplatesPage({ selectedName, extraContent }: PageProps) {
  const { data, isLoading, isError, error } = useQuery({
    queryKey: queryKeys.templates(),
    queryFn: fetchTemplates,
  })

  let listContent: ReactNode = <p>Loading…</p>
  if (isError) {
    listContent = (
      <p role="alert" className={styles.error}>
        Failed to load templates: {error instanceof Error ? error.message : String(error)}
      </p>
    )
  } else if (!isLoading && data) {
    listContent = (
      <TemplatesIndexList templates={data.templates} selectedName={selectedName} />
    )
  }

  return (
    <Page
      title="Authoring templates"
      subtitle="Resolved across three tiers. The highest-priority present tier wins at authoring time — but every tier is inspectable here, regardless of current config."
    >
      {listContent}
      {extraContent}
    </Page>
  )
}

export function LibraryTemplatesIndex() {
  return <TemplatesPage />
}
