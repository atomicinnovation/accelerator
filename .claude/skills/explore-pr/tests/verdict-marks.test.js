const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const VerdictMarks = require(path.join(__dirname, '..', 'assets', 'verdict-marks.js'));

test('each item the reviewer gives a verdict on is recorded under its own target', () => {
  assert.equal(VerdictMarks.targetOf('decision', 'cap-depth'), 'design:cap-depth');
  assert.equal(VerdictMarks.targetOf('invariant', 'claims-i1'), 'inv:claims-i1');
  assert.equal(VerdictMarks.targetOf('stop', 'claims-s1'), 'stop:claims-s1');
  assert.equal(VerdictMarks.targetOf('chapter', 'claims'), 'chapter:claims');
  assert.equal(VerdictMarks.targetOf('finding', 'R1-01'), 'finding:R1-01');
  assert.equal(VerdictMarks.targetOf('ledger', 'L-1'), 'ledger:L-1');
});

test('an abstraction or a risk takes no verdict, so it has no target', () => {
  assert.equal(VerdictMarks.targetOf('abstraction', 'root'), null);
  assert.equal(VerdictMarks.targetOf('risk', 'r1'), null);
});

test('an item with no verdict yet is marked open, saying what is still to do', () => {
  assert.deepEqual(VerdictMarks.of('design:cap-depth', {}), { target: 'design:cap-depth', tone: 'open', said: 'No verdict yet' });
  assert.equal(VerdictMarks.of('stop:claims-s1', {}).said, 'Not read yet');
  assert.equal(VerdictMarks.of('chapter:claims', {}).said, 'Not signed off');
  assert.equal(VerdictMarks.of('finding:R1-01', {}).said, 'Not triaged');
  assert.equal(VerdictMarks.of('ledger:L-1', {}).said, 'Not checked');
});

test('an item with a verdict is marked in that verdict\'s tone and named by it', () => {
  assert.deepEqual(VerdictMarks.of('design:cap-depth', { status: 'agree' }), { target: 'design:cap-depth', tone: 'ok', said: 'Agree' });
  assert.deepEqual(VerdictMarks.of('inv:claims-i1', { status: 'question' }), { target: 'inv:claims-i1', tone: 'warn', said: 'Unsure' });
  assert.deepEqual(VerdictMarks.of('chapter:claims', { status: 'blocking' }), { target: 'chapter:claims', tone: 'bad', said: 'Blocking' });
  assert.deepEqual(VerdictMarks.of('finding:R1-01', { status: 'follow-up' }), { target: 'finding:R1-01', tone: 'info', said: 'Follow-up' });
  assert.deepEqual(VerdictMarks.of('finding:R1-01', { status: 'reject' }), { target: 'finding:R1-01', tone: 'none', said: 'Reject' });
});

test('a stop whose lines were all read elsewhere is marked read, naming where', () => {
  assert.deepEqual(VerdictMarks.of('stop:claims-s1', {}, ['All files']), { target: 'stop:claims-s1', tone: 'ok', said: 'Read via All files' });
});

test('a verdict on a stop outranks its lines being read elsewhere', () => {
  assert.equal(VerdictMarks.of('stop:claims-s1', { status: 'issue' }, ['All files']).said, 'Issue');
});

test('a target the page records no verdict for has no mark', () => {
  assert.equal(VerdictMarks.of('principle:layering', {}), null);
  assert.equal(VerdictMarks.of('nonsense', {}), null);
});

test('a status the kind does not know is treated as no verdict', () => {
  assert.equal(VerdictMarks.of('design:cap-depth', { status: 'ok' }).tone, 'open');
});

test('a mark is drawn by shape as well as colour, so it reads without colour', () => {
  const glyphs = ['open', 'ok', 'warn', 'bad', 'info', 'none'].map(tone => VerdictMarks.glyphOf(tone));

  assert.equal(new Set(glyphs).size, glyphs.length);
});

test('the verdict vocabulary names every option the controls offer', () => {
  assert.deepEqual(VerdictMarks.KINDS.design.opts.map(([value]) => value), ['agree', 'question', 'disagree']);
  assert.deepEqual(VerdictMarks.KINDS.stop.opts.map(([value]) => value), ['ok', 'question', 'issue']);
});
