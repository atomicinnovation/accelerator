const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const Readiness = require(path.join(__dirname, '..', 'assets', 'readiness.js'));

const gateOf = (id, content, records = {}, coverage = {}) => Readiness
  .gates(content, target => records[target] || {}, filePath => coverage[filePath] || null)
  .find(gate => gate.id === id);
const stateOf = (...args) => (gateOf(...args) || {}).state;

test('a gate whose content is absent is left out', () => {
  assert.deepEqual(Readiness.gates({}, () => ({}), () => null), []);
});

test('every gate says what it is, how it stands, what is outstanding and where to look', () => {
  const gate = gateOf('design', { design: { decisions: [{ id: 'd-a', decision: 'A' }] } });

  assert.deepEqual(Object.keys(gate).sort(), ['detail', 'id', 'jump', 'outstanding', 'state', 'title']);
  assert.deepEqual(gate.outstanding, [{ target: 'design:d-a', label: 'A' }]);
  assert.deepEqual(gate.jump, { route: 'decisions', filter: { verdict: 'none' } });
});

test('CI: a failing check is not met, and all passing or skipped is met', () => {
  assert.equal(stateOf('ci', { ci: { checks: [{ name: 'a', status: 'pass' }, { name: 'b', status: 'fail' }] } }), 'not-met');
  assert.equal(stateOf('ci', { ci: { checks: [{ name: 'a', status: 'pass' }, { name: 'b', status: 'skipped' }] } }), 'met');
  assert.equal(stateOf('ci', { ci: { checks: [{ name: 'a', status: 'cancelled' }] } }), 'needs-you');
});

const DECISIONS = { design: { decisions: [{ id: 'd-a', decision: 'A' }] }, chapters: [{ id: 'c', decisions: [{ id: 'd-b', decision: 'B' }] }] };
test('design decisions: a disagreement is not met, an unanswered one needs you, agreeing or asking is met', () => {
  assert.equal(stateOf('design', DECISIONS, { 'design:d-a': { status: 'disagree' } }), 'not-met');
  assert.equal(stateOf('design', DECISIONS, { 'design:d-a': { status: 'agree' } }), 'needs-you');
  assert.equal(stateOf('design', DECISIONS, { 'design:d-a': { status: 'agree' }, 'design:d-b': { status: 'question' } }), 'met');
});

const PRINCIPLES = verdicts => ({ design: { principles: verdicts.map((verdict, n) => ({ principle: `p${n}`, verdict })) } });
test('fit with the architecture: a broken principle is not met, a strained one needs you, all holding is met', () => {
  assert.equal(stateOf('architecture', PRINCIPLES(['holds', 'broken'])), 'not-met');
  assert.equal(stateOf('architecture', PRINCIPLES(['holds', 'strained'])), 'needs-you');
  assert.equal(stateOf('architecture', PRINCIPLES(['holds'])), 'met');
});

const FINDINGS = { findings: [{ id: 'R1', severity: 'major', summary: 's' }, { id: 'R2', severity: 'minor', summary: 't' }] };
test('findings: one triaged to block is not met, a serious one untriaged needs you, otherwise met', () => {
  assert.equal(stateOf('findings', FINDINGS, { 'finding:R2': { status: 'block' }, 'finding:R1': { status: 'fix-now' } }), 'not-met');
  assert.equal(stateOf('findings', FINDINGS), 'needs-you');
  assert.deepEqual(gateOf('findings', FINDINGS).outstanding.map(each => each.target), ['finding:R1']);
  assert.equal(stateOf('findings', FINDINGS, { 'finding:R1': { status: 'follow-up' } }), 'met');
});

const FILES = { files: [{ path: 'a.rs', risk: 'high' }, { path: 'b.rs', risk: 'low' }] };
test('high-risk files: one not read in full needs you, all read is met', () => {
  assert.equal(stateOf('high-risk', FILES, {}, { 'a.rs': { state: 'partly' } }), 'needs-you');
  assert.equal(stateOf('high-risk', FILES, {}, { 'a.rs': { state: 'read' } }), 'met');
  assert.equal(stateOf('high-risk', { files: [{ path: 'b.rs', risk: 'low' }] }), undefined);
});

test('high-risk files open All files on the high-risk files still to read', () => {
  assert.deepEqual(gateOf('high-risk', FILES, {}, { 'a.rs': { state: 'partly' } }).jump, {
    route: 'files', filter: { risk: ['high'], progress: ['unread', 'partial'] },
  });
});

