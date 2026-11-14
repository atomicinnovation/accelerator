const test = require('node:test');
const assert = require('node:assert/strict');

const { svgOf, headOf, tailOf, endLabelAt } = require('../lib/render.js');
const { SHAPES } = require('../lib/shapes.js');
const { layoutGraphOf, positionedFrom, laidOut } = require('../lib/layout.js');
const { widthOf } = require('../lib/text.js');

const groupHolding = (markup, attribute) => markup.slice(markup.lastIndexOf('<g ', markup.indexOf(attribute))).split('</g>')[0];

test('a classifier with only a name is as tall as a box', () => {
  assert.deepEqual(SHAPES.classifier.sizeOf({ label: 'Clock' }), { width: 96, height: 32 });
});

test('a classifier grows for its stereotype, for being abstract, and for each member', () => {
  const sized = SHAPES.classifier.sizeOf({ label: 'Clock', stereotype: 'interface', abstract: true, compartments: [['+ now(): Instant'], ['- a: u8', '- b: u8']] });
  assert.equal(sized.height, 32 + 14 + 14 + (11 + 15) + (11 + 30));
  const wide = SHAPES.classifier.sizeOf({ label: 'C', compartments: [['+ claim(version: &str, launcher: &LauncherId): Result<Claim>']] });
  assert.ok(wide.width >= widthOf('+ claim(version: &str, launcher: &LauncherId): Result<Claim>', 'name') + 20);
});

const CLOCK = { id: 'clock', shape: 'classifier', label: 'Clock', stereotype: 'interface', abstract: true, status: 'new',
  compartments: [['+ now(): Instant', { text: '+ epoch(): Instant', static: true }]], x: 20, y: 10, width: 180, height: 101 };
const PLAIN = { id: 'claim', shape: 'classifier', label: 'claim: VersionClaim', underlined: true, compartments: [['version = "1.26.0"']], x: 20, y: 150, width: 180, height: 58 };
const svg = svgOf({ width: 220, height: 220, nodes: [CLOCK, PLAIN], edges: [] }, { uid: 'c' });
const node = id => groupHolding(svg, `data-node="${id}"`);

test('a classifier is a box with its stereotype over its name', () => {
  const clock = node('clock');
  assert.match(clock, /^<g class="node classifier leaf s-new"/);
  assert.match(clock, /<rect class="body" x="20" y="10" width="180" height="101" rx="3"\/>/);
  assert.match(clock, /<text class="stereotype" x="110" y="27" text-anchor="middle">«interface»<\/text>/);
  assert.match(clock, /<text class="title" x="110" y="44" text-anchor="middle">Clock<\/text>/);
  assert.match(clock, /<text class="stereotype" x="110" y="58" text-anchor="middle">\{abstract\}<\/text>/);
});

test('each compartment is ruled off and lists its members from the left', () => {
  const clock = node('clock');
  assert.match(clock, /<line class="rule" x1="20" y1="70" x2="200" y2="70"\/>/);
  assert.match(clock, /<text class="member" x="30" y="87" text-anchor="start">\+ now\(\): Instant<\/text>/);
  assert.match(clock, /<text class="member static" x="30" y="102" text-anchor="start">\+ epoch\(\): Instant<\/text>/);
});

test('an instance has its name underlined', () => {
  assert.match(node('claim'), /<text class="title underlined" x="110" y="170" text-anchor="middle">claim: VersionClaim<\/text>/);
  assert.match(node('claim'), /<text class="member" x="30" y="199" text-anchor="start">version = &quot;1.26.0&quot;<\/text>/);
});

test('a hollow head is a closed triangle the line stops short of', () => {
  assert.deepEqual(headOf('hollow', [[50, 100], [50, 40]]), { path: 'M44 52 L50 40 L56 52 Z', cut: 12 });
});

test('a diamond sits at the start of its line, pointing along it', () => {
  assert.deepEqual(tailOf('diamond', [[50, 40], [50, 100]]), { path: 'M50 40 L55 47 L50 54 L45 47 Z', cut: 14 });
  assert.deepEqual(tailOf('filled-diamond', [[10, 5], [60, 5]]), { path: 'M10 5 L17 0 L24 5 L17 10 Z', cut: 14 });
  assert.deepEqual(tailOf('none', [[0, 0], [0, 9]]), { path: '', cut: 0 });
});

