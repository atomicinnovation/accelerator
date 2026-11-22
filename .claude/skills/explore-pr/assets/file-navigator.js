(function (root) {
  'use strict';

  const BLANK = { progress: [], verdict: [], layer: [], risk: [], change: [], chapter: [], mentions: [], commit: [] };
  const FACETS = Object.keys(BLANK);
  const RISKS = ['high', 'medium', 'low', 'skim'];
  const CHANGES = ['added', 'modified', 'deleted', 'renamed'];
  const OUTSIDE_THE_TOUR = 'Not in the tour';

  const riskOf = entry => entry.risk || 'skim';
  function progressOf(entry) {
    const { changed, state } = entry.coverage || {};
    if (changed) return state;
    return entry.verdict ? 'read' : 'unread';
  }
  const VALUES_OF = {
    progress: entry => [progressOf(entry)],
    verdict: entry => [entry.verdict || 'none'],
    layer: entry => [entry.layer],
    risk: entry => [riskOf(entry)],
    change: entry => entry.changes || [],
    chapter: entry => (entry.chapter ? [entry.chapter] : []),
    mentions: entry => entry.mentions || [],
    commit: entry => entry.commits || [],
  };

  const literally = text => text.replace(/[.+?^${}()|[\]\\]/g, '\\$&');
  const termMatcher = term => new RegExp(term.split('*').map(literally).join('.*'), 'i');
  function parseQuery(text) {
    const terms = String(text || '').trim().split(/\s+/).filter(Boolean);
    const excluded = terms.filter(term => term.startsWith('!') && term.length > 1).map(term => termMatcher(term.slice(1)));
    const required = terms.filter(term => !term.startsWith('!')).map(termMatcher);
    return filePath => required.every(re => re.test(filePath)) && !excluded.some(re => re.test(filePath));
  }

  const facetHolds = (entry, facet, chosen) => !chosen.length || VALUES_OF[facet](entry).some(value => chosen.includes(value));
  const holdsExcept = (entry, filters, skipped) => FACETS.every(facet => facet === skipped || facetHolds(entry, facet, filters[facet] || []));

  function matches(entry, filters, query) {
    return holdsExcept(entry, filters, null) && parseQuery(query)(entry.path);
  }

  function facetCounts(entries, filters, query) {
    const inQuery = entries.filter(entry => parseQuery(query)(entry.path));
    return Object.fromEntries(FACETS.map(facet => {
      const counts = Object.fromEntries((filters[facet] || []).map(value => [value, 0]));
      inQuery.filter(entry => holdsExcept(entry, filters, facet))
        .forEach(entry => VALUES_OF[facet](entry).forEach(value => { counts[value] = (counts[value] || 0) + 1; }));
      return [facet, counts];
    }));
  }

  const riskRank = risk => (RISKS.includes(risk) ? RISKS.indexOf(risk) : RISKS.length);
  const orderOrLast = value => (value == null ? Infinity : value);

  function fileTotals(entry) {
    const { changed = 0, read = 0 } = entry.coverage || {};
    return {
      files: 1, changed, read, unread: changed - read, size: (entry.add || 0) + (entry.del || 0), highestRisk: riskOf(entry),
      verdicts: { ok: 0, skip: 0, issue: 0, none: 0, [entry.verdict || 'none']: 1 },
      readingOrder: orderOrLast(entry.readingOrder), firstCommit: orderOrLast(entry.firstCommit),
    };
  }
  function summed(nodes) {
    const totals = { files: 0, changed: 0, read: 0, unread: 0, size: 0, highestRisk: null, verdicts: { ok: 0, skip: 0, issue: 0, none: 0 }, readingOrder: Infinity, firstCommit: Infinity };
    for (const { totals: each } of nodes) {
      for (const key of ['files', 'changed', 'read', 'unread', 'size']) totals[key] += each[key];
      for (const key of Object.keys(totals.verdicts)) totals.verdicts[key] += each.verdicts[key] || 0;
      if (totals.highestRisk === null || riskRank(each.highestRisk) < riskRank(totals.highestRisk)) totals.highestRisk = each.highestRisk;
      totals.readingOrder = Math.min(totals.readingOrder, each.readingOrder);
      totals.firstCommit = Math.min(totals.firstCommit, each.firstCommit);
    }
    return totals;
  }

  const MEASURES = {
    path: () => 0,
    unread: totals => -totals.unread,
    size: totals => -totals.size,
    risk: totals => riskRank(totals.highestRisk),
    reading: totals => totals.readingOrder,
    commit: totals => totals.firstCommit,
  };
  function bySort(sortBy) {
    const measure = MEASURES[sortBy] || MEASURES.path;
    return (a, b) => (a.kind === b.kind ? 0 : a.kind === 'folder' ? -1 : 1)
      || (measure(a.totals) - measure(b.totals) || 0)
      || a.name.localeCompare(b.name, 'en-GB');
  }

  function folderTree(entries, scope, sortBy) {
    const top = { folders: new Map(), files: [] };
    for (const entry of entries) {
      const parts = entry.path.split('/');
      let at = top;
      for (const part of parts.slice(0, -1)) {
        if (!at.folders.has(part)) at.folders.set(part, { folders: new Map(), files: [] });
        at = at.folders.get(part);
      }
      at.files.push(entry);
    }
    const order = bySort(sortBy);
    const build = (folder, prefix) => {
      const folders = [...folder.folders].map(([part, inner]) => {
        let name = part + '/', path = prefix + name;
        while (inner.files.length === 0 && inner.folders.size === 1) {
          const [[nextPart, next]] = [...inner.folders];
          name += nextPart + '/'; path += nextPart + '/'; inner = next;
        }
        const children = build(inner, path);
        return { kind: 'folder', id: `folder:${scope}:${path}`, path, name, children, totals: summed(children) };
      });
      const files = folder.files.map(entry => ({ kind: 'file', id: `file:${scope}:${entry.path}`, path: entry.path, name: entry.path.slice(prefix.length), entry, totals: fileTotals(entry) }));
      return [...folders, ...files].sort(order);
    };
    return build(top, '');
  }

  const GROUP_KEY = {
    layer: entry => entry.layer,
    chapter: entry => entry.chapter || '',
    risk: riskOf,
    change: entry => (entry.changes || [])[0] || 'modified',
  };
  const DEFAULT_GROUPS = { risk: RISKS.map(risk => [risk, risk]), change: CHANGES.map(change => [change, change]) };

  function flatList(entries, sortBy) {
    return entries.map(entry => ({ kind: 'file', id: `file::${entry.path}`, path: entry.path, name: entry.path, entry, totals: fileTotals(entry) }))
      .sort(bySort(sortBy));
  }

  function treeOf(entries, { groupBy = 'tree', sortBy = 'path', groups = {} } = {}) {
    if (groupBy === 'none') return flatList(entries, sortBy);
    const keyOf = GROUP_KEY[groupBy];
    if (!keyOf) return folderTree(entries, '', sortBy);
    const known = groups[groupBy] || DEFAULT_GROUPS[groupBy] || [];
    const byKey = new Map();
    entries.forEach(entry => { const key = keyOf(entry); (byKey.get(key) || byKey.set(key, []).get(key)).push(entry); });
    const order = [...known.map(([id]) => id).filter(id => byKey.has(id)), ...[...byKey.keys()].filter(key => key && !known.some(([id]) => id === key))];
    if (byKey.has('') && !order.includes('')) order.push('');
    const labelOf = key => (known.find(([id]) => id === key) || [])[1] || (key === '' && groupBy === 'chapter' ? OUTSIDE_THE_TOUR : key);
    return order.map(key => {
      const children = folderTree(byKey.get(key), key, sortBy);
      return { kind: 'group', id: `group:${groupBy}:${key}`, name: labelOf(key), key, children, totals: summed(children) };
    });
  }

  function rowsOf(nodes, collapsed, depth = 0, rows = []) {
    for (const node of nodes) {
      rows.push({ node, depth });
      if (node.children && !collapsed.has(node.id)) rowsOf(node.children, collapsed, depth + 1, rows);
    }
    return rows;
  }

  const filesUnder = node => (node.kind === 'file' ? [node.path] : node.children.flatMap(filesUnder));
  const fileOrder = nodes => nodes.flatMap(filesUnder);

  function ancestorsOf(nodes, filePath) {
    for (const node of nodes) {
      if (node.kind === 'file') { if (node.path === filePath) return []; continue; }
      const below = ancestorsOf(node.children, filePath);
      if (below.length || node.children.some(child => child.kind === 'file' && child.path === filePath)) return [node.id, ...below];
    }
    return [];
  }

  const FileNavigator = { BLANK, FACETS, parseQuery, matches, facetCounts, treeOf, rowsOf, fileOrder, ancestorsOf, filesUnder, progressOf };
  if (typeof module === 'object' && module.exports) module.exports = FileNavigator;
  else root.FileNavigator = FileNavigator;
})(typeof window !== 'undefined' ? window : globalThis);
