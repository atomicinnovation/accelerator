const test = require('node:test');
const assert = require('node:assert/strict');

const notation = require('../notation.js');

const SPEC = {
  id: 'claims', kind: 'class-diagram', title: 'Claims reach the disk through a port',
  classes: [
    { id: 'Disk', stereotype: 'trait', operations: ['+ lstat(path): Metadata', '+ write(path, bytes)'] },
    { id: 'FsDisk', stereotype: 'struct', link: 'src/outbound/fs.rs:10' },
    { id: 'VersionClaims', status: 'new', attributes: ['- root: PathBuf', { text: '+ WINDOW: Duration', static: true }], operations: ['+ claim(version): Result<Claim>'] },
    { id: 'VersionClaim', name: 'VersionClaim<\'a>', attributes: ['+ version: &str', '+ launcher_id: &str'] },
    { id: 'Retention', abstract: true, tone: 'risk' },
  ],
  relationships: [
    { from: 'FsDisk', to: 'Disk', kind: 'realisation' },
    { from: 'VersionClaims', to: 'Retention', kind: 'generalisation' },
    { from: 'VersionClaims', to: 'Disk', kind: 'dependency', label: 'uses', status: 'new' },
    { from: 'VersionClaims', to: 'VersionClaim', kind: 'composition', fromMultiplicity: '1', toMultiplicity: '0..*', toRole: 'claims', link: 'src/claims.rs:20' },
    { from: 'Retention', to: 'Disk', kind: 'association', navigable: true, toRole: 'disk' },
    { from: 'Retention', to: 'VersionClaim', kind: 'aggregation' },
  ],
};
const scene = notation.sceneOf(SPEC);
const node = id => scene.nodes.find(each => each.id === id);
const edge = id => scene.edges.find(each => each.id === id);
const problems = change => notation.problemsIn({ ...SPEC, ...change });
const two = [{ id: 'A' }, { id: 'B' }];

test('it draws diagrams of kind class-diagram', () => {
  assert.equal(notation.kind, 'class-diagram');
});

test('a sound diagram has no problems and nothing to remark on', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
  assert.deepEqual(notation.remarksOn(SPEC), []);
});

test('a class is a classifier named by its name or its id, keeping its stereotype and marks', () => {
  assert.deepEqual([node('Disk').shape, node('Disk').label, node('Disk').stereotype], ['classifier', 'Disk', 'trait']);
  assert.equal(node('VersionClaim').label, 'VersionClaim<\'a>');
  assert.deepEqual([node('VersionClaims').status, node('Retention').tone, node('FsDisk').link, node('Retention').abstract], ['new', 'risk', 'src/outbound/fs.rs:10', true]);
});

test('a class lists its attributes, then its operations, leaving out a compartment it has nothing for', () => {
  assert.deepEqual(node('VersionClaims').compartments, [['- root: PathBuf', { text: '+ WINDOW: Duration', static: true }], ['+ claim(version): Result<Claim>']]);
  assert.deepEqual(node('Disk').compartments, [['+ lstat(path): Metadata', '+ write(path, bytes)']]);
  assert.deepEqual(node('FsDisk').compartments, []);
});

test('a generalisation is a solid line to a hollow triangle at the general class, which is drawn above', () => {
  const drawn = edge('VersionClaims->Retention');
  assert.deepEqual([drawn.from, drawn.to, drawn.line, drawn.head, drawn.tail, drawn.against], ['VersionClaims', 'Retention', 'solid', 'hollow', undefined, true]);
});

test('a realisation is the same, dashed', () => {
  const drawn = edge('FsDisk->Disk');
  assert.deepEqual([drawn.line, drawn.head, drawn.against], ['dashed', 'hollow', true]);
});

test('a dependency is a dashed line with an open head', () => {
  const drawn = edge('VersionClaims->Disk');
  assert.deepEqual([drawn.line, drawn.head, drawn.against, drawn.label, drawn.status], ['dashed', 'open', undefined, 'uses', 'new']);
});

test('an association is a solid line, with an open head only when it says which way it can be followed', () => {
  assert.deepEqual([edge('Retention->Disk').line, edge('Retention->Disk').head], ['solid', 'open']);
  const plain = notation.sceneOf({ ...SPEC, classes: two, relationships: [{ from: 'A', to: 'B', kind: 'association' }] }).edges[0];
  assert.deepEqual([plain.line, plain.head, plain.tail], ['solid', 'none', undefined]);
});

test('a composition has a filled diamond at the whole, and an aggregation a hollow one', () => {
  assert.deepEqual([edge('VersionClaims->VersionClaim').tail, edge('VersionClaims->VersionClaim').head], ['filled-diamond', 'none']);
  assert.equal(edge('Retention->VersionClaim').tail, 'diamond');
});

test('each end says how many there are and what they are called there', () => {
  const drawn = edge('VersionClaims->VersionClaim');
  assert.deepEqual([drawn.fromLabel, drawn.toLabel, drawn.link], ['1', '0..* claims', 'src/claims.rs:20']);
  assert.deepEqual([edge('Retention->Disk').fromLabel, edge('Retention->Disk').toLabel], [undefined, 'disk']);
});

test('relationships between the same classes are told apart', () => {
  const twice = notation.sceneOf({ ...SPEC, classes: two, relationships: [{ from: 'A', to: 'B', kind: 'association' }, { from: 'A', to: 'B', kind: 'dependency' }] });
  assert.deepEqual(twice.edges.map(each => each.id), ['A->B', 'A->B:2']);
});

