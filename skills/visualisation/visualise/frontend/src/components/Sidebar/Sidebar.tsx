import { useMemo } from 'react'
import { Link, useRouterState } from '@tanstack/react-router'
import { PHASE_DOC_TYPES, type DocType, type DocTypeKey } from '../../api/types'
import { useUnseenDocTypesContext } from '../../api/use-unseen-doc-types'
import styles from './Sidebar.module.css'

interface Props {
  docTypes: DocType[]
}

export function Sidebar({ docTypes }: Props) {
  const pathname = useRouterState({ select: s => s.location.pathname })
  const { unseenSet } = useUnseenDocTypesContext()
  const byKey = useMemo(
    () => new Map(docTypes.map(t => [t.key, t])),
    [docTypes],
  )
  const templates = byKey.get('templates')

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
        <h2 id="library-heading" className={styles.libraryHeading}>LIBRARY</h2>
        {PHASE_DOC_TYPES.map(phase => (
          <section key={phase.phase} className={styles.phase}>
            <h3 className={styles.phaseHeading}>{phase.label.toUpperCase()}</h3>
            <ul className={styles.list}>
              {phase.docTypes.map((key: DocTypeKey) => {
                const t = byKey.get(key)
                if (!t) {
                  if (import.meta.env.DEV) {
                    console.warn(
                      `[Sidebar] PHASE_DOC_TYPES key '${key}' missing from /api/types payload — nav item will not render.`,
                    )
                  }
                  return null
                }
                const active =
                  pathname === `/library/${key}` ||
                  pathname.startsWith(`/library/${key}/`)
                const hasUnseen = unseenSet.has(key)
                const linkLabel = hasUnseen
                  ? `${t.label} (unseen changes)`
                  : t.label
                return (
                  <li key={key}>
                    <Link
                      to="/library/$type"
                      params={{ type: key }}
                      aria-label={linkLabel}
                      title={hasUnseen ? 'Unseen changes since your last visit' : undefined}
                      className={`${styles.link} ${active ? styles.active : ''}`}
                    >
                      <span className={styles.label}>{t.label}</span>
                      {hasUnseen && (
                        <span className={styles.dot} aria-hidden="true" />
                      )}
                      {t.count !== undefined && t.count > 0 && (
                        <span className={styles.count}>{t.count}</span>
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
