const test = require('node:test');
const assert = require('node:assert/strict');

const { laidOutInLanes } = require('../lib/lanes.js');
const { svgOf } = require('../lib/render.js');
const { SHAPES } = require('../lib/shapes.js');

const SCENE = {
  lanes: [{ id: 'launcher', label: 'Launcher' }, { id: 'store', label: 'Store', status: 'new' }],
  nodes: [
    { id: 'start', shape: 'initial', lane: 'launcher' },
    { id: 'resolve', shape: 'state', label: 'Resolve the version', lane: 'launcher' },
    { id: 'cached', shape: 'choice', lane: 'launcher' },
    { id: 'fetch', shape: 'state', label: 'Fetch and verify', lane: 'store', status: 'new' },
    { id: 'prune', shape: 'state', label: 'Prune stale trees', lane: 'store', tone: 'risk' },
    { id: 'exec', shape: 'state', label: 'Run the sub-binary', lane: 'launcher' },
    { id: 'end', shape: 'final', lane: 'launcher' },
  ],
  edges: [
    { id: 'e1', from: 'start', to: 'resolve', head: 'open' },
    { id: 'e2', from: 'resolve', to: 'cached', head: 'open' },
    { id: 'e3', from: 'cached', to: 'fetch', label: '[not cached]', head: 'open' },
    { id: 'e4', from: 'fetch', to: 'prune', head: 'open' },
    { id: 'e5', from: 'prune', to: 'exec', head: 'open' },
    { id: 'e6', from: 'cached', to: 'exec', label: '[cached]', head: 'open' },
    { id: 'e7', from: 'exec', to: 'end', head: 'open' },
    { id: 'e8', from: 'exec', to: 'resolve', label: '[retry]', head: 'open' },
  ],
};
const scene = laidOutInLanes(SCENE);
const node = id => scene.nodes.find(each => each.id === id);
const edge = id => scene.edges.find(each => each.id === id);
const middle = each => each.x + each.width / 2;
const inside = (inner, outer) => inner.x >= outer.x && inner.x + inner.width <= outer.x + outer.width && inner.y >= outer.y && inner.y + inner.height <= outer.y + outer.height;

test('each lane is a column the height of the diagram, side by side in the order given', () => {
  const [launcher, store] = scene.nodes.filter(each => each.shape === 'lane');
  assert.deepEqual([launcher.id, launcher.label, launcher.inert, launcher.behind], ['lane:launcher', 'Launcher', true, true]);
  assert.equal(store.status, 'new');
  assert.equal(launcher.x + launcher.width, store.x);
  assert.deepEqual([launcher.y, launcher.height], [store.y, store.height]);
  assert.ok(launcher.y + launcher.height <= scene.height && store.x + store.width <= scene.width);
});

test('each node stands inside its own lane, below the lane heading', () => {
  for (const each of SCENE.nodes) {
    const lane = node(`lane:${each.lane}`);
    assert.ok(inside(node(each.id), lane), each.id);
    assert.ok(node(each.id).y >= lane.y + 30, `${each.id} is on the heading`);
  }
});

test('a node is sized as its shape is, and keeps what the scene said about it', () => {
  assert.deepEqual([node('resolve').width, node('resolve').height], Object.values(SHAPES.state.sizeOf(SCENE.nodes[1])));
  assert.deepEqual([node('prune').tone, node('fetch').status, node('fetch').lane], ['risk', 'new', 'store']);
});

test('what follows is drawn below what it follows, across lanes as within them', () => {
  for (const [before, after] of [['start', 'resolve'], ['resolve', 'cached'], ['cached', 'fetch'], ['fetch', 'prune'], ['prune', 'exec'], ['exec', 'end']]) {
    assert.ok(node(before).y + node(before).height < node(after).y, `${after} is not below ${before}`);
  }
});

test('a node alone in its row is centred in its lane', () => {
  assert.equal(middle(node('resolve')), middle(node('lane:launcher')));
  assert.equal(middle(node('fetch')), middle(node('lane:store')));
});

