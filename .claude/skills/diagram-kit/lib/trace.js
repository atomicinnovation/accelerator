(function (root) {
  'use strict';

  function focusOfAll(edges, traced) {
    if (!traced.length) return null;
    const isTraced = new Set(traced);
    const outbound = [], inbound = [], neighbours = new Set();
    edges.forEach((edge, i) => {
      if (isTraced.has(edge.from)) outbound.push(i);
      else if (isTraced.has(edge.to)) inbound.push(i);
      else return;
      [edge.from, edge.to].filter(id => !isTraced.has(id)).forEach(id => neighbours.add(id));
    });
    return { outbound, inbound, neighbours: [...neighbours] };
  }

  function focusOf(edges, node) {
    const focus = focusOfAll(edges, [node]);
    return focus.outbound.length || focus.inbound.length ? focus : null;
  }

  function toggle(traced, id) {
    const next = new Set(traced);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    return next;
  }

  const GRAPH = 'svg[data-graph]';
  const NODE = '[data-node]';
  const LINK = '.node-link';
  const TRACE_CLASSES = ['traced', 'traced-neighbour', 'traced-out', 'traced-in'];

  function attach(doc) {
    const tracedIn = new Map();

    function show(svg) {
      svg.querySelectorAll(TRACE_CLASSES.map(cls => '.' + cls).join(',')).forEach(el => el.classList.remove(...TRACE_CLASSES));
      const ends = [...svg.querySelectorAll('[data-from][data-to]')];
      const focus = focusOfAll(ends.map(el => ({ from: el.getAttribute('data-from'), to: el.getAttribute('data-to') })), [...(tracedIn.get(svg) || [])]);
      svg.classList.toggle('tracing', !!focus);
      if (!focus) return;
      focus.outbound.forEach(i => ends[i].classList.add('traced-out'));
      focus.inbound.forEach(i => ends[i].classList.add('traced-in'));
      const traced = tracedIn.get(svg);
      svg.querySelectorAll(NODE).forEach(el => {
        const id = el.getAttribute('data-node');
        if (traced.has(id)) el.classList.add('traced');
        else if (focus.neighbours.includes(id)) el.classList.add('traced-neighbour');
      });
    }

    function clearAll() {
      [...tracedIn.keys()].forEach(svg => { tracedIn.delete(svg); show(svg); });
    }

    const press = e => {
      const target = e.target.closest ? e.target : null;
      if (!target || target.closest(LINK)) return;
      const node = target.closest(`${GRAPH} ${NODE}`);
      if (!node) return;
      const svg = node.closest(GRAPH);
      tracedIn.forEach((_, known) => { if (!known.isConnected) tracedIn.delete(known); });
      const traced = toggle(tracedIn.get(svg) || new Set(), node.getAttribute('data-node'));
      if (traced.size) tracedIn.set(svg, traced);
      else tracedIn.delete(svg);
      show(svg);
      e.preventDefault();
    };
    const key = e => {
      if (e.key === 'Escape') clearAll();
      else if ((e.key === 'Enter' || e.key === ' ') && e.target.matches && e.target.matches(`${GRAPH} ${NODE}`)) press(e);
    };
    doc.addEventListener('click', press, true);
    doc.addEventListener('keydown', key);
  }

  const tracing = { focusOf, focusOfAll, toggle, attachTracing: attach };
  if (typeof module === 'object' && module.exports) module.exports = tracing;
  else root.DiagramKit = Object.assign(root.DiagramKit || {}, tracing);
})(typeof window !== 'undefined' ? window : globalThis);
