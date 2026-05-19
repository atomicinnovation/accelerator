import { useEffect, useMemo, useRef } from 'react'
import { useParams } from '@tanstack/react-router'
import hljs from 'highlight.js/lib/core'
import markdownLang from 'highlight.js/lib/languages/markdown'
import { useQuery } from '@tanstack/react-query'
import { fetchTemplateDetail } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import { Chip } from '../../components/Chip/Chip'
import type { TemplateDetail, TemplateTier } from '../../api/types'
import { TIER_LABELS } from './template-tier'
import { TemplatesPage } from './LibraryTemplatesIndex'
import styles from './LibraryTemplatesView.module.css'

hljs.registerLanguage('markdown', markdownLang)

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
        {tier.present ? (
          <span className={styles.tierNote}>
            {isActive ? 'highest priority — present' : 'present in this repo'}
          </span>
        ) : (
          <span className={styles.tierNote}>not currently configured</span>
        )}
      </div>
    </section>
  )
}

function findWinningTier(data: TemplateDetail): TemplateTier | undefined {
  return data.tiers.find((t) => t.source === data.activeTier && t.present)
}

function TemplatePreviewPane({ data }: { data: TemplateDetail }) {
  const winning = findWinningTier(data)
  const content = winning?.content ?? ''
  const codeRef = useRef<HTMLElement | null>(null)

  const highlightedHtml = useMemo<string | null>(() => {
    if (!content) return null
    try {
      return hljs.highlight(content, { language: 'markdown' }).value
    } catch {
      return null
    }
  }, [content])

  useEffect(() => {
    // hljs.highlight returns HTML; if highlight failed (returns null), we
    // fall back to textContent below — no effect needed.
    if (!codeRef.current) return
    if (highlightedHtml == null) {
      codeRef.current.textContent = content
    } else {
      codeRef.current.innerHTML = highlightedHtml
    }
  }, [highlightedHtml, content])

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
          <span className={styles.contentHashLabel} aria-label="Content hash">
            {data.sha256}
          </span>
        ) : null}
      </div>
      <pre className={styles.previewCodeBlock}>
        <code
          ref={codeRef}
          className={`language-markdown ${styles.previewCode} hljs`}
        />
      </pre>
    </div>
  )
}

