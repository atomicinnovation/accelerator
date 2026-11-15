const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const notation = require('../notation.js');
const { kitWith } = require('../../diagram-kit/lib/index.js');

const SPEC = {
  id: 'accelerator-containers', kind: 'c4-diagram', title: 'The launcher is the only container that touches the cache',
  people: [{ id: 'dev', name: 'Developer', description: 'Uses Claude Code with the plugin.' }],
  systems: [
    { id: 'accelerator', name: 'Accelerator', status: 'changed', link: 'README.md:1', containers: [
      { id: 'launcher', name: 'Launcher', technology: 'Rust', description: 'Resolves a version and runs its sub-binary.', status: 'changed', tone: 'risk' },
      { id: 'cache', name: 'Cache', technology: 'directory', kind: 'store', description: 'Trees and claims.', status: 'new' },
      { id: 'skills', name: 'Skills' },
    ] },
    { id: 'claude', name: 'Claude Code', description: 'Loads the plugin.', external: true },
    { id: 'github', name: 'GitHub Releases', external: true },
  ],
  relationships: [
    { from: 'dev', to: 'claude', label: 'Invokes skills in' },
    { from: 'claude', to: 'skills', label: 'Loads', technology: 'Markdown' },
    { from: 'skills', to: 'launcher', label: 'Runs commands through', technology: 'shell' },
    { from: 'launcher', to: 'cache', label: 'Reads, claims and prunes', technology: 'file system', status: 'new', link: 'src/retention.rs:1' },
    { from: 'launcher', to: 'github', label: 'Fetches versions from', technology: 'HTTPS' },
  ],
};
const scene = notation.sceneOf(SPEC);
const flat = nodes => nodes.flatMap(each => [each, ...flat(each.children || [])]);
const node = id => flat(scene.nodes).find(each => each.id === id);
const edge = id => scene.edges.find(each => each.id === id);
const problems = change => notation.problemsIn({ ...SPEC, ...change });
const remarks = change => notation.remarksOn({ ...SPEC, ...change });

test('it draws diagrams of kind c4-diagram', () => {
  assert.equal(notation.kind, 'c4-diagram');
});

test('a sound diagram has no problems and nothing to remark on', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
  assert.deepEqual(notation.remarksOn(SPEC), []);
});

test('a person is drawn as a person, saying who they are', () => {
  assert.deepEqual([node('dev').shape, node('dev').label, node('dev').type, node('dev').description], ['person', 'Developer', 'Person', 'Uses Claude Code with the plugin.']);
});

test('a software system is an element, muted when it is someone else\'s', () => {
  assert.deepEqual([node('claude').shape, node('claude').label, node('claude').type, node('claude').muted], ['element', 'Claude Code', 'Software System', true]);
  assert.equal(node('github').description, undefined);
});

test('a system whose containers are shown is a boundary around them', () => {
  const system = node('accelerator');
  assert.deepEqual([system.shape, system.stereotype, system.label, system.status, system.link], ['boundary', 'software system', 'Accelerator', 'changed', 'README.md:1']);
  assert.deepEqual(system.children.map(each => each.id), ['launcher', 'cache', 'skills']);
});

test('a container says what it is built with, and a store says it is a store', () => {
  assert.deepEqual([node('launcher').shape, node('launcher').type, node('launcher').status, node('launcher').tone], ['element', 'Container: Rust', 'changed', 'risk']);
  assert.equal(node('cache').type, 'Store: directory');
  assert.equal(node('skills').type, 'Container');
});

test('a relationship is an arrow that says what it is for and how', () => {
  const used = edge('launcher->cache');
  assert.deepEqual([used.line, used.head, used.label, used.status, used.link], ['solid', 'open', 'Reads, claims and prunes [file system]', 'new', 'src/retention.rs:1']);
  assert.equal(edge('dev->claude').label, 'Invokes skills in');
});

test('a diagram needs something in it, each thing with an id and a name of its own', () => {
  assert.deepEqual(problems({ people: [], systems: [] }), ['no people and no systems']);
  assert.deepEqual(problems({ people: [{ name: 'x' }, { id: 'a' }], systems: [{ id: 'a', name: 'A' }, { id: 's', name: 'S', containers: [{ id: 'c' }, { id: 's', name: 'dup' }] }], relationships: [] }), [
    'person 1: no id',
    'person a: no name',
    'system a: id is used twice',
    'system s: container c: no name',
    'system s: container s: id is used twice',
  ]);
});

test('what a thing says about itself must be the right kind of thing', () => {
  assert.deepEqual(problems({ people: [{ id: 'p', name: 'P', description: 4, status: 'fresh' }], systems: [{ id: 's', name: 'S', external: 'yes', containers: [{ id: 'c', name: 'C', kind: 'lake', technology: 9 }] }], relationships: [], direction: 'up' }), [
    'direction "up" is not one of down, right',
    'person p: description must be text',
    'person p: status "fresh" is not one of new, changed, removed, existing',
    'system s: external must be true or false',
    'system s: container c: technology must be text',
    'system s: container c: kind "lake" is not one of container, store',
  ]);
});

test('a relationship runs between things the diagram has, once, and says what it is for', () => {
  assert.deepEqual(problems({ relationships: [{ from: 'dev', to: 'nowhere', label: 'x' }, { to: 'dev', label: 'x' }, { from: 'dev', to: 'claude' }, { from: 'dev', to: 'github', label: 'Reads' }, { from: 'dev', to: 'github', label: 'Reads', tone: 'scary' }, { from: 'dev', to: 'launcher', label: 'Runs', technology: 3 }] }), [
    'relationship dev->nowhere: nothing called "nowhere"',
    'relationship 2: no from',
    'relationship dev->claude: no label; say what the relationship is for',
    'relationship dev->github: written twice',
    'relationship dev->launcher: technology must be text',
  ]);
});

test('a thing joined to nothing is remarked on', () => {
  assert.deepEqual(remarks({ systems: [...SPEC.systems, { id: 'jira', name: 'Jira', external: true }] }), ['Jira is joined to nothing in the diagram']);
});

test('a relationship to a system whose containers are shown is asked to say which container', () => {
  assert.deepEqual(remarks({ relationships: [...SPEC.relationships, { from: 'dev', to: 'accelerator', label: 'Configures' }] }), ['Developer → Accelerator: its containers are shown, so say which one']);
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
