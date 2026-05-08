import { Link, useRouterState } from '@tanstack/react-router'
import type { DocType } from '../../api/types'
import styles from './Sidebar.module.css'

const VIEW_TYPES: Array<{ path: string; label: string }> = [
  { path: '/lifecycle', label: 'Lifecycle' },
  { path: '/kanban', label: 'Kanban' },
]

interface Props {
  docTypes: DocType[]
}

export function Sidebar({ docTypes }: Props) {
  const location = useRouterState({ select: s => s.location })

  // Partition on the server-provided `virtual` flag. Real doc types go
  // under Documents; virtual types (Templates today, future derived views)
  // go under Meta.
  const mainTypes = docTypes.filter(t => !t.virtual)
  const metaTypes = docTypes.filter(t => t.virtual)

  return (
    <nav className={styles.sidebar}>
      <section className={styles.section}>
        <h2 className={styles.sectionHeading}>Documents</h2>
        <ul className={styles.list}>
          {mainTypes.map(t => (
            <li key={t.key}>
              <Link
                to="/library/$type"
                params={{ type: t.key }}
                className={`${styles.link} ${
                  location.pathname.startsWith(`/library/${t.key}`) ? styles.active : ''
                }`}
              >
                {t.label}
              </Link>
            </li>
          ))}
        </ul>
      </section>

      <section className={styles.section}>
        <h2 className={styles.sectionHeading}>Views</h2>
        <ul className={styles.list}>
          {VIEW_TYPES.map(v => (
            <li key={v.path}>
              <Link
                to={v.path as never}
                className={`${styles.link} ${
                  location.pathname === v.path ? styles.active : ''
                }`}
              >
                {v.label}
              </Link>
            </li>
          ))}
        </ul>
      </section>

      <section className={`${styles.section} ${styles.meta}`}>
        <h2 className={styles.sectionHeading}>Meta</h2>
        <ul className={styles.list}>
          {metaTypes.map(t => (
            <li key={t.key}>
              <Link
                to="/library/$type"
                params={{ type: t.key }}
                className={`${styles.link} ${styles.muted} ${
                  location.pathname.startsWith(`/library/${t.key}`) ? styles.active : ''
                }`}
              >
                {t.label}
              </Link>
            </li>
          ))}
        </ul>
      </section>

    </nav>
  )
}
