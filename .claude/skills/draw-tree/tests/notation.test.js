const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const notation = require('../notation.js');
const { kitWith } = require('../../diagram-kit/lib/index.js');

const SPEC = {
  id: 'claims-dir', kind: 'tree', title: 'What a prune reads in claims/', root: '<root>/claims/',
  items: [
    { path: '1.26.0.c0ffee', state: 'kept', note: 'the running version' },
    { path: '1.24.0.012345', state: 'removed', note: '20 d old: `stale`', link: 'src/claims.rs:159' },
    { path: 'locks/prune.lock', state: 'added' },
    { path: 'locks/old/', state: 'changed' },
    { path: 'notes.txt' },
  ],
};
const resolveLink = link => ({ href: `#${link}`, label: `Open ${link}` });
const html = notation.htmlOf(SPEC, { resolveLink });
const rows = html.match(/<div class="dk-row[^"]*">.*?<\/div>/g);
const problems = change => notation.problemsIn({ ...SPEC, ...change });

test('it draws diagrams of kind tree, by itself', () => {
  assert.equal(notation.kind, 'tree');
  assert.equal(typeof notation.htmlOf, 'function');
});

test('a sound tree has no problems', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
});

test('the root heads the tree, and every entry and every directory on the way to one has a row', () => {
  assert.match(html, /^<div class="dk-tree"><div class="dk-row"><span class="dk-name dk-dir">&lt;root&gt;\/claims\/<\/span><\/div>/);
  assert.equal(rows.length, 1 + 6);
  assert.equal(notation.htmlOf({ ...SPEC, root: undefined }, { resolveLink }).match(/<div class="dk-row/g).length, 6);
});

test('guides show which directory each entry is in, and which is the last there', () => {
  const guides = rows.slice(1).map(row => row.match(/<span class="dk-guide">(.*?)<\/span>/)[1]);

  assert.deepEqual(guides, ['├── ', '├── ', '├── ', '│   ├── ', '│   └── ', '└── ']);
});

test('an entry says what the change did to it, in its mark and its colour', () => {
  assert.match(rows[2], /^<div class="dk-row s-removed">.*<span class="dk-name"><a [^>]*>− /);
  assert.match(rows[4], /^<div class="dk-row s-added">.*<span class="dk-name">\+ prune\.lock<\/span>/);
  assert.match(rows[5], /^<div class="dk-row s-changed">.*<span class="dk-name dk-dir">~ old\/<\/span>/);
  assert.match(rows[1], /^<div class="dk-row s-kept">.*<span class="dk-name">1\.26\.0\.c0ffee<\/span>/);
  assert.match(rows[6], /^<div class="dk-row">.*<span class="dk-name">notes\.txt<\/span>/);
});

test('a directory nobody wrote is drawn as one, with no state', () => {
  assert.match(rows[3], /^<div class="dk-row"><span class="dk-guide">├── <\/span><span class="dk-name dk-dir">locks\/<\/span><\/div>$/);
});

test('an entry carries its note, and opens its link from its name', () => {
  assert.match(rows[1], /<span class="dk-note">the running version<\/span>/);
  assert.match(rows[2], /<a href="#src\/claims.rs:159" aria-label="Open src\/claims.rs:159">− 1\.24\.0\.012345<\/a><\/span><span class="dk-note">20 d old: <code>stale<\/code><\/span>/);
});

test('a tree needs entries, each with a path', () => {
  assert.deepEqual(problems({ items: [] }), ['no items']);
  assert.deepEqual(problems({ root: 4, items: [{ note: 'x' }, { path: 'a', state: 'new', note: 4, link: [] }, { path: 'a' }, { path: 'b', where: 'src/b.rs:1' }] }), [
    'root must be text',
    'item 1: no path',
    'item a: state "new" is not one of added, removed, changed, kept',
    'item a: note must be text',
    'item a: link must be text',
    'item a: path is written twice',
    'item b: where is now link',
  ]);
});

const EXAMPLES = path.join(__dirname, '..', 'examples');
for (const file of fs.readdirSync(EXAMPLES).filter(name => name.endsWith('.json'))) {
  test(`the example ${file} is sound and draws`, () => {
    const kit = kitWith(), example = JSON.parse(fs.readFileSync(path.join(EXAMPLES, file), 'utf8'));
    assert.deepEqual(kit.problemsIn(example), []);
    assert.match(kit.figureOf(example, null), /<div class="dk-drawn"><div class="dk-tree">/);
  });
}
