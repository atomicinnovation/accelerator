(function (root) {
  'use strict';
  const { FONT_METRICS } = typeof module === 'object' && module.exports ? require('./font-metrics.js') : root.DiagramKit;

  const ROLES = {
    name: { face: 'mono-400', size: 12, lineHeight: 15, letterSpacing: 0 },
    title: { face: 'mono-600', size: 12, lineHeight: 15, letterSpacing: 0 },
    detail: { face: 'sans-400', size: 11, lineHeight: 14, letterSpacing: 0 },
    edge: { face: 'sans-400', size: 11, lineHeight: 14, letterSpacing: 0 },
    heading: { face: 'sans-600', size: 10.5, lineHeight: 14, letterSpacing: 0.08 },
    message: { face: 'sans-400', size: 12, lineHeight: 15, letterSpacing: 0 },
    operator: { face: 'sans-600', size: 11, lineHeight: 14, letterSpacing: 0 },
    guard: { face: 'sans-400', size: 11, lineHeight: 14, letterSpacing: 0 },
  };

  function roleOf(role) {
    if (typeof role === 'object') return role;
    if (!ROLES[role]) throw new Error(`no text role "${role}"`);
    return ROLES[role];
  }

  function widthOf(text, role) {
    const { face, size, letterSpacing } = roleOf(role);
    const { usual, others } = FONT_METRICS[face];
    let ems = 0;
    for (const character of String(text ?? '')) ems += (others[character] ?? usual) + letterSpacing;
    return ems * size;
  }

  function wrapped(text, role, limit) {
    const lines = [];
    let line = '';
    for (const word of String(text ?? '').split(/\s+/).filter(Boolean)) {
      const longer = line ? `${line} ${word}` : word;
      if (line && widthOf(longer, role) > limit) { lines.push(line); line = word; } else line = longer;
    }
    if (line) lines.push(line);
    return lines.length ? lines : [''];
  }

  function blockOf(text, role, limit = Infinity) {
    const lines = wrapped(text, role, limit);
    return { role, lines, width: Math.max(...lines.map(line => widthOf(line, role))), height: lines.length * roleOf(role).lineHeight };
  }

  const api = { ROLES, widthOf, wrapped, blockOf };
  if (typeof module === 'object' && module.exports) module.exports = api;
  else root.DiagramKit = Object.assign(root.DiagramKit || {}, api);
})(typeof window !== 'undefined' ? window : globalThis);
