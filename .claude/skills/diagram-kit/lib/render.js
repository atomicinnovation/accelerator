(function (root) {
  'use strict';
  const inNode = typeof module === 'object' && module.exports;
  const { blockOf, ROLES } = inNode ? require('./text.js') : root.DiagramKit;
  const { SHAPES, LINK_CONTROL_WIDTH } = inNode ? require('./shapes.js') : root.DiagramKit;

  const MARGIN = 8;
  const HEAD_LENGTH = 9, HEAD_HALF_WIDTH = 4.5;
  const CORNER = 6;
  const STATUSES = { new: 'new in this change', changed: 'changed', removed: 'removed', existing: 'unchanged' };
  const TONES = { risk: 'risk', ok: 'safe' };

  const escaped = text => String(text ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  const tidy = number => String(Math.round(number * 100) / 100);
  const marks = item => `${STATUSES[item.status] ? ` s-${item.status}` : ''}${TONES[item.tone] ? ` t-${item.tone}` : ''}`;

  function addressOnly(reference) {
    return /^(https?:|#)/.test(String(reference)) ? { href: String(reference), label: `Open ${reference}` } : null;
  }

  function textOf(block, role, x, firstBaseline, anchor) {
    const open = `<text class="${role}" x="${tidy(x)}" y="${tidy(firstBaseline)}" text-anchor="${anchor}">`;
    if (block.lines.length === 1) return `${open}${escaped(block.lines[0])}</text>`;
    return `${open}${block.lines.map((line, i) => `<tspan x="${tidy(x)}" dy="${i ? ROLES[block.role].lineHeight : 0}">${escaped(line)}</tspan>`).join('')}</text>`;
  }

  function linkControl(node, target) {
    const right = node.x + node.width, left = right - LINK_CONTROL_WIDTH, top = node.y, bottom = node.y + node.height, r = 7;
    const cx = left + LINK_CONTROL_WIDTH / 2, cy = top + node.height / 2;
    const attributes = Object.entries(target.attributes || {}).map(([name, value]) => ` ${name}="${escaped(value)}"`).join('');
    const hit = `M${tidy(left)} ${tidy(top)} H${tidy(right - r)} Q${tidy(right)} ${tidy(top)} ${tidy(right)} ${tidy(top + r)} V${tidy(bottom - r)} Q${tidy(right)} ${tidy(bottom)} ${tidy(right - r)} ${tidy(bottom)} H${tidy(left)} Z`;
    return `<a class="node-link" href="${escaped(target.href)}"${attributes} aria-label="${escaped(target.label)}"><title>${escaped(target.label)}</title>`
      + `<path class="hit" d="${hit}"/><line class="divider" x1="${tidy(left)}" y1="${tidy(top + 6)}" x2="${tidy(left)}" y2="${tidy(bottom - 6)}"/>`
      + `<path class="glyph" d="M${tidy(cx - 4)} ${tidy(cy + 4)} L${tidy(cx + 4)} ${tidy(cy - 4)} M${tidy(cx - 2)} ${tidy(cy - 4)} L${tidy(cx + 4)} ${tidy(cy - 4)} L${tidy(cx + 4)} ${tidy(cy + 2)}"/></a>`;
  }

  const bodyOf = (node, radius) => `<rect class="body" x="${tidy(node.x)}" y="${tidy(node.y)}" width="${tidy(node.width)}" height="${tidy(node.height)}" rx="${radius}"/>`;
  const untold = (node, target) => (node.link && !target ? `<title>${escaped(node.link)}</title>` : '');

  const DRAWN = {
    box(node, target) {
      const { label, detail } = SHAPES.box.blocksOf(node);
      const centre = node.x + (node.width - (target ? LINK_CONTROL_WIDTH : 0)) / 2;
      const firstBaseline = node.y + (detail ? 19 : 20);
      return bodyOf(node, 7) + textOf(label, 'name', centre, firstBaseline, 'middle')
        + (detail ? textOf(detail, 'detail', centre, firstBaseline + label.height + 1, 'middle') : '')
        + (target ? linkControl(node, target) : '') + untold(node, target);
    },
    group(node, target) {
      const heading = textOf(blockOf(String(node.label ?? node.id).toUpperCase(), 'heading'), 'heading', node.x + 14, node.y + 20, 'start');
      return bodyOf(node, 9) + (target ? `<a href="${escaped(target.href)}" aria-label="${escaped(target.label)}">${heading}</a>` : heading) + untold(node, target);
    },
  };

  function nodeMarkup(node, resolveLink) {
    const target = node.link ? resolveLink(node.link) : null;
    const name = escaped(node.label ?? node.id);
    return `<g class="node ${node.shape}${marks(node)}" data-node="${escaped(node.id)}" tabindex="0" role="button" aria-label="Trace the arrows of ${name}">${DRAWN[node.shape](node, target)}</g>`;
  }

  function roundedPath(points, radius = CORNER) {
    const lengthOf = (a, b) => Math.hypot(b[0] - a[0], b[1] - a[1]);
    const towards = (from, to, distance) => { const length = lengthOf(from, to) || 1; return [from[0] + (to[0] - from[0]) * distance / length, from[1] + (to[1] - from[1]) * distance / length]; };
    let path = `M${tidy(points[0][0])} ${tidy(points[0][1])}`;
    for (let i = 1; i < points.length - 1; i++) {
      const corner = points[i], r = Math.min(radius, lengthOf(points[i - 1], corner) / 2, lengthOf(corner, points[i + 1]) / 2);
      const before = towards(corner, points[i - 1], r), after = towards(corner, points[i + 1], r);
      path += ` L${tidy(before[0])} ${tidy(before[1])} Q${tidy(corner[0])} ${tidy(corner[1])} ${tidy(after[0])} ${tidy(after[1])}`;
    }
    const last = points[points.length - 1];
    return `${path} L${tidy(last[0])} ${tidy(last[1])}`;
  }

  function headOf(kind, points) {
    if (!kind || kind === 'none' || points.length < 2) return { path: '', cut: 0 };
    const tip = points[points.length - 1], from = points[points.length - 2];
    const length = Math.hypot(tip[0] - from[0], tip[1] - from[1]) || 1;
    const along = [(tip[0] - from[0]) / length, (tip[1] - from[1]) / length];
    const base = [tip[0] - along[0] * HEAD_LENGTH, tip[1] - along[1] * HEAD_LENGTH];
    const wings = [-1, 1].map(side => [base[0] - along[1] * HEAD_HALF_WIDTH * side, base[1] + along[0] * HEAD_HALF_WIDTH * side])
      .sort((a, b) => a[0] - b[0] || a[1] - b[1]);
    const strokes = `M${tidy(wings[0][0])} ${tidy(wings[0][1])} L${tidy(tip[0])} ${tidy(tip[1])} L${tidy(wings[1][0])} ${tidy(wings[1][1])}`;
    return kind === 'open' ? { path: strokes, cut: 0 } : { path: `${strokes} Z`, cut: HEAD_LENGTH };
  }

  function shortened(points, cut) {
    if (!cut) return points;
    const tip = points[points.length - 1], from = points[points.length - 2];
    const length = Math.hypot(tip[0] - from[0], tip[1] - from[1]) || 1;
    return [...points.slice(0, -1), [tip[0] - (tip[0] - from[0]) * cut / length, tip[1] - (tip[1] - from[1]) * cut / length]];
  }

  function edgeLabelMarkup(edge) {
    if (!edge.label || !edge.labelBox) return '';
    const { x, y, width, height } = edge.labelBox;
    const block = blockOf(edge.label, 'edge', width - 12);
    return `<rect x="${tidy(x)}" y="${tidy(y)}" width="${tidy(width)}" height="${tidy(height)}" rx="9"/>${textOf(block, 'edge-text', x + width / 2, y + (height - block.height) / 2 + 11, 'middle')}`;
  }

  function edgeMarkup(edge) {
    if (edge.points.length < 2) return '';
    const head = headOf(edge.head, edge.points);
    const line = `<path class="line" d="${roundedPath(shortened(edge.points, head.cut))}"/>`;
    const arrow = head.path ? `<path class="head ${edge.head}" d="${head.path}"/>` : '';
    return `<g class="edge${edge.line === 'dashed' ? ' dashed' : ''}${marks(edge)}" data-edge="${escaped(edge.id)}" data-from="${escaped(edge.from)}" data-to="${escaped(edge.to)}">${line}${arrow}${edgeLabelMarkup(edge)}</g>`;
  }

  function svgOf(scene, { uid = 'diagram', title = '', resolveLink = addressOnly } = {}) {
    const width = scene.width + 2 * MARGIN, height = scene.height + 2 * MARGIN;
    const holders = scene.nodes.filter(node => SHAPES[node.shape].holdsOthers), leaves = scene.nodes.filter(node => !SHAPES[node.shape].holdsOthers);
    const draw = nodes => nodes.map(node => nodeMarkup(node, resolveLink)).join('');
    return `<svg class="dk" data-graph="${escaped(uid)}" xmlns="http://www.w3.org/2000/svg" width="${tidy(width)}" height="${tidy(height)}" viewBox="${-MARGIN} ${-MARGIN} ${tidy(width)} ${tidy(height)}" role="img" aria-label="${escaped(title)}">`
      + `${draw(holders)}${scene.edges.map(edgeMarkup).join('')}${draw(leaves)}</svg>`;
  }

  function legendOf(scene) {
    const items = [...scene.nodes, ...scene.edges];
    const keys = [
      ...Object.keys(STATUSES).filter(status => items.some(item => item.status === status)).map(status => `<span><span class="dk-key s-${status}"></span>${STATUSES[status]}</span>`),
      ...Object.keys(TONES).filter(tone => items.some(item => item.tone === tone)).map(tone => `<span><span class="dk-key t-${tone}"></span>${TONES[tone]}</span>`),
    ];
    return keys.length ? `<div class="dk-legend">${keys.join('')}</div>` : '';
  }

  const api = { svgOf, legendOf, roundedPath, headOf };
  if (inNode) module.exports = api;
  else root.DiagramKit = Object.assign(root.DiagramKit || {}, api);
})(typeof window !== 'undefined' ? window : globalThis);
