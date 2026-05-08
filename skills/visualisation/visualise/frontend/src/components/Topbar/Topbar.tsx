import { Brand } from '../Brand/Brand'
import { Breadcrumbs } from '../Breadcrumbs/Breadcrumbs'
import { OriginPill } from '../OriginPill/OriginPill'
import { SseIndicator } from '../SseIndicator/SseIndicator'
import { ThemeToggle } from '../ThemeToggle/ThemeToggle'
import { FontModeToggle } from '../FontModeToggle/FontModeToggle'
import styles from './Topbar.module.css'

export function Topbar() {
  return (
    <header className={styles.topbar}>
      <Brand />
      <div className={styles.divider} />
      <Breadcrumbs />
      <div className={styles.spacer} />
      <OriginPill />
      <SseIndicator />
      <div className={styles.toggleGroup}>
        <div className={styles.slot} data-slot="theme-toggle">
          <ThemeToggle />
        </div>
        <div className={styles.slot} data-slot="font-mode-toggle">
          <FontModeToggle />
        </div>
      </div>
    </header>
  )
}
