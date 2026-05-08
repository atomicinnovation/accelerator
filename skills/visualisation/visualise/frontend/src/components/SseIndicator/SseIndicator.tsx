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

  // `data-animated={animated ? 'true' : undefined}` — React omits
  // attributes whose value is `undefined`, so the attribute is
  // absent from the DOM in non-reconnecting states.
  return (
    <span
      className={styles.sse}
      aria-label={LABELS[connectionState]}
      data-state={connectionState}
      data-animated={animated ? 'true' : undefined}
    />
  )
}
