// CSS-free constants for `Glyph`. Kept in a separate module so consumers
// that only need the type/runtime tables (notably the Playwright visual-
// regression spec, whose TS transformer can't parse CSS modules) can
// import them without pulling `Glyph.module.css` into the import graph.

import type { DocTypeKey } from "../../api/types";
import type { ColorTokenKey } from "../../styles/tokens";

// Per-doc-type colour token key. Templates is a virtual doc-type with
// no dedicated colour token — it borrows --ac-fg-muted (a neutral text
// token) for visual consistency with its --ac-fg-faint neighbours.
export const DOC_TYPE_TOKEN_KEY: Record<DocTypeKey, ColorTokenKey> = {
  decisions: "ac-doc-decisions",
  "work-items": "ac-doc-work-items",
  plans: "ac-doc-plans",
  research: "ac-doc-research",
  "plan-reviews": "ac-doc-plan-reviews",
  "pr-reviews": "ac-doc-pr-reviews",
  "work-item-reviews": "ac-doc-work-item-reviews",
  validations: "ac-doc-validations",
  notes: "ac-doc-notes",
  "pr-descriptions": "ac-doc-pr-descriptions",
  "design-gaps": "ac-doc-design-gaps",
  "design-inventories": "ac-doc-design-inventories",
  templates: "ac-fg-muted",
};

// Direct literal (no Object.fromEntries cast) so the Record<…,…>
// constraint is enforced at definition rather than via post-hoc `as`.
export const DOC_TYPE_COLOR_VAR: Record<DocTypeKey, string> = {
  decisions: `var(--${DOC_TYPE_TOKEN_KEY.decisions})`,
  "work-items": `var(--${DOC_TYPE_TOKEN_KEY["work-items"]})`,
  plans: `var(--${DOC_TYPE_TOKEN_KEY.plans})`,
  research: `var(--${DOC_TYPE_TOKEN_KEY.research})`,
  "plan-reviews": `var(--${DOC_TYPE_TOKEN_KEY["plan-reviews"]})`,
  "pr-reviews": `var(--${DOC_TYPE_TOKEN_KEY["pr-reviews"]})`,
  "work-item-reviews": `var(--${DOC_TYPE_TOKEN_KEY["work-item-reviews"]})`,
  validations: `var(--${DOC_TYPE_TOKEN_KEY.validations})`,
  notes: `var(--${DOC_TYPE_TOKEN_KEY.notes})`,
  "pr-descriptions": `var(--${DOC_TYPE_TOKEN_KEY["pr-descriptions"]})`,
  "design-gaps": `var(--${DOC_TYPE_TOKEN_KEY["design-gaps"]})`,
  "design-inventories": `var(--${DOC_TYPE_TOKEN_KEY["design-inventories"]})`,
  templates: `var(--${DOC_TYPE_TOKEN_KEY.templates})`,
};
