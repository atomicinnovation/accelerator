const { markProblems } = require('../diagram-kit/lib/notations.js');
const { SHAPES } = require('../diagram-kit/lib/shapes.js');
const { widthOf } = require('../diagram-kit/lib/text.js');

const FORMS = ['terminal', 'rule', 'sequence', 'choice', 'optional', 'oneOrMore', 'zeroOrMore'];
const JOIN = 14, TURN = 12, SIDE = 2 * TURN, BETWEEN_OPTIONS = 10, BAR = 8, BAR_GAP = 4, LEAD = 14, NAME_HEIGHT = 16, UNDER_NAME = 8, BETWEEN_RULES = 24;
const listOf = value => (Array.isArray(value) ? value : []);
const SKIP = { is: 'skip' };

function read(written) {
  if (typeof written === 'string') return { is: 'terminal', text: written, marks: {} };
  if (Array.isArray(written)) return { is: 'sequence', parts: written.map(read) };
  const form = written && typeof written === 'object' ? FORMS.filter(each => each in written) : [];
  if (form.length !== 1) return { is: 'unknown', written, many: form.length > 1 };
  const marks = { status: written.status, tone: written.tone, link: written.link };
  switch (form[0]) {
    case 'terminal': return { is: 'terminal', text: written.terminal, marks };
    case 'rule': return { is: 'rule', text: written.rule, marks };
    case 'sequence': return { is: 'sequence', parts: listOf(written.sequence).map(read) };
    case 'choice': return { is: 'choice', options: listOf(written.choice).map(read), line: 0 };
    case 'optional': return { is: 'choice', optional: true, options: [SKIP, written.optional == null ? { is: 'missing' } : read(written.optional)], line: 0 };
    case 'oneOrMore': return repeatOf(written.oneOrMore, written.separator);
    default: return { is: 'choice', optional: true, options: [SKIP, repeatOf(written.zeroOrMore, written.separator)], line: 0 };
  }
}

function repeatOf(part, separator) {
  return { is: 'repeat', part: part == null ? { is: 'missing' } : read(part), separator: separator == null ? SKIP : read(separator) };
}

function partProblems(part) {
  switch (part.is) {
    case 'terminal': case 'rule':
      if (typeof part.text !== 'string' || !part.text.trim()) return [`a ${part.is} has no text`];
      return markProblems(part.marks, `${part.is} ${part.text}`);
    case 'sequence': return part.parts.length ? part.parts.flatMap(partProblems) : ['a sequence is empty'];
    case 'choice':
      if (part.optional) return part.options.flatMap(partProblems);
      return part.options.length < 2 ? ['a choice needs at least two options'] : part.options.flatMap(partProblems);
    case 'repeat': return [...partProblems(part.part), ...partProblems(part.separator)];
    case 'missing': return ['an optional part is missing'];
    case 'unknown': return [part.many ? `${JSON.stringify(part.written)} is more than one thing` : `${JSON.stringify(part.written)} is not a ${FORMS.slice(0, -1).join(', ')} or ${FORMS[FORMS.length - 1]}`];
    default: return [];
  }
}

function problemsIn(spec) {
  if (!listOf(spec.rules).length) return ['no rules'];
  const known = new Set();
  return spec.rules.flatMap((rule, position) => {
    if (!rule || typeof rule.name !== 'string' || !rule.name.trim()) return [`rule ${position + 1}: no name`];
    const name = `rule ${rule.name}`;
    if (known.has(rule.name)) return [`${name}: name is used twice`];
    known.add(rule.name);
    if (rule.is == null) return [`${name}: does not say what it is`];
    return partProblems(read(rule.is)).map(problem => `${name}: ${problem}`);
  });
}

function usedBy(part) {
  switch (part.is) {
    case 'rule': return [part.text];
    case 'sequence': return part.parts.flatMap(usedBy);
    case 'choice': return part.options.flatMap(usedBy);
    case 'repeat': return [...usedBy(part.part), ...usedBy(part.separator)];
    default: return [];
  }
}

function remarksOn(spec) {
  const used = new Set(spec.rules.flatMap(rule => usedBy(read(rule.is)).filter(name => name !== rule.name)));
  return spec.rules.slice(1).filter(rule => !used.has(rule.name)).map(rule => `${rule.name} is defined, and no other rule uses it`);
}

