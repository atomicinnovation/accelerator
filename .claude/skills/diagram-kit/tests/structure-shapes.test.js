const test = require('node:test');
const assert = require('node:assert/strict');

const { svgOf } = require('../lib/render.js');
const { SHAPES } = require('../lib/shapes.js');
const { layoutGraphOf, laidOut } = require('../lib/layout.js');
const { cyclesIn, loopThrough } = require('../lib/graphs.js');
const { widthOf } = require('../lib/text.js');

const groupHolding = (markup, attribute) => markup.slice(markup.lastIndexOf('<g ', markup.indexOf(attribute))).split('</g>')[0];
const drawn = (node, extra = []) => groupHolding(svgOf({ width: 400, height: 300, nodes: [node, ...extra], edges: [] }, { uid: 's' }), `data-node="${node.id}"`);

test('a frame holds others, with room for what it is and what it is called', () => {
  assert.equal(SHAPES.frame.holdsOthers, true);
  const frame = SHAPES.frame.frameOf({ stereotype: 'component', label: 'Retention' });
  assert.equal(frame.padding.top, 36);
  assert.ok(frame.minimumWidth >= widthOf('«component»', 'stereotype') + widthOf('Retention', 'title') + 34);
});

test('a frame is a solid square-cornered box that says what it is, then its name', () => {
  const frame = drawn({ id: 'f', shape: 'frame', stereotype: 'component', label: 'Retention', x: 10, y: 20, width: 280, height: 160 });
  assert.match(frame, /^<g class="node frame"/);
  assert.match(frame, /<rect class="body" x="10" y="20" width="280" height="160" rx="3"\/><text class="stereotype" x="24" y="40" text-anchor="start">«component»<\/text><text class="title" x="[\d.]+" y="40" text-anchor="start">Retention<\/text>/);
});

test('a package that holds others is a folder: a tab with its name, over a body', () => {
  assert.equal(SHAPES.package.holdsOthers, true);
  assert.equal(SHAPES.package.frameOf({ label: 'retention' }).padding.top, 38);
  const folder = drawn({ id: 'p', shape: 'package', label: 'retention', x: 10, y: 20, width: 280, height: 160 });
  const tab = Math.ceil(widthOf('retention', 'title')) + 24;
  assert.match(folder, new RegExp(`<path class="body" d="M10 20 H${10 + tab} V42 H290 V180 H10 Z"/><path class="rule" d="M10 42 H${10 + tab}"/><text class="title" x="22" y="36" text-anchor="start">retention</text>`));
});

test('a package that holds nothing is a small folder with its name in the body', () => {
  const size = SHAPES.folder.sizeOf({ label: 'retention' });
  assert.ok(size.width >= widthOf('retention', 'title') + 28);
  assert.equal(size.height, 50);
  const folder = drawn({ id: 'p', shape: 'folder', label: 'retention', x: 10, y: 20, width: 120, height: 50 });
  assert.match(folder, /<path class="body" d="M10 20 H52 V30 H130 V70 H10 Z"\/><path class="rule" d="M10 30 H52"\/><text class="title" x="70" y="54" text-anchor="middle">retention<\/text>/);
});

test('an interface is a small ball, with its name beside it and not inside', () => {
  assert.deepEqual(SHAPES.ball.sizeOf({ label: 'Disk' }), { width: 16, height: 16 });
  assert.equal(SHAPES.ball.outline, 'disc');
  const [ball] = layoutGraphOf({ nodes: [{ id: 'i', shape: 'ball', label: 'Disk' }], edges: [] }).children;
  assert.deepEqual([ball.width, ball.height], [16, 16]);
  assert.equal(ball.labels[0].text, 'Disk');
  assert.ok(ball.labels[0].width >= widthOf('Disk', 'name'));
  assert.match(ball.layoutOptions['elk.nodeLabels.placement'], /OUTSIDE/);
});

test('the layout places the name outside its ball, clear of the ball and of other nodes', async () => {
  const scene = await laidOut({ nodes: [{ id: 'c', shape: 'box', label: 'claims' }, { id: 'i', shape: 'ball', label: 'Disk' }, { id: 'f', shape: 'box', label: 'FsDisk' }], edges: [{ id: 'c->i', from: 'c', to: 'i' }, { id: 'f->i', from: 'f', to: 'i' }] });
  const ball = scene.nodes.find(each => each.id === 'i'), { labelBox } = ball;
  const overlap = (a, b) => a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;
  assert.ok(labelBox && labelBox.width > 0);
  for (const each of scene.nodes) assert.ok(!overlap(labelBox, each), `the name lies on ${each.id}`);
  assert.ok(Math.abs(labelBox.x + labelBox.width / 2 - (ball.x + 8)) < 60 && Math.abs(labelBox.y - ball.y) < 40);
});

test('a ball is drawn as a circle with its name where the layout put it', () => {
  const ball = drawn({ id: 'i', shape: 'ball', label: 'Disk', status: 'new', x: 100, y: 50, width: 16, height: 16, labelBox: { x: 92, y: 70, width: 32, height: 15 } });
  assert.match(ball, /^<g class="node ball leaf s-new"/);
  assert.match(ball, /<circle class="body" cx="108" cy="58" r="8"\/><text class="name outside" x="92" y="81" text-anchor="start">Disk<\/text>/);
});

