const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const notation = require('../notation.js');
const { kitWith } = require('../../diagram-kit/lib/index.js');

const SPEC = {
  id: 'prune-by-root', kind: 'matrix', title: 'Only a relocated root claims before it prunes',
  rowLabel: 'Root', colLabel: 'Trigger',
  rows: ['Default', { text: '`Relocated`', link: 'src/root.rs:12' }], cols: ['Start-up', 'Cold path'],
  cells: [['no claim', { text: 'prune, `Try`', tone: 'ok' }], [{ text: 'claim', tone: 'warn', link: 'src/start_up.rs:78' }, { text: 'prune <all>', tone: 'bad' }]],
};
const resolveLink = link => ({ href: `#${link}`, label: `Open ${link}` });
const html = notation.htmlOf(SPEC, { resolveLink });
const problems = change => notation.problemsIn({ ...SPEC, ...change });

test('it draws diagrams of kind matrix, by itself', () => {
  assert.equal(notation.kind, 'matrix');
  assert.equal(typeof notation.htmlOf, 'function');
  assert.equal(notation.sceneOf, undefined);
});

test('a sound matrix has no problems', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
});

test('it is a table that scrolls sideways inside itself when it is too wide', () => {
  assert.match(html, /^<div class="dk-table"><table><thead>/);
  assert.match(html, /<\/tbody><\/table><\/div>$/);
});

test('the corner says what the rows and the columns are', () => {
  assert.match(html, /<thead><tr><th>Root <span class="dk-by">×<\/span> Trigger<\/th><th scope="col">Start-up<\/th><th scope="col">Cold path<\/th><\/tr><\/thead>/);
  assert.match(notation.htmlOf({ ...SPEC, rowLabel: undefined }, { resolveLink }), /<thead><tr><th>Trigger<\/th>/);
});

test('each row is headed by its name, and each cell says its outcome in its tone', () => {
  assert.match(html, /<tr><th scope="row">Default<\/th><td>no claim<\/td><td class="t-ok">prune, <code>Try<\/code><\/td><\/tr>/);
  assert.match(html, /<td class="t-bad">prune &lt;all&gt;<\/td>/);
});

test('a header or a cell with a link opens it', () => {
  assert.match(html, /<th scope="row"><a href="#src\/root.rs:12" aria-label="Open src\/root.rs:12"><code>Relocated<\/code><\/a><\/th>/);
  assert.match(html, /<td class="t-warn"><a href="#src\/start_up.rs:78"[^>]*>claim<\/a><\/td>/);
});

test('a link that does not resolve leaves the text as it is', () => {
  assert.match(notation.htmlOf(SPEC, { resolveLink: () => null }), /<td class="t-warn">claim<\/td>/);
});

test('a matrix needs rows and columns, each named', () => {
  assert.deepEqual(problems({ rows: [], cols: 'two' }), ['no rows', 'cols must be a list']);
  assert.deepEqual(problems({ rows: ['a', 4], cols: ['x', {}], cells: [['1', '2'], ['3', '4']] }), ['row 2: no text', 'column 2: no text']);
});

test('the cells fit the rows and the columns', () => {
  assert.deepEqual(problems({ cells: [['a', 'b']] }), ['has 1 rows of cells for 2 rows']);
  assert.deepEqual(problems({ cells: [['a', 'b'], ['c']] }), ['row 2 does not have 2 cells']);
  assert.deepEqual(problems({ cells: 'none' }), ['cells must be a list of rows']);
});

test('a cell is text, or text with a tone the notation knows and a link that is text', () => {
  assert.deepEqual(problems({ cells: [['a', { tone: 'ok' }], [{ text: 'c', tone: 'fine', link: 4 }, 7]] }), [
    'row 1, column 2: no text',
    'row 2, column 1: tone "fine" is not one of ok, warn, bad, info, none',
    'row 2, column 1: link must be text',
    'row 2, column 2: no text',
  ]);
});

test('the labels of the axes are text', () => {
  assert.deepEqual(problems({ rowLabel: 4, colLabel: [] }), ['rowLabel must be text', 'colLabel must be text']);
});

const EXAMPLES = path.join(__dirname, '..', 'examples');
for (const file of fs.readdirSync(EXAMPLES).filter(name => name.endsWith('.json'))) {
  test(`the example ${file} is sound and draws`, () => {
    const kit = kitWith(), example = JSON.parse(fs.readFileSync(path.join(EXAMPLES, file), 'utf8'));
    assert.deepEqual(kit.problemsIn(example), []);
    assert.match(kit.figureOf(example, null), /<div class="dk-drawn"><div class="dk-table">/);
  });
}
