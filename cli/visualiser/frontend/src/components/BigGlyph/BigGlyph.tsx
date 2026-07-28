// Preview every doc-type hero at 96px in both themes at /dev#bigglyphs.
import type { ReactElement } from "react";
import { DOC_TYPE_KEYS, type DocTypeKey } from "../../api/types";
import { DOC_TYPE_HUE } from "../../styles/tokens";
import { type BigGlyphDraw, bigPalette } from "./bigPalette";
import { DecisionsBigGlyph } from "./icons/DecisionsBigGlyph";
import { DefaultBigGlyph } from "./icons/DefaultBigGlyph";
import { DesignGapsBigGlyph } from "./icons/DesignGapsBigGlyph";
import { DesignInventoriesBigGlyph } from "./icons/DesignInventoriesBigGlyph";
import { NotesBigGlyph } from "./icons/NotesBigGlyph";
import { PlanReviewsBigGlyph } from "./icons/PlanReviewsBigGlyph";
import { PlansBigGlyph } from "./icons/PlansBigGlyph";
import { PrDescriptionsBigGlyph } from "./icons/PrDescriptionsBigGlyph";
import { PrReviewsBigGlyph } from "./icons/PrReviewsBigGlyph";
import { ResearchBigGlyph } from "./icons/ResearchBigGlyph";
import { RootCauseAnalysesBigGlyph } from "./icons/RootCauseAnalysesBigGlyph";
import { TemplatesBigGlyph } from "./icons/TemplatesBigGlyph";
import { ValidationsBigGlyph } from "./icons/ValidationsBigGlyph";
import { WorkItemReviewsBigGlyph } from "./icons/WorkItemReviewsBigGlyph";
import { WorkItemsBigGlyph } from "./icons/WorkItemsBigGlyph";

/** Exhaustiveness enforced by `Record<DocTypeKey, BigGlyphDraw>` across every
 *  key. Each entry is a render FUNCTION (`(p: BigPalette) => ReactElement`)
 *  invoked as `draw(palette)`, NOT a zero-arg `ComponentType` rendered as
 *  `<Icon />` — the palette must be threaded in. This is the key divergence from
 *  `Glyph`'s `ICON_COMPONENTS` map; a contributor adding a type must follow the
 *  same `(p) => <g>…</g>` shape, not reach for `<XBigGlyph />`. */
const BIG_GLYPHS: Record<DocTypeKey, BigGlyphDraw> = {
  decisions: DecisionsBigGlyph,
  "work-items": WorkItemsBigGlyph,
  plans: PlansBigGlyph,
  research: ResearchBigGlyph,
  "plan-reviews": PlanReviewsBigGlyph,
  "pr-reviews": PrReviewsBigGlyph,
  "work-item-reviews": WorkItemReviewsBigGlyph,
  validations: ValidationsBigGlyph,
  notes: NotesBigGlyph,
  "pr-descriptions": PrDescriptionsBigGlyph,
  "design-gaps": DesignGapsBigGlyph,
  "design-inventories": DesignInventoriesBigGlyph,
  "root-cause-analyses": RootCauseAnalysesBigGlyph,
  templates: TemplatesBigGlyph,
};

/** Neutral blue hue for the off-union fallback path (mirrors the prototype's
 *  `|| 215` default). Named so it is not mistaken for `templates`' canonical
 *  hue, which happens to be 215 too — they are independent facts. */
const DEFAULT_BIG_HUE = 215;

export interface BigGlyphProps {
  /** Per-doc-type hero illustration to render. Optional: an absent value (the
   *  404 catch-all / load-error surfaces, which carry no valid type) renders
   *  `DefaultBigGlyph` at `DEFAULT_BIG_HUE` through this same component — a
   *  single rendering authority, no hand-rolled fallback shell at the call
   *  site. */
  docType?: DocTypeKey;
  /** Rendered px (square). Defaults to 96 — the EmptyState hero column width.
   *  Freely scalable, unlike `Glyph`'s fixed `16 | 24 | 32` size union — this is
   *  an illustrative hero, not a fixed-grid icon. */
  size?: number;
  /** Numeric HSL hue (0–360) override. Defaults to DOC_TYPE_HUE[docType].
   *  Exposed for the 0083 showcase / off-canon rendering. Unlike `Glyph.colorVar`
   *  (a CSS-var string), BigGlyph overrides via a raw numeric hue because it
   *  constructs `hsl()` tones at render time (the runtime-hsl model). */
  hue?: number;
}

/** Decorative per-doc-type hero illustration. Runtime-hsl coloured from a single
 *  numeric hue; theme-agnostic (the surrounding panel handles light/dark).
 *  Decorative-only by design — always `aria-hidden`, with no `ariaLabel` escape
 *  hatch (unlike the small `Glyph`): the empty-state copy carries the meaning, so
 *  a labelled/announced hero is intentionally out of scope. */
export function BigGlyph({
  docType,
  size = 96,
  hue,
}: BigGlyphProps): ReactElement {
  // `??` (not `||`) so an explicit `hue={0}` (valid red) is honoured rather than
  // discarded. `DOC_TYPE_HUE[docType]` is undefined for off-union keys (cast /
  // JS callers) and `docType` is undefined for the no-type surfaces, so fall
  // back to DEFAULT_BIG_HUE — this keeps both paths null-safe so the
  // `?? DefaultBigGlyph` fallback can render.
  const resolvedHue =
    hue ?? (docType ? DOC_TYPE_HUE[docType] : undefined) ?? DEFAULT_BIG_HUE;
  const draw = (docType ? BIG_GLYPHS[docType] : undefined) ?? DefaultBigGlyph;
  // Warn only for a genuinely off-union *supplied* key. An absent docType is a
  // sanctioned default (catch-all / load-error surfaces) and must stay silent.
  if (import.meta.env.DEV && docType !== undefined && !BIG_GLYPHS[docType]) {
    console.warn(
      `[BigGlyph] Unknown docType "${docType}"; falling back to DEFAULT_BIG. ` +
        `Expected one of: ${DOC_TYPE_KEYS.join(", ")}.`,
    );
  }
  return (
    <svg
      viewBox="0 0 80 80"
      width={size}
      height={size}
      aria-hidden="true"
      style={{ display: "block" }}
    >
      {draw(bigPalette(resolvedHue))}
    </svg>
  );
}

// Exported for the dispatch-collision guard in BigGlyph.test.tsx — the
// per-DocTypeKey values must be referentially distinct functions.
export { BIG_GLYPHS, DEFAULT_BIG_HUE };
