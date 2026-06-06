// Hidden developer route — design system reference.
//
// Not exposed in the sidebar. Accessible via:
//   • URL hash:        #dev   (also #dev/<section>)
//   • Keyboard:        Cmd/Ctrl + Shift + D
//   • Sidebar version: triple-click the version string in the sidebar foot
//
// Surfaces every primitive used in the Accelerator visualiser so you can
// eyeball colour-token drift, icon coverage, etc. Single scrollable page
// broken into labelled sections.

const { useState: useStateDev, useEffect: useEffectDev, useMemo: useMemoDev } = React;

// ─── Section list (used for jump nav) ──────────────────────────────────────
const DEV_SECTIONS = [
  { id: "overview",   label: "Overview" },
  { id: "colors",     label: "Colours" },
  { id: "type",       label: "Type" },
  { id: "spacing",    label: "Spacing" },
  { id: "radii",      label: "Radii & shadows" },
  { id: "icons",      label: "Icons" },
  { id: "glyphs",     label: "Doc-type glyphs" },
  { id: "bigglyphs",  label: "Empty-state glyphs" },
  { id: "mark",       label: "Atomic mark" },
  { id: "chips",      label: "Chips" },
  { id: "badges",     label: "Status badges" },
  { id: "stagedots",  label: "Stage dots" },
  { id: "tierpills",  label: "Tier pills" },
  { id: "buttons",    label: "Buttons" },
  { id: "form",       label: "Inputs & form" },
  { id: "nav",        label: "Sidebar nav" },
  { id: "cards",      label: "Cards" },
  { id: "table",      label: "Tables" },
  { id: "markdown",   label: "Markdown" },
  { id: "code",       label: "Code blocks" },
  { id: "frontmatter",label: "Frontmatter" },
  { id: "empty",      label: "Empty & banners" },
  { id: "toast",      label: "Toasts" },
  { id: "topbar",     label: "Topbar" },
];

// ─── Section wrapper ───────────────────────────────────────────────────────
function DSSection({ id, title, hint, children }) {
  return (
    <section id={`ds-${id}`} className="ds-section">
      <header className="ds-section__head">
        <div className="ds-section__head-l">
          <span className="ds-section__id">§ {id}</span>
          <h2 className="ds-section__title">{title}</h2>
        </div>
        {hint && <div className="ds-section__hint">{hint}</div>}
      </header>
      <div className="ds-section__body">{children}</div>
    </section>
  );
}

// A small labelled cell used to show "this is the live thing, this is its name"
function DSSpec({ name, mono, children, span = 1 }) {
  return (
    <div className="ds-spec" style={{ gridColumn: `span ${span}` }}>
      <div className="ds-spec__preview">{children}</div>
      <div className="ds-spec__meta">
        <span className="ds-spec__name">{name}</span>
        {mono && <span className="ds-spec__mono">{mono}</span>}
      </div>
    </div>
  );
}

// ─── Colour swatch ─────────────────────────────────────────────────────────
function Swatch({ token, label, note }) {
  const [hex, setHex] = useStateDev("");
  const ref = React.useRef(null);
  useEffectDev(() => {
    if (!ref.current) return;
    const v = getComputedStyle(ref.current).backgroundColor;
    setHex(v);
  }, [token]);
  return (
    <div className="ds-swatch">
      <div ref={ref} className="ds-swatch__chip" style={{ background: `var(${token})` }}/>
      <div className="ds-swatch__meta">
        <div className="ds-swatch__name">{label || token}</div>
        <div className="ds-swatch__token mono">{token}</div>
        {hex && <div className="ds-swatch__hex mono">{hex}</div>}
        {note && <div className="ds-swatch__note">{note}</div>}
      </div>
    </div>
  );
}

