const { markProblems } = require('../diagram-kit/lib/notations.js');
const { cyclesIn, loopThrough } = require('../diagram-kit/lib/graphs.js');

const DIRECTIONS = ['down', 'right'];
const KINDS = ['provides', 'requires', 'dependency'];
const DRAWN = {
  provides: { line: 'solid', head: 'none', against: true },
  requires: { line: 'dashed', head: 'open' },
  dependency: { line: 'dashed', head: 'open' },
};

const nameOfPart = part => part.name ?? part.id;
const listOf = value => (Array.isArray(value) ? value : []);
const holdsOthers = component => listOf(component.components).length > 0;
const listed = names => (names.length < 3 ? names.join(' and ') : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`);
const nameOf = (relationship, position) => (typeof relationship.from === 'string' && typeof relationship.to === 'string' ? `relationship ${relationship.from}->${relationship.to}` : `relationship ${position + 1}`);

function everyComponent(components, visit) {
  for (const component of listOf(components)) {
    visit(component);
    if (component) everyComponent(component.components, visit);
  }
}

function partProblems(spec) {
  const problems = [], components = new Map(), interfaces = new Map();
  const seen = id => components.has(id) || interfaces.has(id);
  let position = 0;
  everyComponent(spec.components, component => {
    position++;
    if (!component || typeof component.id !== 'string' || !component.id) { problems.push(`component ${position}: no id`); return; }
    const name = `component ${component.id}`;
    if (seen(component.id)) { problems.push(`${name}: id is used twice`); return; }
    components.set(component.id, component);
    if (component.stereotype != null && typeof component.stereotype !== 'string') problems.push(`${name}: stereotype must be text`);
    problems.push(...markProblems(component, name));
  });
  listOf(spec.interfaces).forEach((each, i) => {
    if (!each || typeof each.id !== 'string' || !each.id) { problems.push(`interface ${i + 1}: no id`); return; }
    const name = `interface ${each.id}`;
    if (seen(each.id)) { problems.push(`${name}: id is used twice`); return; }
    interfaces.set(each.id, each);
    problems.push(...markProblems(each, name));
  });
  return { problems, components, interfaces };
}

function relationshipProblems(relationships, components, interfaces) {
  const written = new Set();
  return relationships.flatMap((relationship, position) => {
    const name = nameOf(relationship, position);
    const missing = ['from', 'to'].find(end => typeof relationship[end] !== 'string' || !relationship[end]);
    if (missing) return [`${name}: no ${missing}`];
    const unknown = [relationship.from, relationship.to].find(end => !components.has(end) && !interfaces.has(end));
    if (unknown) return [`${name}: nothing called "${unknown}"`];
    const { kind } = relationship;
    if (kind == null) return [`${name}: no kind`];
    if (!KINDS.includes(kind)) return [`${name}: kind "${kind}" is not one of ${KINDS.join(', ')}`];
    const joinsInterface = components.has(relationship.from) && interfaces.has(relationship.to);
    if (kind !== 'dependency' && !joinsInterface) return [`${name}: ${kind} runs from a component to an interface`];
    if (kind === 'dependency' && !(components.has(relationship.from) && components.has(relationship.to))) return [`${name}: a dependency runs between components; a component requires an interface`];
    const key = [relationship.from, relationship.to, kind].join('\n');
    if (written.has(key)) return [`${name}: written twice`];
    written.add(key);
    return markProblems(relationship, name);
  });
}

function problemsIn(spec) {
  if (!listOf(spec.components).length) return ['no components'];
  const direction = spec.direction == null || DIRECTIONS.includes(spec.direction) ? [] : [`direction "${spec.direction}" is not one of ${DIRECTIONS.join(', ')}`];
  const { problems, components, interfaces } = partProblems(spec);
  return [...direction, ...problems, ...relationshipProblems(listOf(spec.relationships), components, interfaces)];
}

function remarksOn(spec) {
  const relationships = listOf(spec.relationships), names = new Map(), leaves = [], remarks = [];
  everyComponent(spec.components, component => { names.set(component.id, nameOfPart(component)); if (!holdsOthers(component)) leaves.push(component); });
  const providers = id => relationships.filter(each => each.kind === 'provides' && each.to === id).map(each => each.from);
  const users = id => relationships.filter(each => each.kind === 'requires' && each.to === id).map(each => each.from);
  for (const each of listOf(spec.interfaces)) {
    const name = nameOfPart(each), needed = users(each.id), given = providers(each.id);
    if (!needed.length && !given.length) remarks.push(`${name} is neither provided nor required`);
    else if (needed.length && !given.length) remarks.push(`${name} is required by ${listed(needed.map(id => names.get(id)))}, but nothing in the diagram provides it`);
  }
  const joined = new Set(relationships.flatMap(each => [each.from, each.to]));
  const alone = leaves.filter(component => !joined.has(component.id)).map(nameOfPart);
  if (relationships.length && alone.length) remarks.push(`${listed(alone)} ${alone.length === 1 ? 'is' : 'are'} joined to nothing in the diagram`);
  const needs = [
    ...relationships.filter(each => each.kind === 'dependency').map(({ from, to }) => ({ from, to })),
    ...relationships.filter(each => each.kind === 'requires').flatMap(each => providers(each.to).map(provider => ({ from: each.from, to: provider }))),
  ];
  return [...remarks, ...cyclesIn(needs).map(knot => `Cycle: ${loopThrough(knot, needs).map(id => names.get(id)).join(' → ')}`)];
}

function nodeOf(component) {
  const marks = { status: component.status, tone: component.tone, link: component.link };
  const stereotype = component.stereotype ?? 'component', label = nameOfPart(component);
  return holdsOthers(component)
    ? { id: component.id, shape: 'frame', stereotype, label, ...marks, children: component.components.map(nodeOf) }
    : { id: component.id, shape: 'classifier', stereotype, label, compartments: [], ...marks };
}

function sceneOf(spec) {
  return {
    direction: spec.direction || 'down',
    nodes: [
      ...spec.components.map(nodeOf),
      ...listOf(spec.interfaces).map(each => ({ id: each.id, shape: 'ball', label: nameOfPart(each), status: each.status, tone: each.tone, link: each.link })),
    ],
    edges: listOf(spec.relationships).map(relationship => ({
      id: `${relationship.from}->${relationship.to}`, from: relationship.from, to: relationship.to, label: relationship.label,
      ...DRAWN[relationship.kind], status: relationship.status, tone: relationship.tone, link: relationship.link,
    })),
  };
}

module.exports = { kind: 'component-diagram', problemsIn, sceneOf, remarksOn };
