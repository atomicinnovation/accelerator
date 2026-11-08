(function (root) {
  'use strict';

  const lineKey = (side, line) => `${side}:${line}`;

  function changedLinesOf(fileDiff) {
    const changed = [];
    for (const hunk of (fileDiff && fileDiff.hunks) || []) {
      let old = hunk.o, head = hunk.n;
      for (const row of hunk.l) {
        if (row[0] === '+') changed.push({ side: 'new', line: head++ });
        else if (row[0] === '-') changed.push({ side: 'old', line: old++ });
        else { old++; head++; }
      }
    }
    return changed;
  }

  function readingClaimsIn(content) {
    const claims = [];
    (content.chapters || []).forEach((chapter, i) => (chapter.stops || []).forEach((stop, j) => claims.push({
      path: stop.file, side: stop.side || 'new', start: stop.start, end: stop.end,
      by: { target: 'stop:' + stop.id, label: `Stop ${i + 1}.${j + 1}` },
    })));
    (content.ledger || []).forEach(row => claims.push({
      path: row.file, side: 'new', start: row.start ?? row.line, end: row.end ?? row.line,
      by: { target: 'ledger:' + row.id, label: row.id },
    }));
    (content.files || []).forEach(file => claims.push({
      path: file.path, whole: true, by: { target: 'file:' + file.path, label: 'All files' },
    }));
    return claims;
  }

  const reads = (claim, side, line) => claim.whole || (claim.side === side && claim.start <= line && line <= claim.end);
  const readersOf = (claims, side, line) => claims.filter(claim => reads(claim, side, line));
  const labelsOf = claims => [...new Set(claims.map(claim => claim.by.label))];

  function unreadRanges(lines) {
    const ranges = [];
    for (const { side, line } of lines) {
      const last = ranges[ranges.length - 1];
      if (last && last.side === side && last.end === line - 1) last.end = line;
      else ranges.push({ side, start: line, end: line });
    }
    return ranges;
  }

  function fileCoverage(path, fileDiff, claims) {
    const own = claims.filter(claim => claim.path === path);
    const changed = changedLinesOf(fileDiff);
    const readBy = {};
    const unread = [];
    for (const { side, line } of changed) {
      const readers = readersOf(own, side, line);
      if (readers.length) readBy[lineKey(side, line)] = labelsOf(readers);
      else unread.push({ side, line });
    }
    const read = changed.length - unread.length;
    const wholeFileRead = own.some(claim => claim.whole);
    const state = changed.length
      ? (read === changed.length ? 'read' : read ? 'partial' : 'unread')
      : (wholeFileRead ? 'read' : 'unread');
    return { changed: changed.length, read, state, unread: unreadRanges(unread), readBy };
  }

  function linesClaimedBy(claim, fileDiff) {
    if (claim.whole) return changedLinesOf(fileDiff);
    const lines = [];
    for (let line = claim.start; line <= claim.end; line++) lines.push({ side: claim.side, line });
    return lines;
  }

  function coveredElsewhere(claim, claims, fileDiff) {
    const others = claims.filter(other => other.path === claim.path && other.by.target !== claim.by.target);
    const lines = linesClaimedBy(claim, fileDiff);
    if (!lines.length) return [];
    const readers = [];
    for (const { side, line } of lines) {
      const found = readersOf(others, side, line);
      if (!found.length) return [];
      readers.push(...found);
    }
    return labelsOf(readers);
  }

  const ReviewCoverage = { changedLinesOf, readingClaimsIn, fileCoverage, coveredElsewhere };
  if (typeof module === 'object' && module.exports) module.exports = ReviewCoverage;
  else root.ReviewCoverage = ReviewCoverage;
})(typeof window !== 'undefined' ? window : globalThis);
