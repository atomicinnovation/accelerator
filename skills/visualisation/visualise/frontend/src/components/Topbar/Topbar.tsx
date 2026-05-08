import { Brand } from '../Brand/Brand'
import { Breadcrumbs } from '../Breadcrumbs/Breadcrumbs'
import { OriginPill } from '../OriginPill/OriginPill'
import { SseIndicator } from '../SseIndicator/SseIndicator'
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
      <div className={styles.slot} data-slot="theme-toggle" />
      <div className={styles.slot} data-slot="font-mode-toggle" />
    </header>
  )
}
