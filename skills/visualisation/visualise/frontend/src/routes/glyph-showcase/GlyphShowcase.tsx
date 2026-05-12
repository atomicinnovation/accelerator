import type { ReactElement } from 'react'
import { Glyph, GLYPH_DOC_TYPE_KEYS } from '../../components/Glyph/Glyph'
import { DOC_TYPE_LABELS } from '../../api/types'
import styles from './GlyphShowcase.module.css'

const SIZES = [16, 24, 32] as const

export function GlyphShowcase(): ReactElement {
  // `data-testid="glyph-cell-<docType>-<size>"` is the locator contract for
  // tests/visual-regression/glyph-showcase.spec.ts and glyph-resolved-fill.spec.ts;
  // any change here must update both specs.
  return (
    <main className={styles.root}>
      <h1>Glyph Showcase</h1>
      <p className={styles.note}>
        Toggle <code>document.documentElement.dataset.theme</code> between{' '}
        <code>light</code> and <code>dark</code> in dev tools to compare.
      </p>
      <div className={styles.grid}>
        <div className={styles.headerRow}>
          <span className={styles.headerLabel}>doc type</span>
          {SIZES.map((s) => (
            <span key={s} className={styles.headerCell}>{s}px</span>
          ))}
        </div>
        {GLYPH_DOC_TYPE_KEYS.map((docType) => (
          <div key={docType} className={styles.row}>
            <span className={styles.label}>
              <code>{docType}</code>
              <span className={styles.friendly}>{DOC_TYPE_LABELS[docType]}</span>
            </span>
            {SIZES.map((size) => (
              <span
                key={size}
                className={styles.cell}
                data-testid={`glyph-cell-${docType}-${size}`}
              >
                <Glyph docType={docType} size={size} />
              </span>
            ))}
          </div>
        ))}
      </div>
    </main>
  )
}
