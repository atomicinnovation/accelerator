import type { ChipVariant } from "../components/Chip/Chip";
import { normaliseValue } from "./normalise-value";

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
