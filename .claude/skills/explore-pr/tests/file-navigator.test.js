const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const FileNavigator = require(path.join(__dirname, '..', 'assets', 'file-navigator.js'));

const entry = (filePath, over = {}) => ({
  path: filePath, layer: 'prod', risk: 'low', chapter: '', readingOrder: null, firstCommit: null,
  changes: ['modified'], add: 1, del: 0, coverage: { changed: 1, read: 0, state: 'unread' },
  verdict: '', mentions: [], commits: [], ...over,
});
const filters = over => ({ ...FileNavigator.BLANK, ...over });
const shown = (entries, chosen = {}, query = '') => entries
  .filter(each => FileNavigator.matches(each, filters(chosen), query)).map(each => each.path);
const names = nodes => nodes.map(node => node.name);
const labelsOf = rows => rows.map(({ node, depth }) => `${'  '.repeat(depth)}${node.name}`);

const SRC = [entry('src/a.rs'), entry('src/b.rs'), entry('docs/readme.md', { layer: 'records' })];

test('a query keeps paths holding every one of its terms, ignoring case', () => {
  assert.deepEqual(shown(SRC, {}, 'SRC a.'), ['src/a.rs']);
  assert.deepEqual(shown(SRC, {}, ''), ['src/a.rs', 'src/b.rs', 'docs/readme.md']);
});

test('a star in a query term matches any run of characters', () => {
  assert.deepEqual(shown(SRC, {}, 'src/*.rs'), ['src/a.rs', 'src/b.rs']);
  assert.deepEqual(shown(SRC, {}, 'd*me'), ['docs/readme.md']);
});

test('a query term starting with ! excludes the paths it matches', () => {
  assert.deepEqual(shown(SRC, {}, '!docs'), ['src/a.rs', 'src/b.rs']);
  assert.deepEqual(shown(SRC, {}, 'src !b.*'), ['src/a.rs']);
});

test('a query treats the characters of a regular expression literally', () => {
  assert.deepEqual(shown([entry('a+b(c).rs'), entry('ab.rs')], {}, 'a+b('), ['a+b(c).rs']);
});

test('values chosen in one facet widen the match, and different facets narrow it', () => {
  const entries = [
    entry('a', { layer: 'prod', risk: 'high' }),
    entry('b', { layer: 'tests', risk: 'high' }),
    entry('c', { layer: 'build', risk: 'low' }),
  ];

  assert.deepEqual(shown(entries, { layer: ['prod', 'tests'] }), ['a', 'b']);
  assert.deepEqual(shown(entries, { layer: ['prod', 'build'], risk: ['low'] }), ['c']);
});

test('progress comes from the coverage, and a file with no changed lines is read only once it has a verdict', () => {
  const entries = [
    entry('unread'),
    entry('partial', { coverage: { changed: 4, read: 2, state: 'partial' } }),
    entry('read', { coverage: { changed: 4, read: 4, state: 'read' } }),
    entry('renamed-only', { coverage: { changed: 0, read: 0, state: 'read' } }),
    entry('renamed-and-reviewed', { coverage: { changed: 0, read: 0, state: 'read' }, verdict: 'ok' }),
  ];

  assert.deepEqual(shown(entries, { progress: ['unread', 'partial'] }), ['unread', 'partial', 'renamed-only']);
  assert.deepEqual(shown(entries, { progress: ['read'] }), ['read', 'renamed-and-reviewed']);
});

test('the verdict facet matches a file without a verdict as none', () => {
  const entries = [entry('a', { verdict: 'ok' }), entry('b'), entry('c', { verdict: 'issue' })];

  assert.deepEqual(shown(entries, { verdict: ['none', 'issue'] }), ['b', 'c']);
});

test('a file with no risk counts as skim', () => {
  assert.deepEqual(shown([entry('a', { risk: undefined }), entry('b')], { risk: ['skim'] }), ['a']);
});

test('change, mentions and commit match any of the file’s values', () => {
  const entries = [
    entry('a', { changes: ['modified', 'binary'], mentions: ['stop', 'finding'], commits: ['c1', 'c2'] }),
    entry('b', { changes: ['added'], mentions: ['ledger'], commits: ['c2'] }),
  ];

  assert.deepEqual(shown(entries, { change: ['binary'] }), ['a']);
  assert.deepEqual(shown(entries, { mentions: ['ledger', 'finding'] }), ['a', 'b']);
  assert.deepEqual(shown(entries, { commit: ['c1'] }), ['a']);
});

