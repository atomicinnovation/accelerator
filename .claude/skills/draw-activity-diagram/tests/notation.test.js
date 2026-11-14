const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const notation = require('../notation.js');
const { kitWith } = require('../../diagram-kit/lib/index.js');

const SPEC = {
  id: 'cold-start', kind: 'activity-diagram', title: 'A cold start fetches, then prunes',
  nodes: [
    { id: 'start', kind: 'initial' },
    { id: 'resolve', label: 'Resolve the version', link: 'src/resolve.rs:10' },
    { id: 'cached', kind: 'decision' },
    { id: 'fetch', label: 'Fetch and verify', status: 'new' },
    { id: 'split', kind: 'fork' },
    { id: 'prune', label: 'Prune stale trees', tone: 'risk' },
    { id: 'log', label: 'Record the fetch' },
    { id: 'gather', kind: 'join' },
    { id: 'ready', kind: 'merge' },
    { id: 'exec', label: 'Run the sub-binary' },
    { id: 'end', kind: 'final' },
  ],
  flows: [
    { from: 'start', to: 'resolve' }, { from: 'resolve', to: 'cached' },
    { from: 'cached', to: 'ready', guard: 'cached' }, { from: 'cached', to: 'fetch', guard: 'else', status: 'new' },
    { from: 'fetch', to: 'split' }, { from: 'split', to: 'prune' }, { from: 'split', to: 'log' },
    { from: 'prune', to: 'gather' }, { from: 'log', to: 'gather' }, { from: 'gather', to: 'ready' },
    { from: 'ready', to: 'exec' }, { from: 'exec', to: 'end', label: 'exit code', link: 'src/exec.rs:5' },
  ],
};
const scene = notation.sceneOf(SPEC);
const node = id => scene.nodes.find(each => each.id === id);
const edge = id => scene.edges.find(each => each.id === id);
const problems = change => notation.problemsIn({ ...SPEC, ...change });
const remarks = change => notation.remarksOn({ ...SPEC, ...change });
const LANED = { ...SPEC, partitions: [{ id: 'launcher', label: 'Launcher' }, { id: 'store', label: 'Store', status: 'new' }],
  nodes: SPEC.nodes.map(each => ({ ...each, partition: ['fetch', 'split', 'prune', 'log', 'gather'].includes(each.id) ? 'store' : 'launcher' })) };

test('it draws diagrams of kind activity-diagram', () => {
  assert.equal(notation.kind, 'activity-diagram');
});

test('a sound activity has no problems and nothing to remark on', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
  assert.deepEqual(notation.remarksOn(SPEC), []);
  assert.deepEqual(notation.problemsIn(LANED), []);
});

test('each kind of node has its own shape', () => {
  assert.deepEqual(['start', 'resolve', 'cached', 'split', 'gather', 'ready', 'end'].map(id => node(id).shape), ['initial', 'state', 'choice', 'bar', 'bar', 'choice', 'final']);
  assert.equal(notation.sceneOf({ ...SPEC, nodes: [{ id: 'a', label: 'A' }, { id: 'stop', kind: 'flow final' }], flows: [{ from: 'a', to: 'stop' }] }).nodes[1].shape, 'flowfinal');
});

test('an action is named by its label and keeps its marks and link', () => {
  assert.deepEqual([node('resolve').label, node('resolve').link, node('fetch').status, node('prune').tone], ['Resolve the version', 'src/resolve.rs:10', 'new', 'risk']);
});

test('a flow is a solid arrow with an open head, labelled by its guard and what it carries', () => {
  assert.deepEqual([edge('cached->fetch').line, edge('cached->fetch').head, edge('cached->fetch').label, edge('cached->fetch').status], ['solid', 'open', '[else]', 'new']);
  assert.deepEqual([edge('exec->end').label, edge('exec->end').link], ['exit code', 'src/exec.rs:5']);
  assert.equal(edge('start->resolve').label, undefined);
  const both = notation.sceneOf({ ...SPEC, flows: [...SPEC.flows, { from: 'exec', to: 'end', guard: 'failed', label: 'error' }] });
  assert.deepEqual(both.edges.slice(-1).map(each => [each.id, each.label]), [['exec->end:2', '[failed] error']]);
});

test('a bar stands upright when the activity runs to the right', () => {
  assert.equal(node('split').upright, undefined);
  assert.equal(notation.sceneOf({ ...SPEC, direction: 'right' }).nodes.find(each => each.id === 'split').upright, true);
});

test('an activity without partitions is left to the layout engine', () => {
  assert.equal(notation.placedSceneOf(SPEC), null);
});

test('an activity with partitions is laid out in lanes, each node in its own', () => {
  const placed = notation.placedSceneOf(LANED);
  const at = id => placed.nodes.find(each => each.id === id);
  assert.deepEqual(placed.nodes.filter(each => each.shape === 'lane').map(each => [each.id, each.label, each.status]), [['lane:launcher', 'Launcher', undefined], ['lane:store', 'Store', 'new']]);
  assert.ok(at('fetch').x >= at('lane:store').x && at('resolve').x + at('resolve').width <= at('lane:store').x);
  assert.equal(placed.edges.length, SPEC.flows.length);
});

