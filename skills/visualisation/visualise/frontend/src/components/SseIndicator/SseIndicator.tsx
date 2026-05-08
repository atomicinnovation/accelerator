import { useDocEventsContext } from '../../api/use-doc-events'
import type { ConnectionState } from '../../api/reconnecting-event-source'
import styles from './SseIndicator.module.css'

const LABELS: Record<ConnectionState, string> = {
  open: 'SSE connection: open',
  reconnecting: 'SSE connection: reconnecting',
  connecting: 'SSE connection: connecting',
  closed: 'SSE connection: closed',
}

export function SseIndicator() {
  const { connectionState } = useDocEventsContext()
  const animated = connectionState === 'reconnecting'

  return (
    <span
      className={styles.sse}
      aria-label={LABELS[connectionState]}
      data-state={connectionState}
      data-animated={animated ? 'true' : undefined}
    >
      <svg
        width="12"
        height="12"
        viewBox="0 0 24 24"
        fill="none"
        aria-hidden="true"
        className={styles.icon}
      >
        <path
          d="M3 12h4l3-8 4 16 3-8h4"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
      <span className={styles.label}>SSE</span>
    </span>
  )
}
