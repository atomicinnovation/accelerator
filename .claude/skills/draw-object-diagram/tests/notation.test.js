const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const notation = require('../notation.js');
const { kitWith } = require('../../diagram-kit/lib/index.js');

const SPEC = {
  id: 'claims-on-disk', kind: 'object-diagram', title: 'Two installs each hold a claim on one version',
  objects: [
    { id: 'claims', name: 'claims', class: 'VersionClaims', slots: ['root = "~/.cache/accelerator"'], status: 'new', link: 'src/claims.rs:18' },
    { id: 'a', class: 'VersionClaim', slots: ['version = "1.26.0"', 'launcher_id = "c0ffee00"'] },
    { id: 'b', name: 'stale', slots: [{ text: 'mtime = 21 d ago' }], tone: 'risk' },
    { id: 'bare' },
  ],
  links: [
    { from: 'claims', to: 'a', label: 'holds', toRole: 'claims', status: 'new' },
    { from: 'claims', to: 'b', fromRole: 'owner', toRole: 'claims', link: 'src/claims.rs:20' },
    { from: 'claims', to: 'bare' },
  ],
};
const scene = notation.sceneOf(SPEC);
const node = id => scene.nodes.find(each => each.id === id);
const problems = change => notation.problemsIn({ ...SPEC, ...change });
const two = [{ id: 'x' }, { id: 'y' }];

test('it draws diagrams of kind object-diagram', () => {
  assert.equal(notation.kind, 'object-diagram');
});

test('a sound diagram has no problems and nothing to remark on', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
  assert.deepEqual(notation.remarksOn(SPEC), []);
});

test('an object is an underlined name and class, with whichever it has', () => {
  assert.deepEqual([node('claims').shape, node('claims').underlined, node('claims').label], ['classifier', true, 'claims: VersionClaims']);
  assert.equal(node('a').label, ': VersionClaim');
  assert.equal(node('b').label, 'stale');
  assert.equal(node('bare').label, 'bare');
});

test('an object lists its slots, and keeps its marks and link', () => {
  assert.deepEqual(node('a').compartments, [['version = "1.26.0"', 'launcher_id = "c0ffee00"']]);
  assert.deepEqual(node('bare').compartments, []);
  assert.deepEqual([node('claims').status, node('claims').link, node('b').tone], ['new', 'src/claims.rs:18', 'risk']);
});

test('a link is a plain solid line that says what each end is', () => {
  const [named, roles, bare] = scene.edges;
  assert.deepEqual([named.id, named.line, named.head, named.label, named.toLabel, named.status], ['claims->a', 'solid', 'none', 'holds', 'claims', 'new']);
  assert.deepEqual([roles.fromLabel, roles.toLabel, roles.link], ['owner', 'claims', 'src/claims.rs:20']);
  assert.deepEqual([bare.label, bare.fromLabel, bare.toLabel], [undefined, undefined, undefined]);
});

test('links between the same objects are told apart', () => {
  const twice = notation.sceneOf({ ...SPEC, objects: two, links: [{ from: 'x', to: 'y', label: 'a' }, { from: 'x', to: 'y', label: 'b' }] });
  assert.deepEqual(twice.edges.map(each => each.id), ['x->y', 'x->y:2']);
});

test('a diagram needs objects, each with an id of its own', () => {
  assert.deepEqual(problems({ objects: [] }), ['no objects']);
  assert.deepEqual(problems({ objects: [{ name: 'n' }, { id: 'x' }, { id: 'x' }], links: [] }), ['object 1: no id', 'object x: id is used twice']);
});

test('what an object says about itself must be the right kind of thing', () => {
  assert.deepEqual(problems({ objects: [{ id: 'x', name: 7, class: [], slots: 'a = 1', status: 'fresh' }, { id: 'y', slots: ['ok = 1', 2] }], links: [] }), [
    'object x: name must be text',
    'object x: class must be text',
    'object x: slots must be a list',
    'object x: status "fresh" is not one of new, changed, removed, existing',
    'object y: slot 2 must be text, or an object with text',
  ]);
});

test('a link runs between objects the diagram has, once', () => {
  assert.deepEqual(problems({ objects: two, links: [{ from: 'x', to: 'z' }, { to: 'x' }, { from: 'x', to: 'y' }, { from: 'x', to: 'y' }, { from: 'x', to: 'y', label: 'other', tone: 'scary' }] }), [
    'link x->z: no object "z"',
    'link 2: no from',
    'link x->y: written twice',
    'link x->y: tone "scary" is not one of risk, ok',
  ]);
});

test('direction outside what it may be is reported', () => {
  assert.deepEqual(problems({ direction: 'up' }), ['direction "up" is not one of down, right']);
});

test('an object linked to nothing in a diagram of links is remarked on', () => {
  assert.deepEqual(notation.remarksOn({ ...SPEC, objects: [...SPEC.objects, { id: 'stray', name: 'orphan', class: 'Tree' }] }), ['orphan: Tree is linked to nothing in the diagram']);
  assert.deepEqual(notation.remarksOn({ ...SPEC, objects: two, links: [] }), []);
});

const EXAMPLES = path.join(__dirname, '..', 'examples');
for (const file of fs.readdirSync(EXAMPLES).filter(name => name.endsWith('.json'))) {
  test(`the example ${file} is sound and lays out`, async () => {
    const kit = kitWith(), example = JSON.parse(fs.readFileSync(path.join(EXAMPLES, file), 'utf8'));
    assert.deepEqual(kit.problemsIn(example), []);
    assert.deepEqual(kit.remarksOn(example), []);
    assert.equal((await kit.laidOut(example)).edges.every(edge => edge.points.length >= 2), true);
  });
}