test('the chapter facet matches a file in one of the chosen chapters', () => {
  assert.deepEqual(shown([entry('a', { chapter: 'claims' }), entry('b')], { chapter: ['claims'] }), ['a']);
});

test('a facet value counts what choosing it alone would show, with the other facets and the query applied', () => {
  const entries = [
    entry('src/a', { layer: 'prod', risk: 'high' }),
    entry('src/b', { layer: 'prod', risk: 'low' }),
    entry('src/c', { layer: 'tests', risk: 'high' }),
    entry('docs/d', { layer: 'records', risk: 'high' }),
  ];

  const counts = FileNavigator.facetCounts(entries, filters({ layer: ['prod'], risk: ['high'] }), 'src');

  assert.deepEqual(counts.layer, { prod: 1, tests: 1 });
  assert.deepEqual(counts.risk, { high: 1, low: 1 });
});

test('a chosen facet value that nothing matches is still counted, as zero', () => {
  const counts = FileNavigator.facetCounts([entry('a', { layer: 'prod' })], filters({ layer: ['tests'] }), '');

  assert.equal(counts.layer.tests, 0);
  assert.equal(counts.layer.prod, 1);
});

const tree = (entries, options = {}) => FileNavigator.treeOf(entries, { groupBy: 'tree', sortBy: 'path', groups: {}, ...options });
const visible = (entries, options, collapsed = new Set()) => labelsOf(FileNavigator.rowsOf(tree(entries, options), collapsed));

test('the tree nests files by directory, folders before files', () => {
  assert.deepEqual(visible([entry('z.md'), entry('src/b.rs'), entry('src/a.rs'), entry('docs/x.md')]), [
    'docs/', '  x.md', 'src/', '  a.rs', '  b.rs', 'z.md',
  ]);
});

test('a chain of folders that each hold only one folder is drawn as one folder', () => {
  assert.deepEqual(visible([entry('cli/visualiser/src/main.rs'), entry('cli/visualiser/src/lib.rs'), entry('cli/visualiser/Cargo.toml')]), [
    'cli/visualiser/', '  src/', '    lib.rs', '    main.rs', '  Cargo.toml',
  ]);
});

test('a folder holding a single file keeps the file as its own row', () => {
  assert.deepEqual(visible([entry('a/b/only.rs'), entry('top.md')]), ['a/b/', '  only.rs', 'top.md']);
});

test('a folder carries the totals of everything under it', () => {
  const [folder] = tree([
    entry('src/a.rs', { risk: 'low', add: 3, del: 1, coverage: { changed: 4, read: 4, state: 'read' }, verdict: 'ok' }),
    entry('src/deep/b.rs', { risk: 'high', add: 5, del: 0, coverage: { changed: 5, read: 1, state: 'partial' }, verdict: 'issue' }),
    entry('src/deep/c.rs', { risk: undefined, coverage: { changed: 1, read: 0, state: 'unread' } }),
  ]);

  assert.deepEqual(folder.totals, {
    files: 3, changed: 10, read: 5, unread: 5, size: 10, highestRisk: 'high',
    verdicts: { ok: 1, skip: 0, issue: 1, none: 1 }, readingOrder: Infinity, firstCommit: Infinity,
  });
});

test('sorting by lines still to read puts the most unread first, folders by their totals', () => {
  const entries = [
    entry('a/one', { coverage: { changed: 2, read: 0, state: 'unread' } }),
    entry('b/two', { coverage: { changed: 9, read: 0, state: 'unread' } }),
    entry('b/three', { coverage: { changed: 9, read: 9, state: 'read' } }),
    entry('x', { coverage: { changed: 1, read: 0, state: 'unread' } }),
    entry('y', { coverage: { changed: 5, read: 0, state: 'unread' } }),
  ];

  assert.deepEqual(visible(entries, { sortBy: 'unread' }), ['b/', '  two', '  three', 'a/', '  one', 'y', 'x']);
});

test('sorting by size puts the largest change first', () => {
  assert.deepEqual(names(tree([entry('s', { add: 1 }), entry('l', { add: 9, del: 9 }), entry('m', { add: 5 })], { sortBy: 'size' })), ['l', 'm', 's']);
});

