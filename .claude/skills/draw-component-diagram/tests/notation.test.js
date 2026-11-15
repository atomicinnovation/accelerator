const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const notation = require('../notation.js');
const { kitWith } = require('../../diagram-kit/lib/index.js');

const SPEC = {
  id: 'launcher-parts', kind: 'component-diagram', title: 'Retention meets the disk only through an interface',
  components: [
    { id: 'launch', name: 'Launcher', stereotype: 'subsystem', status: 'changed', components: [
      { id: 'start_up', name: 'StartUp', link: 'src/start_up.rs:1' },
      { id: 'retention', name: 'Retention', status: 'new', tone: 'risk' },
    ] },
    { id: 'fs', name: 'FsDisk' },
    { id: 'clock', name: 'SystemClock' },
  ],
  interfaces: [{ id: 'Disk', status: 'new' }, { id: 'Clock', name: 'Clock' }],
  relationships: [
    { from: 'retention', to: 'Disk', kind: 'requires', status: 'new' },
    { from: 'fs', to: 'Disk', kind: 'provides', link: 'src/fs.rs:9' },
    { from: 'start_up', to: 'retention', kind: 'dependency', label: 'refreshes claims' },
    { from: 'clock', to: 'Clock', kind: 'provides' },
    { from: 'retention', to: 'Clock', kind: 'requires' },
  ],
};
const scene = notation.sceneOf(SPEC);
const flat = nodes => nodes.flatMap(each => [each, ...flat(each.children || [])]);
const node = id => flat(scene.nodes).find(each => each.id === id);
const edge = id => scene.edges.find(each => each.id === id);
const problems = change => notation.problemsIn({ ...SPEC, ...change });
const remarks = change => notation.remarksOn({ ...SPEC, ...change });
const two = [{ id: 'a' }, { id: 'b' }];

test('it draws diagrams of kind component-diagram', () => {
  assert.equal(notation.kind, 'component-diagram');
});

test('a sound diagram has no problems and nothing to remark on', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
  assert.deepEqual(notation.remarksOn(SPEC), []);
});

test('a component is a box marked «component», or with the stereotype it gives', () => {
  assert.deepEqual([node('fs').shape, node('fs').stereotype, node('fs').label], ['classifier', 'component', 'FsDisk']);
  assert.deepEqual([node('retention').status, node('retention').tone, node('start_up').link], ['new', 'risk', 'src/start_up.rs:1']);
});

test('a component that holds others is a frame around them', () => {
  const launch = node('launch');
  assert.deepEqual([launch.shape, launch.stereotype, launch.label, launch.status], ['frame', 'subsystem', 'Launcher', 'changed']);
  assert.deepEqual(launch.children.map(each => each.id), ['start_up', 'retention']);
});

test('an interface is a ball named by its name or its id', () => {
  assert.deepEqual([node('Disk').shape, node('Disk').label, node('Disk').status], ['ball', 'Disk', 'new']);
});

test('a provided interface is joined to its component by a plain line, the component drawn below it', () => {
  const provided = edge('fs->Disk');
  assert.deepEqual([provided.line, provided.head, provided.against, provided.link], ['solid', 'none', true, 'src/fs.rs:9']);
});

test('a required interface is reached by a dashed arrow from the component that needs it', () => {
  const required = edge('retention->Disk');
  assert.deepEqual([required.line, required.head, required.against, required.status], ['dashed', 'open', undefined, 'new']);
});

test('a dependency between components is a dashed arrow with what it is for', () => {
  const uses = edge('start_up->retention');
  assert.deepEqual([uses.line, uses.head, uses.label], ['dashed', 'open', 'refreshes claims']);
});

test('a diagram needs components, and everything in it an id of its own', () => {
  assert.deepEqual(problems({ components: [] }), ['no components']);
  assert.deepEqual(problems({ components: [{ name: 'x' }, { id: 'a', components: [{ id: 'a' }] }], interfaces: [{ id: 'a' }, { name: 'I' }], relationships: [] }), [
    'component 1: no id',
    'component a: id is used twice',
    'interface a: id is used twice',
    'interface 2: no id',
  ]);
});

test('marks and stereotypes must be what they may be', () => {
  assert.deepEqual(problems({ components: [{ id: 'a', stereotype: 3, status: 'fresh' }], interfaces: [{ id: 'I', tone: 'scary' }], relationships: [], direction: 'up' }), [
    'direction "up" is not one of down, right',
    'component a: stereotype must be text',
    'component a: status "fresh" is not one of new, changed, removed, existing',
    'interface I: tone "scary" is not one of risk, ok',
  ]);
});

test('a relationship runs between things the diagram has, and is of a kind the notation draws', () => {
  assert.deepEqual(problems({ components: two, interfaces: [{ id: 'I' }], relationships: [{ from: 'a', to: 'z', kind: 'dependency' }, { to: 'a', kind: 'dependency' }, { from: 'a', to: 'b' }, { from: 'a', to: 'b', kind: 'calls' }, { from: 'a', to: 'I', kind: 'provides' }, { from: 'a', to: 'I', kind: 'provides' }] }), [
    'relationship a->z: nothing called "z"',
    'relationship 2: no from',
    'relationship a->b: no kind',
    'relationship a->b: kind "calls" is not one of provides, requires, dependency',
    'relationship a->I: written twice',
  ]);
});

test('an interface is provided and required by components, and a dependency runs between components', () => {
  assert.deepEqual(problems({ components: two, interfaces: [{ id: 'I' }, { id: 'J' }], relationships: [{ from: 'a', to: 'b', kind: 'provides' }, { from: 'I', to: 'J', kind: 'requires' }, { from: 'a', to: 'I', kind: 'dependency' }] }), [
    'relationship a->b: provides runs from a component to an interface',
    'relationship I->J: requires runs from a component to an interface',
    'relationship a->I: a dependency runs between components; a component requires an interface',
  ]);
});

test('an interface nothing provides, or that nothing touches, is remarked on', () => {
  assert.deepEqual(remarks({ relationships: SPEC.relationships.filter(each => each.from !== 'fs') }), ['Disk is required by Retention, but nothing in the diagram provides it', 'FsDisk is joined to nothing in the diagram']);
  assert.deepEqual(remarks({ interfaces: [...SPEC.interfaces, { id: 'Net' }] }), ['Net is neither provided nor required']);
});

test('components that need each other in a ring are remarked on, through interfaces as well as directly', () => {
  const ring = [...SPEC.relationships, { from: 'retention', to: 'Back', kind: 'provides' }, { from: 'fs', to: 'Back', kind: 'requires' }];
  assert.deepEqual(remarks({ interfaces: [...SPEC.interfaces, { id: 'Back' }], relationships: ring }), ['Cycle: Retention → FsDisk → Retention']);
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
