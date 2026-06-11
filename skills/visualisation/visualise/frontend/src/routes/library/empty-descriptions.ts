import type { DocTypeKey } from "../../api/types";
import { DOC_TYPE_HUE } from "../../styles/tokens";

/** Per-doc-type copy used by the list-view empty state. Mirrors the
 *  prototype's `TYPE_COPY` table — each entry carries:
 *    purpose — short sentence explaining what the doc type captures.
 *    path    — directory path shown in mono in the empty-state eyebrow + footer.
 *    hue     — HSL hue used to tint the radial-gradient background + BigGlyph
 *              hero. Single-sourced from `DOC_TYPE_HUE` in styles/tokens.ts so
 *              the empty-state panel and the BigGlyph hero cannot drift.
 */
export interface TypeCopy {
  purpose: string;
  path: string;
  hue: number;
}

export const TYPE_COPY: Record<DocTypeKey, TypeCopy> = {
  "work-items": {
    purpose: "Atomic, shippable units of work — one story per file.",
    path: "meta/work/",
    hue: DOC_TYPE_HUE["work-items"],
  },
  "work-item-reviews": {
    purpose: "Round-by-round reviews of work-item scope and breakdown.",
    path: "meta/reviews/work/",
    hue: DOC_TYPE_HUE["work-item-reviews"],
  },
  "design-inventories": {
    purpose: "Captured snapshots of an existing surface, screen-by-screen.",
    path: "meta/research/design-inventories/",
    hue: DOC_TYPE_HUE["design-inventories"],
  },
  "design-gaps": {
    purpose: "Annotated diffs between a current surface and a target design.",
    path: "meta/research/design-gaps/",
    hue: DOC_TYPE_HUE["design-gaps"],
  },
  research: {
    purpose: "Prior-art write-ups and exploration notes before planning.",
    path: "meta/research/codebase/",
    hue: DOC_TYPE_HUE.research,
  },
  plans: {
    purpose: "Design proposals for a work item, ready for review.",
    path: "meta/plans/",
    hue: DOC_TYPE_HUE.plans,
  },
  "plan-reviews": {
    purpose: "Round-by-round reviews of a plan's design.",
    path: "meta/reviews/plans/",
    hue: DOC_TYPE_HUE["plan-reviews"],
  },
  validations: {
    purpose: "Empirical checks that a plan's promises hold in code.",
    path: "meta/validations/",
    hue: DOC_TYPE_HUE.validations,
  },
  "pr-descriptions": {
    purpose: "Long-form PR descriptions co-located with the plan.",
    path: "meta/prs/",
    hue: DOC_TYPE_HUE["pr-descriptions"],
  },
  "pr-reviews": {
    purpose: "Round-by-round reviews of a specific PR.",
    path: "meta/reviews/prs/",
    hue: DOC_TYPE_HUE["pr-reviews"],
  },
  decisions: {
    purpose: "Architecture Decision Records — durable, non-reversible choices.",
    path: "meta/decisions/",
    hue: DOC_TYPE_HUE.decisions,
  },
  notes: {
    purpose:
      "Short hallway captures and open questions that don't warrant a full plan.",
    path: "meta/notes/",
    hue: DOC_TYPE_HUE.notes,
  },
  templates: {
    purpose: "Authoring templates seeded into every new artifact.",
    path: "meta/templates/",
    hue: DOC_TYPE_HUE.templates,
  },
};

/** Plural noun used in the "No {plural} yet." headline. */
export const EMPTY_TYPE_PLURALS: Record<DocTypeKey, string> = {
  decisions: "decisions",
  "work-items": "work items",
  plans: "plans",
  research: "research notes",
  "plan-reviews": "plan reviews",
  "pr-reviews": "pr reviews",
  "work-item-reviews": "work-item reviews",
  validations: "validations",
  notes: "notes",
  "pr-descriptions": "pr descriptions",
  "design-gaps": "design gaps",
  "design-inventories": "design inventories",
  templates: "templates",
};

/** Back-compat re-export so older imports keep working. Maps each doc type
 *  to the same `purpose` sentence used by the new `TYPE_COPY` table. */
export const EMPTY_DESCRIPTIONS: Record<DocTypeKey, string> =
  Object.fromEntries(
    (Object.keys(TYPE_COPY) as DocTypeKey[]).map((k) => [
      k,
      TYPE_COPY[k].purpose,
    ]),
  ) as Record<DocTypeKey, string>;
