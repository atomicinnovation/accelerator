const { markProblems } = require('../diagram-kit/lib/notations.js');

const DIRECTIONS = ['down', 'right'];
const listOf = value => (Array.isArray(value) ? value : []);
const nameOfPart = part => part.name ?? part.id;
const holdsAnything = node => listOf(node.nodes).length > 0 || listOf(node.artifacts).length > 0;

function everyNode(nodes, visit, holder = null) {
  for (const node of listOf(nodes)) {
    visit(node, holder);
    if (node) everyNode(node.nodes, visit, node);
  }
}

function partProblems(spec) {
  const problems = [], nodes = new Map(), artifacts = new Map();
  const seen = id => nodes.has(id) || artifacts.has(id);
  let position = 0;
  everyNode(spec.nodes, node => {
    position++;
    if (!node || typeof node.id !== 'string' || !node.id) { problems.push(`node ${position}: no id`); return; }
    const name = `node ${node.id}`;
    if (seen(node.id)) { problems.push(`${name}: id is used twice`); return; }
    nodes.set(node.id, node);
    listOf(node.artifacts).forEach((artifact, i) => {
      if (!artifact || typeof artifact.id !== 'string' || !artifact.id) { problems.push(`${name}: artefact ${i + 1}: no id`); return; }
      const artefact = `${name}: artefact ${artifact.id}`;
      if (seen(artifact.id)) { problems.push(`${artefact}: id is used twice`); return; }
      artifacts.set(artifact.id, artifact);
      if (artifact.stereotype != null && typeof artifact.stereotype !== 'string') problems.push(`${artefact}: stereotype must be text`);
      problems.push(...markProblems(artifact, artefact));
    });
    if (node.stereotype != null && typeof node.stereotype !== 'string') problems.push(`${name}: stereotype must be text`);
    problems.push(...markProblems(node, name));
  });
  return { problems, nodes, artifacts };
}

function joinProblems(joins, word, arrow, own, other, ownWord, otherWord, undirected) {
  const written = new Set();
  return joins.flatMap((join, position) => {
    const named = typeof join.from === 'string' && typeof join.to === 'string';
    const name = named ? `${word} ${join.from}${arrow}${join.to}` : `${word} ${position + 1}`;
    const missing = ['from', 'to'].find(end => typeof join[end] !== 'string' || !join[end]);
    if (missing) return [`${name}: no ${missing}`];
    const misplaced = [join.from, join.to].find(end => other.has(end));
    if (misplaced) return [`${name}: a ${word === 'path' ? 'communication path' : 'dependency'} joins ${ownWord}s, and ${misplaced} is ${otherWord}`];
    const unknown = [join.from, join.to].find(end => !own.has(end));
    if (unknown) return [`${name}: no ${ownWord} "${unknown}"`];
    const key = (undirected ? [join.from, join.to].sort() : [join.from, join.to]).join('\n');
    if (written.has(key)) return [`${name}: written twice`];
    written.add(key);
    return markProblems(join, name);
  });
}

function problemsIn(spec) {
  if (!listOf(spec.nodes).length) return ['no nodes'];
  const direction = spec.direction == null || DIRECTIONS.includes(spec.direction) ? [] : [`direction "${spec.direction}" is not one of ${DIRECTIONS.join(', ')}`];
  const { problems, nodes, artifacts } = partProblems(spec);
  return [
    ...direction, ...problems,
    ...joinProblems(listOf(spec.paths), 'path', '--', nodes, artifacts, 'node', 'an artefact', true),
    ...joinProblems(listOf(spec.dependencies), 'dependency', '->', artifacts, nodes, 'artefact', 'a node', false),
  ];
}

function remarksOn(spec) {
  const paths = listOf(spec.paths), holderOf = new Map(), on = new Map(), names = new Map(), remarks = [];
  everyNode(spec.nodes, (node, holder) => {
    names.set(node.id, nameOfPart(node));
    holderOf.set(node.id, holder ? holder.id : null);
    listOf(node.artifacts).forEach(artifact => { on.set(artifact.id, node.id); names.set(artifact.id, nameOfPart(artifact)); });
  });
  const withHolders = id => { const up = [id]; for (let at = holderOf.get(id); at; at = holderOf.get(at)) up.push(at); return up; };
  const joined = (a, b) => {
    const here = withHolders(a), there = withHolders(b);
    return here.some(id => there.includes(id)) || paths.some(path => (here.includes(path.from) && there.includes(path.to)) || (here.includes(path.to) && there.includes(path.from)));
  };
  for (const { from, to } of listOf(spec.dependencies)) {
    const [here, there] = [on.get(from), on.get(to)];
    if (here !== there && !joined(here, there)) remarks.push(`${names.get(from)} on ${names.get(here)} depends on ${names.get(to)} on ${names.get(there)}, but no communication path joins them`);
  }
  everyNode(spec.nodes, node => {
    const touched = paths.some(path => path.from === node.id || path.to === node.id);
    if (!holdsAnything(node) && !touched) remarks.push(`${nameOfPart(node)} has nothing deployed on it and is joined to nothing`);
  });
  return remarks;
}

function nodeOf(node) {
  const marks = { status: node.status, tone: node.tone, link: node.link }, label = nameOfPart(node);
  if (!holdsAnything(node)) return { id: node.id, shape: 'cuboidbox', stereotype: node.stereotype, label, ...marks };
  const artifacts = listOf(node.artifacts).map(artifact => ({
    id: artifact.id, shape: 'classifier', stereotype: artifact.stereotype ?? 'artifact', label: nameOfPart(artifact), compartments: [],
    status: artifact.status, tone: artifact.tone, link: artifact.link,
  }));
  return { id: node.id, shape: 'cuboid', stereotype: node.stereotype, label, ...marks, children: [...listOf(node.nodes).map(nodeOf), ...artifacts] };
}

function sceneOf(spec) {
  const marksOf = join => ({ label: join.label, status: join.status, tone: join.tone, link: join.link });
  return {
    direction: spec.direction || 'down',
    nodes: spec.nodes.map(nodeOf),
    edges: [
      ...listOf(spec.paths).map(path => ({ id: `${path.from}--${path.to}`, from: path.from, to: path.to, line: 'solid', head: 'none', ...marksOf(path) })),
      ...listOf(spec.dependencies).map(each => ({ id: `${each.from}->${each.to}`, from: each.from, to: each.to, line: 'dashed', head: 'open', ...marksOf(each) })),
    ],
  };
}

module.exports = { kind: 'deployment-diagram', symbols: require('./symbols.js'), problemsIn, sceneOf, remarksOn };
