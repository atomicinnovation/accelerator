// Preview all 33 stroke icons in the Icons section of /dev (DevDesignSystem).
import type { ReactElement } from "react";
import { ICON_NAMES, type IconName } from "./Icon.constants";
import styles from "./Icon.module.css";

// Inner SVG geometry per icon, ported verbatim from the prototype `ui.jsx:8-53`
// Icon primitive. All paths target a 24×24 viewBox with 2px rounded strokes;
// several (`chevron-down`, `filter`, `sort`) are byte-identical to the live
// hand-written icons, confirming the shared authoring convention. The
// `Record<IconName, …>` constraint enforces exhaustiveness over all 33 names at
// compile time.
const ICON_PATHS: Record<IconName, ReactElement> = {
  search: (
    <>
      <circle cx="11" cy="11" r="7" />
      <path d="m20 20-3.5-3.5" />
    </>
  ),
  library: (
    <>
      <path d="M4 4h4v16H4z" />
      <path d="M10 4h4v16h-4z" />
      <path d="m17 5 3 1-4 14-3-1z" />
    </>
  ),
  kanban: (
    <>
      <rect x="3" y="4" width="5" height="16" rx="1" />
      <rect x="10" y="4" width="5" height="10" rx="1" />
      <rect x="17" y="4" width="4" height="13" rx="1" />
    </>
  ),
  lifecycle: (
    <>
      <circle cx="6" cy="6" r="2" />
      <circle cx="18" cy="6" r="2" />
      <circle cx="6" cy="18" r="2" />
      <circle cx="18" cy="18" r="2" />
      <path d="M8 6h8M6 8v8M18 8v8M8 18h8" />
    </>
  ),
  activity: (
    <>
      <path d="M3 12h4l3-8 4 16 3-8h4" />
    </>
  ),
  clock: (
    <>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 7v5l3 2" />
    </>
  ),
  link: (
    <>
      <path d="M10 14a4 4 0 0 0 6 0l3-3a4 4 0 0 0-6-6l-1 1" />
      <path d="M14 10a4 4 0 0 0-6 0l-3 3a4 4 0 0 0 6 6l1-1" />
    </>
  ),
  "chevron-right": (
    <>
      <path d="m9 6 6 6-6 6" />
    </>
  ),
  "chevron-down": (
    <>
      <path d="m6 9 6 6 6-6" />
    </>
  ),
  "chevron-left": (
    <>
      <path d="m15 6-6 6 6 6" />
    </>
  ),
  doc: (
    <>
      <path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" />
      <path d="M14 3v6h6" />
    </>
  ),
  edit: (
    <>
      <path d="M12 20h9" />
      <path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4z" />
    </>
  ),
  close: (
    <>
      <path d="M18 6 6 18M6 6l12 12" />
    </>
  ),
  check: (
    <>
      <path d="m5 12 5 5L20 7" />
    </>
  ),
  dot: (
    <>
      <circle cx="12" cy="12" r="3" />
    </>
  ),
  plus: (
    <>
      <path d="M12 5v14M5 12h14" />
    </>
  ),
  minus: (
    <>
      <path d="M5 12h14" />
    </>
  ),
  "git-pr": (
    <>
      <circle cx="6" cy="6" r="2.5" />
      <circle cx="6" cy="18" r="2.5" />
      <circle cx="18" cy="18" r="2.5" />
      <path d="M6 8v8" />
      <path d="M15 18H9" />
      <path d="M18 16v-4a5 5 0 0 0-5-5h-2" />
      <path d="m13 4-2 3 2 3" />
    </>
  ),
  "git-branch": (
    <>
      <circle cx="6" cy="5" r="2" />
      <circle cx="6" cy="19" r="2" />
      <circle cx="18" cy="12" r="2" />
      <path d="M6 7v10" />
      <path d="M18 10a6 6 0 0 0-6-6" />
    </>
  ),
  filter: (
    <>
      <path d="M4 4h16l-6 8v6l-4 2v-8z" />
    </>
  ),
  sort: (
    <>
      <path d="M7 4v16" />
      <path d="m3 8 4-4 4 4" />
      <path d="M17 20V4" />
      <path d="m13 16 4 4 4-4" />
    </>
  ),
  sparkle: (
    <>
      <path d="m12 3 1.8 4.8L18 9.6l-4.2 1.8L12 16.2 10.2 11.4 6 9.6l4.2-1.8z" />
    </>
  ),
  hex: (
    <>
      <path d="m12 3 8 5v8l-8 5-8-5V8z" />
    </>
  ),
  shield: (
    <>
      <path d="M12 3 4 6v6c0 4.5 3.5 8.5 8 9 4.5-.5 8-4.5 8-9V6z" />
    </>
  ),
  moon: (
    <>
      <path d="M20 14a8 8 0 0 1-10-10 8 8 0 1 0 10 10" />
    </>
  ),
  sun: (
    <>
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
    </>
  ),
  settings: (
    <>
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" />
    </>
  ),
  terminal: (
    <>
      <path d="m4 17 6-6-6-6" />
      <path d="M12 19h8" />
    </>
  ),
  "arrow-right": (
    <>
      <path d="M5 12h14M13 5l7 7-7 7" />
    </>
  ),
  flag: (
    <>
      <path d="M4 21V4h11l-2 4 2 4H4" />
    </>
  ),
  folder: (
    <>
      <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
    </>
  ),
  layers: (
    <>
      <path d="m12 3 9 5-9 5-9-5z" />
      <path d="m3 13 9 5 9-5" />
      <path d="m3 18 9 5 9-5" />
    </>
  ),
  alert: (
    <>
      <path d="M12 3 2 21h20z" />
      <path d="M12 10v5" />
      <circle cx="12" cy="18" r=".5" />
    </>
  ),
};

