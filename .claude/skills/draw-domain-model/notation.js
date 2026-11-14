const { markProblems } = require('../diagram-kit/lib/notations.js');

const DIRECTIONS = ['down', 'right'];
const ROOT = 'aggregate root';
const MEMBER_KINDS = [ROOT, 'entity', 'value object'];
const OTHER_KINDS = ['domain service', 'repository', 'factory', 'domain event', 'entity', 'value object'];
const KINDS = ['composition', 'association', 'reference', 'dependency', 'generalisation'];
const HAVE_ENDS = ['composition', 'association'];
const END_FIELDS = ['fromMultiplicity', 'toMultiplicity', 'fromRole', 'toRole', 'navigable'];
const MEMBER_LISTS = { attributes: 'attribute', operations: 'operation' };
const MULTIPLICITY = /^(\d+|\*)(\.\.(\d+|\*))?$/;
const DRAWN = {
  composition: { line: 'solid', tail: 'filled-diamond' },
  association: { line: 'solid' },
  reference: { line: 'dashed', head: 'open' },
  dependency: { line: 'dashed', head: 'open' },
  generalisation: { line: 'solid', head: 'hollow', against: true },
};

const nameOfElement = element => element.name ?? element.id;
const withArticle = word => `${/^[aeiou]/.test(word) ? 'an' : 'a'} ${word}`;
const isText = member => typeof member === 'string' || (member !== null && typeof member === 'object' && typeof member.text === 'string');
const nameOf = (relationship, position) => (typeof relationship.from === 'string' && typeof relationship.to === 'string' ? `relationship ${relationship.from}->${relationship.to}` : `relationship ${position + 1}`);
const listOf = value => (Array.isArray(value) ? value : []);

function elementProblems(element, name, kinds, known) {
  if (known.has(element.id)) return [`${name}: id is used twice`];
  known.add(element.id);
  if (element.kind == null) return [`${name}: no kind`];
  if (!kinds.includes(element.kind)) return [`${name}: kind "${element.kind}" is not one of ${kinds.join(', ')}`, ...markProblems(element, name)];
  const problems = [];
  for (const [field, member] of Object.entries(MEMBER_LISTS)) {
    if (element[field] == null) continue;
    if (!Array.isArray(element[field])) { problems.push(`${name}: ${field} must be a list`); continue; }
    element[field].forEach((value, i) => { if (!isText(value)) problems.push(`${name}: ${member} ${i + 1} must be text, or an object with text`); });
  }
  return [...problems, ...markProblems(element, name)];
}

function aggregateProblems(aggregates, known) {
  return aggregates.flatMap((aggregate, position) => {
    if (!aggregate || typeof aggregate.id !== 'string' || !aggregate.id) return [`aggregate ${position + 1}: no id`];
    const name = `aggregate ${aggregate.id}`;
    if (known.has(aggregate.id)) return [`${name}: id is used twice`];
    known.add(aggregate.id);
    const members = listOf(aggregate.members);
    if (!members.length) return [`${name}: no members`];
    const within = members.flatMap((member, i) => (member && typeof member.id === 'string' && member.id
      ? elementProblems(member, `${name}: member ${member.id}`, MEMBER_KINDS, known)
      : [`${name}: member ${i + 1}: no id`]));
    const roots = members.filter(member => member && member.kind === ROOT).length;
    return [...markProblems(aggregate, name), ...within, ...(roots === 1 ? [] : [`${name}: needs exactly one aggregate root, and has ${roots}`])];
  });
}

function otherProblems(others, known) {
  return others.flatMap((element, position) => (element && typeof element.id === 'string' && element.id
    ? elementProblems(element, `element ${element.id}`, OTHER_KINDS, known)
    : [`element ${position + 1}: no id`]));
}

