const test = require('node:test');
const assert = require('node:assert/strict');

const notation = require('../notation.js');

const SPEC = {
  id: 'start-up', kind: 'sequence-diagram', title: 'Start-up claims before any lookup',
  participants: [{ id: 'user', kind: 'actor', label: 'Operator' }, { id: 'launcher', link: 'src/start_up.rs:1' }, { id: 'claims', status: 'new' }, { id: 'disk', label: 'Disk' }],
  steps: [
    { from: 'user', to: 'launcher', label: 'accelerator vcs status', steps: [
      { from: 'launcher', to: 'claims', label: 'claim(version)', reply: 'Fresh', status: 'new', link: 'src/claims.rs:45', steps: [
        { from: 'claims', to: 'disk', label: 'lstat', reply: 'mtime' },
        { fragment: 'opt', guard: 'older than a tenth of the window', steps: [{ from: 'claims', to: 'disk', label: 'write claim', kind: 'async', tone: 'risk' }] },
      ] },
      { fragment: 'alt', operands: [
        { guard: 'cached', steps: [{ from: 'launcher', to: 'launcher', label: 'exec' }] },
        { guard: 'else', steps: [{ note: 'Cold path: fetch, then prune.', over: ['launcher', 'disk'], tone: 'risk' }] },
      ] },
    ] },
  ],
};
const scene = notation.placedSceneOf(SPEC);
const node = id => scene.nodes.find(each => each.id === id);
const edge = id => scene.edges.find(each => each.id === id);
const centre = id => node(id).x + node(id).width / 2;
const lineOf = id => edge(id).points[0][1];
const of = shape => scene.nodes.filter(each => each.shape === shape);
const inside = (inner, outer) => inner.x >= outer.x && inner.y >= outer.y && inner.x + inner.width <= outer.x + outer.width && inner.y + inner.height <= outer.y + outer.height;
const boundsOf = each => { const xs = each.points.map(p => p[0]), ys = each.points.map(p => p[1]); return { x: Math.min(...xs), y: Math.min(...ys), width: Math.max(...xs) - Math.min(...xs), height: Math.max(...ys) - Math.min(...ys) }; };

test('participants stand left to right in the order given, feet on one line', () => {
  const heads = ['user', 'launcher', 'claims', 'disk'].map(node);
  assert.deepEqual(heads.map(each => each.shape), ['actor', 'box', 'box', 'box']);
  heads.slice(1).forEach((head, i) => assert.ok(head.x >= heads[i].x + heads[i].width + 20));
  assert.equal(new Set(heads.map(each => each.y + each.height)).size, 1);
  assert.equal(node('disk').label, 'Disk');
  assert.equal(node('claims').status, 'new');
  assert.equal(node('launcher').link, 'src/start_up.rs:1');
});

test('each participant has a lifeline from its feet to the foot of the diagram', () => {
  for (const id of ['user', 'launcher', 'claims', 'disk']) {
    const life = node(`life:${id}`);
    assert.deepEqual([life.shape, life.inert, life.behind, life.width], ['lifeline', true, true, 0]);
    assert.equal(life.x, centre(id));
    assert.equal(life.y, node(id).y + node(id).height);
    assert.ok(life.y + life.height <= scene.height && life.y + life.height > scene.height - 30);
  }
});

test('messages are numbered in the order they are sent, and replies are not', () => {
  assert.deepEqual(scene.edges.map(each => [each.id, each.label]), [
    ['m1', '1: accelerator vcs status'], ['m2', '2: claim(version)'], ['m3', '3: lstat'], ['m3:reply', 'mtime'],
    ['m4', '4: write claim'], ['m2:reply', 'Fresh'], ['m5', '5: exec'],
  ]);
});

test('a sequence that asks not to be numbered is not', () => {
  assert.equal(notation.placedSceneOf({ ...SPEC, numbered: false }).edges[0].label, 'accelerator vcs status');
});

test('time runs down the page: each message is below the one before', () => {
  const lines = scene.edges.map(each => each.points[0][1]);
  lines.slice(1).forEach((y, i) => assert.ok(y > lines[i], `${scene.edges[i + 1].id} is not below ${scene.edges[i].id}`));
});

