// Empty-state surfaces for doc-type pages — used when a LIBRARY doc type has
// zero documents. Two flavours:
//
//   • LibraryLandingEmptyCard — the at-rest card variant for the landing grid.
//     Replaces the "no docs yet" subtext with a card that *looks* empty
//     (paper-fold pattern + faded glyph), still clickable through to the type.
//
//   • LibraryIndexEmpty — the full-page state for /library/<type> when the
//     type has zero rows. Reads from window.TYPE_COPY for the per-type
//     description and example titles.

function PaperFold({ size = 64, hue = 215 }) {
  // A friendly "empty doc" graphic. Two folded sheets of paper, the top one
  // showing a missing-content placeholder. Tinted by the doc-type hue so it
  // colour-matches the rest of the page without inventing a new palette.
  const stroke = `hsl(${hue} 50% 50%)`;
  const fill   = `hsl(${hue} 78% 96%)`;
  const fold   = `hsl(${hue} 50% 86%)`;
  const line   = `hsl(${hue} 30% 78%)`;
  return (
    <svg viewBox="0 0 80 80" width={size} height={size} aria-hidden="true" style={{display:"block"}}>
      {/* back sheet, slightly rotated */}
      <g transform="rotate(-6 26 44)">
        <rect x="10" y="20" width="34" height="44" rx="2" fill={fill} stroke={stroke} strokeWidth="1.25"/>
        <line x1="16" y1="30" x2="38" y2="30" stroke={line} strokeWidth="1.25" strokeLinecap="round"/>
        <line x1="16" y1="36" x2="34" y2="36" stroke={line} strokeWidth="1.25" strokeLinecap="round"/>
      </g>
      {/* front sheet, with corner fold */}
      <g transform="rotate(5 50 42)">
        <path d="M30 14 L58 14 L66 22 L66 64 L30 64 Z"
              fill={fill} stroke={stroke} strokeWidth="1.4" strokeLinejoin="round"/>
        <path d="M58 14 L58 22 L66 22 Z" fill={fold} stroke={stroke} strokeWidth="1.4" strokeLinejoin="round"/>
        {/* placeholder content lines */}
        <line x1="36" y1="32" x2="58" y2="32" stroke={line} strokeWidth="1.4" strokeLinecap="round"/>
        <line x1="36" y1="40" x2="60" y2="40" stroke={line} strokeWidth="1.4" strokeLinecap="round" strokeDasharray="2 3"/>
        <line x1="36" y1="48" x2="54" y2="48" stroke={line} strokeWidth="1.4" strokeLinecap="round" strokeDasharray="2 3"/>
        <line x1="36" y1="56" x2="58" y2="56" stroke={line} strokeWidth="1.4" strokeLinecap="round" strokeDasharray="2 3"/>
      </g>
      {/* subtle "plus" badge bottom-right — invitation to create */}
      <g transform="translate(54 54)">
        <circle r="9" fill="#fff" stroke={stroke} strokeWidth="1.4"/>
        <line x1="-4" y1="0" x2="4" y2="0" stroke={stroke} strokeWidth="1.6" strokeLinecap="round"/>
        <line x1="0" y1="-4" x2="0" y2="4" stroke={stroke} strokeWidth="1.6" strokeLinecap="round"/>
      </g>
    </svg>
  );
}

function LibraryLandingEmptyCard({ def, type, setRoute }) {
  return (
    <div key={type}
         className="ac-lcard ac-lcard--empty"
         style={{padding:"14px 16px", gridTemplateColumns:"auto 1fr"}}
         onClick={() => setRoute({view:"library", type, docId:null, slug:null})}>
      <div className="ac-lcard__empty-glyph" aria-hidden="true">
        <TypeGlyph type={type} size={34}/>
        <span className="ac-lcard__empty-dot">0</span>
      </div>
      <div>
        <div style={{display:"flex",alignItems:"center",justifyContent:"space-between"}}>
          <div className="ac-lcard__title" style={{fontSize:14}}>{def.label}</div>
          <span className="ac-lcard__empty-tag">empty</span>
        </div>
        <div className="ac-lcard__empty-sub">
          No documents yet.
        </div>
      </div>
    </div>
  );
}

function LibraryIndexEmpty({ type, setRoute }) {
  const def = window.DOC_TYPES.find(d => d.key === type) || { key: type, label: type };
  const copy = (window.TYPE_COPY || {})[type] || {
    purpose: "Documents of this type live here.",
    examples: [],
    path: `meta/${type}/`,
  };
  const meta = window.TYPE_META[type] || { hue: 215, label: def.label };

  return (
    <div className="ac-page">
      <div className="ac-pagehead">
        <div className="ac-pagehead__l">
          <div className="ac-pagehead__eyebrow"><TypeGlyph type={type} size={16}/> {def.label}</div>
          <h1>{def.label}</h1>
          <div className="ac-pagehead__sub">0 documents</div>
        </div>
      </div>

      <div className="ac-empty-page" style={{"--ac-empty-page-hue": meta.hue}}>
        <div className="ac-empty-page__hero">
          <BigGlyph type={type} size={96}/>
        </div>
        <div className="ac-empty-page__body">
          <div className="ac-empty-page__eyebrow mono">{copy.path}</div>
          <h2 className="ac-empty-page__title">No {def.label.toLowerCase()} yet.</h2>
          <p className="ac-empty-page__lede">{copy.purpose}</p>
          <p className="ac-empty-page__foot">
            New files added to <span className="mono">{copy.path}</span> are picked up live — this view will populate as soon as the indexer sees them.
          </p>
        </div>
      </div>
    </div>
  );
}

Object.assign(window, { LibraryLandingEmptyCard, LibraryIndexEmpty, PaperFold });
