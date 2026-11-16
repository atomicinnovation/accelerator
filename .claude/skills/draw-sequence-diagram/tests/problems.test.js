const test = require('node:test');
const assert = require('node:assert/strict');

const notation = require('../notation.js');

const SPEC = {
  id: 'start-up', kind: 'sequence-diagram', title: 'Start-up claims before any lookup',
  participants: [{ id: 'user', kind: 'actor', label: 'Operator' }, { id: 'launcher' }, { id: 'claims', status: 'new' }, { id: 'disk', label: 'Disk' }],
  steps: [
    { from: 'user', to: 'launcher', label: 'accelerator vcs status', steps: [
      { from: 'launcher', to: 'claims', label: 'claim(version)', reply: 'Fresh', steps: [
        { from: 'claims', to: 'disk', label: 'lstat', reply: 'mtime' },
        { fragment: 'opt', guard: 'older than a tenth of the window', steps: [{ from: 'claims', to: 'disk', label: 'write claim', kind: 'async' }] },
      ] },
      { fragment: 'alt', operands: [
        { guard: 'cached', steps: [{ from: 'launcher', to: 'launcher', label: 'exec' }] },
        { guard: 'else', steps: [{ note: 'Cold path: fetch, then prune.', over: ['launcher', 'disk'] }] },
      ] },
    ] },
  ],
};
const problems = steps => notation.problemsIn({ ...SPEC, steps });
const call = (from, to, extra = {}) => ({ from, to, label: 'call', ...extra });

test('it draws diagrams of kind sequence-diagram', () => {
  assert.equal(notation.kind, 'sequence-diagram');
});

test('a sound sequence has no problems', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
});

test('a sequence needs participants and steps', () => {
  assert.deepEqual(notation.problemsIn({ ...SPEC, participants: [] }), ['no participants']);
  assert.deepEqual(problems([]), ['no steps']);
  assert.deepEqual(problems(undefined), ['no steps']);
});

test('a participant needs an id of its own, a known kind and sound marks', () => {
  const participants = [{ label: 'x' }, { id: 'a' }, { id: 'a' }, { id: 'b', kind: 'robot', status: 'fresh' }];
  assert.deepEqual(notation.problemsIn({ ...SPEC, participants, steps: [call('a', 'b')] }), [
    'participant 1: no id',
    'participant a: id is used twice',
    'participant b: kind "robot" is not one of participant, actor',
    'participant b: status "fresh" is not one of new, changed, removed, existing',
  ]);
});

test('a step is a message, a fragment or a note, and nothing else', () => {
  assert.deepEqual(problems([{ label: 'floating' }, 'text']), [
    'step 1: is not a message (from, to), a fragment or a note',
    'step 2: is not a message (from, to), a fragment or a note',
  ]);
});

test('a message runs between participants the sequence has, and says what is sent', () => {
  assert.deepEqual(problems([call('launcher', 'nowhere'), { from: 'launcher', to: 'disk' }]), [
    'step 1: no participant "nowhere"',
    'step 2: no label',
  ]);
});

test('a message is a call unless it says otherwise, in a word the notation has', () => {
  assert.deepEqual(problems([call('launcher', 'disk', { kind: 'sync' }), call('launcher', 'disk', { kind: 'async' }), call('launcher', 'disk', { kind: 'reply' })]), [
    'step 3: kind "reply" is not one of sync, async, create, destroy; a reply belongs to its call, as "reply"',
  ]);
});

test('only a call has a reply', () => {
  assert.deepEqual(problems([call('launcher', 'disk', { kind: 'async', reply: 'done' }), call('launcher', 'disk', { reply: 7 })]), [
    'step 1: only a synchronous call has a reply',
    'step 2: reply must be text',
  ]);
});

test('problems inside a call are found, and named by where they are', () => {
  assert.deepEqual(problems([call('user', 'launcher', { steps: [call('launcher', 'disk'), call('launcher', 'ghost')] })]), [
    'step 1.2: no participant "ghost"',
  ]);
});

