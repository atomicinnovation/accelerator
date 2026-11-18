(function (root) {
  'use strict';

  const KINDS = { decisions: 'decision', abstractions: 'abstraction', invariants: 'invariant', risks: 'risk' };
  const LABELLED_BY = { decision: 'decision', abstraction: 'name', invariant: 'text', risk: 'text' };

  function declarations(content) {
    const holders = [[null, content.design || {}], ...(content.chapters || []).map(chapter => [chapter.id, chapter])];
    const declared = new Map();
    holders.forEach(([chapter, holder]) => {
      Object.entries(KINDS).forEach(([collection, kind]) => {
        (holder[collection] || []).forEach(item => {
          if (!item.id || declared.has(item.id)) return;
          declared.set(item.id, { id: item.id, kind, label: item[LABELLED_BY[kind]] || item.id, chapter: item.chapter || chapter });
        });
      });
    });
    return declared;
  }

  function showings(content) {
    const shown = new Map();
    (content.chapters || []).forEach(chapter => (chapter.stops || []).forEach(stop => (stop.carries || []).forEach(id => {
      if (!shown.has(id)) shown.set(id, []);
      shown.get(id).push(stop.id);
    })));
    return shown;
  }

  function index(content) {
    const declared = declarations(content), shown = showings(content);
    return {
      declared: id => declared.get(id) || null,
      carriedBy: stop => (stop.carries || []).filter(id => declared.has(id)).map(id => declared.get(id)),
      shownAt: id => shown.get(id) || [],
    };
  }

  const StopEvidence = { index };
  if (typeof module === 'object' && module.exports) module.exports = StopEvidence;
  else root.StopEvidence = StopEvidence;
})(typeof window !== 'undefined' ? window : globalThis);
