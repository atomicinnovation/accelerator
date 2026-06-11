import type { ReactElement } from "react";
import type { Completeness, IndexEntry } from "../../api/types";
import { WorkItemCardPresentation } from "../kanban/WorkItemCardPresentation";
import styles from "./KanbanCardShowcase.module.css";

// Fully-static fixture so the drag-state screenshot frame is reproducible —
// `now` and `mtimeMs` are fixed (1h apart) so the relative time reads "1h ago"
// regardless of wall-clock, and the completeness set is constant.
const NOW = 1_700_000_000_000;
const SAMPLE_COMPLETENESS: Completeness = {
  hasWorkItem: true,
  hasResearch: true,
  hasPlan: true,
  hasPlanReview: false,
  hasValidation: false,
  hasPrDescription: false,
  hasPrReview: false,
  hasDecision: false,
  hasNotes: false,
  hasDesignInventory: false,
  hasDesignGap: false,
  present: ["work-items", "research", "plans"],
};
const SAMPLE_ENTRY: IndexEntry = {
  type: "work-items",
  path: "/x/meta/work/0086-kanban-drag-and-drop.md",
  relPath: "meta/work/0086-kanban-drag-and-drop.md",
  slug: "kanban-drag-and-drop",
  workItemId: "0086",
  title: "Kanban drag-and-drop",
  frontmatter: { kind: "feature", status: "in-progress" },
  frontmatterState: "parsed",
  workItemRefs: [],
  mtimeMs: NOW - 3_600_000,
  size: 1024,
  etag: "sha256-showcase",
  bodyPreview: "",
  completeness: SAMPLE_COMPLETENESS,
  linkedCount: 3,
  clusterKey: "0086",
};

const STATES = [
  { id: "resting", props: {} },
  { id: "dragging", props: { dragging: true } },
  { id: "overlay", props: { overlay: true } },
] as const;

export function KanbanCardShowcase(): ReactElement {
  // `data-testid="kanban-card-cell-<state>"` is the locator contract for
  // tests/visual-regression/kanban-card-showcase.spec.ts and
  // kanban-card-resolved-styles.spec.ts; any change here must update both specs.
  return (
    <main className={styles.root}>
      <h1>Kanban Card Showcase</h1>
      <p className={styles.note}>
        Static surface for the drag-state visual-regression oracle: resting,
        dragging (source-card lift), and the lifted overlay clone.
      </p>
      <div className={styles.grid}>
        {STATES.map((state) => (
          <div key={state.id} className={styles.cellWrap}>
            <span className={styles.label}>
              <code>{state.id}</code>
            </span>
            <div
              className={styles.cell}
              data-testid={`kanban-card-cell-${state.id}`}
            >
              <WorkItemCardPresentation
                entry={SAMPLE_ENTRY}
                now={NOW}
                {...state.props}
              />
            </div>
          </div>
        ))}
      </div>
    </main>
  );
}
