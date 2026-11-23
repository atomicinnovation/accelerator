const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const StopTerms = require(path.join(__dirname, '..', 'assets', 'stop-terms.js'));

const CLAIM = { term: 'version claim', meaning: 'A hold on a version.', code: 'VersionClaim' };
const LEDGER = { term: 'ledger', meaning: 'The run record.' };
const stop = (id, fields = {}) => ({ id, title: '', fit: '', why: '', ...fields });
const contentWith = (stops, domain = [CLAIM, LEDGER]) => ({ chapters: [{ id: 'claims', domain, stops }] });
const termsAt = (content, stopId, codeOf = () => '') => {
  const index = StopTerms.index(content, codeOf);
  const found = content.chapters.flatMap(chapter => chapter.stops).find(each => each.id === stopId);
  return index.at(found).map(term => term.term);
};

test('a stop shows a term whose identifier its code names', () => {
  const content = contentWith([stop('s1')]);

  assert.deepEqual(termsAt(content, 's1', () => 'let claim = VersionClaim::new();'), ['version claim']);
});

test('an identifier inside a longer identifier is not the term', () => {
  const content = contentWith([stop('s1')]);

  assert.deepEqual(termsAt(content, 's1', () => 'VersionClaimer::run(); MyVersionClaim'), []);
});

test('a stop shows a term its text uses, in any case and in the plural', () => {
  const content = contentWith([stop('s1', { fit: 'Every Version claims check runs here.' })]);

  assert.deepEqual(termsAt(content, 's1'), ['version claim']);
});

test('a term inside a longer word is not the term', () => {
  const content = contentWith([stop('s1', { why: 'The subledgers stay separate.' })]);

  assert.deepEqual(termsAt(content, 's1'), []);
});

test('a stop shows a term whose identifier its text names', () => {
  const content = contentWith([stop('s1', { why: 'It returns a `VersionClaim`.' })]);

  assert.deepEqual(termsAt(content, 's1'), ['version claim']);
});

test('a term used only in a comment of the code is not shown, the code is read for identifiers', () => {
  const content = contentWith([stop('s1')]);

  assert.deepEqual(termsAt(content, 's1', () => '// writes the ledger'), []);
});

test('every part of a stop that the reviewer reads is searched', () => {
  const content = contentWith([stop('s1', {
    title: 'Hold the ledger',
    checks: [{ question: 'Is it held?', problem: 'If a version claim leaks.' }],
    focus: [{ line: 3, why: 'Writes the ledger.' }],
  })]);

  assert.deepEqual(termsAt(content, 's1'), ['version claim', 'ledger']);
});

test('a term defined in another chapter is still found, and a term defined twice is shown once', () => {
  const content = {
    chapters: [
      { id: 'one', domain: [CLAIM], stops: [] },
      { id: 'two', domain: [{ ...CLAIM, meaning: 'Again.' }], stops: [stop('s2', { fit: 'A version claim.' })] },
    ],
  };

  assert.deepEqual(termsAt(content, 's2'), ['version claim']);
});

test('terms come in the order they are defined', () => {
  const content = contentWith([stop('s1', { fit: 'The ledger holds each version claim.' })]);

  assert.deepEqual(termsAt(content, 's1'), ['version claim', 'ledger']);
});

test('each term knows the first stop in tour order that shows it', () => {
  const content = contentWith([stop('s1', { fit: 'The ledger.' }), stop('s2', { fit: 'A version claim and the ledger.' })]);
  const index = StopTerms.index(content, () => '');

  assert.equal(index.firstAt(LEDGER), 's1');
  assert.equal(index.firstAt(CLAIM), 's2');
  assert.equal(index.firstAt({ term: 'unused' }), null);
});

test('a term has an anchor safe to put in an address and an element id', () => {
  assert.equal(StopTerms.anchorOf(CLAIM), 'term-version-claim');
  assert.equal(StopTerms.anchorOf({ term: 'pending-push marker (v2)' }), 'term-pending-push-marker-v2');
});

test('the terms the PR defines are listed once each, as first defined', () => {
  const content = { chapters: [{ id: 'a', domain: [CLAIM, LEDGER] }, { id: 'b', domain: [{ ...LEDGER, meaning: 'Later.' }] }] };

  assert.deepEqual(StopTerms.defined(content).map(term => [term.term, term.meaning, term.chapter]),
    [['version claim', 'A hold on a version.', 'a'], ['ledger', 'The run record.', 'a']]);
});

test('a term an abstraction already names, by its words or its identifier, is left out', () => {
  const terms = [CLAIM, LEDGER, { term: 'marker reading', code: 'MarkerReading' }];

  assert.deepEqual(StopTerms.withoutNamed(terms, ['`VersionClaim`', 'Marker reading']).map(term => term.term), ['ledger']);
});

test('a name that merely contains a term does not leave it out', () => {
  assert.deepEqual(StopTerms.withoutNamed([LEDGER], ['LedgerWriter']).map(term => term.term), ['ledger']);
});
