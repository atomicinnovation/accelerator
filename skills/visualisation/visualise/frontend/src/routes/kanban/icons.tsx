// Feather-style 24×24 stroke icons for the kanban board's page header and
// card chrome. Mirror the prototype's `Icon` set (`ui.jsx`) so the live board
// reads the same as the design; exported standalone like the lifecycle icons
// rather than routed through `Glyph` (which is keyed off `DocTypeKey`).

import { IconFrame } from "../../components/Glyph/IconFrame";

interface IconProps {
  size?: number;
}

/** Framed kanban eyebrow glyph for `Page`'s eyebrow slot — the tinted-square
 *  treatment shared with the lifecycle eyebrow and per-doc-type framed glyphs.
 *  The three-bar mark matches the prototype `kanban` Icon and the sidebar. */
export function KanbanEyebrowIcon({ size = 16 }: IconProps) {
  return (
    <IconFrame size={size}>
      <svg
        width="100%"
        height="100%"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
      >
        <rect x="3" y="4" width="5" height="16" rx="1" />
        <rect x="10" y="4" width="5" height="10" rx="1" />
        <rect x="17" y="4" width="4" height="13" rx="1" />
      </svg>
    </IconFrame>
  );
}

/** Pulse glyph — the leading icon on the "live" chip (prototype `activity`). */
export function ActivityIcon({ size = 10 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M3 12h4l3-8 4 16 3-8h4" />
    </svg>
  );
}

/** Link glyph for the card foot's "N linked" meta (prototype `link`). */
export function LinkIcon({ size = 11 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M10 14a4 4 0 0 0 6 0l3-3a4 4 0 0 0-6-6l-1 1" />
      <path d="M14 10a4 4 0 0 0-6 0l-3 3a4 4 0 0 0 6 6l1-1" />
    </svg>
  );
}
