// Shared UI primitives for the Accelerator visualiser.
// Icons, chips, stage dots, markdown render, hex mark, etc.

const { useState, useEffect, useRef, useMemo } = React;

// ───────────────────────────── Iconography ────────────────────────────────
// Feather-style stroke icons at 16 / 18 / 20. 2px stroke, rounded caps.
function Icon({ name, size = 18, className = "", style }) {
  const s = size;
  const common = {
    width: s, height: s, viewBox: "0 0 24 24",
    fill: "none", stroke: "currentColor",
    strokeWidth: 2, strokeLinecap: "round", strokeLinejoin: "round",
    className: "ac-icon " + className,
    style,
  };
  const paths = {
    search:   <><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></>,
    library:  <><path d="M4 4h4v16H4z"/><path d="M10 4h4v16h-4z"/><path d="m17 5 3 1-4 14-3-1z"/></>,
    kanban:   <><rect x="3"  y="4" width="5" height="16" rx="1"/><rect x="10" y="4" width="5" height="10" rx="1"/><rect x="17" y="4" width="4" height="13" rx="1"/></>,
    lifecycle:<><circle cx="6" cy="6" r="2"/><circle cx="18" cy="6" r="2"/><circle cx="6" cy="18" r="2"/><circle cx="18" cy="18" r="2"/><path d="M8 6h8M6 8v8M18 8v8M8 18h8"/></>,
    activity: <><path d="M3 12h4l3-8 4 16 3-8h4"/></>,
    clock:    <><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/></>,
    link:     <><path d="M10 14a4 4 0 0 0 6 0l3-3a4 4 0 0 0-6-6l-1 1"/><path d="M14 10a4 4 0 0 0-6 0l-3 3a4 4 0 0 0 6 6l1-1"/></>,
    "chevron-right": <><path d="m9 6 6 6-6 6"/></>,
    "chevron-down":  <><path d="m6 9 6 6 6-6"/></>,
    "chevron-left":  <><path d="m15 6-6 6 6 6"/></>,
    doc:      <><path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><path d="M14 3v6h6"/></>,
    edit:     <><path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4z"/></>,
    close:    <><path d="M18 6 6 18M6 6l12 12"/></>,
    check:    <><path d="m5 12 5 5L20 7"/></>,
    dot:      <><circle cx="12" cy="12" r="3"/></>,
    plus:     <><path d="M12 5v14M5 12h14"/></>,
    minus:    <><path d="M5 12h14"/></>,
    "git-pr": <><circle cx="6" cy="6" r="2.5"/><circle cx="6" cy="18" r="2.5"/><circle cx="18" cy="18" r="2.5"/><path d="M6 8v8"/><path d="M15 18H9"/><path d="M18 16v-4a5 5 0 0 0-5-5h-2"/><path d="m13 4-2 3 2 3"/></>,
    "git-branch": <><circle cx="6" cy="5" r="2"/><circle cx="6" cy="19" r="2"/><circle cx="18" cy="12" r="2"/><path d="M6 7v10"/><path d="M18 10a6 6 0 0 0-6-6"/></>,
    filter:   <><path d="M4 4h16l-6 8v6l-4 2v-8z"/></>,
    sort:     <><path d="M7 4v16"/><path d="m3 8 4-4 4 4"/><path d="M17 20V4"/><path d="m13 16 4 4 4-4"/></>,
    sparkle:  <><path d="m12 3 1.8 4.8L18 9.6l-4.2 1.8L12 16.2 10.2 11.4 6 9.6l4.2-1.8z"/></>,
    hex:      <><path d="m12 3 8 5v8l-8 5-8-5V8z"/></>,
    shield:   <><path d="M12 3 4 6v6c0 4.5 3.5 8.5 8 9 4.5-.5 8-4.5 8-9V6z"/></>,
    moon:     <><path d="M20 14a8 8 0 0 1-10-10 8 8 0 1 0 10 10"/></>,
    sun:      <><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/></>,
    settings: <><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/></>,
    terminal: <><path d="m4 17 6-6-6-6"/><path d="M12 19h8"/></>,
    "arrow-right": <><path d="M5 12h14M13 5l7 7-7 7"/></>,
    flag:     <><path d="M4 21V4h11l-2 4 2 4H4"/></>,
    folder:   <><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/></>,
    layers:   <><path d="m12 3 9 5-9 5-9-5z"/><path d="m3 13 9 5 9-5"/><path d="m3 18 9 5 9-5"/></>,
    alert:    <><path d="M12 3 2 21h20z"/><path d="M12 10v5"/><circle cx="12" cy="18" r=".5"/></>,
  };
  return <svg {...common}>{paths[name] || null}</svg>;
}

