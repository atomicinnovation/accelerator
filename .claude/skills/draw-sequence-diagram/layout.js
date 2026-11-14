const { SHAPES } = require('../diagram-kit/lib/shapes.js');
const { blockOf, widthOf } = require('../diagram-kit/lib/text.js');
const { isMessage, isFragment, labelOf, SEVERAL_OPERANDS } = require('./checks.js');

const BAR = 10, BAR_STEP = 6, ROW_GAP = 14, LABEL_LIMIT = 320, LABEL_LIFT = 4;
const LOOP_OUT = 28, LOOP_DROP = 18, CROSS = 16, FRAME_PAD = 14, FRAME_HEAD = 28, NOTE_REACH = 50;
const DEFAULT_LABELS = { create: '«create»', destroy: '«destroy»' };
const LINES = { sync: ['solid', 'filled'], async: ['solid', 'open'], create: ['dashed', 'open'], destroy: ['solid', 'filled'] };

const even = number => 2 * Math.ceil(number / 2);
const settled = number => Math.round(number * 100) / 100;

function planned(steps, numbered, count = { sent: 0 }) {
  return steps.map(step => {
    if (isMessage(step)) {
      const kind = step.kind ?? 'sync', sent = ++count.sent, said = step.label ?? DEFAULT_LABELS[kind];
      return { ...step, is: 'message', kind, id: `m${sent}`, label: numbered ? `${sent}: ${said}` : said, steps: planned(step.steps || [], numbered, count) };
    }
    if (!isFragment(step)) return { ...step, is: 'note' };
    const operands = SEVERAL_OPERANDS.includes(step.fragment) ? step.operands : [{ guard: step.guard, steps: step.steps }];
    return { is: 'fragment', operator: step.fragment, operands: operands.map(operand => ({ guard: operand.guard, steps: planned(operand.steps, numbered, count) })) };
  });
}

function everyPlanned(steps, visit) {
  for (const step of steps) {
    visit(step);
    if (step.is === 'message') everyPlanned(step.steps, visit);
    if (step.is === 'fragment') step.operands.forEach(operand => everyPlanned(operand.steps, visit));
  }
}

const labelBlock = text => blockOf(text, 'message', LABEL_LIMIT);
const noteWidthOver = (step, centres) => (step.over.length > 1
  ? Math.max(...step.over.map(id => centres.get(id))) - Math.min(...step.over.map(id => centres.get(id))) + 2 * NOTE_REACH
  : even(Math.min(Math.max(blockOf(step.note, 'message').width + 20, 120), 220)));

function columnCentres(participants, sizes, steps) {
  const column = new Map(participants.map((participant, i) => [participant.id, i]));
  const gaps = participants.slice(1).map((participant, i) => (sizes[i].width + sizes[i + 1].width) / 2 + 30);
  const needs = [];
  const need = (from, to, room) => { if (to > from && to <= gaps.length) needs.push({ from, to, room }); };
  everyPlanned(steps, step => {
    if (step.is === 'note' && step.over.length === 1) {
      const at = column.get(step.over[0]), half = noteWidthOver(step, new Map()) / 2 + 16;
      need(at - 1, at, half);
      need(at, at + 1, half);
    }
    if (step.is !== 'message') return;
    const from = column.get(step.from), to = column.get(step.to);
    const widest = Math.max(labelBlock(step.label).width, step.reply ? labelBlock(step.reply).width : 0);
    if (from === to) need(from, from + 1, BAR + 3 * BAR_STEP + LOOP_OUT + 8 + widest + 16);
    else need(Math.min(from, to), Math.max(from, to), widest + 44 + (step.kind === 'create' ? sizes[to].width / 2 : 0));
  });
  needs.sort((a, b) => (a.to - a.from) - (b.to - b.from));
  for (const { from, to, room } of needs) {
    if (from < 0) continue;
    const between = gaps.slice(from, to).reduce((sum, gap) => sum + gap, 0);
    if (between < room) for (let i = from; i < to; i++) gaps[i] += (room - between) / (to - from);
  }
  const centres = new Map();
  let x = sizes[0].width / 2;
  participants.forEach((participant, i) => { centres.set(participant.id, Math.round(x)); x += gaps[i] ?? 0; });
  return centres;
}

