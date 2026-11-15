const test = require('node:test');
const assert = require('node:assert/strict');

const notation = require('../notation.js');

const SPEC = {
  id: 'claim-life', kind: 'state-machine', title: 'A claim is fresh, then stale, then gone',
  states: [
    { id: 'start', kind: 'initial' },
    { id: 'claimed', label: 'Claimed', status: 'new', link: 'src/claims.rs:45', states: [
      { id: 'begin', kind: 'initial' },
      { id: 'fresh', label: 'Fresh', entry: 'write mtime', do: 'protect the version' },
      { id: 'ageing', label: 'Ageing', tone: 'risk' },
    ] },
    { id: 'check', kind: 'choice' },
    { id: 'orphaned', label: 'Orphaned', exit: 'unlink the claim' },
    { id: 'end', kind: 'final' },
  ],
  transitions: [
    { from: 'start', to: 'claimed', action: 'create the claim' },
    { from: 'begin', to: 'fresh' },
    { from: 'fresh', to: 'ageing', event: 'after(CLAIM_WINDOW / 10)' },
    { from: 'ageing', to: 'fresh', event: 'start-up', action: 'refresh', status: 'new' },
    { from: 'claimed', to: 'check', event: 'prune' },
    { from: 'check', to: 'orphaned', guard: 'age > CLAIM_WINDOW' },
    { from: 'check', to: 'claimed', guard: 'else' },
    { from: 'orphaned', to: 'end', event: 'swept', link: 'src/prune.rs:540' },
  ],
};
const scene = notation.sceneOf(SPEC);
const node = (id, nodes = scene.nodes) => nodes.flatMap(each => [each, ...(each.children || [])]).find(each => each.id === id);
const edge = id => scene.edges.find(each => each.id === id);
const problems = change => notation.problemsIn({ ...SPEC, ...change });
const remarks = change => notation.remarksOn({ ...SPEC, ...change });
const plain = (...ids) => ids.map(id => ({ id }));

test('it draws diagrams of kind state-machine', () => {
  assert.equal(notation.kind, 'state-machine');
});

test('a sound machine has no problems and nothing to remark on', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
  assert.deepEqual(notation.remarksOn(SPEC), []);
});

test('each kind of state has its own shape', () => {
  assert.deepEqual(['start', 'check', 'orphaned', 'end', 'claimed'].map(id => node(id).shape), ['initial', 'choice', 'state', 'final', 'composite']);
});

test('a state is named by its label or its id, and keeps its marks and link', () => {
  assert.equal(node('orphaned').label, 'Orphaned');
  assert.equal(notation.sceneOf({ ...SPEC, states: plain('a'), transitions: [] }).nodes[0].label, 'a');
  assert.deepEqual([node('claimed').status, node('claimed').link, node('ageing').tone], ['new', 'src/claims.rs:45', 'risk']);
});

test('a state lists what it does on entry, while it lasts and on exit, in that order', () => {
  assert.deepEqual(node('fresh').activities, ['entry / write mtime', 'do / protect the version']);
  assert.deepEqual(node('orphaned').activities, ['exit / unlink the claim']);
  assert.equal(node('ageing').activities, undefined);
});

test('a state that holds others is a composite state, holding them', () => {
  assert.deepEqual(node('claimed').children.map(each => [each.id, each.shape]), [['begin', 'initial'], ['fresh', 'state'], ['ageing', 'state']]);
});

test('a transition is a solid arrow with an open head', () => {
  const drawn = edge('fresh->ageing');
  assert.deepEqual([drawn.from, drawn.to, drawn.line, drawn.head], ['fresh', 'ageing', 'solid', 'open']);
});

test('a transition is labelled event [guard] / action, with whichever parts it has', () => {
  assert.equal(edge('ageing->fresh').label, 'start-up / refresh');
  assert.equal(edge('check->orphaned').label, '[age > CLAIM_WINDOW]');
  assert.equal(edge('start->claimed').label, '/ create the claim');
  assert.equal(edge('begin->fresh').label, undefined);
  const full = notation.sceneOf({ ...SPEC, states: plain('a', 'b'), transitions: [{ from: 'a', to: 'b', event: 'tick', guard: 'armed', action: 'fire' }] });
  assert.equal(full.edges[0].label, 'tick [armed] / fire');
});

