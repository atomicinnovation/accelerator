import { useOrigin } from '../../api/use-origin'
import styles from './OriginPill.module.css'

export function OriginPill() {
  const host = useOrigin()
  return (
    <div className={styles.originPill} aria-label="Server origin">
      <span className={styles.pulseDot} aria-hidden="true" />
      <span className={styles.originText}>{host}</span>
    </div>
  )
}
