const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const BulkVerdicts = require(path.join(__dirname, '..', 'assets', 'bulk-verdicts.js'));

const recordsOf = records => target => records[target] || {};
const RECORDS = recordsOf({
  'file:a': { status: 'ok', note: 'fine', head: 'h1', claimed: ['new:1-3'] },
  'file:b': { status: 'issue', note: 'broken here', head: 'h1', claimed: [] },
  'file:c': { status: 'skip', note: '' },
});

test('a bulk verdict writes the status to every target whose status differs', () => {
  const { writes } = BulkVerdicts.plan(RECORDS, ['file:a', 'file:c', 'file:new'], 'skip');

  assert.deepEqual(writes, [{ target: 'file:a', patch: { status: 'skip' } }, { target: 'file:new', patch: { status: 'skip' } }]);
});

test('a target already holding the status is neither written nor skipped', () => {
  const { writes, skipped } = BulkVerdicts.plan(RECORDS, ['file:c'], 'skip');

  assert.deepEqual(writes, []);
  assert.deepEqual(skipped, []);
});

test('a target with an issue is skipped, because an issue is the reviewer’s own call', () => {
  const { writes, skipped } = BulkVerdicts.plan(RECORDS, ['file:a', 'file:b'], 'ok');

  assert.deepEqual(writes, []);
  assert.deepEqual(skipped, [{ target: 'file:b', reason: 'issue' }]);
});

test('clearing removes every status, issues included, and keeps the notes', () => {
  const { writes, skipped } = BulkVerdicts.plan(RECORDS, ['file:a', 'file:b', 'file:new'], '');

  assert.deepEqual(writes, [{ target: 'file:a', patch: { status: '' } }, { target: 'file:b', patch: { status: '' } }]);
  assert.deepEqual(skipped, []);
});

test('an issue cannot be given in bulk', () => {
  assert.throws(() => BulkVerdicts.plan(RECORDS, ['file:a'], 'issue'), /one file at a time/);
});

test('a target named twice is written once', () => {
  const { writes } = BulkVerdicts.plan(RECORDS, ['file:new', 'file:new'], 'ok');

  assert.equal(writes.length, 1);
});

test('the undo restores exactly what each written target held before', () => {
  const { undo } = BulkVerdicts.plan(RECORDS, ['file:a', 'file:b', 'file:new'], '');

  assert.deepEqual(undo, [
    { target: 'file:a', patch: { status: 'ok', note: 'fine', head: 'h1', claimed: ['new:1-3'] } },
    { target: 'file:b', patch: { status: 'issue', note: 'broken here', head: 'h1', claimed: [] } },
  ]);
});

test('a target with no record before is undone to no status and no note', () => {
  const { undo } = BulkVerdicts.plan(RECORDS, ['file:new'], 'ok');

  assert.deepEqual(undo, [{ target: 'file:new', patch: { status: '', note: '' } }]);
});

test('the undo of any change can be taken before it is made, line marks included', () => {
  const records = recordsOf({ 'lines:a': { status: 'marked', note: '', lines: ['new:4-6'], head: 'h1', claimed: [] } });

  assert.deepEqual(BulkVerdicts.undoOf(records, ['lines:a', 'lines:b']), [
    { target: 'lines:a', patch: { status: 'marked', note: '', lines: ['new:4-6'], head: 'h1', claimed: [] } },
    { target: 'lines:b', patch: { status: '', note: '', lines: [] } },
  ]);
});