test('a message between participants is level, from its sender to its receiver', () => {
  for (const id of ['m1', 'm2', 'm3', 'm3:reply', 'm4', 'm2:reply']) assert.equal(edge(id).points.length === 2 && edge(id).points[0][1], edge(id).points[1][1], id);
  assert.deepEqual([edge('m2').from, edge('m2').to], ['launcher', 'claims']);
  assert.ok(edge('m2').points[0][0] < edge('m2').points[1][0]);
  assert.deepEqual([edge('m2:reply').from, edge('m2:reply').to], ['claims', 'launcher']);
  assert.ok(edge('m2:reply').points[0][0] > edge('m2:reply').points[1][0]);
});

test('a call is a solid line with a filled head, an asynchronous message has an open one, and a reply is dashed', () => {
  assert.deepEqual([edge('m2').line, edge('m2').head], ['solid', 'filled']);
  assert.deepEqual([edge('m4').line, edge('m4').head], ['solid', 'open']);
  assert.deepEqual([edge('m2:reply').line, edge('m2:reply').head], ['dashed', 'open']);
});

test('a message keeps its status, tone and link, and its label has no plate', () => {
  assert.deepEqual([edge('m2').status, edge('m2').link, edge('m2').plate], ['new', 'src/claims.rs:45', false]);
  assert.equal(edge('m4').tone, 'risk');
});

test('a call activates its receiver from the call until the reply', () => {
  const bar = node('act:m2');
  assert.deepEqual([bar.shape, bar.inert, bar.behind, bar.status], ['activation', true, true, 'new']);
  assert.equal(bar.x + bar.width / 2, centre('claims'));
  assert.equal(bar.y, lineOf('m2'));
  assert.equal(bar.y + bar.height, lineOf('m2:reply'));
});

test('a call with no reply drawn is active until what it does is done', () => {
  const bar = node('act:m1');
  assert.equal(bar.y, lineOf('m1'));
  assert.ok(bar.y + bar.height > lineOf('m5'));
});

test('an arrow stops at the bar it activates and starts from the bar it is sent in', () => {
  const sender = node('act:m1'), receiver = node('act:m2');
  assert.deepEqual(edge('m2').points.map(point => point[0]), [sender.x + sender.width, receiver.x]);
  assert.deepEqual(edge('m2:reply').points.map(point => point[0]), [receiver.x, sender.x + sender.width]);
});

test('an asynchronous message that starts nothing leaves no bar', () => {
  assert.equal(node('act:m4'), undefined);
});

test('a label sits above its line, between the lifelines the message joins', () => {
  for (const id of ['m1', 'm2', 'm3', 'm3:reply', 'm4', 'm2:reply']) {
    const { labelBox, points } = edge(id), [left, right] = points.map(point => point[0]).sort((a, b) => a - b);
    assert.ok(labelBox.y + labelBox.height <= points[0][1], `${id} label overlaps its line`);
    assert.ok(labelBox.x >= left && labelBox.x + labelBox.width <= right, `${id} label does not fit`);
  }
});

test('a message to oneself loops out to the right, with its label beside it', () => {
  const loop = edge('m5');
  assert.equal(loop.points.length, 4);
  assert.ok(loop.points[1][0] > loop.points[0][0] && loop.points[2][1] > loop.points[1][1] && loop.points[3][0] < loop.points[2][0]);
  assert.ok(loop.labelBox.x > loop.points[1][0]);
  assert.equal(node('act:m5').x > node('act:m1').x, true);
});

test('a fragment frames everything that happens in it', () => {
  const [opt, alt] = of('fragment');
  assert.deepEqual([opt.operator, opt.guard, opt.inert, opt.behind], ['opt', 'older than a tenth of the window', true, true]);
  assert.ok(inside(boundsOf(edge('m4')), opt) && inside(edge('m4').labelBox, opt));
  assert.ok(!inside(boundsOf(edge('m3')), opt));
  assert.ok(inside(boundsOf(edge('m5')), alt) && inside(edge('m5').labelBox, alt) && inside(of('note')[0], alt));
});

