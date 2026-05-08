import {
  useMatches,
  useRouter,
} from '@tanstack/react-router'
import { rootRouteId } from '@tanstack/router-core'
import type { MouseEvent } from 'react'
import styles from './Breadcrumbs.module.css'

type Match = ReturnType<typeof useMatches>[number]
type CrumbMatch = Omit<Match, 'loaderData'> & {
  loaderData: { crumb: string }
}

function hasCrumb(m: Match): m is CrumbMatch {
  const ld = m.loaderData as { crumb?: unknown } | null | undefined
  return typeof ld?.crumb === 'string' && ld.crumb.length > 0
}

export function Breadcrumbs() {
  const matches = useMatches()
  const router = useRouter()

  if (import.meta.env.DEV) {
    for (const m of matches) {
      if (m.status === 'success' && m.routeId !== rootRouteId && !hasCrumb(m)) {
        console.warn(
          `[Breadcrumbs] Route ${m.routeId} has no loaderData.crumb. ` +
            `Did you forget to use withCrumb()?`,
        )
      }
    }
  }

  const crumbs = matches.filter(hasCrumb) as unknown as CrumbMatch[]
  if (crumbs.length === 0) return null

  const handleClick = (pathname: string) => (e: MouseEvent) => {
    // Allow modifier-click (cmd/ctrl/middle/shift) to open in a new
    // tab via the native href; intercept plain left-click for SPA navigation.
    if (e.metaKey || e.ctrlKey || e.shiftKey || e.button !== 0) return
    e.preventDefault()
    router.navigate({ to: pathname as never })
  }

  return (
    <nav className={styles.breadcrumbs} aria-label="Breadcrumb">
      <ol className={styles.list}>
        {crumbs.map((m, i) => {
          const isLast = i === crumbs.length - 1
          return (
            <li key={m.id} className={styles.crumb}>
              {i > 0 && (
                <span className={styles.sep} aria-hidden="true">
                  ›
                </span>
              )}
              {isLast ? (
                <span className={styles.current} aria-current="page">{m.loaderData.crumb}</span>
              ) : (
                <a
                  href={m.pathname}
                  onClick={handleClick(m.pathname)}
                  className={styles.link}
                >
                  {m.loaderData.crumb}
                </a>
              )}
            </li>
          )
        })}
      </ol>
    </nav>
  )
}