// ───────────────────────────── Atomic hex mark ────────────────────────────
function AtomicMark({ size = 28 }) {
  return (
    <svg width={size} height={size} viewBox="0 0 40 40" className="ac-mark">
      <defs>
        <linearGradient id="hexg" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%"  stopColor="var(--accent)"   stopOpacity="1"/>
          <stop offset="100%" stopColor="var(--accent-2)" stopOpacity="1"/>
        </linearGradient>
      </defs>
      <path d="M20 2 36 11v18L20 38 4 29V11z" fill="none" stroke="url(#hexg)" strokeWidth="2"/>
      <circle cx="20" cy="20" r="3" fill="var(--accent)"/>
      <circle cx="20" cy="20" r="7.5" fill="none" stroke="var(--accent-2)" strokeWidth="1" strokeOpacity="0.5"/>
    </svg>
  );
}

// ───────────────────────────── Chips & badges ─────────────────────────────
function Chip({ children, tone = "neutral", size = "sm" }) {
  return <span className={`ac-chip ac-chip--${tone} ac-chip--${size}`}>{children}</span>;
}

function StatusBadge({ status }) {
  const map = {
    "todo":        { tone: "neutral",  label: "Todo" },
    "in-progress": { tone: "indigo",   label: "In progress" },
    "done":        { tone: "green",    label: "Done" },
    "draft":       { tone: "neutral",  label: "Draft" },
    "accepted":    { tone: "green",    label: "Accepted" },
    "proposed":    { tone: "indigo",   label: "Proposed" },
    "open":        { tone: "indigo",   label: "Open" },
    "merged":      { tone: "violet",   label: "Merged" },
    "approve":               { tone: "green",  label: "Approve" },
    "approve-with-changes":  { tone: "amber",  label: "Approve w/ changes" },
    "request-changes":       { tone: "red",    label: "Request changes" },
    "pass":                  { tone: "green",  label: "Pass" },
    "sev-1":      { tone: "red",     label: "SEV-1" },
    "sev-2":      { tone: "amber",   label: "SEV-2" },
    "sev-3":      { tone: "neutral", label: "SEV-3" },
    "resolved":   { tone: "green",   label: "Resolved" },
    "monitoring": { tone: "indigo",  label: "Monitoring" },
  };
  const cfg = map[status] || { tone: "neutral", label: status };
  return <Chip tone={cfg.tone}>{cfg.label}</Chip>;
}

// Work-item kind badge — story / epic / spike / task / bug.
const WORK_KIND_META = {
  epic:  { tone: "violet",  label: "Epic" },
  story: { tone: "indigo",  label: "Story" },
  spike: { tone: "amber",   label: "Spike" },
  task:  { tone: "neutral", label: "Task" },
  bug:   { tone: "red",     label: "Bug" },
};
function WorkKindBadge({ kind }) {
  const cfg = WORK_KIND_META[kind] || { tone: "neutral", label: kind };
  return <span className={`ac-kindbadge ac-kindbadge--${cfg.tone}`}>{cfg.label}</span>;
}
// Horizontal pipeline of filled/empty dots for a cluster row.
function PipelineMini({ present, stages, compact = false }) {
  const size = compact ? 6 : 8;
  return (
    <div className="ac-stagedots" style={{ gap: compact ? 4 : 6 }}>
      {stages.map(s => {
        const on = present.includes(s.key);
        return (
          <span key={s.key}
                className={`ac-stagedot ${on ? "on" : ""}`}
                title={`${s.label}: ${on ? "present" : "missing"}`}
                style={{
                  width: size, height: size,
                  background: on ? `hsl(${s.hue} 72% 56%)` : "transparent",
                  borderColor: on ? `hsl(${s.hue} 72% 56%)` : "var(--ac-stroke)",
                }}/>
        );
      })}
    </div>
  );
}

