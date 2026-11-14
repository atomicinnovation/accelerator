const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const notation = require('../notation.js');
const { kitWith } = require('../../diagram-kit/lib/index.js');

const SPEC = {
  id: 'claims-schema', kind: 'entity-relationship-diagram', title: 'A claim belongs to one install and one version',
  entities: [
    { id: 'install', attributes: [{ name: 'id', type: 'uuid', key: 'primary' }, { name: 'path', type: 'text', key: 'unique' }] },
    { id: 'version', name: 'release_version', status: 'changed', link: 'schema.sql:12', attributes: [{ name: 'id', type: 'uuid', key: 'primary' }, 'tag'] },
    { id: 'claim', status: 'new', tone: 'risk', attributes: [
      { name: 'install_id', type: 'uuid', key: ['primary', 'foreign'] },
      { name: 'version_id', type: 'uuid', key: ['primary', 'foreign'], status: 'new' },
      { name: 'refreshed_at', type: 'timestamp', nullable: true },
    ] },
  ],
  relationships: [
    { from: 'install', to: 'claim', label: 'holds', fromCardinality: 'one', toCardinality: 'zero-or-many', status: 'new' },
    { from: 'version', to: 'claim', fromCardinality: 'one', toCardinality: 'zero-or-many', identifying: false, link: 'schema.sql:30' },
  ],
};
const scene = notation.sceneOf(SPEC);
const node = id => scene.nodes.find(each => each.id === id);
const problems = change => notation.problemsIn({ ...SPEC, ...change });
const remarks = change => notation.remarksOn({ ...SPEC, ...change });
const keyed = id => ({ id, attributes: [{ name: 'id', key: 'primary' }] });

test('it draws diagrams of kind entity-relationship-diagram', () => {
  assert.equal(notation.kind, 'entity-relationship-diagram');
});

test('a sound diagram has no problems and nothing to remark on', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
  assert.deepEqual(notation.remarksOn(SPEC), []);
});

test('an entity is named by its name or its id, and keeps its marks and link', () => {
  assert.deepEqual([node('version').shape, node('version').label, node('version').status, node('version').link], ['entity', 'release_version', 'changed', 'schema.sql:12']);
  assert.deepEqual([node('claim').label, node('claim').tone], ['claim', 'risk']);
});

test('each attribute is a row with its key, name and type', () => {
  assert.deepEqual(node('install').rows, [{ key: 'PK', name: 'id', type: 'uuid', status: undefined }, { key: 'UK', name: 'path', type: 'text', status: undefined }]);
  assert.deepEqual(node('version').rows[1], { key: undefined, name: 'tag', type: undefined, status: undefined });
});

test('an attribute with several keys shows them all, one that may be empty says so, and one keeps its status', () => {
  assert.deepEqual(node('claim').rows, [
    { key: 'PK, FK', name: 'install_id', type: 'uuid', status: undefined },
    { key: 'PK, FK', name: 'version_id', type: 'uuid', status: 'new' },
    { key: undefined, name: 'refreshed_at', type: 'timestamp ?', status: undefined },
  ]);
});

test('a relationship is a line with how many at each end and nothing else', () => {
  const [holds, names] = scene.edges;
  assert.deepEqual([holds.id, holds.from, holds.to, holds.tail, holds.head, holds.line, holds.label, holds.status], ['install->claim', 'install', 'claim', 'one', 'zero-or-many', 'solid', 'holds', 'new']);
  assert.deepEqual([names.line, names.link], ['dashed', 'schema.sql:30']);
});

test('a diagram needs entities, each with an id of its own', () => {
  assert.deepEqual(problems({ entities: [] }), ['no entities']);
  assert.deepEqual(problems({ entities: [{ name: 'x' }, keyed('a'), keyed('a')], relationships: [], direction: 'up' }), [
    'direction "up" is not one of down, right',
    'entity 1: no id',
    'entity a: id is used twice',
  ]);
});

test('an attribute has a name of its own in its entity, and keys the notation knows', () => {
  const attributes = [{ type: 'uuid' }, { name: 'id', key: 'main' }, { name: 'id' }, 7, { name: 'at', type: 4, nullable: 'yes', status: 'fresh' }];
  assert.deepEqual(problems({ entities: [{ id: 'a', attributes, status: 'fresh' }, { id: 'b', attributes: 'id' }], relationships: [] }), [
    'entity a: attribute 1: no name',
    'entity a: attribute id: key "main" is not one of primary, foreign, unique',
    'entity a: attribute id: name is used twice',
    'entity a: attribute 4: must be a name, or an object with a name',
    'entity a: attribute at: type must be text',
    'entity a: attribute at: nullable must be true or false',
    'entity a: attribute at: status "fresh" is not one of new, changed, removed, existing',
    'entity a: status "fresh" is not one of new, changed, removed, existing',
    'entity b: attributes must be a list',
  ]);
});

test('a relationship runs between entities the diagram has, once, and says how many at each end', () => {
  const sound = { from: 'a', to: 'b', fromCardinality: 'one', toCardinality: 'one' };
  assert.deepEqual(problems({ entities: [keyed('a'), keyed('b')], relationships: [{ ...sound, to: 'z' }, { ...sound, from: undefined }, { from: 'a', to: 'b', toCardinality: 'lots' }, sound, sound, { ...sound, label: 'other', identifying: 'no', tone: 'scary' }] }), [
    'relationship a->z: no entity "z"',
    'relationship 2: no from',
    'relationship a->b: no fromCardinality',
    'relationship a->b: toCardinality "lots" is not one of one, zero-or-one, one-or-many, zero-or-many',
    'relationship a->b: written twice',
    'relationship a->b: tone "scary" is not one of risk, ok',
    'relationship a->b: identifying must be true or false',
  ]);
});

test('an entity with no primary key is remarked on', () => {
  assert.deepEqual(remarks({ entities: [...SPEC.entities.slice(0, 2), { id: 'claim', attributes: [{ name: 'install_id', key: 'foreign' }] }] }), ['claim has no primary key']);
});

test('an entity related to nothing in a diagram of relationships is remarked on', () => {
  assert.deepEqual(remarks({ entities: [...SPEC.entities, keyed('audit')] }), ['audit is related to nothing in the diagram']);
});

test('two entities related many to many are remarked on, since a table must stand between them', () => {
  assert.deepEqual(remarks({ relationships: [...SPEC.relationships, { from: 'install', to: 'version', fromCardinality: 'zero-or-many', toCardinality: 'one-or-many' }] }), [
    'install and release_version are related many to many; in a database that needs a table between them',
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
