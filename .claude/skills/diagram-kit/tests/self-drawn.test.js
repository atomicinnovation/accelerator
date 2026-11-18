const test = require('node:test');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const { notations } = require('../lib/notations.js');
const { escaped, inlineOf, linked } = require('../lib/html.js');
const words = require('./fixtures/draw-words/notation.js');

const CLI = path.join(__dirname, '..', 'bin', 'diagram.js');
const FIXTURES = path.join(__dirname, 'fixtures');
const SPEC = { id: 'w1', kind: 'words', title: 'Three words', caption: 'c', words: ['one', '#two', 'three'] };
const kit = notations([words]);

function run(specs, ...args) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'diagram-kit-self-'));
  fs.writeFileSync(path.join(dir, 'spec.json'), JSON.stringify(specs));
  return { dir, ...spawnSync('node', [CLI, ...args, '--notations', FIXTURES], { cwd: dir, encoding: 'utf8' }) };
}

test('a notation that draws itself is known as one', () => {
  assert.equal(kit.drawsItself(SPEC), true);
  assert.equal(kit.drawsItself({ kind: 'nothing' }), false);
});

test('a notation that draws itself is given no scene to place', async () => {
  assert.equal(await kit.laidOut(SPEC), null);
});

test('its figure holds what it drew, between the title and the remarks', () => {
  const figure = kit.figureOf(SPEC, null);

  assert.match(figure, /^<figure class="dk-figure" id="diagram-w1"><figcaption class="dk-title">Three words<\/figcaption><div class="dk-drawn"><p class="words">/);
  assert.match(figure, /<\/p><\/div><ul class="dk-remarks"><li>More than two words\.<\/li><\/ul><p class="dk-caption">c<\/p><\/figure>$/);
  assert.doesNotMatch(figure, /<svg/);
});

test('it is handed the way links are resolved', () => {
  assert.match(kit.figureOf(SPEC, null), /<a href="#two">#two<\/a>/);
  assert.match(kit.figureOf(SPEC, null, { resolveLink: word => ({ href: `/${word}` }) }), /<a href="\/one">one<\/a>/);
});

test('preparing one reports its problems and remarks, and says it draws itself', () => {
  const { stdout, status } = run([SPEC, { ...SPEC, id: 'w2', words: undefined }], 'prepare', 'spec.json');
  const [sound, broken] = JSON.parse(stdout).diagrams;

  assert.equal(status, 0);
  assert.deepEqual([sound.problems, sound.scene, sound.selfDrawn, sound.remarks], [[], null, true, ['More than two words.']]);
  assert.deepEqual([broken.problems, broken.selfDrawn], [['no words'], true]);
});

test('a page shows it beside diagrams the kit lays out', () => {
  const boxes = { id: 'd1', kind: 'boxes', title: 'Two boxes', boxes: [{ id: 'a', label: 'alpha' }, { id: 'b', label: 'beta' }] };
  const { dir, status, stderr } = run([SPEC, boxes], 'render', 'spec.json', '--out', 'page.html');
  const page = fs.readFileSync(path.join(dir, 'page.html'), 'utf8');

  assert.equal(status, 0, stderr);
  assert.match(page, /<p class="words">/);
  assert.match(page, /<svg class="dk" data-graph="d1"/);
});

test('one has no scene to print and cannot be a drawing on its own', () => {
  const laid = run(SPEC, 'layout', 'spec.json'), drawn = run(SPEC, 'render', 'spec.json', '--out', 'w.svg');

  assert.equal(laid.status, 1);
  assert.match(laid.stderr, /warning: diagram w1: a words draws itself, so it has no scene/);
  assert.equal(drawn.status, 1);
  assert.match(drawn.stderr, /warning: diagram w1: a words draws itself; render it to \.html/);
});

test('text is made safe to put in a page', () => {
  assert.equal(escaped('<a href="x">&\'</a>'), '&lt;a href=&quot;x&quot;&gt;&amp;&#39;&lt;/a&gt;');
  assert.equal(escaped(null), '');
});

test('inline text keeps its code spans and nothing else', () => {
  assert.equal(inlineOf('prune, `Try<T>` & go'), 'prune, <code>Try&lt;T&gt;</code> &amp; go');
  assert.equal(inlineOf('a `lone tick'), 'a `lone tick');
  assert.equal(inlineOf(7), '7');
});

test('markup is wrapped in a link when its target resolves, and left alone when it does not', () => {
  const target = { href: '#files', label: 'Open src/a.rs:4', attributes: { 'data-open': 'src/a.rs', 'data-line': 4 } };

  assert.equal(linked('<b>a.rs</b>', target), '<a href="#files" data-open="src/a.rs" data-line="4" aria-label="Open src/a.rs:4"><b>a.rs</b></a>');
  assert.equal(linked('<b>a.rs</b>', null), '<b>a.rs</b>');
});
