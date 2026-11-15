(function (root) {
  'use strict';
  const inNode = typeof module === 'object' && module.exports;
  const { laidOut } = inNode ? require('./layout.js') : root.DiagramKit;
  const { svgOf, legendOf } = inNode ? require('./render.js') : root.DiagramKit;
  const { addressOnly, inlineOf } = inNode ? require('./html.js') : root.DiagramKit;
  const symbolLegendOf = inNode ? require('./legend.js').symbolsOf : root.DiagramKit.symbolLegendOf;

  const STATUSES = ['new', 'changed', 'removed', 'existing'];
  const TONES = ['risk', 'ok'];
  const TRACE_HINT = '<div class="dk-legend dk-trace-hint">Press a box to trace its arrows (Esc clears): <span><span class="dk-key trace-out"></span>out</span><span><span class="dk-key trace-in"></span>in</span></div>';

  const escaped = text => String(text ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  const outside = (field, value, allowed) => (value == null || allowed.includes(value) ? [] : [`${field} "${value}" is not one of ${allowed.join(', ')}`]);

  function markProblems(item, where) {
    return [
      ...outside('status', item.status, STATUSES),
      ...outside('tone', item.tone, TONES),
      ...(item.link == null || typeof item.link === 'string' ? [] : ['link must be text']),
    ].map(problem => `${where}: ${problem}`);
  }

  function notations(drawn) {
    const byKind = new Map();
    for (const notation of drawn) {
      if (byKind.has(notation.kind)) throw new Error(`two notations draw kind "${notation.kind}"`);
      byKind.set(notation.kind, notation);
    }

    function faultsIn(spec) {
      if (!spec || typeof spec !== 'object' || Array.isArray(spec)) return ['must be an object'];
      if (typeof spec.id !== 'string' || !spec.id.trim()) return ['no id'];
      if (!spec.kind) return ['no kind'];
      const notation = byKind.get(spec.kind);
      if (!notation) return [`no notation draws kind "${spec.kind}"; known kinds: ${[...byKind.keys()].join(', ')}`];
      const envelope = typeof spec.title === 'string' && spec.title.trim() ? [] : ['no title; say the point the diagram makes'];
      try { return [...envelope, ...notation.problemsIn(spec)]; } catch (error) { return [...envelope, `could not be checked: ${error.message}`]; }
    }

    function problemsIn(spec) {
      const id = spec && typeof spec.id === 'string' && spec.id.trim() ? spec.id : '?';
      return faultsIn(spec).map(problem => `diagram ${id}: ${problem}`);
    }

    async function laidOutSpec(spec) {
      const problems = problemsIn(spec);
      if (problems.length) throw new Error(problems.join('\n'));
      const notation = byKind.get(spec.kind);
      if (notation.htmlOf) return null;
      return (notation.placedSceneOf && notation.placedSceneOf(spec)) || laidOut(notation.sceneOf(spec));
    }

    const drawsItself = spec => Boolean(spec && byKind.get(spec.kind) && byKind.get(spec.kind).htmlOf);

    const remarksOn = spec => (byKind.get(spec.kind).remarksOn || (() => []))(spec);

    function remarksMarkup(spec) {
      const remarks = remarksOn(spec);
      return remarks.length ? `<ul class="dk-remarks">${remarks.map(remark => `<li>${escaped(remark)}</li>`).join('')}</ul>` : '';
    }

    const symbolsOf = (spec, scene) => symbolLegendOf(byKind.get(spec.kind), spec, scene);

    function drawnOf(spec, scene, resolveLink, legend) {
      const symbols = legend ? symbolsOf(spec, scene) : '';
      if (drawsItself(spec)) return `<div class="dk-drawn">${byKind.get(spec.kind).htmlOf(spec, { resolveLink: resolveLink || addressOnly })}</div>${symbols}`;
      const drawing = svgOf(scene, { uid: spec.id, title: spec.title, ...(resolveLink ? { resolveLink } : {}) });
      return `<div class="dk-scroll">${drawing}</div>${legendOf(scene)}${symbols || (scene.edges.length ? TRACE_HINT : '')}`;
    }

    function figureOf(spec, scene, { resolveLink, legend = false } = {}) {
      return `<figure class="dk-figure" id="diagram-${escaped(spec.id)}"><figcaption class="dk-title">${escaped(spec.title)}</figcaption>`
        + `${drawnOf(spec, scene, resolveLink, legend)}${remarksMarkup(spec)}${spec.caption ? `<p class="dk-caption">${inlineOf(spec.caption)}</p>` : ''}</figure>`;
    }

    return { kinds: () => [...byKind.keys()], faultsIn, problemsIn, remarksOn, laidOut: laidOutSpec, drawsItself, symbolsOf, figureOf };
  }

  const api = { notations, markProblems, STATUSES, TONES };
  if (inNode) module.exports = api;
  else root.DiagramKit = Object.assign(root.DiagramKit || {}, api);
})(typeof window !== 'undefined' ? window : globalThis);
