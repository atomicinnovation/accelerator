import { useEffect, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import { fetchActivity, fetchTypes } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import { formatRelative } from '../../api/format'
import { fileSlugFromRelPath } from '../../api/path-utils'
import { useDocEventsContext } from '../../api/use-doc-events'
import {
  DOC_TYPE_LABELS,
  type ActivityEvent,
  type DocTypeKey,
  type SseEvent,
} from '../../api/types'
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
  // Reuse the cached /api/types result that Sidebar/RootLayout already
  // primed, so doc-type labels match the rest of the UI (plural, server-
  // provided). Falls back to DOC_TYPE_LABELS if the cache is cold or the
  // key is unknown.
  const { data: docTypes } = useQuery({
    queryKey: queryKeys.types(),
    queryFn: fetchTypes,
  })
  const labelFor = (key: DocTypeKey): string =>
    docTypes?.find(t => t.key === key)?.label ?? DOC_TYPE_LABELS[key] ?? key

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
        <span>ACTIVITY</span>
        {isLive && (
          <span data-testid="activity-live-badge" className={styles.liveBadge}>LIVE</span>
        )}
      </h2>
      {isEmptyHistory ? (
        <p data-testid="activity-empty" className={styles.empty}>No recent activity</p>
      ) : (
        <ul className={styles.list}>
          {rows.map(r => {
            const fileSlug = fileSlugFromRelPath(r.path)
            const label = labelFor(r.docType)
            const file = basename(r.path)
            return (
              <li key={activityRowId(r)}>
                <Link
                  to="/library/$type/$fileSlug"
                  params={{ type: r.docType, fileSlug }}
                  className={styles.row}
                  aria-label={`${label}, ${r.action}, ${file}`}
                >
                  {isGlyphDocTypeKey(r.docType) ? (
                    <Glyph docType={r.docType} size={16} />
                  ) : (
                    <span className={styles.glyphSlot} />
                  )}
                  <div className={styles.body}>
                    <div className={styles.line1}>
                      <span className={styles.docType}>{label}</span>
                      <span className={styles.separator}> · </span>
                      <span className={styles.action}>{r.action}</span>
                    </div>
                    <div className={styles.line2}>
                      {file}
                      <span className={styles.separator}> · </span>
                      {formatRelative(Date.parse(r.timestamp))}
                    </div>
                  </div>
                </Link>
              </li>
            )
          })}
        </ul>
      )}
    </section>
  )
}
