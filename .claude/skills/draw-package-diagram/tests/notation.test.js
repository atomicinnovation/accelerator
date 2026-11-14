const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const notation = require('../notation.js');
const { kitWith } = require('../../diagram-kit/lib/index.js');

const SPEC = {
  id: 'launcher-packages', kind: 'package-diagram', title: 'Outbound is the only package that touches the system',
  packages: [
    { id: 'launch', status: 'changed', link: 'src/launch/mod.rs:1', packages: [
      { id: 'start_up' },
      { id: 'retention', name: 'launch::retention', status: 'new', tone: 'risk' },
    ] },
    { id: 'outbound', packages: [{ id: 'fs' }, { id: 'clock' }] },
    { id: 'kernel' },
  ],
  relationships: [
    { from: 'start_up', to: 'retention' },
    { from: 'retention', to: 'outbound', kind: 'import', status: 'new' },
    { from: 'retention', to: 'kernel', kind: 'access', link: 'src/launch/retention/mod.rs:3' },
    { from: 'fs', to: 'kernel', kind: 'dependency', label: 'errors' },
    { from: 'launch', to: 'kernel', kind: 'merge' },
  ],
};
const scene = notation.sceneOf(SPEC);
const flat = nodes => nodes.flatMap(each => [each, ...flat(each.children || [])]);
const node = id => flat(scene.nodes).find(each => each.id === id);
const edge = id => scene.edges.find(each => each.id === id);
const problems = change => notation.problemsIn({ ...SPEC, ...change });
const remarks = relationships => notation.remarksOn({ ...SPEC, relationships });

test('it draws diagrams of kind package-diagram', () => {
  assert.equal(notation.kind, 'package-diagram');
});

test('a sound diagram has no problems and nothing to remark on', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
  assert.deepEqual(notation.remarksOn(SPEC), []);
});

test('a package that holds others is an open folder around them, and one that does not is a closed one', () => {
  assert.deepEqual([node('launch').shape, node('launch').children.map(each => each.id)], ['package', ['start_up', 'retention']]);
  assert.deepEqual([node('kernel').shape, node('start_up').shape], ['folder', 'folder']);
});

test('a package is named by its name or its id, and keeps its marks and link', () => {
  assert.deepEqual([node('retention').label, node('kernel').label], ['launch::retention', 'kernel']);
  assert.deepEqual([node('launch').status, node('launch').link, node('retention').tone], ['changed', 'src/launch/mod.rs:1', 'risk']);
});

test('every relationship is a dashed arrow with an open head', () => {
  for (const each of scene.edges) assert.deepEqual([each.line, each.head], ['dashed', 'open'], each.id);
});

test('an import, an access and a merge say which they are; a dependency says only what it is given', () => {
  assert.equal(edge('start_up->retention').label, undefined);
  assert.deepEqual([edge('retention->outbound').label, edge('retention->outbound').status], ['«import»', 'new']);
  assert.deepEqual([edge('retention->kernel').label, edge('retention->kernel').link], ['«access»', 'src/launch/retention/mod.rs:3']);
  assert.equal(edge('fs->kernel').label, 'errors');
  assert.equal(edge('launch->kernel').label, '«merge»');
});

test('a diagram needs packages, each with an id of its own', () => {
  assert.deepEqual(problems({ packages: [] }), ['no packages']);
  assert.deepEqual(problems({ packages: [{ name: 'x' }, { id: 'a', packages: [{ id: 'a' }] }, { id: 'b', status: 'fresh' }], relationships: [], direction: 'up' }), [
    'direction "up" is not one of down, right',
    'package 1: no id',
    'package a: id is used twice',
    'package b: status "fresh" is not one of new, changed, removed, existing',
  ]);
});

test('a relationship runs between packages the diagram has, once, and is of a kind the notation draws', () => {
  assert.deepEqual(problems({ relationships: [{ from: 'kernel', to: 'nowhere' }, { to: 'kernel' }, { from: 'fs', to: 'kernel', kind: 'friend' }, { from: 'fs', to: 'clock' }, { from: 'fs', to: 'clock' }, { from: 'fs', to: 'clock', kind: 'import', tone: 'scary' }] }), [
    'relationship kernel->nowhere: no package "nowhere"',
    'relationship 2: no from',
    'relationship fs->kernel: kind "friend" is not one of dependency, import, access, merge',
    'relationship fs->clock: written twice',
    'relationship fs->clock: tone "scary" is not one of risk, ok',
  ]);
});

test('packages that depend on each other in a ring are remarked on', () => {
  assert.deepEqual(remarks([{ from: 'start_up', to: 'retention' }, { from: 'retention', to: 'fs', kind: 'import' }, { from: 'fs', to: 'start_up' }]), ['Cycle: start_up → launch::retention → fs → start_up']);
});

test('a package that depends on one it holds, or on the one that holds it, is remarked on', () => {
  assert.deepEqual(remarks([{ from: 'launch', to: 'retention' }, { from: 'fs', to: 'outbound' }]), [
    'launch depends on launch::retention, which it holds',
    'fs depends on outbound, which holds it',
  ]);
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