test('sorting by risk goes high, medium, low, skim, a folder by the highest risk under it', () => {
  const entries = [entry('k'), entry('h', { risk: 'skim' }), entry('d/x', { risk: 'high' }), entry('m', { risk: 'medium' })];

  assert.deepEqual(names(tree(entries, { sortBy: 'risk' })), ['d/', 'm', 'k', 'h']);
});

test('sorting by reading order follows the tour, with files outside it last', () => {
  const entries = [entry('outside'), entry('second', { readingOrder: 2 }), entry('first', { readingOrder: 1 })];

  assert.deepEqual(names(tree(entries, { sortBy: 'reading' })), ['first', 'second', 'outside']);
});

test('sorting by first commit follows the order the commits touched the files', () => {
  const entries = [entry('late', { firstCommit: 3 }), entry('none'), entry('early', { firstCommit: 0 })];

  assert.deepEqual(names(tree(entries, { sortBy: 'commit' })), ['early', 'late', 'none']);
});

test('files with the same measure fall back to path order', () => {
  assert.deepEqual(names(tree([entry('b'), entry('a'), entry('c')], { sortBy: 'size' })), ['a', 'b', 'c']);
});

test('grouping by none gives every file in one list named by its path, sorted across them all', () => {
  const entries = [entry('b/x', { add: 1 }), entry('a/deep/y', { add: 9 }), entry('c', { add: 5 })];

  assert.deepEqual(visible(entries, { groupBy: 'none', sortBy: 'size' }), ['a/deep/y', 'c', 'b/x']);
  assert.deepEqual(visible(entries, { groupBy: 'none', sortBy: 'path' }), ['a/deep/y', 'b/x', 'c']);
  assert.deepEqual(FileNavigator.ancestorsOf(tree(entries, { groupBy: 'none' }), 'b/x'), []);
});

const GROUPS = { layer: [['prod', 'Production code'], ['tests', 'Tests']], chapter: [['claims', 'Claims'], ['prune', 'Prune']] };

test('grouping gives one group per value in the order given, each holding a tree of its files', () => {
  const entries = [entry('t/x.rs', { layer: 'tests' }), entry('src/a.rs'), entry('src/b.rs')];

  assert.deepEqual(visible(entries, { groupBy: 'layer', groups: GROUPS }), [
    'Production code', '  src/', '    a.rs', '    b.rs', 'Tests', '  t/', '    x.rs',
  ]);
});

test('a group value outside the given order follows, named by its id', () => {
  assert.deepEqual(names(tree([entry('a', { layer: 'odd' }), entry('b', { layer: 'tests' })], { groupBy: 'layer', groups: GROUPS })), ['Tests', 'odd']);
});

test('grouping by chapter puts files outside the tour in a final group', () => {
  const entries = [entry('a'), entry('b', { chapter: 'prune' }), entry('c', { chapter: 'claims' })];

  assert.deepEqual(names(tree(entries, { groupBy: 'chapter', groups: GROUPS })), ['Claims', 'Prune', 'Not in the tour']);
});

test('grouping by risk and by change type have their own order', () => {
  const entries = [entry('a', { risk: 'low', changes: ['deleted'] }), entry('b', { risk: 'high', changes: ['added'] }), entry('c', { risk: undefined, changes: ['modified', 'binary'] })];

  assert.deepEqual(names(tree(entries, { groupBy: 'risk' })), ['high', 'low', 'skim']);
  assert.deepEqual(names(tree(entries, { groupBy: 'change' })), ['added', 'modified', 'deleted']);
});

test('a group carries the totals of its files', () => {
  const [group] = tree([entry('a', { coverage: { changed: 3, read: 1, state: 'partial' } }), entry('b')], { groupBy: 'layer', groups: GROUPS });

  assert.equal(group.totals.files, 2);
  assert.equal(group.totals.unread, 3);
});

test('a folder or group can be collapsed, hiding everything under it', () => {
  const nodes = tree([entry('src/a.rs'), entry('src/deep/b.rs'), entry('src/deep/c.rs'), entry('top.md')]);
  const deep = nodes[0].children[0];

  assert.deepEqual(labelsOf(FileNavigator.rowsOf(nodes, new Set([deep.id]))), ['src/', '  deep/', '  a.rs', 'top.md']);
  assert.deepEqual(labelsOf(FileNavigator.rowsOf(nodes, new Set([nodes[0].id]))), ['src/', 'top.md']);
});

