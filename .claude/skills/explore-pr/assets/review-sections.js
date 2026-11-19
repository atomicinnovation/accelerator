(function (root) {
  'use strict';

  const GROUPS = ['Start', 'Read', 'Interact', 'Decide', 'Reference'];
  const RETIRED = { criteria: 'tests' };

  const chaptersOf = content => content.chapters || [];
  const anyChapterHas = (content, key) => chaptersOf(content).some(chapter => (chapter[key] || []).length);
  const listed = (content, key) => (content[key] || []).length > 0;
  const design = content => content.design || {};

  const hasDecisions = content => (design(content).decisions || []).length > 0 || anyChapterHas(content, 'decisions');
  const hasAbstractions = content => listed(content, 'keyAbstractions')
    || (design(content).abstractions || []).length > 0 || anyChapterHas(content, 'abstractions');
  const hasDesign = content => !!design(content).summary || (design(content).figures || []).length > 0
    || (design(content).article || []).length > 0
    || hasAbstractions(content) || hasDecisions(content);
  const namesTests = content => listed(content, 'acs') || anyChapterHas(content, 'tests')
    || chaptersOf(content).some(chapter => (chapter.invariants || []).some(invariant => (invariant.pinnedBy || []).length))
    || (content.ledger || []).some(row => (row.tests || []).length);

  function builtIn(content) {
    const ledgerTitle = (content.ledgerMeta || {}).title || 'Risk ledger';
    return [
      ['Start', 'overview', 'Overview', true],
      ['Start', 'design', 'Design', hasDesign(content)],
      ['Start', 'map', 'Attention map', true],
      ['Read', 'tour', 'Guided tour', chaptersOf(content).length > 0],
      ['Read', 'commits', 'By commit', listed(content, 'commits')],
      ['Read', 'files', 'All files', true],
      ['Decide', 'readiness', 'Readiness', true],
      ['Decide', 'notes', 'Your notes', true],
      ['Reference', 'findings', 'Findings', listed(content, 'findings')],
      ['Reference', 'decisions', 'Decisions', hasDecisions(content)],
      ['Reference', 'abstractions', 'Abstractions', hasAbstractions(content)],
      ['Reference', 'invariants', 'Invariants & risks', anyChapterHas(content, 'invariants') || anyChapterHas(content, 'risks')],
      ['Reference', 'tests', 'Tests', namesTests(content)],
      ['Reference', 'terms', 'Domain terms', anyChapterHas(content, 'domain')],
      ['Reference', 'ledger', ledgerTitle, listed(content, 'ledger')],
    ].filter(([, , , present]) => present).map(([group, id, title]) => ({ group, id, title }));
  }

  function of(content, customViews) {
    const custom = (customViews || []).map(view => ({
      group: GROUPS.includes(view.group) ? view.group : 'Interact', id: view.id, title: view.title,
    }));
    const routes = [...builtIn(content || {}), ...custom];
    return GROUPS
      .map(group => ({ group, routes: routes.filter(route => route.group === group).map(({ id, title }) => ({ id, title })) }))
      .filter(section => section.routes.length);
  }

  const routeFor = hash => RETIRED[hash] || hash;

  const ReviewSections = { of, routeFor };
  if (typeof module === 'object' && module.exports) module.exports = ReviewSections;
  else root.ReviewSections = ReviewSections;
})(typeof window !== 'undefined' ? window : globalThis);
