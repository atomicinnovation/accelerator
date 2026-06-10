import type { BigGlyphDraw } from '../bigPalette'

/** VALIDATIONS — shield with checkmark, surrounded by ticks. Traced from
 *  big-glyphs.jsx `validations` at the 80×80 viewBox. */
export const ValidationsBigGlyph: BigGlyphDraw = (p) => (
  <g>
    {/* halo dots */}
    <g fill={p.accent}>
      <circle cx="14" cy="18" r="1.4" />
      <circle cx="66" cy="20" r="1.4" />
      <circle cx="10" cy="46" r="1.4" />
      <circle cx="70" cy="48" r="1.4" />
    </g>
    <g transform="translate(40 42)">
      <path d="M0 -28 L22 -19 L22 4 C22 16 12 24 0 28 C-12 24 -22 16 -22 4 L-22 -19 Z" fill={p.fill} stroke={p.stroke} strokeWidth="1.8" />
      <path d="M0 -28 L22 -19 L22 4 C22 16 12 24 0 28 Z" fill={p.fold} stroke="none" opacity="0.6" />
      {/* check */}
      <path d="m-10 0 8 8 14 -15" stroke={p.deep} strokeWidth="3.4" fill="none" strokeLinecap="round" strokeLinejoin="round" />
      {/* badge ring */}
      <circle r="3" fill={p.accent} cx="14" cy="-12" />
    </g>
  </g>
)