test('an end label sits beside the end of its line, clear of the marker there', () => {
  assert.deepEqual(endLabelAt([50, 40], [50, 100], 14), { x: 57, y: 70, anchor: 'start' });
  assert.deepEqual(endLabelAt([50, 100], [50, 40], 0), { x: 57, y: 92, anchor: 'start' });
  assert.deepEqual(endLabelAt([10, 5], [60, 5], 0), { x: 16, y: -1, anchor: 'start' });
  assert.deepEqual(endLabelAt([60, 5], [10, 5], 12), { x: 42, y: -1, anchor: 'end' });
  assert.deepEqual(endLabelAt([50, 40], [50, 100], 14, -1), { x: 43, y: 70, anchor: 'end' });
  assert.deepEqual(endLabelAt([10, 5], [60, 5], 0, -1), { x: 16, y: 18, anchor: 'start' });
});

const EDGE = { id: 'e', from: 'claim', to: 'clock', line: 'solid', tail: 'filled-diamond', head: 'open', fromLabel: '1', toLabel: '0..* clocks', points: [[110, 150], [110, 111]], labelBox: null };
const joined = svgOf({ width: 220, height: 220, nodes: [CLOCK, PLAIN], edges: [EDGE] }, { uid: 'c' });
const edge = groupHolding(joined, 'data-edge="e"');

test('an edge draws the marker at each end and stops its line short of both', () => {
  assert.match(edge, /<path class="tail filled-diamond" d="M110 150 L105 143 L110 136 L115 143 Z"\/>/);
  assert.match(edge, /<path class="head open" d="M105.5 120 L110 111 L114.5 120"\/>/);
  assert.match(edge, /<path class="line" d="M110 136 L110 111"\/>/);
});

test('an edge writes what is at each of its ends beside that end, on opposite sides of the line', () => {
  assert.match(edge, /<text class="end" x="103" y="128" text-anchor="end">1<\/text>/);
  assert.match(edge, /<text class="end" x="117" y="136" text-anchor="start">0\.\.\* clocks<\/text>/);
});

const SCENE = {
  nodes: [{ id: 'sub', shape: 'classifier', label: 'Sub' }, { id: 'super', shape: 'classifier', label: 'Super' }],
  edges: [{ id: 'sub->super', from: 'sub', to: 'super', head: 'hollow', against: true }],
};

test('an edge drawn against the flow is laid out from its far end', () => {
  const [laidEdge] = layoutGraphOf(SCENE).edges;
  assert.deepEqual([laidEdge.sources, laidEdge.targets], [['super'], ['sub']]);
});

test('an edge drawn against the flow still runs from its own start to its own end', async () => {
  const scene = await laidOut(SCENE);
  const at = id => scene.nodes.find(each => each.id === id);
  const [drawn] = scene.edges;
  assert.ok(at('super').y < at('sub').y, 'the general is not above the specific');
  assert.equal(drawn.points[0][1], at('sub').y);
  assert.equal(drawn.points.at(-1)[1], at('super').y + at('super').height);
  assert.deepEqual([drawn.from, drawn.to], ['sub', 'super']);
});

test('reversing a route keeps the bend that gave its head room at the head', () => {
  const laid = { id: 'root', width: 100, height: 200,
    children: [{ id: 'super', x: 0, y: 0, width: 96, height: 32 }, { id: 'sub', x: 0, y: 150, width: 96, height: 32 }],
    edges: [{ id: 'sub->super', container: 'root', sections: [{ startPoint: { x: 30, y: 32 }, bendPoints: [{ x: 30, y: 38 }, { x: 60, y: 38 }], endPoint: { x: 60, y: 150 } }] }] };
  assert.deepEqual(positionedFrom(laid, SCENE).edges[0].points, [[60, 150], [60, 48], [30, 48], [30, 32]]);
});
