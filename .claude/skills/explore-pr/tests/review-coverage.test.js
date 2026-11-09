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

const MARKS = { target: 'lines:src/a.rs', label: 'your marks' };

test('marking lines merges them into ranges per side', () => {
  let lines = ReviewCoverage.withLinesMarked([], 'new', [3, 2, 4], true);
  lines = ReviewCoverage.withLinesMarked(lines, 'old', [2], true);
  lines = ReviewCoverage.withLinesMarked(lines, 'new', [5, 9], true);

  assert.deepEqual(lines, ['new:2-5', 'new:9', 'old:2']);
});

test('unmarking a line splits the range around it', () => {
  const lines = ReviewCoverage.withLinesMarked(['new:2-5'], 'new', [3], false);

  assert.deepEqual(lines, ['new:2', 'new:4-5']);
});

test('marked lines claim what they name', () => {
  const marks = ReviewCoverage.lineMarkClaims('src/a.rs', ['new:2-3', 'old:4'], MARKS);
  const coverage = ReviewCoverage.fileCoverage('src/a.rs', MODIFIED, marks);

  assert.equal(coverage.read, 3);
  assert.deepEqual(coverage.readBy['old:4'], ['your marks']);
});

const COMMIT = {
  sha: 'c1abcdef0000', merge: false,
  files: [
    { path: 'src/a.rs', headPath: 'src/a.rs', hunks: [{ o: 1, n: 1, l: [' a', '-b', '+c', '+d', ' e'] }], fate: ['old:2', 'new:2', '~c2'] },
    { path: 'src/old.rs', headPath: 'src/new.rs', hunks: [{ o: 5, n: 5, l: ['-x', '+y'] }], fate: ['^c0', 'new:7'] },
  ],
};
const COMMIT_BY = { target: 'commit:c1abcdef0000', label: 'Commit c1abcde' };

test('a commit line the whole diff shows takes its key there; any other stays the commit\'s own', () => {
  const rows = ReviewCoverage.commitLines(COMMIT);

  assert.deepEqual(rows.map(r => [r.path, r.key, r.counted]), [
    ['src/a.rs', 'old:2', true], ['src/a.rs', 'new:2', true], ['commit:c1abcdef0000:src/a.rs', 'new:3', false],
    ['commit:c1abcdef0000:src/old.rs', 'old:5', false], ['src/new.rs', 'new:7', true],
  ]);
});

test('a commit verdict reads every line of the commit, in the whole diff and its own', () => {
  const claims = ReviewCoverage.commitClaims(COMMIT, COMMIT_BY);

  const whole = ReviewCoverage.fileCoverage('src/a.rs', MODIFIED, claims);
  assert.deepEqual(whole.readBy['new:2'], ['Commit c1abcde']);
  assert.equal(whole.read, 2);
  assert.equal(ReviewCoverage.commitCoverage(COMMIT, claims).state, 'read');
});

test('a commit is read once every line it counts is read, whatever reads it', () => {
  const marks = [
    ...ReviewCoverage.lineMarkClaims('src/a.rs', ['old:2', 'new:2'], MARKS),
    ...ReviewCoverage.lineMarkClaims('src/new.rs', ['new:7'], MARKS),
  ];

  const coverage = ReviewCoverage.commitCoverage(COMMIT, marks);

  assert.equal(coverage.changed, 3);
  assert.equal(coverage.read, 3);
  assert.equal(coverage.superseded, 2);
  assert.equal(coverage.state, 'read');
});

test('a commit with lines left to read is read in part', () => {
  const marks = ReviewCoverage.lineMarkClaims('src/a.rs', ['new:2'], MARKS);

  assert.equal(ReviewCoverage.commitCoverage(COMMIT, marks).state, 'partial');
});

test('the readers of one line are named for it', () => {
  const marks = ReviewCoverage.lineMarkClaims('src/a.rs', ['new:2'], MARKS);

  assert.deepEqual(ReviewCoverage.readersAt(marks, 'src/a.rs', 'new', 2), ['your marks']);
  assert.deepEqual(ReviewCoverage.readersAt(marks, 'src/a.rs', 'new', 3), []);
});

test('each line of the whole diff names the commit that last changed it', () => {
  const later = { sha: 'c2', merge: false, files: [{ path: 'src/a.rs', headPath: 'src/a.rs', hunks: [{ o: 6, n: 6, l: ['+z'] }], fate: ['new:6'] }] };

  const origins = ReviewCoverage.originsOf([COMMIT, later]);

  assert.deepEqual(origins['src/a.rs'], { 'old:2': 'c1abcdef0000', 'new:2': 'c1abcdef0000', 'new:6': 'c2' });
  assert.deepEqual(origins['src/new.rs'], { 'new:7': 'c1abcdef0000' });
});
