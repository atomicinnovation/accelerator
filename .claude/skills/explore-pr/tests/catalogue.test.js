const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const Catalogue = require(path.join(__dirname, '..', 'assets', 'catalogue.js'));

const CONTENT = {
  design: {
    decisions: [{ id: 'd-root', decision: 'One root' }],
    principles: [{ principle: 'Layers point down', verdict: 'holds' }],
    extension: [{ change: 'A new store', where: 'src/store.rs:1' }],
  },
  keyAbstractions: [{ item: { id: 'a-root', name: 'Root' }, chapters: ['claims'] }],
  chapters: [
    {
      id: 'identity',
      invariants: [{ id: 'identity-i1', text: 'One launcher.', pinnedBy: ['tests/id.rs::one_launcher'] }],
      tests: [{ file: 'tests/id.rs', name: 'one_launcher', line: 4, proves: 'One launcher at a time' }],
    },
    {
      id: 'claims',
      decisions: [{ id: 'd-heartbeat', decision: 'Claim by heartbeat' }],
      invariants: [{ id: 'claims-i1', text: 'Clock trouble only keeps.' }],
      risks: [{ id: 'claims-r1', level: 'high', text: 'A live version is pruned.' }],
      tests: ['tests/claims.rs::claims::refreshes'],
    },
  ],
  ledger: [{ id: 'L-3', tests: ['tests/claims.rs::claims::refreshes', { file: 'tests/prune.rs', name: 'keeps_live' }] }],
  acs: [{ n: 4, tests: [{ ref: 'keeps_live', hits: [{ name: 'keeps_live', file: 'tests/prune.rs', line: 12 }] }, { ref: 'missing', hits: [] }] }],
};
const catalogue = Catalogue.of(CONTENT);

test('decisions gather the design\'s and every chapter\'s, each with the chapter that declares it', () => {
  assert.deepEqual(catalogue.decisions.map(({ item, chapter }) => [item.id, chapter]), [['d-root', null], ['d-heartbeat', 'claims']]);
});

test('abstractions are the key abstractions with their chapters', () => {
  assert.deepEqual(catalogue.abstractions, CONTENT.keyAbstractions);
});

test('invariants and risks come from every chapter in tour order, each with its chapter', () => {
  assert.deepEqual(catalogue.invariants.map(({ item, chapter }) => [item.id, chapter]), [['identity-i1', 'identity'], ['claims-i1', 'claims']]);
  assert.deepEqual(catalogue.risks.map(({ item, chapter }) => [item.id, chapter]), [['claims-r1', 'claims']]);
});

test('every test named anywhere is listed once, by path and name', () => {
  assert.deepEqual(catalogue.tests.map(each => each.key), [
    'tests/id.rs::one_launcher',
    'tests/claims.rs::refreshes',
    'tests/prune.rs::keeps_live',
  ]);
});

test('each test lists what names it', () => {
  const namedBy = Object.fromEntries(catalogue.tests.map(each => [each.key, each.namedBy]));

  assert.deepEqual(namedBy['tests/id.rs::one_launcher'], ['chapter:identity', 'inv:identity-i1']);
  assert.deepEqual(namedBy['tests/claims.rs::refreshes'], ['chapter:claims', 'ledger:L-3']);
  assert.deepEqual(namedBy['tests/prune.rs::keeps_live'], ['ledger:L-3', 'criterion:4']);
});

test('a test keeps what it proves and the line it was found at', () => {
  const [launcher, , prune] = catalogue.tests;

  assert.equal(launcher.proves, 'One launcher at a time');
  assert.equal(launcher.line, 4);
  assert.equal(prune.line, 12);
});

test('a criterion\'s unresolved test is not listed', () => {
  assert.ok(!catalogue.tests.some(each => each.name === 'missing'));
});

test('principles and the next change come from the design', () => {
  assert.deepEqual(catalogue.principles, CONTENT.design.principles);
  assert.deepEqual(catalogue.extension, CONTENT.design.extension);
});

test('empty content has empty catalogues', () => {
  assert.deepEqual(Catalogue.of({}), { decisions: [], abstractions: [], invariants: [], risks: [], tests: [], principles: [], extension: [] });
});
