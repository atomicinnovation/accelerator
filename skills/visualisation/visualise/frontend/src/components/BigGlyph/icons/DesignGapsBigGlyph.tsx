import type { BigGlyphDraw } from "../bigPalette";

/** DESIGN-GAPS — two columns with a missing-piece arrow. Traced from
 *  big-glyphs.jsx `design-gaps` at the 80×80 viewBox. */
export const DesignGapsBigGlyph: BigGlyphDraw = (p) => (
  <g>
    {/* left column — current */}
    <g transform="translate(0 0)">
      <rect
        x="10"
        y="12"
        width="22"
        height="56"
        rx="2"
        fill={p.fill}
        stroke={p.stroke}
        strokeWidth="1.5"
      />
      <rect x="13" y="16" width="16" height="10" rx="1" fill={p.accent} />
      <line
        x1="13"
        y1="32"
        x2="29"
        y2="32"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
      />
      <line
        x1="13"
        y1="37"
        x2="26"
        y2="37"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
      />
      <rect
        x="13"
        y="44"
        width="16"
        height="10"
        rx="1"
        fill="none"
        stroke={p.stroke}
        strokeWidth="1.2"
      />
      <line
        x1="13"
        y1="60"
        x2="27"
        y2="60"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
      />
    </g>
    {/* gap arrow */}
    <g transform="translate(40 40)">
      <line
        x1="-4"
        y1="0"
        x2="6"
        y2="0"
        stroke={p.deep}
        strokeWidth="1.6"
        strokeLinecap="round"
      />
      <path
        d="m2 -4 4 4-4 4"
        stroke={p.deep}
        strokeWidth="1.6"
        fill="none"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </g>
    {/* right column — target (dashed gaps) */}
    <g>
      <rect
        x="48"
        y="12"
        width="22"
        height="56"
        rx="2"
        fill="none"
        stroke={p.stroke}
        strokeWidth="1.5"
        strokeDasharray="3 2"
      />
      <rect x="51" y="16" width="16" height="10" rx="1" fill={p.fold} />
      <line
        x1="51"
        y1="32"
        x2="67"
        y2="32"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
        strokeDasharray="2 2"
      />
      <line
        x1="51"
        y1="37"
        x2="64"
        y2="37"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
        strokeDasharray="2 2"
      />
      <rect x="51" y="44" width="16" height="10" rx="1" fill={p.accent} />
      <line
        x1="51"
        y1="60"
        x2="65"
        y2="60"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
        strokeDasharray="2 2"
      />
    </g>
  </g>
);
