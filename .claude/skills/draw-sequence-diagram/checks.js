const { markProblems } = require('../diagram-kit/lib/notations.js');

const PARTICIPANT_KINDS = ['participant', 'actor'];
const MESSAGE_KINDS = ['sync', 'async', 'create', 'destroy'];
const OPERATORS = ['alt', 'opt', 'loop', 'par', 'break'];
const SEVERAL_OPERANDS = ['alt', 'par'];

const isObject = value => value !== null && typeof value === 'object' && !Array.isArray(value);
const isMessage = step => isObject(step) && 'from' in step && 'to' in step;
const isFragment = step => isObject(step) && 'fragment' in step;
const isNote = step => isObject(step) && 'note' in step;
const labelOf = participant => participant.label ?? participant.id;
const withArticle = word => `${/^[aeiou]/.test(word) ? 'an' : 'a'} ${word}`;
const outside = (field, value, allowed) => (value == null || allowed.includes(value) ? [] : [`${field} "${value}" is not one of ${allowed.join(', ')}`]);

const within = (name, position) => (name === '' ? `step ${position}` : / operand \d+$/.test(name) ? `${name}, step ${position}` : `${name}.${position}`);

function participantProblems(participants) {
  const problems = [], known = new Set();
  participants.forEach((participant, position) => {
    if (!isObject(participant) || typeof participant.id !== 'string' || !participant.id) { problems.push(`participant ${position + 1}: no id`); return; }
    const name = `participant ${participant.id}`;
    if (known.has(participant.id)) problems.push(`${name}: id is used twice`);
    known.add(participant.id);
    problems.push(...outside('kind', participant.kind, PARTICIPANT_KINDS).map(problem => `${name}: ${problem}`), ...markProblems(participant, name));
  });
  return { problems, known };
}

function messageProblems(step, name, taking) {
  const kind = step.kind ?? 'sync';
  const unknown = [step.from, step.to].find(end => !taking.known.has(end));
  if (unknown !== undefined) return [`${name}: no participant "${unknown}"`];
  if (!MESSAGE_KINDS.includes(kind)) return [`${name}: kind "${kind}" is not one of ${MESSAGE_KINDS.join(', ')}; a reply belongs to its call, as "reply"`];
  const gone = [step.from, step.to].find(end => taking.destroyed.has(end));
  if (gone) return [`${name}: ${gone} has been destroyed`];
  const problems = [];
  if (kind === 'create') {
    if (step.from === step.to) return [`${name}: a participant cannot create itself`];
    if (taking.part.has(step.to)) return [`${name}: ${step.to} cannot be created after it has taken part`];
  } else if (kind !== 'destroy' && (typeof step.label !== 'string' || !step.label.trim())) problems.push(`${name}: no label`);
  if (step.reply != null && kind !== 'sync') problems.push(`${name}: only a synchronous call has a reply`);
  else if (step.reply != null && typeof step.reply !== 'string') problems.push(`${name}: reply must be text`);
  taking.part.add(step.from).add(step.to);
  if (kind === 'destroy') taking.destroyed.add(step.to);
  problems.push(...markProblems(step, name));
  if (step.steps != null && !Array.isArray(step.steps)) return [...problems, `${name}: steps must be a list`];
  return [...problems, ...stepProblems(step.steps || [], name, taking)];
}

function fragmentProblems(step, name, taking) {
  const operator = step.fragment;
  if (!OPERATORS.includes(operator)) return [`${name}: fragment "${operator}" is not one of ${OPERATORS.join(', ')}`];
  if (!SEVERAL_OPERANDS.includes(operator)) {
    if (!Array.isArray(step.steps) || !step.steps.length) return [`${name}: ${withArticle(operator)} fragment needs steps`];
    return stepProblems(step.steps, name, taking);
  }
  if (!Array.isArray(step.operands) || !step.operands.length) return [`${name}: ${withArticle(operator)} fragment needs operands, each with its steps`];
  if (operator === 'par' && step.operands.length < 2) return [`${name}: a par fragment needs at least two operands`];
  return step.operands.flatMap((operand, position) => {
    const operandName = `${name}, operand ${position + 1}`;
    if (!isObject(operand) || !Array.isArray(operand.steps) || !operand.steps.length) return [`${operandName}: no steps`];
    return stepProblems(operand.steps, operandName, taking);
  });
}

function noteProblems(step, name, taking) {
  if (typeof step.note !== 'string' || !step.note.trim()) return [`${name}: a note needs text`];
  if (!Array.isArray(step.over) || !step.over.length) return [`${name}: a note needs the participants it is over`];
  const unknown = step.over.find(over => !taking.known.has(over));
  if (unknown !== undefined) return [`${name}: no participant "${unknown}"`];
  return markProblems(step, name);
}

function stepProblems(steps, parent, taking) {
  return steps.flatMap((step, position) => {
    const name = within(parent, position + 1);
    if (isMessage(step)) return messageProblems(step, name, taking);
    if (isFragment(step)) return fragmentProblems(step, name, taking);
    if (isNote(step)) return noteProblems(step, name, taking);
    return [`${name}: is not a message (from, to), a fragment or a note`];
  });
}

function problemsIn(spec) {
  if (!Array.isArray(spec.participants) || !spec.participants.length) return ['no participants'];
  if (!Array.isArray(spec.steps) || !spec.steps.length) return ['no steps'];
  const numbered = spec.numbered == null || typeof spec.numbered === 'boolean' ? [] : ['numbered must be true or false'];
  const { problems, known } = participantProblems(spec.participants);
  return [...numbered, ...problems, ...stepProblems(spec.steps, '', { known, part: new Set(), destroyed: new Set() })];
}

function everyMessage(steps, visit) {
  for (const step of steps || []) {
    if (isMessage(step)) { visit(step); everyMessage(step.steps, visit); }
    else if (isFragment(step)) { everyMessage(step.steps, visit); (step.operands || []).forEach(operand => everyMessage(operand.steps, visit)); }
  }
}

const listed = names => (names.length < 3 ? names.join(' and ') : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`);

function remarksOn(spec) {
  const part = new Set();
  everyMessage(spec.steps, step => part.add(step.from).add(step.to));
  const idle = spec.participants.filter(participant => !part.has(participant.id)).map(labelOf);
  return idle.length ? [`${listed(idle)} ${idle.length === 1 ? 'takes' : 'take'} no part in the sequence`] : [];
}

module.exports = { problemsIn, remarksOn, isMessage, isFragment, isNote, labelOf, SEVERAL_OPERANDS };
