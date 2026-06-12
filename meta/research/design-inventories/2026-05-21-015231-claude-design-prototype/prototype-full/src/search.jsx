// Sidebar search — implements work item 0054.
//
// • 200ms trailing-edge debounce
// • 2-character minimum
// • Substring match across title + slug + id, across the twelve LIBRARY doc types
//   (Templates excluded by structural omission of LIBRARY_INDEX.templates)
// • Inline results panel beneath the input
// • <a href> rows so modifier-click / middle-click / Enter all work natively
// • role="status" "No matches" empty state
// • / global keybind that early-returns while a text input / textarea /
//   contenteditable element has focus
// • Esc clears the query and blurs the input

const { useState: useStateS, useEffect: useEffectS, useMemo: useMemoS, useRef: useRefS } = React;

// ── 1. Build a flat search corpus from LIBRARY_INDEX, excluding Templates ──
// Each entry mirrors the response row shape pinned by the story:
//   { docType, title, slug, path, mtime_ms, id }
function buildCorpus() {
  const out = [];
  const LIB = window.LIBRARY_INDEX || {};
  const LIBRARY_TYPES = [
    "work", "work-reviews",
    "design-inventories", "design-gaps", "research",
    "plans", "plan-reviews", "validations",
    "pr-descriptions", "pr-reviews",
    "root-cause-analyses",
    "decisions", "notes",
  ];
  for (const t of LIBRARY_TYPES) {
    const rows = LIB[t] || [];
    for (const r of rows) {
      const id = r.id || r.name || r.slug || "untitled";
      const slug = r.slug || "";
      const title = r.title || id;
      const mtime_ms = r.date ? Date.parse(r.date) || 0 : 0;
      // Mirror the on-disk path scheme the server returns.
      let path;
      if (t === "work") path = `meta/work/${id}.md`;
      else if (t === "work-reviews") path = `meta/work-reviews/${id}.md`;
      else path = `meta/${t}/${r.date || ""}-${slug || id}.md`;
      out.push({ docType: t, title, slug, id, path, mtime_ms });
    }
  }
  return out;
}

// ── 2. Substring search + ranking ────────────────────────────────────────
// Initial ranking per the story's Technical Notes: case-insensitive substring
// match on title + slug (plus id, for things like "PROJ-0001"), ordered by
// mtime_ms descending, with `path` ascending as tiebreaker. We also boost
// title-prefix matches so typing "rev" surfaces reviews before random hits.
function rankCorpus(corpus, q) {
  const needle = q.trim().toLowerCase();
  if (needle.length < 2) return [];
  const hits = [];
  for (const e of corpus) {
    const t = (e.title || "").toLowerCase();
    const s = (e.slug || "").toLowerCase();
    const id = (e.id || "").toLowerCase();
    const titleHit = t.indexOf(needle);
    const slugHit  = s.indexOf(needle);
    const idHit    = id.indexOf(needle);
    if (titleHit < 0 && slugHit < 0 && idHit < 0) continue;
    // Lower score = better. Prefix on title beats slug beats anywhere.
    let score = 999;
    if (titleHit === 0) score = 0;
    else if (slugHit === 0) score = 1;
    else if (idHit === 0) score = 2;
    else if (titleHit >= 0) score = 10;
    else if (slugHit  >= 0) score = 20;
    else if (idHit    >= 0) score = 30;
    hits.push({ entry: e, score });
  }
  hits.sort((a, b) => (
    a.score - b.score ||
    (b.entry.mtime_ms - a.entry.mtime_ms) ||
    a.entry.path.localeCompare(b.entry.path)
  ));
  return hits.slice(0, 40).map(h => h.entry);
}

// ── 3. useDebouncedValue — 10-line net-new helper ────────────────────────
function useDebouncedValue(value, delayMs) {
  const [debounced, setDebounced] = useStateS(value);
  useEffectS(() => {
    const t = setTimeout(() => setDebounced(value), delayMs);
    return () => clearTimeout(t);
  }, [value, delayMs]);
  return debounced;
}

// ── 4. useSearch — composes debounce + corpus into the search results ────
//
// In production this would hit GET /api/search?q=<settled>; here we resolve
// against the in-memory corpus inside a microtask so the loading state has
// a moment to render and the dataflow matches the real-world async path.
function useSearch(query) {
  const corpus = useMemoS(() => buildCorpus(), []);
  const settled = useDebouncedValue(query.trim(), 200);
  const enabled = settled.length >= 2;
  const [state, setState] = useStateS({ status: "idle", data: [], q: "" });

  useEffectS(() => {
    if (!enabled) { setState({ status: "idle", data: [], q: settled }); return; }
    setState(s => ({ status: "loading", data: [], q: settled }));
    let cancelled = false;
    // 90ms ≈ a local network round-trip; lets the empty transitional area
    // render between every settled query and its results.
    const t = setTimeout(() => {
      if (cancelled) return;
      const data = rankCorpus(corpus, settled);
      setState({ status: "done", data, q: settled });
    }, 90);
    return () => { cancelled = true; clearTimeout(t); };
  }, [settled, enabled, corpus]);

  return state;
}

// ── 5. Highlight matched substring inside a title/slug ───────────────────
function Highlight({ text, q }) {
  if (!q || q.length < 2) return <>{text}</>;
  const lower = text.toLowerCase();
  const needle = q.toLowerCase();
  const i = lower.indexOf(needle);
  if (i < 0) return <>{text}</>;
  return (
    <>
      {text.slice(0, i)}
      <mark className="ac-search__mark">{text.slice(i, i + needle.length)}</mark>
      {text.slice(i + needle.length)}
    </>
  );
}