function relationshipProblems(relationships, known) {
  const written = new Set();
  return relationships.flatMap((relationship, position) => {
    const name = nameOf(relationship, position);
    const missing = ['from', 'to'].find(end => typeof relationship[end] !== 'string' || !relationship[end]);
    if (missing) return [`${name}: no ${missing}`];
    const unknown = [relationship.from, relationship.to].find(end => !known.has(end));
    if (unknown) return [`${name}: nothing called "${unknown}"`];
    const { kind } = relationship;
    if (kind == null) return [`${name}: no kind`];
    if (!KINDS.includes(kind)) return [`${name}: kind "${kind}" is not one of ${KINDS.join(', ')}`];
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

function problemsIn(spec) {
  const aggregates = listOf(spec.aggregates), others = listOf(spec.others);
  if (!aggregates.length && !others.length) return ['no aggregates and no other elements'];
  const direction = spec.direction == null || DIRECTIONS.includes(spec.direction) ? [] : [`direction "${spec.direction}" is not one of ${DIRECTIONS.join(', ')}`];
  const known = new Set();
  const structure = [...aggregateProblems(aggregates, known), ...otherProblems(others, known)];
  return [...direction, ...structure, ...relationshipProblems(listOf(spec.relationships), known)];
}

function placed(spec) {
  const at = new Map();
  for (const aggregate of listOf(spec.aggregates)) {
    at.set(aggregate.id, { name: nameOfElement(aggregate), aggregate, kind: 'aggregate' });
    for (const member of aggregate.members) at.set(member.id, { name: nameOfElement(member), aggregate, kind: member.kind });
  }
  for (const element of listOf(spec.others)) at.set(element.id, { name: nameOfElement(element), aggregate: null, kind: element.kind });
  return at;
}

function remarksOn(spec) {
  const at = placed(spec);
  return listOf(spec.relationships).flatMap(({ from, to, kind }) => {
    const source = at.get(from), target = at.get(to), remarks = [];
    const crosses = target.aggregate && source.aggregate !== target.aggregate;
    if (crosses && MEMBER_KINDS.includes(target.kind) && target.kind !== ROOT) remarks.push(`${source.name} reaches into the ${nameOfElement(target.aggregate)} aggregate past its root, at ${target.name}`);
    if (crosses && source.aggregate && HAVE_ENDS.includes(kind)) remarks.push(`${source.name} holds ${target.name} across an aggregate boundary; refer to another aggregate by identity`);
    if (kind === 'composition' && source.kind === 'value object' && [ROOT, 'entity'].includes(target.kind)) remarks.push(`${source.name} is a value object but owns the entity ${target.name}`);
    return remarks;
  });
}

const endLabel = (multiplicity, role) => [multiplicity, role].filter(Boolean).join(' ') || undefined;

function classifierOf(element) {
  return {
    id: element.id, shape: 'classifier', label: nameOfElement(element), stereotype: element.kind,
    compartments: [element.attributes, element.operations].filter(members => Array.isArray(members) && members.length),
    status: element.status, tone: element.tone, link: element.link,
  };
}

function sceneOf(spec) {
  const drawn = new Map();
  return {
    direction: spec.direction || 'down',
    nodes: [
      ...listOf(spec.aggregates).map(aggregate => ({
        id: aggregate.id, shape: 'boundary', stereotype: 'aggregate', label: nameOfElement(aggregate),
        status: aggregate.status, tone: aggregate.tone, link: aggregate.link, children: aggregate.members.map(classifierOf),
      })),
      ...listOf(spec.others).map(classifierOf),
    ],
    edges: listOf(spec.relationships).map(relationship => {
      const ends = `${relationship.from}->${relationship.to}`, nth = (drawn.get(ends) || 0) + 1;
      drawn.set(ends, nth);
      const hasEnds = HAVE_ENDS.includes(relationship.kind);
      return {
        id: nth > 1 ? `${ends}:${nth}` : ends, from: relationship.from, to: relationship.to,
        label: relationship.label ?? (relationship.kind === 'reference' ? 'by identity' : undefined),
        ...DRAWN[relationship.kind],
        ...(hasEnds ? { head: relationship.navigable ? 'open' : 'none' } : {}),
        fromLabel: hasEnds ? endLabel(relationship.fromMultiplicity, relationship.fromRole) : undefined,
        toLabel: hasEnds ? endLabel(relationship.toMultiplicity, relationship.toRole) : undefined,
        status: relationship.status, tone: relationship.tone, link: relationship.link,
      };
    }),
  };
}

module.exports = { kind: 'domain-model', name: 'Domain model (domain-driven design)', symbols: require('./symbols.js'), problemsIn, sceneOf, remarksOn };
