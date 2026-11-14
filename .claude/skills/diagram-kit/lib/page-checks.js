// What a page of diagrams is put through in a browser, as expressions to
// evaluate there. A page that embeds diagrams runs the same checks on them
// as the kit runs on its own pages.

const DRAWING_CHECKS = `(async () => {
  await document.fonts.ready;
  const fontsLoaded = document.fonts.check('12px "JetBrains Mono"') && document.fonts.check('11px "IBM Plex Sans"');
  const boxOf = element => { const b = element.getBBox(); return { left: b.x, top: b.y, right: b.x + b.width, bottom: b.y + b.height }; };
  const overlap = (a, b, slack = 1) => a.left < b.right - slack && b.left < a.right - slack && a.top < b.bottom - slack && b.top < a.bottom - slack;
  const beyond = (inner, outer) => Math.ceil(Math.max(outer.left - inner.left, inner.right - outer.right, outer.top - inner.top, inner.bottom - outer.bottom, 0));
  const diagrams = [...document.querySelectorAll('svg.dk')].map(svg => {
    const problems = [];
    const boxes = [...svg.querySelectorAll('.node.leaf')].map(node => ({ id: node.getAttribute('data-node'), node, body: boxOf(node.querySelector('.body')) }));
    for (const node of svg.querySelectorAll('.node')) {
      const body = boxOf(node.querySelector('.body'));
      for (const text of node.querySelectorAll('text')) {
        const outside = beyond(boxOf(text), body);
        if (outside > 0) problems.push('node ' + node.getAttribute('data-node') + ': text runs ' + outside + 'px outside its box');
      }
    }
    boxes.forEach((a, i) => boxes.slice(i + 1).forEach(b => { if (overlap(a.body, b.body)) problems.push('node ' + a.id + ': overlaps node ' + b.id); }));
    for (const edge of svg.querySelectorAll('.edge')) {
      const id = edge.getAttribute('data-edge'), ends = [edge.getAttribute('data-from'), edge.getAttribute('data-to')];
      const line = edge.querySelector('.line'), length = line.getTotalLength();
      const label = edge.querySelector('rect'), text = edge.querySelector('text');
      if (label && text && beyond(boxOf(text), boxOf(label)) > 0) problems.push('edge ' + id + ': label runs outside its plate');
      const crossed = new Set();
      for (let at = 0; at <= length; at += 3) {
        const point = line.getPointAtLength(at);
        boxes.forEach(box => { if (!ends.includes(box.id) && point.x > box.body.left + 1 && point.x < box.body.right - 1 && point.y > box.body.top + 1 && point.y < box.body.bottom - 1) crossed.add(box.id); });
      }
      crossed.forEach(box => problems.push('edge ' + id + ': runs across node ' + box));
    }
    const extent = svg.viewBox.baseVal;
    const stray = [...svg.querySelectorAll('.node .body, .edge .line')].some(shape => { const b = boxOf(shape); return b.left < extent.x - 1 || b.top < extent.y - 1 || b.right > extent.x + extent.width + 1 || b.bottom > extent.y + extent.height + 1; });
    if (stray) problems.push('something is drawn outside the drawing');
    return { id: svg.getAttribute('data-graph'), nodes: svg.querySelectorAll('.node').length, edges: svg.querySelectorAll('.edge').length, problems };
  });
  return { fontsLoaded, diagrams };
})()`;

const TRACING_CHECK = `(async () => {
  const settle = () => new Promise(resolve => setTimeout(resolve, 120));
  const svg = [...document.querySelectorAll('svg.dk')].find(each => each.querySelector('.edge'));
  if (!svg) return { skipped: 'no drawing with an arrow' };
  const from = svg.querySelector('.edge').getAttribute('data-from');
  const node = svg.querySelector('[data-node="' + CSS.escape(from) + '"] .body');
  node.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
  await settle();
  const traces = svg.classList.contains('tracing') && !!svg.querySelector('.edge.traced-out');
  document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  await settle();
  return { traces, clears: !svg.classList.contains('tracing') };
})()`;

module.exports = { DRAWING_CHECKS, TRACING_CHECK };
