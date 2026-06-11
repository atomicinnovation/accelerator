import type { DocTypeKey, TemplateTierSource } from "../../api/types";

export const TIER_LABELS: Record<TemplateTierSource, string> = {
  "plugin-default": "Plugin default",
  "user-override": "User override",
  "config-override": "Config override",
};

export const TIER_SHORT_LABELS: Record<TemplateTierSource, string> = {
  "plugin-default": "default",
  "user-override": "user",
  "config-override": "config",
};

/** Fixed left-to-right render order for the index tier-presence row
 *  (resolution order, lowest priority first). */
export const TIER_ORDER: readonly TemplateTierSource[] = [
  "plugin-default",
  "user-override",
  "config-override",
] as const;

/** Stem → glyph doc-type lookup. Matches both whole template names
 *  (e.g. `adr` ⇒ `decisions`) and any dash-separated token within a
 *  template name (e.g. `codebase-research` ⇒ `research`, because
 *  `research` is a known stem). Looked up first-to-last in token order,
 *  so the rightmost matching stem wins for names like
 *  `something-plan-review` → `plan-reviews`. */
// Stems map to physical doc-type keys only — templates is the umbrella,
// not a target.
const STEM_TO_GLYPH: Readonly<Record<string, DocTypeKey>> = {
  // Exact-template-name shortcuts.
  adr: "decisions",
  plan: "plans",
  research: "research",
  validation: "validations",
  "pr-description": "pr-descriptions",
  "work-item": "work-items",
  "design-gap": "design-gaps",
  "design-inventory": "design-inventories",

  // Stem tokens that may appear as a part of compound template names.
  decision: "decisions",
  decisions: "decisions",
  plans: "plans",
  validations: "validations",
  "pr-descriptions": "pr-descriptions",
  "work-items": "work-items",
  "design-gaps": "design-gaps",
  "design-inventories": "design-inventories",
  "plan-review": "plan-reviews",
  "plan-reviews": "plan-reviews",
  "pr-review": "pr-reviews",
  "pr-reviews": "pr-reviews",
  "work-item-review": "work-item-reviews",
  "work-item-reviews": "work-item-reviews",
  note: "notes",
  notes: "notes",
};

/** Back-compat: kept for callers that expect a single-shot map. The
 *  preferred lookup is `glyphKeyForTemplate`, which also handles
 *  compound names. */
export const TEMPLATE_NAME_TO_GLYPH_KEY: Readonly<Record<string, DocTypeKey>> =
  STEM_TO_GLYPH;

export function glyphKeyForTemplate(name: string): DocTypeKey | null {
  // Exact-name match wins first.
  if (STEM_TO_GLYPH[name]) return STEM_TO_GLYPH[name];
  // Multi-token compound names: try the rightmost token first (the
  // semantically dominant suffix in the project's naming convention —
  // e.g. `codebase-research` is a kind of research), falling back
  // through earlier tokens.
  const parts = name.split("-");
  for (let i = parts.length; i > 0; i--) {
    const stem = parts.slice(parts.length - i).join("-");
    if (STEM_TO_GLYPH[stem]) return STEM_TO_GLYPH[stem];
  }
  return null;
}
