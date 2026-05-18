// Preview all 12 doc types × 3 sizes in both themes at /glyph-showcase (see frontend README).
import type { ComponentType, ReactElement } from 'react'
import styles from './Glyph.module.css'
import { GLYPH_DOC_TYPE_KEYS, type GlyphDocTypeKey } from './Glyph.constants'
import { DecisionsIcon } from './icons/DecisionsIcon'
import { DesignGapsIcon } from './icons/DesignGapsIcon'
import { DesignInventoriesIcon } from './icons/DesignInventoriesIcon'
import { NotesIcon } from './icons/NotesIcon'
import { PlanReviewsIcon } from './icons/PlanReviewsIcon'
import { PlansIcon } from './icons/PlansIcon'
import { PrReviewsIcon } from './icons/PrReviewsIcon'
import { PrDescriptionsIcon } from './icons/PrDescriptionsIcon'
import { ResearchIcon } from './icons/ResearchIcon'
import { ValidationsIcon } from './icons/ValidationsIcon'
import { WorkItemReviewsIcon } from './icons/WorkItemReviewsIcon'
import { WorkItemsIcon } from './icons/WorkItemsIcon'

// Re-export the CSS-free constants module's public surface so existing
// app-side imports from `'./Glyph'` keep working. CSS-free callers (e.g.
// the Playwright spec, whose TS transformer cannot parse CSS modules)
// should import directly from `'./Glyph.constants'`.
export {
  GLYPH_DOC_TYPE_KEYS,
  isGlyphDocTypeKey,
  type GlyphDocTypeKey,
} from './Glyph.constants'

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
  'pr-descriptions': PrDescriptionsIcon,
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
  /** When true, the glyph is wrapped in a tinted square frame. Used in the
   *  library list view eyebrow and overview hub cards. */
  framed?: boolean
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
export function Glyph({ docType, size, ariaLabel, framed }: GlyphProps): ReactElement | null {
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

  // Framed mode: `size` denotes the OUTER tile dimension (matches the
  // prototype's `.ac-glyph` convention). Padding scales at ~14% so a
  // size-16 tile gets 2px pad with a 12px icon inside, size-24 → 3/18,
  // size-32 → 4/24. The wrapper carries the tinted background; the SVG
  // fills the remaining inner area and inherits its `--ac-doc-{type}`
  // colour from the inline style below.
  if (framed) {
    const pad = Math.round(size * 0.14)
    const inner = size - 2 * pad
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
          style={{ color: `var(--ac-doc-${docType})` }}
          data-doc-type={docType}
          {...a11y}
        >
          <Icon />
        </svg>
      </span>
    )
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
      style={{ color: `var(--ac-doc-${docType})` }}
      data-doc-type={docType}
      {...a11y}
    >
      <Icon />
    </svg>
  )
}
