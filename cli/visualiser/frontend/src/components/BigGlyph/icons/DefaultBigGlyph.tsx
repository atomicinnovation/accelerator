import type { BigGlyphDraw } from "../bigPalette";

/** Fallback used when a docType isn't in the dispatch table — a simple rotated
 *  paper sheet. Traced from big-glyphs.jsx `DEFAULT_BIG` (lines 408-415). The
 *  `data-testid` marker lets the off-union fallback unit test assert the
 *  DEFAULT_BIG shape specifically was selected (not merely "an <svg>"). */
export const DefaultBigGlyph: BigGlyphDraw = (p) => (
  <g data-testid="default-big-glyph" transform="rotate(-4 40 40)">
    <rect
      x="16"
      y="14"
      width="48"
      height="56"
      rx="2"
      fill={p.fill}
      stroke={p.stroke}
      strokeWidth="1.5"
    />
    <line
      x1="22"
      y1="24"
      x2="56"
      y2="24"
      stroke={p.line}
      strokeWidth="1.4"
      strokeLinecap="round"
    />
    <line
      x1="22"
      y1="32"
      x2="50"
      y2="32"
      stroke={p.line}
      strokeWidth="1.4"
      strokeLinecap="round"
    />
    <line
      x1="22"
      y1="40"
      x2="54"
      y2="40"
      stroke={p.line}
      strokeWidth="1.4"
      strokeLinecap="round"
    />
  </g>
);
