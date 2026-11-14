const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const notation = require('../notation.js');
const { kitWith } = require('../../diagram-kit/lib/index.js');

const SPEC = {
  id: 'retention-domain', kind: 'domain-model', title: 'A claim belongs to the cache root that holds it',
  aggregates: [
    { id: 'cache-root', name: 'CacheRoot', status: 'changed', link: 'src/root.rs:1', members: [
      { id: 'CacheRoot', kind: 'aggregate root', operations: ['+ prune(now): Report'] },
      { id: 'VersionClaim', kind: 'entity', status: 'new', attributes: ['+ version: Version', '+ refreshed: Instant'] },
      { id: 'LauncherId', kind: 'value object', attributes: ['- digest: [u8; 8]'] },
    ] },
    { id: 'install', name: 'Install', members: [{ id: 'Install', kind: 'aggregate root' }] },
  ],
  others: [
    { id: 'Retention', kind: 'domain service', tone: 'risk' },
    { id: 'Roots', kind: 'repository' },
    { id: 'ClaimExpired', kind: 'domain event' },
  ],
  relationships: [
    { from: 'CacheRoot', to: 'VersionClaim', kind: 'composition', toMultiplicity: '0..*', toRole: 'claims' },
    { from: 'VersionClaim', to: 'LauncherId', kind: 'composition', toMultiplicity: '1' },
    { from: 'VersionClaim', to: 'Install', kind: 'reference', status: 'new' },
    { from: 'Retention', to: 'CacheRoot', kind: 'dependency', label: 'decides for' },
    { from: 'Roots', to: 'CacheRoot', kind: 'association', navigable: true, label: 'loads' },
    { from: 'CacheRoot', to: 'ClaimExpired', kind: 'dependency', label: 'publishes' },
  ],
};
const scene = notation.sceneOf(SPEC);
const within = id => scene.nodes.find(each => each.id === id);
const member = id => scene.nodes.flatMap(each => each.children || []).find(each => each.id === id);
const edge = id => scene.edges.find(each => each.id === id);
const problems = change => notation.problemsIn({ ...SPEC, ...change });
const remarks = relationships => notation.remarksOn({ ...SPEC, relationships });
const lone = (id, kind = 'aggregate root') => ({ id: id.toLowerCase(), name: id, members: [{ id, kind }] });

test('it draws diagrams of kind domain-model', () => {
  assert.equal(notation.kind, 'domain-model');
});

test('a sound model has no problems and nothing to remark on', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
  assert.deepEqual(notation.remarksOn(SPEC), []);
});

test('an aggregate is a boundary, named, around its members', () => {
  const root = within('cache-root');
  assert.deepEqual([root.shape, root.stereotype, root.label, root.status, root.link], ['boundary', 'aggregate', 'CacheRoot', 'changed', 'src/root.rs:1']);
  assert.deepEqual(root.children.map(each => each.id), ['CacheRoot', 'VersionClaim', 'LauncherId']);
});

test('each member and each other element is a classifier that says which building block it is', () => {
  assert.deepEqual([member('CacheRoot').shape, member('CacheRoot').stereotype], ['classifier', 'aggregate root']);
  assert.equal(member('LauncherId').stereotype, 'value object');
  assert.deepEqual([within('Retention').shape, within('Retention').stereotype, within('Retention').tone], ['classifier', 'domain service', 'risk']);
  assert.equal(within('ClaimExpired').stereotype, 'domain event');
});

test('an element lists its attributes then its operations, and keeps its marks', () => {
  assert.deepEqual(member('VersionClaim').compartments, [['+ version: Version', '+ refreshed: Instant']]);
  assert.deepEqual(member('CacheRoot').compartments, [['+ prune(now): Report']]);
  assert.equal(member('VersionClaim').status, 'new');
});

test('owning is a composition, with a filled diamond at the owner and what is at each end', () => {
  const owned = edge('CacheRoot->VersionClaim');
  assert.deepEqual([owned.line, owned.tail, owned.head, owned.toLabel], ['solid', 'filled-diamond', 'none', '0..* claims']);
});

test('a reference to another aggregate is a dashed arrow that says it is by identity', () => {
  const reference = edge('VersionClaim->Install');
  assert.deepEqual([reference.line, reference.head, reference.label, reference.status], ['dashed', 'open', 'by identity', 'new']);
  const named = notation.sceneOf({ ...SPEC, relationships: [{ from: 'VersionClaim', to: 'Install', kind: 'reference', label: 'installed by' }] }).edges[0];
  assert.equal(named.label, 'installed by');
});

test('the other relationships are drawn as a class diagram draws them', () => {
  assert.deepEqual([edge('Retention->CacheRoot').line, edge('Retention->CacheRoot').head, edge('Retention->CacheRoot').label], ['dashed', 'open', 'decides for']);
  assert.deepEqual([edge('Roots->CacheRoot').line, edge('Roots->CacheRoot').head], ['solid', 'open']);
  const general = notation.sceneOf({ ...SPEC, relationships: [{ from: 'VersionClaim', to: 'CacheRoot', kind: 'generalisation' }] }).edges[0];
  assert.deepEqual([general.head, general.against], ['hollow', true]);
});

test('a model needs something in it', () => {
  assert.deepEqual(problems({ aggregates: [], others: [] }), ['no aggregates and no other elements']);
  assert.deepEqual(problems({ aggregates: undefined, others: [{ id: 'S', kind: 'domain service' }], relationships: [] }), []);
});

