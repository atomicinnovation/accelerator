const { markProblems } = require('../diagram-kit/lib/notations.js');

const DIRECTIONS = ['down', 'right'];

function everyModule(modules, visit) {
  for (const module of Array.isArray(modules) ? modules : []) {
    visit(module);
    everyModule(module.modules, visit);
  }
}

const labelOf = module => module.label ?? module.id;
const holdsOthers = module => Array.isArray(module.modules) && module.modules.length > 0;
const sameEnds = (a, b) => a.from === b.from && a.to === b.to;
const nameOf = (dependency, position) => (typeof dependency.from === 'string' && typeof dependency.to === 'string' ? `dependency ${dependency.from}->${dependency.to}` : `dependency ${position + 1}`);

function idOf(dependency, dependencies) {
  const shared = dependencies.filter(other => sameEnds(other, dependency)).length > 1;
  return `${dependency.from}->${dependency.to}${shared && dependency.kind ? `:${dependency.kind}` : ''}`;
}

function moduleProblems(modules) {
  const problems = [], seen = new Set();
  let position = 0;
  everyModule(modules, module => {
    position++;
    if (typeof module.id !== 'string' || !module.id) { problems.push(`module ${position}: no id`); return; }
    if (seen.has(module.id)) problems.push(`module ${module.id}: id is used twice`);
    seen.add(module.id);
    problems.push(...markProblems(module, `module ${module.id}`));
  });
  return { problems, known: seen };
}

function dependencyProblems(dependencies, known) {
  const problems = [], written = new Set();
  dependencies.forEach((dependency, position) => {
    const name = nameOf(dependency, position);
    const missing = ['from', 'to'].find(end => typeof dependency[end] !== 'string' || !dependency[end]);
    if (missing) { problems.push(`${name}: no ${missing}`); return; }
    const unknown = [dependency.from, dependency.to].find(end => !known.has(end));
    if (unknown) { problems.push(`${name}: no module "${unknown}"`); return; }
    const key = `${dependency.from}\n${dependency.to}\n${dependency.kind ?? ''}`;
    if (written.has(key)) problems.push(`${name}: written twice`);
    written.add(key);
    problems.push(...markProblems(dependency, name));
  });
  return problems;
}

function problemsIn(spec) {
  if (!Array.isArray(spec.modules) || !spec.modules.length) return ['no modules'];
  const direction = spec.direction == null || DIRECTIONS.includes(spec.direction) ? [] : [`direction "${spec.direction}" is not one of ${DIRECTIONS.join(', ')}`];
  const { problems, known } = moduleProblems(spec.modules);
  return [...direction, ...problems, ...dependencyProblems(spec.dependencies || [], known)];
}

function cyclesIn(dependencies) {
  const needs = new Map();
  for (const { from, to } of dependencies) {
    for (const end of [from, to]) if (!needs.has(end)) needs.set(end, []);
    needs.get(from).push(to);
  }
  const order = new Map(), lowest = new Map(), stack = [], onStack = new Set(), knots = [];
  const visit = module => {
    order.set(module, order.size);
    lowest.set(module, order.get(module));
    stack.push(module);
    onStack.add(module);
    for (const needed of needs.get(module)) {
      if (!order.has(needed)) { visit(needed); lowest.set(module, Math.min(lowest.get(module), lowest.get(needed))); }
      else if (onStack.has(needed)) lowest.set(module, Math.min(lowest.get(module), order.get(needed)));
    }
    if (lowest.get(module) !== order.get(module)) return;
    const knot = [];
    let member;
    do { member = stack.pop(); onStack.delete(member); knot.push(member); } while (member !== module);
    if (knot.length > 1 || needs.get(module).includes(module)) knots.push(new Set(knot));
  };
  for (const module of needs.keys()) if (!order.has(module)) visit(module);
  return knots;
}

function loopThrough(knot, dependencies) {
  const start = dependencies.find(({ from }) => knot.has(from)).from;
  const walk = (module, path) => {
    for (const { from, to } of dependencies) {
      if (from !== module || !knot.has(to)) continue;
      if (to === start) return [...path, to];
      if (path.includes(to)) continue;
      const found = walk(to, [...path, to]);
      if (found) return found;
    }
    return null;
  };
  return walk(start, [start]);
}

function remarksOn(spec) {
  const dependencies = spec.dependencies || [], labels = new Map();
  everyModule(spec.modules, module => labels.set(module.id, labelOf(module)));
  return cyclesIn(dependencies).map(knot => `Cycle: ${loopThrough(knot, dependencies).map(id => labels.get(id)).join(' → ')}`);
}

function nodeOf(module) {
  const { modules, id, status, tone, link, detail } = module;
  const node = { id, shape: holdsOthers(module) ? 'group' : 'box', label: labelOf(module), status, tone, link, detail };
  return holdsOthers(module) ? { ...node, children: modules.map(nodeOf) } : node;
}

function sceneOf(spec) {
  const dependencies = spec.dependencies || [];
  const knots = cyclesIn(dependencies);
  const inCycle = ({ from, to }) => knots.some(knot => knot.has(from) && knot.has(to));
  return {
    direction: spec.direction || 'down',
    nodes: spec.modules.map(nodeOf),
    edges: dependencies.map(dependency => ({
      id: idOf(dependency, dependencies), from: dependency.from, to: dependency.to,
      label: dependency.label ?? dependency.kind, line: 'dashed', head: 'open',
      status: dependency.status, tone: dependency.tone ?? (inCycle(dependency) ? 'risk' : undefined), link: dependency.link,
    })),
  };
}

module.exports = { kind: 'dependency-graph', problemsIn, sceneOf, remarksOn };
