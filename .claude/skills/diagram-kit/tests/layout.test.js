const test = require('node:test');
const assert = require('node:assert/strict');

const { layoutGraphOf, positionedFrom, laidOut } = require('../lib/layout.js');
const { widthOf } = require('../lib/text.js');

const SCENE = {
  direction: 'down',
  nodes: [
    { id: 'cli', shape: 'box', label: 'cli::main' },
    {
      id: 'launch', shape: 'group', label: 'launch', status: 'changed',
      children: [
        { id: 'start_up', shape: 'box', label: 'start_up', link: 'src/start_up.rs:12' },
        { id: 'retention', shape: 'group', label: 'retention', children: [{ id: 'prune', shape: 'box', label: 'prune', detail: 'removes stale trees', tone: 'risk' }] },
      ],
    },
  ],
  edges: [
    { id: 'cli->start_up', from: 'cli', to: 'start_up', label: 'runs', head: 'open', line: 'dashed' },
    { id: 'start_up->prune', from: 'start_up', to: 'prune', status: 'new' },
  ],
};
const graph = layoutGraphOf(SCENE);
const child = (parent, id) => parent.children.find(each => each.id === id);

test('a scene lays out downwards or rightwards, with orthogonal edges across groups', () => {
  assert.equal(graph.layoutOptions['elk.direction'], 'DOWN');
  assert.equal(layoutGraphOf({ ...SCENE, direction: 'right' }).layoutOptions['elk.direction'], 'RIGHT');
  assert.equal(graph.layoutOptions['elk.edgeRouting'], 'ORTHOGONAL');
  assert.equal(graph.layoutOptions['elk.hierarchyHandling'], 'INCLUDE_CHILDREN');
});

test('a box is sized to hold its label', () => {
  const cli = child(graph, 'cli');
  assert.ok(cli.width >= widthOf('cli::main', 'name') + 20);
  assert.ok(cli.height >= 30);
  assert.equal(cli.children, undefined);
});

test('a box with a detail is taller than one without', () => {
  const retention = child(child(graph, 'launch'), 'retention');
  assert.ok(child(retention, 'prune').height > child(graph, 'cli').height);
});

test('a linked box leaves room for the control that opens its link', () => {
  const linked = layoutGraphOf({ nodes: [{ id: 'a', shape: 'box', label: 'start_up', link: 'x' }], edges: [] }).children[0];
  const plain = layoutGraphOf({ nodes: [{ id: 'a', shape: 'box', label: 'start_up' }], edges: [] }).children[0];
  assert.equal(linked.width - plain.width, 24);
});

test('a group takes its size from what it holds, with room for its title', () => {
  const launch = child(graph, 'launch');
  assert.equal(launch.width, undefined);
  assert.deepEqual(launch.children.map(each => each.id), ['start_up', 'retention']);
  assert.match(launch.layoutOptions['elk.padding'], /top=3\d/);
});

test('an edge joins its ends and reserves room for its label', () => {
  const [labelled, bare] = graph.edges;
  assert.deepEqual([labelled.sources, labelled.targets], [['cli'], ['start_up']]);
  assert.ok(labelled.labels[0].width >= widthOf('runs', 'edge') + 10);
  assert.deepEqual(bare.labels, []);
});

test('an unknown shape is refused by name', () => {
  assert.throws(() => layoutGraphOf({ nodes: [{ id: 'a', shape: 'blob' }], edges: [] }), /node a: no shape "blob"/);
});

const LAID = {
  id: 'root', x: 0, y: 0, width: 300, height: 400, $H: 3,
  children: [
    { id: 'cli', x: 100, y: 12, width: 100, height: 32, $H: 4 },
    { id: 'launch', x: 20, y: 90, width: 260, height: 290, children: [
      { id: 'start_up', x: 14, y: 34, width: 120, height: 32 },
      { id: 'retention', x: 14, y: 120, width: 200, height: 150, children: [{ id: 'prune', x: 14, y: 34, width: 150, height: 46 }] },
    ] },
  ],
  edges: [
    { id: 'cli->start_up', container: 'root', sections: [{ startPoint: { x: 150, y: 44 }, bendPoints: [{ x: 150, y: 70 }, { x: 94, y: 70 }], endPoint: { x: 94, y: 124 } }], labels: [{ x: 154, y: 50, width: 34, height: 18 }] },
    { id: 'start_up->prune', container: 'launch', sections: [{ startPoint: { x: 74, y: 66 }, endPoint: { x: 74, y: 154 } }], labels: [] },
  ],
};
const placed = positionedFrom(LAID, SCENE);
const node = id => placed.nodes.find(each => each.id === id);
const edge = id => placed.edges.find(each => each.id === id);

test('a positioned scene is as large as its layout', () => {
  assert.deepEqual([placed.width, placed.height], [300, 400]);
});

test('every node is placed on the page, however deeply it is nested', () => {
  assert.deepEqual([node('cli').x, node('cli').y], [100, 12]);
  assert.deepEqual([node('start_up').x, node('start_up').y], [34, 124]);
  assert.deepEqual([node('prune').x, node('prune').y, node('prune').width, node('prune').height], [48, 244, 150, 46]);
});

