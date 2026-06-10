import type { BigGlyphDraw } from '../bigPalette'

/** RESEARCH — two stacked sheets with a magnifier. Traced from big-glyphs.jsx
 *  `research` at the 80×80 viewBox. */
export const ResearchBigGlyph: BigGlyphDraw = (p) => (
  <g>
    {/* back sheet */}
    <g transform="rotate(-6 30 40)">
      <rect x="10" y="14" width="34" height="48" rx="2" fill={p.fill} stroke={p.stroke} strokeWidth="1.4" />
      <line x1="16" y1="22" x2="38" y2="22" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" />
      <line x1="16" y1="28" x2="34" y2="28" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" />
      <line x1="16" y1="34" x2="36" y2="34" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" />
    </g>
    {/* front sheet */}
    <g transform="rotate(4 48 44)">
      <rect x="32" y="20" width="34" height="48" rx="2" fill={p.fold} stroke={p.stroke} strokeWidth="1.4" />
      <line x1="38" y1="28" x2="60" y2="28" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" />
      <line x1="38" y1="34" x2="56" y2="34" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" />
      <line x1="38" y1="40" x2="60" y2="40" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" />
      <line x1="38" y1="46" x2="54" y2="46" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" />
    </g>
    {/* magnifier */}
    <g transform="translate(50 50)">
      <circle r="13" fill={p.white} stroke={p.deep} strokeWidth="2" />
      <circle r="13" fill="none" stroke={p.accent} strokeWidth="1" strokeOpacity="0.6" />
      <line x1="9.5" y1="9.5" x2="17" y2="17" stroke={p.deep} strokeWidth="3" strokeLinecap="round" />
    </g>
  </g>
)
