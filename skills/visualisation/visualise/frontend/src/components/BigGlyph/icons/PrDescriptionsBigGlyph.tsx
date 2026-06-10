import type { BigGlyphDraw } from '../bigPalette'

/** PR-DESCRIPTIONS — git branch graph + PR ticket card. Time flows bottom → top
 *  (git log convention): the oldest commit sits at the bottom, the open feature
 *  tip sits at the top. Traced from big-glyphs.jsx `pr-descriptions` at the
 *  80×80 viewBox. */
export const PrDescriptionsBigGlyph: BigGlyphDraw = (p) => (
  <g>
    {/* main branch line */}
    <line x1="14" y1="12" x2="14" y2="68" stroke={p.line} strokeWidth="2" strokeLinecap="round" />
    {/* feature branch — diverges lower down and runs upward to its open tip */}
    <path d="M14 58 C 14 50, 26 50, 26 42 L 26 24" stroke={p.deep} strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round" />
    {/* main commit nodes (neutral) — bottom is oldest, top is newest */}
    <circle cx="14" cy="64" r="2.8" fill={p.fill} stroke={p.stroke} strokeWidth="1.5" />
    <circle cx="14" cy="40" r="2.8" fill={p.fill} stroke={p.stroke} strokeWidth="1.5" />
    <circle cx="14" cy="16" r="2.8" fill={p.fill} stroke={p.stroke} strokeWidth="1.5" />
    {/* fork node — where feature diverges from main */}
    <circle cx="14" cy="58" r="3.2" fill={p.accent} stroke={p.deep} strokeWidth="1.5" />
    {/* feature commits */}
    <circle cx="26" cy="42" r="3.2" fill={p.accent} stroke={p.deep} strokeWidth="1.5" />
    <circle cx="26" cy="34" r="3.2" fill={p.accent} stroke={p.deep} strokeWidth="1.5" />
    {/* feature tip — newest, open commit at the TOP, ringed with soft halo */}
    <circle cx="26" cy="24" r="6" fill={p.accent} opacity="0.25" />
    <circle cx="26" cy="24" r="3.6" fill={p.white} stroke={p.deep} strokeWidth="2" />
    {/* PR ticket card to the right, hooked off the tip */}
    <g transform="translate(36 16) rotate(3)">
      <rect x="0" y="0" width="30" height="26" rx="2" fill={p.fill} stroke={p.stroke} strokeWidth="1.6" />
      <rect x="0" y="0" width="30" height="7" fill={p.deep} />
      <text x="15" y="5.2" fontFamily="ui-monospace, monospace" fontSize="4.5" fontWeight="700" fill={p.white} textAnchor="middle" letterSpacing="0.1em">PR-0133</text>
      <line x1="3" y1="13" x2="22" y2="13" stroke={p.line} strokeWidth="1.2" strokeLinecap="round" />
      <line x1="3" y1="18" x2="26" y2="18" stroke={p.line} strokeWidth="1.2" strokeLinecap="round" />
      <line x1="3" y1="23" x2="18" y2="23" stroke={p.line} strokeWidth="1.2" strokeLinecap="round" />
    </g>
    {/* connector — feature tip → PR card */}
    <line x1="30" y1="24" x2="36" y2="26" stroke={p.stroke} strokeWidth="1.2" strokeDasharray="2 2" strokeLinecap="round" />
  </g>
)