test('what happens during a call is a list of steps', () => {
  assert.deepEqual(problems([call('user', 'launcher', { steps: 'then things happen' })]), ['step 1: steps must be a list']);
});

test('a creation makes a participant that has taken no part yet', () => {
  const created = { from: 'launcher', to: 'claims', kind: 'create' };
  assert.deepEqual(problems([created]), []);
  assert.deepEqual(problems([call('launcher', 'claims'), created]), ['step 2: claims cannot be created after it has taken part']);
  assert.deepEqual(problems([created, created]), ['step 2: claims cannot be created after it has taken part']);
  assert.deepEqual(problems([{ from: 'launcher', to: 'launcher', kind: 'create' }]), ['step 1: a participant cannot create itself']);
});

test('nothing reaches a participant before it is created', () => {
  assert.deepEqual(problems([call('launcher', 'disk', { steps: [call('disk', 'claims')] }), { from: 'launcher', to: 'claims', kind: 'create' }]), [
    'step 2: claims cannot be created after it has taken part',
  ]);
});

test('nothing reaches a participant after it is destroyed', () => {
  const destroyed = { from: 'launcher', to: 'claims', kind: 'destroy' };
  assert.deepEqual(problems([destroyed]), []);
  assert.deepEqual(problems([destroyed, call('launcher', 'claims')]), ['step 2: claims has been destroyed']);
  assert.deepEqual(problems([destroyed, call('claims', 'disk')]), ['step 2: claims has been destroyed']);
});

test('a fragment has an operator the notation draws', () => {
  assert.deepEqual(problems([{ fragment: 'critical', steps: [call('launcher', 'disk')] }]), [
    'step 1: fragment "critical" is not one of alt, opt, loop, par, break',
  ]);
});

test('a fragment with one operand holds steps; one with several holds operands', () => {
  assert.deepEqual(problems([{ fragment: 'loop', guard: 'each tree' }, { fragment: 'opt', steps: [] }, { fragment: 'alt', steps: [call('launcher', 'disk')] }, { fragment: 'par', operands: [{ steps: [call('launcher', 'disk')] }] }, { fragment: 'loop', operands: [] }]), [
    'step 1: a loop fragment needs steps',
    'step 2: an opt fragment needs steps',
    'step 3: an alt fragment needs operands, each with its steps',
    'step 4: a par fragment needs at least two operands',
    'step 5: a loop fragment needs steps',
  ]);
});

test('each operand needs steps, and problems in them are named by operand', () => {
  assert.deepEqual(problems([{ fragment: 'alt', operands: [{ guard: 'cached' }, { guard: 'else', steps: [call('launcher', 'ghost')] }] }]), [
    'step 1, operand 1: no steps',
    'step 1, operand 2, step 1: no participant "ghost"',
  ]);
});

test('a note says something, over participants the sequence has', () => {
  assert.deepEqual(problems([{ note: 'fine', over: ['launcher'] }, { note: '', over: ['launcher'] }, { note: 'x' }, { note: 'x', over: ['ghost'] }, { note: 'x', over: ['launcher'], tone: 'loud' }]), [
    'step 2: a note needs text',
    'step 3: a note needs the participants it is over',
    'step 4: no participant "ghost"',
    'step 5: tone "loud" is not one of risk, ok',
  ]);
});

test('whether messages are numbered is yes or no', () => {
  assert.deepEqual(notation.problemsIn({ ...SPEC, numbered: 'no' }), ['numbered must be true or false']);
});

test('a participant that takes no part is remarked on', () => {
  assert.deepEqual(notation.remarksOn(SPEC), []);
  assert.deepEqual(notation.remarksOn({ ...SPEC, steps: [call('user', 'launcher')] }), ['claims and Disk take no part in the sequence']);
  assert.deepEqual(notation.remarksOn({ ...SPEC, participants: SPEC.participants.slice(0, 3), steps: [call('user', 'launcher')] }), ['claims takes no part in the sequence']);
});
