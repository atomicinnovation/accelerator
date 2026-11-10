(function (root) {
  'use strict';

  const ARROWHEAD = 9, CLEARANCE = 1, ARROW_SPACE = ARROWHEAD + CLEARANCE;
  const RADIUS = 6, STRAIGHT_END = 8;
  const MARGIN = 28, TIGHT_MARGIN = 20, TRACK = 10, LABELLED_TRACK = 22, MIN_GAP = 62;
  const PORT_STEP = 14, LANE = 10, CLEAR = 12, SNAP = 48, PORT_SEPARATION = 8;

  const centreOf = node => (node.portLeft + node.portRight) / 2;

  function spreadPorts(node, attachments) {
    const sorted = attachments.slice().sort((p, q) => p.other - q.other || p.route - q.route);
    const k = sorted.length;
    const step = k > 1 ? Math.min(PORT_STEP, (node.portRight - node.portLeft) / (k - 1)) : 0;
    sorted.forEach((attachment, i) => { attachment.x = centreOf(node) + (i - (k - 1) / 2) * step; });
  }

  function canMovePort(port, node, side, x) {
    return x >= node.portLeft && x <= node.portRight && side.every(other => other === port
      || (Math.abs(other.x - x) >= PORT_SEPARATION && Math.sign(other.x - x) === Math.sign(other.x - port.x)));
  }

  function lineUp(r, bottom, top) {
    const dx = r.into.x - r.out.x;
    if (!dx || Math.abs(dx) > SNAP) return;
    if (canMovePort(r.into, r.to, top[r.to.id], r.out.x)) r.into.x = r.out.x;
    else if (canMovePort(r.out, r.from, bottom[r.from.id], r.into.x)) r.out.x = r.into.x;
  }

  function chooseLane(x1, x2, blocked) {
    const isFree = x => blocked.every(([lo, hi]) => x <= lo || x >= hi);
    const lo = Math.min(x1, x2), hi = Math.max(x1, x2);
    const candidates = [x1, x2, ...blocked.flat()];
    const cost = x => (x < lo ? lo - x : x > hi ? x - hi : 0) * 10 + (x === x1 ? 0 : x === x2 ? 1 : 2) + Math.abs(x - x1) * 0.001;
    return candidates.filter(isFree).sort((p, q) => cost(p) - cost(q))[0];
  }

  function crossingsBetween(s, ys, t, yt, bottom) {
    const sideY = side => (side === 'top' ? 0 : bottom);
    const pieces = (u, y) => ({
      vertical: [[u.from.x, sideY(u.from.side), y], [u.to.x, y, sideY(u.to.side)]],
      horizontal: [y, Math.min(u.from.x, u.to.x), Math.max(u.from.x, u.to.x)],
    });
    const cross = ([x, y0, y1], [y, x0, x1]) => x > x0 && x < x1 && y > Math.min(y0, y1) && y < Math.max(y0, y1);
    const a = pieces(s, ys), b = pieces(t, yt);
    return a.vertical.filter(v => cross(v, b.horizontal)).length + b.vertical.filter(v => cross(v, a.horizontal)).length;
  }

  function orderTracks(uses) {
    const total = order => {
      const bottom = order.length + 1;
      let sum = 0;
      order.forEach((u, i) => order.slice(i + 1).forEach((w, j) => { sum += crossingsBetween(u, i + 1, w, i + j + 2, bottom); }));
      return sum;
    };
    const byLength = uses.slice().sort((p, q) => Math.abs(q.to.x - q.from.x) - Math.abs(p.to.x - p.from.x));
    return byLength.reduce((order, use) => {
      let best = null;
      for (let at = 0; at <= order.length; at++) {
        const tried = [...order.slice(0, at), use, ...order.slice(at)];
        const score = total(tried);
        if (!best || score < best.score) best = { order: tried, score };
      }
      return best.order;
    }, []);
  }

  function inversions(rows, edges) {
    const where = {};
    rows.forEach((row, r) => row.forEach((id, i) => { where[id] = [r, i]; }));
    const spans = edges.map(e => [where[e.from], where[e.to]]).filter(([a, b]) => a && b && a[0] !== b[0])
      .map(([a, b]) => (a[0] < b[0] ? [a, b] : [b, a]));
    let count = 0;
    spans.forEach(([a, b], i) => spans.slice(i + 1).forEach(([c, d]) => {
      if (a[0] === c[0] && b[0] === d[0] && (a[1] - c[1]) * (b[1] - d[1]) < 0) count++;
    }));
    return count;
  }

  function orderRows(rows, edges, sweeps = 12) {
    const neighbours = {};
    edges.forEach(e => { (neighbours[e.from] = neighbours[e.from] || []).push(e.to); (neighbours[e.to] = neighbours[e.to] || []).push(e.from); });
    let current = rows.map(r => r.slice()), best = current, bestScore = inversions(current, edges);
    const position = order => Object.fromEntries(order.flatMap(row => row.map((id, i) => [id, row.length > 1 ? i / (row.length - 1) : 0.5])));
    for (let sweep = 0; sweep < sweeps && bestScore > 0; sweep++) {
      const downward = sweep % 2 === 0;
      const indices = current.map((_, i) => i);
      (downward ? indices : indices.reverse()).forEach(r => {
        const fixed = (downward ? current.slice(0, r) : current.slice(r + 1)).flat();
        if (!fixed.length) return;
        const at = position(current), inFixed = new Set(fixed);
        const centre = id => {
          const near = (neighbours[id] || []).filter(n => inFixed.has(n));
          return near.length ? near.reduce((sum, n) => sum + at[n], 0) / near.length : at[id];
        };
        current[r] = current[r].slice().sort((p, q) => centre(p) - centre(q) || at[p] - at[q]);
      });
      const score = inversions(current, edges);
      if (score < bestScore) { best = current.map(r => r.slice()); bestScore = score; }
      current = current.map(r => r.slice());
    }
    return best;
  }

  function plan(nodes, edges) {
    const byId = Object.fromEntries(nodes.map(n => [n.id, n]));
    const bottom = {}, top = {};
    const attach = (side, id, attachment) => { (side[id] = side[id] || []).push(attachment); return attachment; };
    const routes = edges.map((edge, i) => {
      const a = byId[edge.from], b = byId[edge.to];
      if (!a || !b) return null;
      const kind = b.rank > a.rank ? 'down' : b.rank === a.rank ? 'across' : 'up';
      const out = attach(bottom, a.id, { route: i, other: kind === 'up' ? -Infinity : centreOf(b) });
      const into = attach(kind === 'across' ? bottom : top, b.id, { route: i, other: kind === 'up' ? -Infinity : centreOf(a) });
      return { edge, kind, from: a, to: b, out, into, labelled: !!edge.label };
    });
    nodes.forEach(n => { if (bottom[n.id]) spreadPorts(n, bottom[n.id]); if (top[n.id]) spreadPorts(n, top[n.id]); });
    routes.filter(r => r && r.kind === 'down' && r.to.rank - r.from.rank === 1).forEach(r => lineUp(r, bottom, top));

    const lanes = [];
    const leftmost = Math.min(...nodes.map(n => n.x));
    routes.filter(r => r && r.kind === 'up')
      .sort((p, q) => (q.from.rank - q.to.rank) - (p.from.rank - p.to.rank))
      .forEach((r, i) => { r.lane = leftmost - CLEAR - i * LANE; });
    routes.filter(r => r && r.kind === 'down' && r.to.rank - r.from.rank > 1).forEach(r => {
      const between = nodes.filter(n => n.rank > r.from.rank && n.rank < r.to.rank);
      const blocked = between.map(n => [n.x - CLEAR, n.x + n.w + CLEAR])
        .concat(lanes.filter(l => l.top < r.to.rank && l.bottom > r.from.rank).map(l => [l.x - LANE, l.x + LANE]));
      r.lane = chooseLane(r.out.x, r.into.x, blocked);
      lanes.push({ x: r.lane, top: r.from.rank, bottom: r.to.rank });
    });

    const gaps = {};
    const use = (r, gap, from, to) => {
      const piece = { route: routes.indexOf(r), gap, from, to, labelled: r.labelled };
      (r.uses = r.uses || []).push(piece);
      if (from.x !== to.x || from.side === to.side) (gaps[gap] = gaps[gap] || { uses: [] }).uses.push(piece);
    };
    routes.forEach(r => {
      if (!r) return;
      const x1 = r.out.x, x2 = r.into.x, below = r.from.rank;
      if (r.kind === 'across') use(r, below, { x: x1, side: 'top' }, { x: x2, side: 'top' });
      else if (r.kind === 'up') {
        use(r, below, { x: x1, side: 'top' }, { x: r.lane, side: 'top' });
        use(r, r.to.rank - 1, { x: r.lane, side: 'bottom' }, { x: x2, side: 'bottom' });
      } else if (r.lane === undefined) use(r, below, { x: x1, side: 'top' }, { x: x2, side: 'bottom' });
      else {
        use(r, below, { x: x1, side: 'top' }, { x: r.lane, side: 'bottom' });
        use(r, r.to.rank - 1, { x: r.lane, side: 'top' }, { x: x2, side: 'bottom' });
      }
    });
    Object.values(gaps).forEach(g => { g.tracks = orderTracks(g.uses); });
    return { routes, gaps };
  }

  const spacingAfter = (tracks, i) => (tracks[i].labelled || tracks[i + 1].labelled ? LABELLED_TRACK : TRACK);
  const trackSpan = tracks => tracks.slice(1).reduce((sum, _, i) => sum + spacingAfter(tracks, i), 0);

  function gapHeight(gap) {
    return Math.max(MIN_GAP, 2 * MARGIN + trackSpan(gap.tracks || []));
  }

  function placeTracks(gap, top, bottom) {
    const span = trackSpan(gap.tracks), room = bottom - top;
    const scale = span > room - 2 * TIGHT_MARGIN && span > 0 ? Math.max(0, room - 2 * TIGHT_MARGIN) / span : 1;
    let y = top + (room - span * scale) / 2;
    gap.tracks.forEach((piece, i) => {
      piece.y = y;
      if (i < gap.tracks.length - 1) y += spacingAfter(gap.tracks, i) * scale;
    });
  }

  function simplify(points) {
    const kept = points.filter((p, i) => i === 0 || p[0] !== points[i - 1][0] || p[1] !== points[i - 1][1]);
    return kept.filter((p, i) => {
      if (i === 0 || i === kept.length - 1) return true;
      const [a, b] = [kept[i - 1], kept[i + 1]];
      return !((a[0] === p[0] && p[0] === b[0]) || (a[1] === p[1] && p[1] === b[1]));
    });
  }

  const round = n => Math.round(n * 100) / 100;

  function roundedPath(points, shortenEnd = ARROW_SPACE) {
    const pts = points.map(p => p.slice());
    const [pa, pb] = [pts[pts.length - 2], pts[pts.length - 1]];
    const lastLength = Math.hypot(pb[0] - pa[0], pb[1] - pa[1]) || 1;
    pb[0] -= (pb[0] - pa[0]) / lastLength * shortenEnd;
    pb[1] -= (pb[1] - pa[1]) / lastLength * shortenEnd;
    let d = `M${round(pts[0][0])} ${round(pts[0][1])}`;
    for (let i = 1; i < pts.length - 1; i++) {
      const [p, c, n] = [pts[i - 1], pts[i], pts[i + 1]];
      const before = Math.hypot(c[0] - p[0], c[1] - p[1]), after = Math.hypot(n[0] - c[0], n[1] - c[1]);
      const room = (length, isEnd) => (isEnd ? length - STRAIGHT_END : length / 2);
      const r = Math.max(0, Math.min(RADIUS, room(before, i === 1), room(after, i === pts.length - 2)));
      const inn = [c[0] - (c[0] - p[0]) / before * r, c[1] - (c[1] - p[1]) / before * r];
      const out = [c[0] + (n[0] - c[0]) / after * r, c[1] + (n[1] - c[1]) / after * r];
      d += ` L ${round(inn[0])} ${round(inn[1])} Q ${round(c[0])} ${round(c[1])}, ${round(out[0])} ${round(out[1])}`;
    }
    const last = pts[pts.length - 1];
    return `${d} L ${round(last[0])} ${round(last[1])}`;
  }

  function labelPoint(points) {
    const segments = points.slice(1).map((p, i) => [points[i], p]);
    const horizontal = segments.filter(([a, b]) => a[1] === b[1]).sort(([a, b], [c, d]) => Math.abs(d[0] - c[0]) - Math.abs(b[0] - a[0]))[0];
    const [a, b] = horizontal || segments[0];
    return [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
  }

  function draw(planned, nodes, { gapTop, gapBottom }) {
    Object.entries(planned.gaps).forEach(([g, gap]) => placeTracks(gap, gapTop(+g), gapBottom(+g)));
    return planned.routes.map(r => {
      if (!r) return null;
      const start = [r.out.x, r.from.y + r.from.h];
      const end = r.kind === 'across' ? [r.into.x, r.to.y + r.to.h] : [r.into.x, r.to.y];
      const middle = r.uses.flatMap(u => (u.y === undefined ? [] : [[u.from.x, u.y], [u.to.x, u.y]]));
      const points = simplify([start, ...middle, end]);
      return { points, d: roundedPath(points), heading: r.kind === 'across' ? [0, -1] : [0, 1], labelAt: labelPoint(points) };
    });
  }

  const GraphRoutes = { orderRows, plan, draw, gapHeight, roundedPath, ARROWHEAD, CLEARANCE, ARROW_SPACE };
  if (typeof module === 'object' && module.exports) module.exports = GraphRoutes;
  else root.GraphRoutes = GraphRoutes;
})(typeof window !== 'undefined' ? window : globalThis);
