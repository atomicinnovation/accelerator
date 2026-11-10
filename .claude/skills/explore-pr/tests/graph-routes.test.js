const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const GraphRoutes = require(path.join(__dirname, '..', 'assets', 'graph-routes.js'));

const ROW_TOP = [0, 100, 200, 300], ROW_HEIGHT = 40;

function box(id, rank, x, w = 100) {
  return { id, rank, x, y: ROW_TOP[rank], w, h: ROW_HEIGHT, portLeft: x + 10, portRight: x + w - 10 };
}

function route(nodes, edges) {
  const plan = GraphRoutes.plan(nodes, edges);
  const gapTop = g => (g < 0 ? -60 : ROW_TOP[g] + ROW_HEIGHT);
  const gapBottom = g => (g + 1 < ROW_TOP.length ? ROW_TOP[g + 1] : gapTop(g) + 60);
  return GraphRoutes.draw(plan, nodes, { gapTop, gapBottom });
}

const segmentsOf = points => points.slice(1).map((p, i) => [points[i], p]);
const inside = ([x, y], b) => x > b.x + 1 && x < b.x + b.w - 1 && y > b.y + 1 && y < b.y + b.h - 1;
const sample = points => segmentsOf(points).flatMap(([a, b]) => {
  const n = Math.max(1, Math.ceil(Math.hypot(b[0] - a[0], b[1] - a[1]) / 2));
  return Array.from({ length: n + 1 }, (_, i) => [a[0] + (b[0] - a[0]) * i / n, a[1] + (b[1] - a[1]) * i / n]);
});

function crossings(routes) {
  let count = 0;
  routes.forEach((r, i) => routes.slice(i + 1).forEach(s => {
    for (const [a, b] of segmentsOf(r.points)) for (const [c, d] of segmentsOf(s.points)) {
      const [h, v] = a[1] === b[1] && c[0] === d[0] ? [[a, b], [c, d]] : c[1] === d[1] && a[0] === b[0] ? [[c, d], [a, b]] : [null, null];
      if (!h) continue;
      const x = v[0][0], y = h[0][1];
      if (x > Math.min(h[0][0], h[1][0]) && x < Math.max(h[0][0], h[1][0]) && y > Math.min(v[0][1], v[1][1]) && y < Math.max(v[0][1], v[1][1])) count++;
    }
  }));
  return count;
}

test('every segment of a route is horizontal or vertical', () => {
  const routes = route([box('a', 0, 0), box('b', 1, 300), box('c', 2, 40)], [{ from: 'a', to: 'b' }, { from: 'b', to: 'c' }, { from: 'a', to: 'c' }]);

  for (const r of routes) for (const [p, q] of segmentsOf(r.points)) assert.ok(p[0] === q[0] || p[1] === q[1], JSON.stringify(r.points));
});

test('an edge between boxes stacked above each other is one vertical line', () => {
  const [r] = route([box('a', 0, 0), box('b', 1, 0)], [{ from: 'a', to: 'b' }]);

  assert.equal(r.points.length, 2);
  assert.equal(r.points[0][0], r.points[1][0]);
});

test('an edge leaves the bottom of its source and arrives on the top of its target, pointing down', () => {
  const [r] = route([box('a', 0, 0), box('b', 1, 300)], [{ from: 'a', to: 'b' }]);

  assert.equal(r.points[0][1], ROW_HEIGHT);
  assert.equal(r.points.at(-1)[1], ROW_TOP[1]);
  assert.deepEqual(r.heading, [0, 1]);
});

test('an edge that skips a row goes around the box in its way', () => {
  const nodes = [box('a', 0, 0), box('blocker', 1, -20, 160), box('c', 2, 0)];
  const [r] = route(nodes, [{ from: 'a', to: 'c' }]);

  assert.ok(sample(r.points).every(p => !inside(p, nodes[1])), JSON.stringify(r.points));
});

test('edges leaving one box leave from separate points, ordered so they do not cross', () => {
  const routes = route([box('a', 0, 100), box('left', 1, 0), box('right', 1, 300)], [{ from: 'a', to: 'right' }, { from: 'a', to: 'left' }]);

  assert.notEqual(routes[0].points[0][0], routes[1].points[0][0]);
  assert.equal(crossings(routes), 0);
});

