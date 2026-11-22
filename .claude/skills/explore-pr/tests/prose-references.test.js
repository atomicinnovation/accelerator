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
const STOP = '<a class="ref" href="#tour-claims/stop-claims-s1"><span class="mark" data-mark-of="stop:claims-s1"></span>2.1 Claim before lookup</a>';
const CHAPTER = '<a class="ref" href="#tour-claims"><span class="mark" data-mark-of="chapter:claims"></span>Version claims</a>';
const IDENTITY = '<a class="ref" href="#tour-identity"><span class="mark" data-mark-of="chapter:identity"></span>Launcher identity</a>';

test('a stop id in code becomes a link to the stop, named by its number and title and marked with its verdict', () => {
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

test('a chapter id in code after the word chapter becomes a link named by its title and marked with its sign-off', () => {
  assert.equal(link('Chapter <code>claims</code> uses it.'), `Chapter ${CHAPTER} uses it.`);
});

test('every chapter id in a run after the word chapters is linked', () => {
  assert.equal(
    link('chapters <code>identity</code> and <code>claims</code>'),
    `chapters ${IDENTITY} and ${CHAPTER}`,
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
    `chapter ${IDENTITY} passes <code>claims</code> on`,
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

const DECLARED = [
  { id: 'd-root', kind: 'decision', label: 'Keep one `root`' },
  { id: 'a-store', kind: 'abstraction', label: 'Store' },
  { id: 'claims-i1', kind: 'invariant', label: 'A claim <holds>' },
];
const linkDeclared = (html, except) => ProseReferences.link(html, CHAPTERS, DECLARED, except);
const DECISION = '<a class="ref" href="#decisions/dec-d-root" data-peek="decision:d-root"><span class="mark" data-mark-of="design:d-root"></span>Keep one root</a>';

test('a declared decision id in code becomes a link that peeks at its entry, named by what was decided and marked with its verdict', () => {
  assert.equal(linkDeclared('As <code>d-root</code> says.'), `As ${DECISION} says.`);
});

test('a declared decision id written bare becomes the same link', () => {
  assert.equal(linkDeclared('As d-root says.'), `As ${DECISION} says.`);
});

test('an abstraction and an invariant peek at their own catalogue entries', () => {
  assert.equal(linkDeclared('<code>a-store</code>'), '<a class="ref" href="#abstractions/abs-a-store" data-peek="abstraction:a-store">Store</a>');
  assert.equal(linkDeclared('see claims-i1'), 'see <a class="ref" href="#invariants/inv-claims-i1" data-peek="invariant:claims-i1"><span class="mark" data-mark-of="inv:claims-i1"></span>A claim &lt;holds&gt;</a>');
});

test('an abstraction takes no verdict, so its link carries no mark', () => {
  assert.doesNotMatch(linkDeclared('<code>a-store</code>'), /class="mark"/);
});

test('an entry whose id needs escaping in an address keeps it escaped', () => {
  const declared = [{ id: 'd/odd', kind: 'decision', label: 'Odd' }];

  assert.match(ProseReferences.link('<code>d/odd</code>', CHAPTERS, declared), /href="#decisions\/dec-d%2Fodd"/);
});

test('an id on its own card is not linked', () => {
  assert.equal(linkDeclared('As <code>d-root</code> and d-root say.', 'd-root'), 'As <code>d-root</code> and d-root say.');
});

test('an id nothing declares is left alone', () => {
  assert.equal(linkDeclared('<code>d-gone</code> and d-gone'), '<code>d-gone</code> and d-gone');
});
