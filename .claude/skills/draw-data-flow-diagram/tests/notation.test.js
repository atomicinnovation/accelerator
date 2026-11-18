const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const notation = require('../notation.js');
const { kitWith } = require('../../diagram-kit/lib/index.js');

const SPEC = {
  id: 'sign-in', kind: 'data-flow-diagram', title: 'A password crosses two boundaries before it is checked',
  externals: [{ id: 'user', name: 'User', link: 'docs/users.md:1' }],
  processes: [
    { id: 'web', name: 'Serve the sign-in form', status: 'changed' },
    { id: 'auth', name: 'Check the password', tone: 'risk', link: 'src/auth.rs:12' },
  ],
  stores: [{ id: 'accounts', name: 'Accounts', status: 'existing' }],
  boundaries: [
    { id: 'ours', name: 'Our network', holds: ['web', 'inner'] },
    { id: 'inner', name: 'Private subnet', holds: ['auth', 'accounts'], tone: 'risk' },
  ],
  flows: [
    { from: 'user', to: 'web', data: 'password', tone: 'risk', link: 'src/web.rs:40' },
    { from: 'web', to: 'auth', data: 'password' },
    { from: 'accounts', to: 'auth', data: 'password hash' },
    { from: 'auth', to: 'web', data: 'session', status: 'new' },
    { from: 'web', to: 'user', data: 'session cookie' },
  ],
};
const CROSSINGS = '4 flows cross a trust boundary: User → Serve the sign-in form (password), Serve the sign-in form → Check the password (password), '
  + 'Check the password → Serve the sign-in form (session) and Serve the sign-in form → User (session cookie)';
const scene = notation.sceneOf(SPEC);
const everyNode = nodes => nodes.flatMap(each => [each, ...everyNode(each.children || [])]);
const node = id => everyNode(scene.nodes).find(each => each.id === id);
const edge = id => scene.edges.find(each => each.id === id);
const problems = change => notation.problemsIn({ ...SPEC, ...change });
const remarks = change => notation.remarksOn({ ...SPEC, ...change });
const bare = { externals: [{ id: 'e', name: 'E' }], processes: [{ id: 'p', name: 'P' }, { id: 'q', name: 'Q' }], stores: [{ id: 's', name: 'S' }], boundaries: [] };

test('it draws diagrams of kind data-flow-diagram', () => {
  assert.equal(notation.kind, 'data-flow-diagram');
});

test('a sound diagram has no problems, and only its boundary crossings to remark on', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
  assert.deepEqual(notation.remarksOn(SPEC), [CROSSINGS]);
});

test('a process is a bubble, a store an open box and an external entity a rectangle', () => {
  assert.deepEqual([node('auth').shape, node('auth').label, node('auth').tone, node('auth').link], ['bubble', 'Check the password', 'risk', 'src/auth.rs:12']);
  assert.deepEqual([node('accounts').shape, node('accounts').label, node('accounts').status], ['store', 'Accounts', 'existing']);
  assert.deepEqual([node('user').shape, node('user').label, node('user').link], ['external', 'User', 'docs/users.md:1']);
});

test('a trust boundary is a dashed frame around what it holds, boundaries within it included', () => {
  assert.deepEqual(scene.nodes.map(each => each.id), ['ours', 'user']);
  assert.deepEqual([node('ours').shape, node('ours').label, node('ours').children.map(each => each.id)], ['boundary', 'Our network', ['web', 'inner']]);
  assert.deepEqual([node('inner').tone, node('inner').children.map(each => each.id)], ['risk', ['auth', 'accounts']]);
});

test('a flow is a solid arrow named for the data it carries', () => {
  const sent = edge('user->web');
  assert.deepEqual([sent.from, sent.to, sent.label, sent.line, sent.head, sent.tone, sent.link], ['user', 'web', 'password', 'solid', 'filled', 'risk', 'src/web.rs:40']);
  assert.equal(edge('auth->web').status, 'new');
});

test('two flows the same way between the same two each keep an id of their own', () => {
  const twice = notation.sceneOf({ ...SPEC, flows: [...SPEC.flows, { from: 'user', to: 'web', data: 'one-time code' }] });
  assert.deepEqual(twice.edges.filter(each => each.from === 'user').map(each => each.id), ['user->web', 'user->web#2']);
});

test('a flow that returns data to somewhere earlier flows have already left is laid out against the rest', () => {
  assert.deepEqual(scene.edges.filter(each => each.against).map(each => each.id), ['auth->web', 'web->user']);
  assert.equal(edge('user->web').against, undefined);
});

test('data flows from left to right unless the diagram says down', () => {
  assert.equal(scene.direction, 'right');
  assert.equal(notation.sceneOf({ ...SPEC, direction: 'down' }).direction, 'down');
});

test('a diagram needs a process, and every element an id of its own and a name', () => {
  assert.deepEqual(problems({ processes: [] }), ['no processes; a data flow diagram shows what is done to data']);
  assert.deepEqual(problems({ ...bare, externals: [{ name: 'x' }, { id: 'p', name: 'Again' }], stores: [{ id: 's' }], flows: [], direction: 'up' }), [
    'direction "up" is not one of right, down',
    'external 1: no id',
    'process p: id is used twice',
    'store s: no name',
  ]);
});

