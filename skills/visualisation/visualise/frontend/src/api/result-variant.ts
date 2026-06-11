import type { ChipVariant } from "../components/Chip/Chip";
import { normaliseValue } from "./normalise-value";

const GREEN = new Set(["pass"]);
const AMBER = new Set(["partial"]);
const RED = new Set(["fail"]);

export const __SETS_FOR_TEST = [GREEN, AMBER, RED];

export function resultToVariant(value: unknown): ChipVariant {
  const key = normaliseValue(value);
  if (GREEN.has(key)) return "green";
  if (AMBER.has(key)) return "amber";
  if (RED.has(key)) return "red";
  return "neutral";
}
