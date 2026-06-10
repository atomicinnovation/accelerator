import type { ReactElement } from 'react'
import { BigGlyph } from '../../components/BigGlyph/BigGlyph'
import { DOC_TYPE_KEYS, DOC_TYPE_LABELS } from '../../api/types'
import styles from './BigGlyphShowcase.module.css'

export function BigGlyphShowcase(): ReactElement {
  // `data-testid="big-glyph-cell-<docType>"` is the locator contract for
  // tests/visual-regression/big-glyph-showcase.spec.ts; any change here must
  // update that spec (and its committed baselines).
  //
  // NOTE (story 0083): this is a throwaway dev-only fixture purely to host the
  // visual-regression spec. When 0083's consolidated DevDesignSystem supersedes
  // it, the spec's testid contract and the 26 committed baselines must be
  // MIGRATED to the new surface, not just deleted, or the coverage is lost.
  return (
    <main className={styles.root}>
      <h1>Big Glyph Showcase</h1>
      <p className={styles.note}>
        Toggle <code>document.documentElement.dataset.theme</code> between{' '}
        <code>light</code> and <code>dark</code> in dev tools to compare. Cells
        sit on <code>--ac-bg-card</code> so each themed surface is exercised.
      </p>
      <div className={styles.grid}>
        {DOC_TYPE_KEYS.map((docType) => (
          <div
            key={docType}
            className={styles.cell}
            data-testid={`big-glyph-cell-${docType}`}
          >
            <BigGlyph docType={docType} size={96} />
            <span className={styles.label}>
              <code>{docType}</code>
              <span className={styles.friendly}>{DOC_TYPE_LABELS[docType]}</span>
            </span>
          </div>
        ))}
      </div>
    </main>
  )
}