test('a fragment with several operands shows the first guard in its head and divides the rest', () => {
  const alt = of('fragment')[1];
  assert.deepEqual([alt.operator, alt.guard], ['alt', 'cached']);
  assert.deepEqual(alt.dividers.map(divider => divider.guard), ['else']);
  assert.ok(alt.dividers[0].y > lineOf('m5') && alt.dividers[0].y < of('note')[0].y);
});

test('a fragment is wide enough for its operator and guard', () => {
  const { SHAPES } = require('../../diagram-kit/lib/shapes.js');
  for (const fragment of of('fragment')) assert.ok(fragment.width >= SHAPES.fragment.headWidthOf(fragment) + 8);
});

test('a note lies over the participants it names', () => {
  const [note] = of('note');
  assert.deepEqual([note.text, note.tone, note.inert], ['Cold path: fetch, then prune.', 'risk', true]);
  assert.ok(note.x < centre('launcher') && note.x + note.width > centre('disk'));
});

test('everything is on the page and no two labels collide', () => {
  for (const each of scene.nodes) assert.ok(each.x >= 0 && each.y >= 0 && each.x + each.width <= scene.width && each.y + each.height <= scene.height, each.id);
  const boxes = scene.edges.map(each => each.labelBox);
  boxes.forEach((a, i) => boxes.slice(i + 1).forEach(b => assert.ok(!(a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height))));
});

test('lifelines are drawn first, then bars, then fragments over them, then the participants', () => {
  const order = scene.nodes.map(each => each.shape);
  assert.ok(order.lastIndexOf('lifeline') < order.indexOf('activation'));
  assert.ok(order.lastIndexOf('activation') < order.indexOf('fragment'));
  assert.ok(order.lastIndexOf('fragment') < order.indexOf('actor'));
});

const LIVES = {
  id: 'lives', kind: 'sequence-diagram', title: 'A claim is made and dropped',
  participants: [{ id: 'launcher' }, { id: 'claim' }],
  steps: [
    { from: 'launcher', to: 'claim', kind: 'create' },
    { from: 'launcher', to: 'claim', label: 'refresh()' },
    { from: 'launcher', to: 'claim', kind: 'destroy', label: 'drop' },
  ],
};
const lives = notation.placedSceneOf(LIVES);
const lived = id => lives.nodes.find(each => each.id === id);
const sent = id => lives.edges.find(each => each.id === id);

test('a created participant appears where it is created, at the end of a dashed arrow', () => {
  const head = lived('claim'), made = sent('m1');
  assert.ok(head.y > lived('launcher').y);
  assert.deepEqual([made.label, made.line, made.head], ['1: «create»', 'dashed', 'open']);
  assert.equal(made.points[1][0], head.x);
  assert.ok(made.points[1][1] > head.y && made.points[1][1] < head.y + head.height);
  assert.equal(lived('life:claim').y, head.y + head.height);
});

test('a destroyed participant ends in a cross where the message reaches it', () => {
  const cross = lives.nodes.find(each => each.shape === 'destroy'), dropped = sent('m3');
  assert.equal(cross.x + cross.width / 2, lived('claim').x + lived('claim').width / 2);
  assert.equal(cross.y + cross.height / 2, dropped.points[1][1]);
  assert.equal(dropped.points[1][0], cross.x);
  assert.equal(lived('life:claim').y + lived('life:claim').height, dropped.points[1][1]);
  assert.equal(dropped.label, '3: drop');
});

test('the same sequence is laid out the same way every time', () => {
  assert.deepEqual(notation.placedSceneOf(SPEC), scene);
});

const fs = require('node:fs');
const path = require('node:path');
const { kitWith } = require('../../diagram-kit/lib/index.js');

const EXAMPLES = path.join(__dirname, '..', 'examples');

for (const file of fs.readdirSync(EXAMPLES).filter(name => name.endsWith('.json'))) {
  test(`the example ${file} is sound and lays out`, async () => {
    const kit = kitWith(), example = JSON.parse(fs.readFileSync(path.join(EXAMPLES, file), 'utf8'));
    assert.deepEqual(kit.problemsIn(example), []);
    assert.ok((await kit.laidOut(example)).edges.length > 0);
    assert.match(kit.figureOf(example, await kit.laidOut(example)), /<svg class="dk"/);
  });
}
