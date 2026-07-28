import type { BigGlyphDraw } from "../bigPalette";

/** PLANS — blueprint with grid + plotted milestone path. Traced from
 *  big-glyphs.jsx `plans` at the 80×80 viewBox. */
export const PlansBigGlyph: BigGlyphDraw = (p) => (
  <g>
    <g transform="rotate(-2 40 40)">
      <rect
        x="10"
        y="12"
        width="60"
        height="56"
        rx="2"
        fill={p.fill}
        stroke={p.stroke}
        strokeWidth="1.6"
      />
      {/* grid */}
      <g stroke={p.line} strokeWidth="0.8" opacity="0.55">
        <line x1="22" y1="12" x2="22" y2="68" />
        <line x1="34" y1="12" x2="34" y2="68" />
        <line x1="46" y1="12" x2="46" y2="68" />
        <line x1="58" y1="12" x2="58" y2="68" />
        <line x1="10" y1="24" x2="70" y2="24" />
        <line x1="10" y1="36" x2="70" y2="36" />
        <line x1="10" y1="48" x2="70" y2="48" />
        <line x1="10" y1="60" x2="70" y2="60" />
      </g>
      {/* ruler tick marks along top edge — reads as a measured plan */}
      <g stroke={p.stroke} strokeWidth="1" opacity="0.7">
        <line x1="16" y1="12" x2="16" y2="15" />
        <line x1="28" y1="12" x2="28" y2="15" />
        <line x1="40" y1="12" x2="40" y2="15" />
        <line x1="52" y1="12" x2="52" y2="15" />
        <line x1="64" y1="12" x2="64" y2="15" />
      </g>
      {/* milestone path */}
      <path
        d="M14 58 L26 50 L38 52 L50 36 L66 22"
        stroke={p.deep}
        strokeWidth="2"
        fill="none"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <circle
        cx="14"
        cy="58"
        r="2.6"
        fill={p.accent}
        stroke={p.deep}
        strokeWidth="1"
      />
      <circle
        cx="26"
        cy="50"
        r="2.6"
        fill={p.white}
        stroke={p.deep}
        strokeWidth="1.4"
      />
      <circle
        cx="38"
        cy="52"
        r="2.6"
        fill={p.white}
        stroke={p.deep}
        strokeWidth="1.4"
      />
      <circle
        cx="50"
        cy="36"
        r="2.6"
        fill={p.white}
        stroke={p.deep}
        strokeWidth="1.4"
      />
      <circle
        cx="66"
        cy="22"
        r="3.2"
        fill={p.accent}
        stroke={p.deep}
        strokeWidth="1.4"
      />
    </g>
  </g>
);
