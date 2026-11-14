const { markProblems } = require('../diagram-kit/lib/notations.js');

const DIRECTIONS = ['down', 'right'];
const CONTAINER_KINDS = ['container', 'store'];
const TYPES = { container: 'Container', store: 'Store' };
const listOf = value => (Array.isArray(value) ? value : []);
const listed = names => (names.length < 3 ? names.join(' and ') : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`);
const showsContainers = system => listOf(system.containers).length > 0;
const nameOf = (relationship, position) => (typeof relationship.from === 'string' && typeof relationship.to === 'string' ? `relationship ${relationship.from}->${relationship.to}` : `relationship ${position + 1}`);

function thingProblems(thing, name, known) {
  if (known.has(thing.id)) return [`${name}: id is used twice`];
  known.add(thing.id);
  if (typeof thing.name !== 'string' || !thing.name.trim()) return [`${name}: no name`];
  return [
    ...['description', 'technology'].filter(field => thing[field] != null && typeof thing[field] !== 'string').map(field => `${name}: ${field} must be text`),
    ...(thing.external == null || typeof thing.external === 'boolean' ? [] : [`${name}: external must be true or false`]),
    ...(thing.kind == null || CONTAINER_KINDS.includes(thing.kind) ? [] : [`${name}: kind "${thing.kind}" is not one of ${CONTAINER_KINDS.join(', ')}`]),
    ...markProblems(thing, name),
  ];
}

function everyThing(spec, visit) {
  listOf(spec.people).forEach((person, i) => visit(person, person && person.id ? `person ${person.id}` : `person ${i + 1}`));
  listOf(spec.systems).forEach((system, i) => {
    const name = system && system.id ? `system ${system.id}` : `system ${i + 1}`;
    listOf(system && system.containers).forEach((container, j) => visit(container, `${name}: container ${container && container.id ? container.id : j + 1}`, system));
    visit(system, name);
  });
}

function problemsIn(spec) {
  if (!listOf(spec.people).length && !listOf(spec.systems).length) return ['no people and no systems'];
  const direction = spec.direction == null || DIRECTIONS.includes(spec.direction) ? [] : [`direction "${spec.direction}" is not one of ${DIRECTIONS.join(', ')}`];
  const known = new Set(), problems = [];
  listOf(spec.people).forEach((person, i) => {
    if (!person || typeof person.id !== 'string' || !person.id) problems.push(`person ${i + 1}: no id`);
    else problems.push(...thingProblems(person, `person ${person.id}`, known));
  });
  listOf(spec.systems).forEach((system, i) => {
    if (!system || typeof system.id !== 'string' || !system.id) { problems.push(`system ${i + 1}: no id`); return; }
    const name = `system ${system.id}`;
    const own = thingProblems(system, name, known);
    const held = listOf(system.containers).flatMap((container, j) => (container && typeof container.id === 'string' && container.id
      ? thingProblems(container, `${name}: container ${container.id}`, known)
      : [`${name}: container ${j + 1}: no id`]));
    problems.push(...own, ...held);
  });
  const written = new Set();
  const between = listOf(spec.relationships).flatMap((relationship, position) => {
    const name = nameOf(relationship, position);
    const missing = ['from', 'to'].find(end => typeof relationship[end] !== 'string' || !relationship[end]);
    if (missing) return [`${name}: no ${missing}`];
    const unknown = [relationship.from, relationship.to].find(end => !known.has(end));
    if (unknown) return [`${name}: nothing called "${unknown}"`];
    if (typeof relationship.label !== 'string' || !relationship.label.trim()) return [`${name}: no label; say what the relationship is for`];
    const key = [relationship.from, relationship.to, relationship.label].join('\n');
    if (written.has(key)) return [`${name}: written twice`];
    written.add(key);
    return [...markProblems(relationship, name), ...(relationship.technology == null || typeof relationship.technology === 'string' ? [] : [`${name}: technology must be text`])];
  });
  return [...direction, ...problems, ...between];
}

function remarksOn(spec) {
  const relationships = listOf(spec.relationships), names = new Map(), opened = new Set(), leaves = [];
  everyThing(spec, (thing, _, system) => {
    names.set(thing.id, thing.name);
    if (!system && showsContainers(thing)) opened.add(thing.id);
    else leaves.push(thing);
  });
  const joined = new Set(relationships.flatMap(each => [each.from, each.to]));
  const alone = leaves.filter(thing => !joined.has(thing.id)).map(thing => thing.name);
  const vague = relationships.flatMap(({ from, to }) => [from, to].filter(end => opened.has(end)).map(() => `${names.get(from)} → ${names.get(to)}: its containers are shown, so say which one`));
  return [...(relationships.length && alone.length ? [`${listed(alone)} ${alone.length === 1 ? 'is' : 'are'} joined to nothing in the diagram`] : []), ...vague];
}

const marksOf = thing => ({ status: thing.status, tone: thing.tone, link: thing.link });
const elementOf = (thing, shape, type) => ({ id: thing.id, shape, label: thing.name, type, description: thing.description, muted: thing.external ? true : undefined, ...marksOf(thing) });

function sceneOf(spec) {
  const containerOf = container => elementOf(container, 'element', [TYPES[container.kind ?? 'container'], container.technology].filter(Boolean).join(': '));
  return {
    direction: spec.direction || 'down',
    nodes: [
      ...listOf(spec.people).map(person => elementOf(person, 'person', 'Person')),
      ...listOf(spec.systems).map(system => (showsContainers(system)
        ? { id: system.id, shape: 'boundary', stereotype: 'software system', label: system.name, ...marksOf(system), children: system.containers.map(containerOf) }
        : elementOf(system, 'element', 'Software System'))),
    ],
    edges: listOf(spec.relationships).map(relationship => ({
      id: `${relationship.from}->${relationship.to}`, from: relationship.from, to: relationship.to,
      label: relationship.technology ? `${relationship.label} [${relationship.technology}]` : relationship.label,
      line: 'solid', head: 'open', ...marksOf(relationship),
    })),
  };
}

module.exports = { kind: 'c4-diagram', symbols: require('./symbols.js'), problemsIn, sceneOf, remarksOn };
