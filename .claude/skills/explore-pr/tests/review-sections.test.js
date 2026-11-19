const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const ReviewSections = require(path.join(__dirname, '..', 'assets', 'review-sections.js'));

const FULL = {
  design: { summary: 'One root.', decisions: [{ id: 'd-root', decision: 'One root' }] },
  keyAbstractions: [{ item: { id: 'a-root', name: 'Root' }, chapters: ['claims'] }],
  chapters: [{
    id: 'claims',
    domain: [{ term: 'claim', meaning: 'a hold' }],
    invariants: [{ id: 'claims-i1', text: 'Holds.' }],
    stops: [],
  }],
  commits: [{ sha: 'abc' }],
  findings: [{ id: 'R1-01' }],
  acs: [{ n: 1, tests: [] }],
  ledger: [{ id: 'L-1' }],
  ledgerMeta: { title: 'Removal ledger' },
};
const groupsOf = (content, views = []) => ReviewSections.of(content, views).map(section => section.group);
const routesIn = (content, group, views = []) =>
  (ReviewSections.of(content, views).find(section => section.group === group) || { routes: [] }).routes.map(route => route.id);

test('the groups come in reading order: start, read, interact, decide, reference', () => {
  assert.deepEqual(groupsOf(FULL, [{ id: 'sim', title: 'Simulator' }]), ['Start', 'Read', 'Interact', 'Decide', 'Reference']);
});

test('a group with nothing in it is left out, so Interact needs a custom view', () => {
  assert.deepEqual(groupsOf(FULL), ['Start', 'Read', 'Decide', 'Reference']);
});

test('a custom view without a group, or with one the page does not know, goes in Interact', () => {
  const views = [{ id: 'sim', title: 'Simulator' }, { id: 'stepper', title: 'Stepper', group: 'Elsewhere' }];

  assert.deepEqual(routesIn(FULL, 'Interact', views), ['sim', 'stepper']);
});

test('a custom view that names a known group goes after that group\'s own views', () => {
  assert.deepEqual(routesIn(FULL, 'Start', [{ id: 'intro', title: 'Intro', group: 'Start' }]), ['overview', 'design', 'map', 'intro']);
});

test('start holds the overview, the design when there is one, and the attention map', () => {
  assert.deepEqual(routesIn(FULL, 'Start'), ['overview', 'design', 'map']);
  assert.deepEqual(routesIn({}, 'Start'), ['overview', 'map']);
});

test('read holds the tour, the commits and every file, in that order', () => {
  assert.deepEqual(routesIn(FULL, 'Read'), ['tour', 'commits', 'files']);
  assert.deepEqual(routesIn({}, 'Read'), ['files']);
});

test('decide holds your notes', () => {
  assert.deepEqual(routesIn({}, 'Decide'), ['notes']);
});

test('reference holds each catalogue whose content exists, the ledger last', () => {
  assert.deepEqual(routesIn(FULL, 'Reference'), ['findings', 'decisions', 'abstractions', 'invariants', 'tests', 'terms', 'ledger']);
  assert.deepEqual(routesIn({}, 'Reference'), []);
});

test('the ledger is titled as the manifest names it', () => {
  const reference = ReviewSections.of(FULL, []).find(section => section.group === 'Reference');

  assert.equal(reference.routes.find(route => route.id === 'ledger').title, 'Removal ledger');
  assert.equal(ReviewSections.of({ ledger: [{ id: 'L-1' }] }, []).at(-1).routes.at(-1).title, 'Risk ledger');
});

test('risks alone are enough for the invariants catalogue', () => {
  assert.ok(routesIn({ chapters: [{ id: 'c', risks: [{ id: 'c-r1' }] }] }, 'Reference').includes('invariants'));
});

test('a chapter decision is enough for the decisions catalogue', () => {
  assert.deepEqual(routesIn({ chapters: [{ id: 'c', decisions: [{ id: 'd-x' }] }] }, 'Reference'), ['decisions']);
});

test('a test named by a chapter, an invariant or a ledger row is enough for the tests catalogue', () => {
  const named = [
    { chapters: [{ id: 'c', tests: [{ file: 't.rs', name: 'a' }] }] },
    { chapters: [{ id: 'c', invariants: [{ id: 'c-i1', pinnedBy: ['t.rs::a'] }] }] },
    { ledger: [{ id: 'L-1', tests: ['t.rs::a'] }] },
  ];

  named.forEach(content => assert.ok(routesIn(content, 'Reference').includes('tests')));
});

test('a hash that was retired opens the route that replaced it', () => {
  assert.equal(ReviewSections.routeFor('criteria'), 'tests');
  assert.equal(ReviewSections.routeFor('findings'), 'findings');
});
