import { useQuery } from "@tanstack/react-query";
import { fetchWorkItemConfig } from "../../api/fetch";
import { queryKeys } from "../../api/query-keys";
import styles from "./WorkItemProblemBanner.module.css";

/** Why work-item IDs cannot be trusted, with the remedy the server names,
 *  while they cannot. */
export function WorkItemProblemBanner() {
  const { data } = useQuery({
    queryKey: queryKeys.workItemProblem(),
    queryFn: fetchWorkItemConfig,
    select: (config) => config.problem ?? null,
  });
  if (!data) return null;
  return (
    <div className={styles.banner} role="alert">
      {data}
    </div>
  );
}
