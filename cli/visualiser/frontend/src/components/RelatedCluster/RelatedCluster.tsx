import { Link } from "@tanstack/react-router";
import { formatMtime, pluralise } from "../../api/format";
import type { LifecycleCluster } from "../../api/types";
import { Icon } from "../Icon/Icon";
import styles from "./RelatedCluster.module.css";

/** Lifecycle dot-grid mark (unframed, muted) — signals the card navigates
 *  into the pipeline view. Mirrors the prototype's `Icon name="lifecycle"`. */
function LifecycleMark() {
  return <Icon name="lifecycle" size={16} className={styles.mark} />;
}

function Chevron() {
  return <Icon name="chevron-right" size={14} className={styles.chevron} />;
}

/** Detail-page aside block linking to the document's lifecycle pipeline
 *  view. Rendered as a bordered card (prototype `.ac-related__item` cluster
 *  affordance): lifecycle mark + title + `<n> artifacts · <updated>` meta +
 *  chevron, navigating to `/lifecycle/<slug>` on click. */
export function RelatedCluster({ cluster }: { cluster: LifecycleCluster }) {
  return (
    <Link
      to="/lifecycle/$slug"
      params={{ slug: cluster.slug }}
      className={styles.card}
    >
      <LifecycleMark />
      <span className={styles.body}>
        <span className={styles.title}>{cluster.title}</span>
        <span className={styles.meta}>
          {pluralise(cluster.entries.length, "artifact")}
          {" · "}
          <time>{formatMtime(cluster.lastChangedMs)}</time>
        </span>
      </span>
      <Chevron />
    </Link>
  );
}
