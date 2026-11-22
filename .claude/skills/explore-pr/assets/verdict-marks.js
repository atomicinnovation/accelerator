(function (root) {
  'use strict';

  const KINDS = {
    decision: { label: 'Verdict', opts: [['approve', 'Approve', 'ok'], ['request-changes', 'Request changes', 'bad'], ['comment', 'Comment', 'warn']] },
    design: { label: 'Design decision', open: 'No verdict yet', opts: [['agree', 'Agree', 'ok'], ['question', 'Question', 'warn'], ['disagree', 'Disagree', 'bad']] },
    chapter: { label: 'Sign-off', open: 'Not signed off', opts: [['understood', 'Signed off', 'ok'], ['concerns', 'Concerns', 'warn'], ['blocking', 'Blocking', 'bad']] },
    stop: { label: 'Stop', open: 'Not read yet', opts: [['ok', 'Read', 'ok'], ['question', 'Question', 'warn'], ['issue', 'Issue', 'bad']] },
    invariant: { label: 'Invariant', open: 'No verdict yet', opts: [['ok', 'Holds', 'ok'], ['question', 'Unsure', 'warn'], ['issue', 'Broken', 'bad']] },
    ledger: { label: 'Removal site', open: 'Not checked', opts: [['ok', 'Safe', 'ok'], ['question', 'Unsure', 'warn'], ['issue', 'Unsafe', 'bad']] },
    finding: { label: 'Triage', open: 'Not triaged', opts: [['block', 'Block merge', 'bad'], ['fix-now', 'Fix in PR', 'warn'], ['follow-up', 'Follow-up', 'info'], ['reject', 'Reject', 'none']] },
    file: { label: 'File', opts: [['ok', 'Reviewed', 'ok'], ['skip', 'Skimmed', 'none'], ['issue', 'Issue', 'bad']] },
    commit: { label: 'Commit', opts: [['ok', 'Reviewed', 'ok'], ['skip', 'Skimmed', 'none'], ['issue', 'Issue', 'bad']] },
    commitfile: { label: 'File in commit', opts: [['ok', 'Reviewed', 'ok'], ['skip', 'Skimmed', 'none'], ['issue', 'Issue', 'bad']] },
  };

  const LINKED = {
    decision: ['design', 'design'],
    invariant: ['inv', 'invariant'],
    stop: ['stop', 'stop'],
    chapter: ['chapter', 'chapter'],
    finding: ['finding', 'finding'],
    ledger: ['ledger', 'ledger'],
  };
  const KIND_RECORDED_UNDER = Object.fromEntries(Object.values(LINKED));

  const GLYPHS = { open: '', ok: '✓', warn: '?', bad: '✕', info: '↻', none: '–' };

  function targetOf(itemKind, id) {
    const linked = LINKED[itemKind];
    return linked ? `${linked[0]}:${id}` : null;
  }

  function kindRecordedUnder(target) {
    const at = String(target || '').indexOf(':');
    return at < 0 ? null : KIND_RECORDED_UNDER[target.slice(0, at)] || null;
  }

  function of(target, record, readElsewhere = []) {
    const kind = KINDS[kindRecordedUnder(target)];
    if (!kind) return null;
    const chosen = kind.opts.find(([value]) => value === (record || {}).status);
    if (chosen) return { target, tone: chosen[2], said: chosen[1] };
    if (readElsewhere.length) return { target, tone: 'ok', said: `Read via ${readElsewhere.join(', ')}` };
    return { target, tone: 'open', said: kind.open };
  }

  const glyphOf = tone => GLYPHS[tone];

  const VerdictMarks = { KINDS, targetOf, of, glyphOf };
  if (typeof module === 'object' && module.exports) module.exports = VerdictMarks;
  else root.VerdictMarks = VerdictMarks;
})(typeof window !== 'undefined' ? window : globalThis);
