const test = require('node:test');
const assert = require('node:assert/strict');

const notation = require('../notation.js');

const SPEC = {
  id: 'deps', kind: 'dependency-graph', title: 'What launch depends on',
  modules: [
    { id: 'cli', label: 'cli::main' },
    { id: 'launch', status: 'changed', modules: [
      { id: 'start_up', link: 'src/start_up.rs:12' },
      { id: 'retention', modules: [{ id: 'claims', label: 'version_claims', status: 'new' }, { id: 'prune', detail: 'removes stale trees' }] },
    ] },
    { id: 'clock', label: 'SystemClock' },
  ],
  dependencies: [
    { from: 'cli', to: 'start_up' },
    { from: 'start_up', to: 'claims', kind: 'runtime', status: 'new' },
    { from: 'claims', to: 'prune' },
    { from: 'prune', to: 'claims', label: 'reads back' },
    { from: 'claims', to: 'clock', kind: 'test', tone: 'ok' },
  ],
};
const scene = notation.sceneOf(SPEC);
const edge = id => scene.edges.find(each => each.id === id);
const problems = change => notation.problemsIn({ ...SPEC, ...change });

test('it draws diagrams of kind dependency-graph', () => {
  assert.equal(notation.kind, 'dependency-graph');
});

test('a sound graph has no problems', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
});

test('a module that holds no others is a box, named by its label or its id', () => {
  const [cli] = scene.nodes;
  assert.deepEqual([cli.shape, cli.label], ['box', 'cli::main']);
  assert.equal(scene.nodes[1].children[0].label, 'start_up');
});

test('a module that holds others is a group, to any depth', () => {
  const launch = scene.nodes[1];
  assert.equal(launch.shape, 'group');
  assert.deepEqual(launch.children.map(each => [each.id, each.shape]), [['start_up', 'box'], ['retention', 'group']]);
  assert.deepEqual(launch.children[1].children.map(each => each.id), ['claims', 'prune']);
});

test('a module keeps its status, tone, link and detail', () => {
  const launch = scene.nodes[1];
  assert.equal(launch.status, 'changed');
  assert.equal(launch.children[0].link, 'src/start_up.rs:12');
  assert.equal(launch.children[1].children[1].detail, 'removes stale trees');
});

test('a dependency is a dashed arrow with an open head, from the dependent to what it needs', () => {
  const drawn = edge('cli->start_up');
  assert.deepEqual([drawn.from, drawn.to, drawn.line, drawn.head], ['cli', 'start_up', 'dashed', 'open']);
});

test('a dependency is labelled by its label, or else by its kind', () => {
  assert.equal(edge('start_up->claims').label, 'runtime');
  assert.equal(edge('prune->claims').label, 'reads back');
  assert.equal(edge('cli->start_up').label, undefined);
});

test('a dependency keeps its status and tone', () => {
  assert.equal(edge('start_up->claims').status, 'new');
  assert.equal(edge('claims->clock').tone, 'ok');
});

test('dependencies that form a cycle are drawn as a risk', () => {
  assert.equal(edge('claims->prune').tone, 'risk');
  assert.equal(edge('prune->claims').tone, 'risk');
  assert.equal(edge('cli->start_up').tone, undefined);
});

test('each cycle is remarked on, by the modules it runs through', () => {
  assert.deepEqual(notation.remarksOn(SPEC), ['Cycle: version_claims → prune → version_claims']);
});

test('a module that depends on itself is a cycle of one', () => {
  const looped = { ...SPEC, dependencies: [{ from: 'cli', to: 'cli' }] };
  assert.equal(notation.sceneOf(looped).edges[0].tone, 'risk');
  assert.deepEqual(notation.remarksOn(looped), ['Cycle: cli::main → cli::main']);
});

test('a graph without cycles has nothing to remark on', () => {
  assert.deepEqual(notation.remarksOn({ ...SPEC, dependencies: SPEC.dependencies.filter(each => each.from !== 'prune') }), []);
});

test('two dependencies between the same modules are told apart by kind', () => {
  const both = { ...SPEC, dependencies: [{ from: 'cli', to: 'clock', kind: 'runtime' }, { from: 'cli', to: 'clock', kind: 'test' }] };
  assert.deepEqual(notation.problemsIn(both), []);
  assert.deepEqual(notation.sceneOf(both).edges.map(each => each.id), ['cli->clock:runtime', 'cli->clock:test']);
});

test('the graph runs down the page unless it says right', () => {
  assert.equal(scene.direction, 'down');
  assert.equal(notation.sceneOf({ ...SPEC, direction: 'right' }).direction, 'right');
});

test('a graph needs modules', () => {
  assert.deepEqual(problems({ modules: [] }), ['no modules']);
  assert.deepEqual(problems({ modules: undefined }), ['no modules']);
});

test('a module needs an id, and no two may share one', () => {
  assert.deepEqual(problems({ modules: [{ label: 'x' }], dependencies: [] }), ['module 1: no id']);
  assert.deepEqual(problems({ modules: [{ id: 'a' }, { id: 'b', modules: [{ id: 'a' }] }], dependencies: [] }), ['module a: id is used twice']);
});

test('a dependency must run between modules the graph has', () => {
  assert.deepEqual(problems({ dependencies: [{ from: 'cli', to: 'nowhere' }, { to: 'cli' }] }), [
    'dependency cli->nowhere: no module "nowhere"',
    'dependency 2: no from',
  ]);
});

test('the same dependency written twice is reported', () => {
  assert.deepEqual(problems({ dependencies: [{ from: 'cli', to: 'clock' }, { from: 'cli', to: 'clock' }] }), ['dependency cli->clock: written twice']);
});

test('marks outside their vocabulary are reported where they are', () => {
  const marked = { modules: [{ id: 'a', status: 'fresh' }, { id: 'b' }], dependencies: [{ from: 'a', to: 'b', tone: 'scary' }], direction: 'up' };
  assert.deepEqual(problems(marked), [
    'direction "up" is not one of down, right',
    'module a: status "fresh" is not one of new, changed, removed, existing',
    'dependency a->b: tone "scary" is not one of risk, ok',
  ]);
});

const fs = require('node:fs');
const path = require('node:path');
const { kitWith } = require('../../diagram-kit/lib/index.js');

const EXAMPLES = path.join(__dirname, '..', 'examples');

for (const file of fs.readdirSync(EXAMPLES).filter(name => name.endsWith('.json'))) {
  test(`the example ${file} is sound and lays out`, async () => {
    const kit = kitWith(), example = JSON.parse(fs.readFileSync(path.join(EXAMPLES, file), 'utf8'));
    assert.deepEqual(kit.problemsIn(example), []);
    const scene = await kit.laidOut(example);
    assert.equal(scene.edges.every(edge => edge.points.length >= 2), true);
  });
}
