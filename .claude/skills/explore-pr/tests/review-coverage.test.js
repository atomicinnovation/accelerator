const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const ReviewCoverage = require(path.join(__dirname, '..', 'assets', 'review-coverage.js'));

const MODIFIED = { hunks: [{ o: 1, n: 1, l: [' a', '-b', '+c', '+d', ' e', '-f', ' g'] }] };
const ADDED = { hunks: [{ o: 0, n: 1, l: ['+x', '+y', '+z'] }] };
const BINARY = { binary: true, hunks: [] };
const LIMITED_CONTEXT = { hunks: [{ o: 10, n: 10, l: [' k', '+l', ' m'] }, { o: 50, n: 51, l: [' p', '-q', ' r'] }] };

const CONTENT = {
  chapters: [
    { id: 'one', stops: [{ id: 's1', file: 'src/a.rs', start: 2, end: 3 }, { id: 's2', file: 'src/a.rs', side: 'old', start: 2, end: 2 }] },
    { id: 'two', stops: [{ id: 's3', file: 'src/b.rs', start: 1, end: 3 }] },
  ],
  ledger: [
    { id: 'L-1', file: 'src/b.rs', line: 2 },
    { id: 'L-2', file: 'src/a.rs', line: 3, start: 2, end: 4 },
  ],
  files: [{ path: 'src/a.rs' }, { path: 'src/b.rs' }],
};
const claims = ReviewCoverage.readingClaimsIn(CONTENT);
const claimOf = target => claims.find(c => c.by.target === target);
const recorded = (...targets) => claims.filter(c => targets.includes(c.by.target));

test('a changed line is every added head line and every removed base line', () => {
  assert.deepEqual(ReviewCoverage.changedLinesOf(MODIFIED), [
    { side: 'old', line: 2 }, { side: 'new', line: 2 }, { side: 'new', line: 3 }, { side: 'old', line: 4 },
  ]);
});

test('changed lines keep their numbers across hunks with limited context', () => {
  assert.deepEqual(ReviewCoverage.changedLinesOf(LIMITED_CONTEXT), [
    { side: 'new', line: 11 }, { side: 'old', line: 51 },
  ]);
});

test('a stop claims its lines on its own side, head by default', () => {
  assert.deepEqual(claimOf('stop:s1'), { path: 'src/a.rs', side: 'new', start: 2, end: 3, by: { target: 'stop:s1', label: 'Stop 1.1' } });
  assert.deepEqual(claimOf('stop:s2'), { path: 'src/a.rs', side: 'old', start: 2, end: 2, by: { target: 'stop:s2', label: 'Stop 1.2' } });
});

test('a ledger row claims its range, or only its call line without one', () => {
  assert.deepEqual(claimOf('ledger:L-1'), { path: 'src/b.rs', side: 'new', start: 2, end: 2, by: { target: 'ledger:L-1', label: 'L-1' } });
  assert.deepEqual(claimOf('ledger:L-2'), { path: 'src/a.rs', side: 'new', start: 2, end: 4, by: { target: 'ledger:L-2', label: 'L-2' } });
});

test('a file claims every line of itself', () => {
  assert.deepEqual(claimOf('file:src/a.rs'), { path: 'src/a.rs', whole: true, by: { target: 'file:src/a.rs', label: 'All files' } });
});

test('a file nobody has read is unread, with every changed line outstanding', () => {
  const coverage = ReviewCoverage.fileCoverage('src/a.rs', MODIFIED, []);

  assert.equal(coverage.state, 'unread');
  assert.equal(coverage.changed, 4);
  assert.equal(coverage.read, 0);
  assert.deepEqual(coverage.unread, [
    { side: 'old', start: 2, end: 2 }, { side: 'new', start: 2, end: 3 }, { side: 'old', start: 4, end: 4 },
  ]);
});

