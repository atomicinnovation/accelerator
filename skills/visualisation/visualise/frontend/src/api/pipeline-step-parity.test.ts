import { describe, expect, it } from "vitest";
import {
  LIFECYCLE_PIPELINE_STEPS,
  LONG_TAIL_PIPELINE_STEPS,
  WORKFLOW_PIPELINE_STEPS,
} from "./types";

/** Cross-language parity anchor: the Rust STAGE_PUSH_ORDER literal in
 *  `server/src/clusters.rs` — the canonical `present` superset. The frontend
 *  renders every stage EXCEPT `decisions`, which still clusters and is pushed
 *  into `present` server-side but is intentionally not a lifecycle pipeline
 *  stage on any surface. Any reordering must update both sides. */
const CANONICAL_PRESENT_ORDER = [
  "work-items",
  "research",
  "plans",
  "plan-reviews",
  "validations",
  "pr-descriptions",
  "pr-reviews",
  "decisions",
  "notes",
  "design-inventories",
  "design-gaps",
] as const;

/** The frontend pipeline omits `decisions` (backend-only present key). */
const RENDERED_PRESENT_ORDER = CANONICAL_PRESENT_ORDER.filter(
  (d) => d !== "decisions",
);

describe("LIFECYCLE_PIPELINE_STEPS parity", () => {
  it("matches the canonical present ordering minus the backend-only decisions key", () => {
    const order = LIFECYCLE_PIPELINE_STEPS.map((s) => s.docType);
    expect(order).toEqual(RENDERED_PRESENT_ORDER);
  });

  it("places workflow steps before long-tail steps", () => {
    const order = WORKFLOW_PIPELINE_STEPS.map((s) => s.docType).concat(
      LONG_TAIL_PIPELINE_STEPS.map((s) => s.docType),
    );
    expect(order).toEqual(RENDERED_PRESENT_ORDER);
  });
});
