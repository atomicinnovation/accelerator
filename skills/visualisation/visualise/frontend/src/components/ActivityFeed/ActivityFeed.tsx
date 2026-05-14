import { useEffect, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { fetchActivity } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import { formatRelative } from '../../api/format'
import { useDocEventsContext } from '../../api/use-doc-events'
import type { ActivityEvent, SseEvent } from '../../api/types'
import { Glyph, isGlyphDocTypeKey } from '../Glyph/Glyph'
import styles from './ActivityFeed.module.css'

const LIMIT = 5

function basename(path: string): string {
  const ix = path.lastIndexOf('/')
  return ix >= 0 ? path.slice(ix + 1) : path
}

function activityRowId(event: ActivityEvent): string {
  return `${event.timestamp}|${event.path}|${event.action}`
}

function dedupeAndSortRows(rows: ActivityEvent[]): ActivityEvent[] {
  const seen = new Set<string>()
  const out: ActivityEvent[] = []
  for (const r of rows) {
    const k = activityRowId(r)
    if (seen.has(k)) continue
    seen.add(k)
    out.push(r)
  }
  // RFC-3339 ISO strings sort lexicographically as time.
  out.sort((a, b) => b.timestamp.localeCompare(a.timestamp))
  return out
}

export function ActivityFeed() {
  const { data: initial, isSuccess } = useQuery({
    queryKey: queryKeys.activity(LIMIT),
    queryFn: () => fetchActivity(LIMIT),
  })
  const { connectionState, subscribe } = useDocEventsContext()
  const [live, setLive] = useState<ActivityEvent[]>([])
  const [, setTick] = useState(0)

  useEffect(() => {
    const id = setInterval(() => setTick(t => t + 1), 60_000)
    return () => clearInterval(id)
  }, [])

  useEffect(() => {
    const unsub = subscribe((event: SseEvent) => {
      if (event.type !== 'doc-changed') return
      if (typeof event.action !== 'string' || typeof event.timestamp !== 'string'
          || (event.action as string) === '' || event.timestamp === '') {
        console.warn(
          '[activity-feed] dropping doc-changed event with missing/empty ' +
          'action or timestamp — likely cross-version dev drift OR a server ' +
          'serialisation regression',
          event,
        )
        return
      }
      setLive(prev => [
        {
          action: event.action,
          docType: event.docType,
          path: event.path,
          timestamp: event.timestamp,
        },
        ...prev,
      ].slice(0, LIMIT))
    })
    return unsub
  }, [subscribe])

  const rows = dedupeAndSortRows([...live, ...(initial ?? [])]).slice(0, LIMIT)
  const isLive = connectionState === 'open'
  const isEmptyHistory = isSuccess && (initial?.length ?? 0) === 0 && live.length === 0

  return (
    <section aria-labelledby="activity-heading" className={styles.section}>
      <h2 id="activity-heading" className={styles.heading}>
        <span>Activity</span>
        {isLive && (
          <span data-testid="activity-live-badge" className={styles.liveBadge}>LIVE</span>
        )}
      </h2>
      {isEmptyHistory ? (
        <p data-testid="activity-empty" className={styles.empty}>No recent activity</p>
      ) : (
        <ul className={styles.list}>
          {rows.map(r => (
            <li key={activityRowId(r)} className={styles.row}>
              {isGlyphDocTypeKey(r.docType) ? (
                <Glyph docType={r.docType} size={16} />
              ) : (
                <span />
              )}
              <span className={styles.action}>{r.action}</span>
              <span className={styles.meta}>
                {formatRelative(Date.parse(r.timestamp))} · {basename(r.path)}
              </span>
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}
