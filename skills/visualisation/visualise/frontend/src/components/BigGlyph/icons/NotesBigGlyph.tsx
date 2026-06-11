import type { BigGlyphDraw } from "../bigPalette";

/** NOTES — sticky note pinned with a thumbtack. Traced from big-glyphs.jsx
 *  `notes` at the 80×80 viewBox. */
export const NotesBigGlyph: BigGlyphDraw = (p) => (
  <g>
    {/* note (rotates around the thumbtack at top-center) */}
    <g transform="rotate(-5 40 18)">
      {/* Soft drop shadow. `rgba(0,0,0,0.08)` is a sanctioned non-palette
          structural constant (research resolved decision #1) — a translucent
          black shadow is not a doc-type tone, so it is intentionally NOT a
          member of the seven-tone bigPalette. */}
      <rect
        x="15"
        y="19"
        width="50"
        height="50"
        rx="2"
        fill="rgba(0,0,0,0.08)"
      />
      {/* note body */}
      <path
        d="M14 16 L66 16 L66 60 L58 66 L14 66 Z"
        fill={p.fill}
        stroke={p.stroke}
        strokeWidth="1.5"
        strokeLinejoin="round"
      />
      {/* peeled corner */}
      <path
        d="M66 60 L58 66 L58 60 Z"
        fill={p.fold}
        stroke={p.stroke}
        strokeWidth="1.3"
        strokeLinejoin="round"
      />
      {/* content lines */}
      <line
        x1="22"
        y1="30"
        x2="54"
        y2="30"
        stroke={p.stroke}
        strokeWidth="1.6"
        strokeLinecap="round"
      />
      <line
        x1="22"
        y1="38"
        x2="58"
        y2="38"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
      />
      <line
        x1="22"
        y1="45"
        x2="56"
        y2="45"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
      />
      <line
        x1="22"
        y1="52"
        x2="50"
        y2="52"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
      />
    </g>
    {/* thumbtack at pivot */}
    <g transform="translate(40 18)">
      <circle r="5.5" fill={p.accent} stroke={p.deep} strokeWidth="1.4" />
      <circle r="2" fill={p.white} />
      <circle
        r="5.5"
        fill="none"
        stroke={p.white}
        strokeWidth="0.6"
        strokeOpacity="0.35"
      />
    </g>
  </g>
);
