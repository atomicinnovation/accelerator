// ─── Big empty-state glyphs ────────────────────────────────────────────────
// One large illustration per doc type. Used as the hero of the per-type
// empty page (replacing the generic PaperFold) and surfaced in the dev
// design-system page. Each is an 80×80 viewBox so callers can scale to any
// pixel size; the palette is derived from the type's hue (TYPE_META.hue),
// keeping each illustration colour-coordinated with the chips, glyphs and
// page tints already used elsewhere.
//
// Style rules
//   • Sketchbook, slightly tactile — small rotations, soft fills.
//   • 1.4–1.6 stroke widths.
//   • Same five-tone palette per hue, so the family reads as one set.
//   • The "content" of each illustration tells you what type it is — no
//     glyph is decorative for its own sake.

function bigPalette(hue) {
  return {
    stroke: `hsl(${hue} 50% 50%)`,
    fill:   `hsl(${hue} 78% 96%)`,
    fold:   `hsl(${hue} 50% 86%)`,
    line:   `hsl(${hue} 30% 78%)`,
    accent: `hsl(${hue} 65% 56%)`,
    deep:   `hsl(${hue} 55% 38%)`,
    white:  "#FFFFFF",
  };
}

// Per-type illustration functions. Each receives the palette and returns
// the SVG inner content. The wrapping <svg> is added by <BigGlyph/>.

