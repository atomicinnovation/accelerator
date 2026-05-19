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

/** Inline layers SVG (Feather "layers" path), used as the page eyebrow
 *  glyph for the Templates view. Sourced from the design prototype's
 *  Icon component (ui.jsx). */
export function LayersIcon({ size = 12 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="m12 3 9 5-9 5-9-5z" />
      <path d="m3 13 9 5 9-5" />
      <path d="m3 18 9 5 9-5" />
    </svg>
  )
}

/** Inline chevron-right SVG. Used as the inter-pill separator and as
 *  the row disclosure marker. */
export function ChevronRightIcon({ size = 10, className }: { size?: number; className?: string }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      className={className}
    >
      <path d="m9 6 6 6-6 6" />
    </svg>
  )
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
              <ChevronRightIcon size={10} className={styles.tierSeparator} />
            ) : null}
            <span className={styles.tierPill} data-state={state}>
              <span className={styles.tierPillBullet} aria-hidden="true" />
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
              <span className={styles.rowName}>
                <span className={styles.rowGlyph} aria-hidden="true">
                  {glyphKey ? (
                    <Glyph docType={glyphKey} size={24} framed />
                  ) : (
                    <span className={styles.rowGlyphFallback} />
                  )}
                </span>
                {t.name}.md
              </span>
              <TierPills tiers={t.tiers} />
              <ChevronRightIcon size={14} className={styles.rowChevron} />
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
      eyebrow={
        <>
          <LayersIcon size={12} />
          TEMPLATES
        </>
      }
      title="Templates"
      subtitle="The starting shape for every new doc. Pick a template to see which version is active and what the other tiers look like."
    >
      {listContent}
      {extraContent}
    </Page>
  )
}

export function LibraryTemplatesIndex() {
  return <TemplatesPage />
}
