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

test('a boundary holds others, with room for what it is and what it is called', () => {
  assert.equal(SHAPES.boundary.holdsOthers, true);
  const frame = SHAPES.boundary.frameOf({ stereotype: 'aggregate', label: 'VersionClaims' });
  assert.equal(frame.padding.top, 36);
  assert.ok(frame.minimumWidth >= widthOf('«aggregate»', 'stereotype') + widthOf('VersionClaims', 'title') + 34);
});

test('a boundary is a dashed frame that says what it is, then its name', () => {
  const drawn = svgOf({ width: 300, height: 200, nodes: [{ id: 'agg', shape: 'boundary', stereotype: 'aggregate', label: 'Order', status: 'new', x: 10, y: 20, width: 280, height: 160 }], edges: [] }, { uid: 'b' });
  const boundary = groupHolding(drawn, 'data-node="agg"');
  assert.match(boundary, /^<g class="node boundary s-new"/);
  assert.match(boundary, /<rect class="body" x="10" y="20" width="280" height="160" rx="10"\/>/);
  assert.match(boundary, /<text class="stereotype" x="24" y="40" text-anchor="start">«aggregate»<\/text><text class="title" x="[\d.]+" y="40" text-anchor="start">Order<\/text>/);
});

const ENDS = {
  nodes: [{ id: 'whole', shape: 'classifier', label: 'Whole' }, { id: 'part', shape: 'classifier', label: 'Part' }, { id: 'other', shape: 'classifier', label: 'Other' }, { id: 'general', shape: 'classifier', label: 'General' }],
  edges: [
    { id: 'owns', from: 'whole', to: 'part', label: 'holds', tail: 'filled-diamond', head: 'none', fromLabel: '1', toLabel: '0..* parts' },
    { id: 'knows', from: 'whole', to: 'other', head: 'open', toLabel: '0..1 other' },
    { id: 'is', from: 'whole', to: 'general', head: 'hollow', against: true, fromLabel: 'specific', toLabel: 'general' },
  ],
};

test('the layout is told about the label at each end of an edge, and which end it belongs to', () => {
  const [owns, knows, is] = layoutGraphOf(ENDS).edges;
  assert.deepEqual(owns.labels.map(label => [label.id, (label.layoutOptions || {})['elk.edgeLabels.placement']]), [['owns#centre', undefined], ['owns#from', 'TAIL'], ['owns#to', 'HEAD']]);
  assert.deepEqual(knows.labels.map(label => label.id), ['knows#to']);
  assert.deepEqual(is.labels.map(label => [label.id, label.layoutOptions['elk.edgeLabels.placement']]), [['is#from', 'HEAD'], ['is#to', 'TAIL']]);
  assert.ok(owns.labels[2].width >= widthOf('0..* parts', 'end'));
});

test('each end label is placed by the layout, clear of every box and of every other label', async () => {
  const scene = await laidOut(ENDS);
  const boxes = scene.edges.flatMap(edge => [edge.labelBox, edge.fromLabelBox, edge.toLabelBox].filter(Boolean));
  assert.equal(boxes.length, 6);
  const overlap = (a, b) => a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;
  boxes.forEach((a, i) => boxes.slice(i + 1).forEach(b => assert.ok(!overlap(a, b), 'two labels overlap')));
  for (const box of boxes) for (const each of scene.nodes) assert.ok(!overlap(box, each), `a label lies on ${each.id}`);
});

test('an end label is placed near the end it describes', async () => {
  const scene = await laidOut(ENDS);
  const near = (box, [x, y]) => Math.abs(box.y + box.height / 2 - y) < 40 && Math.abs(box.x + box.width / 2 - x) < 80;
  for (const edge of scene.edges) {
    if (edge.fromLabelBox) assert.ok(near(edge.fromLabelBox, edge.points[0]), `${edge.id} from`);
    if (edge.toLabelBox) assert.ok(near(edge.toLabelBox, edge.points.at(-1)), `${edge.id} to`);
  }
});

test('an end label is drawn where the layout put it', () => {
  const placed = { ...EDGE, fromLabelBox: { x: 120, y: 130, width: 10, height: 14 }, toLabelBox: { x: 40, y: 114, width: 62, height: 14 } };
  const drawn = groupHolding(svgOf({ width: 220, height: 220, nodes: [CLOCK, PLAIN], edges: [placed] }, { uid: 'c' }), 'data-edge="e"');
  assert.match(drawn, /<text class="end" x="120" y="141" text-anchor="start">1<\/text>/);
  assert.match(drawn, /<text class="end" x="40" y="125" text-anchor="start">0\.\.\* clocks<\/text>/);
});