test('a head-side stop reads only the added lines it spans', () => {
  const coverage = ReviewCoverage.fileCoverage('src/a.rs', MODIFIED, recorded('stop:s1'));

  assert.equal(coverage.state, 'partial');
  assert.equal(coverage.read, 2);
  assert.deepEqual(coverage.unread, [{ side: 'old', start: 2, end: 2 }, { side: 'old', start: 4, end: 4 }]);
  assert.deepEqual(coverage.readBy['new:2'], ['Stop 1.1']);
});

test('a base-side stop reads removed lines', () => {
  const coverage = ReviewCoverage.fileCoverage('src/a.rs', MODIFIED, recorded('stop:s2'));

  assert.equal(coverage.read, 1);
  assert.deepEqual(coverage.readBy['old:2'], ['Stop 1.2']);
});

test('overlapping claims count each line once and name every reader', () => {
  const coverage = ReviewCoverage.fileCoverage('src/a.rs', MODIFIED, recorded('stop:s1', 'ledger:L-2'));

  assert.equal(coverage.read, 2);
  assert.deepEqual(coverage.readBy['new:3'], ['Stop 1.1', 'L-2']);
});

test('claims on another file read nothing here', () => {
  const coverage = ReviewCoverage.fileCoverage('src/a.rs', MODIFIED, recorded('stop:s3', 'ledger:L-1'));

  assert.equal(coverage.read, 0);
});

test('a file verdict reads the whole file', () => {
  const coverage = ReviewCoverage.fileCoverage('src/a.rs', MODIFIED, recorded('file:src/a.rs'));

  assert.equal(coverage.state, 'read');
  assert.equal(coverage.read, 4);
  assert.deepEqual(coverage.unread, []);
});

test('stops that between them span every changed line read the file in full', () => {
  const coverage = ReviewCoverage.fileCoverage('src/b.rs', ADDED, recorded('stop:s3'));

  assert.equal(coverage.state, 'read');
});

test('a file without changed lines is read only through its own verdict', () => {
  assert.equal(ReviewCoverage.fileCoverage('img.png', BINARY, []).state, 'unread');
  assert.equal(ReviewCoverage.fileCoverage('img.png', BINARY, [{ path: 'img.png', whole: true, by: { target: 'file:img.png', label: 'All files' } }]).state, 'read');
});

test('a stop inside a file read in full is covered elsewhere', () => {
  assert.deepEqual(ReviewCoverage.coveredElsewhere(claimOf('stop:s1'), recorded('file:src/a.rs'), MODIFIED), ['All files']);
});

test('a claim is not covered by its own verdict', () => {
  assert.deepEqual(ReviewCoverage.coveredElsewhere(claimOf('stop:s1'), recorded('stop:s1'), MODIFIED), []);
});

test('a claim covered only in part is not covered elsewhere', () => {
  assert.deepEqual(ReviewCoverage.coveredElsewhere(claimOf('ledger:L-2'), recorded('stop:s1'), MODIFIED), []);
});

test('a claim spanned by other claims names each of them', () => {
  assert.deepEqual(ReviewCoverage.coveredElsewhere(claimOf('stop:s1'), recorded('ledger:L-2'), MODIFIED), ['L-2']);
});

test('ranges on the other side do not cover a claim', () => {
  assert.deepEqual(ReviewCoverage.coveredElsewhere(claimOf('stop:s2'), recorded('stop:s1'), MODIFIED), []);
});

test('a file whose changed lines the tour has read is covered elsewhere', () => {
  assert.deepEqual(ReviewCoverage.coveredElsewhere(claimOf('file:src/b.rs'), recorded('stop:s3'), ADDED), ['Stop 2.1']);
});

test('a file with no changed lines is never covered elsewhere', () => {
  const file = { path: 'img.png', whole: true, by: { target: 'file:img.png', label: 'All files' } };

  assert.deepEqual(ReviewCoverage.coveredElsewhere(file, [], BINARY), []);
});
