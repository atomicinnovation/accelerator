import { describe, it, expect } from 'vitest'
import {
  LIFECYCLE_PIPELINE_STEPS,
  WORKFLOW_PIPELINE_STEPS,
  LONG_TAIL_PIPELINE_STEPS,
} from './types'

/** Cross-language parity anchor: the frontend's canonical stage ordering
 *  must match the Rust STAGE_PUSH_ORDER literal in
 *  `server/src/clusters.rs`. Any reordering must update both. */
const CANONICAL_PRESENT_ORDER = [
  'work-items',
  'research',
  'plans',
  'plan-reviews',
  'validations',
  'pr-descriptions',
  'pr-reviews',
  'decisions',
  'notes',
  'design-inventories',
  'design-gaps',
] as const

describe('LIFECYCLE_PIPELINE_STEPS parity', () => {
  it('matches the canonical present ordering', () => {
    const order = LIFECYCLE_PIPELINE_STEPS.map(s => s.docType)
    expect(order).toEqual([...CANONICAL_PRESENT_ORDER])
  })

  it('places workflow steps before long-tail steps', () => {
    const order = WORKFLOW_PIPELINE_STEPS.map(s => s.docType)
      .concat(LONG_TAIL_PIPELINE_STEPS.map(s => s.docType))
    expect(order).toEqual([...CANONICAL_PRESENT_ORDER])
  })
})
