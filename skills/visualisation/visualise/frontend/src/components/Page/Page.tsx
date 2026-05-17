import type { ReactNode } from 'react'
import styles from './Page.module.css'

export interface PageProps {
  eyebrow?: ReactNode
  title: ReactNode
  subtitle?: ReactNode
  actions?: ReactNode
  maxWidth?: 'default' | 'narrow'
  children: ReactNode
}

export function Page({
  eyebrow,
  title,
  subtitle,
  actions,
  maxWidth = 'default',
  children,
}: PageProps) {
  const className =
    maxWidth === 'narrow' ? `${styles.page} ${styles.narrow}` : styles.page
  return (
    <section className={className}>
      <header className={styles.header}>
        {eyebrow !== undefined && (
          <div className={styles.eyebrow} data-slot="eyebrow">{eyebrow}</div>
        )}
        <div className={styles.headerTopRow}>
          <div>
            <h1 className={styles.title}>{title}</h1>
            {subtitle !== undefined && (
              <div className={styles.subtitle} data-slot="subtitle">{subtitle}</div>
            )}
          </div>
          {actions !== undefined && (
            <div className={styles.actions} data-slot="actions">{actions}</div>
          )}
        </div>
      </header>
      <hr className={styles.divider} />
      <div className={styles.content}>{children}</div>
    </section>
  )
}
