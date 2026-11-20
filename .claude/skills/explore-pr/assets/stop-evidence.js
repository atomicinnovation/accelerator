(function (root) {
  'use strict';

  const KINDS = { decisions: 'decision', abstractions: 'abstraction', invariants: 'invariant', risks: 'risk' };
  const LABELLED_BY = { decision: 'decision', abstraction: 'name', invariant: 'text', risk: 'text' };

  const INTRODUCED = new Set(['decision', 'abstraction']);
  const holdersOf = content => [[null, content.design || {}], ...(content.chapters || []).map(chapter => [chapter.id, chapter])];
  const stopsOf = content => (content.chapters || []).flatMap(chapter => chapter.stops || []);

  function declarations(content) {
    const declared = new Map();
    holdersOf(content).forEach(([chapter, holder]) => {
      Object.entries(KINDS).forEach(([collection, kind]) => {
        (holder[collection] || []).forEach(item => {
          if (!item.id || declared.has(item.id)) return;
          declared.set(item.id, { id: item.id, kind, label: item[LABELLED_BY[kind]] || item.id, chapter: item.chapter || chapter });
        });
      });
    });
    return declared;
  }

  function namedCarriers(content, declared) {
    const carriers = new Map();
    holdersOf(content).forEach(([, holder]) => Object.keys(KINDS).forEach(collection => (holder[collection] || []).forEach(item => {
      if (!declared.has(item.id) || carriers.has(item.id)) return;
      carriers.set(item.id, item.carriedBy || []);
    })));
    return carriers;
  }

  function showings(content, declared) {
    const tourOrder = new Map(stopsOf(content).map((stop, position) => [stop.id, position]));
    const shown = new Map();
    const show = (id, stopId) => {
      if (!tourOrder.has(stopId)) return;
      if (!shown.has(id)) shown.set(id, new Set());
      shown.get(id).add(stopId);
    };
    stopsOf(content).forEach(stop => (stop.carries || []).forEach(id => show(id, stop.id)));
    namedCarriers(content, declared).forEach((stopIds, id) => stopIds.forEach(stopId => show(id, stopId)));
    return new Map([...shown].map(([id, stops]) => [id, [...stops].sort((a, b) => tourOrder.get(a) - tourOrder.get(b))]));
  }

  function index(content) {
    const declared = declarations(content), shown = showings(content, declared);
    const shownAt = id => shown.get(id) || [];
    const carriedBy = stop => {
      const ids = [...(stop.carries || []), ...[...declared.keys()].filter(id => shownAt(id).includes(stop.id))];
      return [...new Set(ids)].filter(id => declared.has(id)).map(id => declared.get(id));
    };
    const introducedAt = stop => [...declared.values()]
      .filter(item => INTRODUCED.has(item.kind) && shownAt(item.id)[0] === stop.id);
    return { declared: id => declared.get(id) || null, carriedBy, shownAt, introducedAt };
  }

  const StopEvidence = { index };
  if (typeof module === 'object' && module.exports) module.exports = StopEvidence;
  else root.StopEvidence = StopEvidence;
})(typeof window !== 'undefined' ? window : globalThis);
