import type { BigGlyphDraw } from '../bigPalette'

/** WORK-ITEMS — task ticket with checkbox + ID tag. Traced from big-glyphs.jsx
 *  `work` at the 80×80 viewBox. */
export const WorkItemsBigGlyph: BigGlyphDraw = (p) => (
  <g>
    <g transform="rotate(-3 40 42)">
      <rect x="14" y="16" width="52" height="50" rx="3" fill={p.fill} stroke={p.stroke} strokeWidth="1.6" />
      {/* checkbox */}
      <rect x="20" y="24" width="9" height="9" rx="1.5" fill={p.white} stroke={p.stroke} strokeWidth="1.4" />
      <path d="m22 28.5 2 2 3.5-3.5" stroke={p.accent} strokeWidth="1.6" fill="none" strokeLinecap="round" strokeLinejoin="round" />
      {/* title-y line */}
      <line x1="33" y1="26" x2="55" y2="26" stroke={p.stroke} strokeWidth="1.6" strokeLinecap="round" />
      <line x1="33" y1="31" x2="48" y2="31" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" />
      {/* body lines */}
      <line x1="20" y1="42" x2="58" y2="42" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" strokeDasharray="2 3" />
      <line x1="20" y1="48" x2="54" y2="48" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" strokeDasharray="2 3" />
      <line x1="20" y1="54" x2="50" y2="54" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" strokeDasharray="2 3" />
    </g>
    {/* ID badge */}
    <g transform="translate(48 56)">
      <rect x="0" y="0" width="22" height="13" rx="2" fill={p.accent} />
      <text x="11" y="9.2" fontFamily="ui-monospace, monospace" fontSize="7" fontWeight="700" fill={p.white} textAnchor="middle" letterSpacing="0.04em">WRK</text>
    </g>
  </g>
)
