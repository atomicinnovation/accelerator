import type { DocTypeKey } from "../../api/types";

/**
 * Pure label derivation: explains why an entry joined its cluster,
 * shown as a small "clustered via" debug tag on the cluster detail
 * view. The label is determined deterministically from the entry's
 * type and the cluster's cluster_key — no frontmatter inspection.
 */
export function clusterViaLabel(
  entry: { type: DocTypeKey; clusterKey: string | null },
  cluster: { clusterKey: string | null },
): string {
  if (cluster.clusterKey === null) return "clustered via: slug";
  const wid = `work-item:${cluster.clusterKey}`;
  switch (entry.type) {
    case "work-items":
    case "plans":
    case "research":
    case "pr-descriptions":
      return `clustered via: parent → ${wid}`;
    case "work-item-reviews":
      return `clustered via: target → ${wid}`;
    case "plan-reviews":
    case "validations":
      return "clustered via: target → plan → parent";
    case "pr-reviews":
      return "clustered via: target → pr-description → parent";
    default:
      return "clustered via: slug";
  }
}
