// Templates — three-tier virtual doc type.

function TierPill({ tier, present, active }) {
  const label = tier === "config-override" ? "config" : tier === "user-override" ? "user" : "default";
  const cls = !present ? "is-absent" : active ? "is-active" : "is-present";
  return <span className={`ac-tier-pill ${cls}`}><span className="dot"/> {label}</span>;
}

function TemplatesIndex({ setRoute }) {
  const [selected, setSelected] = React.useState("adr");
  const templates = window.LIBRARY_INDEX.templates;
  const detail = window.DOC_CONTENT["adr-template"];
  const [activeTier, setActiveTier] = React.useState("user-override");

  const tierName = (s) => s === "config-override" ? "Config override" : s === "user-override" ? "User override" : "Plugin default";
  const tierNum  = (s) => s === "config-override" ? "Tier 1" : s === "user-override" ? "Tier 2" : "Tier 3";
  const tierDesc = (s) => s === "config-override" ? "highest priority · .claude/accelerator.md"
                        : s === "user-override"   ? "meta/templates/ in this repo"
                                                  : "plugin-default · always present";

  const selectedTier = detail.tiers.find(t => t.source === activeTier);

  return (
    <div className="ac-page">
      <div className="ac-pagehead">
        <div className="ac-pagehead__l">
          <div className="ac-pagehead__eyebrow">
            <span className="ac-eyebrow-glyph"><Icon name="layers" size={12}/></span> Templates
          </div>
          <h1>Templates</h1>
          <div className="ac-pagehead__sub">The starting shape for every new doc. Pick a template to see which version is active and what the other tiers look like.</div>
        </div>
      </div>

      <div className="ac-tpl-list">
        {templates.map(t => (
          <div key={t.name}
               className={`ac-tpl-row ${selected === t.name ? "is-active" : ""}`}
               onClick={() => setSelected(t.name)}>
            <div className="ac-tpl-row__name">
              <TypeGlyph type={t.name === "adr" ? "decisions" : t.name === "plan" ? "plans" : t.name === "research" ? "research" : t.name === "validation" ? "validations" : "pr-descriptions"} size={22}/>
              <span>{t.name}.md</span>
            </div>
            <div className="ac-tpl-row__tiers">
              <TierPill tier="config-override" present={t.tiers[0] !== "—"} active={t.active === "config"}/>
              <Icon name="chevron-right" size={10} className="faint"/>
              <TierPill tier="user-override"   present={t.tiers[1] !== "—"} active={t.active === "user"}/>
              <Icon name="chevron-right" size={10} className="faint"/>
              <TierPill tier="plugin-default"  present={true}                active={t.active === "default"}/>
            </div>
            <Icon name="chevron-right" size={14} className="faint"/>
          </div>
        ))}
      </div>

      <div className="ac-tpl-detail">
        <div>
          <div style={{fontFamily:"var(--ac-font-mono)",fontSize:11,color:"var(--ac-fg-faint)",letterSpacing:"0.12em",textTransform:"uppercase",marginBottom:10}}>Tiers · {selected}.md</div>
          <div className="ac-tpl-tiers">
            {detail.tiers.map(tier => (
              <div key={tier.source}
                   className={`ac-tpl-tier ${activeTier === tier.source ? "is-selected" : ""} ${!tier.present ? "is-inactive" : ""}`}
                   onClick={() => tier.present && setActiveTier(tier.source)}>
                <div className="ac-tpl-tier__num">{tierNum(tier.source)}</div>
                <div className="ac-tpl-tier__name">
                  {tierName(tier.source)}
                  {tier.active && <Chip tone="indigo">active</Chip>}
                  {!tier.present && <Chip tone="neutral">absent</Chip>}
                </div>
                <div className="ac-tpl-tier__path">{tier.path}</div>
                <div className="mono faint" style={{fontSize:10,marginTop:6}}>{tierDesc(tier.source)}</div>
              </div>
            ))}
          </div>
        </div>

        <div className="ac-tpl-preview">
          <div className="ac-tpl-preview__head">
            <span>{selectedTier && selectedTier.path}</span>
            <span>{selectedTier && selectedTier.etag}</span>
          </div>
          <div className="ac-tpl-preview__body">
            {selectedTier && selectedTier.present ? syntaxHighlight(selectedTier.content) :
              <span className="faint">tier not present</span>}
          </div>
        </div>
      </div>
    </div>
  );
}