// ───────────────────────────── Markdown render ────────────────────────────
// Minimal markdown -> React. Handles h1/h2/h3, paragraphs, fenced code,
// ordered/unordered lists, inline code, bold, italic, [[wiki-links]].
function renderMarkdown(src) {
  const lines = src.split("\n");
  const out = [];
  let i = 0;
  const inline = (t) => {
    // [[wiki]] first so we can wrap in span
    const parts = [];
    let rest = t;
    let key = 0;
    while (rest.length) {
      const m = rest.match(/\[\[([^\]]+)\]\]/);
      if (!m) { parts.push(renderInlineBasic(rest, key++)); break; }
      parts.push(renderInlineBasic(rest.slice(0, m.index), key++));
      parts.push(<a key={"w"+key++} className="ac-md-wikilink" href="#">{m[1]}</a>);
      rest = rest.slice(m.index + m[0].length);
    }
    return parts;
  };
  while (i < lines.length) {
    const l = lines[i];
    if (l.startsWith("```")) {
      const lang = l.slice(3).trim();
      const buf = [];
      i++;
      while (i < lines.length && !lines[i].startsWith("```")) { buf.push(lines[i]); i++; }
      i++;
      const code = buf.join("\n");
      const known = !!window.tokenize && lang && (window.langLabel ? true : false);
      out.push(
        <div key={"c"+i} className="ac-codeblock" data-lang={lang || "text"}>
          <div className="ac-codeblock__head">
            <span className="ac-codeblock__lang">{window.langLabel ? window.langLabel(lang) : (lang || "plain")}</span>
            <div className="ac-codeblock__dots" aria-hidden>
              <span/><span/><span/>
            </div>
          </div>
          <pre className="ac-md-pre">
            {window.HighlightedCode
              ? <window.HighlightedCode code={code} lang={lang}/>
              : <code>{code}</code>}
          </pre>
        </div>
      );
      continue;
    }
    if (l.startsWith("# "))  { out.push(<h1 key={"h"+i} className="ac-md-h1">{inline(l.slice(2))}</h1>); i++; continue; }
    if (l.startsWith("## ")) { out.push(<h2 key={"h"+i} className="ac-md-h2">{inline(l.slice(3))}</h2>); i++; continue; }
    if (l.startsWith("### ")){ out.push(<h3 key={"h"+i} className="ac-md-h3">{inline(l.slice(4))}</h3>); i++; continue; }
    // table — header row, separator, body rows (all start with `|`)
    if (l.trim().startsWith("|") && i + 1 < lines.length && /^\s*\|?[\s:|-]+\|[\s:|-]+\|?\s*$/.test(lines[i+1])) {
      const splitRow = (row) => row.trim().replace(/^\|/, "").replace(/\|$/, "").split("|").map(c => c.trim());
      const headers = splitRow(l);
      const aligns = splitRow(lines[i+1]).map(s => {
        const left = s.startsWith(":"), right = s.endsWith(":");
        return left && right ? "center" : right ? "right" : "left";
      });
      i += 2;
      const rows = [];
      while (i < lines.length && lines[i].trim().startsWith("|")) {
        rows.push(splitRow(lines[i])); i++;
      }
      out.push(
        <div key={"tw"+i} className="ac-md-tablewrap">
          <table className="ac-md-table">
            <thead><tr>{headers.map((h, k) => <th key={k} style={{ textAlign: aligns[k] || "left" }}>{inline(h)}</th>)}</tr></thead>
            <tbody>{rows.map((r, ri) => (
              <tr key={ri}>{r.map((c, ck) => <td key={ck} style={{ textAlign: aligns[ck] || "left" }}>{inline(c)}</td>)}</tr>
            ))}</tbody>
          </table>
        </div>
      );
      continue;
    }
    // ordered list
    if (/^\d+\.\s/.test(l)) {
      const items = [];
      while (i < lines.length && /^\d+\.\s/.test(lines[i])) {
        items.push(lines[i].replace(/^\d+\.\s/, "")); i++;
      }
      out.push(<ol key={"ol"+i} className="ac-md-ol">{items.map((it,k) => <li key={k}>{inline(it)}</li>)}</ol>);
      continue;
    }
    // unordered list — also handles GitHub-style task lists (`- [ ]` / `- [x]`)
    if (/^[-*]\s/.test(l)) {
      const items = [];
      while (i < lines.length && /^[-*]\s/.test(lines[i])) {
        items.push(lines[i].replace(/^[-*]\s/, "")); i++;
      }
      const isTaskList = items.every(it => /^\[[ xX]\]\s/.test(it));
      if (isTaskList) {
        out.push(
          <ul key={"tl"+i} className="ac-md-tasklist">
            {items.map((it,k) => {
              const checked = /^\[[xX]\]/.test(it);
              const text = it.replace(/^\[[ xX]\]\s/, "");
              return (
                <li key={k} className={`ac-md-task ${checked ? "is-done" : ""}`}>
                  <span className="ac-md-task__box" aria-hidden="true">
                    {checked && <Icon name="check" size={11}/>}
                  </span>
                  <span className="ac-md-task__label">{inline(text)}</span>
                </li>
              );
            })}
          </ul>
        );
        continue;
      }
      out.push(<ul key={"ul"+i} className="ac-md-ul">{items.map((it,k) => <li key={k}>{inline(it)}</li>)}</ul>);
      continue;
    }
    if (l.trim() === "") { i++; continue; }
    // paragraph
    const buf = [];
    while (i < lines.length && lines[i].trim() !== "" && !lines[i].startsWith("#") && !lines[i].startsWith("```") && !/^[-*]\s/.test(lines[i]) && !/^\d+\.\s/.test(lines[i])) {
      buf.push(lines[i]); i++;
    }
    out.push(<p key={"p"+i} className="ac-md-p">{inline(buf.join(" "))}</p>);
  }
  return out;
}

