import type { ReactElement } from 'react'

export interface BigPalette {
  stroke: string
  fill: string
  fold: string
  line: string
  accent: string
  deep: string
  white: string
}

/** Render signature for a single per-doc-type BigGlyph illustration: it
 *  receives the resolved palette and returns the inner SVG `<g>` (the wrapping
 *  `<svg>` is owned by `BigGlyph`). Named alias so the dispatch contract lives
 *  in one place and each of the thirteen illustration call sites is
 *  self-documenting. This is the key divergence from `Glyph`'s zero-arg
 *  `ComponentType` — the palette must be threaded in at render time. */
export type BigGlyphDraw = (p: BigPalette) => ReactElement

/** Derive the seven-tone BigGlyph palette from a single HSL hue (0–360).
 *  Six hue-derived tones + a fixed `white`. Value-preserving port of the
 *  prototype's `bigPalette` (big-glyphs.jsx:16-26) — the only normalisation is
 *  `white`, lowercased to `#ffffff` (the prototype writes `#FFFFFF`) to match the
 *  codebase's lowercase-hex convention. No per-doc-type tone is hard-coded —
 *  every illustration colours itself from this one hue. */
export function bigPalette(hue: number): BigPalette {
  return {
    stroke: `hsl(${hue} 50% 50%)`,
    fill: `hsl(${hue} 78% 96%)`,
    fold: `hsl(${hue} 50% 86%)`,
    line: `hsl(${hue} 30% 78%)`,
    accent: `hsl(${hue} 65% 56%)`,
    deep: `hsl(${hue} 55% 38%)`,
    white: '#ffffff',
  }
}
