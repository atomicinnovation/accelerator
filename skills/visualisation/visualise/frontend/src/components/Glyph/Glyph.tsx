// Preview all 13 doc types × 3 sizes in both themes at /glyph-showcase (see frontend README).
import type { ComponentType, ReactElement } from "react";
import { DOC_TYPE_KEYS } from "../../api/types";
import { DOC_TYPE_COLOR_VAR, type GlyphDocType } from "./Glyph.constants";
import styles from "./Glyph.module.css";
import { DecisionsIcon } from "./icons/DecisionsIcon";
import { DesignGapsIcon } from "./icons/DesignGapsIcon";
import { DesignInventoriesIcon } from "./icons/DesignInventoriesIcon";
import { NotesIcon } from "./icons/NotesIcon";
import { PlanReviewsIcon } from "./icons/PlanReviewsIcon";
import { PlansIcon } from "./icons/PlansIcon";
import { PrDescriptionsIcon } from "./icons/PrDescriptionsIcon";
import { PrReviewsIcon } from "./icons/PrReviewsIcon";
import { ResearchIcon } from "./icons/ResearchIcon";
import { RootCauseAnalysesIcon } from "./icons/RootCauseAnalysesIcon";
import { TemplatesIcon } from "./icons/TemplatesIcon";
import { ValidationsIcon } from "./icons/ValidationsIcon";
import { WorkItemReviewsIcon } from "./icons/WorkItemReviewsIcon";
import { WorkItemsIcon } from "./icons/WorkItemsIcon";

// Ordering mirrors the Colour Token Table in meta/work/0037-glyph-component.md.
// `Record<GlyphDocType, ...>` constraint enforces exhaustiveness at compile
// time across all 13 server doc-type keys (including the virtual `templates`
// key) plus the glyph-only `root-cause-analyses` key (rendered for the `rca`
// template; not a browsable doc type — see Glyph.constants).
const ICON_COMPONENTS: Record<GlyphDocType, ComponentType> = {
  decisions: DecisionsIcon,
  "work-items": WorkItemsIcon,
  plans: PlansIcon,
  research: ResearchIcon,
  "plan-reviews": PlanReviewsIcon,
  "pr-reviews": PrReviewsIcon,
  "work-item-reviews": WorkItemReviewsIcon,
  validations: ValidationsIcon,
  notes: NotesIcon,
  "pr-descriptions": PrDescriptionsIcon,
  "design-gaps": DesignGapsIcon,
  "design-inventories": DesignInventoriesIcon,
  "root-cause-analyses": RootCauseAnalysesIcon,
  templates: TemplatesIcon,
};

export interface GlyphProps {
  docType: GlyphDocType;
  size: 16 | 24 | 32 | 48;
  /** Accessible label. If provided (including empty string), Glyph renders
   *  with `role="img"` + `aria-label`. If omitted (undefined), Glyph is
   *  decorative (`aria-hidden`). */
  ariaLabel?: string;
  /** When true, the glyph is wrapped in a tinted square frame. Used in the
   *  library list view eyebrow and overview hub cards. */
  framed?: boolean;
  /** Override the inline `color` driving the glyph's fill. Pass a CSS
   *  custom-property reference (e.g. `var(--ac-stage-plans)`) when a
   *  consumer surface needs a different hue family than the default
   *  per-doc-type `--ac-doc-<key>` token (Pipeline uses this to render
   *  active stages in the bright `--ac-stage-*` chain palette). */
  colorVar?: string;
}

/**
 * Render a per-doc-type icon at 16/24/32 px with theme-aware fill.
 *
 * **Consumer Contract** (downstream WIs 0036/0040/0041/0042/0043/0053/0054/0055):
 * 1. Do not override `fill` on Glyph or any ancestor that targets it via CSS.
 *    Glyph drives colour through `color: var(--ac-doc-<key>)` on the `<svg>`
 *    and `fill="currentColor"` on children; overriding `color` would tint,
 *    overriding `fill` would break the theme contract.
 * 2. Provide an adjacent text label OR pass `ariaLabel` for any Glyph used as
 *    a standalone visual without nearby text. The default render is
 *    `aria-hidden` and assumes a sibling text label is present.
 * 3. Do not wrap Glyph in another `<svg>`. Glyph owns the `<svg>` boundary.
 * 4. Sizes are restricted to 16/24/32/48 (48 added for the DevDesignSystem
 *    Doc-type-glyphs four-size ramp). For off-grid sizes, widen the union
 *    with a documented specimen — do not cast.
 * 5. `docType` accepts any `GlyphDocType` — every `DocTypeKey` (all 13,
 *    including the virtual `templates` key) plus glyph-only keys like
 *    `root-cause-analyses`. Colour resolves via `DOC_TYPE_COLOR_VAR`.
 */
export function Glyph({
  docType,
  size,
  ariaLabel,
  framed,
  colorVar,
}: GlyphProps): ReactElement | null {
  const Icon = ICON_COMPONENTS[docType];
  if (!Icon) {
    if (import.meta.env.DEV) {
      console.warn(
        `[Glyph] Unknown docType: ${String(docType)}. Expected one of: ${DOC_TYPE_KEYS.join(", ")}.`,
      );
    }
    return null;
  }
  const resolvedColor = colorVar ?? DOC_TYPE_COLOR_VAR[docType];
  const a11y =
    ariaLabel !== undefined
      ? { role: "img" as const, "aria-label": ariaLabel }
      : { "aria-hidden": true as const };

  // Framed mode: `size` denotes the OUTER tile dimension (matches the
  // prototype's `.ac-glyph` convention). Padding scales at ~14% so a
  // size-16 tile gets 2px pad with a 12px icon inside, size-24 → 3/18,
  // size-32 → 4/24. The wrapper carries the tinted background; the SVG
  // fills the remaining inner area and inherits its `--ac-doc-{type}`
  // colour from the inline style below.
  if (framed) {
    const pad = Math.round(size * 0.14);
    const inner = size - 2 * pad;
    return (
      <span
        className={styles.frame}
        data-doc-type={docType}
        style={{ width: `${size}px`, height: `${size}px`, padding: `${pad}px` }}
      >
        <svg
          width={inner}
          height={inner}
          viewBox="0 0 24 24"
          style={{ color: resolvedColor }}
          data-doc-type={docType}
          {...a11y}
        >
          <title>{ariaLabel ?? `${docType} glyph`}</title>
          <Icon />
        </svg>
      </span>
    );
  }

  // viewBox 0 0 24 24 — see meta/work/0037-glyph-component.md (Colour Token
  // Table). Theme contract: `color: var(--ac-doc-<key>)` on this <svg> +
  // `fill="currentColor"` on children. Any child overriding `fill` fails
  // loudly visually rather than silently breaking the theme contract.
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      style={{ color: resolvedColor }}
      data-doc-type={docType}
      {...a11y}
    >
      <title>{ariaLabel ?? `${docType} glyph`}</title>
      <Icon />
    </svg>
  );
}
