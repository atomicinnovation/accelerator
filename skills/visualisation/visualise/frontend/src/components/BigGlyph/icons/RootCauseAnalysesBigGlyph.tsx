import type { BigGlyphDraw } from "../bigPalette";

/** ROOT-CAUSE-ANALYSES — fishbone (Ishikawa) diagram tracing contributing
 *  causes back along a spine into the failure (the burst "effect" head), with
 *  a magnifier examining the chain. Traced from big-glyphs.jsx
 *  `root-cause-analyses` at the 80×80 viewBox. */
export const RootCauseAnalysesBigGlyph: BigGlyphDraw = (p) => (
  <g>
    {/* spine — causes flow left → right into the effect */}
    <line
      x1="11"
      y1="44"
      x2="55"
      y2="44"
      stroke={p.deep}
      strokeWidth="2.4"
      strokeLinecap="round"
    />
    {/* bones — two pairs of contributing-cause branches */}
    <g stroke={p.stroke} strokeWidth="1.6" strokeLinecap="round">
      <line x1="20" y1="19" x2="29" y2="44" />
      <line x1="39" y1="19" x2="46" y2="44" />
      <line x1="20" y1="69" x2="29" y2="44" />
      <line x1="39" y1="69" x2="46" y2="44" />
    </g>
    {/* category label ticks at each bone tip */}
    <g stroke={p.line} strokeWidth="1.4" strokeLinecap="round">
      <line x1="13" y1="16" x2="23" y2="16" />
      <line x1="33" y1="16" x2="43" y2="16" />
      <line x1="13" y1="72" x2="23" y2="72" />
      <line x1="33" y1="72" x2="43" y2="72" />
    </g>
    {/* bone-tip cause nodes */}
    <g fill={p.fill} stroke={p.stroke} strokeWidth="1.3">
      <circle cx="20" cy="19" r="2.4" />
      <circle cx="39" cy="19" r="2.4" />
      <circle cx="20" cy="69" r="2.4" />
      <circle cx="39" cy="69" r="2.4" />
    </g>
    {/* effect head — the failure the analysis traces back from */}
    <g transform="translate(58 44)">
      <g stroke={p.accent} strokeWidth="1.6" strokeLinecap="round">
        <line x1="0" y1="-13" x2="0" y2="-9.5" />
        <line x1="9" y1="-9" x2="6.6" y2="-6.6" />
        <line x1="13" y1="0" x2="9.5" y2="0" />
        <line x1="9" y1="9" x2="6.6" y2="6.6" />
        <line x1="0" y1="13" x2="0" y2="9.5" />
      </g>
      <circle r="7.5" fill={p.accent} stroke={p.deep} strokeWidth="1.6" />
      <line
        x1="0"
        y1="-3.6"
        x2="0"
        y2="1.4"
        stroke={p.white}
        strokeWidth="1.8"
        strokeLinecap="round"
      />
      <circle cx="0" cy="4.2" r="1" fill={p.white} />
    </g>
    {/* magnifier drilling into a contributing cause */}
    <g transform="translate(24 60)">
      <circle r="9" fill={p.white} stroke={p.deep} strokeWidth="2" />
      <circle
        r="9"
        fill="none"
        stroke={p.accent}
        strokeWidth="1"
        strokeOpacity="0.55"
      />
      <line
        x1="6.6"
        y1="6.6"
        x2="12.5"
        y2="12.5"
        stroke={p.deep}
        strokeWidth="3"
        strokeLinecap="round"
      />
    </g>
  </g>
);
