const { markProblems } = require('../diagram-kit/lib/notations.js');
const { laidOutInLanes } = require('../diagram-kit/lib/lanes.js');

const DIRECTIONS = ['down', 'right'];
const KINDS = ['process', 'start', 'end', 'decision', 'input', 'output', 'subprocess'];
const SHAPES = { process: 'process', start: 'terminator', end: 'terminator', decision: 'decision', input: 'io', output: 'io', subprocess: 'subprocess' };

const kindOf = step => step.kind ?? 'process';
const listOf = value => (Array.isArray(value) ? value : []);
const listed = names => (names.length < 3 ? names.join(' and ') : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`);
const nameOf = (flow, position) => (typeof flow.from === 'string' && typeof flow.to === 'string' ? `flow ${flow.from}->${flow.to}` : `flow ${position + 1}`);

function laneProblems(lanes) {
  const problems = [], known = new Set();
  lanes.forEach((lane, position) => {
    if (!lane || typeof lane.id !== 'string' || !lane.id) { problems.push(`lane ${position + 1}: no id`); return; }
    if (known.has(lane.id)) problems.push(`lane ${lane.id}: id is used twice`);
    known.add(lane.id);
    problems.push(...markProblems(lane, `lane ${lane.id}`));
  });
  return { problems, known };
}

function stepProblems(steps, lanes, hasLanes) {
  const problems = [], byId = new Map();
  steps.forEach((step, position) => {
    if (!step || typeof step.id !== 'string' || !step.id) { problems.push(`step ${position + 1}: no id`); return; }
    const name = `step ${step.id}`;
    if (byId.has(step.id)) { problems.push(`${name}: id is used twice`); return; }
    byId.set(step.id, step);
    if (!KINDS.includes(kindOf(step))) { problems.push(`${name}: kind "${step.kind}" is not one of ${KINDS.join(', ')}`); return; }
    if (typeof step.label !== 'string' || !step.label.trim()) problems.push(`${name}: no label`);
    if (step.lane != null && !lanes.has(step.lane)) problems.push(`${name}: no lane "${step.lane}"`);
    else if (hasLanes && step.lane == null) problems.push(`${name}: no lane, and the flowchart has lanes`);
    problems.push(...markProblems(step, name));
  });
  return { problems, byId };
}

function flowProblems(flows, byId) {
  return flows.flatMap((flow, position) => {
    const name = nameOf(flow, position);
    const missing = ['from', 'to'].find(end => typeof flow[end] !== 'string' || !flow[end]);
    if (missing) return [`${name}: no ${missing}`];
    const unknown = [flow.from, flow.to].find(end => !byId.has(end));
    if (unknown) return [`${name}: no step "${unknown}"`];
    if (kindOf(byId.get(flow.from)) === 'end') return [`${name}: nothing flows out of an end`];
    if (kindOf(byId.get(flow.to)) === 'start') return [`${name}: nothing flows into a start`];
    return [...markProblems(flow, name), ...(flow.label == null || typeof flow.label === 'string' ? [] : [`${name}: label must be text`])];
  });
}

function problemsIn(spec) {
  if (!Array.isArray(spec.steps) || !spec.steps.length) return ['no steps'];
  const hasLanes = listOf(spec.lanes).length > 0;
  const direction = spec.direction == null || DIRECTIONS.includes(spec.direction) ? [] : [`direction "${spec.direction}" is not one of ${DIRECTIONS.join(', ')}`];
  const sideways = hasLanes && spec.direction != null && spec.direction !== 'down' && !direction.length ? ['lanes are drawn down the page; leave direction out'] : [];
  const { problems: lanes, known } = laneProblems(listOf(spec.lanes));
  const { problems: steps, byId } = stepProblems(spec.steps, known, hasLanes);
  const flows = listOf(spec.flows), between = flowProblems(flows, byId);
  const structural = [...direction, ...sideways, ...lanes, ...steps, ...between];
  if (structural.some(problem => / no id$| used twice$| is not one of process| no step "| no (from|to)$/.test(problem))) return structural;
  const out = id => flows.filter(flow => flow.from === id).length;
  const shaped = [...byId.values()].flatMap(step => {
    if (kindOf(step) === 'start' && !out(step.id)) return [`step ${step.id}: a start needs a flow out`];
    if (kindOf(step) === 'decision' && out(step.id) < 2) return [`step ${step.id}: a decision needs at least two flows out, and has ${out(step.id)}`];
    return [];
  });
  return [...structural, ...shaped];
}

function remarksOn(spec) {
  const flows = listOf(spec.flows), remarks = [];
  const starts = spec.steps.filter(step => kindOf(step) === 'start');
  if (!starts.length) remarks.push('Nothing says where the flowchart starts: it has no start');
  for (const step of spec.steps) {
    if (kindOf(step) !== 'end' && !flows.some(flow => flow.from === step.id)) remarks.push(`${step.label} leads nowhere, and is not an end`);
  }
  if (starts.length) {
    const reached = new Set(), ahead = starts.map(step => step.id);
    while (ahead.length) {
      const at = ahead.pop();
      if (reached.has(at)) continue;
      reached.add(at);
      flows.filter(flow => flow.from === at).forEach(flow => ahead.push(flow.to));
    }
    const lost = spec.steps.filter(step => !reached.has(step.id)).map(step => step.label);
    if (lost.length) remarks.push(`${listed(lost)} cannot be reached`);
  }
  for (const step of spec.steps.filter(each => kindOf(each) === 'decision')) {
    const answers = flows.filter(flow => flow.from === step.id).map(flow => flow.label);
    if (answers.some(answer => !answer)) remarks.push(`"${step.label}" has a flow out with no answer on it`);
    const repeated = answers.find((answer, i) => answer && answers.indexOf(answer) !== i);
    if (repeated) remarks.push(`"${step.label}" has two flows out both labelled ${repeated}`);
  }
  return remarks;
}

function sceneOf(spec) {
  const drawn = new Map();
  return {
    direction: spec.direction || 'down',
    nodes: spec.steps.map(step => ({ id: step.id, shape: SHAPES[kindOf(step)], label: step.label, lane: step.lane, status: step.status, tone: step.tone, link: step.link })),
    edges: listOf(spec.flows).map(flow => {
      const ends = `${flow.from}->${flow.to}`, nth = (drawn.get(ends) || 0) + 1;
      drawn.set(ends, nth);
      return { id: nth > 1 ? `${ends}:${nth}` : ends, from: flow.from, to: flow.to, label: flow.label, line: 'solid', head: 'filled', status: flow.status, tone: flow.tone, link: flow.link };
    }),
  };
}

function placedSceneOf(spec) {
  const lanes = listOf(spec.lanes);
  return lanes.length ? laidOutInLanes({ lanes, ...sceneOf(spec) }) : null;
}

module.exports = { kind: 'flowchart', problemsIn, sceneOf, placedSceneOf, remarksOn };
