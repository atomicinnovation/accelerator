import type { BigGlyphDraw } from '../bigPalette'

/** DESIGN-INVENTORIES — 2×3 grid of UI component tiles. Traced from
 *  big-glyphs.jsx `design-inventories` at the 80×80 viewBox. */
export const DesignInventoriesBigGlyph: BigGlyphDraw = (p) => (
  <g>
    <g transform="rotate(-2 40 40)">
      <rect x="10" y="10" width="60" height="60" rx="3" fill={p.fill} stroke={p.stroke} strokeWidth="1.6" />
      {/* a row of tile dividers */}
      <line x1="10" y1="28" x2="70" y2="28" stroke={p.fold} strokeWidth="1" />
      <line x1="40" y1="10" x2="40" y2="70" stroke={p.fold} strokeWidth="1" />
      <line x1="10" y1="48" x2="70" y2="48" stroke={p.fold} strokeWidth="1" />
      {/* tile 1: button */}
      <rect x="16" y="16" width="18" height="7" rx="2" fill={p.accent} />
      {/* tile 2: avatar */}
      <circle cx="55" cy="20" r="4" fill={p.deep} />
      <line x1="46" y1="22" x2="64" y2="22" stroke={p.line} strokeWidth="1.2" strokeLinecap="round" />
      {/* tile 3: input field */}
      <rect x="14" y="34" width="22" height="8" rx="1.5" fill={p.white} stroke={p.stroke} strokeWidth="1.2" />
      <line x1="17" y1="38" x2="28" y2="38" stroke={p.line} strokeWidth="1.2" strokeLinecap="round" />
      {/* tile 4: chips */}
      <rect x="46" y="34" width="9" height="6" rx="3" fill={p.accent} />
      <rect x="57" y="34" width="10" height="6" rx="3" fill="none" stroke={p.stroke} strokeWidth="1" />
      {/* tile 5: card sketch */}
      <rect x="16" y="54" width="20" height="10" rx="1.5" fill={p.white} stroke={p.stroke} strokeWidth="1.2" />
      <line x1="19" y1="58" x2="33" y2="58" stroke={p.line} strokeWidth="1" strokeLinecap="round" />
      <line x1="19" y1="61" x2="29" y2="61" stroke={p.line} strokeWidth="1" strokeLinecap="round" />
      {/* tile 6: list */}
      <line x1="46" y1="55" x2="66" y2="55" stroke={p.stroke} strokeWidth="1.2" strokeLinecap="round" />
      <line x1="46" y1="59" x2="62" y2="59" stroke={p.line} strokeWidth="1.2" strokeLinecap="round" />
      <line x1="46" y1="63" x2="64" y2="63" stroke={p.line} strokeWidth="1.2" strokeLinecap="round" />
    </g>
  </g>
)