function syntaxHighlight(src) {
  // Frontmatter + markdown body. Each line is rendered as its own <div>
  // (so empty lines preserve height via &nbsp;) and the active section —
  // YAML between the two `---` fences vs markdown elsewhere — chooses the
  // tokenizer for that line.
  const out = [];
  const lines = src.split("\n");
  let inFm = false;
  let fmSeen = 0;
  lines.forEach((l, i) => {
    if (l === "---" && fmSeen < 2) {
      inFm = !inFm;
      fmSeen++;
      out.push(<div key={i} className="tpl-line"><span className="fm-delim">---</span></div>);
      return;
    }
    if (l === "") {
      out.push(<div key={i} className="tpl-line">{"\u00a0"}</div>);
      return;
    }
    if (inFm) {
      const m = l.match(/^([a-zA-Z_][\w-]*):(\s*)(.*)$/);
      if (m) {
        out.push(
          <div key={i} className="tpl-line">
            <span className="fm-key">{m[1]}</span>
            <span className="fm-delim">:</span>
            {m[2]}
            {renderFmValue(m[3])}
          </div>
        );
        return;
      }
      out.push(<div key={i} className="tpl-line">{l}</div>);
      return;
    }
    out.push(<div key={i} className="tpl-line">{renderMdLine(l)}</div>);
  });
  return out;
}

function renderFmValue(v) {
  if (v === "null" || v === "true" || v === "false") return <span className="md-lit">{v}</span>;
  if (/^-?\d+(\.\d+)?$/.test(v)) return <span className="md-num">{v}</span>;
  if (/^"(?:[^"\\]|\\.)*"$/.test(v) || /^'(?:[^'\\]|\\.)*'$/.test(v)) {
    return <span className="md-str">{v}</span>;
  }
  // Bare scalar (ISO date, identifier, etc.) — leave plain.
  return v;
}

function renderMdLine(l) {
  const h = l.match(/^(#{1,6})\s+(.*)$/);
  if (h) {
    return <><span className="md-hash">{h[1]}</span> <span className="md-head">{renderInline(h[2])}</span></>;
  }
  const b = l.match(/^(\s*)([-*+])\s+(.*)$/);
  if (b) {
    return <>{b[1]}<span className="md-bullet">{b[2]}</span> {renderInline(b[3])}</>;
  }
  return renderInline(l);
}

function renderInline(text) {
  // Order matters: try **bold** before *em*; `code` and [[wiki]] are independent.
  const parts = [];
  const re = /(\*\*[^*\n]+\*\*|\*[^*\n]+\*|`[^`\n]+`|\[\[[^\]\n]+\]\])/g;
  let last = 0;
  let m;
  let i = 0;
  while ((m = re.exec(text))) {
    if (m.index > last) parts.push(<React.Fragment key={`t${i}`}>{text.slice(last, m.index)}</React.Fragment>);
    const t = m[0];
    let cls = "md-em";
    if (t.startsWith("**")) cls = "md-strong";
    else if (t.startsWith("`")) cls = "md-code";
    else if (t.startsWith("[[")) cls = "md-link";
    parts.push(<span key={`s${i}`} className={cls}>{t}</span>);
    i++;
    last = re.lastIndex;
  }
  if (last < text.length) parts.push(<React.Fragment key={`t${i}`}>{text.slice(last)}</React.Fragment>);
  return parts;
}

Object.assign(window, { TemplatesIndex });
