const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const notation = require('../notation.js');
const { kitWith } = require('../../diagram-kit/lib/index.js');

const SPEC = {
  id: 'work-contexts', kind: 'context-map', title: 'Work items keep their own model behind a layer for each tracker',
  contexts: [
    { id: 'work', name: 'Work items', subdomain: 'core', team: 'Accelerator', description: 'What is to be done, and its state.', status: 'changed', link: 'cli/work/src/lib.rs:1' },
    { id: 'planning', name: 'Planning', subdomain: 'core' },
    { id: 'jira', name: 'Jira', external: true, description: 'Atlassian\'s issue tracker.' },
    { id: 'kernel', name: 'Kernel', subdomain: 'generic', tone: 'risk' },
  ],
  relationships: [
    { upstream: 'jira', downstream: 'work', upstreamPatterns: ['open host service', 'published language'], downstreamPatterns: ['anticorruption layer'], label: 'issues', status: 'new', link: 'cli/jira-client/src/lib.rs:1' },
    { upstream: 'work', downstream: 'planning', kind: 'customer-supplier' },
    { between: ['work', 'kernel'], kind: 'shared kernel' },
    { between: ['planning', 'kernel'], kind: 'partnership', label: 'release together' },
  ],
};
const scene = notation.sceneOf(SPEC);
const node = id => scene.nodes.find(each => each.id === id);
const edge = id => scene.edges.find(each => each.id === id);
const problems = change => notation.problemsIn({ ...SPEC, ...change });
const remarks = change => notation.remarksOn({ ...SPEC, ...change });
const two = [{ id: 'a', name: 'A' }, { id: 'b', name: 'B' }];

test('it draws diagrams of kind context-map', () => {
  assert.equal(notation.kind, 'context-map');
});

test('a sound map has no problems and nothing to remark on', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
  assert.deepEqual(notation.remarksOn(SPEC), []);
});

test('a bounded context says its name, what kind of subdomain it is, whose it is and what it is for', () => {
  assert.deepEqual([node('work').shape, node('work').label, node('work').type, node('work').description, node('work').status, node('work').link],
    ['element', 'Work items', 'Core · Accelerator', 'What is to be done, and its state.', 'changed', 'cli/work/src/lib.rs:1']);
  assert.equal(node('planning').type, 'Core');
  assert.deepEqual([node('kernel').type, node('kernel').tone], ['Generic', 'risk']);
});

test('a context that is someone else\'s is greyed and says so', () => {
  assert.deepEqual([node('jira').muted, node('jira').type], [true, 'External']);
  assert.equal(node('work').muted, undefined);
});

test('an upstream context is joined to its downstream by a line that says which is which and how each takes part', () => {
  const taken = edge('jira->work');
  assert.deepEqual([taken.from, taken.to, taken.line, taken.head, taken.fromLabel, taken.toLabel, taken.label, taken.status, taken.link],
    ['jira', 'work', 'solid', 'none', 'U · OHS, PL', 'D · ACL', 'issues', 'new', 'cli/jira-client/src/lib.rs:1']);
});

test('a customer and its supplier are named on the line between them', () => {
  const served = edge('work->planning');
  assert.deepEqual([served.fromLabel, served.toLabel, served.label], ['U', 'D', 'customer/supplier']);
});

test('a relationship between equals has no upstream, and says what it is', () => {
  const shared = edge('work--kernel');
  assert.deepEqual([shared.from, shared.to, shared.line, shared.head, shared.label, shared.fromLabel, shared.toLabel], ['work', 'kernel', 'solid', 'none', 'shared kernel', undefined, undefined]);
  assert.equal(edge('planning--kernel').label, 'partnership: release together');
});

test('the scene spells out every abbreviation it uses, and no other', () => {
  assert.deepEqual(scene.key, [['U', 'upstream'], ['D', 'downstream'], ['OHS', 'open host service'], ['PL', 'published language'], ['ACL', 'anticorruption layer']]);
  assert.equal(notation.sceneOf({ ...SPEC, relationships: SPEC.relationships.slice(2) }).key, undefined);
});

test('a map needs contexts, each with an id of its own and a name', () => {
  assert.deepEqual(problems({ contexts: [] }), ['no contexts']);
  assert.deepEqual(problems({ contexts: [{ name: 'x' }, { id: 'a' }, { id: 'b', name: 'B' }, { id: 'b', name: 'B' }], relationships: [], direction: 'up' }), [
    'direction "up" is not one of down, right',
    'context 1: no id',
    'context a: no name',
    'context b: id is used twice',
  ]);
});

