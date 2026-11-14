(function (root) {
  'use strict';
  const inNode = typeof module === 'object' && module.exports;
  const { escaped, inlineOf, linked } = inNode ? require('../diagram-kit/lib/html.js') : root.DiagramKit;

  const STATES = ['added', 'removed', 'changed', 'kept'];
  const MARKS = { added: '+ ', removed: '− ', changed: '~ ' };
  const listOf = value => (Array.isArray(value) ? value : []);

  function problemsIn(spec) {
    if (!listOf(spec.items).length) return ['no items'];
    const written = new Set();
    const items = spec.items.flatMap((item, position) => {
      if (!item || typeof item.path !== 'string' || !item.path.trim()) return [`item ${position + 1}: no path`];
      const where = `item ${item.path}`;
      if (written.has(item.path)) return [`${where}: path is written twice`];
      written.add(item.path);
      return [
        ...(item.state == null || STATES.includes(item.state) ? [] : [`${where}: state "${item.state}" is not one of ${STATES.join(', ')}`]),
        ...['note', 'link'].filter(field => item[field] != null && typeof item[field] !== 'string').map(field => `${where}: ${field} must be text`),
        ...(item.where == null ? [] : [`${where}: where is now link`]),
      ];
    });
    return [...(spec.root == null || typeof spec.root === 'string' ? [] : ['root must be text']), ...items];
  }

  function entriesOf(items) {
    const top = { children: new Map() };
    for (const item of items) {
      const parts = item.path.split('/').filter(Boolean);
      let entry = top;
      parts.forEach((part, depth) => {
        if (!entry.children.has(part)) entry.children.set(part, { name: part, children: new Map(), isDirectory: depth < parts.length - 1 });
        entry = entry.children.get(part);
      });
      entry.item = item;
      if (item.path.endsWith('/')) entry.isDirectory = true;
    }
    const rows = [];
    const walk = (entry, prefix) => {
      const children = [...entry.children.values()];
      children.forEach((child, i) => {
        const last = i === children.length - 1;
        rows.push({ guide: prefix + (last ? '└── ' : '├── '), entry: child });
        walk(child, prefix + (last ? '    ' : '│   '));
      });
    };
    walk(top, '');
    return rows;
  }

  function rowOf({ guide, entry }, resolveLink) {
    const item = entry.item || {};
    const name = escaped(`${MARKS[item.state] || ''}${entry.name}${entry.isDirectory ? '/' : ''}`);
    return `<div class="dk-row${item.state ? ` s-${item.state}` : ''}"><span class="dk-guide">${escaped(guide)}</span>`
      + `<span class="dk-name${entry.isDirectory ? ' dk-dir' : ''}">${linked(name, item.link ? resolveLink(item.link) : null)}</span>`
      + `${item.note ? `<span class="dk-note">${inlineOf(item.note)}</span>` : ''}</div>`;
  }

  function htmlOf(spec, { resolveLink }) {
    const head = spec.root ? `<div class="dk-row"><span class="dk-name dk-dir">${escaped(spec.root)}</span></div>` : '';
    return `<div class="dk-tree">${head}${entriesOf(spec.items).map(row => rowOf(row, resolveLink)).join('')}</div>`;
  }

  const notation = { kind: 'tree', problemsIn, htmlOf };
  if (inNode) module.exports = notation;
  else root.DiagramKit.NOTATIONS = Object.assign(root.DiagramKit.NOTATIONS || {}, { [notation.kind]: notation });
})(typeof window !== 'undefined' ? window : globalThis);