function measured(part) {
  switch (part.is) {
    case 'terminal': case 'rule': {
      const { width, height } = SHAPES[part.is === 'terminal' ? 'token' : 'ruleref'].sizeOf({ label: part.text });
      return { ...part, width, up: height / 2, down: height / 2 };
    }
    case 'sequence': {
      const parts = part.parts.map(measured);
      return { ...part, parts, width: parts.reduce((sum, each) => sum + each.width, 0) + JOIN * (parts.length - 1), up: Math.max(...parts.map(each => each.up)), down: Math.max(...parts.map(each => each.down)) };
    }
    case 'choice': {
      const options = part.options.map(measured), offsets = [0];
      for (let i = 1; i < options.length; i++) offsets.push(offsets[i - 1] + Math.max(2 * TURN, options[i - 1].down + BETWEEN_OPTIONS + options[i].up));
      const last = options.length - 1;
      return { ...part, options, offsets, width: Math.max(...options.map(each => each.width)) + 2 * SIDE, up: options[0].up, down: offsets[last] + options[last].down };
    }
    case 'repeat': {
      const inner = measured(part.part), separator = measured(part.separator);
      const drop = Math.max(2 * TURN, inner.down + BETWEEN_OPTIONS + separator.up);
      return { ...part, part: inner, separator, drop, width: Math.max(inner.width, separator.width) + 2 * SIDE, up: inner.up, down: drop + separator.down };
    }
    default: return { ...part, width: 0, up: 0, down: 0 };
  }
}

function placed(part, x, y, drawn) {
  const right = x + part.width;
  switch (part.is) {
    case 'terminal': case 'rule':
      drawn.symbols.push({ shape: part.is === 'terminal' ? 'token' : 'ruleref', label: part.text, ...part.marks, x, y: y - part.up, width: part.width, height: part.up + part.down });
      return;
    case 'sequence': {
      let at = x;
      part.parts.forEach((each, i) => {
        if (i) drawn.lines.push([[at - JOIN, y], [at, y]]);
        placed(each, at, y, drawn);
        at += each.width + JOIN;
      });
      return;
    }
    case 'choice':
      part.options.forEach((option, i) => {
        const level = y + part.offsets[i], from = x + SIDE, to = from + option.width;
        placed(option, from, level, drawn);
        if (i === 0) {
          drawn.lines.push([[x, y], [from, y]], [[to, y], [right, y]]);
          if (option.is === 'skip') drawn.lines.push([[from, y], [right - SIDE, y]]);
          return;
        }
        drawn.lines.push([[x, y], [x + TURN, y], [x + TURN, level], [from, level]]);
        drawn.lines.push([[to, level], [right - TURN, level], [right - TURN, y], [right, y]]);
      });
      return;
    case 'repeat': {
      const from = x + SIDE, to = from + part.part.width, below = y + part.drop;
      const back = from + (part.width - 2 * SIDE - part.separator.width) / 2, backEnd = back + part.separator.width;
      placed(part.part, from, y, drawn);
      drawn.lines.push([[x, y], [from, y]], [[to, y], [right, y]]);
      const down = [[right - SIDE, y], [right - TURN, y], [right - TURN, below]], up = [[x + TURN, below], [x + TURN, y], [x + SIDE, y]];
      if (part.separator.is === 'skip') { drawn.lines.push([...down, ...up]); return; }
      placed(part.separator, back, below, drawn);
      drawn.lines.push([...down, [backEnd, below]], [[back, below], ...up]);
      return;
    }
    default:
  }
}

function placedSceneOf(spec) {
  const nodes = [];
  let top = 0, widest = 0;
  for (const rule of spec.rules) {
    const track = measured(read(rule.is)), drawn = { symbols: [], lines: [] };
    const y = top + NAME_HEIGHT + UNDER_NAME + Math.max(track.up, BAR), start = BAR_GAP + LEAD, end = start + track.width + LEAD;
    placed(track, start, y, drawn);
    const bars = [0, BAR_GAP, end, end + BAR_GAP].map(x => [[x, y - BAR], [x, y + BAR]]);
    const bottom = y + Math.max(track.down, BAR), width = end + BAR_GAP;
    nodes.push(
      { id: `${rule.name}/rails`, shape: 'rails', inert: true, behind: true, lines: [...bars, [[BAR_GAP, y], [start, y]], [[start + track.width, y], [end, y]], ...drawn.lines], x: 0, y: top + NAME_HEIGHT, width, height: bottom - top - NAME_HEIGHT },
      { id: rule.name, shape: 'rulename', inert: true, label: rule.name, x: 0, y: top, width: Math.ceil(widthOf(rule.name, 'title')) + 1, height: NAME_HEIGHT },
      ...drawn.symbols.map((symbol, i) => ({ id: `${rule.name}/${i + 1}`, inert: true, ...symbol })),
    );
    widest = Math.max(widest, width, Math.ceil(widthOf(rule.name, 'title')) + 1);
    top = bottom + BETWEEN_RULES;
  }
  return { width: widest + 1, height: top - BETWEEN_RULES + 1, nodes, edges: [] };
}

module.exports = { kind: 'railroad-diagram', name: 'Railroad (syntax) diagram', symbols: require('./symbols.js'), problemsIn, sceneOf: placedSceneOf, placedSceneOf, remarksOn };
