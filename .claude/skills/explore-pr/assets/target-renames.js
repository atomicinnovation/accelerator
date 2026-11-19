(function (root) {
  'use strict';

  const BOOKKEEPING = new Set(['target', 'kind', 'status', 'note', 'head', 'claimed', 'updatedAt']);
  const metaOf = record => Object.fromEntries(Object.entries(record).filter(([key]) => !BOOKKEEPING.has(key)));

  function pending(records, renames) {
    const byTarget = new Map((records || []).map(record => [record.target, record]));
    return Object.entries(renames || {})
      .filter(([from, to]) => byTarget.has(from) && !byTarget.has(to))
      .map(([from, to]) => [byTarget.get(from), to])
      .filter(([record]) => record.status || record.note)
      .map(([record, to]) => ({
        target: to, kind: 'finding', status: record.status || '', note: record.note || '',
        head: record.head, claimed: record.claimed, meta: metaOf(record),
      }));
  }

  const TargetRenames = { pending };
  if (typeof module === 'object' && module.exports) module.exports = TargetRenames;
  else root.TargetRenames = TargetRenames;
})(typeof window !== 'undefined' ? window : globalThis);
