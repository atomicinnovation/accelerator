import type { DocTypeKey } from '../../api/types'

/** Per-doc-type descriptive sentence shown in the doc-type-empty state. */
export const EMPTY_DESCRIPTIONS: Record<DocTypeKey, string> = {
  'decisions': 'Architectural decision records capture the why behind your choices.',
  'work-items': 'Work items track features, bugs, spikes, and tasks across phases.',
  'plans': 'Plans break work items into actionable implementation steps.',
  'research': 'Research notes capture the context, prior art, and findings behind decisions.',
  'plan-reviews': 'Plan reviews record critique and approval of implementation plans.',
  'pr-reviews': 'PR reviews capture review notes against a specific pull request.',
  'work-item-reviews': 'Work item reviews record critique and refinement of work items.',
  'validations': 'Validations record acceptance evidence for a completed work item.',
  'notes': 'Notes are freeform observations and quick captures.',
  'pr-descriptions': 'PR descriptions document the intent and contents of a pull request.',
  'design-gaps': 'Design gaps record divergences between current and target UX.',
  'design-inventories': 'Design inventories catalogue current-state UI assets and screens.',
  'templates': 'Templates seed new documents for each doc type.',
}

/** Plural noun used in the "no {type-plural} yet" headline. */
export const EMPTY_TYPE_PLURALS: Record<DocTypeKey, string> = {
  'decisions': 'decisions',
  'work-items': 'work items',
  'plans': 'plans',
  'research': 'research notes',
  'plan-reviews': 'plan reviews',
  'pr-reviews': 'pr reviews',
  'work-item-reviews': 'work-item reviews',
  'validations': 'validations',
  'notes': 'notes',
  'pr-descriptions': 'pr descriptions',
  'design-gaps': 'design gaps',
  'design-inventories': 'design inventories',
  'templates': 'templates',
}
