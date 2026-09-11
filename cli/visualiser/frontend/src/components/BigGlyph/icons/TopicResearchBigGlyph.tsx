import type { BigGlyphDraw } from "../bigPalette";

/** TOPIC-RESEARCH — a gathered dossier: layered sheets behind a cover, with
 *  strands of enquiry converging into one synthesis node. Traced from
 *  big-glyphs.jsx `topic-research` at the 80×80 viewBox. */
export const TopicResearchBigGlyph: BigGlyphDraw = (p) => (
  <g>
    <g transform="rotate(-9 34 44)">
      <rect
        x="12"
        y="20"
        width="38"
        height="46"
        rx="2"
        fill={p.fold}
        stroke={p.stroke}
        strokeWidth="1.3"
      />
    </g>
    <g transform="rotate(-4 36 42)">
      <rect
        x="16"
        y="16"
        width="40"
        height="48"
        rx="2"
        fill={p.fill}
        stroke={p.stroke}
        strokeWidth="1.4"
      />
    </g>
    <g transform="rotate(3 42 40)">
      <rect
        x="20"
        y="12"
        width="46"
        height="54"
        rx="2.5"
        fill={p.white}
        stroke={p.stroke}
        strokeWidth="1.6"
      />
      <rect x="20" y="12" width="46" height="9" rx="2.5" fill={p.deep} />
      <rect x="20" y="18" width="46" height="3" fill={p.deep} />
      <text
        x="43"
        y="18.6"
        fontFamily="ui-monospace, monospace"
        fontSize="5.5"
        fontWeight="700"
        fill={p.white}
        textAnchor="middle"
        letterSpacing="0.14em"
      >
        TOPIC
      </text>
      <line
        x1="26"
        y1="28"
        x2="58"
        y2="28"
        stroke={p.line}
        strokeWidth="1.5"
        strokeLinecap="round"
      />
      <path
        d="M26 37 L45 46"
        stroke={p.stroke}
        strokeWidth="1.5"
        fill="none"
        strokeLinecap="round"
      />
      <path
        d="M26 46 L45 46"
        stroke={p.stroke}
        strokeWidth="1.5"
        fill="none"
        strokeLinecap="round"
      />
      <path
        d="M26 55 L45 46"
        stroke={p.stroke}
        strokeWidth="1.5"
        fill="none"
        strokeLinecap="round"
      />
      <circle
        cx="26"
        cy="37"
        r="2.2"
        fill={p.fill}
        stroke={p.stroke}
        strokeWidth="1.3"
      />
      <circle
        cx="26"
        cy="46"
        r="2.2"
        fill={p.fill}
        stroke={p.stroke}
        strokeWidth="1.3"
      />
      <circle
        cx="26"
        cy="55"
        r="2.2"
        fill={p.fill}
        stroke={p.stroke}
        strokeWidth="1.3"
      />
      <circle
        cx="47"
        cy="46"
        r="5"
        fill={p.accent}
        stroke={p.deep}
        strokeWidth="1.5"
      />
      <line
        x1="26"
        y1="62"
        x2="52"
        y2="62"
        stroke={p.line}
        strokeWidth="1.4"
        strokeLinecap="round"
        strokeDasharray="2 3"
      />
    </g>
  </g>
);