test('folder ids are stable and differ between groups holding the same directory', () => {
  const entries = [entry('src/a.rs'), entry('src/b.rs', { layer: 'tests' })];
  const [prod, tests] = tree(entries, { groupBy: 'layer', groups: GROUPS });

  assert.equal(tree(entries, { groupBy: 'layer', groups: GROUPS })[0].children[0].id, prod.children[0].id);
  assert.notEqual(prod.children[0].id, tests.children[0].id);
});

test('the file order is every file in display order, collapsed or not', () => {
  const nodes = tree([entry('z.md'), entry('src/b.rs'), entry('src/a.rs')]);

  assert.deepEqual(FileNavigator.fileOrder(nodes), ['src/a.rs', 'src/b.rs', 'z.md']);
});

test('the ancestors of a file are the folders and group to open to show it', () => {
  const nodes = tree([entry('src/deep/b.rs'), entry('src/a.rs'), entry('t.md', { layer: 'tests' })], { groupBy: 'layer', groups: GROUPS });
  const [prod] = nodes;

  assert.deepEqual(FileNavigator.ancestorsOf(nodes, 'src/deep/b.rs'), [prod.id, prod.children[0].id, prod.children[0].children[0].id]);
  assert.deepEqual(FileNavigator.ancestorsOf(nodes, 'missing'), []);
});

test('the files under a folder are every file beneath it', () => {
  const [src] = tree([entry('src/deep/b.rs'), entry('src/a.rs'), entry('top.md')]);

  assert.deepEqual(FileNavigator.filesUnder(src), ['src/deep/b.rs', 'src/a.rs']);
});

const COMMITS = [{ sha: 'c1', subject: 'Add the store', merge: false }, { sha: 'c2', subject: 'Merge main', merge: true }, { sha: 'c3', subject: 'Wire it in', merge: false }];
const BY_COMMIT = { c1: [entry('src/store.rs'), entry('src/lib.rs')], c3: [entry('src/lib.rs'), entry('README.md')] };
const commitTree = (options = {}) => FileNavigator.commitTreeOf(COMMITS, BY_COMMIT, { groupBy: 'tree', sortBy: 'path', groups: {}, ...options });

test('each commit is a top-level node, in order, holding the tree of its files', () => {
  const nodes = commitTree();

  assert.deepEqual(nodes.map(node => [node.kind, node.sha, node.index, node.name, node.merge]), [
    ['commit', 'c1', 0, 'Add the store', false], ['commit', 'c2', 1, 'Merge main', true], ['commit', 'c3', 2, 'Wire it in', false],
  ]);
  assert.deepEqual(labelsOf(FileNavigator.rowsOf(nodes[0].children, new Set())), ['src/', '  lib.rs', '  store.rs']);
});

test('a commit with no matching files is kept, empty, so the commits never have gaps', () => {
  assert.deepEqual(commitTree()[1].children, []);
});

test('a commit carries the totals of its files', () => {
  assert.equal(commitTree()[0].totals.files, 2);
});

test('the file order runs through every commit, naming each file by its commit', () => {
  assert.deepEqual(FileNavigator.fileOrder(commitTree()), ['c1:src/lib.rs', 'c1:src/store.rs', 'c3:src/lib.rs', 'c3:README.md']);
});

test('a collapsed commit hides its files', () => {
  const nodes = commitTree();

  assert.deepEqual(labelsOf(FileNavigator.rowsOf(nodes, new Set([nodes[0].id, nodes[2].id]))), ['Add the store', 'Merge main', 'Wire it in']);
});

test('the same directory in two commits is two folders', () => {
  const [first, , third] = commitTree();

  assert.notEqual(first.children[0].id, third.children[0].id);
});

test('the ancestors of a file in a commit are its commit and its folders', () => {
  const [first] = commitTree();

  assert.deepEqual(FileNavigator.ancestorsOf(commitTree(), 'c1:src/store.rs'), [first.id, first.children[0].id]);
  assert.deepEqual(FileNavigator.ancestorsOf(commitTree(), 'c3:README.md'), [commitTree()[2].id]);
});

test('a file is known by its path, or within a commit by its commit and path', () => {
  const [file] = tree([entry('src/a.rs')])[0].children;
  const [commit] = commitTree();

  assert.equal(file.key, 'src/a.rs');
  assert.deepEqual(FileNavigator.filesUnder(commit.children[0]), ['c1:src/lib.rs', 'c1:src/store.rs']);
});
