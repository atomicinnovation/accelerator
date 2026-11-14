(function (root) {
  'use strict';
  const inNode = typeof module === 'object' && module.exports;
  const { inlineOf, linked } = inNode ? require('../diagram-kit/lib/html.js') : root.DiagramKit;

  const TONES = ['ok', 'warn', 'bad', 'info', 'none'];
  const entryOf = written => (written && typeof written === 'object' ? written : { text: written });
  const hasText = entry => typeof entry.text === 'string' && entry.text.trim() !== '';
  const mustBeText = (spec, field) => (spec[field] == null || typeof spec[field] === 'string' ? [] : [`${field} must be text`]);

  function entryProblems(written, where, tones) {
    const entry = entryOf(written);
    if (!hasText(entry)) return [`${where}: no text`];
    return [
      ...(!tones || entry.tone == null || TONES.includes(entry.tone) ? [] : [`${where}: tone "${entry.tone}" is not one of ${TONES.join(', ')}`]),
      ...(entry.link == null || typeof entry.link === 'string' ? [] : [`${where}: link must be text`]),
    ];
  }

  function axisProblems(spec, field, name) {
    if (!Array.isArray(spec[field])) return [`${field} must be a list`];
    if (!spec[field].length) return [`no ${field}`];
    return spec[field].flatMap((entry, i) => entryProblems(entry, `${name} ${i + 1}`, false));
  }

  function cellProblems(spec) {
    if (!Array.isArray(spec.cells)) return ['cells must be a list of rows'];
    if (spec.cells.length !== spec.rows.length) return [`has ${spec.cells.length} rows of cells for ${spec.rows.length} rows`];
    return spec.cells.flatMap((row, r) => (Array.isArray(row) && row.length === spec.cols.length
      ? row.flatMap((cell, c) => entryProblems(cell, `row ${r + 1}, column ${c + 1}`, true))
      : [`row ${r + 1} does not have ${spec.cols.length} cells`]));
  }

  function problemsIn(spec) {
    const axes = [...axisProblems(spec, 'rows', 'row'), ...axisProblems(spec, 'cols', 'column')];
    const labels = [...mustBeText(spec, 'rowLabel'), ...mustBeText(spec, 'colLabel')];
    return axes.length ? [...axes, ...labels] : [...cellProblems(spec), ...labels];
  }

  function htmlOf(spec, { resolveLink }) {
    const said = written => { const entry = entryOf(written); return linked(inlineOf(entry.text), entry.link ? resolveLink(entry.link) : null); };
    const corner = [spec.rowLabel, spec.colLabel].filter(Boolean).map(inlineOf).join(' <span class="dk-by">×</span> ');
    const toned = cell => (entryOf(cell).tone ? ` class="t-${entryOf(cell).tone}"` : '');
    const head = `<tr><th>${corner}</th>${spec.cols.map(col => `<th scope="col">${said(col)}</th>`).join('')}</tr>`;
    const body = spec.rows.map((row, r) => `<tr><th scope="row">${said(row)}</th>${spec.cells[r].map(cell => `<td${toned(cell)}>${said(cell)}</td>`).join('')}</tr>`).join('');
    return `<div class="dk-table"><table><thead>${head}</thead><tbody>${body}</tbody></table></div>`;
  }

  const cellsOf = spec => (Array.isArray(spec.cells) ? spec.cells.flat() : []).map(entryOf);
  const toned = (tone, name, means) => ({ name, means, html: `<span class="dk-cell t-${tone}">outcome</span>`, usedIn: spec => cellsOf(spec).some(cell => cell.tone === tone) });
  const symbols = [
    { name: 'Cell', means: 'What happens for the case in its row under the condition in its column.', html: '<span class="dk-cell">outcome</span>', usedIn: () => true },
    toned('ok', 'Safe', 'The safe or intended outcome.'),
    toned('warn', 'Caution', 'It works, with something to be careful of.'),
    toned('bad', 'Problem', 'It is wrong or dangerous, or is refused when it should not be.'),
    toned('info', 'Worth noticing', 'Neither good nor bad, but not what a reader might assume.'),
    toned('none', 'Nothing', 'Nothing happens in this case.'),
  ];

  const notation = { kind: 'matrix', symbols, problemsIn, htmlOf };
  if (inNode) module.exports = notation;
  else root.DiagramKit.NOTATIONS = Object.assign(root.DiagramKit.NOTATIONS || {}, { [notation.kind]: notation });
})(typeof window !== 'undefined' ? window : globalThis);
