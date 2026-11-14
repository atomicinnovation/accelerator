const test = require('node:test');
const assert = require('node:assert/strict');

const { svgOf } = require('../lib/render.js');
const { SHAPES } = require('../lib/shapes.js');
const { layoutGraphOf, positionedFrom, laidOut } = require('../lib/layout.js');
const { widthOf } = require('../lib/text.js');

const groupHolding = (markup, attribute) => markup.slice(markup.lastIndexOf('<g ', markup.indexOf(attribute))).split('</g>')[0];

test('a state is sized for its name and for each activity it lists', () => {
  const plain = SHAPES.state.sizeOf({ label: 'Idle' });
  const busy = SHAPES.state.sizeOf({ label: 'Idle', activities: ['entry / start the timer for the claim window', 'exit / stop it'] });
  assert.deepEqual(plain, { width: 96, height: 32 });
  assert.ok(busy.width >= widthOf('entry / start the timer for the claim window', 'detail') + 24);
  assert.equal(busy.height, 32 + 8 + 2 * 14 + 4);
});

test('the start, the end and a choice have fixed sizes', () => {
  assert.deepEqual(SHAPES.initial.sizeOf({}), { width: 14, height: 14 });
  assert.deepEqual(SHAPES.final.sizeOf({}), { width: 20, height: 20 });
  assert.deepEqual(SHAPES.choice.sizeOf({}), { width: 28, height: 28 });
});

test('a composite state holds others, with room for its name above a rule', () => {
  assert.equal(SHAPES.composite.holdsOthers, true);
  const frame = SHAPES.composite.frameOf({ label: 'Claiming a version' });
  assert.equal(frame.padding.top, 40);
  assert.ok(frame.minimumWidth >= widthOf('Claiming a version', 'name') + 28);
  assert.match(layoutGraphOf({ nodes: [{ id: 'c', shape: 'composite', label: 'C', children: [{ id: 's', shape: 'state', label: 'S' }] }], edges: [] }).children[0].layoutOptions['elk.padding'], /top=40/);
});

const SCENE = {
  width: 300, height: 300,
  nodes: [
    { id: 'claiming', shape: 'composite', label: 'Claiming', status: 'new', x: 20, y: 60, width: 260, height: 200 },
    { id: 'start', shape: 'initial', x: 143, y: 10, width: 14, height: 14 },
    { id: 'idle', shape: 'state', label: 'Idle', activities: ['entry / arm', 'do / wait'], tone: 'risk', x: 60, y: 110, width: 120, height: 72 },
    { id: 'pick', shape: 'choice', x: 200, y: 120, width: 28, height: 28 },
    { id: 'done', shape: 'final', x: 140, y: 270, width: 20, height: 20 },
  ],
  edges: [{ id: 'start->idle', from: 'start', to: 'idle', head: 'open', points: [[150, 24], [150, 110]], labelBox: null }],
};
const svg = svgOf(SCENE, { uid: 'sm', title: 'States' });
const node = id => groupHolding(svg, `data-node="${id}"`);

test('a state is a rounded box with its name over a rule and its activities below', () => {
  const idle = node('idle');
  assert.match(idle, /^<g class="node state leaf t-risk"/);
  assert.match(idle, /<rect class="body" x="60" y="110" width="120" height="72" rx="12"\/>/);
  assert.match(idle, /<text class="name" x="120" y="130" text-anchor="middle">Idle<\/text>/);
  assert.match(idle, /<line class="rule" x1="60" y1="142" x2="180" y2="142"\/>/);
  assert.match(idle, /<text class="detail" x="72" y="158" text-anchor="start">entry \/ arm<\/text><text class="detail" x="72" y="172" text-anchor="start">do \/ wait<\/text>/);
});

