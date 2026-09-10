// Kanban view — three columns with drag-and-drop simulation.

function KanbanCard({ item, onDragStart, dragging, setRoute }) {
  const cluster = item.cluster != null ? window.CLUSTERS[item.cluster] : null;
  return (
    <div className={`ac-kcard ${dragging ? "is-dragging" : ""}`}
         draggable
         onDragStart={e => onDragStart(item)}
         onClick={() => setRoute({view:"library", type:"work", docId: item.id, slug: item.slug})}>
      <div className="ac-kcard__top">
        <span className="ac-kcard__meta">
          {item.kind && <WorkKindBadge kind={item.kind}/>}
          <span className="ac-kcard__id">{item.id}</span>
        </span>
        {cluster && <PipelineMini present={cluster.present} stages={window.STAGES} compact/>}
      </div>
      <div className="ac-kcard__title">{item.title}</div>
      <div className="ac-kcard__slug">{item.slug}</div>
      <div className="ac-kcard__foot">
        <span className="ac-kcard__links"><Icon name="link" size={11}/> {item.linked || 1} linked</span>
        <span className="ac-kcard__mtime">{item.mtime}</span>
      </div>
    </div>
  );
}

function KanbanBoard({ setRoute, pushToast }) {
  const [items, setItems] = React.useState(window.WORK_ITEMS.map(t => ({...t})));
  const [dragging, setDragging] = React.useState(null);

  const cols = [
    { key: "todo",        title: "Todo",        dot: "var(--ac-fg-faint)" },
    { key: "in-progress", title: "In progress", dot: "var(--ac-accent)" },
    { key: "done",        title: "Done",        dot: "var(--ac-ok)" },
  ];

  const onDrop = (colKey) => {
    if (!dragging) return;
    const from = dragging.status;
    if (from === colKey) { setDragging(null); return; }
    setItems(ts => ts.map(t => t.id === dragging.id ? {...t, status: colKey} : t));
    pushToast({
      kind: "ok",
      title: `${dragging.id} moved to ${colKey}`,
      body: <>PATCH <code>/api/docs/work/{dragging.id}.md/frontmatter</code> → <code>204</code> · fresh ETag received</>,
    });
    setDragging(null);
  };

  return (
    <div className="ac-page">
      <div className="ac-pagehead">
        <div className="ac-pagehead__l">
          <div className="ac-pagehead__eyebrow"><span className="ac-eyebrow-glyph"><Icon name="kanban" size={12}/></span> Kanban</div>
          <h1>Work items</h1>
          <div className="ac-pagehead__sub">Every work item, grouped by status. Drag a card to move it between columns — the change writes back to the file on disk.</div>
        </div>
        <div className="ac-pagehead__actions">
          <Chip tone="indigo"><Icon name="activity" size={10}/> live</Chip>
          <span className="mono faint" style={{fontSize:11}}>{items.length} total</span>
        </div>
      </div>

      <div className="ac-kanban">
        {cols.map(col => {
          const colItems = items.filter(t => t.status === col.key);
          return (
            <div key={col.key}
                 className="ac-kcol"
                 onDragOver={e => { e.preventDefault(); }}
                 onDrop={e => onDrop(col.key)}>
              <div className="ac-kcol__head">
                <div className="ac-kcol__title">
                  <span className="dot" style={{background: col.dot}}/>
                  {col.title}
                </div>
                <span className="ac-kcol__count">{colItems.length}</span>
              </div>
              {colItems.map(t => (
                <KanbanCard key={t.id} item={t} setRoute={setRoute}
                            onDragStart={tk => setDragging(tk)}
                            dragging={dragging && dragging.id === t.id}/>
              ))}
              {colItems.length === 0 && (
                <div className="ac-empty" style={{padding:20,fontSize:12}}>
                  <div className="ac-empty__title">Nothing here</div>
                  <div className="ac-empty__body">Drop a work item to set its status to {col.key}.</div>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

Object.assign(window, { KanbanBoard });
