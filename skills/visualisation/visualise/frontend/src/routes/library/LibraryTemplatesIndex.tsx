import { Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { fetchTemplates } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import type { TemplateSummary, TemplateTierSource } from '../../api/types'
import styles from './LibraryTemplatesIndex.module.css'

const TIER_LABELS: Record<TemplateTierSource, string> = {
  'plugin-default':  'Plugin default',
  'user-override':   'User override',
  'config-override': 'Config override',
}

export function LibraryTemplatesIndex() {
  const { data, isLoading, isError, error } = useQuery({
    queryKey: queryKeys.templates(),
    queryFn: fetchTemplates,
  })

  if (isError) {
    return (
      <p role="alert" className={styles.error}>
        Failed to load templates: {error instanceof Error ? error.message : String(error)}
      </p>
    )
  }
  if (isLoading || !data) return <p>Loading…</p>

  return (
    <div className={styles.container}>
      <h1>Templates</h1>
      <ul className={styles.list}>
        {data.templates.map((t: TemplateSummary) => (
          <li key={t.name}>
            <Link to="/library/templates/$name" params={{ name: t.name }}>
              {t.name}
            </Link>
            <span className={styles.active}>{TIER_LABELS[t.activeTier]}</span>
          </li>
        ))}
      </ul>
    </div>
  )
}
