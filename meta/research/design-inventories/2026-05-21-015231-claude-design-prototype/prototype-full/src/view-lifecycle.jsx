// Lifecycle views — index of clusters (with a pipeline chain of type-icon
// tiles) and a per-cluster timeline detail view. Tiles share the icon system
// used in the library so the visual language is consistent across views.

function StageTile({ on, stage, size = 28 }) {
  const color = `hsl(${stage.hue} 68% 46%)`;
  const icon = window.TYPE_ICONS[stage.key];
  const pad = Math.round(size * 0.16);
  return (
    <div className={`ac-chain__tile ${on ? "on" : ""}`}
         style={{
           width: size, height: size,
           color: on ? "#FFFFFF" : color,
           background: on ? color : `hsl(${stage.hue} 78% 95%)`,
           borderColor: on ? color : `hsl(${stage.hue} 40% 82%)`,
           padding: pad,
         }}
         title={stage.label}>
      {icon ? (
        <svg viewBox="0 0 24 24" width="100%" height="100%" aria-hidden>{icon}</svg>
      ) : (
        <span style={{fontFamily:"var(--ac-font-mono)",fontSize:9,fontWeight:600}}>{stage.short}</span>
      )}
    </div>
  );
}

function Pipeline({ present, stages = window.STAGES, size = 28 }) {
  return (
    <div className="ac-hexchain">
      {stages.map((s, i) => {
        const on = present.includes(s.key);
        const nextOn = i < stages.length - 1 && present.includes(stages[i+1].key);
        const color = `hsl(${s.hue} 68% 46%)`;
        return (
          <div key={s.key} className={`ac-hexchain__stage ${on ? "on" : ""}`}
               style={{color: color, "--chain-tile-h": `${Math.round(size/2)}px`}}>
            <StageTile on={on} stage={s} size={size}/>
            <div className="ac-hexchain__label">{s.label}</div>
            {i < stages.length - 1 && (
              <div className="ac-hexchain__link"
                   style={{background: (on && nextOn) ? color : "var(--ac-stroke)"}}/>
            )}
          </div>
        );
      })}
    </div>
  );
}

