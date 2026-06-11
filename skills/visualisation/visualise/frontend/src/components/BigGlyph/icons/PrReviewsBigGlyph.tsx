import { PR_REVIEW_DIFF_TINTS } from "../BigGlyph.constants";
import type { BigGlyphDraw } from "../bigPalette";

/** PR-REVIEWS — diff lines + comment bubble. The added/removed line tints use
 *  the fixed-hue PR_REVIEW_DIFF_TINTS (green 140 / red 0) rather than the
 *  doc-type hue, since diff semantics are colour-coded by convention. Traced
 *  from big-glyphs.jsx `pr-reviews` at the 80×80 viewBox. */
export const PrReviewsBigGlyph: BigGlyphDraw = (p) => (
  <g>
    <g transform="rotate(-3 40 40)">
      <rect
        x="12"
        y="14"
        width="50"
        height="50"
        rx="2"
        fill={p.fill}
        stroke={p.stroke}
        strokeWidth="1.6"
      />
      {/* line gutter */}
      <line x1="20" y1="14" x2="20" y2="64" stroke={p.fold} strokeWidth="1" />
      {/* added lines */}
      <rect
        x="22"
        y="22"
        width="34"
        height="4"
        rx="1"
        fill={PR_REVIEW_DIFF_TINTS.addedBg}
      />
      <rect
        x="22"
        y="28"
        width="28"
        height="4"
        rx="1"
        fill={PR_REVIEW_DIFF_TINTS.addedBg}
      />
      <text
        x="17"
        y="26"
        fontFamily="ui-monospace, monospace"
        fontSize="5.5"
        fill={PR_REVIEW_DIFF_TINTS.addedMarker}
        textAnchor="middle"
      >
        +
      </text>
      <text
        x="17"
        y="32"
        fontFamily="ui-monospace, monospace"
        fontSize="5.5"
        fill={PR_REVIEW_DIFF_TINTS.addedMarker}
        textAnchor="middle"
      >
        +
      </text>
      {/* removed lines */}
      <rect
        x="22"
        y="36"
        width="30"
        height="4"
        rx="1"
        fill={PR_REVIEW_DIFF_TINTS.removedBg}
      />
      <text
        x="17"
        y="40"
        fontFamily="ui-monospace, monospace"
        fontSize="5.5"
        fill={PR_REVIEW_DIFF_TINTS.removedMarker}
        textAnchor="middle"
      >
        −
      </text>
      {/* context lines */}
      <line
        x1="22"
        y1="46"
        x2="50"
        y2="46"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
      />
      <line
        x1="22"
        y1="52"
        x2="46"
        y2="52"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
      />
      <line
        x1="22"
        y1="58"
        x2="48"
        y2="58"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
      />
    </g>
    {/* comment bubble */}
    <g transform="translate(46 46)">
      <path
        d="M0 4 a4 4 0 0 1 4 -4 h16 a4 4 0 0 1 4 4 v8 a4 4 0 0 1 -4 4 h-13 l-4 4 v-4 h-1 a4 4 0 0 1 -4 -4 z"
        fill={p.deep}
      />
      <circle cx="8" cy="9" r="1" fill={p.white} />
      <circle cx="12" cy="9" r="1" fill={p.white} />
      <circle cx="16" cy="9" r="1" fill={p.white} />
    </g>
  </g>
);
