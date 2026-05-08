import styles from './Brand.module.css'

export function Brand() {
  return (
    <div className={styles.brand}>
      <svg
        width="32"
        height="32"
        viewBox="0 0 40 40"
        aria-hidden="true"
        className={styles.mark}
      >
        <defs>
          <linearGradient id="hexg" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0%" stopColor="var(--ac-accent)" stopOpacity="0.6" />
            <stop offset="100%" stopColor="var(--ac-accent)" stopOpacity="1" />
          </linearGradient>
        </defs>
        <path
          d="M20 2 36 11v18L20 38 4 29V11z"
          fill="none"
          stroke="url(#hexg)"
          strokeWidth="2"
        />
        <circle cx="20" cy="20" r="3" fill="var(--ac-accent-2)" />
        <circle
          cx="20"
          cy="20"
          r="7.5"
          fill="none"
          stroke="var(--ac-accent)"
          strokeWidth="1"
          strokeOpacity="0.5"
        />
      </svg>
      <div className={styles.text}>
        <span className={styles.brandName}>Accelerator</span>
        <span className={styles.brandSub}>VISUALISER</span>
      </div>
    </div>
  )
}
