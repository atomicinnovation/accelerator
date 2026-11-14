const { markProblems } = require('../diagram-kit/lib/notations.js');

const DIRECTIONS = ['down', 'right'];
const isObject = value => value !== null && typeof value === 'object' && !Array.isArray(value);
const isQuestion = node => isObject(node) && 'question' in node;
const isOutcome = node => isObject(node) && 'outcome' in node;
const whereOf = answers => (answers.length ? `after ${answers.map(answer => `"${answer}"`).join(', ')}` : 'the root');

function nodeProblems(node, answers, seen) {
  const where = whereOf(answers);
  if (isQuestion(node) && isOutcome(node)) return [`${where}: is both a question and an outcome`];
  if (!isQuestion(node) && !isOutcome(node)) return [`${where}: is neither a question nor an outcome`];
  const problems = [];
  if (node.id != null) {
    if (typeof node.id !== 'string' || !node.id) problems.push(`${where}: id must be text`);
    else if (seen.has(node.id)) problems.push(`${where}: id "${node.id}" is used twice`);
    seen.add(node.id);
  }
  problems.push(...markProblems(node, where));
  if (isOutcome(node)) return [...problems, ...(typeof node.outcome === 'string' && node.outcome.trim() ? [] : [`${where}: outcome must be text`])];
  if (typeof node.question !== 'string' || !node.question.trim()) problems.push(`${where}: question must be text`);
  if (!Array.isArray(node.branches) || !node.branches.length) return [...problems, `${where}: a question needs branches`];
  return [...problems, ...node.branches.flatMap((branch, position) => {
    const at = `${where}, branch ${position + 1}`;
    if (!isObject(branch)) return [`${at}: is not an answer with what follows`];
    if (typeof branch.answer !== 'string' || !branch.answer.trim()) return [`${at}: no answer`];
    if (branch.then == null) return [`${at}: nothing follows it`];
    return [...markProblems(branch, at), ...nodeProblems(branch.then, [...answers, branch.answer], seen)];
  })];
}

function problemsIn(spec) {
  if (spec.root == null) return ['no root'];
  const direction = spec.direction == null || DIRECTIONS.includes(spec.direction) ? [] : [`direction "${spec.direction}" is not one of ${DIRECTIONS.join(', ')}`];
  return [...direction, ...nodeProblems(spec.root, [], new Set())];
}

function everyQuestion(node, visit) {
  if (!isQuestion(node)) return;
  visit(node);
  node.branches.forEach(branch => everyQuestion(branch.then, visit));
}

function remarksOn(spec) {
  const remarks = [];
  everyQuestion(spec.root, question => {
    const answers = question.branches.map(branch => branch.answer);
    if (answers.length === 1) remarks.push(`"${question.question}" has only one answer, so it decides nothing`);
    const repeated = answers.find((answer, i) => answers.indexOf(answer) !== i);
    if (repeated) remarks.push(`"${question.question}" has the answer ${repeated} twice`);
    const outcomes = question.branches.map(branch => (isOutcome(branch.then) ? branch.then.outcome : null));
    if (answers.length > 1 && outcomes[0] && outcomes.every(outcome => outcome === outcomes[0])) remarks.push(`Every answer to "${question.question}" leads to ${outcomes[0]}, so the question changes nothing`);
  });
  return remarks;
}

function sceneOf(spec) {
  const nodes = [], edges = [];
  const place = (node, id) => {
    const own = node.id ?? id;
    nodes.push({ id: own, shape: isQuestion(node) ? 'process' : 'terminator', label: isQuestion(node) ? node.question : node.outcome, status: node.status, tone: node.tone, link: node.link });
    (node.branches || []).forEach((branch, position) => {
      const next = place(branch.then, `${own}.${position + 1}`);
      edges.push({ id: `${own}->${next}`, from: own, to: next, label: branch.answer, line: 'solid', head: 'open', status: branch.status, tone: branch.tone, link: branch.link });
    });
    return own;
  };
  place(spec.root, 'q');
  const inWrittenOrder = new Map(nodes.map((node, position) => [node.id, position]));
  edges.sort((a, b) => inWrittenOrder.get(a.from) - inWrittenOrder.get(b.from) || inWrittenOrder.get(a.to) - inWrittenOrder.get(b.to));
  return { direction: spec.direction || 'down', nodes, edges };
}

module.exports = { kind: 'decision-tree', name: 'Decision tree', symbols: require('./symbols.js'), problemsIn, sceneOf, remarksOn };
