import type { ChipVariant } from "../components/Chip/Chip";
import { normaliseValue } from "./normalise-value";
import type { DocTypeKey } from "./types";

// These sets are a shared, doc-type-agnostic status lexicon (matching the
// prototype's StatusBadge) — they colour the status column of EVERY doc type,
// not just one. `resolved`/`monitoring` were added for the RCA status verbs
// (0110) but apply wherever those words appear; they are not RCA-private and
// should not be "cleaned up" as such.
const GREEN = new Set([
  "done",
  "complete",
  "accepted",
  "approved",
  "implemented",
  "final",
  "shipped",
  "resolved",
]);
const INDIGO = new Set([
  "inprogress",
  "reviewed",
  "ready",
  "active",
  "proposed",
  "live",
  "monitoring",
]);
const AMBER = new Set([
  "approvewithchanges",
  "approvewchanges",
  "review",
  "revised",
]);
const RED = new Set([
  "blocked",
  "rejected",
  "deprecated",
  "superseded",
  "abandoned",
]);

export const __SETS_FOR_TEST = [GREEN, INDIGO, AMBER, RED];

export function statusToVariant(value: unknown): ChipVariant {
  const key = normaliseValue(value);
  if (GREEN.has(key)) return "green";
  if (INDIGO.has(key)) return "indigo";
  if (AMBER.has(key)) return "amber";
  if (RED.has(key)) return "red";
  return "neutral";
}

// The topic-research lifecycle states, in order. Mapped to a dedicated ordinal
// `lifecycle-*` chip ramp rather than the shared ok/warn/err lexicon, because
// `complete` already belongs to the shared GREEN set — a value-only map could
// not route these without recolouring every type's `complete` chip. Kept in
// step with the Rust manifest `status_vocab` by the drift fixture that
// `status-variant.test.ts` reads.
const LIFECYCLE_TO_VARIANT: Record<string, ChipVariant> = {
  briefed: "lifecycle-briefed",
  outlined: "lifecycle-outlined",
  researching: "lifecycle-researching",
  synthesised: "lifecycle-synthesised",
  complete: "lifecycle-complete",
};

export function lifecycleToVariant(value: unknown): ChipVariant {
  return LIFECYCLE_TO_VARIANT[normaliseValue(value)] ?? "neutral";
}

// Which chip scale each doc type uses. Centralised so the "lifecycle types"
// decision lives in one map — a future lifecycle-classified type onboards by
// one entry rather than a per-site `type === …` conditional.
const CHIP_SCALE: Record<DocTypeKey, "lifecycle" | "semantic"> = {
  decisions: "semantic",
  "work-items": "semantic",
  plans: "semantic",
  "codebase-research": "semantic",
  "plan-reviews": "semantic",
  "pr-reviews": "semantic",
  "work-item-reviews": "semantic",
  validations: "semantic",
  notes: "semantic",
  "pr-descriptions": "semantic",
  "design-gaps": "semantic",
  "design-inventories": "semantic",
  "topic-research": "lifecycle",
  "root-cause-analyses": "semantic",
  templates: "semantic",
};

/** Resolve a status chip variant for a doc type: lifecycle-classified types
 *  use the ordinal `lifecycle-*` ramp; every other type keeps the shared
 *  semantic lexicon. */
export function chipVariantFor(type: DocTypeKey, status: unknown): ChipVariant {
  return CHIP_SCALE[type] === "lifecycle"
    ? lifecycleToVariant(status)
    : statusToVariant(status);
}
