(function (root) {
  'use strict';

  const fromChapters = (content, key) => (content.chapters || [])
    .flatMap(chapter => (chapter[key] || []).map(item => ({ item, chapter: chapter.id })));

  function testNamed(test) {
    if (typeof test !== 'string') return { file: test.file, name: test.name, line: test.line, proves: test.proves };
    const [file, ...qualified] = test.split('::');
    return { file, name: qualified.at(-1) };
  }

  function tests(content) {
    const byKey = new Map();
    const name = (test, by) => {
      const named = testNamed(test);
      if (!named.file || !named.name) return;
      const key = `${named.file}::${named.name}`;
      const entry = byKey.get(key) || byKey.set(key, { key, file: named.file, name: named.name, namedBy: [] }).get(key);
      if (entry.line == null && named.line != null) entry.line = named.line;
      if (!entry.proves && named.proves) entry.proves = named.proves;
      if (!entry.namedBy.includes(by)) entry.namedBy.push(by);
    };
    (content.chapters || []).forEach(chapter => {
      (chapter.tests || []).forEach(test => name(test, 'chapter:' + chapter.id));
      (chapter.invariants || []).forEach(invariant => (invariant.pinnedBy || []).forEach(test => name(test, 'inv:' + invariant.id)));
    });
    (content.ledger || []).forEach(row => (row.tests || []).forEach(test => name(test, 'ledger:' + row.id)));
    (content.acs || []).forEach(criterion => (criterion.tests || [])
      .forEach(test => (test.hits || []).forEach(hit => name(hit, 'criterion:' + criterion.n))));
    return [...byKey.values()];
  }

  function of(content) {
    const design = content.design || {};
    return {
      decisions: [...(design.decisions || []).map(item => ({ item, chapter: null })), ...fromChapters(content, 'decisions')],
      abstractions: content.keyAbstractions || [],
      invariants: fromChapters(content, 'invariants'),
      risks: fromChapters(content, 'risks'),
      tests: tests(content),
      principles: design.principles || [],
      extension: design.extension || [],
    };
  }

  const Catalogue = { of };
  if (typeof module === 'object' && module.exports) module.exports = Catalogue;
  else root.Catalogue = Catalogue;
})(typeof window !== 'undefined' ? window : globalThis);
