import type { BigGlyphDraw } from "../bigPalette";

/** DECISIONS — signed seal / ribbon with fork glyph. Traced from big-glyphs.jsx
 *  `decisions` at the 80×80 viewBox. */
export const DecisionsBigGlyph: BigGlyphDraw = (p) => (
  <g>
    {/* paper behind */}
    <g transform="rotate(-3 38 42)">
      <rect
        x="14"
        y="14"
        width="44"
        height="52"
        rx="2"
        fill={p.fill}
        stroke={p.stroke}
        strokeWidth="1.4"
      />
      <line
        x1="20"
        y1="22"
        x2="48"
        y2="22"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
      />
      <line
        x1="20"
        y1="28"
        x2="44"
        y2="28"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
        strokeDasharray="2 3"
      />
      <line
        x1="20"
        y1="34"
        x2="46"
        y2="34"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
        strokeDasharray="2 3"
      />
    </g>
    {/* seal */}
    <g transform="translate(48 50)">
      <circle r="16" fill={p.accent} stroke={p.deep} strokeWidth="1.6" />
      <circle
        r="13"
        fill="none"
        stroke={p.white}
        strokeWidth="1.2"
        strokeDasharray="2 2"
      />
      {/* fork glyph inside the seal */}
      <path
        d="M0 -7 L0 0 M0 0 L-5 6 M0 0 L5 6"
        stroke={p.white}
        strokeWidth="2"
        fill="none"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <circle cx="0" cy="-7" r="1.6" fill={p.white} />
      <circle cx="-5" cy="6" r="1.6" fill={p.white} />
      <circle cx="5" cy="6" r="1.6" fill={p.white} />
    </g>
    {/* ribbon tails */}
    <path
      d="M40 64 L36 76 L42 72 L48 76 L44 64"
      fill={p.deep}
      stroke={p.deep}
      strokeWidth="1"
      strokeLinejoin="round"
    />
  </g>
);
