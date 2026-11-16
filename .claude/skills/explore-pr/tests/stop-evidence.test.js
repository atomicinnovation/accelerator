const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const StopEvidence = require(path.join(__dirname, '..', 'assets', 'stop-evidence.js'));

const CONTENT = {
  design: {
    decisions: [{ id: 'd-top', decision: 'One root for every version' }],
    abstractions: [{ id: 'a-owned', name: 'Root', chapter: 'claims' }],
  },
  chapters: [
    {
      id: 'identity',
      stops: [{ id: 'identity-s1', carries: ['d-top', 'a-claims-fresh'] }, { id: 'identity-s2' }],
    },
    {
      id: 'claims',
      decisions: [{ id: 'd-claims-heartbeat', decision: 'Claim by heartbeat' }],
      abstractions: [{ id: 'a-claims-fresh', name: '`FreshVersions`' }],
      invariants: [{ id: 'claims-i1', text: 'Clock trouble can only keep.' }],
      risks: [{ id: 'claims-r1', level: 'high', text: 'A live version is pruned.' }],
      stops: [
        { id: 'claims-s1', carries: ['d-claims-heartbeat', 'claims-i1', 'claims-r1', 'a-claims-fresh'] },
        { id: 'claims-s2', carries: ['d-claims-heartbeat', 'd-lost'] },
      ],
    },
  ],
};
const evidence = StopEvidence.index(CONTENT);
const stop = id => CONTENT.chapters.flatMap(chapter => chapter.stops).find(each => each.id === id);

test('a carried decision is named by what was decided, in the chapter that declares it', () => {
  assert.deepEqual(evidence.carriedBy(stop('claims-s2')), [
    { id: 'd-claims-heartbeat', kind: 'decision', label: 'Claim by heartbeat', chapter: 'claims' },
  ]);
});

test('an abstraction is named by its name, an invariant and a risk by their text', () => {
  assert.deepEqual(
    evidence.carriedBy(stop('claims-s1')).map(({ kind, label }) => [kind, label]),
    [
      ['decision', 'Claim by heartbeat'],
      ['invariant', 'Clock trouble can only keep.'],
      ['risk', 'A live version is pruned.'],
      ['abstraction', '`FreshVersions`'],
    ],
  );
});

test('what the design declares belongs to no chapter unless it names one', () => {
  const [decision] = evidence.carriedBy(stop('identity-s1'));
  assert.equal(decision.chapter, null);
  assert.equal(evidence.declared('a-owned').chapter, 'claims');
});

test('a stop may carry what another chapter declares', () => {
  const carried = evidence.carriedBy(stop('identity-s1'));
  assert.deepEqual(carried[1], { id: 'a-claims-fresh', kind: 'abstraction', label: '`FreshVersions`', chapter: 'claims' });
});

test('something carried that nothing declares is left out', () => {
  assert.deepEqual(evidence.carriedBy(stop('claims-s2')).map(each => each.id), ['d-claims-heartbeat']);
});

test('a stop that carries nothing has no evidence to show', () => {
  assert.deepEqual(evidence.carriedBy(stop('identity-s2')), []);
});

test('the stops that show something are listed in tour order', () => {
  assert.deepEqual(evidence.shownAt('a-claims-fresh'), ['identity-s1', 'claims-s1']);
  assert.deepEqual(evidence.shownAt('d-claims-heartbeat'), ['claims-s1', 'claims-s2']);
});

test('something no stop carries is shown nowhere', () => {
  assert.deepEqual(evidence.shownAt('a-owned'), []);
});

test('content with no design and no chapters carries nothing', () => {
  assert.deepEqual(StopEvidence.index({}).shownAt('d-top'), []);
});
