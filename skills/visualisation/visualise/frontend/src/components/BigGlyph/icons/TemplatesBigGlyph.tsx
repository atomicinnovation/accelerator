import type { BigGlyphDraw } from "../bigPalette";

/** TEMPLATES — stack of layered template sheets with placeholders. Traced from
 *  big-glyphs.jsx `templates` at the 80×80 viewBox. */
export const TemplatesBigGlyph: BigGlyphDraw = (p) => (
  <g>
    {/* back sheet */}
    <g transform="rotate(-8 38 44)">
      <rect
        x="14"
        y="18"
        width="40"
        height="48"
        rx="2"
        fill={p.fold}
        stroke={p.stroke}
        strokeWidth="1.3"
      />
    </g>
    {/* middle sheet */}
    <g transform="rotate(-3 40 42)">
      <rect
        x="18"
        y="14"
        width="42"
        height="50"
        rx="2"
        fill={p.fill}
        stroke={p.stroke}
        strokeWidth="1.4"
      />
    </g>
    {/* top sheet */}
    <g transform="rotate(3 44 40)">
      <rect
        x="22"
        y="12"
        width="44"
        height="52"
        rx="2"
        fill={p.white}
        stroke={p.stroke}
        strokeWidth="1.5"
      />
      {/* frontmatter delim */}
      <line
        x1="26"
        y1="20"
        x2="60"
        y2="20"
        stroke={p.line}
        strokeWidth="1"
        strokeDasharray="2 2"
      />
      <text
        x="28"
        y="28"
        fontFamily="ui-monospace, monospace"
        fontSize="5.5"
        fill={p.deep}
        fontWeight="600"
      >
        slug:
      </text>
      <line
        x1="40"
        y1="26.5"
        x2="56"
        y2="26.5"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
      />
      <text
        x="28"
        y="36"
        fontFamily="ui-monospace, monospace"
        fontSize="5.5"
        fill={p.deep}
        fontWeight="600"
      >
        date:
      </text>
      <line
        x1="40"
        y1="34.5"
        x2="58"
        y2="34.5"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
      />
      <line
        x1="26"
        y1="42"
        x2="60"
        y2="42"
        stroke={p.line}
        strokeWidth="1"
        strokeDasharray="2 2"
      />
      {/* placeholder var pill */}
      <rect x="28" y="46" width="22" height="8" rx="2" fill={p.accent} />
      <text
        x="39"
        y="51.7"
        fontFamily="ui-monospace, monospace"
        fontSize="5.5"
        fontWeight="700"
        fill={p.white}
        textAnchor="middle"
      >
        {"{{ title }}"}
      </text>
      <line
        x1="28"
        y1="58"
        x2="58"
        y2="58"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
        strokeDasharray="2 2"
      />
    </g>
  </g>
);
