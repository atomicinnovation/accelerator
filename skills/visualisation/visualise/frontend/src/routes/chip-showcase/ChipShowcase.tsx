import type { ReactElement } from 'react'
import { Chip, type ChipVariant, type ChipSize } from '../../components/Chip/Chip'
import styles from './ChipShowcase.module.css'

const VARIANTS: ReadonlyArray<ChipVariant> = ['neutral', 'indigo', 'green', 'amber', 'red', 'violet']
const SIZES: ReadonlyArray<ChipSize> = ['sm', 'md']

export function ChipShowcase(): ReactElement {
  // `data-testid="chip-cell-<variant>-<size>"` is the locator contract for
  // tests/visual-regression/chip-showcase.spec.ts and chip-resolved-colours.spec.ts;
  // any change here must update both specs.
  return (
    <main className={styles.root}>
      <h1>Chip Showcase</h1>
      <p className={styles.note}>
        Toggle <code>document.documentElement.dataset.theme</code> between{' '}
        <code>light</code> and <code>dark</code> in dev tools to compare.
      </p>
      <div className={styles.grid}>
        <div className={styles.headerRow}>
          <span className={styles.headerLabel}>variant</span>
          {SIZES.map((s) => (
            <span key={s} className={styles.headerCell}>{s}</span>
          ))}
        </div>
        {VARIANTS.map((variant) => (
          <div key={variant} className={styles.row}>
            <span className={styles.label}>
              <code>{variant}</code>
            </span>
            {SIZES.map((size) => (
              <span
                key={size}
                className={styles.cell}
                data-testid={`chip-cell-${variant}-${size}`}
              >
                <Chip variant={variant} size={size}>{variant}</Chip>
              </span>
            ))}
          </div>
        ))}
      </div>
    </main>
  )
}
