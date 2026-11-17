import { useQuery } from "@tanstack/react-query";
import { fetchWorkItemConfig } from "../../api/fetch";
import { queryKeys } from "../../api/query-keys";
import styles from "./WorkItemProblemBanner.module.css";

/** Why work-item IDs cannot be trusted, with the remedy the server names,
 *  while they cannot. */
export function WorkItemProblemBanner() {
  const { data } = useQuery({
    queryKey: queryKeys.workItemConfig(),
    queryFn: fetchWorkItemConfig,
    select: (config) =>
      config.problem
        ? { problem: config.problem, restartNeeded: !!config.restartNeeded }
        : null,
  });
  if (!data) return null;
  return (
    <div className={styles.banner} role="alert">
      <p className={styles.lead}>
        Work-item IDs are switched off: nothing clusters or links by work-item
        ID, and the <code>work</code> commands refuse with:
      </p>
      <p className={styles.problem}>{data.problem}</p>
      <p>
        {data.restartNeeded
          ? "Restart the visualiser once the configuration is fixed."
          : "This clears by itself once the work items are fixed."}
      </p>
    </div>
  );
}
