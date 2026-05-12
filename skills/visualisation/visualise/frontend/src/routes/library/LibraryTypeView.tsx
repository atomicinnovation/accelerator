import { useMemo, useState } from 'react'
import { Link, Outlet, useParams } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { fetchDocs } from '../../api/fetch'
import { formatMtime } from '../../api/format'
import { queryKeys } from '../../api/query-keys'
import type { IndexEntry, DocTypeKey } from '../../api/types'
import { isDocTypeKey } from '../../api/types'
import { useMarkDocTypeSeen } from '../../api/use-unseen-doc-types'
import { fileSlugFromRelPath } from '../../api/path-utils'
import styles from './LibraryTypeView.module.css'

type SortKey = 'title' | 'slug' | 'status' | 'mtime'
type SortDir = 'asc' | 'desc'

/** Extract the status-cell's displayed value, matching the fallback
 *  chain in the rendered cell. Sort contract: clicking a column header
 *  orders rows by what the user sees in that column. */
function statusCellValue(entry: IndexEntry): string {
  const fm = entry.frontmatter as Record<string, unknown> | null
  return String(fm?.status ?? fm?.date ?? '')
}

function sortEntries(entries: IndexEntry[], key: SortKey, dir: SortDir): IndexEntry[] {
  return [...entries].sort((a, b) => {
    let av: string | number, bv: string | number
    if (key === 'title') { av = a.title; bv = b.title }
    else if (key === 'slug') { av = a.slug ?? ''; bv = b.slug ?? '' }
    else if (key === 'status') {
      av = statusCellValue(a)
      bv = statusCellValue(b)
    }
    else { av = a.mtimeMs; bv = b.mtimeMs }
    if (av < bv) return dir === 'asc' ? -1 : 1
    if (av > bv) return dir === 'asc' ? 1 : -1
    return 0
  })
}

interface Props { type?: DocTypeKey }

export function LibraryTypeView({ type: propType }: Props) {
  // Prop takes precedence (for tests that render the component directly);
  // otherwise read from the router. The route's `parseParams` (see
  // router.ts) has already narrowed the URL param to DocTypeKey — the
  // `isDocTypeKey` check below is belt-and-braces for the prop path.
  const params = useParams({ strict: false }) as { type?: string; fileSlug?: string }
  const rawType = propType ?? params.type

  const [sortKey, setSortKey] = useState<SortKey>('mtime')
  const [sortDir, setSortDir] = useState<SortDir>('desc')

  // Narrowed only when rawType passes the type guard; undefined otherwise.
  const type: DocTypeKey | undefined =
    rawType && isDocTypeKey(rawType) ? rawType : undefined
  const hasFileSlug = Boolean(params.fileSlug)

  // Pass undefined on child-doc paths so the unseen dot on the parent
  // type persists — the user has not actually seen the list view.
  useMarkDocTypeSeen(hasFileSlug ? undefined : type)

  // Call useQuery unconditionally (Rules of Hooks). When `type` is invalid
  // we disable the query and render an error; the key uses a sentinel so
  // the invalid case does not share a cache entry with a real type.
  const { data: entries = [], isLoading, isError, error } = useQuery({
    queryKey: type ? queryKeys.docs(type) : ['docs', '__invalid__'] as const,
    queryFn: () => fetchDocs(type!),
    enabled: type !== undefined,
  })

  function toggleSort(key: SortKey) {
    if (sortKey === key) setSortDir(d => d === 'asc' ? 'desc' : 'asc')
    else { setSortKey(key); setSortDir('asc') }
  }

  // Memoise BEFORE any conditional early returns — Rules of Hooks require
  // every hook to run in the same order on every render.
  const sorted = useMemo(
    () => sortEntries(entries, sortKey, sortDir),
    [entries, sortKey, sortDir],
  )

  const ariaSortFor = (key: SortKey): 'ascending' | 'descending' | 'none' =>
    sortKey === key ? (sortDir === 'asc' ? 'ascending' : 'descending') : 'none'

  // When a child route (document detail) is active, delegate rendering to it.
  // LibraryDocView is a child of this route in the route tree, so it only
  // renders if we provide <Outlet />.
  if (params.fileSlug) return <Outlet />

  if (type === undefined) {
    return <p role="alert">Unknown doc type: {String(rawType)}</p>
  }
  if (isLoading) return <p>Loading…</p>
  if (isError) {
    return (
      <p role="alert" className={styles.error}>
        Failed to load documents: {error instanceof Error ? error.message : String(error)}
      </p>
    )
  }

  return (
    <div className={styles.container}>
      <table className={styles.table}>
        <thead>
          <tr>
            <SortHeader label="Title"    skey="title"  ariaSort={ariaSortFor('title')}  onToggle={toggleSort} current={sortKey} dir={sortDir} />
            <SortHeader label="Status"   skey="status" ariaSort={ariaSortFor('status')} onToggle={toggleSort} current={sortKey} dir={sortDir} />
            <SortHeader label="Slug"     skey="slug"   ariaSort={ariaSortFor('slug')}   onToggle={toggleSort} current={sortKey} dir={sortDir} />
            <SortHeader label="Modified" skey="mtime"  ariaSort={ariaSortFor('mtime')}  onToggle={toggleSort} current={sortKey} dir={sortDir} />
          </tr>
        </thead>
        <tbody>
          {sorted.map(entry => (
            <tr key={entry.relPath}>
              <td>
                <Link to="/library/$type/$fileSlug" params={{ type, fileSlug: entry.slug ?? fileSlugFromRelPath(entry.relPath) }}>
                  {entry.title}
                </Link>
              </td>
              <td>
                <span className={styles.badge}>
                  {statusCellValue(entry) || '—'}
                </span>
              </td>
              <td className={styles.slug}>{entry.slug ?? '—'}</td>
              <td className={styles.mtime}>
                {formatMtime(entry.mtimeMs)}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {entries.length === 0 && <p className={styles.empty}>No documents found.</p>}
    </div>
  )
}

/** Sortable column header. `<button>` semantics give keyboard users
 *  Enter/Space to toggle; `aria-sort` communicates current state to
 *  assistive technologies. */
function SortHeader({
  label, skey, ariaSort, current, dir, onToggle,
}: {
  label: string
  skey: SortKey
  ariaSort: 'ascending' | 'descending' | 'none'
  current: SortKey
  dir: SortDir
  onToggle: (k: SortKey) => void
}) {
  const isActive = current === skey
  const arrow = isActive ? (dir === 'asc' ? ' ▲' : ' ▼') : ''
  return (
    <th aria-sort={ariaSort}>
      <button
        type="button"
        className={styles.sortButton}
        onClick={() => onToggle(skey)}
      >
        {label}{arrow}
      </button>
    </th>
  )
}