test('the start is a filled disc, the end a ringed one, and a choice a diamond', () => {
  assert.match(node('start'), /<circle class="body" cx="150" cy="17" r="7"\/>/);
  assert.match(node('done'), /<circle class="body" cx="150" cy="280" r="10"\/><circle class="core" cx="150" cy="280" r="5.5"\/>/);
  assert.match(node('pick'), /<path class="body" d="M214 120 L228 134 L214 148 L200 134 Z"\/>/);
});

test('a part with no name is announced by what it is', () => {
  assert.match(node('start'), /aria-label="Trace the arrows of the start"/);
  assert.match(node('done'), /aria-label="Trace the arrows of the end"/);
  assert.match(node('pick'), /aria-label="Trace the arrows of the choice"/);
});

test('a composite state is a rounded frame with its name over a rule, drawn behind what it holds', () => {
  const claiming = node('claiming');
  assert.match(claiming, /^<g class="node composite s-new"/);
  assert.match(claiming, /<rect class="body" x="20" y="60" width="260" height="200" rx="12"\/>/);
  assert.match(claiming, /<text class="name" x="34" y="80" text-anchor="start">Claiming<\/text><line class="rule" x1="20" y1="90" x2="280" y2="90"\/>/);
  assert.ok(svg.indexOf('data-node="claiming"') < svg.indexOf('data-edge='));
  assert.ok(svg.indexOf('data-edge=') < svg.indexOf('data-node="idle"'));
});

test('only what can be traced and holds nothing is a leaf', () => {
  assert.doesNotMatch(node('claiming'), /class="[^"]* leaf/);
  const boxed = svgOf({ width: 10, height: 10, nodes: [{ id: 'a', shape: 'box', label: 'a', x: 0, y: 0, width: 10, height: 10 }, { id: 'l', shape: 'lifeline', inert: true, x: 0, y: 0, width: 0, height: 5 }], edges: [] }, {});
  assert.match(boxed, /class="node box leaf"/);
  assert.match(boxed, /class="node lifeline"/);
});

const ROUND = {
  nodes: [{ id: 'start', shape: 'initial' }, { id: 'pick', shape: 'choice' }, { id: 'a', shape: 'state', label: 'A' }],
  edges: [{ id: 'e1', from: 'start', to: 'pick' }, { id: 'e2', from: 'pick', to: 'a' }],
};

test('an arrow that meets a disc or a diamond off its middle is carried on to its outline', () => {
  const laid = {
    id: 'root', width: 100, height: 200,
    children: [{ id: 'start', x: 43, y: 0, width: 14, height: 14 }, { id: 'pick', x: 36, y: 60, width: 28, height: 28 }, { id: 'a', x: 2, y: 140, width: 96, height: 32 }],
    edges: [
      { id: 'e1', container: 'root', sections: [{ startPoint: { x: 54, y: 14 }, endPoint: { x: 54, y: 60 } }] },
      { id: 'e2', container: 'root', sections: [{ startPoint: { x: 43, y: 88 }, endPoint: { x: 43, y: 140 } }] },
    ],
  };
  const scene = positionedFrom(laid, ROUND);
  const [first, second] = scene.edges.map(edge => edge.points);
  assert.deepEqual(first, [[54, 12.74], [54, 64]]);
  assert.deepEqual(second, [[43, 81], [43, 140]]);
});

test('the layout engine brings every arrow to the outline of a disc or a diamond', async () => {
  const scene = await laidOut(ROUND);
  const at = id => scene.nodes.find(each => each.id === id);
  const [intoChoice, outOfChoice] = scene.edges;
  const start = at('start'), pick = at('pick');
  const leaves = intoChoice.points[0], arrives = intoChoice.points.at(-1), departs = outOfChoice.points[0];
  assert.ok(Math.abs(Math.hypot(leaves[0] - (start.x + 7), leaves[1] - (start.y + 7)) - 7) < 0.02);
  for (const [x, y] of [arrives, departs]) assert.ok(Math.abs(Math.abs(x - (pick.x + 14)) + Math.abs(y - (pick.y + 14)) - 14) < 0.02);
});
