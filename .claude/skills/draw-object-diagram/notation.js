const { markProblems } = require('../diagram-kit/lib/notations.js');

const DIRECTIONS = ['down', 'right'];

const isSlot = slot => typeof slot === 'string' || (slot !== null && typeof slot === 'object' && typeof slot.text === 'string');
const nameOf = (link, position) => (typeof link.from === 'string' && typeof link.to === 'string' ? `link ${link.from}->${link.to}` : `link ${position + 1}`);
const listed = names => (names.length < 3 ? names.join(' and ') : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`);

function headingOf(object) {
  if (object.class) return `${object.name ?? ''}: ${object.class}`;
  return object.name ?? object.id;
}

function objectProblems(objects) {
  const problems = [], known = new Set();
  objects.forEach((object, position) => {
    if (!object || typeof object.id !== 'string' || !object.id) { problems.push(`object ${position + 1}: no id`); return; }
    const name = `object ${object.id}`;
    if (known.has(object.id)) { problems.push(`${name}: id is used twice`); return; }
    known.add(object.id);
    for (const field of ['name', 'class']) if (object[field] != null && typeof object[field] !== 'string') problems.push(`${name}: ${field} must be text`);
    if (object.slots != null && !Array.isArray(object.slots)) problems.push(`${name}: slots must be a list`);
    problems.push(...markProblems(object, name));
    if (Array.isArray(object.slots)) object.slots.forEach((slot, i) => { if (!isSlot(slot)) problems.push(`${name}: slot ${i + 1} must be text, or an object with text`); });
  });
  return { problems, known };
}

function linkProblems(links, known) {
  const written = new Set();
  return links.flatMap((link, position) => {
    const name = nameOf(link, position);
    const missing = ['from', 'to'].find(end => typeof link[end] !== 'string' || !link[end]);
    if (missing) return [`${name}: no ${missing}`];
    const unknown = [link.from, link.to].find(end => !known.has(end));
    if (unknown) return [`${name}: no object "${unknown}"`];
    const key = [link.from, link.to, link.label ?? ''].join('\n');
    if (written.has(key)) return [`${name}: written twice`];
    written.add(key);
    return markProblems(link, name);
  });
}

function problemsIn(spec) {
  if (!Array.isArray(spec.objects) || !spec.objects.length) return ['no objects'];
  const direction = spec.direction == null || DIRECTIONS.includes(spec.direction) ? [] : [`direction "${spec.direction}" is not one of ${DIRECTIONS.join(', ')}`];
  const { problems, known } = objectProblems(spec.objects);
  return [...direction, ...problems, ...linkProblems(Array.isArray(spec.links) ? spec.links : [], known)];
}

function remarksOn(spec) {
  const links = spec.links || [];
  const linked = new Set(links.flatMap(link => [link.from, link.to]));
  const alone = spec.objects.filter(object => !linked.has(object.id)).map(headingOf);
  return links.length && alone.length ? [`${listed(alone)} ${alone.length === 1 ? 'is' : 'are'} linked to nothing in the diagram`] : [];
}

function sceneOf(spec) {
  const drawn = new Map();
  return {
    direction: spec.direction || 'down',
    nodes: spec.objects.map(object => ({
      id: object.id, shape: 'classifier', underlined: true, label: headingOf(object),
      compartments: Array.isArray(object.slots) && object.slots.length ? [object.slots] : [],
      status: object.status, tone: object.tone, link: object.link,
    })),
    edges: (spec.links || []).map(link => {
      const ends = `${link.from}->${link.to}`, nth = (drawn.get(ends) || 0) + 1;
      drawn.set(ends, nth);
      return { id: nth > 1 ? `${ends}:${nth}` : ends, from: link.from, to: link.to, label: link.label, line: 'solid', head: 'none',
        fromLabel: link.fromRole, toLabel: link.toRole, status: link.status, tone: link.tone, link: link.link };
    }),
  };
}

module.exports = { kind: 'object-diagram', name: 'UML object diagram', symbols: require('./symbols.js'), problemsIn, sceneOf, remarksOn };
