const { markProblems } = require('../diagram-kit/lib/notations.js');
const { laidOutInLanes } = require('../diagram-kit/lib/lanes.js');

const DIRECTIONS = ['down', 'right'];
const KINDS = ['action', 'initial', 'final', 'flow final', 'decision', 'merge', 'fork', 'join'];
const ENDS = ['final', 'flow final'];
const SHAPES = { action: 'state', initial: 'initial', final: 'final', 'flow final': 'flowfinal', decision: 'choice', merge: 'choice', fork: 'bar', join: 'bar' };

const kindOf = node => node.kind ?? 'action';
const nameOfNode = node => node.label ?? `the ${kindOf(node)} node`;
const listOf = value => (Array.isArray(value) ? value : []);
const listed = names => (names.length < 3 ? names.join(' and ') : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`);
const nameOf = (flow, position) => (typeof flow.from === 'string' && typeof flow.to === 'string' ? `flow ${flow.from}->${flow.to}` : `flow ${position + 1}`);

function partitionProblems(partitions) {
  const problems = [], known = new Set();
  partitions.forEach((partition, position) => {
    if (!partition || typeof partition.id !== 'string' || !partition.id) { problems.push(`partition ${position + 1}: no id`); return; }
    if (known.has(partition.id)) problems.push(`partition ${partition.id}: id is used twice`);
    known.add(partition.id);
    problems.push(...markProblems(partition, `partition ${partition.id}`));
  });
  return { problems, known };
}

function nodeProblems(nodes, partitions, hasPartitions) {
  const problems = [], byId = new Map();
  nodes.forEach((node, position) => {
    if (!node || typeof node.id !== 'string' || !node.id) { problems.push(`node ${position + 1}: no id`); return; }
    const name = `node ${node.id}`;
    if (byId.has(node.id)) { problems.push(`${name}: id is used twice`); return; }
    byId.set(node.id, node);
    if (!KINDS.includes(kindOf(node))) { problems.push(`${name}: kind "${node.kind}" is not one of ${KINDS.join(', ')}`); return; }
    if (kindOf(node) === 'action' && (typeof node.label !== 'string' || !node.label.trim())) problems.push(`${name}: an action needs a label`);
    if (node.partition != null && !partitions.has(node.partition)) problems.push(`${name}: no partition "${node.partition}"`);
    else if (hasPartitions && node.partition == null) problems.push(`${name}: no partition, and the activity has partitions`);
    problems.push(...markProblems(node, name));
  });
  return { problems, byId };
}

function flowProblems(flows, byId) {
  return flows.flatMap((flow, position) => {
    const name = nameOf(flow, position);
    const missing = ['from', 'to'].find(end => typeof flow[end] !== 'string' || !flow[end]);
    if (missing) return [`${name}: no ${missing}`];
    const unknown = [flow.from, flow.to].find(end => !byId.has(end));
    if (unknown) return [`${name}: no node "${unknown}"`];
    if (ENDS.includes(kindOf(byId.get(flow.from)))) return [`${name}: nothing flows out of a final node`];
    if (kindOf(byId.get(flow.to)) === 'initial') return [`${name}: nothing flows into the initial node`];
    return [...markProblems(flow, name), ...['guard', 'label'].filter(part => flow[part] != null && typeof flow[part] !== 'string').map(part => `${name}: ${part} must be text`)];
  });
}

function arityProblems(flows, byId) {
  const out = id => flows.filter(flow => flow.from === id).length, into = id => flows.filter(flow => flow.to === id).length;
  const few = (kind, way, count) => `a ${kind} needs at least two flows ${way}, and has ${count}`;
  const one = (kind, way) => `a ${kind} needs exactly one flow ${way}`;
  const RULES = {
    initial: id => (out(id) ? [] : ['the initial node needs a flow out']),
    decision: id => (out(id) < 2 ? [few('decision', 'out', out(id))] : []),
    merge: id => (into(id) < 2 ? [few('merge', 'in', into(id))] : out(id) !== 1 ? [one('merge', 'out')] : []),
    fork: id => (out(id) < 2 ? [few('fork', 'out', out(id))] : into(id) !== 1 ? [one('fork', 'in')] : []),
    join: id => (into(id) < 2 ? [few('join', 'in', into(id))] : out(id) !== 1 ? [one('join', 'out')] : []),
  };
  return [...byId.values()].flatMap(node => (RULES[kindOf(node)] || (() => []))(node.id).map(problem => `node ${node.id}: ${problem}`));
}

function problemsIn(spec) {
  if (!Array.isArray(spec.nodes) || !spec.nodes.length) return ['no nodes'];
  const hasPartitions = listOf(spec.partitions).length > 0;
  const direction = spec.direction == null || DIRECTIONS.includes(spec.direction) ? [] : [`direction "${spec.direction}" is not one of ${DIRECTIONS.join(', ')}`];
  const sideways = hasPartitions && spec.direction != null && spec.direction !== 'down' && !direction.length ? ['partitions are drawn as lanes down the page; leave direction out'] : [];
  const { problems: partitions, known } = partitionProblems(listOf(spec.partitions));
  const { problems: nodes, byId } = nodeProblems(spec.nodes, known, hasPartitions);
  const flows = listOf(spec.flows), between = flowProblems(flows, byId);
  const structural = [...direction, ...sideways, ...partitions, ...nodes, ...between];
  const sound = flows.filter(flow => byId.has(flow.from) && byId.has(flow.to));
  return structural.some(problem => / no id$| used twice$| is not one of action| no node "| no (from|to)$/.test(problem)) ? structural : [...structural, ...arityProblems(sound, byId)];
}

function remarksOn(spec) {
  const flows = listOf(spec.flows), byId = new Map(spec.nodes.map(node => [node.id, node])), remarks = [];
  const starts = spec.nodes.filter(node => kindOf(node) === 'initial');
  if (!starts.length) remarks.push('Nothing says where the activity starts: it has no initial node');
  for (const node of spec.nodes) {
    if (kindOf(node) === 'action' && !flows.some(flow => flow.from === node.id)) remarks.push(`${nameOfNode(node)} has no flow out, and the activity does not end there`);
  }
  if (starts.length) {
    const reached = new Set(), ahead = starts.map(node => node.id);
    while (ahead.length) {
      const at = ahead.pop();
      if (reached.has(at)) continue;
      reached.add(at);
      flows.filter(flow => flow.from === at).forEach(flow => ahead.push(flow.to));
    }
    const lost = spec.nodes.filter(node => !reached.has(node.id)).map(nameOfNode);
    if (lost.length) remarks.push(`${listed(lost)} cannot be reached`);
  }
  for (const node of spec.nodes.filter(each => kindOf(each) === 'decision')) {
    const unguarded = flows.filter(flow => flow.from === node.id && !flow.guard).length;
    const before = flows.find(flow => flow.to === node.id);
    if (unguarded > 1) remarks.push(`The decision${before ? ` after ${nameOfNode(byId.get(before.from))}` : ''} has ${unguarded} flows out with no guard to choose between them`);
  }
  return remarks;
}

function sceneOf(spec) {
  const drawn = new Map(), sideways = spec.direction === 'right';
  return {
    direction: spec.direction || 'down',
    nodes: spec.nodes.map(node => ({
      id: node.id, shape: SHAPES[kindOf(node)], label: node.label, lane: node.partition,
      upright: SHAPES[kindOf(node)] === 'bar' && sideways ? true : undefined,
      status: node.status, tone: node.tone, link: node.link,
    })),
    edges: listOf(spec.flows).map(flow => {
      const ends = `${flow.from}->${flow.to}`, nth = (drawn.get(ends) || 0) + 1;
      drawn.set(ends, nth);
      const label = [flow.guard ? `[${flow.guard}]` : null, flow.label].filter(Boolean).join(' ') || undefined;
      return { id: nth > 1 ? `${ends}:${nth}` : ends, from: flow.from, to: flow.to, label, line: 'solid', head: 'open', status: flow.status, tone: flow.tone, link: flow.link };
    }),
  };
}

function placedSceneOf(spec) {
  const partitions = listOf(spec.partitions);
  return partitions.length ? laidOutInLanes({ lanes: partitions, ...sceneOf(spec) }) : null;
}

module.exports = { kind: 'activity-diagram', name: 'UML activity diagram', symbols: require('./symbols.js'), problemsIn, sceneOf, placedSceneOf, remarksOn };
