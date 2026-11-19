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

const LINKED = {
  design: {
    decisions: [{ id: 'd-top', decision: 'One root', carriedBy: ['claims-s2', 'identity-s1', 'nowhere-s9'] }],
    abstractions: [{ id: 'a-root', name: 'Root', carriedBy: ['claims-s1'] }],
  },
  chapters: [
    { id: 'identity', stops: [{ id: 'identity-s1' }, { id: 'identity-s2', carries: ['d-top'] }] },
    {
      id: 'claims',
      decisions: [{ id: 'd-claims', decision: 'Claim by heartbeat' }],
      abstractions: [{ id: 'a-fresh', name: 'Fresh' }],
      invariants: [{ id: 'claims-i1', text: 'Holds.' }],
      stops: [
        { id: 'claims-s1', carries: ['claims-i1', 'a-fresh'] },
        { id: 'claims-s2', carries: ['d-claims', 'a-fresh'] },
      ],
    },
  ],
};
const linked = StopEvidence.index(LINKED);
const linkedStop = id => LINKED.chapters.flatMap(chapter => chapter.stops).find(each => each.id === id);

test('an item that names the stops carrying it is shown at them as if they carried it', () => {
  assert.deepEqual(linked.carriedBy(linkedStop('identity-s1')).map(each => each.id), ['d-top']);
  assert.deepEqual(linked.carriedBy(linkedStop('claims-s1')).map(each => each.id), ['claims-i1', 'a-fresh', 'a-root']);
});

test('the stops that show an item merge both directions, once each, in tour order', () => {
  assert.deepEqual(linked.shownAt('d-top'), ['identity-s1', 'identity-s2', 'claims-s2']);
});

test('a stop an item names but the tour does not have is ignored', () => {
  assert.ok(!linked.shownAt('d-top').includes('nowhere-s9'));
});

test('a decision or abstraction is introduced at the first stop that shows it', () => {
  assert.deepEqual(linked.introducedAt(linkedStop('identity-s1')).map(each => each.id), ['d-top']);
  assert.deepEqual(linked.introducedAt(linkedStop('claims-s1')).map(each => each.id), ['a-root', 'a-fresh']);
  assert.deepEqual(linked.introducedAt(linkedStop('claims-s2')).map(each => each.id), ['d-claims']);
  assert.deepEqual(linked.introducedAt(linkedStop('identity-s2')), []);
});
