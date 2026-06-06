// App shell — sidebar + topbar + route dispatch.

const { useState: useStateShell, useEffect: useEffectShell } = React;

function Sidebar({ route, setRoute, initialSearch, onSecret }) {
  const inDev = route.view === "__dev__";
  const [clickCount, setClickCount] = useStateShell(0);
  const clickTimer = React.useRef(null);
  const onFootClick = () => {
    setClickCount(c => {
      const next = c + 1;
      if (next >= 3 && onSecret) {
        onSecret();
        return 0;
      }
      clearTimeout(clickTimer.current);
      clickTimer.current = setTimeout(() => setClickCount(0), 600);
      return next;
    });
  };
  const item = (def, activeMatch) => {
    const active = activeMatch(route);
    return (
      <div key={def.key}
           className={`ac-nav__item ${active ? "is-active" : ""}`}
           onClick={() => setRoute({ view: def.kind === "view" ? def.key : "library", type: def.kind === "doc" || def.kind === "meta" ? def.key : null, docId: null, slug: null })}>
        <span className="ac-nav__label-l">{def.label}</span>
        <span className="ac-nav__right">
          {def.pulse && <span className="ac-pulse" title="Unseen changes"/>}
          {def.count != null && <span className="ac-nav__count">{def.count}</span>}
        </span>
      </div>
    );
  };
  return (
    <aside className="ac-sidebar">
      <SearchBox setRoute={setRoute} initialQuery={initialSearch || ""}/>

      <div className="ac-nav">
        <div className="ac-nav__group">
          <div
            className={`ac-nav__label ac-nav__label--clickable ${route.view === "library" && !route.type ? "is-active" : ""}`}
            onClick={() => setRoute({ view: "library", type: null, docId: null, slug: null })}
            role="button"
            tabIndex={0}>
            <span>Library</span>
            <span className="ac-nav__label-hint" aria-hidden="true">All</span>
          </div>
          {window.LIBRARY_GROUPS.map(group => (
            <div key={group.key} className="ac-nav__subgroup">
              <div className="ac-nav__sublabel">{group.label}</div>
              {group.types.map(typeKey => {
                const d = window.DOC_TYPES.find(x => x.key === typeKey);
                return d ? item(d, r => r.view === "library" && r.type === d.key) : null;
              })}
            </div>
          ))}
        </div>

        <div className="ac-nav__group">
          <div className="ac-nav__label"><span>Views</span></div>
          {window.VIEWS.map(v => (
            <div key={v.key}
                 className={`ac-nav__item ${route.view === v.key ? "is-active" : ""}`}
                 onClick={() => setRoute({ view: v.key, type: null, docId: null, slug: null })}>
              <span className="ac-nav__label-l">
                <Icon name={v.key === "kanban" ? "kanban" : "lifecycle"} size={14}/>
                {v.label}
              </span>
            </div>
          ))}
        </div>

        <div className="ac-nav__group">
          <div className="ac-nav__label"><span>Activity</span><span className="ac-nav__count">live</span></div>
          <div className="ac-activity">
            {window.ACTIVITY.slice(0, 5).map((a, i) => {
              const def = window.DOC_TYPES.find(d => d.key === a.type) || window.META.find(d => d.key === a.type);
              const label = def ? def.label : a.type;
              const glyph = window.TYPE_ICONS[a.type];
              return (
                <div className="ac-activity__item" key={i}
                     onClick={() => setRoute({ view: "lifecycle", type: null, slug: a.slug, docId: null })}>
                  <span className="ac-activity__glyph" aria-hidden="true">
                    {glyph ? (
                      <svg viewBox="0 0 24 24" width="14" height="14">
                        {glyph}
                      </svg>
                    ) : null}
                  </span>
                  <div>
                    <div className="ac-activity__line1">
                      {label} <span className="ac-activity__action">· {a.action}</span>
                    </div>
                    <div className="ac-activity__line2">{a.doc} · {a.when}</div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        <div className="ac-nav__meta">
          <div className="ac-nav__group">
            <div className="ac-nav__label"><span>Meta</span></div>
            {window.META.map(m => item(m, r => r.view === "library" && r.type === m.key))}
          </div>
        </div>
      </div>

      <div className="ac-sidebar__foot"
           onClick={onFootClick}
           title="triple-click for /#dev"
           style={{ cursor: "default", userSelect: "none" }}>
        <div>accelerator-visualiser</div>
        <div>v0.4.1 · embed-dist {inDev && <span style={{color: "var(--ac-accent)", marginLeft: 4}}>· dev</span>}</div>
      </div>
    </aside>
  );
}

function Topbar({ route, setRoute, theme, setTheme, fontMode, setFontMode }) {
  const crumbs = [];
  if (route.view === "library") {
    crumbs.push({ label: "Library", onClick: () => setRoute({ view: "library", type: null, docId: null, slug: null }) });
    if (route.type) crumbs.push({ label: route.type, onClick: () => setRoute({ ...route, docId: null }) });
    if (route.docId) crumbs.push({ label: route.docId });
  } else if (route.view === "lifecycle") {
    crumbs.push({ label: "Lifecycle", onClick: () => setRoute({ view: "lifecycle", type: null, slug: null, docId: null }) });
    if (route.slug) crumbs.push({ label: route.slug });
  } else if (route.view === "kanban") {
    crumbs.push({ label: "Kanban" });
  } else if (route.view === "__dev__") {
    crumbs.push({ label: "dev", onClick: () => setRoute({ view: "lifecycle", type: null, slug: "meta-visualisation", docId: null }) });
    crumbs.push({ label: "design system" });
  }

  return (
    <header className="ac-topbar">
      <div className="ac-topbar__brand">
        <AtomicMark size={24}/>
        <div className="ac-topbar__brand-text">
          <span className="ac-topbar__brand-name">Accelerator</span>
          <span className="ac-topbar__brand-sub">VISUALISER</span>
        </div>
      </div>
      <div className="ac-topbar__sep"/>
      <div className="ac-topbar__crumbs">
        {crumbs.map((c, i) => (
          <React.Fragment key={i}>
            {i > 0 && <Icon name="chevron-right" size={12}/>}
            {c.onClick
              ? <button onClick={c.onClick} className={i === crumbs.length - 1 ? "" : "muted"}>{i === crumbs.length - 1 ? <strong>{c.label}</strong> : c.label}</button>
              : <strong>{c.label}</strong>}
          </React.Fragment>
        ))}
      </div>
      <div className="ac-topbar__spacer"/>
      <div className="ac-topbar__status">
        <span className="dot"/> 127.0.0.1:52914
      </div>
      <div className="ac-topbar__status">
        <Icon name="activity" size={12} style={{color:"var(--ac-ok)"}}/> SSE
      </div>
      <button className="ac-topbar__btn" onClick={() => setTheme(theme === "dark" ? "light" : "dark")} title="Toggle theme">
        <Icon name={theme === "dark" ? "sun" : "moon"} size={14}/>
      </button>
    </header>
  );
}

Object.assign(window, { Sidebar, Topbar });
