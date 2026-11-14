const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const ProseReferences = require(path.join(__dirname, '..', 'assets', 'prose-references.js'));

const CHAPTERS = [
  { id: 'identity', title: 'Launcher identity', stops: [{ id: 'identity-s1', title: 'Identify the launcher' }] },
  {
    id: 'claims',
    title: 'Version claims',
    stops: [
      { id: 'claims-s1', title: 'Claim before lookup' },
      { id: 'claims-s2', title: 'Heartbeat <refresh>' },
    ],
  },
];
const link = html => ProseReferences.link(html, CHAPTERS);
const STOP = '<a class="ref" href="#tour-claims" data-scroll-stop="claims-s1">2.1 Claim before lookup</a>';
const CHAPTER = '<a class="ref" href="#tour-claims">Version claims</a>';

test('a stop id in code becomes a link to the stop, named by its number and title', () => {
  assert.equal(link('See <code>claims-s1</code> first.'), `See ${STOP} first.`);
});

test('a stop id written bare becomes the same link', () => {
  assert.equal(link('(claims-s1) reaches it.'), `(${STOP}) reaches it.`);
});

test('a stop id inside a longer token is left alone', () => {
  assert.equal(link('the xclaims-s10 and claims-s1-old cases'), 'the xclaims-s10 and claims-s1-old cases');
});

test('a stop title is escaped in its link', () => {
  assert.match(link('<code>claims-s2</code>'), />2\.2 Heartbeat &lt;refresh&gt;<\/a>$/);
});

test('a chapter id in code after the word chapter becomes a link named by its title', () => {
  assert.equal(link('Chapter <code>claims</code> uses it.'), `Chapter ${CHAPTER} uses it.`);
});

test('every chapter id in a run after the word chapters is linked', () => {
  assert.equal(
    link('chapters <code>identity</code> and <code>claims</code>'),
    `chapters <a class="ref" href="#tour-identity">Launcher identity</a> and ${CHAPTER}`,
  );
});

test('a chapter id in code followed by the word chapter is linked', () => {
  assert.equal(link('the <code>claims</code> chapter'), `the ${CHAPTER} chapter`);
});

test('code that merely shares a chapter id is left alone', () => {
  assert.equal(link('reads <code>claims</code> from disk'), 'reads <code>claims</code> from disk');
});

test('a chapter run ends at the first code that is not a chapter', () => {
  assert.equal(
    link('chapter <code>identity</code> passes <code>claims</code> on'),
    'chapter <a class="ref" href="#tour-identity">Launcher identity</a> passes <code>claims</code> on',
  );
});

test('references inside a link or a code block are left alone', () => {
  const linked = '<a href="#x">claims-s1 and <code>claims-s1</code></a>';
  const block = '<pre><code>claims-s1</code></pre>';
  assert.equal(link(linked), linked);
  assert.equal(link(block), block);
});

test('an attribute that carries a stop id is left alone', () => {
  assert.equal(link('<span title="claims-s1">x</span>'), '<span title="claims-s1">x</span>');
});

test('prose without chapters to refer to is returned unchanged', () => {
  assert.equal(ProseReferences.link('See <code>claims-s1</code>.', []), 'See <code>claims-s1</code>.');
});
