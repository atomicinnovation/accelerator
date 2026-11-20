const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const ReviewNotes = require(path.join(__dirname, '..', 'assets', 'review-notes.js'));

const KINDS = {
  decision: { opts: [['approve', 'Approve'], ['request-changes', 'Request changes']] },
  design: { opts: [['agree', 'Agree'], ['question', 'Question'], ['disagree', 'Disagree']] },
  finding: { opts: [['block', 'Block merge'], ['fix-now', 'Fix in PR'], ['reject', 'Reject']] },
  stop: { opts: [['ok', 'Read'], ['issue', 'Issue']] },
};
const HEAD = 'c0ffee0000';
const RECORDS = [
  { target: 'decision', kind: 'decision', status: 'approve', note: 'Ship it.', head: HEAD },
  { target: 'notes:general', kind: 'notes', note: 'Ask about the lock.\nAnd the clock.', label: 'General notes' },
  { target: 'stop:claims-s1', kind: 'stop', status: 'ok', label: 'Claims 1.1', where: 'src/claims.rs:1-3', head: HEAD },
  { target: 'design:d-root', kind: 'design', status: 'disagree', note: 'Two roots.', label: 'Design decision: One root', head: HEAD },
  { target: 'finding:R1-01', kind: 'finding', status: 'fix-now', label: 'R1-01', head: 'beef000000' },
  { target: 'sim:run-3', kind: 'sim', status: 'odd', label: 'Simulator run 3' },
  { target: 'lines:src/claims.rs', kind: 'lines', status: 'marked', lines: ['new:2'] },
  { target: 'own:claims-1', kind: 'finding', status: 'block', label: 'Old observation' },
  { target: 'file:src/x.rs', kind: 'file', status: '', note: '' },
];
const RENAMES = { 'own:claims-1': 'finding:N-claims-1' };
const entries = ReviewNotes.entries(RECORDS, RENAMES);
const markdown = ReviewNotes.markdown(entries, { number: 7, head: HEAD, kinds: KINDS, verdict: RECORDS[0], linesRead: '\n## Lines read\n\n3 of 9 changed lines read.\n' });
const sections = () => markdown.split('\n').filter(line => line.startsWith('## ')).map(line => line.slice(3));
const items = () => markdown.split('\n').filter(line => line.startsWith('- ['));

test('the Markdown opens with the reviewer\'s general notes', () => {
  assert.equal(sections()[0], 'General notes');
  assert.match(markdown, /## General notes\n\nAsk about the lock\.\nAnd the clock\.\n/);
});

test('without general notes there is no general notes section', () => {
  const without = ReviewNotes.markdown(ReviewNotes.entries(RECORDS.filter(r => r.kind !== 'notes'), {}), { number: 7, head: HEAD, kinds: KINDS });

  assert.doesNotMatch(without, /General notes/);
});

test('the verdict heads the notes, with its own note', () => {
  assert.match(markdown, /^# PR 7 review notes\n\nHead: c0ffee0\n\nVerdict: Approve\n\nShip it\.\n/);
});

test('a record of a kind with no section of its own goes under Other, so none is dropped', () => {
  assert.ok(sections().includes('Other'));
  assert.ok(items().some(line => line.includes('Simulator run 3')));
});

test('every entry but the general notes is one item in the Markdown', () => {
  assert.equal(items().length, entries.filter(entry => entry.section !== 'General notes').length);
  assert.equal(items().length, 4);
});

test('sections keep their order, with Other after the known ones and the lines read last', () => {
  assert.deepEqual(sections(), ['General notes', 'Design decisions', 'Findings', 'Tour stops', 'Other', 'Lines read']);
});

test('an item says its verdict, its label, where it is and its note', () => {
  assert.ok(items().includes('- [Disagree] Design decision: One root'));
  assert.match(markdown, /- \[Read\] Claims 1\.1 \(src\/claims\.rs:1-3\)\n/);
  assert.match(markdown, /Design decision: One root\n  Two roots\.\n/);
});

test('a record from before the pull request moved is flagged', () => {
  assert.ok(items().some(line => line.startsWith('- [Fix in PR] R1-01 — recorded at beef000, before the pull request moved')));
});

test('a record whose target was renamed is left out', () => {
  assert.ok(!entries.some(entry => entry.target === 'own:claims-1'));
});

test('the verdict, line marks and empty records are not entries', () => {
  const targets = entries.map(entry => entry.target);

  assert.ok(!targets.includes('decision'));
  assert.ok(!targets.includes('lines:src/claims.rs'));
  assert.ok(!targets.includes('file:src/x.rs'));
});

test('an entry keeps what its control needs to be drawn again', () => {
  const stop = entries.find(entry => entry.target === 'stop:claims-s1');

  assert.deepEqual(
    { kind: stop.kind, label: stop.label, where: stop.where, section: stop.section },
    { kind: 'stop', label: 'Claims 1.1', where: 'src/claims.rs:1-3', section: 'Tour stops' },
  );
});

test('an undecided pull request says so', () => {
  assert.match(ReviewNotes.markdown([], { number: 7, kinds: KINDS }), /^# PR 7 review notes\n\nVerdict: undecided\n$/);
});

test('the sections are listed in the order the Markdown gives them', () => {
  assert.deepEqual(
    ReviewNotes.SECTIONS.filter(title => sections().includes(title)),
    sections().filter(title => !['General notes', 'Lines read'].includes(title)),
  );
  assert.equal(ReviewNotes.SECTIONS.at(-1), 'Other');
});