test('a transition keeps its status, tone and link', () => {
  assert.equal(edge('ageing->fresh').status, 'new');
  assert.equal(edge('orphaned->end').link, 'src/prune.rs:540');
});

test('transitions between the same states are told apart', () => {
  const twice = notation.sceneOf({ ...SPEC, states: plain('a', 'b'), transitions: [{ from: 'a', to: 'b', event: 'x' }, { from: 'a', to: 'b', event: 'y' }, { from: 'a', to: 'a', event: 'z' }] });
  assert.deepEqual(twice.edges.map(each => each.id), ['a->b', 'a->b:2', 'a->a']);
});

test('the machine runs down the page unless it says right', () => {
  assert.equal(scene.direction, 'down');
  assert.equal(notation.sceneOf({ ...SPEC, direction: 'right' }).direction, 'right');
});

test('a machine needs states', () => {
  assert.deepEqual(problems({ states: [] }), ['no states']);
});

test('a state needs an id of its own and a kind the notation draws', () => {
  assert.deepEqual(problems({ states: [{ label: 'x' }, { id: 'a' }, { id: 'b', states: [{ id: 'a' }] }, { id: 'c', kind: 'history' }], transitions: [] }), [
    'state 1: no id',
    'state a: id is used twice',
    'state c: kind "history" is not one of state, initial, final, choice',
  ]);
});

test('only a plain state holds others or does anything', () => {
  assert.deepEqual(problems({ states: [{ id: 's', kind: 'initial', states: plain('a') }, { id: 'e', kind: 'final', entry: 'log' }, { id: 't', entry: 7 }], transitions: [{ from: 's', to: 't' }, { from: 't', to: 'e' }] }), [
    'state s: an initial state cannot hold other states',
    'state e: a final state has no activities',
    'state t: entry must be text',
  ]);
});

test('a region starts in one place', () => {
  assert.deepEqual(problems({ states: [{ id: 's1', kind: 'initial' }, { id: 's2', kind: 'initial' }, { id: 'a' }, { id: 'c', states: [{ id: 'i1', kind: 'initial' }, { id: 'i2', kind: 'initial' }, { id: 'b' }] }],
    transitions: [{ from: 's1', to: 'a' }, { from: 's2', to: 'a' }, { from: 'i1', to: 'b' }, { from: 'i2', to: 'b' }] }), [
    'the machine has 2 initial states; a region starts in one place',
    'state c has 2 initial states; a region starts in one place',
  ]);
});

test('a transition runs between states the machine has', () => {
  assert.deepEqual(problems({ transitions: [...SPEC.transitions, { from: 'fresh', to: 'nowhere' }, { to: 'fresh' }] }), [
    'transition fresh->nowhere: no state "nowhere"',
    'transition 10: no from',
  ]);
});

test('nothing leaves a final state or arrives at an initial one', () => {
  assert.deepEqual(problems({ transitions: [...SPEC.transitions, { from: 'end', to: 'fresh' }, { from: 'fresh', to: 'begin' }] }), [
    'transition end->fresh: a final state has no way out',
    'transition fresh->begin: nothing arrives at an initial state',
  ]);
});

test('an initial state has one way out, with no event or guard', () => {
  const states = [{ id: 's', kind: 'initial' }, { id: 'a' }, { id: 'b' }];
  assert.deepEqual(problems({ states, transitions: [] }), ['state s: an initial state needs exactly one transition out, and has 0']);
  assert.deepEqual(problems({ states, transitions: [{ from: 's', to: 'a' }, { from: 's', to: 'b' }, { from: 'a', to: 'b' }] }), ['state s: an initial state needs exactly one transition out, and has 2']);
  assert.deepEqual(problems({ states, transitions: [{ from: 's', to: 'a', event: 'go' }, { from: 'a', to: 'b' }] }), ['transition s->a: the transition out of an initial state has no event or guard']);
});

