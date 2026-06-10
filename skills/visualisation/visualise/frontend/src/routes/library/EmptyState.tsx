import type { CSSProperties } from 'react'
import type { DocTypeKey } from '../../api/types'
import { DOC_TYPE_LABELS } from '../../api/types'
import { TYPE_COPY, EMPTY_TYPE_PLURALS } from './empty-descriptions'
import { BigGlyph } from '../../components/BigGlyph/BigGlyph'
import styles from './EmptyState.module.css'

export interface EmptyStateProps {
  docType: DocTypeKey
  /**
   * On-disk directory shown in the eyebrow + footer copy. Defaults to the
   * canonical project-relative path from `TYPE_COPY`. Callers can pass
   * the server's `DocType.dirPath` when it differs (e.g. when a project
   * has remapped the path via config).
   */
  dirPath?: string
}

export function EmptyState({ docType, dirPath }: EmptyStateProps) {
  const copy = TYPE_COPY[docType]
  const path = dirPath ?? copy.path
  const plural = EMPTY_TYPE_PLURALS[docType]
  const label = DOC_TYPE_LABELS[docType]
  const hue = copy.hue
  const cssVars = {
    ['--ac-empty-page-hue' as never]: String(hue),
  } satisfies CSSProperties

  return (
    <div className={styles.card} style={cssVars} role="status">
      <div className={styles.hero}>
        <BigGlyph docType={docType} size={96} />
      </div>
      <div className={styles.body}>
        <div className={styles.eyebrow}>{path}</div>
        <h2 className={styles.title} data-testid="empty-state-title">No {plural} yet.</h2>
        <p className={styles.lede}>{copy.purpose}</p>
        <p className={styles.foot}>
          New files added to{' '}
          <span className={styles.pathInline}>{path}</span> are picked up
          live — this view will populate as soon as the indexer sees{' '}
          {label.toLowerCase()}.
        </p>
      </div>
    </div>
  )
}
