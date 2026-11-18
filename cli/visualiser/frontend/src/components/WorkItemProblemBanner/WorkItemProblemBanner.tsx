import { useQuery } from "@tanstack/react-query";
import { fetchWorkItemConfig } from "../../api/fetch";
import { queryKeys } from "../../api/query-keys";
import styles from "./WorkItemProblemBanner.module.css";

function CommandList({ commands }: { commands: string[] }) {
  return commands.map((command, index) => (
    <span key={command}>
      {index === 0 ? "" : index === commands.length - 1 ? " and " : ", "}
      <code>work {command}</code>
    </span>
  ));
}

/** Why work-item IDs cannot be trusted, with the remedy the server names,
 *  while they cannot. */
export function WorkItemProblemBanner() {
  const { data } = useQuery({
    queryKey: queryKeys.workItemConfig(),
    queryFn: fetchWorkItemConfig,
    select: (config) =>
      config.problem
        ? {
            ...config.problem,
            restartNeeded: !!config.restartNeeded,
            gatedCommands: config.gatedCommands ?? [],
          }
        : null,
  });
  if (!data) return null;
  return (
    <div className={styles.banner} role="alert">
      <p className={styles.lead}>
        Work-item IDs are switched off: nothing clusters or links by work-item
        ID, and{" "}
        {data.restartNeeded ? (
          <>
            every <code>work</code> command that reads{" "}
            <code>work.id_pattern</code> refuses with:
          </>
        ) : (
          <>
            <CommandList commands={data.gatedCommands} /> refuse with:
          </>
        )}
      </p>
      <p className={styles.problem}>{data.summary}</p>
      {data.items.length > 0 && (
        <ul className={styles.items}>
          {data.items.map((item) => (
            <li key={item}>{item}</li>
          ))}
        </ul>
      )}
      <p>
        {data.restartNeeded ? (
          <>
            Once the configuration is fixed, restart the visualiser: run{" "}
            <code>/accelerator:visualise stop</code>, then{" "}
            <code>/accelerator:visualise</code>, and open the new URL it gives.
          </>
        ) : (
          "This clears by itself once the work items are fixed."
        )}
      </p>
    </div>
  );
}