test('a positioned node keeps what the scene said about it', () => {
  assert.equal(node('prune').tone, 'risk');
  assert.equal(node('prune').detail, 'removes stale trees');
  assert.equal(node('launch').status, 'changed');
  assert.equal(node('start_up').link, 'src/start_up.rs:12');
});

test('a positioned node knows its parent and its depth, and groups come before what they hold', () => {
  assert.deepEqual(placed.nodes.map(each => [each.id, each.parent, each.depth]), [
    ['cli', null, 0], ['launch', null, 0], ['start_up', 'launch', 1], ['retention', 'launch', 1], ['prune', 'retention', 2],
  ]);
  assert.equal(node('launch').children, undefined);
});

test('an edge is placed on the page from wherever its layout measured it', () => {
  assert.deepEqual(edge('cli->start_up').points, [[150, 44], [150, 70], [94, 70], [94, 124]]);
  assert.deepEqual(edge('start_up->prune').points, [[94, 156], [94, 244]]);
});

test('an edge keeps its label, where the layout put it', () => {
  assert.deepEqual(edge('cli->start_up').labelBox, { x: 154, y: 50, width: 34, height: 18 });
  assert.equal(edge('cli->start_up').label, 'runs');
  assert.equal(edge('start_up->prune').labelBox, null);
  assert.equal(edge('start_up->prune').status, 'new');
});

test('nothing of the layout engine is left in a positioned scene', () => {
  assert.doesNotMatch(JSON.stringify(placed), /\$H|sections|layoutOptions/);
});

test('the engine lays a nested scene out the same way every time', async () => {
  const [once, again] = [await laidOut(SCENE), await laidOut(SCENE)];
  assert.deepEqual(once, again);
});

test('the engine keeps each node inside its group and every edge at right angles', async () => {
  const scene = await laidOut(SCENE);
  const at = id => scene.nodes.find(each => each.id === id);
  const inside = (inner, outer) => inner.x >= outer.x && inner.y >= outer.y && inner.x + inner.width <= outer.x + outer.width && inner.y + inner.height <= outer.y + outer.height;
  assert.ok(inside(at('prune'), at('retention')));
  assert.ok(inside(at('retention'), at('launch')));
  assert.ok(!inside(at('cli'), at('launch')));
  for (const each of scene.edges) {
    assert.ok(each.points.length >= 2, each.id);
    for (let i = 1; i < each.points.length; i++) {
      const [[x1, y1], [x2, y2]] = [each.points[i - 1], each.points[i]];
      assert.ok(x1 === x2 || y1 === y2, `${each.id} slants`);
    }
  }
});

test('an edge to a node the scene does not have is refused before layout', async () => {
  const broken = { nodes: [{ id: 'a', shape: 'box', label: 'a' }], edges: [{ id: 'x', from: 'a', to: 'nowhere' }] };
  await assert.rejects(laidOut(broken), /edge x: no node "nowhere"/);
});

for (const direction of ['down', 'right']) {
  test(`a group laid out ${direction}wards is wide enough for its title and no taller than what it holds`, async () => {
    const titled = { direction, nodes: [{ id: 'g', shape: 'group', label: 'a group with a long title to fit', children: [{ id: 'a', shape: 'box', label: 'a' }] }, { id: 'b', shape: 'box', label: 'b' }], edges: [{ id: 'a->b', from: 'a', to: 'b' }] };
    const scene = await laidOut(titled);
    const group = scene.nodes.find(each => each.id === 'g'), held = scene.nodes.find(each => each.id === 'a');
    assert.ok(group.width >= widthOf('A GROUP WITH A LONG TITLE TO FIT', 'heading') + 28, `only ${group.width} wide`);
    assert.equal(group.height, 34 + held.height + 14);
  });
}

const bent = (points, to = 'b') => {
  const [first, ...rest] = points.map(([x, y]) => ({ x, y }));
  const laid = { id: 'root', width: 200, height: 200, children: [{ id: 'a', x: 0, y: 100, width: 40, height: 20 }, { id: 'b', x: 20, y: 0, width: 40, height: 34 }],
    edges: [{ id: 'e', container: 'root', sections: [{ startPoint: first, bendPoints: rest.slice(0, -1), endPoint: rest.at(-1) }] }] };
  const scene = { nodes: [{ id: 'a', shape: 'box', label: 'a' }, { id: 'b', shape: 'box', label: 'b' }], edges: [{ id: 'e', from: 'a', to }] };
  return positionedFrom(laid, scene).edges[0].points;
};

test('an arrow that turns just before it arrives is given a straight run for its head', () => {
  assert.deepEqual(bent([[10, 100], [10, 40], [30, 40], [30, 34]]), [[10, 100], [10, 50], [30, 50], [30, 34]]);
  assert.deepEqual(bent([[100, 60], [66, 60], [66, 20], [60, 20]]), [[100, 60], [76, 60], [76, 20], [60, 20]]);
});

test('an arrow with a long enough last run is left as the engine routed it', () => {
  const routed = [[10, 100], [10, 60], [30, 60], [30, 34]];
  assert.deepEqual(bent(routed), routed);
});

test('an arrow whose earlier run cannot spare the room is left as the engine routed it', () => {
  const tight = [[10, 46], [10, 40], [30, 40], [30, 34]];
  assert.deepEqual(bent(tight), tight);
});
