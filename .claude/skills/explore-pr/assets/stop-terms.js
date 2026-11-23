(function (root) {
  'use strict';

  const WORD_CHARACTER = 'A-Za-z0-9_';
  const escapeRegExp = text => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const keyOf = term => String(term.term || '').toLowerCase();

  function defined(content) {
    const terms = new Map();
    (content.chapters || []).forEach(chapter => (chapter.domain || []).forEach(term => {
      if (term.term && !terms.has(keyOf(term))) terms.set(keyOf(term), { ...term, chapter: chapter.id });
    }));
    return [...terms.values()];
  }

  const readerText = stop => [
    stop.title, stop.fit, stop.why, stop.risk,
    ...(stop.checks || []).flatMap(check => [check.question, check.problem]),
    ...(stop.focus || []).map(focus => focus.why),
  ].filter(Boolean).join('\n');

  function matcher(term) {
    const phrase = escapeRegExp(term.term.trim()).replace(/\s+/g, '\\s+');
    const usesThePhrase = new RegExp(`(?<![${WORD_CHARACTER}-])${phrase}(?:e?s)?(?![${WORD_CHARACTER}-])`, 'i');
    const namesTheIdentifier = term.code ? new RegExp(`(?<![${WORD_CHARACTER}])${escapeRegExp(term.code)}(?![${WORD_CHARACTER}])`) : null;
    return ({ text, code }) => usesThePhrase.test(text)
      || (!!namesTheIdentifier && (namesTheIdentifier.test(text) || namesTheIdentifier.test(code)));
  }

  function index(content, codeOf) {
    const terms = defined(content).map(term => ({ term, shows: matcher(term) }));
    const tour = (content.chapters || []).flatMap(chapter => chapter.stops || []);
    const shownAt = new Map(tour.map(stop => {
      const read = { text: readerText(stop), code: codeOf(stop) || '' };
      return [stop.id, terms.filter(({ shows }) => shows(read)).map(({ term }) => term)];
    }));
    const firstStops = new Map();
    tour.forEach(stop => shownAt.get(stop.id).forEach(term => {
      if (!firstStops.has(keyOf(term))) firstStops.set(keyOf(term), stop.id);
    }));
    return {
      at: stop => shownAt.get(stop.id) || [],
      firstAt: term => firstStops.get(keyOf(term)) || null,
    };
  }

  const anchorOf = term => 'term-' + keyOf(term).replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');

  const asName = text => String(text || '').replace(/`/g, '').trim().toLowerCase();
  function withoutNamed(terms, names) {
    const named = new Set(names.map(asName));
    return terms.filter(term => !named.has(asName(term.term)) && !(term.code && named.has(asName(term.code))));
  }

  const StopTerms = { defined, index, anchorOf, withoutNamed };
  if (typeof module === 'object' && module.exports) module.exports = StopTerms;
  else root.StopTerms = StopTerms;
})(typeof window !== 'undefined' ? window : globalThis);