const BIG_GLYPHS = {
  // WORK — task ticket with checkbox + ID tag.
  work: (p) => (
    <g>
      <g transform="rotate(-3 40 42)">
        <rect x="14" y="16" width="52" height="50" rx="3" fill={p.fill} stroke={p.stroke} strokeWidth="1.6"/>
        {/* checkbox */}
        <rect x="20" y="24" width="9" height="9" rx="1.5" fill={p.white} stroke={p.stroke} strokeWidth="1.4"/>
        <path d="m22 28.5 2 2 3.5-3.5" stroke={p.accent} strokeWidth="1.6" fill="none" strokeLinecap="round" strokeLinejoin="round"/>
        {/* title-y line */}
        <line x1="33" y1="26" x2="55" y2="26" stroke={p.stroke} strokeWidth="1.6" strokeLinecap="round"/>
        <line x1="33" y1="31" x2="48" y2="31" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        {/* body lines */}
        <line x1="20" y1="42" x2="58" y2="42" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" strokeDasharray="2 3"/>
        <line x1="20" y1="48" x2="54" y2="48" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" strokeDasharray="2 3"/>
        <line x1="20" y1="54" x2="50" y2="54" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" strokeDasharray="2 3"/>
      </g>
      {/* ID badge */}
      <g transform="translate(48 56)">
        <rect x="0" y="0" width="22" height="13" rx="2" fill={p.accent}/>
        <text x="11" y="9.2" fontFamily="ui-monospace, monospace" fontSize="7" fontWeight="700" fill={p.white} textAnchor="middle" letterSpacing="0.04em">WRK</text>
      </g>
    </g>
  ),

  // WORK-REVIEWS — task ticket with a review stamp.
  "work-reviews": (p) => (
    <g>
      <g transform="rotate(-4 38 40)">
        <rect x="10" y="14" width="48" height="52" rx="3" fill={p.fill} stroke={p.stroke} strokeWidth="1.6"/>
        <line x1="16" y1="22" x2="50" y2="22" stroke={p.stroke} strokeWidth="1.6" strokeLinecap="round"/>
        <line x1="16" y1="28" x2="42" y2="28" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        <line x1="16" y1="38" x2="50" y2="38" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" strokeDasharray="2 3"/>
        <line x1="16" y1="44" x2="46" y2="44" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" strokeDasharray="2 3"/>
        <line x1="16" y1="50" x2="40" y2="50" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" strokeDasharray="2 3"/>
      </g>
      {/* stamp ring */}
      <g transform="translate(54 50) rotate(14)">
        <circle r="13" fill="none" stroke={p.accent} strokeWidth="1.6"/>
        <circle r="10" fill="none" stroke={p.accent} strokeWidth="1.2" strokeDasharray="2 2"/>
        <path d="m-5 0 3.5 3.5L6 -4" stroke={p.accent} strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round"/>
      </g>
    </g>
  ),

  // DESIGN-INVENTORIES — 2×3 grid of UI component tiles.
  "design-inventories": (p) => (
    <g>
      <g transform="rotate(-2 40 40)">
        <rect x="10" y="10" width="60" height="60" rx="3" fill={p.fill} stroke={p.stroke} strokeWidth="1.6"/>
        {/* a row of tile dividers */}
        <line x1="10" y1="28" x2="70" y2="28" stroke={p.fold} strokeWidth="1"/>
        <line x1="40" y1="10" x2="40" y2="70" stroke={p.fold} strokeWidth="1"/>
        <line x1="10" y1="48" x2="70" y2="48" stroke={p.fold} strokeWidth="1"/>
        {/* tile 1: button */}
        <rect x="16" y="16" width="18" height="7" rx="2" fill={p.accent}/>
        {/* tile 2: avatar */}
        <circle cx="55" cy="20" r="4" fill={p.deep}/>
        <line x1="46" y1="22" x2="64" y2="22" stroke={p.line} strokeWidth="1.2" strokeLinecap="round"/>
        {/* tile 3: input field */}
        <rect x="14" y="34" width="22" height="8" rx="1.5" fill={p.white} stroke={p.stroke} strokeWidth="1.2"/>
        <line x1="17" y1="38" x2="28" y2="38" stroke={p.line} strokeWidth="1.2" strokeLinecap="round"/>
        {/* tile 4: chips */}
        <rect x="46" y="34" width="9" height="6" rx="3" fill={p.accent}/>
        <rect x="57" y="34" width="10" height="6" rx="3" fill="none" stroke={p.stroke} strokeWidth="1"/>
        {/* tile 5: card sketch */}
        <rect x="16" y="54" width="20" height="10" rx="1.5" fill={p.white} stroke={p.stroke} strokeWidth="1.2"/>
        <line x1="19" y1="58" x2="33" y2="58" stroke={p.line} strokeWidth="1" strokeLinecap="round"/>
        <line x1="19" y1="61" x2="29" y2="61" stroke={p.line} strokeWidth="1" strokeLinecap="round"/>
        {/* tile 6: list */}
        <line x1="46" y1="55" x2="66" y2="55" stroke={p.stroke} strokeWidth="1.2" strokeLinecap="round"/>
        <line x1="46" y1="59" x2="62" y2="59" stroke={p.line} strokeWidth="1.2" strokeLinecap="round"/>
        <line x1="46" y1="63" x2="64" y2="63" stroke={p.line} strokeWidth="1.2" strokeLinecap="round"/>
      </g>
    </g>
  ),

  // DESIGN-GAPS — two columns with a missing-piece arrow.
  "design-gaps": (p) => (
    <g>
      {/* left column — current */}
      <g transform="translate(0 0)">
        <rect x="10" y="12" width="22" height="56" rx="2" fill={p.fill} stroke={p.stroke} strokeWidth="1.5"/>
        <rect x="13" y="16" width="16" height="10" rx="1" fill={p.accent}/>
        <line x1="13" y1="32" x2="29" y2="32" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        <line x1="13" y1="37" x2="26" y2="37" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        <rect x="13" y="44" width="16" height="10" rx="1" fill="none" stroke={p.stroke} strokeWidth="1.2"/>
        <line x1="13" y1="60" x2="27" y2="60" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
      </g>
      {/* gap arrow */}
      <g transform="translate(40 40)">
        <line x1="-4" y1="0" x2="6" y2="0" stroke={p.deep} strokeWidth="1.6" strokeLinecap="round"/>
        <path d="m2 -4 4 4-4 4" stroke={p.deep} strokeWidth="1.6" fill="none" strokeLinecap="round" strokeLinejoin="round"/>
      </g>
      {/* right column — target (dashed gaps) */}
      <g>
        <rect x="48" y="12" width="22" height="56" rx="2" fill="none" stroke={p.stroke} strokeWidth="1.5" strokeDasharray="3 2"/>
        <rect x="51" y="16" width="16" height="10" rx="1" fill={p.fold}/>
        <line x1="51" y1="32" x2="67" y2="32" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" strokeDasharray="2 2"/>
        <line x1="51" y1="37" x2="64" y2="37" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" strokeDasharray="2 2"/>
        <rect x="51" y="44" width="16" height="10" rx="1" fill={p.accent}/>
        <line x1="51" y1="60" x2="65" y2="60" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" strokeDasharray="2 2"/>
      </g>
    </g>
  ),

  // RESEARCH — open book / two stacked sheets with a magnifier.
  research: (p) => (
    <g>
      {/* back sheet */}
      <g transform="rotate(-6 30 40)">
        <rect x="10" y="14" width="34" height="48" rx="2" fill={p.fill} stroke={p.stroke} strokeWidth="1.4"/>
        <line x1="16" y1="22" x2="38" y2="22" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        <line x1="16" y1="28" x2="34" y2="28" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        <line x1="16" y1="34" x2="36" y2="34" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
      </g>
      {/* front sheet */}
      <g transform="rotate(4 48 44)">
        <rect x="32" y="20" width="34" height="48" rx="2" fill={p.fold} stroke={p.stroke} strokeWidth="1.4"/>
        <line x1="38" y1="28" x2="60" y2="28" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        <line x1="38" y1="34" x2="56" y2="34" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        <line x1="38" y1="40" x2="60" y2="40" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        <line x1="38" y1="46" x2="54" y2="46" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
      </g>
      {/* magnifier */}
      <g transform="translate(50 50)">
        <circle r="13" fill={p.white} stroke={p.deep} strokeWidth="2"/>
        <circle r="13" fill="none" stroke={p.accent} strokeWidth="1" strokeOpacity="0.6"/>
        <line x1="9.5" y1="9.5" x2="17" y2="17" stroke={p.deep} strokeWidth="3" strokeLinecap="round"/>
      </g>
    </g>
  ),

  // PLANS — blueprint with grid + plotted milestone path.
  plans: (p) => (
    <g>
      <g transform="rotate(-2 40 40)">
        <rect x="10" y="12" width="60" height="56" rx="2" fill={p.fill} stroke={p.stroke} strokeWidth="1.6"/>
        {/* grid */}
        <g stroke={p.line} strokeWidth="0.8" opacity="0.55">
          <line x1="22" y1="12" x2="22" y2="68"/>
          <line x1="34" y1="12" x2="34" y2="68"/>
          <line x1="46" y1="12" x2="46" y2="68"/>
          <line x1="58" y1="12" x2="58" y2="68"/>
          <line x1="10" y1="24" x2="70" y2="24"/>
          <line x1="10" y1="36" x2="70" y2="36"/>
          <line x1="10" y1="48" x2="70" y2="48"/>
          <line x1="10" y1="60" x2="70" y2="60"/>
        </g>
        {/* ruler tick marks along top edge — reads as a measured plan */}
        <g stroke={p.stroke} strokeWidth="1" opacity="0.7">
          <line x1="16" y1="12" x2="16" y2="15"/>
          <line x1="28" y1="12" x2="28" y2="15"/>
          <line x1="40" y1="12" x2="40" y2="15"/>
          <line x1="52" y1="12" x2="52" y2="15"/>
          <line x1="64" y1="12" x2="64" y2="15"/>
        </g>
        {/* milestone path */}
        <path d="M14 58 L26 50 L38 52 L50 36 L66 22"
              stroke={p.deep} strokeWidth="2" fill="none"
              strokeLinecap="round" strokeLinejoin="round"/>
        <circle cx="14" cy="58" r="2.6" fill={p.accent} stroke={p.deep} strokeWidth="1"/>
        <circle cx="26" cy="50" r="2.6" fill={p.white} stroke={p.deep} strokeWidth="1.4"/>
        <circle cx="38" cy="52" r="2.6" fill={p.white} stroke={p.deep} strokeWidth="1.4"/>
        <circle cx="50" cy="36" r="2.6" fill={p.white} stroke={p.deep} strokeWidth="1.4"/>
        <circle cx="66" cy="22" r="3.2" fill={p.accent} stroke={p.deep} strokeWidth="1.4"/>
      </g>
    </g>
  ),

  // PLAN-REVIEWS — annotated plan with circled passages.
  "plan-reviews": (p) => (
    <g>
      <g transform="rotate(-3 38 40)">
        <rect x="12" y="14" width="48" height="54" rx="2" fill={p.fill} stroke={p.stroke} strokeWidth="1.6"/>
        <line x1="18" y1="22" x2="50" y2="22" stroke={p.stroke} strokeWidth="1.6" strokeLinecap="round"/>
        <line x1="18" y1="30" x2="54" y2="30" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        <line x1="18" y1="36" x2="50" y2="36" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        <line x1="18" y1="42" x2="52" y2="42" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        <line x1="18" y1="48" x2="46" y2="48" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        <line x1="18" y1="54" x2="50" y2="54" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        {/* red annotation circle */}
        <ellipse cx="38" cy="36" rx="14" ry="5.5" fill="none" stroke={p.accent} strokeWidth="1.6"/>
        {/* margin comment marker */}
        <path d="M56 42 l4 -2 m-4 2 l4 2" stroke={p.accent} strokeWidth="1.6" fill="none" strokeLinecap="round"/>
      </g>
      {/* comment bubble */}
      <g transform="translate(52 12)">
        <path d="M0 4 a4 4 0 0 1 4 -4 h14 a4 4 0 0 1 4 4 v6 a4 4 0 0 1 -4 4 h-9 l-4 4 v-4 h-1 a4 4 0 0 1 -4 -4 z"
              fill={p.deep} stroke={p.deep} strokeWidth="1"/>
        <circle cx="7"  cy="7" r="1" fill={p.white}/>
        <circle cx="11" cy="7" r="1" fill={p.white}/>
        <circle cx="15" cy="7" r="1" fill={p.white}/>
      </g>
    </g>
  ),

  // VALIDATIONS — shield with checkmark, surrounded by ticks.
  validations: (p) => (
    <g>
      {/* halo dots */}
      <g fill={p.accent}>
        <circle cx="14" cy="18" r="1.4"/>
        <circle cx="66" cy="20" r="1.4"/>
        <circle cx="10" cy="46" r="1.4"/>
        <circle cx="70" cy="48" r="1.4"/>
      </g>
      <g transform="translate(40 42)">
        <path d="M0 -28 L22 -19 L22 4 C22 16 12 24 0 28 C-12 24 -22 16 -22 4 L-22 -19 Z"
              fill={p.fill} stroke={p.stroke} strokeWidth="1.8"/>
        <path d="M0 -28 L22 -19 L22 4 C22 16 12 24 0 28 Z"
              fill={p.fold} stroke="none" opacity="0.6"/>
        {/* check */}
        <path d="m-10 0 8 8 14 -15"
              stroke={p.deep} strokeWidth="3.4" fill="none"
              strokeLinecap="round" strokeLinejoin="round"/>
        {/* badge ring */}
        <circle r="3" fill={p.accent} cx="14" cy="-12"/>
      </g>
    </g>
  ),

  // PR-DESCRIPTIONS — git branch graph + PR ticket card.
  // Time flows bottom → top (git log convention): the oldest commit sits at
  // the bottom, the open feature tip sits at the top. The feature branch
  // diverges from main lower down and grows upward to an un-merged tip
  // (highlighted with a halo). The PR card sits next to the tip.
  "pr-descriptions": (p) => (
    <g>
      {/* main branch line */}
      <line x1="14" y1="12" x2="14" y2="68" stroke={p.line} strokeWidth="2" strokeLinecap="round"/>
      {/* feature branch — diverges lower down and runs upward to its open tip */}
      <path d="M14 58 C 14 50, 26 50, 26 42 L 26 24"
            stroke={p.deep} strokeWidth="2" fill="none"
            strokeLinecap="round" strokeLinejoin="round"/>
      {/* main commit nodes (neutral) — bottom is oldest, top is newest */}
      <circle cx="14" cy="64" r="2.8" fill={p.fill} stroke={p.stroke} strokeWidth="1.5"/>
      <circle cx="14" cy="40" r="2.8" fill={p.fill} stroke={p.stroke} strokeWidth="1.5"/>
      <circle cx="14" cy="16" r="2.8" fill={p.fill} stroke={p.stroke} strokeWidth="1.5"/>
      {/* fork node — where feature diverges from main */}
      <circle cx="14" cy="58" r="3.2" fill={p.accent} stroke={p.deep} strokeWidth="1.5"/>
      {/* feature commits */}
      <circle cx="26" cy="42" r="3.2" fill={p.accent} stroke={p.deep} strokeWidth="1.5"/>
      <circle cx="26" cy="34" r="3.2" fill={p.accent} stroke={p.deep} strokeWidth="1.5"/>
      {/* feature tip — newest, open commit at the TOP, ringed with soft halo */}
      <circle cx="26" cy="24" r="6"   fill={p.accent} opacity="0.25"/>
      <circle cx="26" cy="24" r="3.6" fill={p.white}  stroke={p.deep} strokeWidth="2"/>
      {/* PR ticket card to the right, hooked off the tip */}
      <g transform="translate(36 16) rotate(3)">
        <rect x="0" y="0" width="30" height="26" rx="2" fill={p.fill} stroke={p.stroke} strokeWidth="1.6"/>
        <rect x="0" y="0" width="30" height="7" fill={p.deep}/>
        <text x="15" y="5.2" fontFamily="ui-monospace, monospace" fontSize="4.5" fontWeight="700" fill={p.white} textAnchor="middle" letterSpacing="0.1em">PR-0133</text>
        <line x1="3" y1="13" x2="22" y2="13" stroke={p.line} strokeWidth="1.2" strokeLinecap="round"/>
        <line x1="3" y1="18" x2="26" y2="18" stroke={p.line} strokeWidth="1.2" strokeLinecap="round"/>
        <line x1="3" y1="23" x2="18" y2="23" stroke={p.line} strokeWidth="1.2" strokeLinecap="round"/>
      </g>
      {/* connector — feature tip → PR card */}
      <line x1="30" y1="24" x2="36" y2="26" stroke={p.stroke} strokeWidth="1.2" strokeDasharray="2 2" strokeLinecap="round"/>
    </g>
  ),

  // PR-REVIEWS — diff lines + comment bubble.
  "pr-reviews": (p) => (
    <g>
      <g transform="rotate(-3 40 40)">
        <rect x="12" y="14" width="50" height="50" rx="2" fill={p.fill} stroke={p.stroke} strokeWidth="1.6"/>
        {/* line gutter */}
        <line x1="20" y1="14" x2="20" y2="64" stroke={p.fold} strokeWidth="1"/>
        {/* added lines */}
        <rect x="22" y="22" width="34" height="4" rx="1" fill={`hsl(140 60% 85%)`}/>
        <rect x="22" y="28" width="28" height="4" rx="1" fill={`hsl(140 60% 85%)`}/>
        <text x="17" y="26" fontFamily="ui-monospace, monospace" fontSize="5.5" fill={`hsl(140 50% 40%)`} textAnchor="middle">+</text>
        <text x="17" y="32" fontFamily="ui-monospace, monospace" fontSize="5.5" fill={`hsl(140 50% 40%)`} textAnchor="middle">+</text>
        {/* removed lines */}
        <rect x="22" y="36" width="30" height="4" rx="1" fill={`hsl(0 65% 88%)`}/>
        <text x="17" y="40" fontFamily="ui-monospace, monospace" fontSize="5.5" fill={`hsl(0 55% 45%)`} textAnchor="middle">−</text>
        {/* context lines */}
        <line x1="22" y1="46" x2="50" y2="46" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        <line x1="22" y1="52" x2="46" y2="52" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        <line x1="22" y1="58" x2="48" y2="58" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
      </g>
      {/* comment bubble */}
      <g transform="translate(46 46)">
        <path d="M0 4 a4 4 0 0 1 4 -4 h16 a4 4 0 0 1 4 4 v8 a4 4 0 0 1 -4 4 h-13 l-4 4 v-4 h-1 a4 4 0 0 1 -4 -4 z"
              fill={p.deep}/>
        <circle cx="8"  cy="9" r="1" fill={p.white}/>
        <circle cx="12" cy="9" r="1" fill={p.white}/>
        <circle cx="16" cy="9" r="1" fill={p.white}/>
      </g>
    </g>
  ),

  // DECISIONS — signed seal / ribbon with fork glyph.
  decisions: (p) => (
    <g>
      {/* paper behind */}
      <g transform="rotate(-3 38 42)">
        <rect x="14" y="14" width="44" height="52" rx="2" fill={p.fill} stroke={p.stroke} strokeWidth="1.4"/>
        <line x1="20" y1="22" x2="48" y2="22" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        <line x1="20" y1="28" x2="44" y2="28" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" strokeDasharray="2 3"/>
        <line x1="20" y1="34" x2="46" y2="34" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" strokeDasharray="2 3"/>
      </g>
      {/* seal */}
      <g transform="translate(48 50)">
        <circle r="16" fill={p.accent} stroke={p.deep} strokeWidth="1.6"/>
        <circle r="13" fill="none" stroke={p.white} strokeWidth="1.2" strokeDasharray="2 2"/>
        {/* fork glyph inside the seal */}
        <path d="M0 -7 L0 0 M0 0 L-5 6 M0 0 L5 6"
              stroke={p.white} strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round"/>
        <circle cx="0" cy="-7" r="1.6" fill={p.white}/>
        <circle cx="-5" cy="6" r="1.6" fill={p.white}/>
        <circle cx="5"  cy="6" r="1.6" fill={p.white}/>
      </g>
      {/* ribbon tails */}
      <path d="M40 64 L36 76 L42 72 L48 76 L44 64" fill={p.deep} stroke={p.deep} strokeWidth="1" strokeLinejoin="round"/>
    </g>
  ),

  // ROOT-CAUSE-ANALYSES — fishbone (Ishikawa) diagram tracing contributing
  // causes back along a spine into the failure (the burst "effect" head),
  // with a magnifier examining the chain.
  "root-cause-analyses": (p) => (
    <g>
      {/* spine — causes flow left → right into the effect */}
      <line x1="11" y1="44" x2="55" y2="44" stroke={p.deep} strokeWidth="2.4" strokeLinecap="round"/>
      {/* bones — two pairs of contributing-cause branches */}
      <g stroke={p.stroke} strokeWidth="1.6" strokeLinecap="round">
        <line x1="20" y1="19" x2="29" y2="44"/>
        <line x1="39" y1="19" x2="46" y2="44"/>
        <line x1="20" y1="69" x2="29" y2="44"/>
        <line x1="39" y1="69" x2="46" y2="44"/>
      </g>
      {/* category label ticks at each bone tip */}
      <g stroke={p.line} strokeWidth="1.4" strokeLinecap="round">
        <line x1="13" y1="16" x2="23" y2="16"/>
        <line x1="33" y1="16" x2="43" y2="16"/>
        <line x1="13" y1="72" x2="23" y2="72"/>
        <line x1="33" y1="72" x2="43" y2="72"/>
      </g>
      {/* bone-tip cause nodes */}
      <g fill={p.fill} stroke={p.stroke} strokeWidth="1.3">
        <circle cx="20" cy="19" r="2.4"/>
        <circle cx="39" cy="19" r="2.4"/>
        <circle cx="20" cy="69" r="2.4"/>
        <circle cx="39" cy="69" r="2.4"/>
      </g>
      {/* effect head — the failure the analysis traces back from */}
      <g transform="translate(58 44)">
        <g stroke={p.accent} strokeWidth="1.6" strokeLinecap="round">
          <line x1="0" y1="-13" x2="0" y2="-9.5"/>
          <line x1="9" y1="-9" x2="6.6" y2="-6.6"/>
          <line x1="13" y1="0" x2="9.5" y2="0"/>
          <line x1="9" y1="9" x2="6.6" y2="6.6"/>
          <line x1="0" y1="13" x2="0" y2="9.5"/>
        </g>
        <circle r="7.5" fill={p.accent} stroke={p.deep} strokeWidth="1.6"/>
        <line x1="0" y1="-3.6" x2="0" y2="1.4" stroke={p.white} strokeWidth="1.8" strokeLinecap="round"/>
        <circle cx="0" cy="4.2" r="1" fill={p.white}/>
      </g>
      {/* magnifier drilling into a contributing cause */}
      <g transform="translate(24 60)">
        <circle r="9" fill={p.white} stroke={p.deep} strokeWidth="2"/>
        <circle r="9" fill="none" stroke={p.accent} strokeWidth="1" strokeOpacity="0.55"/>
        <line x1="6.6" y1="6.6" x2="12.5" y2="12.5" stroke={p.deep} strokeWidth="3" strokeLinecap="round"/>
      </g>
    </g>
  ),

  // NOTES — sticky note pinned with a thumbtack.
  notes: (p) => (
    <g>
      {/* note (rotates around the thumbtack at top-center) */}
      <g transform="rotate(-5 40 18)">
        {/* soft drop shadow */}
        <rect x="15" y="19" width="50" height="50" rx="2" fill="rgba(0,0,0,0.08)"/>
        {/* note body */}
        <path d="M14 16 L66 16 L66 60 L58 66 L14 66 Z"
              fill={p.fill} stroke={p.stroke} strokeWidth="1.5" strokeLinejoin="round"/>
        {/* peeled corner */}
        <path d="M66 60 L58 66 L58 60 Z"
              fill={p.fold} stroke={p.stroke} strokeWidth="1.3" strokeLinejoin="round"/>
        {/* content lines */}
        <line x1="22" y1="30" x2="54" y2="30" stroke={p.stroke} strokeWidth="1.6" strokeLinecap="round"/>
        <line x1="22" y1="38" x2="58" y2="38" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        <line x1="22" y1="45" x2="56" y2="45" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        <line x1="22" y1="52" x2="50" y2="52" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
      </g>
      {/* thumbtack at pivot */}
      <g transform="translate(40 18)">
        <circle r="5.5" fill={p.accent} stroke={p.deep} strokeWidth="1.4"/>
        <circle r="2"   fill={p.white}/>
        <circle r="5.5" fill="none" stroke={p.white} strokeWidth="0.6" strokeOpacity="0.35"/>
      </g>
    </g>
  ),

  // TEMPLATES — stack of layered template sheets with placeholders.
  templates: (p) => (
    <g>
      {/* back sheet */}
      <g transform="rotate(-8 38 44)">
        <rect x="14" y="18" width="40" height="48" rx="2" fill={p.fold} stroke={p.stroke} strokeWidth="1.3"/>
      </g>
      {/* middle sheet */}
      <g transform="rotate(-3 40 42)">
        <rect x="18" y="14" width="42" height="50" rx="2" fill={p.fill} stroke={p.stroke} strokeWidth="1.4"/>
      </g>
      {/* top sheet */}
      <g transform="rotate(3 44 40)">
        <rect x="22" y="12" width="44" height="52" rx="2" fill={p.white} stroke={p.stroke} strokeWidth="1.5"/>
        {/* frontmatter delim */}
        <line x1="26" y1="20" x2="60" y2="20" stroke={p.line} strokeWidth="1" strokeDasharray="2 2"/>
        <text x="28" y="28" fontFamily="ui-monospace, monospace" fontSize="5.5" fill={p.deep} fontWeight="600">slug:</text>
        <line x1="40" y1="26.5" x2="56" y2="26.5" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        <text x="28" y="36" fontFamily="ui-monospace, monospace" fontSize="5.5" fill={p.deep} fontWeight="600">date:</text>
        <line x1="40" y1="34.5" x2="58" y2="34.5" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
        <line x1="26" y1="42" x2="60" y2="42" stroke={p.line} strokeWidth="1" strokeDasharray="2 2"/>
        {/* placeholder var pill */}
        <rect x="28" y="46" width="22" height="8" rx="2" fill={p.accent}/>
        <text x="39" y="51.7" fontFamily="ui-monospace, monospace" fontSize="5.5" fontWeight="700" fill={p.white} textAnchor="middle">{"{{ title }}"}</text>
        <line x1="28" y1="58" x2="58" y2="58" stroke={p.line} strokeWidth="1.4" strokeLinecap="round" strokeDasharray="2 2"/>
      </g>
    </g>
  ),
};

// Fallback used when a type isn't in the table. Renders a simple paper.
const DEFAULT_BIG = (p) => (
  <g transform="rotate(-4 40 40)">
    <rect x="16" y="14" width="48" height="56" rx="2" fill={p.fill} stroke={p.stroke} strokeWidth="1.5"/>
    <line x1="22" y1="24" x2="56" y2="24" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
    <line x1="22" y1="32" x2="50" y2="32" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
    <line x1="22" y1="40" x2="54" y2="40" stroke={p.line} strokeWidth="1.4" strokeLinecap="round"/>
  </g>
);

function BigGlyph({ type, size = 88, hue }) {
  const meta = window.TYPE_META[type] || { hue: hue || 215 };
  const palette = bigPalette(hue != null ? hue : meta.hue);
  const draw = BIG_GLYPHS[type] || DEFAULT_BIG;
  return (
    <svg viewBox="0 0 80 80"
         width={size} height={size}
         aria-hidden="true"
         className="ac-bigglyph"
         style={{ display: "block" }}>
      {draw(palette)}
    </svg>
  );
}

Object.assign(window, { BigGlyph, bigPalette, BIG_GLYPHS });
