import { Glyph, isGlyphDocTypeKey } from '../../components/Glyph/Glyph'
import type { DocTypeKey } from '../../api/types'
import { EMPTY_DESCRIPTIONS, EMPTY_TYPE_PLURALS } from './empty-descriptions'
import styles from './EmptyState.module.css'

export interface EmptyStateProps {
  docType: DocTypeKey
  dirPath: string
}

export function EmptyState({ docType, dirPath }: EmptyStateProps) {
  const description = EMPTY_DESCRIPTIONS[docType]
  const plural = EMPTY_TYPE_PLURALS[docType]
  return (
    <div className={styles.card} role="status">
      {isGlyphDocTypeKey(docType) && (
        <div className={styles.glyph}>
          <Glyph docType={docType} size={32} />
        </div>
      )}
      <p className={styles.path}>{dirPath}</p>
      <h2 className={styles.headline}>no {plural} yet.</h2>
      <p className={styles.description}>{description}</p>
      <p className={styles.footer}>
        New files added to {dirPath} are picked up live — this view will populate
        as soon as the indexer sees them.
      </p>
    </div>
  )
}
