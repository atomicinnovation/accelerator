const { markProblems } = require('../diagram-kit/lib/notations.js');

const DIRECTIONS = ['right', 'down'];
const ELEMENTS = [['externals', 'external', 'external'], ['processes', 'process', 'bubble'], ['stores', 'store', 'store']];
const listOf = value => (Array.isArray(value) ? value : []);
const listed = names => (names.length < 3 ? names.join(' and ') : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`);
const hasId = thing => Boolean(thing) && typeof thing.id === 'string' && thing.id !== '';
const hasName = thing => typeof thing.name === 'string' && thing.name.trim() !== '';
const marksOf = thing => ({ status: thing.status, tone: thing.tone, link: thing.link });

const elementsOf = spec => ELEMENTS.flatMap(([field, role, shape]) => listOf(spec[field]).map((element, position) => ({ element, role, shape, position })));
const boundariesOf = spec => listOf(spec.boundaries).filter(hasId);

function holdersOf(spec) {
  const holders = new Map();
  for (const boundary of boundariesOf(spec)) for (const held of listOf(boundary.holds)) if (!holders.has(held)) holders.set(held, boundary.id);
  return holders;
}

function enclosing(id, holders) {
  const around = [];
  for (let holder = holders.get(id); holder != null && !around.includes(holder); holder = holders.get(holder)) around.push(holder);
  return around;
}

function elementProblems(spec, known) {
  return elementsOf(spec).flatMap(({ element, role, position }) => {
    if (!hasId(element)) return [`${role} ${position + 1}: no id`];
    const name = `${role} ${element.id}`;
    if (known.has(element.id)) return [`${name}: id is used twice`];
    known.set(element.id, role);
    return hasName(element) ? markProblems(element, name) : [`${name}: no name`];
  });
}

function boundaryProblems(spec, known) {
  const boundaries = listOf(spec.boundaries), holders = new Map();
  boundaries.filter(hasId).forEach(boundary => { if (!known.has(boundary.id)) known.set(boundary.id, 'boundary'); });
  const inward = holdersOf(spec);
  return boundaries.flatMap((boundary, position) => {
    if (!hasId(boundary)) return [`boundary ${position + 1}: no id`];
    const name = `boundary ${boundary.id}`;
    if (!hasName(boundary)) return [`${name}: no name`];
    if (!listOf(boundary.holds).length) return [`${name}: holds nothing`];
    if (enclosing(boundary.id, inward).includes(boundary.id)) return [`${name}: is inside itself`];
    const held = boundary.holds.flatMap(id => {
      if (!known.has(id)) return [`${name}: nothing called "${id}"`];
      if (holders.has(id)) return [`${name}: ${id} is already inside boundary ${holders.get(id)}`];
      holders.set(id, boundary.id);
      return [];
    });
    return [...held, ...markProblems(boundary, name)];
  });
}

function flowProblems(spec, known) {
  const written = new Set();
  return listOf(spec.flows).flatMap((flow, position) => {
    const missing = ['from', 'to'].find(end => !flow || typeof flow[end] !== 'string' || !flow[end]);
    if (missing) return [`flow ${position + 1}: no ${missing}`];
    const name = `flow ${flow.from}->${flow.to}`;
    const unknown = [flow.from, flow.to].find(end => !known.has(end));
    if (unknown) return [`${name}: nothing called "${unknown}"`];
    const boundary = [flow.from, flow.to].find(end => known.get(end) === 'boundary');
    if (boundary) return [`${name}: ${boundary} is a boundary; a flow joins elements`];
    if (flow.from === flow.to) return [`${name}: a flow does not return to where it began`];
    if (typeof flow.data !== 'string' || !flow.data.trim()) return [`${name}: no data; name what flows`];
    const key = [flow.from, flow.to, flow.data].join('\n');
    if (written.has(key)) return [`${name}: written twice`];
    written.add(key);
    return markProblems(flow, name);
  });
}

function problemsIn(spec) {
  if (!listOf(spec.processes).length) return ['no processes; a data flow diagram shows what is done to data'];
  const direction = spec.direction == null || DIRECTIONS.includes(spec.direction) ? [] : [`direction "${spec.direction}" is not one of ${DIRECTIONS.join(', ')}`];
  const known = new Map();
  return [...direction, ...elementProblems(spec, known), ...boundaryProblems(spec, known), ...flowProblems(spec, known)];
}

function remarksOn(spec) {
  const elements = elementsOf(spec), flows = listOf(spec.flows), holders = holdersOf(spec);
  const roles = new Map(elements.map(({ element, role }) => [element.id, role])), names = new Map(elements.map(({ element }) => [element.id, element.name]));
  const told = flow => `${names.get(flow.from)} → ${names.get(flow.to)} (${flow.data})`;
  const receives = new Set(flows.map(flow => flow.to)), sends = new Set(flows.map(flow => flow.from));
  const alone = elements.filter(({ element }) => !receives.has(element.id) && !sends.has(element.id)).map(({ element }) => element.name);
  const unprocessed = flows.filter(flow => roles.get(flow.from) !== 'process' && roles.get(flow.to) !== 'process')
    .map(flow => `${told(flow)}: neither end is a process, and data moves only through one`);
  const unbalanced = elements.filter(({ role }) => role === 'process').flatMap(({ element }) => {
    if (receives.has(element.id) === sends.has(element.id)) return [];
    return [receives.has(element.id) ? `${element.name} sends no data on, so what it receives goes nowhere` : `${element.name} receives no data, so it makes what it sends from nothing`];
  });
  const crossing = flows.filter(flow => enclosing(flow.from, holders).join('\n') !== enclosing(flow.to, holders).join('\n')).map(told);
  return [
    ...unprocessed, ...unbalanced,
    ...(flows.length && alone.length ? [`${listed(alone)} ${alone.length === 1 ? 'is' : 'are'} joined to nothing in the diagram`] : []),
    ...(crossing.length ? [`${crossing.length} ${crossing.length === 1 ? 'flow crosses' : 'flows cross'} a trust boundary: ${listed(crossing)}`] : []),
  ];
}

function sceneOf(spec) {
  const holders = holdersOf(spec), drawn = new Map();
  for (const { element, shape } of elementsOf(spec)) drawn.set(element.id, { id: element.id, shape, label: element.name, ...marksOf(element) });
  for (const boundary of boundariesOf(spec)) drawn.set(boundary.id, { id: boundary.id, shape: 'boundary', label: boundary.name, ...marksOf(boundary), children: [] });
  for (const boundary of boundariesOf(spec)) {
    drawn.get(boundary.id).children = listOf(boundary.holds).filter(id => holders.get(id) === boundary.id && drawn.has(id)).map(id => drawn.get(id));
  }
  const used = new Map();
  const idOf = flow => {
    const pair = `${flow.from}->${flow.to}`, count = (used.get(pair) || 0) + 1;
    used.set(pair, count);
    return count === 1 ? pair : `${pair}#${count}`;
  };
  const onward = new Map();
  const reaches = (from, to, seen = new Set()) => from === to || [...(onward.get(from) || [])].some(next => !seen.has(next) && reaches(next, to, seen.add(next)));
  const returns = flow => {
    if (reaches(flow.to, flow.from)) return true;
    onward.set(flow.from, (onward.get(flow.from) || new Set()).add(flow.to));
    return undefined;
  };
  return {
    direction: spec.direction || 'right',
    nodes: [...boundariesOf(spec), ...elementsOf(spec).map(({ element }) => element)].filter(thing => !holders.has(thing.id)).map(thing => drawn.get(thing.id)),
    edges: listOf(spec.flows).map(flow => ({ id: idOf(flow), from: flow.from, to: flow.to, label: flow.data, line: 'solid', head: 'filled', against: returns(flow), ...marksOf(flow) })),
  };
}

module.exports = { kind: 'data-flow-diagram', problemsIn, sceneOf, remarksOn };
