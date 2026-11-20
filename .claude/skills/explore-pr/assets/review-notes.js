(function (root) {
  'use strict';

  const GENERAL = 'General notes';
  const OTHER = 'Other';
  const SECTIONS = [
    ['Design decisions', 'design', ['disagree', 'question', 'agree', '']],
    ['Findings', 'finding', ['block', 'fix-now', 'follow-up', 'reject', '']],
    ['Chapters', 'chapter', ['blocking', 'concerns', 'understood', '']],
    ['Invariants', 'invariant', ['issue', 'question', 'ok', '']],
    ['Removal sites', 'ledger', ['issue', 'question', 'ok', '']],
    ['Tour stops', 'stop', ['issue', 'question', 'ok', '']],
    ['Files', 'file', ['issue', 'ok', 'skip', '']],
    ['Commits', 'commit', ['issue', 'ok', 'skip', '']],
  ];
  const SUMMARISED_ELSEWHERE = new Set(['lines']);
  const VERDICT = 'decision';

  const shortSha = sha => String(sha || '').slice(0, 7);
  function sectionOf(kind) {
    if (kind === 'notes') return GENERAL;
    const known = SECTIONS.find(([, sectionKind]) => sectionKind === kind);
    return known ? known[0] : OTHER;
  }

  function entries(records, renames) {
    const renamed = renames || {};
    return (records || [])
      .filter(record => (record.status || record.note) && record.target !== VERDICT
        && !SUMMARISED_ELSEWHERE.has(record.kind) && !(record.target in renamed))
      .map(record => ({
        target: record.target, kind: record.kind, status: record.status || '', note: record.note || '',
        label: record.label, where: record.where, head: record.head, section: sectionOf(record.kind),
      }));
  }

  function markdown(listed, context) {
    const { number, head, kinds = {}, verdict = {}, linesRead = '' } = context;
    const labelOf = (kind, status) => (((kinds[kind] || {}).opts || []).find(([value]) => value === status) || [status, status])[1];
    const earlier = record => !!(record.head && head && record.head !== head);
    let out = `# PR ${number} review notes\n\n${head ? `Head: ${shortSha(head)}\n\n` : ''}`;
    out += `Verdict: ${verdict.status ? labelOf(VERDICT, verdict.status) : 'undecided'}`;
    out += `${earlier(verdict) ? ` (recorded at ${shortSha(verdict.head)}, before the pull request moved)` : ''}\n`;
    out += verdict.note ? `\n${verdict.note}\n` : '';
    const general = listed.find(entry => entry.section === GENERAL);
    if (general) out += `\n## ${GENERAL}\n\n${general.note}\n`;
    const item = entry => `- [${entry.status ? labelOf(entry.kind, entry.status) : 'note'}] ${entry.label || entry.target}`
      + `${entry.where ? ` (${entry.where})` : ''}`
      + `${earlier(entry) ? ` — recorded at ${shortSha(entry.head)}, before the pull request moved` : ''}`
      + `${entry.note ? `\n  ${entry.note.replace(/\n/g, '\n  ')}` : ''}\n`;
    [...SECTIONS, [OTHER, null, null]].forEach(([title, , order]) => {
      const inSection = listed.filter(entry => entry.section === title);
      if (!inSection.length) return;
      const rank = entry => (order ? order.indexOf(entry.status) : 0);
      const sorted = [...inSection].sort((a, b) => (rank(a) - rank(b)) || String(a.target).localeCompare(String(b.target)));
      out += `\n## ${title}\n\n${sorted.map(item).join('')}`;
    });
    return out + linesRead;
  }

  const ReviewNotes = { entries, markdown, GENERAL, SECTIONS: [...SECTIONS.map(([title]) => title), OTHER] };
  if (typeof module === 'object' && module.exports) module.exports = ReviewNotes;
  else root.ReviewNotes = ReviewNotes;
})(typeof window !== 'undefined' ? window : globalThis);
