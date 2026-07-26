import type { Completeness } from "../../api/types";
import {
  WORKFLOW_PIPELINE_STEPS,
  workflowStagesComplete,
} from "../../api/types";
import styles from "./PipelineMini.module.css";

interface Props {
  completeness: Completeness;
}

export function PipelineMini({ completeness }: Props) {
  const present = new Set(completeness.present);
  const complete = workflowStagesComplete(completeness.present);
  return (
    <ol
      className={`${styles.row} ac-stagedots`}
      aria-label={`Lifecycle pipeline, ${complete} of ${WORKFLOW_PIPELINE_STEPS.length} stages complete`}
    >
      {WORKFLOW_PIPELINE_STEPS.map((step) => {
        const active = present.has(step.docType);
        const accent = `var(--ac-stage-${step.docType})`;
        return (
          <li
            key={step.docType}
            className={`${styles.dot} ac-stagedots__dot`}
            data-stage={step.docType}
            data-active={String(active)}
            style={
              active ? { background: accent, borderColor: accent } : undefined
            }
          />
        );
      })}
    </ol>
  );
}