function placedSceneOf(spec) {
  const participants = spec.participants;
  const steps = planned(spec.steps, spec.numbered !== false);
  const sizes = participants.map(participant => {
    const { width, height } = (participant.kind === 'actor' ? SHAPES.actor : SHAPES.box).sizeOf({ label: labelOf(participant) });
    return { width: even(width), height };
  });
  const sizeOf = new Map(participants.map((participant, i) => [participant.id, sizes[i]]));
  const centres = columnCentres(participants, sizes, steps);
  const feet = Math.max(...sizes.map(size => size.height));
  const created = new Set();
  everyPlanned(steps, step => { if (step.is === 'message' && step.kind === 'create') created.add(step.to); });

  const fragments = [], bars = [], notes = [], crosses = [], edges = [], headTop = new Map(), lifeEnd = new Map();
  const active = new Map(), open = [], whole = { left: Infinity, right: -Infinity };
  let y = feet + 18;

  const reach = (left, right) => {
    const frame = open[open.length - 1] ?? whole;
    frame.left = Math.min(frame.left, left);
    frame.right = Math.max(frame.right, right);
  };
  const activeOn = id => active.get(id) ?? 0;
  const barLeft = (id, level) => centres.get(id) - BAR / 2 + BAR_STEP * level;
  const edgeOfTopBar = (id, facingRight) => (activeOn(id) ? barLeft(id, activeOn(id) - 1) + (facingRight ? BAR : 0) : centres.get(id));
  const marksOf = step => ({ status: step.status, tone: step.tone, link: step.link });

  function sendBetween(id, from, to, label, [line, head], x1, x2, marks, lowest = 0) {
    const block = labelBlock(label), width = Math.ceil(block.width) + 2;
    const lineY = Math.max(y + block.height + LABEL_LIFT, y + lowest);
    edges.push({ id, from, to, label, line, head, plate: false, ...marks, points: [[x1, lineY], [x2, lineY]],
      labelBox: { x: settled((x1 + x2) / 2 - width / 2), y: lineY - LABEL_LIFT - block.height, width, height: block.height } });
    reach(Math.min(x1, x2, (x1 + x2) / 2 - width / 2), Math.max(x1, x2, (x1 + x2) / 2 + width / 2));
    return lineY;
  }

  function activate(step, top) {
    const bar = { id: `act:${step.id}`, shape: 'activation', behind: true, inert: true, status: step.status, tone: step.tone, x: barLeft(step.to, activeOn(step.to)), y: top, width: BAR };
    bars.push(bar);
    active.set(step.to, activeOn(step.to) + 1);
    return bar;
  }

  function deactivate(step, bar, bottom) {
    bar.height = bottom - bar.y;
    active.set(step.to, activeOn(step.to) - 1);
    y = bottom + ROW_GAP;
  }

  function placeToSelf(step) {
    const id = step.from, activates = step.kind === 'sync' || step.steps.length > 0;
    const out = edgeOfTopBar(id, true), back = activates ? barLeft(id, activeOn(id)) + BAR : out, turn = Math.max(out, back) + LOOP_OUT;
    const top = y + 4, block = labelBlock(step.label), width = Math.ceil(block.width) + 2;
    edges.push({ id: step.id, from: id, to: id, label: step.label, line: LINES[step.kind][0], head: LINES[step.kind][1], plate: false, ...marksOf(step),
      points: [[out, top], [turn, top], [turn, top + LOOP_DROP], [back, top + LOOP_DROP]],
      labelBox: { x: turn + 8, y: settled(top + LOOP_DROP / 2 - block.height / 2), width, height: block.height } });
    reach(centres.get(id) - BAR / 2, turn + 8 + width);
    y = top + LOOP_DROP + ROW_GAP;
    if (!activates) return;
    const bar = activate(step, top + LOOP_DROP);
    placeSteps(step.steps);
    deactivate(step, bar, Math.max(bar.y + 14, y - 6));
  }

  function placeMessage(step) {
    if (step.from === step.to) return placeToSelf(step);
    const rightwards = centres.get(step.to) > centres.get(step.from), towards = rightwards ? -1 : 1;
    const activates = step.kind === 'sync' || (step.kind === 'async' && step.steps.length > 0);
    const size = sizeOf.get(step.to), at = centres.get(step.to);
    const start = edgeOfTopBar(step.from, rightwards);
    const arrivals = {
      create: at + towards * size.width / 2,
      destroy: at + towards * CROSS / 2,
    };
    const end = arrivals[step.kind] ?? (activates ? barLeft(step.to, activeOn(step.to)) + (rightwards ? 0 : BAR) : edgeOfTopBar(step.to, !rightwards));
    const lineY = sendBetween(step.id, step.from, step.to, step.label, LINES[step.kind], start, end, marksOf(step), step.kind === 'create' ? size.height / 2 : 0);
    y = lineY + ROW_GAP;
    if (step.kind === 'create') {
      headTop.set(step.to, lineY - size.height / 2);
      reach(at - size.width / 2, at + size.width / 2);
      y = lineY + size.height / 2 + ROW_GAP;
    }
    if (step.kind === 'destroy') {
      crosses.push({ id: `end:${step.to}`, shape: 'destroy', inert: true, x: at - CROSS / 2, y: lineY - CROSS / 2, width: CROSS, height: CROSS });
      lifeEnd.set(step.to, lineY);
      y = lineY + CROSS / 2 + ROW_GAP;
    }
    if (!activates) return undefined;
    const bar = activate(step, lineY);
    placeSteps(step.steps);
    if (!step.reply) return deactivate(step, bar, Math.max(bar.y + 14, y - 6));
    const replied = sendBetween(`${step.id}:reply`, step.to, step.from, step.reply, ['dashed', 'open'], bar.x + (rightwards ? 0 : BAR), start, {});
    return deactivate(step, bar, replied);
  }

  function placeFragment(step) {
    const frame = { id: `frag:${fragments.length + 1}`, shape: 'fragment', operator: step.operator, guard: step.operands[0].guard, dividers: [], behind: true, inert: true, y, left: Infinity, right: -Infinity };
    fragments.push(frame);
    open.push(frame);
    y += FRAME_HEAD;
    step.operands.forEach((operand, position) => {
      if (position) { frame.dividers.push({ y: y - 4, guard: operand.guard }); y += 22; }
      placeSteps(operand.steps);
    });
    open.pop();
    const guards = frame.dividers.map(divider => (divider.guard ? widthOf(`[${divider.guard}]`, 'guard') + 24 : 0));
    const { left, right } = frame;
    frame.x = left - FRAME_PAD;
    frame.width = Math.ceil(Math.max(right - left + 2 * FRAME_PAD, SHAPES.fragment.headWidthOf(frame) + 8, ...guards));
    frame.height = y - ROW_GAP + 10 - frame.y;
    delete frame.left;
    delete frame.right;
    reach(frame.x, frame.x + frame.width);
    y = frame.y + frame.height + ROW_GAP;
  }

  function placeNote(step) {
    const width = noteWidthOver(step, centres), middle = (Math.min(...step.over.map(id => centres.get(id))) + Math.max(...step.over.map(id => centres.get(id)))) / 2;
    const { height } = SHAPES.note.sizeOf({ text: step.note }, width);
    notes.push({ id: `note:${notes.length + 1}`, shape: 'note', text: step.note, status: step.status, tone: step.tone, inert: true, x: middle - width / 2, y, width, height });
    reach(middle - width / 2, middle + width / 2);
    y += height + ROW_GAP;
  }

  function placeSteps(list) {
    for (const step of list) ({ message: placeMessage, fragment: placeFragment, note: placeNote })[step.is](step);
  }

  placeSteps(steps);
  const foot = y + 6;
  const heads = participants.map((participant, i) => {
    const { width, height } = sizes[i], at = centres.get(participant.id);
    reach(at - width / 2, at + width / 2);
    return { id: participant.id, shape: participant.kind === 'actor' ? 'actor' : 'box', label: labelOf(participant), status: participant.status, tone: participant.tone, link: participant.link,
      x: at - width / 2, y: headTop.get(participant.id) ?? feet - height, width, height };
  });
  const lifelines = heads.map(head => {
    const top = head.y + head.height;
    return { id: `life:${head.id}`, shape: 'lifeline', behind: true, inert: true, x: head.x + head.width / 2, y: top, width: 0, height: (lifeEnd.get(head.id) ?? foot) - top };
  });

  const shift = 1 - Math.floor(whole.left);
  const moved = node => ({ ...node, x: settled(node.x + shift) });
  return {
    width: Math.ceil(whole.right) + shift + 1,
    height: foot + 2,
    nodes: [...lifelines, ...bars, ...fragments, ...heads, ...notes, ...crosses].map(moved),
    edges: edges.map(edge => ({ ...edge, points: edge.points.map(([x, at]) => [settled(x + shift), at]), labelBox: { ...edge.labelBox, x: settled(edge.labelBox.x + shift) } })),
  };
}

module.exports = { placedSceneOf };