test('an aggregate needs an id, members, and exactly one root', () => {
  assert.deepEqual(problems({ aggregates: [{ name: 'X' }, { id: 'empty', members: [] }, { id: 'headless', members: [{ id: 'E', kind: 'entity' }] }, { id: 'hydra', members: [{ id: 'R1', kind: 'aggregate root' }, { id: 'R2', kind: 'aggregate root' }] }], others: [], relationships: [] }), [
    'aggregate 1: no id',
    'aggregate empty: no members',
    'aggregate headless: needs exactly one aggregate root, and has 0',
    'aggregate hydra: needs exactly one aggregate root, and has 2',
  ]);
});

test('every aggregate, member and other element has an id of its own', () => {
  assert.deepEqual(problems({ aggregates: [lone('Order'), { id: 'x', members: [{ kind: 'aggregate root' }] }], others: [{ id: 'Order', kind: 'repository' }, { id: 'order', kind: 'repository' }], relationships: [] }), [
    'aggregate x: member 1: no id',
    'element Order: id is used twice',
    'element order: id is used twice',
  ]);
});

test('each element is a building block that belongs where it is', () => {
  assert.deepEqual(problems({ aggregates: [{ id: 'a', members: [{ id: 'R', kind: 'aggregate root' }, { id: 'S', kind: 'repository' }, { id: 'N' }] }], others: [{ id: 'T', kind: 'aggregate root' }, { id: 'U', kind: 'widget', status: 'fresh' }], relationships: [] }), [
    'aggregate a: member S: kind "repository" is not one of aggregate root, entity, value object',
    'aggregate a: member N: no kind',
    'element T: kind "aggregate root" is not one of domain service, repository, factory, domain event, entity, value object',
    'element U: kind "widget" is not one of domain service, repository, factory, domain event, entity, value object',
    'element U: status "fresh" is not one of new, changed, removed, existing',
  ]);
});

test('members of an element must be text', () => {
  assert.deepEqual(problems({ aggregates: [{ id: 'a', members: [{ id: 'R', kind: 'aggregate root', attributes: 'x', operations: [1] }] }], others: [], relationships: [] }), [
    'aggregate a: member R: attributes must be a list',
    'aggregate a: member R: operation 1 must be text, or an object with text',
  ]);
});

test('a relationship runs between elements or aggregates the model has, and is of a kind the notation draws', () => {
  assert.deepEqual(problems({ relationships: [{ from: 'Retention', to: 'install', kind: 'dependency' }, { from: 'Retention', to: 'Nope', kind: 'dependency' }, { to: 'Retention', kind: 'dependency' }, { from: 'Retention', to: 'Roots' }, { from: 'Retention', to: 'Roots', kind: 'realisation' }] }), [
    'relationship Retention->Nope: nothing called "Nope"',
    'relationship 3: no from',
    'relationship Retention->Roots: no kind',
    'relationship Retention->Roots: kind "realisation" is not one of composition, association, reference, dependency, generalisation',
  ]);
});

test('ends are checked as a class diagram checks them', () => {
  assert.deepEqual(problems({ relationships: [{ from: 'CacheRoot', to: 'VersionClaim', kind: 'composition', toMultiplicity: 'lots' }, { from: 'Retention', to: 'CacheRoot', kind: 'dependency', toRole: 'root' }, { from: 'Roots', to: 'CacheRoot', kind: 'association' }, { from: 'Roots', to: 'CacheRoot', kind: 'association' }] }), [
    'relationship CacheRoot->VersionClaim: toMultiplicity "lots" is not written like 1, 0..1 or 0..*',
    'relationship Retention->CacheRoot: a dependency has no toRole',
    'relationship Roots->CacheRoot: written twice',
  ]);
});

test('reaching into an aggregate past its root is remarked on', () => {
  assert.deepEqual(remarks([{ from: 'Retention', to: 'VersionClaim', kind: 'dependency' }, { from: 'Install', to: 'LauncherId', kind: 'reference' }]), [
    'Retention reaches into the CacheRoot aggregate past its root, at VersionClaim',
    'Install reaches into the CacheRoot aggregate past its root, at LauncherId',
  ]);
});

test('holding another aggregate directly is remarked on, and referring to it by identity is not', () => {
  assert.deepEqual(remarks([{ from: 'VersionClaim', to: 'Install', kind: 'association' }, { from: 'CacheRoot', to: 'Install', kind: 'composition' }]), [
    'VersionClaim holds Install across an aggregate boundary; refer to another aggregate by identity',
    'CacheRoot holds Install across an aggregate boundary; refer to another aggregate by identity',
  ]);
  assert.deepEqual(remarks([{ from: 'VersionClaim', to: 'Install', kind: 'reference' }, { from: 'Retention', to: 'Install', kind: 'association' }]), []);
});

test('a value object that owns an entity is remarked on', () => {
  assert.deepEqual(remarks([{ from: 'LauncherId', to: 'VersionClaim', kind: 'composition' }]), ['LauncherId is a value object but owns the entity VersionClaim']);
});

const EXAMPLES = path.join(__dirname, '..', 'examples');
for (const file of fs.readdirSync(EXAMPLES).filter(name => name.endsWith('.json'))) {
  test(`the example ${file} is sound and lays out`, async () => {
    const kit = kitWith(), example = JSON.parse(fs.readFileSync(path.join(EXAMPLES, file), 'utf8'));
    assert.deepEqual(kit.problemsIn(example), []);
    assert.equal((await kit.laidOut(example)).edges.every(each => each.points.length >= 2), true);
  });
}
