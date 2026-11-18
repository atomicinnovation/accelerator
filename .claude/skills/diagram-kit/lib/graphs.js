(function (root) {
  'use strict';

  function cyclesIn(arrows) {
    const needs = new Map();
    for (const { from, to } of arrows) {
      for (const end of [from, to]) if (!needs.has(end)) needs.set(end, []);
      needs.get(from).push(to);
    }
    const order = new Map(), lowest = new Map(), stack = [], onStack = new Set(), knots = [];
    const visit = module => {
      order.set(module, order.size);
      lowest.set(module, order.get(module));
      stack.push(module);
      onStack.add(module);
      for (const needed of needs.get(module)) {
        if (!order.has(needed)) { visit(needed); lowest.set(module, Math.min(lowest.get(module), lowest.get(needed))); }
        else if (onStack.has(needed)) lowest.set(module, Math.min(lowest.get(module), order.get(needed)));
      }
      if (lowest.get(module) !== order.get(module)) return;
      const knot = [];
      let member;
      do { member = stack.pop(); onStack.delete(member); knot.push(member); } while (member !== module);
      if (knot.length > 1 || needs.get(module).includes(module)) knots.push(new Set(knot));
    };
    for (const module of needs.keys()) if (!order.has(module)) visit(module);
    return knots;
  }

  function loopThrough(knot, arrows) {
    const start = arrows.find(({ from }) => knot.has(from)).from;
    const walk = (module, path) => {
      for (const { from, to } of arrows) {
        if (from !== module || !knot.has(to)) continue;
        if (to === start) return [...path, to];
        if (path.includes(to)) continue;
        const found = walk(to, [...path, to]);
        if (found) return found;
      }
      return null;
    };
    return walk(start, [start]);
  }

  const api = { cyclesIn, loopThrough };
  if (typeof module === 'object' && module.exports) module.exports = api;
  else root.DiagramKit = Object.assign(root.DiagramKit || {}, api);
})(typeof window !== 'undefined' ? window : globalThis);