test('an activity needs nodes, each with an id of its own and a kind the notation draws', () => {
  assert.deepEqual(problems({ nodes: [] }), ['no nodes']);
  assert.deepEqual(problems({ nodes: [{ label: 'x' }, { id: 'a', label: 'A' }, { id: 'a', label: 'A' }, { id: 'b', kind: 'gateway' }, { id: 'c' }], flows: [] }), [
    'node 1: no id',
    'node a: id is used twice',
    'node b: kind "gateway" is not one of action, initial, final, flow final, decision, merge, fork, join',
    'node c: an action needs a label',
  ]);
});

test('with partitions, every node is in one the activity has, and the activity runs down', () => {
  const partitions = [{ id: 'p', label: 'P' }, { id: 'p', label: 'Again' }, { label: 'nameless' }];
  assert.deepEqual(problems({ partitions, direction: 'right', nodes: [{ id: 'a', label: 'A', partition: 'p' }, { id: 'b', label: 'B' }, { id: 'c', label: 'C', partition: 'q' }], flows: [] }), [
    'partitions are drawn as lanes down the page; leave direction out',
    'partition p: id is used twice',
    'partition 3: no id',
    'node b: no partition, and the activity has partitions',
    'node c: no partition "q"',
  ]);
  assert.deepEqual(problems({ nodes: [{ id: 'a', label: 'A', partition: 'p' }], flows: [] }), ['node a: no partition "p"']);
});

test('a flow runs between nodes the activity has, never out of an end or into the start', () => {
  assert.deepEqual(problems({ flows: [...SPEC.flows, { from: 'exec', to: 'nowhere' }, { to: 'exec' }, { from: 'end', to: 'exec' }, { from: 'exec', to: 'start' }, { from: 'exec', to: 'resolve', guard: 7, tone: 'scary' }] }), [
    'flow exec->nowhere: no node "nowhere"',
    'flow 14: no from',
    'flow end->exec: nothing flows out of a final node',
    'flow exec->start: nothing flows into the initial node',
    'flow exec->resolve: tone "scary" is not one of risk, ok',
    'flow exec->resolve: guard must be text',
  ]);
});

test('control nodes have the flows in and out that their kind needs', () => {
  const nodes = [{ id: 's', kind: 'initial' }, { id: 'a', label: 'A' }, { id: 'd', kind: 'decision' }, { id: 'm', kind: 'merge' }, { id: 'f', kind: 'fork' }, { id: 'j', kind: 'join' }];
  assert.deepEqual(problems({ nodes, flows: [{ from: 'a', to: 'd' }, { from: 'd', to: 'm' }, { from: 'm', to: 'f' }, { from: 'f', to: 'j' }, { from: 'j', to: 'a' }] }), [
    'node s: the initial node needs a flow out',
    'node d: a decision needs at least two flows out, and has 1',
    'node m: a merge needs at least two flows in, and has 1',
    'node f: a fork needs at least two flows out, and has 1',
    'node j: a join needs at least two flows in, and has 1',
  ]);
});

test('an activity with no start says so', () => {
  assert.deepEqual(remarks({ nodes: [{ id: 'a', label: 'A' }, { id: 'b', label: 'B' }], flows: [{ from: 'a', to: 'b' }, { from: 'b', to: 'a' }] }), ['Nothing says where the activity starts: it has no initial node']);
});

test('a node nothing flows to is remarked on', () => {
  assert.deepEqual(remarks({ nodes: [...SPEC.nodes, { id: 'lost', label: 'Clean up' }], flows: [...SPEC.flows, { from: 'lost', to: 'end' }] }), ['Clean up cannot be reached']);
});

test('an action nothing flows out of is remarked on', () => {
  assert.deepEqual(remarks({ flows: SPEC.flows.filter(each => each.from !== 'exec') }), ['Run the sub-binary has no flow out, and the activity does not end there', 'the final node cannot be reached']);
});

test('a decision with more than one flow out without a guard is remarked on', () => {
  assert.deepEqual(remarks({ flows: SPEC.flows.map(each => (each.from === 'cached' ? { ...each, guard: undefined } : each)) }), ['The decision after Resolve the version has 2 flows out with no guard to choose between them']);
});

const EXAMPLES = path.join(__dirname, '..', 'examples');
for (const file of fs.readdirSync(EXAMPLES).filter(name => name.endsWith('.json'))) {
  test(`the example ${file} is sound and lays out`, async () => {
    const kit = kitWith(), example = JSON.parse(fs.readFileSync(path.join(EXAMPLES, file), 'utf8'));
    assert.deepEqual(kit.problemsIn(example), []);
    assert.deepEqual(kit.remarksOn(example), []);
    assert.equal((await kit.laidOut(example)).edges.every(each => each.points.length >= 2), true);
  });
}
