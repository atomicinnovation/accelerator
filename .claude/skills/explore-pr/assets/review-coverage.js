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

  const reads = (claim, side, line) => claim.whole
    || (claim.keys ? claim.keys.has(lineKey(side, line)) : claim.side === side && claim.start <= line && line <= claim.end);
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

  function rangesOf(numbers) {
    const ranges = [];
    for (const n of [...new Set(numbers)].sort((a, b) => a - b)) {
      const last = ranges[ranges.length - 1];
      if (last && last[1] === n - 1) last[1] = n;
      else ranges.push([n, n]);
    }
    return ranges;
  }

  function numbersOf(lines, side) {
    const numbers = [];
    for (const entry of lines) {
      const [entrySide, span] = entry.split(':');
      if (entrySide !== side) continue;
      const [start, end = start] = span.split('-').map(Number);
      for (let n = start; n <= end; n++) numbers.push(n);
    }
    return numbers;
  }

  function withLinesMarked(lines, side, numbers, marked) {
    const out = [];
    for (const each of ['new', 'old']) {
      const current = new Set(numbersOf(lines, each));
      if (each === side) numbers.forEach(n => (marked ? current.add(n) : current.delete(n)));
      out.push(...rangesOf([...current]).map(([start, end]) => `${each}:${start}${end !== start ? '-' + end : ''}`));
    }
    return out;
  }

  function lineMarkClaims(path, lines, by) {
    return ['new', 'old'].flatMap(side => rangesOf(numbersOf(lines, side)).map(([start, end]) => ({ path, side, start, end, by })));
  }

  const spanText = (side, start, end) => `${side}:${start}${end !== start ? '-' + end : ''}`;
  const asLines = (side, numbers) => rangesOf(numbers).map(([start, end]) => spanText(side, start, end));

  function claimedLines(claim, fileDiff) {
    if (!claim.whole) return [spanText(claim.side, claim.start, claim.end)];
    const changed = changedLinesOf(fileDiff);
    return ['new', 'old'].flatMap(side => asLines(side, changed.filter(each => each.side === side).map(each => each.line)));
  }

  function nowAt(runs, number) {
    for (const [from, to, length] of runs || []) if (from <= number && number < from + length) return to + number - from;
    return null;
  }

  function carriedOver(lines, fileLineage) {
    if (fileLineage === 'same') return lines || [];
    if (!fileLineage) return [];
    return ['new', 'old'].flatMap(side => asLines(side, numbersOf(lines || [], side)
      .map(number => nowAt(fileLineage[side], number)).filter(number => number !== null)));
  }

  const recordedEarlier = (record, head) => !!(record && record.head && head && record.head !== head);
  const lineageOf = (record, lineage, path) => ((lineage || {})[record.head] || {})[path];

  function standingClaim(claim, record, head, lineage) {
    if (!recordedEarlier(record, head)) return claim;
    const carried = carriedOver(record.claimed, lineageOf(record, lineage, claim.path));
    const keys = new Set(['new', 'old'].flatMap(side => numbersOf(carried, side).map(number => lineKey(side, number))));
    return { path: claim.path, keys, by: claim.by };
  }

  function changedSince(claim, record, head, lineage, fileDiff) {
    if (!recordedEarlier(record, head)) return 0;
    const standing = standingClaim(claim, record, head, lineage);
    const changed = new Set(changedLinesOf(fileDiff).map(each => lineKey(each.side, each.line)));
    return linesClaimedBy(claim, fileDiff)
      .filter(each => changed.has(lineKey(each.side, each.line)) && !standing.keys.has(lineKey(each.side, each.line))).length;
  }

  function filesChangedSince(paths, recordHead, head, lineage) {
    if (!recordedEarlier({ head: recordHead }, head)) return [];
    const then = (lineage || {})[recordHead] || {};
    return paths.filter(path => !then[path] || (then[path] !== 'same' && then[path].changed !== false));
  }

  const commitPath = (commit, file) => `commit:${commit.sha}:${file.path}`;
  const inWholeDiff = fate => fate.includes(':');

  const commitLinesMemo = new WeakMap();
  function commitLines(commit) {
    const known = commitLinesMemo.get(commit);
    if (known) return known;
    const rows = [];
    for (const file of commit.files || []) {
      let i = 0;
      for (const hunk of file.hunks || []) {
        let old = hunk.o, head = hunk.n;
        for (const row of hunk.l) {
          if (row[0] === '+' || row[0] === '-') {
            const side = row[0] === '+' ? 'new' : 'old', line = side === 'new' ? head : old, fate = file.fate[i++];
            rows.push(inWholeDiff(fate)
              ? { file, side, line, fate, counted: true, path: file.headPath, key: fate }
              : { file, side, line, fate, counted: false, path: commitPath(commit, file), key: lineKey(side, line) });
          }
          if (row[0] !== '+') old++;
          if (row[0] !== '-') head++;
        }
      }
    }
    commitLinesMemo.set(commit, rows);
    return rows;
  }

  function commitClaims(commit, by) {
    return (commit.files || []).flatMap(file => [
      { path: file.headPath, keys: new Set(file.fate.filter(inWholeDiff)), by },
      { path: commitPath(commit, file), whole: true, by },
    ]);
  }

  function readersAt(claims, path, side, line) {
    return labelsOf(claims.filter(claim => claim.path === path && reads(claim, side, line)));
  }

  function commitCoverage(commit, claims) {
    const rows = commitLines(commit);
    const counted = rows.filter(row => row.counted);
    const byPath = new Map();
    claims.forEach(claim => (byPath.get(claim.path) || byPath.set(claim.path, []).get(claim.path)).push(claim));
    const isRead = row => {
      const [side, line] = row.key.split(':');
      return (byPath.get(row.path) || []).some(claim => reads(claim, side, Number(line)));
    };
    const read = counted.filter(isRead).length;
    const state = counted.length ? (read === counted.length ? 'read' : read ? 'partial' : 'unread') : (rows.length && rows.every(isRead) ? 'read' : 'unread');
    return { changed: counted.length, read, superseded: rows.length - counted.length, state };
  }

  function originsOf(commits) {
    const origins = {};
    for (const commit of commits) {
      for (const row of commitLines(commit).filter(each => each.counted)) {
        (origins[row.path] = origins[row.path] || {})[row.key] = commit.sha;
      }
    }
    return origins;
  }

  const ReviewCoverage = {
    changedLinesOf, readingClaimsIn, fileCoverage, coveredElsewhere,
    withLinesMarked, lineMarkClaims, claimedLines, carriedOver, standingClaim, changedSince, filesChangedSince, commitLines, commitClaims, commitCoverage, readersAt, originsOf,
  };
  if (typeof module === 'object' && module.exports) module.exports = ReviewCoverage;
  else root.ReviewCoverage = ReviewCoverage;
})(typeof window !== 'undefined' ? window : globalThis);
