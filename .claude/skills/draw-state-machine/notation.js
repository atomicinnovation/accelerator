const { markProblems } = require('../diagram-kit/lib/notations.js');

const KINDS = ['state', 'initial', 'final', 'choice'];
const DIRECTIONS = ['down', 'right'];
const ACTIVITIES = ['entry', 'do', 'exit'];
const SHAPES = { initial: 'initial', final: 'final', choice: 'choice' };
const PARTS = ['event', 'guard', 'action'];
const DESCRIBED = { initial: 'an initial state', final: 'a final state', choice: 'a choice' };

const kindOf = state => state.kind ?? 'state';
const labelOf = state => state.label ?? state.id;
const holdsOthers = state => Array.isArray(state.states) && state.states.length > 0;
const listed = names => (names.length < 3 ? names.join(' and ') : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`);
const nameOf = (transition, position) => (typeof transition.from === 'string' && typeof transition.to === 'string' ? `transition ${transition.from}->${transition.to}` : `transition ${position + 1}`);

function everyState(states, visit, parent = null) {
  for (const state of Array.isArray(states) ? states : []) {
    if (state === null || typeof state !== 'object') { visit(state, parent); continue; }
    visit(state, parent);
    everyState(state.states, visit, state);
  }
}

function indexed(spec) {
  const byId = new Map(), parentOf = new Map();
  everyState(spec.states, (state, parent) => {
    if (state && typeof state.id === 'string' && !byId.has(state.id)) { byId.set(state.id, state); parentOf.set(state.id, parent ? parent.id : null); }
  });
  const ancestorsOf = id => { const up = []; for (let at = parentOf.get(id); at; at = parentOf.get(at)) up.push(at); return up; };
  return { byId, parentOf, ancestorsOf };
}

function stateProblems(spec) {
  const problems = [], seen = new Set();
  let position = 0;
  everyState(spec.states, state => {
    position++;
    if (!state || typeof state.id !== 'string' || !state.id) { problems.push(`state ${position}: no id`); return; }
    const name = `state ${state.id}`, kind = kindOf(state);
    if (seen.has(state.id)) { problems.push(`${name}: id is used twice`); return; }
    seen.add(state.id);
    if (!KINDS.includes(kind)) { problems.push(`${name}: kind "${kind}" is not one of ${KINDS.join(', ')}`); return; }
    if (kind !== 'state' && holdsOthers(state)) problems.push(`${name}: ${DESCRIBED[kind]} cannot hold other states`);
    if (kind !== 'state' && ACTIVITIES.some(activity => state[activity] != null)) problems.push(`${name}: ${DESCRIBED[kind]} has no activities`);
    if (kind === 'state') problems.push(...ACTIVITIES.filter(activity => state[activity] != null && typeof state[activity] !== 'string').map(activity => `${name}: ${activity} must be text`));
    problems.push(...markProblems(state, name));
  });
  return problems;
}

function regionProblems(spec) {
  const starts = states => (Array.isArray(states) ? states : []).filter(state => state && kindOf(state) === 'initial').length;
  const problems = starts(spec.states) > 1 ? [`the machine has ${starts(spec.states)} initial states; a region starts in one place`] : [];
  everyState(spec.states, state => {
    if (state && starts(state.states) > 1) problems.push(`state ${state.id} has ${starts(state.states)} initial states; a region starts in one place`);
  });
  return problems;
}

function transitionProblems(transitions, byId) {
  return transitions.flatMap((transition, position) => {
    const name = nameOf(transition, position);
    const missing = ['from', 'to'].find(end => typeof transition[end] !== 'string' || !transition[end]);
    if (missing) return [`${name}: no ${missing}`];
    const unknown = [transition.from, transition.to].find(end => !byId.has(end));
    if (unknown) return [`${name}: no state "${unknown}"`];
    if (kindOf(byId.get(transition.from)) === 'final') return [`${name}: a final state has no way out`];
    if (kindOf(byId.get(transition.to)) === 'initial') return [`${name}: nothing arrives at an initial state`];
    const untriggered = kindOf(byId.get(transition.from)) === 'initial' && (transition.event != null || transition.guard != null);
    return [
      ...(untriggered ? [`${name}: the transition out of an initial state has no event or guard`] : []),
      ...markProblems(transition, name),
      ...PARTS.filter(part => transition[part] != null && typeof transition[part] !== 'string').map(part => `${name}: ${part} must be text`),
    ];
  });
}

function shapeProblems(transitions, byId) {
  const out = id => transitions.filter(transition => transition.from === id).length;
  const into = id => transitions.filter(transition => transition.to === id).length;
  return [...byId.values()].flatMap(state => {
    const name = `state ${state.id}`, kind = kindOf(state);
    if (kind === 'initial' && out(state.id) !== 1) return [`${name}: an initial state needs exactly one transition out, and has ${out(state.id)}`];
    if (kind === 'choice' && out(state.id) < 2) return [`${name}: a choice needs at least two transitions out, and has ${out(state.id)}`];
    if (kind === 'choice' && !into(state.id)) return [`${name}: a choice needs a transition in`];
    return [];
  });
}

function problemsIn(spec) {
  if (!Array.isArray(spec.states) || !spec.states.length) return ['no states'];
  const direction = spec.direction == null || DIRECTIONS.includes(spec.direction) ? [] : [`direction "${spec.direction}" is not one of ${DIRECTIONS.join(', ')}`];
  const states = stateProblems(spec);
  if (states.some(problem => / no id$| used twice$| is not one of state, /.test(problem))) return [...direction, ...states];
  const { byId } = indexed(spec), transitions = Array.isArray(spec.transitions) ? spec.transitions : [];
  const between = transitionProblems(transitions, byId);
  const sound = transitions.filter(transition => byId.has(transition.from) && byId.has(transition.to));
  return [...direction, ...states, ...regionProblems(spec), ...between, ...(between.some(problem => / no (from|to|state) /.test(`${problem} `)) ? [] : shapeProblems(sound, byId))];
}

function reachedFrom(spec, { byId, ancestorsOf }) {
  const transitions = spec.transitions || [];
  const start = spec.states.find(state => kindOf(state) === 'initial');
  if (!start) return null;
  const reached = new Set(), ahead = [start.id];
  const enter = id => {
    if (reached.has(id)) return;
    reached.add(id);
    ancestorsOf(id).forEach(ancestor => reached.add(ancestor));
    ahead.push(id);
    const within = (byId.get(id).states || []).find(state => kindOf(state) === 'initial');
    if (within) enter(within.id);
  };
  enter(start.id);
  while (ahead.length) {
    const at = ahead.pop(), here = new Set([at, ...ancestorsOf(at)]);
    transitions.filter(transition => here.has(transition.from)).forEach(transition => enter(transition.to));
  }
  return reached;
}

function remarksOn(spec) {
  const index = indexed(spec), { byId, ancestorsOf } = index, transitions = spec.transitions || [];
  const states = [...byId.values()], remarks = [];
  const reached = reachedFrom(spec, index);
  if (!reached) remarks.push('Nothing says where the machine starts: it has no initial state');
  const lost = reached ? states.filter(state => kindOf(state) !== 'initial' && !reached.has(state.id)).map(labelOf) : [];
  if (lost.length) remarks.push(`${listed(lost)} cannot be reached`);
  const leaves = id => transitions.some(transition => transition.from === id);
  for (const state of states) {
    if (kindOf(state) === 'state' && !holdsOthers(state) && ![state.id, ...ancestorsOf(state.id)].some(leaves)) remarks.push(`${labelOf(state)} has no way out and is not final`);
  }
  for (const state of states.filter(each => kindOf(each) === 'choice')) {
    const unguarded = transitions.filter(transition => transition.from === state.id && !transition.guard).length;
    const before = transitions.find(transition => transition.to === state.id);
    if (unguarded > 1) remarks.push(`The choice after ${labelOf(byId.get(before.from))} has ${unguarded} branches with no guard to choose between them`);
  }
  const onEvent = new Map();
  for (const transition of transitions.filter(each => each.event)) {
    const key = `${transition.from}\n${transition.event}`;
    onEvent.set(key, [...(onEvent.get(key) || []), transition]);
  }
  for (const [key, same] of onEvent) {
    const [from, event] = key.split('\n');
    if (same.length > 1 && same.some(transition => !transition.guard)) remarks.push(`${labelOf(byId.get(from))} has ${same.length} transitions on ${event} that no guard tells apart`);
  }
  return remarks;
}

function nodeOf(state) {
  const kind = kindOf(state), marks = { status: state.status, tone: state.tone, link: state.link };
  if (kind !== 'state') return { id: state.id, shape: SHAPES[kind], label: state.label, ...marks };
  if (holdsOthers(state)) return { id: state.id, shape: 'composite', label: labelOf(state), ...marks, children: state.states.map(nodeOf) };
  const activities = ACTIVITIES.filter(activity => state[activity]).map(activity => `${activity} / ${state[activity]}`);
  return { id: state.id, shape: 'state', label: labelOf(state), activities: activities.length ? activities : undefined, ...marks };
}

function labelFor(transition) {
  const parts = [transition.event, transition.guard ? `[${transition.guard}]` : null, transition.action ? `/ ${transition.action}` : null].filter(Boolean);
  return parts.length ? parts.join(' ') : undefined;
}

function sceneOf(spec) {
  const drawn = new Map();
  return {
    direction: spec.direction || 'down',
    nodes: spec.states.map(nodeOf),
    edges: (spec.transitions || []).map(transition => {
      const ends = `${transition.from}->${transition.to}`, nth = (drawn.get(ends) || 0) + 1;
      drawn.set(ends, nth);
      return { id: nth > 1 ? `${ends}:${nth}` : ends, from: transition.from, to: transition.to, label: labelFor(transition), line: 'solid', head: 'open',
        status: transition.status, tone: transition.tone, link: transition.link };
    }),
  };
}

module.exports = { kind: 'state-machine', problemsIn, sceneOf, remarksOn };
