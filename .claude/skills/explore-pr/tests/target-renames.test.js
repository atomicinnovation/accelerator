const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const TargetRenames = require(path.join(__dirname, '..', 'assets', 'target-renames.js'));

const RENAMES = { 'own:claims-1': 'finding:N-claims-1', 'own:claims-2': 'finding:N-claims-2', 'own:store-1': 'finding:N-store-1' };
const OLD = {
  target: 'own:claims-1', kind: 'finding', status: 'block', note: 'Stale.', head: 'abc', claimed: [], label: 'New observation in Claims', where: 'src/a.rs:2',
};

test('a verdict on a renamed target is copied to its new target, head and all', () => {
  assert.deepEqual(TargetRenames.pending([OLD], RENAMES), [{
    target: 'finding:N-claims-1', kind: 'finding', status: 'block', note: 'Stale.', head: 'abc', claimed: [],
    meta: { label: 'New observation in Claims', where: 'src/a.rs:2' },
  }]);
});

test('a note alone is copied too', () => {
  const noted = { target: 'own:claims-2', kind: 'finding', note: 'Ask.' };

  assert.deepEqual(TargetRenames.pending([noted], RENAMES).map(copy => [copy.target, copy.status, copy.note]), [['finding:N-claims-2', '', 'Ask.']]);
});

test('once the new target has a record nothing more is copied', () => {
  assert.deepEqual(TargetRenames.pending([OLD, { target: 'finding:N-claims-1', kind: 'finding', status: '' }], RENAMES), []);
});

test('a record with neither a verdict nor a note is not copied', () => {
  assert.deepEqual(TargetRenames.pending([{ target: 'own:store-1', kind: 'finding', status: '', note: '' }], RENAMES), []);
});

test('a rename whose old target has no record is ignored', () => {
  assert.deepEqual(TargetRenames.pending([], RENAMES), []);
  assert.deepEqual(TargetRenames.pending([OLD], undefined), []);
});
