import type { BigGlyphDraw } from "../bigPalette";

/** WORK-ITEM-REVIEWS — task ticket with a review stamp. Traced from
 *  big-glyphs.jsx `work-reviews` at the 80×80 viewBox. */
export const WorkItemReviewsBigGlyph: BigGlyphDraw = (p) => (
  <g>
    <g transform="rotate(-4 38 40)">
      <rect
        x="10"
        y="14"
        width="48"
        height="52"
        rx="3"
        fill={p.fill}
        stroke={p.stroke}
        strokeWidth="1.6"
      />
      <line
        x1="16"
        y1="22"
        x2="50"
        y2="22"
        stroke={p.stroke}
        strokeWidth="1.6"
        strokeLinecap="round"
      />
      <line
        x1="16"
        y1="28"
        x2="42"
        y2="28"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
      />
      <line
        x1="16"
        y1="38"
        x2="50"
        y2="38"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
        strokeDasharray="2 3"
      />
      <line
        x1="16"
        y1="44"
        x2="46"
        y2="44"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
        strokeDasharray="2 3"
      />
      <line
        x1="16"
        y1="50"
        x2="40"
        y2="50"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
        strokeDasharray="2 3"
      />
    </g>
    {/* stamp ring */}
    <g transform="translate(54 50) rotate(14)">
      <circle r="13" fill="none" stroke={p.accent} strokeWidth="1.6" />
      <circle
        r="10"
        fill="none"
        stroke={p.accent}
        strokeWidth="1.2"
        strokeDasharray="2 2"
      />
      <path
        d="m-5 0 3.5 3.5L6 -4"
        stroke={p.accent}
        strokeWidth="2"
        fill="none"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </g>
  </g>
);
