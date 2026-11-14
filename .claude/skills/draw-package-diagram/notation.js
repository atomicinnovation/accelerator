const { markProblems } = require('../diagram-kit/lib/notations.js');
const { cyclesIn, loopThrough } = require('../diagram-kit/lib/graphs.js');

const DIRECTIONS = ['down', 'right'];
const KINDS = ['dependency', 'import', 'access', 'merge'];

const nameOfPackage = each => each.name ?? each.id;
const listOf = value => (Array.isArray(value) ? value : []);
const holdsOthers = each => listOf(each.packages).length > 0;
const nameOf = (relationship, position) => (typeof relationship.from === 'string' && typeof relationship.to === 'string' ? `relationship ${relationship.from}->${relationship.to}` : `relationship ${position + 1}`);

function everyPackage(packages, visit, holder = null) {
  for (const each of listOf(packages)) {
    visit(each, holder);
    if (each) everyPackage(each.packages, visit, each);
  }
}

function packageProblems(packages) {
  const problems = [], known = new Set();
  let position = 0;
  everyPackage(packages, each => {
    position++;
    if (!each || typeof each.id !== 'string' || !each.id) { problems.push(`package ${position}: no id`); return; }
    const name = `package ${each.id}`;
    if (known.has(each.id)) { problems.push(`${name}: id is used twice`); return; }
    known.add(each.id);
    problems.push(...markProblems(each, name));
  });
  return { problems, known };
}

function relationshipProblems(relationships, known) {
  const written = new Set();
  return relationships.flatMap((relationship, position) => {
    const name = nameOf(relationship, position);
    const missing = ['from', 'to'].find(end => typeof relationship[end] !== 'string' || !relationship[end]);
    if (missing) return [`${name}: no ${missing}`];
    const unknown = [relationship.from, relationship.to].find(end => !known.has(end));
    if (unknown) return [`${name}: no package "${unknown}"`];
    const kind = relationship.kind ?? 'dependency';
    if (!KINDS.includes(kind)) return [`${name}: kind "${kind}" is not one of ${KINDS.join(', ')}`];
    const key = [relationship.from, relationship.to, kind].join('\n');
    if (written.has(key)) return [`${name}: written twice`];
    written.add(key);
    return markProblems(relationship, name);
  });
}

function problemsIn(spec) {
  if (!listOf(spec.packages).length) return ['no packages'];
  const direction = spec.direction == null || DIRECTIONS.includes(spec.direction) ? [] : [`direction "${spec.direction}" is not one of ${DIRECTIONS.join(', ')}`];
  const { problems, known } = packageProblems(spec.packages);
  return [...direction, ...problems, ...relationshipProblems(listOf(spec.relationships), known)];
}

function remarksOn(spec) {
  const relationships = listOf(spec.relationships), names = new Map(), holderOf = new Map();
  everyPackage(spec.packages, (each, holder) => { names.set(each.id, nameOfPackage(each)); holderOf.set(each.id, holder ? holder.id : null); });
  const isWithin = (inner, outer) => { for (let at = holderOf.get(inner); at; at = holderOf.get(at)) if (at === outer) return true; return false; };
  const nested = relationships.flatMap(({ from, to }) => {
    if (isWithin(to, from)) return [`${names.get(from)} depends on ${names.get(to)}, which it holds`];
    if (isWithin(from, to)) return [`${names.get(from)} depends on ${names.get(to)}, which holds it`];
    return [];
  });
  return [...cyclesIn(relationships).map(knot => `Cycle: ${loopThrough(knot, relationships).map(id => names.get(id)).join(' → ')}`), ...nested];
}

function nodeOf(each) {
  const marks = { status: each.status, tone: each.tone, link: each.link }, label = nameOfPackage(each);
  return holdsOthers(each) ? { id: each.id, shape: 'package', label, ...marks, children: each.packages.map(nodeOf) } : { id: each.id, shape: 'folder', label, ...marks };
}

function sceneOf(spec) {
  const drawn = new Map();
  return {
    direction: spec.direction || 'down',
    nodes: spec.packages.map(nodeOf),
    edges: listOf(spec.relationships).map(relationship => {
      const kind = relationship.kind ?? 'dependency';
      const ends = `${relationship.from}->${relationship.to}`, nth = (drawn.get(ends) || 0) + 1;
      drawn.set(ends, nth);
      return {
        id: nth > 1 ? `${ends}:${nth}` : ends, from: relationship.from, to: relationship.to,
        label: relationship.label ?? (kind === 'dependency' ? undefined : `«${kind}»`), line: 'dashed', head: 'open',
        status: relationship.status, tone: relationship.tone, link: relationship.link,
      };
    }),
  };
}

module.exports = { kind: 'package-diagram', problemsIn, sceneOf, remarksOn };