// ─── Dev page ──────────────────────────────────────────────────────────────
function DevDesignSystem({ setRoute, theme, setTheme }) {
  const [section, setSection] = useStateDev(() => {
    const h = location.hash.replace(/^#/, "").split("/");
    return h[1] || "overview";
  });

  // Sync scroll-spy: update hash when a section enters view. The scroll
  // container is .ac-main (not the viewport), so pass it as the observer
  // root or the callback never fires.
  useEffectDev(() => {
    const main = document.querySelector(".ac-main");
    if (!main) return;
    const ids = DEV_SECTIONS.map(s => `ds-${s.id}`);
    const observer = new IntersectionObserver((entries) => {
      const visible = entries
        .filter(e => e.isIntersecting)
        .sort((a, b) => b.intersectionRatio - a.intersectionRatio)[0];
      if (visible) {
        const id = visible.target.id.replace(/^ds-/, "");
        setSection(id);
        history.replaceState(null, "", `#dev/${id}`);
      }
    }, { root: main, rootMargin: "-80px 0px -55% 0px", threshold: [0, 0.25, 0.5] });
    ids.forEach(id => {
      const el = document.getElementById(id);
      if (el) observer.observe(el);
    });
    return () => observer.disconnect();
  }, []);

  // Click a jumplink → scroll the .ac-main container.
  const jump = (id) => {
    const el = document.getElementById(`ds-${id}`);
    const main = document.querySelector(".ac-main");
    if (el && main) {
      main.scrollTo({ top: el.offsetTop - 60, behavior: "smooth" });
    }
  };

  // Icon names from the Icon component
  const ICON_NAMES = [
    "search","library","kanban","lifecycle","activity","clock","link",
    "chevron-right","chevron-down","chevron-left",
    "doc","edit","close","check","dot","plus","minus",
    "git-pr","git-branch","filter","sort","sparkle","hex","shield",
    "moon","sun","settings","terminal","arrow-right","flag","folder","layers","alert",
  ];

  // Demo toasts for the toast section
  const [demoToasts, setDemoToasts] = useStateDev([
    { id: 1, kind: "ok",   title: "Snapshot saved",     body: "Cluster pinned at commit 9af2c1." },
    { id: 2, kind: "warn", title: "External edit detected", body: <>A reviewer agent updated <code>WORK-0007</code>.</> },
    { id: 3, kind: "err",  title: "Indexer crashed",    body: "Validation pipeline returned exit code 42." },
  ]);

  return (
    <div className="ds-root">
      {/* Marquee header — makes it unmistakeable this is a dev page */}
      <div className="ds-marquee">
        <div className="ds-marquee__inner">
          <span className="ds-marquee__tag">DEV</span>
          <span className="ds-marquee__sep">·</span>
          <span className="ds-marquee__title">Design system reference</span>
          <span className="ds-marquee__sep">·</span>
          <span className="ds-marquee__route mono">/#dev</span>
          <span className="ds-marquee__sep">·</span>
          <span className="ds-marquee__kbd">⌘⇧D toggles</span>
          <span className="ds-marquee__sep">·</span>
          <span>Not exposed in the sidebar — share the link with the team</span>
          <span className="ds-marquee__sep">·</span>
          <span className="ds-marquee__tag">DEV</span>
          <span className="ds-marquee__sep">·</span>
          <span className="ds-marquee__title">Design system reference</span>
          <span className="ds-marquee__sep">·</span>
          <span className="ds-marquee__route mono">/#dev</span>
          <span className="ds-marquee__sep">·</span>
        </div>
      </div>

      <div className="ds-layout">
        {/* Sticky inner sidebar — section jumplinks */}
        <aside className="ds-tocaside">
          <div className="ds-tocaside__head">
            <div className="ds-tocaside__eyebrow mono">CONTENTS</div>
            <button className="ds-tocaside__exit" onClick={() => setRoute({ view: "lifecycle", type: null, slug: "meta-visualisation", docId: null })}>
              <Icon name="arrow-right" size={12}/> exit to app
            </button>
          </div>
          <nav className="ds-toc">
            {DEV_SECTIONS.map(s => (
              <a key={s.id}
                 href={`#dev/${s.id}`}
                 className={`ds-toc__item ${section === s.id ? "is-active" : ""}`}
                 onClick={(e) => { e.preventDefault(); jump(s.id); }}>
                <span className="ds-toc__num mono">{String(DEV_SECTIONS.indexOf(s) + 1).padStart(2, "0")}</span>
                <span>{s.label}</span>
              </a>
            ))}
          </nav>
          <div className="ds-tocaside__foot mono">
            <div>accelerator-visualiser</div>
            <div className="faint">design-system @ {new Date().toISOString().slice(0,10)}</div>
          </div>
        </aside>

        <div className="ds-content">
          <DSSection id="overview" title="Overview">
            <p className="ds-prose">
              This is a private inventory of every visual primitive currently used by
              the Accelerator visualiser prototype. It exists so engineers can spot
              token drift, missing variants, or surface a component they didn't know
              already existed.
            </p>
            <p className="ds-prose">
              Tokens are drawn from <code>tokens.css</code> (Atomic brand) and the
              app-layer overrides in <code>src/app.css</code>. Components live in
              <code> src/ui.jsx</code> and the per-view files.
            </p>
            <div className="ds-overview-grid">
              <div className="ds-overview-card">
                <div className="ds-overview-card__num mono">03</div>
                <div className="ds-overview-card__lbl">font families</div>
                <div className="ds-overview-card__sub">Sora · Inter · Fira Code</div>
              </div>
              <div className="ds-overview-card">
                <div className="ds-overview-card__num mono">{ICON_NAMES.length}</div>
                <div className="ds-overview-card__lbl">stroke icons</div>
                <div className="ds-overview-card__sub">Feather-style, 2px stroke, currentColor</div>
              </div>
              <div className="ds-overview-card">
                <div className="ds-overview-card__num mono">{Object.keys(window.TYPE_META).length}</div>
                <div className="ds-overview-card__lbl">doc-type glyphs</div>
                <div className="ds-overview-card__sub">Hue-tinted square + line drawing per type</div>
              </div>
              <div className="ds-overview-card">
                <div className="ds-overview-card__num mono">02</div>
                <div className="ds-overview-card__lbl">themes</div>
                <div className="ds-overview-card__sub">
                  <button className="ds-link" onClick={() => setTheme(theme === "dark" ? "light" : "dark")}>
                    flip to {theme === "dark" ? "light" : "dark"} →
                  </button>
                </div>
              </div>
            </div>
          </DSSection>

          {/* ──────────────────── COLOURS ───────────────────── */}
          <DSSection id="colors" title="Colours" hint="from app.css · respond to data-theme">
            <h3 className="ds-h3">Surfaces</h3>
            <div className="ds-swatches">
              <Swatch token="--ac-bg"         label="Page"/>
              <Swatch token="--ac-bg-raised"  label="Raised"/>
              <Swatch token="--ac-bg-sunken"  label="Sunken"/>
              <Swatch token="--ac-bg-chrome"  label="Chrome"/>
              <Swatch token="--ac-bg-sidebar" label="Sidebar"/>
              <Swatch token="--ac-bg-card"    label="Card"/>
              <Swatch token="--ac-bg-hover"   label="Hover"/>
              <Swatch token="--ac-bg-active"  label="Active"/>
            </div>

            <h3 className="ds-h3">Foreground</h3>
            <div className="ds-swatches">
              <Swatch token="--ac-fg"         label="Body"/>
              <Swatch token="--ac-fg-strong"  label="Strong"/>
              <Swatch token="--ac-fg-muted"   label="Muted"/>
              <Swatch token="--ac-fg-faint"   label="Faint"/>
            </div>

            <h3 className="ds-h3">Accent &amp; status</h3>
            <div className="ds-swatches">
              <Swatch token="--ac-accent"        label="Accent (indigo)"/>
              <Swatch token="--ac-accent-2"      label="Accent 2 (red)"/>
              <Swatch token="--ac-accent-tint"   label="Accent tint"/>
              <Swatch token="--ac-accent-faint"  label="Accent faint"/>
              <Swatch token="--ac-ok"            label="OK"/>
              <Swatch token="--ac-warn"          label="Warn"/>
              <Swatch token="--ac-err"           label="Error"/>
              <Swatch token="--ac-violet"        label="Violet"/>
            </div>

            <h3 className="ds-h3">Strokes</h3>
            <div className="ds-swatches">
              <Swatch token="--ac-stroke"        label="Default"/>
              <Swatch token="--ac-stroke-soft"   label="Soft"/>
              <Swatch token="--ac-stroke-strong" label="Strong"/>
            </div>

            <h3 className="ds-h3">Brand palette <span className="faint">(tokens.css)</span></h3>
            <div className="ds-swatches">
              <Swatch token="--atomic-night"          label="Night"/>
              <Swatch token="--atomic-night-2"        label="Night 2"/>
              <Swatch token="--atomic-night-3"        label="Night 3"/>
              <Swatch token="--atomic-ink"            label="Ink"/>
              <Swatch token="--atomic-red"            label="Atomic red"/>
              <Swatch token="--atomic-red-2"          label="Red hover"/>
              <Swatch token="--atomic-indigo"         label="Indigo"/>
              <Swatch token="--atomic-indigo-tint"    label="Indigo tint"/>
              <Swatch token="--atomic-medium-purple"  label="Medium purple"/>
              <Swatch token="--atomic-cream-can"      label="Cream can"/>
              <Swatch token="--atomic-steel-blue"     label="Steel blue"/>
              <Swatch token="--atomic-pastel-green"   label="Pastel green"/>
              <Swatch token="--atomic-aquamarine"     label="Aquamarine"/>
              <Swatch token="--atomic-tradewind"      label="Tradewind"/>
              <Swatch token="--atomic-malibu"         label="Malibu"/>
              <Swatch token="--atomic-marigold"       label="Marigold"/>
              <Swatch token="--atomic-bone"           label="Bone"/>
              <Swatch token="--atomic-ash"            label="Ash"/>
              <Swatch token="--atomic-slate"          label="Slate"/>
            </div>

            <h3 className="ds-h3">Doc-type hues</h3>
            <div className="ds-typehues">
              {Object.entries(window.TYPE_META).map(([key, meta]) => (
                <div key={key} className="ds-typehue">
                  <div className="ds-typehue__chip" style={{ background: `hsl(${meta.hue} 68% 56%)` }}/>
                  <div className="ds-typehue__meta">
                    <div className="ds-typehue__name">{meta.label}</div>
                    <div className="mono faint">hue {meta.hue}</div>
                  </div>
                </div>
              ))}
            </div>
          </DSSection>

          {/* ──────────────────── TYPE ───────────────────── */}
          <DSSection id="type" title="Type" hint="Sora · Inter · Fira Code">
            <div className="ds-typesamples">
              <div className="ds-typesample">
                <div className="ds-typesample__bg" style={{ fontFamily: "var(--ac-font-display)", fontWeight: 600, fontSize: 56, lineHeight: 1.05, letterSpacing: "-0.02em", color: "var(--ac-fg-strong)" }}>
                  Sora 56 / 600
                </div>
                <div className="ds-typesample__meta mono">var(--ac-font-display) · h1 hero</div>
              </div>
              <div className="ds-typesample">
                <div className="ds-typesample__bg" style={{ fontFamily: "var(--ac-font-display)", fontWeight: 600, fontSize: 28, lineHeight: 1.15, letterSpacing: "-0.01em", color: "var(--ac-fg-strong)" }}>
                  Sora 28 / 600 — page title
                </div>
                <div className="ds-typesample__meta mono">page-head h1 · .ac-pagehead h1</div>
              </div>
              <div className="ds-typesample">
                <div className="ds-typesample__bg" style={{ fontFamily: "var(--ac-font-display)", fontWeight: 600, fontSize: 18, color: "var(--ac-fg-strong)" }}>
                  Sora 18 / 600 — section heading
                </div>
                <div className="ds-typesample__meta mono">.ac-md-h2</div>
              </div>
              <div className="ds-typesample">
                <div className="ds-typesample__bg" style={{ fontFamily: "var(--ac-font-body)", fontWeight: 400, fontSize: 14.5, lineHeight: 1.65, color: "var(--ac-fg)" }}>
                  Inter 14.5 / 400 — body text. The Accelerator visualiser uses Inter for everything that isn't a heading or code, with mono-spaced metadata in Fira Code interleaved at smaller sizes.
                </div>
                <div className="ds-typesample__meta mono">.ac-md p · default body</div>
              </div>
              <div className="ds-typesample">
                <div className="ds-typesample__bg" style={{ fontFamily: "var(--ac-font-body)", fontWeight: 500, fontSize: 13, color: "var(--ac-fg-strong)" }}>
                  Inter 13 / 500 — UI label
                </div>
                <div className="ds-typesample__meta mono">sidebar nav · row labels</div>
              </div>
              <div className="ds-typesample">
                <div className="ds-typesample__bg mono" style={{ fontSize: 12, color: "var(--ac-fg-muted)" }}>
                  Fira Code 12 — PR-0042 · 14d ago · /meta/work/PROJ-0001.md
                </div>
                <div className="ds-typesample__meta mono">.mono · metadata</div>
              </div>
              <div className="ds-typesample">
                <div className="ds-typesample__bg mono" style={{ fontSize: 10.5, letterSpacing: "0.12em", textTransform: "uppercase", color: "var(--ac-fg-faint)" }}>
                  Fira Code 10.5 — EYEBROW LABEL
                </div>
                <div className="ds-typesample__meta mono">.ac-nav__label · .ds-overview-card__lbl</div>
              </div>
            </div>
          </DSSection>

          {/* ──────────────────── SPACING ───────────────────── */}
          <DSSection id="spacing" title="Spacing scale" hint="--sp-1 .. --sp-11">
            <div className="ds-spacingrow">
              {[
                { t: "--sp-1",  v: 4   }, { t: "--sp-2",  v: 8   }, { t: "--sp-3",  v: 12  },
                { t: "--sp-4",  v: 16  }, { t: "--sp-5",  v: 24  }, { t: "--sp-6",  v: 32  },
                { t: "--sp-7",  v: 40  }, { t: "--sp-8",  v: 48  }, { t: "--sp-9",  v: 64  },
                { t: "--sp-10", v: 80  }, { t: "--sp-11", v: 124 },
              ].map(s => (
                <div key={s.t} className="ds-spacingrow__cell">
                  <div className="ds-spacingrow__bar" style={{ width: s.v }}/>
                  <div className="ds-spacingrow__lbl mono">{s.t}</div>
                  <div className="ds-spacingrow__val mono faint">{s.v}px</div>
                </div>
              ))}
            </div>
          </DSSection>

          {/* ──────────────────── RADII & SHADOWS ───────────────────── */}
          <DSSection id="radii" title="Radii &amp; shadows">
            <h3 className="ds-h3">Corner radii</h3>
            <div className="ds-radii">
              {[
                { t: "--radius-sm",   v: "4px",   note: "buttons, cards, chips" },
                { t: "--radius-md",   v: "8px",   note: "containers" },
                { t: "--radius-lg",   v: "12px",  note: "softer surfaces" },
                { t: "--radius-pill", v: "999px", note: "chips, badges" },
              ].map(r => (
                <div key={r.t} className="ds-radii__cell">
                  <div className="ds-radii__box" style={{ borderRadius: `var(${r.t})` }}/>
                  <div className="ds-radii__lbl mono">{r.t}</div>
                  <div className="ds-radii__val mono faint">{r.v}</div>
                  <div className="ds-radii__note">{r.note}</div>
                </div>
              ))}
            </div>

            <h3 className="ds-h3">Shadows</h3>
            <div className="ds-shadows">
              <div className="ds-shadow ds-shadow--soft">
                <span>soft</span>
              </div>
              <div className="ds-shadow ds-shadow--lift">
                <span>lift</span>
              </div>
              <div className="ds-shadow ds-shadow--brand">
                <span>brand (--shadow-card)</span>
              </div>
            </div>
          </DSSection>

          {/* ──────────────────── ICONS ───────────────────── */}
          <DSSection id="icons" title="Icons" hint={`${ICON_NAMES.length} stroke icons · <Icon name="..." size={N} />`}>
            <div className="ds-iconsgrid">
              {ICON_NAMES.map(name => (
                <div key={name} className="ds-iconcell" title={name}>
                  <div className="ds-iconcell__chip"><Icon name={name} size={20}/></div>
                  <div className="ds-iconcell__name mono">{name}</div>
                </div>
              ))}
            </div>

            <h3 className="ds-h3">Sizes</h3>
            <div className="ds-iconsizes">
              {[12, 14, 16, 18, 20, 24, 28, 32].map(sz => (
                <div key={sz} className="ds-iconsizes__cell">
                  <Icon name="hex" size={sz}/>
                  <div className="mono faint">{sz}px</div>
                </div>
              ))}
            </div>
          </DSSection>

          {/* ──────────────────── DOC-TYPE GLYPHS ───────────────────── */}
          <DSSection id="glyphs" title="Doc-type glyphs" hint={`${Object.keys(window.TYPE_META).length} types · <TypeGlyph type="..."/>`}>
            <div className="ds-glyphs">
              {Object.entries(window.TYPE_META).map(([key, meta]) => (
                <div key={key} className="ds-glyphcell">
                  <div className="ds-glyphcell__row">
                    <TypeGlyph type={key} size={48}/>
                    <div className="ds-glyphcell__meta">
                      <div className="ds-glyphcell__name">{meta.label}</div>
                      <div className="mono faint">{key} · {meta.short}</div>
                    </div>
                  </div>
                  <div className="ds-glyphcell__sizes">
                    <TypeGlyph type={key} size={22}/>
                    <TypeGlyph type={key} size={28}/>
                    <TypeGlyph type={key} size={36}/>
                  </div>
                </div>
              ))}
            </div>
          </DSSection>

          {/* ──────────────────── MARK ───────────────────── */}
          <DSSection id="bigglyphs" title="Empty-state glyphs" hint='<BigGlyph type="..." size={N}/>'>
            <p className="ds-prose">
              Hero illustrations for per-type empty pages. Each is hue-tinted using
              <code> TYPE_META[type].hue</code> and rendered at 80×80 viewBox so it scales
              cleanly between 64–128px. Used in <code>view-empty.jsx</code>.
            </p>
            <div className="ds-bigglyphs">
              {Object.entries(window.TYPE_META).map(([key, meta]) => (
                <div key={key} className="ds-bigglyph-cell" style={{ "--bg-hue": meta.hue }}>
                  <div className="ds-bigglyph-cell__hero">
                    <BigGlyph type={key} size={96}/>
                  </div>
                  <div className="ds-bigglyph-cell__meta">
                    <div className="ds-bigglyph-cell__name">{meta.label}</div>
                    <div className="mono faint">{key}</div>
                  </div>
                </div>
              ))}
            </div>

            <h3 className="ds-h3">Sizes</h3>
            <div className="ds-bigglyph-sizes">
              {[48, 64, 80, 96, 128].map(sz => (
                <div key={sz} className="ds-bigglyph-sizes__cell">
                  <BigGlyph type="plans" size={sz}/>
                  <div className="mono faint">{sz}px</div>
                </div>
              ))}
            </div>
          </DSSection>

          <DSSection id="mark" title="Atomic mark">
            <div className="ds-marks">
              <div className="ds-marks__cell"><AtomicMark size={20}/><div className="mono faint">20</div></div>
              <div className="ds-marks__cell"><AtomicMark size={24}/><div className="mono faint">24 · topbar</div></div>
              <div className="ds-marks__cell"><AtomicMark size={32}/><div className="mono faint">32</div></div>
              <div className="ds-marks__cell"><AtomicMark size={48}/><div className="mono faint">48</div></div>
              <div className="ds-marks__cell"><AtomicMark size={72}/><div className="mono faint">72</div></div>
              <div className="ds-marks__cell ds-marks__cell--dark"><AtomicMark size={48}/><div className="mono faint">on night</div></div>
            </div>
          </DSSection>

          {/* ──────────────────── CHIPS ───────────────────── */}
          <DSSection id="chips" title="Chips" hint='<Chip tone="..." size="sm|md">'>
            <div className="ds-row">
              <Chip>neutral</Chip>
              <Chip tone="indigo">indigo</Chip>
              <Chip tone="green">green</Chip>
              <Chip tone="amber">amber</Chip>
              <Chip tone="red">red</Chip>
              <Chip tone="violet">violet</Chip>
            </div>
            <h3 className="ds-h3">md size</h3>
            <div className="ds-row">
              <Chip size="md">neutral</Chip>
              <Chip size="md" tone="indigo">indigo</Chip>
              <Chip size="md" tone="green">green</Chip>
              <Chip size="md" tone="amber">amber</Chip>
              <Chip size="md" tone="red">red</Chip>
              <Chip size="md" tone="violet">violet</Chip>
            </div>
          </DSSection>

          {/* ──────────────────── BADGES ───────────────────── */}
          <DSSection id="badges" title="Status badges" hint='<StatusBadge status="..."/>'>
            <div className="ds-row">
              <StatusBadge status="todo"/>
              <StatusBadge status="in-progress"/>
              <StatusBadge status="done"/>
              <StatusBadge status="draft"/>
              <StatusBadge status="accepted"/>
              <StatusBadge status="proposed"/>
              <StatusBadge status="open"/>
              <StatusBadge status="merged"/>
            </div>
            <h3 className="ds-h3">Verdicts</h3>
            <div className="ds-row">
              <StatusBadge status="approve"/>
              <StatusBadge status="approve-with-changes"/>
              <StatusBadge status="request-changes"/>
              <StatusBadge status="pass"/>
            </div>
          </DSSection>

          {/* ──────────────────── STAGE DOTS ───────────────────── */}
          <DSSection id="stagedots" title="Stage dots" hint="lifecycle pipeline presence">
            <div className="ds-row">
              <PipelineMini stages={window.STAGES} present={window.STAGES.map(s => s.key)}/>
              <span className="faint">all present</span>
            </div>
            <div className="ds-row">
              <PipelineMini stages={window.STAGES} present={["work","research","plans"]}/>
              <span className="faint">partial</span>
            </div>
            <div className="ds-row">
              <PipelineMini stages={window.STAGES} present={[]}/>
              <span className="faint">none</span>
            </div>
            <h3 className="ds-h3">Compact</h3>
            <div className="ds-row">
              <PipelineMini stages={window.STAGES} present={["work","plans","decisions"]} compact/>
            </div>
          </DSSection>

          {/* ──────────────────── TIER PILLS ───────────────────── */}
          <DSSection id="tierpills" title="Tier pills" hint="template/lifecycle presence pills">
            <div className="ds-row">
              <span className="ac-tier-pill is-present"><span className="dot"/>present</span>
              <span className="ac-tier-pill is-active"><span className="dot"/>active</span>
              <span className="ac-tier-pill is-absent"><span className="dot"/>absent</span>
              <span className="ac-tier-pill"><span className="dot"/>default</span>
            </div>
          </DSSection>

          {/* ──────────────────── BUTTONS ───────────────────── */}
          <DSSection id="buttons" title="Buttons">
            <h3 className="ds-h3">Topbar buttons</h3>
            <div className="ds-row">
              <button className="ac-topbar__btn"><Icon name="moon" size={14}/></button>
              <button className="ac-topbar__btn"><Icon name="filter" size={14}/> Filter</button>
              <button className="ac-topbar__btn ac-sort-btn"><Icon name="sort" size={14}/> Sort</button>
              <button className="ac-topbar__btn ac-sort-btn is-active"><Icon name="sort" size={14}/> updated ↓</button>
            </div>

            <h3 className="ds-h3">Filter badge</h3>
            <div className="ds-row">
              <button className="ac-topbar__btn ac-sort-btn"><Icon name="filter" size={14}/> Filter <span className="ac-filter__badge">3</span></button>
            </div>

            <h3 className="ds-h3">Link</h3>
            <div className="ds-row">
              <a href="#" onClick={(e)=>e.preventDefault()}>standard inline link</a>
              <button className="ds-link">ds-link button</button>
            </div>
          </DSSection>

          {/* ──────────────────── FORM ───────────────────── */}
          <DSSection id="form" title="Inputs &amp; form">
            <h3 className="ds-h3">Search input (sidebar style)</h3>
            <div style={{ maxWidth: 240 }}>
              <div className="ac-sidebar__search">
                <div className="ac-sidebar__search-input" style={{position:"relative"}}>
                  <Icon name="search" size={14}/>
                  <input placeholder="Search docs…" defaultValue=""/>
                  <kbd>⌘K</kbd>
                </div>
              </div>
            </div>

            <h3 className="ds-h3">Checkboxes</h3>
            <div className="ds-row" style={{ flexDirection: "column", alignItems: "flex-start", gap: 4 }}>
              <label className="ac-filter__opt" style={{minWidth: 220}}>
                <input type="checkbox" defaultChecked/>
                <span>checked</span>
                <span className="ac-filter__opt-count">7</span>
              </label>
              <label className="ac-filter__opt" style={{minWidth: 220}}>
                <input type="checkbox"/>
                <span>unchecked</span>
                <span className="ac-filter__opt-count">12</span>
              </label>
            </div>
          </DSSection>

          {/* ──────────────────── SIDEBAR NAV ───────────────────── */}
          <DSSection id="nav" title="Sidebar nav items">
            <div className="ds-navdemo">
              <div className="ac-nav" style={{padding:0}}>
                <div className="ac-nav__label"><span>GROUP LABEL</span></div>
                <div className="ac-nav__sublabel">Sub-group</div>
                <div className="ac-nav__item">
                  <span className="ac-nav__label-l"><Icon name="doc" size={14}/> default item</span>
                  <span className="ac-nav__right"><span className="ac-nav__count">12</span></span>
                </div>
                <div className="ac-nav__item is-active">
                  <span className="ac-nav__label-l"><Icon name="doc" size={14}/> active item</span>
                  <span className="ac-nav__right"><span className="ac-nav__count">8</span></span>
                </div>
                <div className="ac-nav__item">
                  <span className="ac-nav__label-l"><Icon name="doc" size={14}/> with pulse</span>
                  <span className="ac-nav__right"><span className="ac-pulse"/><span className="ac-nav__count">3</span></span>
                </div>
                <div className="ac-nav__item" style={{opacity: 0.6}}>
                  <span className="ac-nav__label-l"><Icon name="doc" size={14}/> meta (faded)</span>
                  <span className="ac-nav__right"><span className="ac-nav__count">5</span></span>
                </div>
              </div>
            </div>
          </DSSection>

          {/* ──────────────────── CARDS ───────────────────── */}
          <DSSection id="cards" title="Cards">
            <h3 className="ds-h3">Lifecycle card</h3>
            <div className="ac-lcard" style={{ cursor: "default" }}>
              <div>
                <div className="ac-lcard__title">Three-layer review system architecture</div>
                <div className="ac-lcard__slug">three-layer-review-system-architecture</div>
              </div>
              <div className="ac-lcard__meta">
                <StatusBadge status="in-progress"/>
                <span>2m ago</span>
              </div>
              <div className="ac-lcard__pipe">
                <PipelineMini stages={window.STAGES} present={["work","research","plans","plan-reviews","validations","decisions"]}/>
              </div>
            </div>

            <h3 className="ds-h3">Kanban card</h3>
            <div style={{ maxWidth: 320 }}>
              <div className="ac-kcard" style={{ cursor: "default" }}>
                <div className="ac-kcard__top">
                  <span className="ac-kcard__id">PROJ-0001</span>
                  <Chip tone="indigo">in progress</Chip>
                </div>
                <div className="ac-kcard__title">Add three-layer review pipeline behind a flag</div>
                <div className="ac-kcard__slug">three-layer-review-system-architecture</div>
                <div className="ac-kcard__foot">
                  <span className="ac-kcard__links"><Icon name="link" size={11}/> 6 linked</span>
                  <span className="ac-kcard__mtime">2m ago</span>
                </div>
              </div>
            </div>

            <h3 className="ds-h3">Related item row</h3>
            <div className="ac-related" style={{ maxWidth: 480 }}>
              <div className="ac-related__item">
                <span className="ac-related__type">PLAN</span>
                <span className="ac-related__title">Three-layer review system architecture</span>
                <span className="ac-related__meta">2026-02-22</span>
              </div>
              <div className="ac-related__item">
                <span className="ac-related__type">PLAN-REVIEW</span>
                <span className="ac-related__title">Plan review · round 1</span>
                <span className="ac-related__meta">2026-03-01</span>
              </div>
              <div className="ac-related__item">
                <span className="ac-related__type">ADR</span>
                <span className="ac-related__title">Three-layer review system architecture</span>
                <span className="ac-related__meta">2026-03-14</span>
              </div>
            </div>

            <h3 className="ds-h3">Empty-state lifecycle card</h3>
            <div className="ac-lcard ac-lcard--empty" style={{cursor:"default"}}>
              <div>
                <div className="ac-lcard__title">No docs yet</div>
                <div className="ac-lcard__empty-sub">Drop the first one in <code>meta/work/</code></div>
              </div>
              <div className="ac-lcard__empty-tag">empty</div>
            </div>
          </DSSection>

          {/* ──────────────────── TABLE ───────────────────── */}
          <DSSection id="table" title="Library table">
            <table className="ac-libtable">
              <thead>
                <tr><th>ID</th><th>Title</th><th>Slug</th><th>Updated</th></tr>
              </thead>
              <tbody>
                <tr>
                  <td className="ac-libtable__id">PROJ-0001</td>
                  <td className="ac-libtable__title">Add three-layer review pipeline</td>
                  <td className="ac-libtable__slug">three-layer-review-system-architecture</td>
                  <td className="ac-libtable__date">2m ago</td>
                </tr>
                <tr className="is-selected">
                  <td className="ac-libtable__id">META-0011</td>
                  <td className="ac-libtable__title">Browser-based visualiser for meta/</td>
                  <td className="ac-libtable__slug">meta-visualisation</td>
                  <td className="ac-libtable__date">5m ago</td>
                </tr>
                <tr>
                  <td className="ac-libtable__id">PROJ-0007</td>
                  <td className="ac-libtable__title">Ship PR review agents behind a flag</td>
                  <td className="ac-libtable__slug">pr-review-agents</td>
                  <td className="ac-libtable__date">1h ago</td>
                </tr>
              </tbody>
            </table>
          </DSSection>

          {/* ──────────────────── MARKDOWN ───────────────────── */}
          <DSSection id="markdown" title="Markdown rendering">
            <div className="ac-md" style={{ maxWidth: "none" }}>
              {renderMarkdown(`## Heading 2

### Heading 3

Paragraph copy with **bold**, *italic*, \`inline code\`, and a [[wiki-link]] reference. The renderer also supports lists, tables, and fenced code blocks.

- Unordered list item one
- Unordered list item two with a longer wrapping line that demonstrates how body text behaves at this size and weight
- Unordered list item three

1. Ordered first
2. Ordered second
3. Ordered third

| Lens | Verdict | Notes |
|------|---------|-------|
| Convention | pass | Schema fields satisfied |
| Agent | approve-with-changes | Two clarifications |
| Orchestrator | request-changes | Missing validation |`)}
            </div>
          </DSSection>

          {/* ──────────────────── CODE ───────────────────── */}
          <DSSection id="code" title="Code blocks" hint="syntax-highlighted, chrome header">
            <div className="ac-md" style={{ maxWidth: "none" }}>
              {renderMarkdown("```typescript\n// orchestrator agent\nexport async function review(plan: Plan): Promise<Verdict> {\n  const lenses = ['convention', 'agent', 'orchestrator'];\n  const results = await Promise.all(lenses.map(l => runLens(l, plan)));\n  return aggregate(results);\n}\n```")}
              {renderMarkdown("```bash\n$ ./accelerator review plan-2026-02-22\n→ running 3 lenses…\n✓ convention   pass\n⚠ agent        approve-with-changes\n✗ orchestrator request-changes\n```")}
            </div>
          </DSSection>

          {/* ──────────────────── FRONTMATTER ───────────────────── */}
          <DSSection id="frontmatter" title="Frontmatter key/value">
            <div className="ac-fm">
              <div className="ac-fm__k">slug</div><div className="ac-fm__v">three-layer-review-system-architecture</div>
              <div className="ac-fm__k">status</div><div className="ac-fm__v">in-progress</div>
              <div className="ac-fm__k">owner</div><div className="ac-fm__v"><a href="#" onClick={e=>e.preventDefault()}>Toby Clemson</a></div>
              <div className="ac-fm__k">updated</div><div className="ac-fm__v">2026-03-14T15:32:00Z</div>
              <div className="ac-fm__k">links</div><div className="ac-fm__v"><a href="#" onClick={e=>e.preventDefault()}>PROJ-0001</a>, <a href="#" onClick={e=>e.preventDefault()}>PLAN-2026-02-22</a></div>
            </div>
          </DSSection>

          {/* ──────────────────── EMPTY / BANNERS ───────────────────── */}
          <DSSection id="empty" title="Empty states &amp; banners">
            <h3 className="ds-h3">Inline empty</h3>
            <div className="ac-empty" style={{maxWidth: 480}}>
              <div className="ac-empty__title">Nothing to show</div>
              <div className="ac-empty__body">No documents of this type exist yet. Drop the first one into <code>meta/notes/</code> to get started.</div>
            </div>

            <h3 className="ds-h3">Warn banner</h3>
            <div className="ac-banner" style={{maxWidth: 640}}>
              <Icon name="alert" size={14} style={{color:"var(--ac-warn)", marginTop: 2}}/>
              <div>
                <b>Validation pipeline lagging.</b> The orchestrator lens hasn't completed in 38s — usually under 5s. Check the watcher.
              </div>
            </div>
          </DSSection>

          {/* ──────────────────── TOASTS ───────────────────── */}
          <DSSection id="toast" title="Toasts">
            <div className="ds-toast-stack">
              {demoToasts.map(t => (
                <div key={t.id} className={`ac-toast ${t.kind === "err" ? "ac-toast--err" : t.kind === "warn" ? "ac-toast--warn" : ""}`}>
                  <div className="ac-toast__icon">
                    <Icon name={t.kind === "err" ? "alert" : t.kind === "warn" ? "alert" : "check"} size={16}/>
                  </div>
                  <div>
                    <div className="ac-toast__title">{t.title}</div>
                    <div className="ac-toast__body">{t.body}</div>
                  </div>
                  <button className="ac-toast__close" onClick={() => setDemoToasts(xs => xs.filter(x => x.id !== t.id))}>
                    <Icon name="close" size={14}/>
                  </button>
                </div>
              ))}
              {demoToasts.length === 0 && (
                <button className="ds-link" onClick={() => setDemoToasts([
                  { id: 1, kind: "ok",   title: "Snapshot saved", body: "Cluster pinned at commit 9af2c1." },
                  { id: 2, kind: "warn", title: "External edit detected", body: <>A reviewer agent updated <code>WORK-0007</code>.</> },
                  { id: 3, kind: "err",  title: "Indexer crashed", body: "Validation pipeline returned exit code 42." },
                ])}>reset toasts ↻</button>
              )}
            </div>
          </DSSection>

          {/* ──────────────────── TOPBAR ───────────────────── */}
          <DSSection id="topbar" title="Topbar chrome">
            <div className="ds-topbar-demo">
              <header className="ac-topbar" style={{height: 48, position: "static", borderRadius: 6}}>
                <div className="ac-topbar__brand">
                  <AtomicMark size={24}/>
                  <div className="ac-topbar__brand-text">
                    <span className="ac-topbar__brand-name">Accelerator</span>
                    <span className="ac-topbar__brand-sub">VISUALISER</span>
                  </div>
                </div>
                <div className="ac-topbar__sep"/>
                <div className="ac-topbar__crumbs">
                  <span className="muted">Library</span>
                  <Icon name="chevron-right" size={12}/>
                  <strong>Plans</strong>
                </div>
                <div className="ac-topbar__spacer"/>
                <div className="ac-topbar__status"><span className="dot"/> 127.0.0.1:52914</div>
                <div className="ac-topbar__status"><Icon name="activity" size={12} style={{color:"var(--ac-ok)"}}/> SSE</div>
                <button className="ac-topbar__btn"><Icon name="moon" size={14}/></button>
              </header>
            </div>
          </DSSection>

          {/* Footer */}
          <footer className="ds-footer">
            <div className="mono faint">— end of design system —</div>
            <div className="ds-footer__hint mono faint">
              press <kbd>⌘⇧D</kbd> to leave · share <span className="ds-footer__route">/#dev</span> with the team
            </div>
          </footer>
        </div>
      </div>
    </div>
  );
}

Object.assign(window, { DevDesignSystem, DEV_SECTIONS });
