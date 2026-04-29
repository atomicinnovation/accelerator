import { useServerInfo } from '../../api/use-server-info'
import styles from './SidebarFooter.module.css'

export function SidebarFooter() {
  const { data: serverInfo } = useServerInfo()
  const versionLabel = serverInfo?.version ? `Visualiser v${serverInfo.version}` : null

  return (
    <div className={styles.footer} aria-label="Sidebar footer">
      {versionLabel && <span className={styles.version}>{versionLabel}</span>}
    </div>
  )
}
