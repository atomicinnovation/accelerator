import styles from "./WorkKindBadge.module.css";

type KindTone = "neutral" | "indigo" | "violet" | "amber" | "red";

// Mirrors the prototype's WORK_KIND_META (ui.jsx): known work-item kinds get a
// tinted tone + a title-cased label; any unrecognised kind falls back to a
// neutral pill showing the raw kind verbatim (so custom kinds still render).
const KIND_META: Record<string, { tone: KindTone; label: string }> = {
  epic: { tone: "violet", label: "Epic" },
  story: { tone: "indigo", label: "Story" },
  spike: { tone: "amber", label: "Spike" },
  task: { tone: "neutral", label: "Task" },
  bug: { tone: "red", label: "Bug" },
};

export interface WorkKindBadgeProps {
  kind: string;
}

export function WorkKindBadge({ kind }: WorkKindBadgeProps) {
  const meta = KIND_META[kind] ?? { tone: "neutral" as const, label: kind };
  return (
    <span className={styles.badge} data-tone={meta.tone}>
      {meta.label}
    </span>
  );
}
