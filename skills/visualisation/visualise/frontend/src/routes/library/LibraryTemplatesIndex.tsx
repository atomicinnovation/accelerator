import type { ReactNode } from 'react'
import { Link } from '@tanstack/react-router'
import { Page } from '../../components/Page/Page'
import { useQuery } from '@tanstack/react-query'
import { fetchTemplates } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import type { TemplateSummary, TemplateTierSource } from '../../api/types'
import { Chip } from '../../components/Chip/Chip'
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

  let content: ReactNode = <p>Loading…</p>
  if (isError) {
    content = (
      <p role="alert" className={styles.error}>
        Failed to load templates: {error instanceof Error ? error.message : String(error)}
      </p>
    )
  } else if (!isLoading && data) {
    content = (
      <ul className={styles.list}>
        {data.templates.map((t: TemplateSummary) => (
          <li key={t.name}>
            <Link to="/library/templates/$name" params={{ name: t.name }}>
              {t.name}
            </Link>
            <Chip variant="neutral">{TIER_LABELS[t.activeTier]}</Chip>
          </li>
        ))}
      </ul>
    )
  }

  return (
    <Page title="Templates" maxWidth="narrow">
      {content}
    </Page>
  )
}