export { ICON_NAMES, type IconName };

export interface IconProps {
  name: IconName;
  /** Pixel width/height. Default 16. Intentionally an open `number` (stroke
   *  icons scale freely with no rasterisation curve), unlike `Glyph`'s curated
   *  `16 | 24 | 32 | 48` union — see the consumer-contract note (4) below. */
  size?: number;
  /** Accessible label. If provided (including empty string), Icon renders with
   *  `role="img"` + `aria-label`. If omitted (undefined), Icon is decorative
   *  (`aria-hidden="true"`) and assumes a sibling text label. */
  ariaLabel?: string;
  className?: string;
}

/**
 * Render a unified stroke icon (24×24 viewBox, 2px rounded, `currentColor`).
 *
 * **Consumer Contract** (mirrors `Glyph`'s):
 * 1. Do not override `fill` on Icon or an ancestor that targets it via CSS.
 *    Icon is stroke-only (`fill="none"`, `stroke="currentColor"`); tint via the
 *    cascade `color`, never `fill`.
 * 2. Provide an adjacent text label OR pass `ariaLabel` for any Icon used as a
 *    standalone visual without nearby text. The default render is `aria-hidden`.
 * 3. Do not wrap Icon in another `<svg>`. Icon owns the `<svg>` boundary; for a
 *    tinted-square treatment, compose it inside the existing `IconFrame`.
 * 4. `size` is an open number (default 16). Stroke icons are intentionally
 *    free-scaling — this divergence from `Glyph`'s size union is deliberate, not
 *    an oversight. Geometry is constant across sizes (the viewBox scales).
 */
export function Icon({
  name,
  size = 16,
  ariaLabel,
  className,
}: IconProps): ReactElement | null {
  const inner = ICON_PATHS[name];
  if (!inner) {
    if (import.meta.env.DEV) {
      console.warn(
        `[Icon] Unknown name: ${String(name)}. Expected one of: ${ICON_NAMES.join(", ")}.`,
      );
    }
    return null;
  }
  // Shared geometry/stroke props. The a11y attribute is written *literally* on
  // each branch's `<svg>` (not spread) so it stays statically analyzable: the
  // decorative branch carries an explicit `aria-hidden="true"` (matching the
  // app's hand-written inline SVGs and satisfying `noSvgWithoutTitle` without a
  // hover-tooltip `<title>`); `ariaLabel` flips to an image role exposed to AT.
  const common = {
    className: className ? `${styles.icon} ${className}` : styles.icon,
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 2,
    strokeLinecap: "round",
    strokeLinejoin: "round",
  } as const;

  if (ariaLabel !== undefined) {
    return (
      <svg {...common} role="img" aria-label={ariaLabel}>
        {inner}
      </svg>
    );
  }
  return (
    <svg {...common} aria-hidden="true">
      {inner}
    </svg>
  );
}
