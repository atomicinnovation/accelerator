// Preview all 12 doc types × 3 sizes in both themes at /glyph-showcase (see frontend README).
import type { ComponentType, ReactElement } from 'react'
import { type DocTypeKey, DOC_TYPE_KEYS, VIRTUAL_DOC_TYPE_KEYS } from '../../api/types'
import { DecisionsIcon } from './icons/DecisionsIcon'
import { DesignGapsIcon } from './icons/DesignGapsIcon'
import { DesignInventoriesIcon } from './icons/DesignInventoriesIcon'
import { NotesIcon } from './icons/NotesIcon'
import { PlanReviewsIcon } from './icons/PlanReviewsIcon'
import { PlansIcon } from './icons/PlansIcon'
import { PrReviewsIcon } from './icons/PrReviewsIcon'
import { PrsIcon } from './icons/PrsIcon'
import { ResearchIcon } from './icons/ResearchIcon'
import { ValidationsIcon } from './icons/ValidationsIcon'
import { WorkItemReviewsIcon } from './icons/WorkItemReviewsIcon'
import { WorkItemsIcon } from './icons/WorkItemsIcon'

/**
 * The 12 non-virtual `DocTypeKey` values Glyph renders.
 *
 * INVARIANT: Glyph is for real document types only. Virtual keys (currently
 * `templates`) are excluded by construction. The exclusion is data-driven: the
 * runtime `GLYPH_DOC_TYPE_KEYS` below filters by `VIRTUAL_DOC_TYPE_KEYS`, so
 * adding a future virtual key in `api/types.ts` automatically removes it from
 * Glyph's set. The type alias must be updated in lock-step (extend the
 * `Exclude` to cover the new virtual key) — caught at unit-test time by the
 * exhaustiveness assertion on `ICON_COMPONENTS`.
 */
export type GlyphDocTypeKey = Exclude<DocTypeKey, 'templates'>

/** Runtime mirror of `GlyphDocTypeKey`. Derived from `VIRTUAL_DOC_TYPE_KEYS`
 *  at module load — assumes the virtual-keys list is statically resolvable. */
export const GLYPH_DOC_TYPE_KEYS: readonly GlyphDocTypeKey[] = DOC_TYPE_KEYS.filter(
  (k): k is GlyphDocTypeKey => !VIRTUAL_DOC_TYPE_KEYS.includes(k),
)

/** Narrow `DocTypeKey` to `GlyphDocTypeKey`. Use in data-driven consumers. */
export function isGlyphDocTypeKey(k: DocTypeKey): k is GlyphDocTypeKey {
  return GLYPH_DOC_TYPE_KEYS.includes(k as GlyphDocTypeKey)
}

// Ordering mirrors the Colour Token Table in meta/work/0037-glyph-component.md.
// `Record<GlyphDocTypeKey, ...>` constraint enforces exhaustiveness at compile
// time; the unit test that compares its keys to GLYPH_DOC_TYPE_KEYS catches a
// future virtual key being filtered out automatically.
const ICON_COMPONENTS: Record<GlyphDocTypeKey, ComponentType> = {
  'decisions': DecisionsIcon,
  'work-items': WorkItemsIcon,
  'plans': PlansIcon,
  'research': ResearchIcon,
  'plan-reviews': PlanReviewsIcon,
  'pr-reviews': PrReviewsIcon,
  'work-item-reviews': WorkItemReviewsIcon,
  'validations': ValidationsIcon,
  'notes': NotesIcon,
  'prs': PrsIcon,
  'design-gaps': DesignGapsIcon,
  'design-inventories': DesignInventoriesIcon,
}

export interface GlyphProps {
  docType: GlyphDocTypeKey
  size: 16 | 24 | 32
  /** Accessible label. If provided (including empty string), Glyph renders
   *  with `role="img"` + `aria-label`. If omitted (undefined), Glyph is
   *  decorative (`aria-hidden`). */
  ariaLabel?: string
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
 * 4. Sizes are restricted to 16/24/32. For off-grid sizes, widen the union
 *    with a documented specimen — do not cast.
 * 5. Narrow `DocTypeKey` to `GlyphDocTypeKey` via `isGlyphDocTypeKey()` or
 *    `GLYPH_DOC_TYPE_KEYS`. Do not reinvent the filter; do not use `as` casts.
 */
export function Glyph({ docType, size, ariaLabel }: GlyphProps): ReactElement | null {
  const Icon = ICON_COMPONENTS[docType]
  if (!Icon) {
    if (import.meta.env.DEV) {
      console.warn(
        `[Glyph] Unknown docType: ${String(docType)}. Expected one of: ${GLYPH_DOC_TYPE_KEYS.join(', ')}.`,
      )
    }
    return null
  }
  const a11y =
    ariaLabel !== undefined
      ? ({ role: 'img' as const, 'aria-label': ariaLabel })
      : ({ 'aria-hidden': true as const })
  return (
    // viewBox 0 0 24 24 — see meta/work/0037-glyph-component.md (Colour Token
    // Table). Theme contract: `color: var(--ac-doc-<key>)` on this <svg> +
    // `fill="currentColor"` on children. Any child overriding `fill` fails
    // loudly visually rather than silently breaking the theme contract.
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      style={{ color: `var(--ac-doc-${docType})` }}
      data-doc-type={docType}
      {...a11y}
    >
      <Icon />
    </svg>
  )
}
