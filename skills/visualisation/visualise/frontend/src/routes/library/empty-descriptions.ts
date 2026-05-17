import type { DocTypeKey } from '../../api/types'

/** Per-doc-type copy used by the list-view empty state. Mirrors the
 *  prototype's `TYPE_COPY` table — each entry carries:
 *    purpose — short sentence explaining what the doc type captures.
 *    path    — directory path shown in mono in the empty-state eyebrow + footer.
 *    hue     — HSL hue used to tint the radial-gradient background + paper-fold
 *              hero. Matches the prototype's `TYPE_META.hue`.
 */
export interface TypeCopy {
  purpose: string
  path: string
  hue: number
}

export const TYPE_COPY: Record<DocTypeKey, TypeCopy> = {
  'work-items': {
    purpose: 'Atomic, shippable units of work — one story per file.',
    path: 'meta/work/',
    hue: 12,
  },
  'work-item-reviews': {
    purpose: 'Round-by-round reviews of work-item scope and breakdown.',
    path: 'meta/reviews/work/',
    hue: 340,
  },
  'design-inventories': {
    purpose: 'Captured snapshots of an existing surface, screen-by-screen.',
    path: 'meta/research/design-inventories/',
    hue: 185,
  },
  'design-gaps': {
    purpose: 'Annotated diffs between a current surface and a target design.',
    path: 'meta/research/design-gaps/',
    hue: 95,
  },
  'research': {
    purpose: "Prior-art write-ups and exploration notes before planning.",
    path: 'meta/research/codebase/',
    hue: 28,
  },
  'plans': {
    purpose: 'Design proposals for a work item, ready for review.',
    path: 'meta/plans/',
    hue: 220,
  },
  'plan-reviews': {
    purpose: "Round-by-round reviews of a plan's design.",
    path: 'meta/reviews/plans/',
    hue: 260,
  },
  'validations': {
    purpose: "Empirical checks that a plan's promises hold in code.",
    path: 'meta/validations/',
    hue: 160,
  },
  'pr-descriptions': {
    purpose: 'Long-form PR descriptions co-located with the plan.',
    path: 'meta/prs/',
    hue: 200,
  },
  'pr-reviews': {
    purpose: 'Round-by-round reviews of a specific PR.',
    path: 'meta/reviews/prs/',
    hue: 280,
  },
  'decisions': {
    purpose: 'Architecture Decision Records — durable, non-reversible choices.',
    path: 'meta/decisions/',
    hue: 355,
  },
  'notes': {
    purpose:
      "Short hallway captures and open questions that don't warrant a full plan.",
    path: 'meta/notes/',
    hue: 50,
  },
  'templates': {
    purpose: 'Authoring templates seeded into every new artifact.',
    path: 'meta/templates/',
    hue: 215,
  },
}

/** Plural noun used in the "No {plural} yet." headline. */
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

/** Back-compat re-export so older imports keep working. Maps each doc type
 *  to the same `purpose` sentence used by the new `TYPE_COPY` table. */
export const EMPTY_DESCRIPTIONS: Record<DocTypeKey, string> = Object.fromEntries(
  (Object.keys(TYPE_COPY) as DocTypeKey[]).map(k => [k, TYPE_COPY[k].purpose]),
) as Record<DocTypeKey, string>