test('high-risk files: before the code loads, a file verdict stands for reading it', () => {
  assert.equal(stateOf('high-risk', FILES, { 'file:a.rs': { status: 'ok' } }), 'met');
});

const INVARIANTS = holds => ({ chapters: [{ id: 'c', invariants: [
  { id: 'c-i1', text: 'pinned', holds: 'yes', pinnedBy: ['t.rs::a'] },
  { id: 'c-i2', text: 'doubtful', holds, pinnedBy: ['t.rs::b'] },
] }] });
test('invariants: one you marked broken is not met, a doubtful or unpinned one you have not confirmed needs you', () => {
  assert.equal(stateOf('invariants', INVARIANTS('yes'), { 'inv:c-i1': { status: 'issue' } }), 'not-met');
  assert.equal(stateOf('invariants', INVARIANTS('partly')), 'needs-you');
  assert.equal(stateOf('invariants', INVARIANTS('partly'), { 'inv:c-i2': { status: 'ok' } }), 'met');
  assert.equal(stateOf('invariants', { chapters: [{ id: 'c', invariants: [{ id: 'c-i3', text: 'unpinned', holds: 'yes' }] }] }), 'needs-you');
  assert.equal(stateOf('invariants', INVARIANTS('yes')), 'met');
});

test('criteria: a test the build could not resolve needs you, all resolved is met', () => {
  const criteria = hits => ({ acs: [{ n: 1, label: 'x', tests: [{ ref: 'a', hits }] }] });

  assert.equal(stateOf('criteria', criteria([])), 'needs-you');
  assert.equal(stateOf('criteria', criteria([{ name: 'a', file: 't.rs', line: 1 }])), 'met');
});

test('breaking changes: one without a changelog entry is not met, all documented is met', () => {
  assert.equal(stateOf('breaking', { breaking: [{ title: 'a', changelog: 'CHANGELOG.md:3' }, { title: 'b' }] }), 'not-met');
  assert.equal(stateOf('breaking', { breaking: [{ title: 'a', changelog: 'CHANGELOG.md:3' }] }), 'met');
});

const ACCEPTED = { acceptedRisks: [{ id: 'ar1', title: 'Evicts old', findings: ['R1'] }, { id: 'ar2', title: 'Other', findings: [] }], findings: [{ id: 'R1', severity: 'minor' }] };
test('accepted risks: a finding that re-raises one and is untriaged needs you, otherwise met', () => {
  assert.equal(stateOf('accepted-risks', ACCEPTED), 'needs-you');
  assert.equal(stateOf('accepted-risks', ACCEPTED, { 'finding:R1': { status: 'reject' } }), 'met');
});

const LEDGER = { ledger: [{ id: 'L-1', op: 'remove', subject: 'a' }, { id: 'L-2', op: 'remove', subject: 'b' }], ledgerMeta: { title: 'Removal ledger' } };
test('ledger: an unsafe row is not met, an unchecked one needs you, all safe is met', () => {
  assert.equal(stateOf('ledger', LEDGER, { 'ledger:L-1': { status: 'issue' } }), 'not-met');
  assert.equal(stateOf('ledger', LEDGER, { 'ledger:L-1': { status: 'ok' } }), 'needs-you');
  assert.equal(stateOf('ledger', LEDGER, { 'ledger:L-1': { status: 'ok' }, 'ledger:L-2': { status: 'ok' } }), 'met');
  assert.equal(gateOf('ledger', LEDGER).title, 'Removal ledger');
});

const CHAPTERS = { chapters: [{ id: 'a', title: 'A' }, { id: 'b', title: 'B' }] };
test('chapters: a blocking one is not met, one not signed off needs you, all signed off is met', () => {
  assert.equal(stateOf('chapters', CHAPTERS, { 'chapter:a': { status: 'blocking' } }), 'not-met');
  assert.equal(stateOf('chapters', CHAPTERS, { 'chapter:a': { status: 'understood' }, 'chapter:b': { status: 'concerns' } }), 'needs-you');
  assert.equal(stateOf('chapters', CHAPTERS, { 'chapter:a': { status: 'understood' }, 'chapter:b': { status: 'understood' } }), 'met');
});

test('the gates come in a fixed order', () => {
  const all = { ...DECISIONS, ...FINDINGS, ...FILES, ...LEDGER, ...CHAPTERS, ci: { checks: [{ name: 'a', status: 'pass' }] } };

  assert.deepEqual(Readiness.gates(all, () => ({}), () => null).map(gate => gate.id), ['ci', 'design', 'findings', 'high-risk', 'ledger', 'chapters']);
});
