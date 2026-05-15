import type { ChipVariant } from '../components/Chip/Chip'

const GREEN = new Set(['done', 'complete', 'accepted', 'approved', 'implemented', 'final', 'shipped'])
const INDIGO = new Set(['inprogress', 'reviewed', 'ready', 'active', 'proposed', 'live'])
const AMBER = new Set(['approvewithchanges', 'approvewchanges', 'review', 'revised'])
const RED = new Set(['blocked', 'rejected', 'deprecated', 'superseded', 'abandoned'])

export const __SETS_FOR_TEST = [GREEN, INDIGO, AMBER, RED]

function normalise(value: unknown): string {
  if (typeof value !== 'string') return ''
  return value.trim().toLowerCase().replace(/[\s_\-/]+/g, '')
}

export function statusToChipVariant(value: unknown): ChipVariant {
  const key = normalise(value)
  if (GREEN.has(key)) return 'green'
  if (INDIGO.has(key)) return 'indigo'
  if (AMBER.has(key)) return 'amber'
  if (RED.has(key)) return 'red'
  return 'neutral'
}

export function isStatusKey(key: string): boolean {
  return key.trim().toLowerCase() === 'status'
}
