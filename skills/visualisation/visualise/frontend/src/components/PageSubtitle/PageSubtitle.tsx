import type { ReactNode } from 'react'
import styles from './PageSubtitle.module.css'

export interface PageSubtitleProps {
  title: string
  children?: ReactNode
}

export function PageSubtitle({ title, children }: PageSubtitleProps) {
  const hasChildren = children !== undefined && children !== null && children !== false
  return (
    <header className={styles.pagehead}>
      <h1 className={styles.title}>{title}</h1>
      {hasChildren && (
        <div className={styles.subtitle} data-slot="subtitle">{children}</div>
      )}
    </header>
  )
}
