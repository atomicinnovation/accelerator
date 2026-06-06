import { Link } from '@tanstack/react-router'
import { formatMtime, pluralise } from '../../api/format'
import type { LifecycleCluster } from '../../api/types'
import styles from './RelatedCluster.module.css'

/** Detail-page aside block linking to the document's lifecycle pipeline
 *  view. Renders the cluster title and a `<n> artifacts · <updated>` meta
 *  row, navigating to `/lifecycle/<slug>` on click. */
export function RelatedCluster({ cluster }: { cluster: LifecycleCluster }) {
  return (
    <Link
      to="/lifecycle/$slug"
      params={{ slug: cluster.slug }}
      className={styles.link}
    >
      <span className={styles.title}>{cluster.title}</span>
      <span className={styles.meta}>
        {pluralise(cluster.entries.length, 'artifact')}
        {' · '}
        <time>{formatMtime(cluster.lastChangedMs)}</time>
      </span>
    </Link>
  )
}