test('the diagram runs down the page unless it says right', () => {
  assert.equal(scene.direction, 'down');
  assert.equal(notation.sceneOf({ ...SPEC, direction: 'right' }).direction, 'right');
});

test('a diagram needs classes, each with an id of its own', () => {
  assert.deepEqual(problems({ classes: [] }), ['no classes']);
  assert.deepEqual(problems({ classes: [{ name: 'X' }, { id: 'A' }, { id: 'A' }], relationships: [] }), ['class 1: no id', 'class A: id is used twice']);
});

test('what a class says about itself must be the right kind of thing', () => {
  const classes = [{ id: 'A', abstract: 'yes', stereotype: 7, attributes: 'x: u8', operations: ['+ ok()', 3, { static: true }], status: 'fresh' }];
  assert.deepEqual(problems({ classes, relationships: [] }), [
    'class A: abstract must be true or false',
    'class A: stereotype must be text',
    'class A: attributes must be a list',
    'class A: operation 2 must be text, or an object with text',
    'class A: operation 3 must be text, or an object with text',
    'class A: status "fresh" is not one of new, changed, removed, existing',
  ]);
});

test('a relationship runs between classes the diagram has, and is of a kind the notation draws', () => {
  assert.deepEqual(problems({ classes: two, relationships: [{ from: 'A', to: 'C', kind: 'association' }, { to: 'A', kind: 'association' }, { from: 'A', to: 'B' }, { from: 'A', to: 'B', kind: 'friendship' }] }), [
    'relationship A->C: no class "C"',
    'relationship 2: no from',
    'relationship A->B: no kind',
    'relationship A->B: kind "friendship" is not one of generalisation, realisation, association, aggregation, composition, dependency',
  ]);
});

test('only an association, aggregation or composition has multiplicities, roles or a direction to follow', () => {
  assert.deepEqual(problems({ classes: two, relationships: [{ from: 'A', to: 'B', kind: 'generalisation', toMultiplicity: '1' }, { from: 'A', to: 'B', kind: 'dependency', navigable: true }] }), [
    'relationship A->B: a generalisation has no toMultiplicity',
    'relationship A->B: a dependency has no navigable',
  ]);
});

test('a multiplicity is a number, a range or a star', () => {
  const ends = ['1', '0..1', '*', '0..*', '2..5'].map(m => ({ from: 'A', to: 'B', kind: 'association', label: m, toMultiplicity: m }));
  assert.deepEqual(problems({ classes: two, relationships: ends }), []);
  assert.deepEqual(problems({ classes: two, relationships: [{ from: 'A', to: 'B', kind: 'association', fromMultiplicity: 'many' }] }), [
    'relationship A->B: fromMultiplicity "many" is not written like 1, 0..1 or 0..*',
  ]);
});

test('a class cannot be more general than itself, directly or round a ring', () => {
  assert.deepEqual(problems({ classes: two, relationships: [{ from: 'A', to: 'A', kind: 'generalisation' }] }), ['relationship A->A: a class cannot specialise itself']);
  assert.deepEqual(problems({ classes: [...two, { id: 'C' }], relationships: [{ from: 'A', to: 'B', kind: 'generalisation' }, { from: 'B', to: 'C', kind: 'realisation' }, { from: 'C', to: 'A', kind: 'generalisation' }] }), [
    'generalisation runs in a ring: A → B → C → A',
  ]);
});

test('the same relationship written twice is reported', () => {
  const same = { from: 'A', to: 'B', kind: 'association' };
  assert.deepEqual(problems({ classes: two, relationships: [same, same] }), ['relationship A->B: written twice']);
});

test('marks and direction outside what they may be are reported', () => {
  assert.deepEqual(problems({ direction: 'up', classes: two, relationships: [{ from: 'A', to: 'B', kind: 'association', tone: 'scary', navigable: 'yes' }] }), [
    'direction "up" is not one of down, right',
    'relationship A->B: tone "scary" is not one of risk, ok',
    'relationship A->B: navigable must be true or false',
  ]);
});

test('a class related to nothing in a diagram of relationships is remarked on', () => {
  assert.deepEqual(notation.remarksOn({ ...SPEC, classes: [...SPEC.classes, { id: 'Stray' }, { id: 'Other', name: 'Other one' }] }), ['Stray and Other one are related to nothing in the diagram']);
  assert.deepEqual(notation.remarksOn({ ...SPEC, classes: two, relationships: [] }), []);
});

test('a part composed into two wholes is remarked on', () => {
  const classes = [{ id: 'A' }, { id: 'B' }, { id: 'Part' }];
  const relationships = [{ from: 'A', to: 'Part', kind: 'composition' }, { from: 'B', to: 'Part', kind: 'composition' }];
  assert.deepEqual(notation.remarksOn({ ...SPEC, classes, relationships }), ['Part is a composite part of A and B; a part has one owner']);
});

const fs = require('node:fs');
const path = require('node:path');
const { kitWith } = require('../../diagram-kit/lib/index.js');

const EXAMPLES = path.join(__dirname, '..', 'examples');

for (const file of fs.readdirSync(EXAMPLES).filter(name => name.endsWith('.json'))) {
  test(`the example ${file} is sound and lays out`, async () => {
    const kit = kitWith(), example = JSON.parse(fs.readFileSync(path.join(EXAMPLES, file), 'utf8'));
    assert.deepEqual(kit.problemsIn(example), []);
    assert.deepEqual(kit.remarksOn(example), []);
    assert.equal((await kit.laidOut(example)).edges.every(edge => edge.points.length >= 2), true);
  });
}