function LifecycleIndex({ setRoute }) {
  const [sort, setSort] = React.useState("updated");
  const clusters = [...window.CLUSTERS];
  if (sort === "completeness") clusters.sort((a,b) => b.present.length - a.present.length);

  return (
    <div className="ac-page">
      <div className="ac-pagehead">
        <div className="ac-pagehead__l">
          <div className="ac-pagehead__eyebrow"><span className="ac-eyebrow-glyph"><Icon name="lifecycle" size={12}/></span> Lifecycle</div>
          <h1>Lifecycle overview</h1>
          <div className="ac-pagehead__sub">Every work unit and how far it has progressed. Each row groups one unit's artifacts; the pipeline shows which stages it has reached.</div>
        </div>
        <div className="ac-pagehead__actions">
          <div className="ac-tweaks__seg">
            <button className={sort === "updated" ? "on" : ""} onClick={() => setSort("updated")}>Updated</button>
            <button className={sort === "completeness" ? "on" : ""} onClick={() => setSort("completeness")}>Completeness</button>
          </div>
        </div>
      </div>

      <div className="ac-lcycle">
        {clusters.map(c => (
          <div key={c.slug} className="ac-lcard"
               onClick={() => setRoute({view:"lifecycle", slug: c.slug, type:null, docId:null})}>
            <div>
              <div className="ac-lcard__title">{c.title}</div>
              <div className="ac-lcard__slug">{c.slug}</div>
            </div>
            <div className="ac-lcard__meta">
              <StatusBadge status={c.status}/>
              <span><Icon name="clock" size={11}/> {c.updated}</span>
              <span>{c.entries.length} artifacts</span>
            </div>
            <div className="ac-lcard__pipe">
              <Pipeline present={c.present} size={26}/>
              <div className="mono faint" style={{fontSize:10.5,whiteSpace:"nowrap",marginLeft:8}}>
                {c.present.length}/{window.STAGES.length}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function LifecycleCluster({ slug, setRoute }) {
  const cluster = window.CLUSTERS.find(c => c.slug === slug) || window.CLUSTERS[2];
  // Build ordered timeline: for each stage in canonical order, emit its entries
  // or a placeholder card.
  const timeline = [];
  window.STAGES.forEach(stage => {
    const matches = cluster.entries.filter(e => e.type === stage.key);
    if (matches.length === 0) {
      timeline.push({ stage, missing: true });
    } else {
      matches.forEach((e, i) => timeline.push({ stage, entry: e }));
    }
  });
  // Notes aren't a stage; append them at the bottom if present.
  const notes = cluster.entries.filter(e => e.type === "notes");
  notes.forEach(e => timeline.push({ stage: { key: "notes", short: "NTE", label: "Note", hue: 50 }, entry: e }));

  return (
    <div className="ac-page">
      <div className="ac-pagehead">
        <div className="ac-pagehead__l">
          <div className="ac-pagehead__eyebrow">
            <span className="ac-eyebrow-glyph"><Icon name="lifecycle" size={12}/></span>
            <button onClick={() => setRoute({view:"lifecycle", slug:null, type:null, docId:null})}>Lifecycle</button>
          </div>
          <h1>{cluster.title}</h1>
          <div className="ac-pagehead__sub" style={{display:"flex",gap:10,alignItems:"center",marginTop:8}}>
            <StatusBadge status={cluster.status}/>
            <span className="mono" style={{fontSize:11}}>{cluster.owner}</span>
            <span className="mono faint" style={{fontSize:11}}>updated {cluster.updated}</span>
          </div>
        </div>
      </div>

      <div style={{padding:"16px 20px",background:"var(--ac-bg-sunken)",border:"1px solid var(--ac-stroke)",borderRadius:6,marginBottom:8}}>
        <div style={{fontFamily:"var(--ac-font-mono)",fontSize:10.5,color:"var(--ac-fg-faint)",letterSpacing:"0.1em",textTransform:"uppercase",marginBottom:14}}>Pipeline</div>
        <Pipeline present={cluster.present} size={34}/>
      </div>

      <div className="ac-timeline">
        {timeline.map((t, i) => {
          const color = `hsl(${t.stage.hue} 68% 46%)`;
          return (
            <div key={i} className={`ac-tstep ${t.missing ? "is-missing" : ""}`}>
              <div className="ac-tstep__node">
                <StageTile on={!t.missing} stage={t.stage} size={36}/>
              </div>
              {t.missing ? (
                <div className="ac-tcard ac-tcard--missing">
                  No {t.stage.label.toLowerCase()} yet
                </div>
              ) : (
                <div className="ac-tcard"
                     onClick={() => setRoute({view:"library", type: t.stage.key, docId: t.entry.id, slug: cluster.slug})}>
                  <div className="ac-tcard__head">
                    <div style={{display:"flex",gap:10,alignItems:"center",minWidth:0}}>
                      <span className="mono faint" style={{fontSize:10.5,letterSpacing:"0.04em"}}>{t.stage.label.toUpperCase()}</span>
                      <span className="ac-tcard__title" style={{whiteSpace:"nowrap",overflow:"hidden",textOverflow:"ellipsis"}}>{t.entry.title}</span>
                    </div>
                    <div style={{display:"flex",gap:8,alignItems:"center"}}>
                      {t.entry.status && <StatusBadge status={t.entry.status}/>}
                      {t.entry.verdict && <StatusBadge status={t.entry.verdict}/>}
                      <span className="ac-tcard__meta">{t.entry.date}</span>
                    </div>
                  </div>
                  <div className="ac-tcard__body">
                    <span className="mono faint">{t.entry.id}</span> · modified {t.entry.mtime}
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

Object.assign(window, { LifecycleIndex, LifecycleCluster, Pipeline, StageTile });
