const { markProblems } = require('../diagram-kit/lib/notations.js');

const DIRECTIONS = ['down', 'right'];
const KINDS = ['generalisation', 'realisation', 'association', 'aggregation', 'composition', 'dependency'];
const HAVE_ENDS = ['association', 'aggregation', 'composition'];
const SPECIALISE = ['generalisation', 'realisation'];
const END_FIELDS = ['fromMultiplicity', 'toMultiplicity', 'fromRole', 'toRole', 'navigable'];
const MEMBER_LISTS = { attributes: 'attribute', operations: 'operation' };
const MULTIPLICITY = /^(\d+|\*)(\.\.(\d+|\*))?$/;
const DRAWN = {
  generalisation: { line: 'solid', head: 'hollow', against: true },
  realisation: { line: 'dashed', head: 'hollow', against: true },
  dependency: { line: 'dashed', head: 'open' },
  association: { line: 'solid' },
  aggregation: { line: 'solid', tail: 'diamond' },
  composition: { line: 'solid', tail: 'filled-diamond' },
};

const nameOfClass = each => each.name ?? each.id;
const listed = names => (names.length < 3 ? names.join(' and ') : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`);
const withArticle = word => `${/^[aeiou]/.test(word) ? 'an' : 'a'} ${word}`;
const isMember = member => typeof member === 'string' || (member !== null && typeof member === 'object' && typeof member.text === 'string');
const nameOf = (relationship, position) => (typeof relationship.from === 'string' && typeof relationship.to === 'string' ? `relationship ${relationship.from}->${relationship.to}` : `relationship ${position + 1}`);

function classProblems(classes) {
  const problems = [], known = new Set();
  classes.forEach((each, position) => {
    if (!each || typeof each.id !== 'string' || !each.id) { problems.push(`class ${position + 1}: no id`); return; }
    const name = `class ${each.id}`;
    if (known.has(each.id)) { problems.push(`${name}: id is used twice`); return; }
    known.add(each.id);
    if (each.abstract != null && typeof each.abstract !== 'boolean') problems.push(`${name}: abstract must be true or false`);
    if (each.stereotype != null && typeof each.stereotype !== 'string') problems.push(`${name}: stereotype must be text`);
    for (const [field, member] of Object.entries(MEMBER_LISTS)) {
      if (each[field] == null) continue;
      if (!Array.isArray(each[field])) { problems.push(`${name}: ${field} must be a list`); continue; }
      each[field].forEach((value, i) => { if (!isMember(value)) problems.push(`${name}: ${member} ${i + 1} must be text, or an object with text`); });
    }
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
    if (unknown) return [`${name}: no class "${unknown}"`];
    const { kind } = relationship;
    if (kind == null) return [`${name}: no kind`];
    if (!KINDS.includes(kind)) return [`${name}: kind "${kind}" is not one of ${KINDS.join(', ')}`];
    if (SPECIALISE.includes(kind) && relationship.from === relationship.to) return [`${name}: a class cannot specialise itself`];
    const key = [relationship.from, relationship.to, kind, relationship.label ?? ''].join('\n');
    if (written.has(key)) return [`${name}: written twice`];
    written.add(key);
    if (!HAVE_ENDS.includes(kind)) return [...END_FIELDS.filter(field => relationship[field] != null).map(field => `${name}: ${withArticle(kind)} has no ${field}`), ...markProblems(relationship, name)];
    return [
      ...markProblems(relationship, name),
      ...['fromMultiplicity', 'toMultiplicity'].filter(field => relationship[field] != null && !MULTIPLICITY.test(relationship[field]))
        .map(field => `${name}: ${field} "${relationship[field]}" is not written like 1, 0..1 or 0..*`),
      ...(relationship.navigable == null || typeof relationship.navigable === 'boolean' ? [] : [`${name}: navigable must be true or false`]),
    ];
  });
}

function ringOfSpecialisation(relationships, order) {
  const general = new Map();
  for (const { from, to, kind } of relationships) if (SPECIALISE.includes(kind)) general.set(from, [...(general.get(from) || []), to]);
  const walk = (at, path) => {
    for (const next of general.get(at) || []) {
      if (next === path[0]) return [...path, next];
      if (path.includes(next)) continue;
      const found = walk(next, [...path, next]);
      if (found) return found;
    }
    return null;
  };
  for (const start of order) { const ring = walk(start, [start]); if (ring) return ring; }
  return null;
}

function problemsIn(spec) {
  if (!Array.isArray(spec.classes) || !spec.classes.length) return ['no classes'];
  const direction = spec.direction == null || DIRECTIONS.includes(spec.direction) ? [] : [`direction "${spec.direction}" is not one of ${DIRECTIONS.join(', ')}`];
  const { problems, known } = classProblems(spec.classes);
  const relationships = Array.isArray(spec.relationships) ? spec.relationships : [];
  const between = relationshipProblems(relationships, known);
  const sound = relationships.filter(each => known.has(each.from) && known.has(each.to) && each.from !== each.to);
  const ring = between.length ? null : ringOfSpecialisation(sound, [...known]);
  return [...direction, ...problems, ...between, ...(ring ? [`generalisation runs in a ring: ${ring.join(' → ')}`] : [])];
}

function remarksOn(spec) {
  const relationships = spec.relationships || [], remarks = [];
  const names = new Map(spec.classes.map(each => [each.id, nameOfClass(each)]));
  const related = new Set(relationships.flatMap(each => [each.from, each.to]));
  const alone = spec.classes.filter(each => !related.has(each.id)).map(nameOfClass);
  if (relationships.length && alone.length) remarks.push(`${listed(alone)} ${alone.length === 1 ? 'is' : 'are'} related to nothing in the diagram`);
  const owners = new Map();
  for (const { from, to, kind } of relationships) if (kind === 'composition') owners.set(to, [...(owners.get(to) || []), from]);
  for (const [part, wholes] of owners) if (wholes.length > 1) remarks.push(`${names.get(part)} is a composite part of ${listed(wholes.map(id => names.get(id)))}; a part has one owner`);
  return remarks;
}

const endLabel = (multiplicity, role) => [multiplicity, role].filter(Boolean).join(' ') || undefined;

function sceneOf(spec) {
  const drawn = new Map();
  return {
    direction: spec.direction || 'down',
    nodes: spec.classes.map(each => ({
      id: each.id, shape: 'classifier', label: nameOfClass(each), stereotype: each.stereotype, abstract: each.abstract === true ? true : undefined,
      compartments: [each.attributes, each.operations].filter(members => Array.isArray(members) && members.length),
      status: each.status, tone: each.tone, link: each.link,
    })),
    edges: (spec.relationships || []).map(relationship => {
      const ends = `${relationship.from}->${relationship.to}`, nth = (drawn.get(ends) || 0) + 1;
      drawn.set(ends, nth);
      const hasEnds = HAVE_ENDS.includes(relationship.kind);
      return {
        id: nth > 1 ? `${ends}:${nth}` : ends, from: relationship.from, to: relationship.to, label: relationship.label,
        ...DRAWN[relationship.kind],
        ...(hasEnds ? { head: relationship.navigable ? 'open' : 'none' } : {}),
        fromLabel: hasEnds ? endLabel(relationship.fromMultiplicity, relationship.fromRole) : undefined,
        toLabel: hasEnds ? endLabel(relationship.toMultiplicity, relationship.toRole) : undefined,
        status: relationship.status, tone: relationship.tone, link: relationship.link,
      };
    }),
  };
}

module.exports = { kind: 'class-diagram', symbols: require('./symbols.js'), problemsIn, sceneOf, remarksOn };
