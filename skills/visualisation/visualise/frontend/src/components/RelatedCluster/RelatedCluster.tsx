import { Link } from "@tanstack/react-router";
import { formatMtime, pluralise } from "../../api/format";
import type { LifecycleCluster } from "../../api/types";
import styles from "./RelatedCluster.module.css";

/** Lifecycle dot-grid mark (unframed, muted) — signals the card navigates
 *  into the pipeline view. Mirrors the prototype's `Icon name="lifecycle"`. */
function LifecycleMark() {
  return (
    <svg
      className={styles.mark}
      width={16}
      height={16}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <circle cx="6" cy="6" r="2" />
      <circle cx="18" cy="6" r="2" />
      <circle cx="6" cy="18" r="2" />
      <circle cx="18" cy="18" r="2" />
      <path d="M8 6h8M6 8v8M18 8v8M8 18h8" />
    </svg>
  );
}

function Chevron() {
  return (
    <svg
      className={styles.chevron}
      width={14}
      height={14}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="m9 6 6 6-6 6" />
    </svg>
  );
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