test('an element is marked only in the ways the kit knows', () => {
  assert.deepEqual(problems({ ...bare, processes: [{ id: 'p', name: 'P', status: 'fresh', tone: 'scary', link: 4 }], flows: [] }), [
    'process p: status "fresh" is not one of new, changed, removed, existing',
    'process p: tone "scary" is not one of risk, ok',
    'process p: link must be text',
  ]);
});

test('a boundary has an id, a name and holds things the diagram has, each in one boundary only', () => {
  assert.deepEqual(problems({ ...bare, flows: [], boundaries: [
    { name: 'x', holds: ['p'] }, { id: 'a', holds: ['p'] }, { id: 'b', name: 'B' }, { id: 'c', name: 'C', holds: ['p', 'zz'] }, { id: 'd', name: 'D', holds: ['p'], status: 'fresh' },
  ] }), [
    'boundary 1: no id',
    'boundary a: no name',
    'boundary b: holds nothing',
    'boundary c: nothing called "zz"',
    'boundary d: p is already inside boundary c',
    'boundary d: status "fresh" is not one of new, changed, removed, existing',
  ]);
});

test('a boundary cannot be inside itself', () => {
  assert.deepEqual(problems({ ...bare, flows: [], boundaries: [{ id: 'a', name: 'A', holds: ['b', 'p'] }, { id: 'b', name: 'B', holds: ['a'] }] }), [
    'boundary a: is inside itself',
    'boundary b: is inside itself',
  ]);
  assert.deepEqual(problems({ ...bare, flows: [], boundaries: [{ id: 'a', name: 'A', holds: ['a'] }] }), ['boundary a: is inside itself']);
});

test('a flow joins two elements the diagram has, and names its data', () => {
  assert.deepEqual(problems({ ...bare, boundaries: [{ id: 'b', name: 'B', holds: ['p'] }], flows: [
    { from: 'e' }, { from: 'e', to: 'zz', data: 'x' }, { from: 'e', to: 'b', data: 'x' }, { from: 'p', to: 'p', data: 'x' }, { from: 'e', to: 'p' }, { from: 'e', to: 'p', data: 'x', tone: 'scary' },
    { from: 'e', to: 'p', data: 'x' },
  ] }), [
    'flow 1: no to',
    'flow e->zz: nothing called "zz"',
    'flow e->b: b is a boundary; a flow joins elements',
    'flow p->p: a flow does not return to where it began',
    'flow e->p: no data; name what flows',
    'flow e->p: tone "scary" is not one of risk, ok',
    'flow e->p: written twice',
  ]);
});

test('a flow that passes through no process is remarked on', () => {
  assert.deepEqual(notation.remarksOn({ ...bare, flows: [{ from: 'e', to: 'p', data: 'x' }, { from: 'p', to: 's', data: 'x' }, { from: 'p', to: 'q', data: 'x' }, { from: 'q', to: 'e', data: 'y' }, { from: 'e', to: 's', data: 'z' }] }),
    ['E → S (z): neither end is a process, and data moves only through one']);
});

test('a process that makes data from nothing, or swallows it, is remarked on', () => {
  assert.deepEqual(notation.remarksOn({ ...bare, flows: [{ from: 'p', to: 's', data: 'x' }, { from: 'e', to: 'q', data: 'y' }] }), [
    'P receives no data, so it makes what it sends from nothing',
    'Q sends no data on, so what it receives goes nowhere',
  ]);
});

test('an element joined to nothing is remarked on', () => {
  assert.deepEqual(notation.remarksOn({ ...bare, flows: [{ from: 'e', to: 'p', data: 'x' }, { from: 'p', to: 'e', data: 'y' }] }), ['Q and S are joined to nothing in the diagram']);
});

test('a diagram with no boundaries has no crossings to remark on, and one crossing is counted as one', () => {
  assert.deepEqual(remarks({ boundaries: [] }), []);
  assert.deepEqual(remarks({ boundaries: [{ id: 'inner', name: 'Private subnet', holds: ['auth', 'accounts', 'web'] }], flows: SPEC.flows.slice(1) }),
    ['1 flow crosses a trust boundary: Serve the sign-in form → User (session cookie)']);
});

const EXAMPLES = path.join(__dirname, '..', 'examples');
for (const file of fs.readdirSync(EXAMPLES).filter(name => name.endsWith('.json'))) {
  test(`the example ${file} is sound, breaks no rule and lays out`, async () => {
    const kit = kitWith(), example = JSON.parse(fs.readFileSync(path.join(EXAMPLES, file), 'utf8'));
    assert.deepEqual(kit.problemsIn(example), []);
    assert.deepEqual(kit.remarksOn(example).filter(remark => !/cross(es)? a trust boundary/.test(remark)), []);
    assert.equal((await kit.laidOut(example)).edges.every(each => each.points.length >= 2), true);
  });
}
