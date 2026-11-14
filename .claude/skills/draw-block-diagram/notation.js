const { markProblems } = require('../diagram-kit/lib/notations.js');

const DIRECTIONS = ['down', 'right'];
const listOf = value => (Array.isArray(value) ? value : []);
const listed = names => (names.length < 3 ? names.join(' and ') : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`);
const nameOf = block => block.name ?? block.id;
const holdsOthers = block => listOf(block.blocks).length > 0;
const mustBeText = (item, fields, where) => fields.filter(field => item[field] != null && typeof item[field] !== 'string').map(field => `${where}: ${field} must be text`);

function everyBlock(blocks, visit) {
  for (const block of listOf(blocks)) {
    visit(block);
    everyBlock(block && block.blocks, visit);
  }
}

function blockProblems(blocks) {
  const problems = [], known = new Set();
  let position = 0;
  everyBlock(blocks, block => {
    position++;
    if (!block || typeof block.id !== 'string' || !block.id) { problems.push(`block ${position}: no id`); return; }
    const where = `block ${block.id}`;
    if (known.has(block.id)) { problems.push(`${where}: id is used twice`); return; }
    known.add(block.id);
    problems.push(...mustBeText(block, ['name', 'detail'], where), ...markProblems(block, where));
  });
  return { problems, known };
}

function arrowProblems(arrows, known) {
  const written = new Set();
  return listOf(arrows).flatMap((arrow, position) => {
    const missing = ['from', 'to'].find(end => !arrow || typeof arrow[end] !== 'string' || !arrow[end]);
    if (missing) return [`arrow ${position + 1}: no ${missing}`];
    const where = `arrow ${arrow.from}->${arrow.to}`;
    const unknown = [arrow.from, arrow.to].find(end => !known.has(end));
    if (unknown) return [`${where}: no block "${unknown}"`];
    if (arrow.from === arrow.to) return [`${where}: an arrow does not return to where it began`];
    const key = [arrow.from, arrow.to, arrow.label ?? ''].join('\n');
    if (written.has(key)) return [`${where}: written twice`];
    written.add(key);
    return [
      ...mustBeText(arrow, ['label'], where),
      ...(arrow.dashed == null || typeof arrow.dashed === 'boolean' ? [] : [`${where}: dashed must be true or false`]),
      ...markProblems(arrow, where),
    ];
  });
}

function problemsIn(spec) {
  if (!listOf(spec.blocks).length) return ['no blocks'];
  const direction = spec.direction == null || DIRECTIONS.includes(spec.direction) ? [] : [`direction "${spec.direction}" is not one of ${DIRECTIONS.join(', ')}`];
  const { problems, known } = blockProblems(spec.blocks);
  return [...direction, ...problems, ...arrowProblems(spec.arrows, known)];
}

function remarksOn(spec) {
  const arrows = listOf(spec.arrows), joined = new Set(arrows.flatMap(arrow => [arrow.from, arrow.to])), alone = [];
  everyBlock(spec.blocks, block => { if (!holdsOthers(block) && !joined.has(block.id)) alone.push(nameOf(block)); });
  return arrows.length && alone.length ? [`${listed(alone)} ${alone.length === 1 ? 'is' : 'are'} joined to nothing in the diagram`] : [];
}

function nodeOf(block) {
  const node = { id: block.id, shape: holdsOthers(block) ? 'group' : 'box', label: nameOf(block), detail: block.detail, status: block.status, tone: block.tone, link: block.link };
  return holdsOthers(block) ? { ...node, children: block.blocks.map(nodeOf) } : node;
}

function sceneOf(spec) {
  const used = new Map();
  const idOf = arrow => {
    const pair = `${arrow.from}->${arrow.to}`, count = (used.get(pair) || 0) + 1;
    used.set(pair, count);
    return count === 1 ? pair : `${pair}#${count}`;
  };
  return {
    direction: spec.direction || 'down',
    nodes: spec.blocks.map(nodeOf),
    edges: listOf(spec.arrows).map(arrow => ({ id: idOf(arrow), from: arrow.from, to: arrow.to, label: arrow.label, line: arrow.dashed ? 'dashed' : 'solid', head: 'filled', status: arrow.status, tone: arrow.tone, link: arrow.link })),
  };
}

module.exports = { kind: 'block-diagram', symbols: require('./symbols.js'), problemsIn, sceneOf, remarksOn };
