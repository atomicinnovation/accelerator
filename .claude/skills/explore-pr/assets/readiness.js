(function (root) {
  'use strict';

  const plain = text => String(text == null ? '' : text).replace(/`/g, '');
  const counted = (n, word) => `${n} ${n === 1 ? word : word + 's'}`;
  const SERIOUS = new Set(['critical', 'major']);
  const DOUBTFUL = new Set(['partly', 'unsure']);

  const outcome = (says, filter = {}) => ({ says: typeof says === 'function' ? says : () => says, filter });
  const UNREACHED = outcome('');

  function gate(id, title, items, judge, { route, notMet, needsYou, met }) {
    if (!items.length) return null;
    const judged = state => items.filter(item => judge(item) === state);
    const [state, outstanding, reached] = judged('not-met').length ? ['not-met', judged('not-met'), notMet]
      : judged('needs-you').length ? ['needs-you', judged('needs-you'), needsYou]
        : ['met', [], met];
    return {
      id, title, state,
      detail: reached.says(outstanding.length, items.length),
      outstanding: outstanding.map(item => ({ target: item.target, label: item.label })),
      jump: { route, filter: reached.filter },
    };
  }

  function gates(content, recordOf, coverageOf) {
    const statusOf = target => (recordOf(target) || {}).status || '';
    const chapters = content.chapters || [];
    const design = content.design || {};
    const findings = content.findings || [];
    const decisions = [...(design.decisions || []), ...chapters.flatMap(chapter => chapter.decisions || [])]
      .map(d => ({ target: 'design:' + (d.id || plain(d.decision)), label: plain(d.decision) }));
    const invariants = chapters.flatMap(chapter => (chapter.invariants || []).map(v => ({ ...v, target: 'inv:' + v.id, label: plain(v.text) })));
    const findingItems = findings.map(f => ({ ...f, target: 'finding:' + f.id, label: `${f.id} ${plain(f.summary || f.title || '')}`.trim() }));
    const reRaising = (content.acceptedRisks || []).flatMap(risk => (risk.findings || []).map(id => ({ target: 'finding:' + id, label: `${id} re-raises ${plain(risk.title)}` })));
    const readInFull = file => {
      const coverage = coverageOf(file.path);
      return coverage ? coverage.state === 'read' : statusOf('file:' + file.path) === 'ok';
    };

    return [
      gate('ci', 'CI at head', ((content.ci || {}).checks || []).map(c => ({ ...c, target: 'ci:' + c.name, label: c.name })),
        c => (c.status === 'fail' ? 'not-met' : ['pass', 'skipped'].includes(c.status) ? 'met' : 'needs-you'),
        { route: 'overview', notMet: outcome(n => `${counted(n, 'check')} failing`), needsYou: outcome(n => `${counted(n, 'check')} neither passed nor skipped`), met: outcome('Every check passes or is skipped') }),
      gate('design', 'Design decisions', decisions,
        d => (statusOf(d.target) === 'disagree' ? 'not-met' : statusOf(d.target) ? 'met' : 'needs-you'),
        { route: 'decisions', notMet: outcome(n => `You disagree with ${counted(n, 'decision')}`, { verdict: 'disagree' }), needsYou: outcome((n, all) => `${n} of ${counted(all, 'decision')} without your verdict`, { verdict: 'none' }), met: outcome('You have agreed with or questioned every decision') }),
      gate('architecture', 'Fit with the architecture', (design.principles || []).map(p => ({ ...p, target: 'principle:' + plain(p.principle), label: plain(p.principle) })),
        p => (p.verdict === 'broken' ? 'not-met' : p.verdict === 'strained' ? 'needs-you' : 'met'),
        { route: 'decisions', notMet: outcome(n => `The change breaks ${counted(n, 'principle')}`), needsYou: outcome(n => `The change strains ${counted(n, 'principle')}`), met: outcome('Every principle holds') }),
      gate('findings', 'Findings', findingItems,
        f => (statusOf(f.target) === 'block' ? 'not-met' : SERIOUS.has(f.severity) && !statusOf(f.target) ? 'needs-you' : 'met'),
        { route: 'findings', notMet: outcome(n => `You would block on ${counted(n, 'finding')}`, { triage: 'block' }), needsYou: outcome(n => `${counted(n, 'critical or major finding')} not triaged`, { triage: 'none' }), met: outcome('No finding blocks, and every serious one is triaged') }),
      gate('high-risk', 'High-risk files', (content.files || []).filter(f => f.risk === 'high').map(f => ({ ...f, target: 'file:' + f.path, label: f.path })),
        f => (readInFull(f) ? 'met' : 'needs-you'),
        { route: 'files', notMet: UNREACHED, needsYou: outcome((n, all) => `${n} of ${counted(all, 'high-risk file')} not read in full`, { risk: ['high'], progress: ['unread', 'partial'] }), met: outcome('Every high-risk file is read in full') }),
      gate('invariants', 'Invariants', invariants,
        v => (statusOf(v.target) === 'issue' ? 'not-met'
          : (DOUBTFUL.has(v.holds) || !(v.pinnedBy || []).length) && statusOf(v.target) !== 'ok' ? 'needs-you' : 'met'),
        { route: 'invariants', notMet: outcome(n => `You marked ${counted(n, 'invariant')} broken`, { verdict: 'issue' }), needsYou: outcome(n => `${counted(n, 'invariant')} doubtful or unpinned and not yet confirmed`, { verdict: 'none' }), met: outcome('Every invariant is pinned or confirmed') }),
      gate('criteria', 'Criteria → tests', (content.acs || []).flatMap(c => (c.tests || []).map(t => ({ ...t, target: `criterion:${c.n}:${t.ref}`, label: `${c.n}: ${t.ref}` }))),
        t => ((t.hits || []).length ? 'met' : 'needs-you'),
        { route: 'tests', notMet: UNREACHED, needsYou: outcome(n => `${counted(n, 'criterion test')} not found in the code`), met: outcome('Every criterion resolves to its tests') }),
      gate('breaking', 'Breaking changes', (content.breaking || []).map(b => ({ ...b, target: 'breaking:' + plain(b.title), label: plain(b.title) })),
        b => (b.changelog ? 'met' : 'not-met'),
        { route: 'overview', notMet: outcome(n => `${counted(n, 'breaking change')} not in the changelog`), needsYou: UNREACHED, met: outcome('Every breaking change is documented') }),
      gate('accepted-risks', 'Accepted risks', reRaising,
        f => (statusOf(f.target) ? 'met' : 'needs-you'),
        { route: 'findings', notMet: UNREACHED, needsYou: outcome(n => `${counted(n, 'finding')} re-raising an accepted risk not triaged`, { triage: 'none' }), met: outcome('Every finding that re-raises an accepted risk is triaged') }),
      gate('ledger', (content.ledgerMeta || {}).title || 'Risk ledger', (content.ledger || []).map(l => ({ ...l, target: 'ledger:' + l.id, label: `${l.id} ${plain(l.subject)}` })),
        l => (statusOf(l.target) === 'issue' ? 'not-met' : statusOf(l.target) === 'ok' ? 'met' : 'needs-you'),
        { route: 'ledger', notMet: outcome(n => `You marked ${counted(n, 'row')} unsafe`, { status: 'issue' }), needsYou: outcome(n => `${counted(n, 'row')} not yet checked safe`, { status: 'none' }), met: outcome('Every row is checked safe') }),
      gate('chapters', 'Chapters', chapters.map(c => ({ ...c, target: 'chapter:' + c.id, label: c.title || c.id })),
        c => (statusOf(c.target) === 'blocking' ? 'not-met' : statusOf(c.target) === 'understood' ? 'met' : 'needs-you'),
        { route: 'tour', notMet: outcome(n => `${counted(n, 'chapter')} blocking`), needsYou: outcome(n => `${counted(n, 'chapter')} not signed off`), met: outcome('Every chapter is signed off') }),
    ].filter(Boolean);
  }

  const Readiness = { gates };
  if (typeof module === 'object' && module.exports) module.exports = Readiness;
  else root.Readiness = Readiness;
})(typeof window !== 'undefined' ? window : globalThis);
