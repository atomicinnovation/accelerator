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
        width="14"
        height="10"
        viewBox="0 0 14 10"
        fill="none"
        aria-hidden="true"
      >
        <path
          d="M1 5 L3 2 L5 8 L7 1 L9 8 L11 2 L13 5"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
      <span>SSE</span>
    </span>
  )
}