test('a choice has a way in and at least two ways out', () => {
  const states = [{ id: 'a' }, { id: 'c', kind: 'choice' }, { id: 'b' }];
  assert.deepEqual(problems({ states, transitions: [{ from: 'a', to: 'c' }, { from: 'c', to: 'b', guard: 'x' }] }), ['state c: a choice needs at least two transitions out, and has 1']);
  assert.deepEqual(problems({ states, transitions: [{ from: 'c', to: 'a', guard: 'x' }, { from: 'c', to: 'b', guard: 'else' }] }), ['state c: a choice needs a transition in']);
});

test('marks, parts and direction outside what they may be are reported', () => {
  assert.deepEqual(problems({ direction: 'up', states: [{ id: 'a', status: 'fresh' }, { id: 'b' }], transitions: [{ from: 'a', to: 'b', tone: 'scary', event: 3 }] }), [
    'direction "up" is not one of down, right',
    'state a: status "fresh" is not one of new, changed, removed, existing',
    'transition a->b: tone "scary" is not one of risk, ok',
    'transition a->b: event must be text',
  ]);
});

test('a state nothing leads to is remarked on', () => {
  assert.deepEqual(remarks({ states: [...SPEC.states, { id: 'lost', label: 'Lost' }], transitions: [...SPEC.transitions, { from: 'lost', to: 'end' }] }), ['Lost cannot be reached']);
});

test('a state inside a composite state is reached through it, and a transition out of it leaves from any state inside', () => {
  const states = [{ id: 's', kind: 'initial' }, { id: 'outer', states: [{ id: 'i', kind: 'initial' }, { id: 'inner' }, { id: 'deep' }] }, { id: 'after' }, { id: 'e', kind: 'final' }];
  const transitions = [{ from: 's', to: 'outer' }, { from: 'i', to: 'inner' }, { from: 'outer', to: 'after', event: 'done' }, { from: 'after', to: 'e' }];
  assert.deepEqual(remarks({ states, transitions }), ['deep cannot be reached']);
});

test('a machine with no start cannot say what is reachable, and says so', () => {
  assert.deepEqual(remarks({ states: plain('a', 'b'), transitions: [{ from: 'a', to: 'b' }, { from: 'b', to: 'a' }] }), ['Nothing says where the machine starts: it has no initial state']);
});

test('a state with no way out that is not final is remarked on', () => {
  const states = [{ id: 's', kind: 'initial' }, { id: 'a' }, { id: 'stuck', label: 'Stuck' }];
  assert.deepEqual(remarks({ states, transitions: [{ from: 's', to: 'a' }, { from: 'a', to: 'stuck', event: 'fail' }] }), ['Stuck has no way out and is not final']);
});

test('a state inside a composite state that has a way out is not stuck', () => {
  const states = [{ id: 's', kind: 'initial' }, { id: 'outer', states: [{ id: 'i', kind: 'initial' }, { id: 'inner' }] }, { id: 'e', kind: 'final' }];
  assert.deepEqual(remarks({ states, transitions: [{ from: 's', to: 'outer' }, { from: 'i', to: 'inner' }, { from: 'outer', to: 'e', event: 'done' }] }), []);
});

test('a choice with more than one branch without a guard is remarked on', () => {
  const transitions = SPEC.transitions.map(each => (each.from === 'check' ? { ...each, guard: undefined } : each));
  assert.deepEqual(remarks({ transitions }), ['The choice after Claimed has 2 branches with no guard to choose between them']);
});

test('two transitions out of a state on one event, with nothing to tell them apart, are remarked on', () => {
  const transitions = [...SPEC.transitions, { from: 'fresh', to: 'end', event: 'after(CLAIM_WINDOW / 10)' }];
  assert.deepEqual(remarks({ transitions }), ['Fresh has 2 transitions on after(CLAIM_WINDOW / 10) that no guard tells apart']);
  const guarded = [...SPEC.transitions.map(each => (each.from === 'fresh' ? { ...each, guard: 'idle' } : each)), { from: 'fresh', to: 'end', event: 'after(CLAIM_WINDOW / 10)', guard: 'busy' }];
  assert.deepEqual(remarks({ transitions: guarded }), []);
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