test('arrows that run in a ring are found, and so is a way round each ring', () => {
  const arrows = [['a', 'b'], ['b', 'c'], ['c', 'a'], ['c', 'd'], ['e', 'e']].map(([from, to]) => ({ from, to }));
  const rings = cyclesIn(arrows);
  assert.deepEqual(rings.map(ring => [...ring].sort()), [['a', 'b', 'c'], ['e']]);
  assert.deepEqual(loopThrough(rings[0], arrows), ['a', 'b', 'c', 'a']);
  assert.deepEqual(loopThrough(rings[1], arrows), ['e', 'e']);
  assert.deepEqual(cyclesIn([{ from: 'a', to: 'b' }]), []);
});

test('a ball\'s name is put beside the flow, so the lines through the ball do not cross it', async () => {
  const joined = direction => laidOut({ direction, nodes: [{ id: 'c', shape: 'box', label: 'claims' }, { id: 'i', shape: 'ball', label: 'Disk' }, { id: 'f', shape: 'box', label: 'FsDisk' }], edges: [{ id: 'c->i', from: 'c', to: 'i' }, { id: 'i->f', from: 'i', to: 'f' }] });
  const down = (await joined('down')).nodes.find(each => each.id === 'i');
  assert.ok(down.labelBox.x >= down.x + down.width, 'the name is not to the right of the ball');
  const right = (await joined('right')).nodes.find(each => each.id === 'i');
  assert.ok(right.labelBox.y >= right.y + right.height, 'the name is not below the ball');
});

test('a deployment node that holds others is a box with depth, with room for its name on the front', () => {
  assert.equal(SHAPES.cuboid.holdsOthers, true);
  const frame = SHAPES.cuboid.frameOf({ stereotype: 'device', label: 'laptop' });
  assert.deepEqual([frame.padding.top, frame.padding.right], [44, 22]);
  const node = drawn({ id: 'n', shape: 'cuboid', stereotype: 'device', label: 'laptop', x: 10, y: 20, width: 200, height: 120 });
  assert.match(node, /<rect class="body" x="10" y="28" width="192" height="112"\/><path class="face" d="M10 28 L18 20 H210 L202 28 Z M202 28 L210 20 V132 L202 140 Z"\/>/);
  assert.match(node, /<text class="stereotype" x="24" y="48" text-anchor="start">«device»<\/text><text class="title" x="[\d.]+" y="48" text-anchor="start">laptop<\/text>/);
});

test('a deployment node that holds nothing is a small box with depth and its name in the middle', () => {
  const size = SHAPES.cuboidbox.sizeOf({ label: 'cdn', stereotype: 'device' });
  assert.ok(size.width >= widthOf('«device»', 'stereotype') + 36);
  assert.equal(size.height, 32 + 14 + 8);
  const node = drawn({ id: 'n', shape: 'cuboidbox', stereotype: 'device', label: 'cdn', x: 10, y: 20, width: 108, height: 54 });
  assert.match(node, /<rect class="body" x="10" y="28" width="100" height="46"\/><path class="face"/);
  assert.match(node, /<text class="stereotype" x="60" y="45" text-anchor="middle">«device»<\/text><text class="title" x="60" y="62" text-anchor="middle">cdn<\/text>/);
});

test('an element says its name, what kind of thing it is, and what it is for', () => {
  const size = SHAPES.element.sizeOf({ label: 'Launcher', type: 'Container: Rust', description: 'Resolves a version and runs its sub-binary.' });
  assert.ok(size.width >= 150 && size.width <= 250);
  const lines = SHAPES.element.descriptionOf({ description: 'Resolves a version and runs its sub-binary.' }, size.width).lines.length;
  assert.equal(size.height, 12 + 15 + 14 + 6 + lines * 14 + 10);
  const element = drawn({ id: 'e', shape: 'element', label: 'Launcher', type: 'Container: Rust', description: 'Runs it.', muted: true, x: 10, y: 20, width: 180, height: 71 });
  assert.match(element, /^<g class="node element leaf muted"/);
  assert.match(element, /<rect class="body" x="10" y="20" width="180" height="71" rx="7"\/><text class="title" x="100" y="43" text-anchor="middle">Launcher<\/text><text class="stereotype" x="100" y="57" text-anchor="middle">\[Container: Rust\]<\/text><text class="detail" x="100" y="77" text-anchor="middle">Runs it\.<\/text>/);
});

test('a person is an element with a head above it', () => {
  const plain = SHAPES.element.sizeOf({ label: 'Operator', type: 'Person' }), person = SHAPES.person.sizeOf({ label: 'Operator', type: 'Person' });
  assert.equal(person.height, plain.height + 18);
  const drawnPerson = drawn({ id: 'p', shape: 'person', label: 'Operator', type: 'Person', x: 10, y: 20, width: 150, height: 69 });
  assert.match(drawnPerson, /<rect class="body" x="10" y="38" width="150" height="51" rx="7"\/><circle class="head" cx="85" cy="32" r="12"\/><text class="title" x="85" y="61" text-anchor="middle">Operator<\/text>/);
});
