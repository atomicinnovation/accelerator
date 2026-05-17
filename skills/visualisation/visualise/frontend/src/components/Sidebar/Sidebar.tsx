import { useMemo } from 'react'
import { Link, useRouterState } from '@tanstack/react-router'
import type { DocType, LibraryDocType, LibraryPhase } from '../../api/types'
import { useUnseenDocTypesContext } from '../../api/use-unseen-doc-types'
import { ActivityFeed } from '../ActivityFeed/ActivityFeed'
import styles from './Sidebar.module.css'

interface Props {
  docTypes: DocType[]
  phases: LibraryPhase[]
  templates: LibraryDocType | null
}

export function Sidebar({ docTypes, phases, templates }: Props) {
  const pathname = useRouterState({ select: s => s.location.pathname })
  const { unseenSet } = useUnseenDocTypesContext()
  // docTypes is kept around for affordances that need dirPath / inLifecycle,
  // but phase grouping comes from the server-driven `phases` prop.
  void docTypes

  return (
    <nav className={styles.sidebar} aria-label="Site navigation">
      {/* TEMPORARY: search row visible for design review. Wire behaviour
          in work item 0054 (search submission, `/` keybind focus). */}
      <div className={styles.searchRow}>
        <SearchIcon />
        <input
          type="search"
          aria-label="Search"
          placeholder="Search meta/..."
          className={styles.searchInput}
        />
        <kbd className={styles.kbd}>/</kbd>
      </div>

      <section aria-labelledby="library-heading" className={styles.section}>
        <Link
          to="/library"
          id="library-heading"
          className={`${styles.libraryHeading} ${styles.libraryHeadingClickable} ${pathname === '/library' ? styles.libraryHeadingActive : ''}`}
        >
          <span>LIBRARY</span>
          <span className={styles.libraryHeadingHint} aria-hidden="true">All</span>
        </Link>
        {phases.map(phase => (
          <section key={phase.id} className={styles.phase}>
            <h3 className={styles.phaseHeading}>{phase.label.toUpperCase()}</h3>
            <ul className={styles.list}>
              {phase.docTypes.map(dt => {
                const active =
                  pathname === `/library/${dt.id}` ||
                  pathname.startsWith(`/library/${dt.id}/`)
                const hasUnseen = unseenSet.has(dt.id)
                const linkLabel = hasUnseen
                  ? `${dt.label} (unseen changes)`
                  : dt.label
                return (
                  <li key={dt.id}>
                    <Link
                      to="/library/$type"
                      params={{ type: dt.id }}
                      aria-label={linkLabel}
                      title={hasUnseen ? 'Unseen changes since your last visit' : undefined}
                      className={`${styles.link} ${active ? styles.active : ''}`}
                    >
                      <span className={styles.label}>{dt.label}</span>
                      {hasUnseen && (
                        <span className={styles.dot} aria-hidden="true" />
                      )}
                      {dt.count > 0 && (
                        <span className={styles.count}>{dt.count}</span>
                      )}
                    </Link>
                  </li>
                )
              })}
            </ul>
          </section>
        ))}
      </section>

      <section aria-labelledby="views-heading" className={styles.section}>
        <h2 id="views-heading" className={styles.sectionHeading}>VIEWS</h2>
        <ul className={styles.list}>
          <li>
            <Link
              to="/kanban"
              className={`${styles.link} ${pathname === '/kanban' ? styles.active : ''}`}
            >
              <KanbanIcon />
              <span className={styles.label}>Kanban</span>
            </Link>
          </li>
          <li>
            <Link
              to="/lifecycle"
              className={`${styles.link} ${pathname.startsWith('/lifecycle') ? styles.active : ''}`}
            >
              <LifecycleIcon />
              <span className={styles.label}>Lifecycle</span>
            </Link>
          </li>
        </ul>
      </section>

      <ActivityFeed />

      {templates && (
        <section aria-labelledby="meta-heading" className={styles.section}>
          <h2 id="meta-heading" className={styles.sectionHeading}>META</h2>
          <ul className={styles.list}>
            <li>
              <Link
                to="/library/$type"
                params={{ type: 'templates' }}
                className={`${styles.link} ${
                  pathname === '/library/templates' ||
                  pathname.startsWith('/library/templates/')
                    ? styles.active
                    : ''
                }`}
              >
                <span className={styles.label}>{templates.label}</span>
              </Link>
            </li>
          </ul>
        </section>
      )}
    </nav>
  )
}

function SearchIcon() {
  return (
    <svg
      className={styles.searchIcon}
      width="16"
      height="16"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <circle cx="10.5" cy="10.5" r="6.5" fill="none" stroke="currentColor" strokeWidth="1.6" />
      <path d="M15.2 15.2 L20 20" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
    </svg>
  )
}

function KanbanIcon() {
  return (
    <svg
      className={styles.viewIcon}
      width="16"
      height="16"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <rect x="3" y="5" width="4" height="12" rx="1" fill="none" stroke="currentColor" strokeWidth="1.6" />
      <rect x="10" y="5" width="4" height="8" rx="1" fill="none" stroke="currentColor" strokeWidth="1.6" />
      <rect x="17" y="5" width="4" height="14" rx="1" fill="none" stroke="currentColor" strokeWidth="1.6" />
    </svg>
  )
}

function LifecycleIcon() {
  return (
    <svg
      className={styles.viewIcon}
      width="16"
      height="16"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <circle cx="12" cy="6" r="2.2" fill="none" stroke="currentColor" strokeWidth="1.6" />
      <circle cx="6" cy="18" r="2.2" fill="none" stroke="currentColor" strokeWidth="1.6" />
      <circle cx="18" cy="18" r="2.2" fill="none" stroke="currentColor" strokeWidth="1.6" />
      <path d="M11 8 L7 16 M13 8 L17 16 M8 18 L16 18" stroke="currentColor" strokeWidth="1.6" fill="none" />
    </svg>
  )
}
