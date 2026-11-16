const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const notation = require('../notation.js');
const { kitWith } = require('../../diagram-kit/lib/index.js');

const SPEC = {
  id: 'prune-one', kind: 'flowchart', title: 'A tree is removed only when no claim covers it',
  steps: [
    { id: 'start', kind: 'start', label: 'For each tree' },
    { id: 'read', kind: 'input', label: 'Read its claims', link: 'src/prune.rs:40' },
    { id: 'claimed', kind: 'decision', label: 'Any claim younger than the window?' },
    { id: 'keep', label: 'Keep the tree', tone: 'ok' },
    { id: 'remove', kind: 'subprocess', label: 'Remove the tree', status: 'new', tone: 'risk' },
    { id: 'report', kind: 'output', label: 'Report what was done' },
    { id: 'end', kind: 'end', label: 'Next tree' },
  ],
  flows: [
    { from: 'start', to: 'read' }, { from: 'read', to: 'claimed' },
    { from: 'claimed', to: 'keep', label: 'yes' }, { from: 'claimed', to: 'remove', label: 'no', status: 'new' },
    { from: 'keep', to: 'report' }, { from: 'remove', to: 'report', link: 'src/prune.rs:90' }, { from: 'report', to: 'end' },
  ],
};
const scene = notation.sceneOf(SPEC);
const node = id => scene.nodes.find(each => each.id === id);
const edge = id => scene.edges.find(each => each.id === id);
const problems = change => notation.problemsIn({ ...SPEC, ...change });
const remarks = change => notation.remarksOn({ ...SPEC, ...change });

test('it draws diagrams of kind flowchart', () => {
  assert.equal(notation.kind, 'flowchart');
});

test('a sound flowchart has no problems and nothing to remark on', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
  assert.deepEqual(notation.remarksOn(SPEC), []);
});

test('each kind of step has its own symbol', () => {
  assert.deepEqual(SPEC.steps.map(step => node(step.id).shape), ['terminator', 'io', 'decision', 'process', 'subprocess', 'io', 'terminator']);
});

test('a step says what it does and keeps its marks and link', () => {
  assert.deepEqual([node('read').label, node('read').link, node('remove').status, node('remove').tone, node('keep').tone], ['Read its claims', 'src/prune.rs:40', 'new', 'risk', 'ok']);
});

test('a flow is a solid arrow with a filled head, labelled when it leaves a decision', () => {
  assert.deepEqual([edge('claimed->remove').line, edge('claimed->remove').head, edge('claimed->remove').label, edge('claimed->remove').status], ['solid', 'filled', 'no', 'new']);
  assert.deepEqual([edge('start->read').label, edge('remove->report').link], [undefined, 'src/prune.rs:90']);
});

test('a flowchart without lanes is left to the layout engine, and one with lanes is laid out in them', () => {
  assert.equal(notation.placedSceneOf(SPEC), null);
  const laned = { ...SPEC, lanes: [{ id: 'prune', label: 'Prune' }, { id: 'disk', label: 'Disk' }], steps: SPEC.steps.map(step => ({ ...step, lane: ['read', 'remove'].includes(step.id) ? 'disk' : 'prune' })) };
  assert.deepEqual(notation.problemsIn(laned), []);
  const placed = notation.placedSceneOf(laned);
  assert.deepEqual(placed.nodes.filter(each => each.shape === 'lane').map(each => each.label), ['Prune', 'Disk']);
  assert.ok(placed.nodes.find(each => each.id === 'remove').x >= placed.nodes.find(each => each.id === 'lane:disk').x);
});

test('a flowchart needs steps, each with an id of its own, a kind the notation draws and something to say', () => {
  assert.deepEqual(problems({ steps: [] }), ['no steps']);
  assert.deepEqual(problems({ steps: [{ label: 'x' }, { id: 'a', label: 'A' }, { id: 'a', label: 'A' }, { id: 'b', kind: 'loop', label: 'B' }, { id: 'c' }], flows: [] }), [
    'step 1: no id',
    'step a: id is used twice',
    'step b: kind "loop" is not one of process, start, end, decision, input, output, subprocess',
    'step c: no label',
  ]);
});

test('with lanes, every step is in one the flowchart has, and the flowchart runs down', () => {
  assert.deepEqual(problems({ lanes: [{ id: 'p', label: 'P' }], direction: 'right', steps: [{ id: 'a', label: 'A', lane: 'p' }, { id: 'b', label: 'B' }, { id: 'c', label: 'C', lane: 'q' }], flows: [] }), [
    'lanes are drawn down the page; leave direction out',
    'step b: no lane, and the flowchart has lanes',
    'step c: no lane "q"',
  ]);
});

test('a flow runs between steps the flowchart has, never out of an end or into a start', () => {
  assert.deepEqual(problems({ flows: [...SPEC.flows, { from: 'keep', to: 'nowhere' }, { to: 'keep' }, { from: 'end', to: 'keep' }, { from: 'keep', to: 'start' }, { from: 'keep', to: 'read', label: 3, tone: 'scary' }] }), [
    'flow keep->nowhere: no step "nowhere"',
    'flow 9: no from',
    'flow end->keep: nothing flows out of an end',
    'flow keep->start: nothing flows into a start',
    'flow keep->read: tone "scary" is not one of risk, ok',
    'flow keep->read: label must be text',
  ]);
});

test('a start leads somewhere, and a decision has at least two ways out', () => {
  assert.deepEqual(problems({ steps: [{ id: 's', kind: 'start', label: 'Start' }, { id: 'a', label: 'A' }, { id: 'd', kind: 'decision', label: 'Ready?' }], flows: [{ from: 'a', to: 'd' }, { from: 'd', to: 'a', label: 'no' }] }), [
    'step s: a start needs a flow out',
    'step d: a decision needs at least two flows out, and has 1',
  ]);
});

test('a flowchart with no start says so', () => {
  assert.deepEqual(remarks({ steps: [{ id: 'a', label: 'A' }, { id: 'b', label: 'B' }], flows: [{ from: 'a', to: 'b' }, { from: 'b', to: 'a' }] }), ['Nothing says where the flowchart starts: it has no start']);
});

test('a step nothing flows to, and a step that leads nowhere, are remarked on', () => {
  assert.deepEqual(remarks({ steps: [...SPEC.steps, { id: 'lost', label: 'Tidy up' }], flows: [...SPEC.flows, { from: 'lost', to: 'end' }] }), ['Tidy up cannot be reached']);
  assert.deepEqual(remarks({ flows: SPEC.flows.filter(flow => flow.from !== 'keep') }), ['Keep the tree leads nowhere, and is not an end']);
});

test('a decision whose ways out are not all labelled, or share a label, is remarked on', () => {
  const unlabelled = SPEC.flows.map(flow => (flow.to === 'keep' ? { ...flow, label: undefined } : flow));
  assert.deepEqual(remarks({ flows: unlabelled }), ['"Any claim younger than the window?" has a flow out with no answer on it']);
  const same = SPEC.flows.map(flow => (flow.from === 'claimed' ? { ...flow, label: 'yes' } : flow));
  assert.deepEqual(remarks({ flows: same }), ['"Any claim younger than the window?" has two flows out both labelled yes']);
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
