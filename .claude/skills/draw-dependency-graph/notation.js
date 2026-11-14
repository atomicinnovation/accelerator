const { markProblems } = require('../diagram-kit/lib/notations.js');
const { cyclesIn, loopThrough } = require('../diagram-kit/lib/graphs.js');

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
  const reduce = spec.reduce == null || typeof spec.reduce === 'boolean' ? [] : ['reduce must be true or false'];
  const { problems, known } = moduleProblems(spec.modules);
  return [...direction, ...reduce, ...problems, ...dependencyProblems(spec.dependencies || [], known)];
}

const isMarked = dependency => ['status', 'tone', 'label', 'kind'].some(mark => dependency[mark] != null);

function reachedAnotherWay(dependency, dependencies) {
  const seen = new Set(), ahead = [dependency.from];
  while (ahead.length) {
    const module = ahead.pop();
    if (seen.has(module)) continue;
    seen.add(module);
    for (const other of dependencies) {
      if (other === dependency || other.from !== module) continue;
      if (other.to === dependency.to) return true;
      ahead.push(other.to);
    }
  }
  return false;
}

function impliedIn(spec) {
  const dependencies = spec.dependencies || [];
  if (!spec.reduce) return new Set();
  const knots = cyclesIn(dependencies);
  const inCycle = ({ from, to }) => knots.some(knot => knot.has(from) && knot.has(to));
  const implied = new Set();
  let drawn = dependencies;
  for (const each of dependencies) {
    if (isMarked(each) || inCycle(each) || !reachedAnotherWay(each, drawn)) continue;
    implied.add(each);
    drawn = drawn.filter(other => other !== each);
  }
  return implied;
}

function remarksOn(spec) {
  const dependencies = spec.dependencies || [], labels = new Map();
  everyModule(spec.modules, module => labels.set(module.id, labelOf(module)));
  const cycles = cyclesIn(dependencies).map(knot => `Cycle: ${loopThrough(knot, dependencies).map(id => labels.get(id)).join(' → ')}`);
  const implied = impliedIn(spec).size;
  const reduced = implied === 1 ? ['1 dependency that a longer path implies is not drawn'] : [`${implied} dependencies that longer paths imply are not drawn`];
  return implied ? [...cycles, ...reduced] : cycles;
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
  const implied = impliedIn(spec);
  return {
    direction: spec.direction || 'down',
    nodes: spec.modules.map(nodeOf),
    edges: dependencies.filter(dependency => !implied.has(dependency)).map(dependency => ({
      id: idOf(dependency, dependencies), from: dependency.from, to: dependency.to,
      label: dependency.label ?? dependency.kind, line: 'dashed', head: 'open',
      status: dependency.status, tone: dependency.tone ?? (inCycle(dependency) ? 'risk' : undefined), link: dependency.link,
    })),
  };
}

module.exports = { kind: 'dependency-graph', problemsIn, sceneOf, remarksOn };