// ── 6. SearchBox — the full input + results panel ────────────────────────
function SearchBox({ setRoute, initialQuery = "" }) {
  const [q, setQ] = useStateS(initialQuery);
  const [focused, setFocused] = useStateS(false);
  const inputRef = useRefS(null);
  const result = useSearch(q);

  // Global "/" keybind. Early-return while focus is in any text input,
  // textarea, or contenteditable element (per AC1+AC2).
  useEffectS(() => {
    const onKey = (e) => {
      if (e.key === "Escape" && document.activeElement === inputRef.current) {
        setQ("");
        inputRef.current && inputRef.current.blur();
        return;
      }
      if (e.key !== "/") return;
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      const a = document.activeElement;
      const tag = a && a.tagName;
      const isEditable = a && (
        tag === "INPUT" || tag === "TEXTAREA" || a.isContentEditable
      );
      if (isEditable) return;          // AC2 — `/` passes through to editors
      e.preventDefault();              // AC1 — steal focus, swallow the `/`
      inputRef.current && inputRef.current.focus();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const open = q.trim().length >= 2;
  const settled = result.q;
  const showLoading = result.status === "loading";
  const showResults = result.status === "done" && result.data.length > 0;
  const showEmpty   = result.status === "done" && result.data.length === 0 && settled.length >= 2;

  const navigate = (entry, e) => {
    // Honor native link semantics: modifier-click / middle-click open in new
    // tab; we only intercept the plain primary-button click.
    if (e && (e.metaKey || e.ctrlKey || e.shiftKey || e.altKey || e.button === 1)) return;
    if (e) e.preventDefault();
    setRoute({ view: "library", type: entry.docType, docId: entry.id, slug: entry.slug || null });
    setQ("");
    inputRef.current && inputRef.current.blur();
  };

  return (
    <div className={`ac-sidebar__search ${open && focused ? "is-open" : ""}`}>
      <div className="ac-sidebar__search-input">
        <Icon name="search" size={14}/>
        <input
          ref={inputRef}
          type="search"
          placeholder="Search meta/…"
          value={q}
          onChange={e => setQ(e.target.value)}
          onFocus={() => setFocused(true)}
          onBlur={() => setTimeout(() => setFocused(false), 120)}
          aria-label="Search meta documents"
        />
        {q ? (
          <button className="ac-sidebar__search-clear" onClick={() => { setQ(""); inputRef.current && inputRef.current.focus(); }} title="Clear (Esc)" aria-label="Clear search">
            <Icon name="close" size={11}/>
          </button>
        ) : (
          <kbd>/</kbd>
        )}
      </div>

      {open && (
        <div className="ac-search__panel" role="region" aria-label="Search results">
          {showLoading && (
            <div className="ac-search__loading" aria-hidden="true">
              <span className="ac-search__loadbar"/>
              <span className="ac-search__loadhint">Searching meta/ for <span className="mono">{settled || q.trim()}</span>…</span>
            </div>
          )}
          {showResults && (
            <>
              <div className="ac-search__meta">
                <span><b>{result.data.length}</b> {result.data.length === 1 ? "match" : "matches"} · <span className="mono">{settled}</span></span>
                <span className="ac-search__hint" title="Enter opens, Esc clears"><kbd>↵</kbd><kbd>esc</kbd></span>
              </div>
              <div className="ac-search__list" role="listbox">
                {result.data.map((r, i) => {
                  const meta = window.TYPE_META[r.docType] || { label: r.docType, hue: 215 };
                  const href = `#/library/${r.docType}/${encodeURIComponent(r.id)}`;
                  return (
                    <a
                      key={`${r.docType}/${r.id}/${i}`}
                      href={href}
                      onClick={(e) => navigate(r, e)}
                      onMouseDown={(e) => { /* prevent input blur from eating the click */ e.preventDefault(); }}
                      className="ac-search__row"
                      role="option"
                      tabIndex={0}
                    >
                      <TypeGlyph type={r.docType} size={26}/>
                      <div className="ac-search__row-body">
                        <div className="ac-search__row-title">
                          <Highlight text={r.title} q={settled}/>
                        </div>
                        <div className="ac-search__row-sub">
                          <span className="ac-search__row-type" style={{color:`hsl(${meta.hue} 50% 42%)`}}>{meta.label}</span>
                          <span className="ac-search__row-sep">·</span>
                          <span className="mono ac-search__row-path">{r.path.replace(/^meta\//,"")}</span>
                        </div>
                      </div>
                      <Icon name="chevron-right" size={12} className="ac-search__row-chev"/>
                    </a>
                  );
                })}
              </div>
            </>
          )}
          {showEmpty && (
            <div className="ac-search__empty" role="status">
              <div className="ac-search__empty-title">No matches</div>
              <div className="ac-search__empty-body">
                Nothing in <span className="mono">meta/</span> matches <span className="mono">"{settled}"</span>. Try a slug, a doc id (e.g. <span className="mono">PROJ-0007</span>), or a fragment of a title.
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

Object.assign(window, { SearchBox, useSearch, useDebouncedValue, buildSearchCorpus: buildCorpus, rankSearchCorpus: rankCorpus });