test('every arrow runs at right angles from the foot of its source', () => {
  for (const each of scene.edges) {
    assert.ok(each.points.length >= 2, each.id);
    for (let i = 1; i < each.points.length; i++) {
      const [[x1, y1], [x2, y2]] = [each.points[i - 1], each.points[i]];
      assert.ok(x1 === x2 || y1 === y2, `${each.id} slants`);
    }
  }
  assert.equal(edge('e4').points[0][1], node('fetch').y + node('fetch').height);
  assert.equal(edge('e4').points.at(-1)[1], node('prune').y);
});

test('an arrow meets a disc or a diamond on its outline', () => {
  assert.deepEqual(edge('e1').points[0], [middle(node('start')), node('start').y + node('start').height]);
  assert.deepEqual(edge('e2').points.at(-1), [middle(node('cached')), node('cached').y]);
});

test('an arrow back up the diagram arrives at the top of its target', () => {
  const back = edge('e8');
  assert.equal(back.points.at(-1)[1], node('resolve').y);
  assert.ok(Math.min(...back.points.map(point => point[0])) < node('resolve').x);
});

test('a label has a plate on its arrow, and an arrow without one has none', () => {
  const { labelBox, points } = edge('e3');
  assert.ok(labelBox.width > 60 && labelBox.height >= 18);
  const xs = points.map(point => point[0]), ys = points.map(point => point[1]);
  assert.ok(labelBox.x + labelBox.width / 2 >= Math.min(...xs) && labelBox.x + labelBox.width / 2 <= Math.max(...xs));
  assert.ok(labelBox.y + labelBox.height / 2 >= Math.min(...ys) && labelBox.y + labelBox.height / 2 <= Math.max(...ys));
  assert.equal(edge('e1').labelBox, null);
});

test('everything is on the page', () => {
  for (const each of scene.nodes) assert.ok(each.x >= 0 && each.y >= 0 && each.x + each.width <= scene.width && each.y + each.height <= scene.height, each.id);
  for (const each of scene.edges) for (const [x, y] of each.points) assert.ok(x >= 0 && y >= 0 && x <= scene.width && y <= scene.height, each.id);
});

test('two nodes of one lane in the same row stand side by side', () => {
  const forked = laidOutInLanes({ lanes: [{ id: 'l', label: 'L' }], nodes: [{ id: 'a', shape: 'state', label: 'A', lane: 'l' }, { id: 'b', shape: 'state', label: 'B', lane: 'l' }, { id: 'c', shape: 'state', label: 'C', lane: 'l' }],
    edges: [{ id: 'ab', from: 'a', to: 'b' }, { id: 'ac', from: 'a', to: 'c' }] });
  const at = id => forked.nodes.find(each => each.id === id);
  assert.equal(at('b').y, at('c').y);
  assert.ok(at('b').x + at('b').width < at('c').x);
});

test('the same scene is laid out the same way every time', () => {
  assert.deepEqual(laidOutInLanes(SCENE), scene);
});

test('a lane is drawn as a frame with its name in a head across the top', () => {
  const drawn = svgOf({ width: 300, height: 200, nodes: [{ id: 'lane:store', shape: 'lane', label: 'Store', inert: true, behind: true, status: 'new', x: 10, y: 0, width: 200, height: 180 }], edges: [] }, { uid: 'l' });
  assert.match(drawn, /<g class="node lane s-new" data-part="lane:store"><rect class="body" x="10" y="0" width="200" height="180"\/><line class="rule" x1="10" y1="28" x2="210" y2="28"\/><text class="heading" x="110" y="19" text-anchor="middle">STORE<\/text>/);
});

test('two arrows out of a diamond leave it at different points on its outline', () => {
  const [one, other] = [edge('e3'), edge('e6')].map(each => each.points[0]);
  assert.notEqual(one[0], other[0]);
  const diamond = node('cached'), cx = middle(diamond), cy = diamond.y + diamond.height / 2;
  for (const [x, y] of [one, other]) assert.ok(Math.abs(Math.abs(x - cx) + Math.abs(y - cy) - diamond.width / 2) < 0.02);
});