test('what a context says about itself must be what it may be', () => {
  assert.deepEqual(problems({ contexts: [{ id: 'a', name: 'A', subdomain: 'main', external: 'yes', team: 4, description: [], status: 'fresh' }], relationships: [] }), [
    'context a: subdomain "main" is not one of core, supporting, generic',
    'context a: external must be true or false',
    'context a: team must be text',
    'context a: description must be text',
    'context a: status "fresh" is not one of new, changed, removed, existing',
  ]);
});

test('a relationship has an upstream and a downstream, or is between two equals', () => {
  assert.deepEqual(problems({ contexts: two, relationships: [{ upstream: 'a' }, { between: ['a'] }, { upstream: 'a', downstream: 'b', between: ['a', 'b'] }, { upstream: 'a', downstream: 'z' }, { between: ['a', 'z'], kind: 'partnership' }, { upstream: 'a', downstream: 'a' }] }), [
    'relationship 1: needs an upstream and a downstream, or two contexts it is between',
    'relationship 2: needs an upstream and a downstream, or two contexts it is between',
    'relationship 3: has an upstream and a downstream, so it is not between equals',
    'relationship a->z: no context "z"',
    'relationship a--z: no context "z"',
    'relationship a->a: a context is not upstream of itself',
  ]);
});

test('each kind and pattern is one the notation knows, in the place it belongs', () => {
  assert.deepEqual(problems({ contexts: two, relationships: [
    { upstream: 'a', downstream: 'b', kind: 'partnership', upstreamPatterns: ['anticorruption layer'], downstreamPatterns: 'conformist' },
    { between: ['a', 'b'], kind: 'customer-supplier', upstreamPatterns: ['open host service'] },
    { between: ['a', 'b'] },
  ] }), [
    'relationship a->b: kind "partnership" is not customer-supplier; a partnership or a shared kernel is between equals',
    'relationship a->b: upstreamPatterns "anticorruption layer" is not one of open host service, published language',
    'relationship a->b: downstreamPatterns must be a list',
    'relationship a--b: kind "customer-supplier" is not one of partnership, shared kernel',
    'relationship a--b: a relationship between equals has no upstreamPatterns',
    'relationship a--b: no kind; say partnership or shared kernel',
  ]);
});

test('a downstream cannot both conform and protect itself, and a conformist is not a customer', () => {
  assert.deepEqual(problems({ contexts: two, relationships: [{ upstream: 'a', downstream: 'b', downstreamPatterns: ['conformist', 'anticorruption layer'] }] }), [
    'relationship a->b: a downstream either conforms or keeps an anticorruption layer, not both',
  ]);
  assert.deepEqual(problems({ contexts: two, relationships: [{ upstream: 'a', downstream: 'b', kind: 'customer-supplier', downstreamPatterns: ['conformist'] }] }), [
    'relationship a->b: a conformist takes what it is given, so it is not a customer',
  ]);
});

test('the same relationship written twice is reported, whichever way round equals are written', () => {
  assert.deepEqual(problems({ contexts: two, relationships: [{ upstream: 'a', downstream: 'b' }, { upstream: 'a', downstream: 'b', tone: 'scary' }, { between: ['a', 'b'], kind: 'partnership' }, { between: ['b', 'a'], kind: 'partnership' }] }), [
    'relationship a->b: written twice',
    'relationship b--a: written twice',
  ]);
});

test('a context related to nothing is remarked on', () => {
  assert.deepEqual(remarks({ contexts: [...SPEC.contexts, { id: 'docs', name: 'Docs site' }] }), ['Docs site is related to nothing on the map']);
});

test('contexts that are each upstream of the other are remarked on', () => {
  assert.deepEqual(remarks({ relationships: [...SPEC.relationships, { upstream: 'planning', downstream: 'work' }] }), ['Influence runs in a ring: Work items → Planning → Work items']);
});

test('a context that takes an outside model without saying how is remarked on', () => {
  const bare = SPEC.relationships.map(each => (each.upstream === 'jira' ? { ...each, downstreamPatterns: undefined } : each));
  assert.deepEqual(remarks({ relationships: bare }), ['Work items is downstream of Jira, which is not ours, and says neither that it conforms nor that it keeps an anticorruption layer']);
});

test('a core context that conforms is remarked on', () => {
  const conforming = SPEC.relationships.map(each => (each.upstream === 'jira' ? { ...each, downstreamPatterns: ['conformist'] } : each));
  assert.deepEqual(remarks({ relationships: conforming }), ['Work items is core, yet conforms to Jira\'s model']);
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
