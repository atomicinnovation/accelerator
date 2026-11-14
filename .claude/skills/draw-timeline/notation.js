(function (root) {
  'use strict';
  const inNode = typeof module === 'object' && module.exports;
  const { escaped, linked } = inNode ? require('../diagram-kit/lib/html.js') : root.DiagramKit;
  const { widthOf } = inNode ? require('../diagram-kit/lib/text.js') : root.DiagramKit;

  const SCALES = ['linear', 'log'], TONES = ['ok', 'warn', 'bad', 'info', 'none'];
  const WIDTH = 760, EDGE = 24, PLOT = WIDTH - 2 * EDGE, AXIS = 58, BAR = 18, LANE = 26, LABEL_BELOW = 16;
  const SPAN_LABEL = 'detail', MARK_LABEL = 'operator';
  const listOf = value => (Array.isArray(value) ? value : []);
  const isNumber = value => typeof value === 'number' && Number.isFinite(value);
  const tidy = number => String(Math.round(number * 100) / 100);
  const plain = text => String(text ?? '').replace(/`/g, '');
  const valuesOf = spec => [spec.from, spec.to, ...listOf(spec.spans).flatMap(span => [span.from, span.to]), ...listOf(spec.marks).map(mark => mark.at), ...listOf(spec.ticks).map(tick => tick.at)].filter(isNumber);

  function placedProblems(item, where, fields) {
    const unplaced = fields.find(field => !isNumber(item[field]));
    if (unplaced) return [`${where}: ${unplaced} must be a number`];
    return [
      ...(fields.length === 2 && item.to < item.from ? [`${where}: ends before it starts`] : []),
      ...(item.tone == null || TONES.includes(item.tone) ? [] : [`${where}: tone "${item.tone}" is not one of ${TONES.join(', ')}`]),
      ...(item.link == null || typeof item.link === 'string' ? [] : [`${where}: link must be text`]),
    ];
  }

  function labelled(items, name, fields) {
    return listOf(items).flatMap((item, position) => {
      const said = item && typeof item.label === 'string' && item.label.trim();
      if (!said && name === 'tick' && item && isNumber(item.at)) return [`tick ${position + 1}: no label`];
      return placedProblems(item || {}, `${name} ${said ? item.label : position + 1}`, fields);
    });
  }

  function problemsIn(spec) {
    if (!listOf(spec.spans).length && !listOf(spec.marks).length) return ['no spans and no marks'];
    const problems = [
      ...(spec.scale == null || SCALES.includes(spec.scale) ? [] : [`scale "${spec.scale}" is not one of ${SCALES.join(', ')}`]),
      ...['from', 'to'].filter(end => spec[end] != null && !isNumber(spec[end])).map(end => `${end} must be a number`),
      ...labelled(spec.ticks, 'tick', ['at']), ...labelled(spec.spans, 'span', ['from', 'to']), ...labelled(spec.marks, 'mark', ['at']),
    ];
    const unplaceable = spec.scale === 'log' ? valuesOf(spec).find(value => value <= 0) : undefined;
    return unplaceable === undefined ? problems : [...problems, `a log scale cannot place ${unplaceable}; start above zero`];
  }

  function scaleOf(spec) {
    const values = valuesOf(spec);
    const low = spec.from ?? Math.min(...values), high = spec.to ?? Math.max(...values);
    const along = spec.scale === 'log'
      ? value => (Math.log(Math.max(value, low)) - Math.log(low)) / ((Math.log(high) - Math.log(low)) || 1)
      : value => (value - low) / ((high - low) || 1);
    return value => EDGE + Math.max(0, Math.min(1, along(value))) * PLOT;
  }

  const textAt = (text, cls, x, y, anchor) => `<text class="${cls}" x="${tidy(x)}" y="${tidy(y)}" text-anchor="${anchor}">${escaped(plain(text))}</text>`;
  const within = (x, half) => Math.max(EDGE + half, Math.min(x, WIDTH - EDGE - half));
  const toned = item => (item.tone ? ` t-${item.tone}` : '');

  function htmlOf(spec, { resolveLink }) {
    const at = scaleOf(spec);
    const opened = (item, markup) => linked(markup, item.link ? resolveLink(item.link) : null);
    const ticks = listOf(spec.ticks).map(tick => {
      const x = at(tick.at);
      return `<g class="tick"><line x1="${tidy(x)}" y1="${AXIS - 5}" x2="${tidy(x)}" y2="${AXIS + 5}"/>${textAt(tick.label, 'tick-label', within(x, widthOf(plain(tick.label), SPAN_LABEL) / 2), AXIS + 19, 'middle')}</g>`;
    });
    const marks = listOf(spec.marks).map((mark, i) => {
      const x = at(mark.at), labelAt = i % 2 ? 14 : 30;
      const label = textAt(mark.label, 'mark-label', within(x, widthOf(plain(mark.label), MARK_LABEL) / 2), labelAt, 'middle');
      return `<g class="mark${toned(mark)}"><line x1="${tidy(x)}" y1="${labelAt + 4}" x2="${tidy(x)}" y2="${AXIS}"/><circle cx="${tidy(x)}" cy="${AXIS}" r="5"/>${opened(mark, label)}</g>`;
    });
    const lanes = [], labelsBelow = [];
    const spans = listOf(spec.spans).map((span, written) => ({ span, written, left: at(span.from), right: at(span.to) })).sort((a, b) => a.left - b.left || a.written - b.written);
    for (const placed of spans) {
      const free = lanes.findIndex(end => end <= placed.left + 1);
      placed.lane = free < 0 ? lanes.length : free;
      lanes[placed.lane] = placed.right;
      placed.width = Math.max(3, placed.right - placed.left);
      placed.labelWidth = widthOf(plain(placed.span.label), SPAN_LABEL);
      placed.inside = placed.labelWidth + 12 <= placed.width;
      if (placed.span.label && !placed.inside) labelsBelow[placed.lane] = true;
    }
    const laneTop = [];
    let top = AXIS + 34;
    lanes.forEach((_, lane) => { laneTop[lane] = top; top += LANE + (labelsBelow[lane] ? LABEL_BELOW : 0); });
    const bars = spans.sort((a, b) => a.written - b.written).map(({ span, left, width, lane, inside, labelWidth }) => {
      const y = laneTop[lane];
      const label = !span.label ? '' : inside
        ? textAt(span.label, 'span-label', left + width / 2, y + 13, 'middle')
        : textAt(span.label, 'span-label', Math.max(EDGE, Math.min(left, WIDTH - EDGE - labelWidth)), y + 32, 'start');
      return `<g class="span${toned(span)}"><rect x="${tidy(left)}" y="${y}" width="${tidy(width)}" height="${BAR}" rx="4"/>${opened(span, label)}</g>`;
    });
    const height = (lanes.length ? top : AXIS + 30) + 8;
    return `<div class="dk-scroll"><svg class="dk-timeline" xmlns="http://www.w3.org/2000/svg" width="${WIDTH}" height="${height}" viewBox="0 0 ${WIDTH} ${height}" role="img" aria-label="${escaped(plain(spec.title))}">`
      + `<line class="axis" x1="${EDGE}" y1="${AXIS}" x2="${WIDTH - EDGE}" y2="${AXIS}"/>${ticks.join('')}${marks.join('')}${bars.join('')}</svg></div>`;
  }

  const drawn = marks => `<svg class="dk-timeline dk-sample" xmlns="http://www.w3.org/2000/svg" width="64" height="24" viewBox="0 0 64 24" aria-hidden="true">${marks}</svg>`;
  const AXIS_SAMPLE = '<line class="axis" x1="2" y1="12" x2="62" y2="12"/>';
  const hasTone = spec => [...listOf(spec.spans), ...listOf(spec.marks)].some(item => item.tone && item.tone !== 'none');
  const symbols = [
    { name: 'Axis', means: 'One scale for every value, in one unit. The ticks name the values that matter.', html: drawn(`${AXIS_SAMPLE}<g class="tick"><line x1="20" y1="7" x2="20" y2="17"/><line x1="44" y1="7" x2="44" y2="17"/></g>`), usedIn: () => true },
    { name: 'Mark', means: 'One moment or value, with its name above it.', html: drawn(`${AXIS_SAMPLE}<g class="mark"><line x1="32" y1="3" x2="32" y2="12"/><circle cx="32" cy="12" r="5"/></g>`), usedIn: spec => listOf(spec.marks).length > 0 },
    { name: 'Span', means: 'A stretch from one value to another. Spans that would overlap sit one under another.', html: drawn('<g class="span"><rect x="6" y="5" width="52" height="14" rx="4"/></g>'), usedIn: spec => listOf(spec.spans).length > 0 },
    { name: 'Colour', means: 'Green is safe, amber a caution, red a problem and blue worth noticing.', html: drawn('<g class="span t-ok"><rect x="4" y="5" width="26" height="14" rx="4"/></g><g class="span t-bad"><rect x="34" y="5" width="26" height="14" rx="4"/></g>'), usedIn: hasTone },
    { name: 'Logarithmic scale', means: 'Equal distances are equal ratios, so values of very different sizes fit on one axis.', html: drawn(`${AXIS_SAMPLE}<g class="tick"><line x1="8" y1="7" x2="8" y2="17"/><line x1="36" y1="7" x2="36" y2="17"/><line x1="50" y1="7" x2="50" y2="17"/><line x1="58" y1="7" x2="58" y2="17"/></g>`), usedIn: spec => spec.scale === 'log' },
  ];

  const notation = { kind: 'timeline', name: 'Timeline', symbols, problemsIn, htmlOf };
  if (inNode) module.exports = notation;
  else root.DiagramKit.NOTATIONS = Object.assign(root.DiagramKit.NOTATIONS || {}, { [notation.kind]: notation });
})(typeof window !== 'undefined' ? window : globalThis);
