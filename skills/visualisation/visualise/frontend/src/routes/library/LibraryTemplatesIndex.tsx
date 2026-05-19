import type { ReactNode } from 'react'
import { Link } from '@tanstack/react-router'
import { Page } from '../../components/Page/Page'
import { useQuery } from '@tanstack/react-query'
import { fetchTemplates } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import type { TemplateSummary, TemplateTier, TemplateTierSource } from '../../api/types'
import { Glyph } from '../../components/Glyph/Glyph'
import { TIER_ORDER, TIER_SHORT_LABELS, glyphKeyForTemplate } from './template-tier'
import styles from './LibraryTemplatesIndex.module.css'

type TierState = 'absent' | 'present' | 'active'

function tierStateFor(t: TemplateTier | undefined): TierState {
  if (!t || !t.present) return 'absent'
  if (t.active) return 'active'
  return 'present'
}

interface TierPillsProps {
  tiers: TemplateTier[]
}

export function TierPills({ tiers }: TierPillsProps) {
  const byKey = new Map<TemplateTierSource, TemplateTier>(
    tiers.map((t) => [t.source, t]),
  )
  return (
    <span className={styles.tierChain}>
      {TIER_ORDER.map((source, idx) => {
        const t = byKey.get(source)
        const state = tierStateFor(t)
        return (
          <span key={source} className={styles.tierChainItem}>
            {idx > 0 ? (
              <span className={styles.tierSeparator} aria-hidden="true">
                ›
              </span>
            ) : null}
            <span className={styles.tierPill} data-state={state}>
              <span className={styles.tierPillBullet} aria-hidden="true">
                •
              </span>
              <span className={styles.tierPillLabel}>{TIER_SHORT_LABELS[source]}</span>
            </span>
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
          <li key={t.name} className={styles.row}>
            <Link
              to="/library/templates/$name"
              params={{ name: t.name }}
              className={styles.rowLink}
              data-selected={isSelected ? 'true' : undefined}
              aria-current={isSelected ? 'page' : undefined}
            >
              <span className={styles.rowGlyph} aria-hidden="true">
                {glyphKey ? (
                  <Glyph docType={glyphKey} size={24} framed />
                ) : (
                  <span className={styles.rowGlyphFallback} />
                )}
              </span>
              <span className={styles.rowName}>{t.name}.md</span>
              <TierPills tiers={t.tiers} />
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
      eyebrow={<>TEMPLATES · VIRTUAL</>}
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
