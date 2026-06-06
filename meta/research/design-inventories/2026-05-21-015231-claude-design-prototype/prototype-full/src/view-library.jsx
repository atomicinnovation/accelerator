// Library index + doc page views.

function LibraryLanding({ setRoute }) {
  const sections = window.LIBRARY_GROUPS.map(g => ({ group: g.label, types: g.types }));
  return (
    <div className="ac-page">
      <div className="ac-pagehead">
        <div className="ac-pagehead__l">
          <div className="ac-pagehead__eyebrow"><span className="ac-eyebrow-glyph"><Icon name="library" size={12}/></span> Library</div>
          <h1>All artifacts in <span className="mono" style={{fontSize:22}}>meta/</span></h1>
          <div className="ac-pagehead__sub">Browse every doc type produced by the research → plan → implement workflow. Click a type to drill in, or jump into a view.</div>
        </div>
      </div>

      {sections.map(sec => (
        <div key={sec.group} style={{marginBottom: 28}}>
          <div style={{fontFamily:"var(--ac-font-mono)",fontSize:11,letterSpacing:"0.12em",textTransform:"uppercase",color:"var(--ac-fg-faint)",marginBottom:10}}>{sec.group}</div>
          <div style={{display:"grid",gridTemplateColumns:"repeat(auto-fill,minmax(280px,1fr))",gap:12}}>
            {sec.types.map(t => {
              const def = window.DOC_TYPES.find(d => d.key === t);
              const rows = window.LIBRARY_INDEX[t] || [];
              const latest = rows[0];
              if (rows.length === 0) {
                return <LibraryLandingEmptyCard key={t} def={def} type={t} setRoute={setRoute}/>;
              }
              return (
                <div key={t} className="ac-lcard" style={{padding:"14px 16px",gridTemplateColumns:"auto 1fr"}}
                     onClick={() => setRoute({view:"library", type:t, docId:null, slug:null})}>
                  <TypeGlyph type={t} size={34}/>
                  <div>
                    <div style={{display:"flex",alignItems:"center",justifyContent:"space-between"}}>
                      <div className="ac-lcard__title" style={{fontSize:14}}>{def.label}</div>
                      <span className="mono faint" style={{fontSize:11}}>{def.count}</span>
                    </div>
                    <div className="ac-lcard__slug" style={{marginTop:4}}>{latest ? "latest · " + latest.title : "no docs yet"}</div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      ))}
    </div>
  );
}

function SlugFacet({ slugs, rows, selected, onToggle }) {
  const [q, setQ] = React.useState("");
  const showSearch = slugs.length > 6;
  const filtered = q.trim()
    ? slugs.filter(s => s.toLowerCase().includes(q.trim().toLowerCase()))
    : slugs;
  return (
    <div className="ac-filter__group">
      <div className="ac-filter__label">Cluster slug</div>
      {showSearch && (
        <div className="ac-filter__search">
          <Icon name="search" size={11}/>
          <input
            type="text"
            placeholder="Filter slugs…"
            value={q}
            onChange={e => setQ(e.target.value)}/>
        </div>
      )}
      <div className="ac-filter__scroll">
        {filtered.map(s => (
          <label key={s} className="ac-filter__opt">
            <input type="checkbox"
                   checked={selected.has(s)}
                   onChange={() => onToggle(s)}/>
            <span className="ac-filter__opt-name mono"
                  title={s}
                  style={{overflow:"hidden",textOverflow:"ellipsis",whiteSpace:"nowrap"}}>
              {s}
            </span>
            <span className="ac-filter__opt-count">
              {rows.filter(r => r.slug === s).length}
            </span>
          </label>
        ))}
        {filtered.length === 0 && (
          <div className="faint" style={{padding:"6px 8px",fontSize:11.5}}>No matches.</div>
        )}
      </div>
    </div>
  );
}

function LibraryIndex({ type, setRoute }) {
  const def = window.DOC_TYPES.find(d => d.key === type) || window.META.find(d => d.key === type);
  const rows = window.LIBRARY_INDEX[type] || [];
  const isTemplates = type === "templates";
  if (isTemplates) return <TemplatesIndex setRoute={setRoute}/>;

  // Zero documents in this type → full-page empty state.
  if (rows.length === 0) return <LibraryIndexEmpty type={type} setRoute={setRoute}/>;

  // ─── Filter state ───────────────────────────────────────────────
  const isWork = type === "work";
  const isVerdict = type === "validations" || (type || "").includes("review");
  const statusKey = isVerdict ? "verdict" : "status";
  const statusLabel = isVerdict ? "Verdict" : "Status";

  // Derive project code from an ID like "PROJ-0001" or "0015"
  const projectOf = (id) => {
    if (!id) return null;
    const m = /^([A-Z]+)-/.exec(id);
    return m ? m[1] : "Unprefixed";
  };

  const allStatuses = Array.from(new Set(rows.map(r => r[statusKey]).filter(Boolean)));
  const allProjects = isWork ? Array.from(new Set(rows.map(r => projectOf(r.id)).filter(Boolean))) : [];
  const allSlugs = Array.from(new Set(rows.map(r => r.slug).filter(Boolean))).sort();

  const [filterOpen, setFilterOpen] = React.useState(false);
  const [statusFilter, setStatusFilter] = React.useState(new Set());
  const [projectFilter, setProjectFilter] = React.useState(new Set());
  const [slugFilter, setSlugFilter] = React.useState(new Set());
  const filterRef = React.useRef(null);

  // ─── Sort state ─────────────────────────────────────────────────
  const SORTS = [
    { key: "modified-desc", label: "Recently modified" },
    { key: "modified-asc",  label: "Oldest first" },
    { key: "title-asc",     label: "Title (A → Z)" },
    { key: "title-desc",    label: "Title (Z → A)" },
    { key: "id-asc",        label: "ID (ascending)" },
  ];
  const [sortKey, setSortKey] = React.useState("modified-desc");
  const [sortOpen, setSortOpen] = React.useState(false);
  const sortRef = React.useRef(null);
  const sortLabel = (SORTS.find(s => s.key === sortKey) || SORTS[0]).label;

  // Close on outside click / Esc
  React.useEffect(() => {
    if (!filterOpen && !sortOpen) return;
    const onDown = (e) => {
      if (filterOpen && filterRef.current && !filterRef.current.contains(e.target)) setFilterOpen(false);
      if (sortOpen && sortRef.current && !sortRef.current.contains(e.target)) setSortOpen(false);
    };
    const onKey = (e) => { if (e.key === "Escape") { setFilterOpen(false); setSortOpen(false); } };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [filterOpen, sortOpen]);

  // Reset filters / sort when switching type
  React.useEffect(() => {
    setStatusFilter(new Set()); setProjectFilter(new Set()); setSlugFilter(new Set());
    setFilterOpen(false); setSortOpen(false);
    setSortKey("modified-desc");
  }, [type]);

  const toggle = (set, setSet, val) => {
    const next = new Set(set);
    if (next.has(val)) next.delete(val); else next.add(val);
    setSet(next);
  };

  const filteredRows = rows.filter(r => {
    if (statusFilter.size && !statusFilter.has(r[statusKey])) return false;
    if (projectFilter.size && !projectFilter.has(projectOf(r.id))) return false;
    if (slugFilter.size && !slugFilter.has(r.slug)) return false;
    return true;
  });
  const activeFilterCount = statusFilter.size + projectFilter.size + slugFilter.size;
  const clearAll = () => { setStatusFilter(new Set()); setProjectFilter(new Set()); setSlugFilter(new Set()); };

  // Sort. The data rows already arrive in modified-desc order, so we use index
  // as a stable proxy for "most recent first" rather than parsing strings like
  // "5m ago" / "2026-04-17". For title/id we use plain string compare.
  const indexed = filteredRows.map((r, i) => ({ r, i }));
  switch (sortKey) {
    case "modified-asc":  indexed.reverse(); break;
    case "title-asc":     indexed.sort((a, b) => (a.r.title || "").localeCompare(b.r.title || "")); break;
    case "title-desc":    indexed.sort((a, b) => (b.r.title || "").localeCompare(a.r.title || "")); break;
    case "id-asc":        indexed.sort((a, b) => (a.r.id || "").localeCompare(b.r.id || "")); break;
    default: /* modified-desc — already in source order */ break;
  }
  const visibleRows = indexed.map(x => x.r);

  return (
    <div className="ac-page">
      <div className="ac-pagehead">
        <div className="ac-pagehead__l">
          <div className="ac-pagehead__eyebrow"><TypeGlyph type={type} size={16}/> {def.label}</div>
          <h1>{def.label}</h1>
          <div className="ac-pagehead__sub">
            {filteredRows.length === rows.length
              ? <>{rows.length} documents</>
              : <>{filteredRows.length} of {rows.length} documents</>}
          </div>
        </div>
        <div className="ac-pagehead__actions">
          <div className="ac-filter" ref={sortRef}>
            <button
              className={`ac-topbar__btn ac-sort-btn ${sortOpen ? "is-open" : ""}`}
              onClick={() => { setSortOpen(o => !o); setFilterOpen(false); }}>
              <Icon name="sort" size={12}/>
              <span>{sortLabel}</span>
              <Icon name="chevron-down" size={11}/>
            </button>
            {sortOpen && (
              <div className="ac-filter__pop ac-sort__pop" role="listbox" aria-label="Sort">
                <div className="ac-filter__head"><span>Sort by</span></div>
                {SORTS.map(s => (
                  <button
                    key={s.key}
                    role="option"
                    aria-selected={s.key === sortKey}
                    className={`ac-sort__opt ${s.key === sortKey ? "is-active" : ""}`}
                    onClick={() => { setSortKey(s.key); setSortOpen(false); }}>
                    <span>{s.label}</span>
                    {s.key === sortKey && <Icon name="check" size={12}/>}
                  </button>
                ))}
              </div>
            )}
          </div>
          <div className="ac-filter" ref={filterRef}>
            <button
              className={`ac-topbar__btn ac-sort-btn ${activeFilterCount ? "is-active" : ""}`}
              onClick={() => setFilterOpen(o => !o)}>
              <Icon name="filter" size={13}/>
              <span>Filter</span>
              {activeFilterCount > 0 && <span className="ac-filter__badge">{activeFilterCount}</span>}
            </button>
            {filterOpen && (
              <div className="ac-filter__pop" role="dialog" aria-label="Filter">
                <div className="ac-filter__head">
                  <span>Filter</span>
                  {activeFilterCount > 0 && (
                    <button className="ac-filter__clear" onClick={clearAll}>
                      Clear all
                    </button>
                  )}
                </div>
                {allStatuses.length > 0 && (
                  <div className="ac-filter__group">
                    <div className="ac-filter__label">{statusLabel}</div>
                    {allStatuses.map(s => (
                      <label key={s} className="ac-filter__opt">
                        <input type="checkbox"
                               checked={statusFilter.has(s)}
                               onChange={() => toggle(statusFilter, setStatusFilter, s)}/>
                        <StatusBadge status={s}/>
                        <span className="ac-filter__opt-count">
                          {rows.filter(r => r[statusKey] === s).length}
                        </span>
                      </label>
                    ))}
                  </div>
                )}
                {allProjects.length > 0 && (
                  <div className="ac-filter__group">
                    <div className="ac-filter__label">Project</div>
                    {allProjects.map(p => (
                      <label key={p} className="ac-filter__opt">
                        <input type="checkbox"
                               checked={projectFilter.has(p)}
                               onChange={() => toggle(projectFilter, setProjectFilter, p)}/>
                        <span className="ac-filter__opt-name mono">{p}</span>
                        <span className="ac-filter__opt-count">
                          {rows.filter(r => projectOf(r.id) === p).length}
                        </span>
                      </label>
                    ))}
                  </div>
                )}
                {allSlugs.length > 1 && (
                  <SlugFacet
                    slugs={allSlugs}
                    rows={rows}
                    selected={slugFilter}
                    onToggle={(v) => toggle(slugFilter, setSlugFilter, v)}/>
                )}
              </div>
            )}
          </div>
        </div>
      </div>

      <table className="ac-libtable">
        <thead>
          <tr>
            <th style={{width:130}}>ID / Date</th>
            <th>Title</th>
            <th style={{width:180}}>{statusLabel}</th>
            <th style={{width:240}}>Slug</th>
            <th style={{width:100, textAlign:"right"}}>Modified</th>
          </tr>
        </thead>
        <tbody>
          {visibleRows.map((r, i) => (
            <tr key={i} onClick={() => setRoute({view:"library", type, docId: r.id, slug: r.slug})}>
              <td className="ac-libtable__id">{r.id}</td>
              <td className="ac-libtable__title">{r.title}</td>
              <td>{r.status ? <StatusBadge status={r.status}/> : r.verdict ? <StatusBadge status={r.verdict}/> : <span className="faint mono" style={{fontSize:11}}>—</span>}</td>
              <td className="ac-libtable__slug">{r.slug || "—"}</td>
              <td className="ac-libtable__date" style={{textAlign:"right"}}>{r.date}</td>
            </tr>
          ))}
          {visibleRows.length === 0 && (
            <tr>
              <td colSpan={5} style={{padding:"40px 16px",textAlign:"center"}}>
                <div className="faint" style={{fontSize:13}}>No documents match the current filter.</div>
                <button className="ac-topbar__btn" style={{marginTop:8}} onClick={clearAll}>
                  Clear filter
                </button>
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}

function DocPage({ type, docId, slug, setRoute }) {
  // Pick sample content for the doc we're showing. Map a few canonical ones.
  // Direct hit by docId (curated content)
  let content = window.DOC_CONTENT[docId];
  // Curated aliases for a handful of known docs
  if (!content) {
    if (type === "plans" && docId === "2026-04-17") content = window.DOC_CONTENT["2026-04-17-plan"];
    if (type === "plan-reviews" && docId === "review-1") content = window.DOC_CONTENT["2026-04-18-review"];
  }
  // Synthesise type-appropriate content from the LIBRARY_INDEX row so every
  // doc opens to a realistic, type-specific detail page rather than a stub.
  if (!content) {
    const row = (window.LIBRARY_INDEX[type] || []).find(r => r.id === docId);
    if (row) content = window.synthDocContent(type, row);
  }
  if (!content) content = window.DOC_CONTENT["ADR-0002"];

  const cluster = window.CLUSTERS.find(c => c.slug === (slug || content.slug));
  const related = cluster ? cluster.entries.filter(e => !(e.id === docId)) : [];

  const chips = [];
  const fm = content.frontmatter || {};
  if (fm.status) chips.push(<StatusBadge key="s" status={fm.status}/>);
  if (fm.verdict) chips.push(<StatusBadge key="v" status={fm.verdict}/>);
  if (fm.date) chips.push(<Chip key="d" tone="neutral">{fm.date}</Chip>);
  if (fm.author) chips.push(<Chip key="a" tone="neutral">{fm.author}</Chip>);

  return (
    <div className="ac-page">
      <div className="ac-pagehead">
        <div className="ac-pagehead__l">
          <div className="ac-pagehead__eyebrow">
            <TypeGlyph type={content.type} size={16}/>
            {content.type} · {fm.slug || content.slug}
          </div>
          <h1>{fm.title}</h1>
          <div className="ac-pagehead__sub" style={{display:"flex",gap:6,alignItems:"center",marginTop:8,flexWrap:"wrap"}}>
            {chips}
          </div>
        </div>
        <div className="ac-pagehead__actions">
          <button className="ac-topbar__btn"><Icon name="edit" size={13}/> Open in editor</button>
          <button className="ac-topbar__btn"><Icon name="link" size={13}/> Copy link</button>
        </div>
      </div>

      <div className="ac-doc-split">
        <div>
          <div className="ac-fm">
            {Object.entries(fm).map(([k, v]) => (
              v != null && <React.Fragment key={k}>
                <div className="ac-fm__k">{k}:</div>
                <div className="ac-fm__v">{typeof v === "string" && v.startsWith("WORK") ? <a>{v}</a> : String(v)}</div>
              </React.Fragment>
            ))}
          </div>
          {content.type === "design-inventories" && (
            <div style={{margin:"4px 0 20px",padding:"14px",border:"1px solid var(--ac-stroke)",borderRadius:8,background:"var(--ac-bg-card)"}}>
              <div style={{fontFamily:"var(--ac-font-mono)",fontSize:11,letterSpacing:"0.1em",textTransform:"uppercase",color:"var(--ac-fg-faint)",marginBottom:10,display:"flex",justifyContent:"space-between"}}>
                <span>Captured screenshots</span><span>6 of 14 screens</span>
              </div>
              <div style={{display:"grid",gridTemplateColumns:"repeat(3,1fr)",gap:8}}>
                {["library / decisions","library / work","lifecycle index","lifecycle cluster","kanban board","templates / adr"].map((cap,i) => (
                  <div key={i} style={{border:"1px solid var(--ac-stroke)",borderRadius:6,overflow:"hidden",background:"linear-gradient(135deg, hsl(220 30% 96%), hsl(220 30% 90%))"}}>
                    <div style={{aspectRatio:"16/10",position:"relative",display:"flex",alignItems:"center",justifyContent:"center"}}>
                      <div style={{position:"absolute",inset:0,backgroundImage:"linear-gradient(var(--ac-stroke) 1px, transparent 1px), linear-gradient(90deg, var(--ac-stroke) 1px, transparent 1px)",backgroundSize:"24px 24px",opacity:0.35}}/>
                      <div style={{position:"absolute",top:8,left:8,width:32,height:6,background:"var(--ac-stroke)",borderRadius:3}}/>
                      <div style={{position:"absolute",top:8,left:44,width:60,height:6,background:"var(--ac-stroke)",borderRadius:3,opacity:0.6}}/>
                      <div style={{position:"absolute",bottom:10,left:8,right:8,height:24,background:"var(--ac-bg-card)",border:"1px solid var(--ac-stroke)",borderRadius:4,opacity:0.85}}/>
                      <Icon name="library" size={28} className="faint" style={{opacity:0.35}}/>
                    </div>
                    <div style={{padding:"6px 8px",fontFamily:"var(--ac-font-mono)",fontSize:10,color:"var(--ac-fg-muted)",borderTop:"1px solid var(--ac-stroke)",whiteSpace:"nowrap",overflow:"hidden",textOverflow:"ellipsis"}}>{cap}</div>
                  </div>
                ))}
              </div>
            </div>
          )}
          <div className="ac-md">{renderMarkdown(content.body)}</div>
        </div>

        <aside className="ac-doc-aside">
          <div className="ac-doc-aside__section">
            <h4>Related artifacts</h4>
            {related.length === 0 ? (
              <div className="faint" style={{fontSize:12.5}}>No cluster matches for this slug.</div>
            ) : (
              <div className="ac-related">
                {related.map((r, i) => (
                  <div key={i} className="ac-related__item"
                       onClick={() => setRoute({view:"library", type:r.type, docId:r.id, slug: content.slug})}>
                    <TypeGlyph type={r.type} size={22}/>
                    <div style={{minWidth:0}}>
                      <div className="ac-related__title" style={{overflow:"hidden",textOverflow:"ellipsis",whiteSpace:"nowrap"}}>{r.title}</div>
                      <div className="ac-related__meta">{r.id} · {r.mtime}<span className="ac-related__tag">(inferred)</span></div>
                    </div>
                    <Icon name="chevron-right" size={14} className="faint"/>
                  </div>
                ))}
              </div>
            )}
          </div>

          {fm.target && (
            <div className="ac-doc-aside__section" style={{marginTop:20,paddingTop:20,borderTop:"1px dashed var(--ac-stroke)"}}>
              <h4>Declared links</h4>
              <div className="ac-related">
                <div className="ac-related__item">
                  <TypeGlyph type="plans" size={22}/>
                  <div style={{minWidth:0}}>
                    <div className="ac-related__title">Reviews target plan</div>
                    <div className="ac-related__meta mono" style={{whiteSpace:"nowrap",overflow:"hidden",textOverflow:"ellipsis"}}>{fm.target}<span className="ac-related__tag is-declared">(declared)</span></div>
                  </div>
                </div>
              </div>
            </div>
          )}

          <div className="ac-doc-aside__section" style={{marginTop:20,paddingTop:20,borderTop:"1px dashed var(--ac-stroke)"}}>
            <h4>File</h4>
            <div className="mono faint" style={{fontSize:11,wordBreak:"break-all",lineHeight:1.5}}>
              meta/{content.type}/{docId || fm.date}-{content.slug}.md
            </div>
            <div className="mono faint" style={{fontSize:10.5,marginTop:8}}>etag · sha256-4f2a19…</div>
            <div className="mono faint" style={{fontSize:10.5}}>size · 4.2 KiB</div>
          </div>

          {cluster && (
            <div className="ac-doc-aside__section" style={{marginTop:20,paddingTop:20,borderTop:"1px dashed var(--ac-stroke)"}}>
              <h4>Cluster</h4>
              <button onClick={() => setRoute({view:"lifecycle", slug: cluster.slug, type:null, docId:null})}
                      className="ac-related__item" style={{width:"100%",textAlign:"left",border:"1px solid var(--ac-stroke)",padding:"8px 10px"}}>
                <Icon name="lifecycle" size={16} className="muted"/>
                <div>
                  <div className="ac-related__title">{cluster.title}</div>
                  <div className="ac-related__meta">{cluster.entries.length} artifacts · {cluster.updated}</div>
                </div>
                <Icon name="chevron-right" size={14} className="faint"/>
              </button>
            </div>
          )}
        </aside>
      </div>
    </div>
  );
}

Object.assign(window, { LibraryLanding, LibraryIndex, DocPage });
