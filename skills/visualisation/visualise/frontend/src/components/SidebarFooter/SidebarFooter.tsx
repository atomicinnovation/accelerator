import { useServerInfo } from '../../api/use-server-info'
import { useDocEventsContext } from '../../api/use-doc-events'
import styles from './SidebarFooter.module.css'

export function SidebarFooter() {
  const { data: serverInfo } = useServerInfo()
  const { connectionState, justReconnected } = useDocEventsContext()
  const versionLabel = serverInfo?.version ? `Visualiser v${serverInfo.version}` : null

  const showReconnecting = connectionState === 'reconnecting'
  const showReconnected = justReconnected && connectionState === 'open'

  return (
    <div className={styles.footer} aria-label="Sidebar footer">
      {showReconnecting && (
        <span className={styles.reconnecting} role="status">
          Reconnecting…
        </span>
      )}
      {showReconnected && !showReconnecting && (
        <span className={styles.reconnected} role="status">
          Reconnected — refreshing
        </span>
      )}
      {versionLabel && <span className={styles.version}>{versionLabel}</span>}
    </div>
  )
}
