import type { BigGlyphDraw } from "../bigPalette";

/** PLAN-REVIEWS — annotated plan with circled passages. Traced from
 *  big-glyphs.jsx `plan-reviews` at the 80×80 viewBox. */
export const PlanReviewsBigGlyph: BigGlyphDraw = (p) => (
  <g>
    <g transform="rotate(-3 38 40)">
      <rect
        x="12"
        y="14"
        width="48"
        height="54"
        rx="2"
        fill={p.fill}
        stroke={p.stroke}
        strokeWidth="1.6"
      />
      <line
        x1="18"
        y1="22"
        x2="50"
        y2="22"
        stroke={p.stroke}
        strokeWidth="1.6"
        strokeLinecap="round"
      />
      <line
        x1="18"
        y1="30"
        x2="54"
        y2="30"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
      />
      <line
        x1="18"
        y1="36"
        x2="50"
        y2="36"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
      />
      <line
        x1="18"
        y1="42"
        x2="52"
        y2="42"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
      />
      <line
        x1="18"
        y1="48"
        x2="46"
        y2="48"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
      />
      <line
        x1="18"
        y1="54"
        x2="50"
        y2="54"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
      />
      {/* red annotation circle */}
      <ellipse
        cx="38"
        cy="36"
        rx="14"
        ry="5.5"
        fill="none"
        stroke={p.accent}
        strokeWidth="1.6"
      />
      {/* margin comment marker */}
      <path
        d="M56 42 l4 -2 m-4 2 l4 2"
        stroke={p.accent}
        strokeWidth="1.6"
        fill="none"
        strokeLinecap="round"
      />
    </g>
    {/* comment bubble */}
    <g transform="translate(52 12)">
      <path
        d="M0 4 a4 4 0 0 1 4 -4 h14 a4 4 0 0 1 4 4 v6 a4 4 0 0 1 -4 4 h-9 l-4 4 v-4 h-1 a4 4 0 0 1 -4 -4 z"
        fill={p.deep}
        stroke={p.deep}
        strokeWidth="1"
      />
      <circle cx="7" cy="7" r="1" fill={p.white} />
      <circle cx="11" cy="7" r="1" fill={p.white} />
      <circle cx="15" cy="7" r="1" fill={p.white} />
    </g>
  </g>
);