function renderInlineBasic(t, key) {
  // code first
  const segs = [];
  let rest = t;
  let k = 0;
  while (rest.length) {
    const m = rest.match(/`([^`]+)`/);
    if (!m) { segs.push(renderEmphasis(rest, key + "-" + k++)); break; }
    segs.push(renderEmphasis(rest.slice(0, m.index), key + "-" + k++));
    segs.push(<code key={key + "-c" + k++} className="ac-md-code">{m[1]}</code>);
    rest = rest.slice(m.index + m[0].length);
  }
  return <React.Fragment key={key}>{segs}</React.Fragment>;
}

function renderEmphasis(t, key) {
  // bold then italic (simple, non-nested)
  const parts = [];
  let rest = t;
  let k = 0;
  while (rest.length) {
    const b = rest.match(/\*\*([^*]+)\*\*/);
    const i = rest.match(/\*([^*]+)\*/);
    let pick = null;
    if (b && (!i || b.index <= i.index)) pick = { m: b, tag: "strong" };
    else if (i) pick = { m: i, tag: "em" };
    if (!pick) { parts.push(<React.Fragment key={key + "-t" + k++}>{rest}</React.Fragment>); break; }
    parts.push(<React.Fragment key={key + "-t" + k++}>{rest.slice(0, pick.m.index)}</React.Fragment>);
    const Tag = pick.tag;
    parts.push(<Tag key={key + "-em" + k++}>{pick.m[1]}</Tag>);
    rest = rest.slice(pick.m.index + pick.m[0].length);
  }
  return <React.Fragment key={key}>{parts}</React.Fragment>;
}

// ───────────────────────────── Type glyph ─────────────────────────────────
// Small colored hex with 3-letter code, used as a leading glyph on cards.
// Hand-drawn icons per doc type, rendered in a soft tinted rounded square.
// Each glyph is a 24×24 line drawing that reads at 20–40px sizes.

const TYPE_META = {
  work:          { hue: 12,  label: "Work item",   short: "WRK" },
  decisions:     { hue: 355, label: "Decision",    short: "ADR" },
  "root-cause-analyses": { hue: 310, label: "Root cause analysis", short: "RCA" },
  research:      { hue: 28,  label: "Research",    short: "RSC" },
  plans:         { hue: 220, label: "Plan",        short: "PLN" },
  "plan-reviews":{ hue: 260, label: "Plan review", short: "P/R" },
  validations:   { hue: 160, label: "Validation",  short: "VAL" },
  "pr-descriptions": { hue: 200, label: "PR description", short: "PR"  },
  "pr-reviews":  { hue: 280, label: "PR review",   short: "P/R" },
  "work-reviews":     { hue: 340, label: "Work item review", short: "W/R" },
  "design-inventories": { hue: 185, label: "Design inventory", short: "INV" },
  "design-gaps":      { hue: 95,  label: "Design gap",      short: "GAP" },
  notes:         { hue: 50,  label: "Note",        short: "NTE" },
  templates:     { hue: 215, label: "Template",    short: "TPL" },
};

const TYPE_ICONS = {
  work: (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <path d="M3.5 7.5h11l2 2v5l-2 2h-11z"/>
      <path d="M14.5 7.5v9" strokeDasharray="1.2 1.4"/>
      <path d="M6 11h5.5M6 13.5h4"/>
      <circle cx="18.5" cy="9"  r="0.55" fill="currentColor" stroke="none"/>
      <circle cx="18.5" cy="12" r="0.55" fill="currentColor" stroke="none"/>
      <circle cx="18.5" cy="15" r="0.55" fill="currentColor" stroke="none"/>
    </g>
  ),
  research: (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <path d="M4 6.5h7M4 9.5h8.5M4 12.5h5"/>
      <circle cx="15" cy="14" r="4"/>
      <path d="M18 17 20.5 19.5"/>
    </g>
  ),
  plans: (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3.5" y="4.5" width="17" height="15" rx="1.4"/>
      <path d="M3.5 9.5h17M3.5 14.5h17M8.5 4.5v15M13.5 4.5v15" strokeOpacity="0.3"/>
      <path d="M6 17 10.5 12 13 14 17.5 7" strokeWidth="1.5"/>
      <circle cx="6"    cy="17" r="1.1" fill="currentColor" stroke="none"/>
      <circle cx="17.5" cy="7"  r="1.1" fill="currentColor" stroke="none"/>
    </g>
  ),
  "plan-reviews": (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <path d="M5.5 3.5h8L18 8v9.5A1.5 1.5 0 0 1 16.5 19h-11A1.5 1.5 0 0 1 4 17.5V5A1.5 1.5 0 0 1 5.5 3.5z"/>
      <path d="M13.5 3.5V8H18"/>
      <path d="M7 11.5h4.5M7 14.5h3.5"/>
      <circle cx="17" cy="17" r="3.8" fill="var(--ac-bg-raised)" stroke="currentColor"/>
      <path d="m15.3 17.1 1.3 1.3 2.2-2.5" strokeWidth="1.4"/>
    </g>
  ),
  validations: (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 3 5 5.5v6c0 4 3 7.5 7 9 4-1.5 7-5 7-9v-6z"/>
      <path d="m8.5 12 2.4 2.4L15.5 9.5" strokeWidth="1.5"/>
    </g>
  ),
  "pr-descriptions": (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="6"  cy="5.5"  r="2"/>
      <circle cx="6"  cy="18.5" r="2"/>
      <circle cx="18" cy="12"   r="2"/>
      <path d="M6 7.5v9"/>
      <path d="M6 9.5c0 4 4.5 5 10 2.5"/>
      <path d="M14.2 10.8 16.3 12.3 14.4 14"/>
    </g>
  ),
  "work-reviews": (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <path d="M3.5 7.5h10l2 2v5l-2 2h-10z"/>
      <path d="M6 11h5M6 13.5h3.5"/>
      <circle cx="17.5" cy="16.5" r="3.8" fill="var(--ac-bg-raised)" stroke="currentColor"/>
      <path d="m15.8 16.6 1.3 1.3 2.2-2.5" strokeWidth="1.4"/>
    </g>
  ),
  "design-inventories": (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3.5" y="4.5" width="7" height="7" rx="1"/>
      <rect x="13.5" y="4.5" width="7" height="7" rx="1"/>
      <rect x="3.5" y="13.5" width="7" height="7" rx="1"/>
      <rect x="13.5" y="13.5" width="7" height="7" rx="1"/>
      <circle cx="7" cy="8" r="1.2"/>
      <path d="m4.5 11 2-2 1.5 1.5L9.5 9l1 1" strokeOpacity="0.5"/>
      <path d="M15 7h4M15 9h3" strokeOpacity="0.55"/>
      <path d="M15 16h4M15 18h2.5" strokeOpacity="0.55"/>
      <circle cx="17" cy="16.5" r="1" strokeOpacity="0.6"/>
    </g>
  ),
  "design-gaps": (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3" y="5" width="7" height="14" rx="1"/>
      <rect x="14" y="5" width="7" height="14" rx="1"/>
      <path d="M10.5 12h3" strokeDasharray="1.2 1.4"/>
      <path d="m12 9.5 1.5 2.5L12 14.5" strokeWidth="1.3"/>
      <path d="M5 9h3M5 11.5h2.5M5 14h3" strokeOpacity="0.55"/>
      <path d="M16 9h3M16 11.5h2.5M16 14h3" strokeOpacity="0.55"/>
    </g>
  ),
  "pr-reviews": (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <path d="M4.5 4.5h11A1.5 1.5 0 0 1 17 6v6A1.5 1.5 0 0 1 15.5 13.5h-4.5l-2 3-2-3H4.5A1.5 1.5 0 0 1 3 12V6A1.5 1.5 0 0 1 4.5 4.5z"/>
      <path d="M7.5 8h6M7.5 10.5h4"/>
      <circle cx="6" cy="8" r="0.55" fill="currentColor" stroke="none"/>
      <circle cx="6" cy="10.5" r="0.55" fill="currentColor" stroke="none"/>
    </g>
  ),
  decisions: (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 3.5v17"/>
      <path d="M12 8 6.5 6.5v4L12 12" strokeOpacity="0.5"/>
      <path d="M12 11 18 9v4.5L12 15" strokeWidth="1.5"/>
      <circle cx="12" cy="20.5" r="1" fill="currentColor" stroke="none"/>
    </g>
  ),
  "root-cause-analyses": (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <path d="M3.5 12h12"/>
      <path d="M6 6.5 8.5 12M11 6.5 13.5 12M6 17.5 8.5 12M11 17.5 13.5 12"/>
      <circle cx="6"  cy="6.5"  r="0.6" fill="currentColor" stroke="none"/>
      <circle cx="11" cy="6.5"  r="0.6" fill="currentColor" stroke="none"/>
      <circle cx="6"  cy="17.5" r="0.6" fill="currentColor" stroke="none"/>
      <circle cx="11" cy="17.5" r="0.6" fill="currentColor" stroke="none"/>
      <circle cx="18" cy="12" r="2.2" fill="currentColor" stroke="none"/>
    </g>
  ),
  notes: (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <path d="M6 3.5h9l4 4V19a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 5 19V5a1.5 1.5 0 0 1 1-1.5"/>
      <path d="M15 3.5V7.5H19"/>
      <path d="M8 11h7M8 14h7M8 17h4.5"/>
    </g>
  ),
  templates: (
    <g fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <rect x="7"   y="7"   width="12" height="12" rx="1.4"/>
      <path d="M5 16V6A1.5 1.5 0 0 1 6.5 4.5H16" strokeOpacity="0.45"/>
      <path d="M3 14V4.5A1.5 1.5 0 0 1 4.5 3H13"  strokeOpacity="0.24"/>
      <path d="M10.5 11h5M10.5 13.5h3.5" strokeWidth="1.15"/>
    </g>
  ),
};

function TypeGlyph({ type, size = 28 }) {
  const meta = TYPE_META[type] || { hue: 215, short: (type || "").slice(0,3).toUpperCase(), label: type };
  const icon = TYPE_ICONS[type];
  const pad = Math.round(size * 0.14);
  return (
    <div className="ac-glyph" style={{
      width: size, height: size,
      color: `hsl(${meta.hue} 68% 44%)`,
      background: `hsl(${meta.hue} 78% 95%)`,
      padding: pad,
    }} aria-label={meta.label}>
      {icon ? (
        <svg viewBox="0 0 24 24" width="100%" height="100%" aria-hidden>{icon}</svg>
      ) : (
        <span className="ac-glyph__label">{meta.short}</span>
      )}
    </div>
  );
}

Object.assign(window, {
  Icon, AtomicMark, Chip, StatusBadge, PipelineMini, TypeGlyph, WorkKindBadge,
  TYPE_META, TYPE_ICONS,
  renderMarkdown,
});
