(function (root) {
  'use strict';

  const ISSUE = 'issue';
  const LINE_MARKS = 'lines:';

  function restoring(target, record) {
    const patch = { status: record.status || '', note: record.note || '' };
    if (target.startsWith(LINE_MARKS)) patch.lines = record.lines || [];
    if ('head' in record) patch.head = record.head;
    if ('claimed' in record) patch.claimed = record.claimed;
    return { target, patch };
  }

  const undoOf = (recordOf, targets) => [...new Set(targets)].map(target => restoring(target, recordOf(target)));

  function plan(recordOf, targets, status) {
    if (status === ISSUE) throw new Error('An issue is raised one file at a time, with its own note');
    const writes = [], skipped = [];
    for (const target of new Set(targets)) {
      const held = recordOf(target).status || '';
      if (held === status) continue;
      if (held === ISSUE && status) { skipped.push({ target, reason: ISSUE }); continue; }
      writes.push({ target, patch: { status } });
    }
    return { writes, skipped, undo: undoOf(recordOf, writes.map(write => write.target)) };
  }

  const BulkVerdicts = { plan, undoOf };
  if (typeof module === 'object' && module.exports) module.exports = BulkVerdicts;
  else root.BulkVerdicts = BulkVerdicts;
})(typeof window !== 'undefined' ? window : globalThis);
