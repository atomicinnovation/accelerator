import type { ChipVariant } from '../components/Chip/Chip'
import { normaliseValue } from './normalise-value'

const GREEN = new Set(['approve'])
const AMBER = new Set(['revise'])
const RED = new Set(['requestchanges'])
// COMMENT and unknown values fall through to neutral. Result-vocabulary
// tokens (pass / partial / fail) live in result-variant.ts; validation
// emits `result:`, not `verdict:`, so they do not need to be handled here.

export const __SETS_FOR_TEST = [GREEN, AMBER, RED]

export function verdictToVariant(value: unknown): ChipVariant {
  const key = normaliseValue(value)
  if (GREEN.has(key)) return 'green'
  if (AMBER.has(key)) return 'amber'
  if (RED.has(key)) return 'red'
  return 'neutral'
}
