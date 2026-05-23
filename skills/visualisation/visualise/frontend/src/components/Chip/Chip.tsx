import type { ReactNode } from 'react'
import styles from './Chip.module.css'

export type ChipVariant = 'neutral' | 'indigo' | 'green' | 'amber' | 'red' | 'violet'

export type ChipSize = 'sm' | 'md'

export interface ChipProps {
  variant: ChipVariant
  size?: ChipSize
  leading?: ReactNode
  'aria-label'?: string
  'data-testid'?: string
  children: ReactNode
}

export function Chip({ variant, size = 'sm', leading, children, ...rest }: ChipProps) {
  const hasLeading = leading !== undefined && leading !== null && leading !== false
  return (
    <span
      className={styles.chip}
      data-variant={variant}
      data-size={size}
      aria-label={rest['aria-label']}
      data-testid={rest['data-testid']}
    >
      {hasLeading && (
        <span className={styles.leading} data-slot="leading">{leading}</span>
      )}
      <span className={styles.label}>{children}</span>
    </span>
  )
}
