import type { ConnectionState } from "../../api/reconnecting-event-source";
import { useDocEventsContext } from "../../api/use-doc-events";
import { Icon } from "../Icon/Icon";
import styles from "./SseIndicator.module.css";

const LABELS: Record<ConnectionState, string> = {
  open: "SSE connection: open",
  reconnecting: "SSE connection: reconnecting",
  connecting: "SSE connection: connecting",
  closed: "SSE connection: closed",
};

export function SseIndicator() {
  const { connectionState } = useDocEventsContext();
  const animated = connectionState === "reconnecting";

  return (
    <span
      className={styles.sse}
      role="img"
      aria-label={LABELS[connectionState]}
      data-state={connectionState}
      data-animated={animated ? "true" : undefined}
    >
      <Icon name="activity" size={12} className={styles.icon} />
      <span className={styles.label}>SSE</span>
    </span>
  );
}