test('edges sharing a gap run along separate tracks and do not cross when they need not', () => {
  const nodes = [box('a', 0, 0), box('b', 0, 150), box('c', 1, 200), box('d', 1, 350)];
  const routes = route(nodes, [{ from: 'a', to: 'c' }, { from: 'b', to: 'd' }]);
  const tracks = routes.map(r => r.points[1][1]);

  assert.notEqual(tracks[0], tracks[1]);
  assert.equal(crossings(routes), 0);
});

test('an edge to a box in its own row arrives from below, pointing up', () => {
  const [r] = route([box('a', 0, 0), box('b', 0, 200)], [{ from: 'a', to: 'b' }]);

  assert.equal(r.points.at(-1)[1], ROW_HEIGHT);
  assert.deepEqual(r.heading, [0, -1]);
});

test('an edge back up the diagram runs left of every box and arrives on top of its target', () => {
  const nodes = [box('a', 0, 0), box('b', 1, 0)];
  const [r] = route(nodes, [{ from: 'b', to: 'a' }]);

  assert.ok(Math.min(...r.points.map(p => p[0])) < 0);
  assert.equal(r.points.at(-1)[1], ROW_TOP[0]);
  assert.deepEqual(r.heading, [0, 1]);
  assert.ok(sample(r.points).every(p => nodes.every(b => !inside(p, b))));
});

test('a gap is asked for as many tracks as the edges crossing it need', () => {
  const plan = GraphRoutes.plan([box('a', 0, 0), box('b', 0, 150), box('c', 1, 200), box('d', 1, 350)], [{ from: 'a', to: 'c' }, { from: 'b', to: 'd' }]);

  assert.equal(plan.gaps[0].tracks.length, 2);
  assert.ok(GraphRoutes.gapHeight(plan.gaps[0]) > GraphRoutes.gapHeight({ tracks: [] }));
});

test('a drawn route rounds its corners and stops short of its target for the arrowhead', () => {
  const [r] = route([box('a', 0, 0), box('b', 1, 300)], [{ from: 'a', to: 'b' }]);
  const numbers = r.d.match(/-?[0-9.]+/g).map(Number);

  assert.match(r.d, /Q/);
  assert.equal(numbers.at(-1), ROW_TOP[1] - GraphRoutes.ARROW_SPACE);
});

test('an edge whose ends nearly line up is drawn as one straight line', () => {
  const [r] = route([box('a', 0, 0), box('b', 1, 20)], [{ from: 'a', to: 'b' }]);

  assert.equal(r.points.length, 2);
});

test('lining up an edge keeps the other arrows on that side apart and in order', () => {
  const nodes = [box('far', 0, -300), box('near', 0, 110), box('target', 1, 100)];
  const routes = route(nodes, [{ from: 'far', to: 'target' }, { from: 'near', to: 'target' }]);
  const [farIn, nearIn] = routes.map(r => r.points.at(-1)[0]);

  assert.equal(routes[1].points.length, 2);
  assert.ok(nearIn - farIn >= 8);
});

test('rows the page may arrange are reordered so arrows between them do not cross', () => {
  const edges = [{ from: 'a', to: 'd' }, { from: 'b', to: 'c' }, { from: 'c', to: 'f' }, { from: 'd', to: 'e' }];
  const rows = GraphRoutes.orderRows([['a', 'b'], ['c', 'd'], ['e', 'f']], edges);
  const at = id => rows.findIndex(r => r.includes(id)) * 10 + rows.find(r => r.includes(id)).indexOf(id);
  const inverted = edges.filter((e, i) => edges.slice(i + 1).some(f =>
    Math.floor(at(e.from) / 10) === Math.floor(at(f.from) / 10) && Math.sign(at(e.from) - at(f.from)) === -Math.sign(at(e.to) - at(f.to))));

  assert.deepEqual(rows.map(r => r.slice().sort()), [['a', 'b'], ['c', 'd'], ['e', 'f']]);
  assert.equal(inverted.length, 0);
});
