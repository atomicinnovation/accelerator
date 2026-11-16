const test = require('node:test');
const assert = require('node:assert/strict');

const { svgOf } = require('../lib/render.js');
const { SHAPES } = require('../lib/shapes.js');
const { widthOf } = require('../lib/text.js');

const groupHolding = (markup, attribute) => markup.slice(markup.lastIndexOf('<g ', markup.indexOf(attribute))).split('</g>')[0];
const drawn = node => groupHolding(svgOf({ width: 400, height: 200, nodes: [node], edges: [] }, { uid: 'f' }), `data-node="${node.id}"`);

test('a process, a terminator, an input or output and a subprocess are sized for their text', () => {
  const text = widthOf('Read the manifest', 'message');
  assert.deepEqual(SHAPES.process.sizeOf({ label: 'Go' }), { width: 96, height: 36 });
  assert.ok(SHAPES.process.sizeOf({ label: 'Read the manifest' }).width >= text + 28);
  assert.ok(SHAPES.terminator.sizeOf({ label: 'Read the manifest' }).width >= text + 40);
  assert.ok(SHAPES.io.sizeOf({ label: 'Read the manifest' }).width >= text + 52);
  assert.ok(SHAPES.subprocess.sizeOf({ label: 'Read the manifest' }).width >= text + 44);
});

test('long text wraps, and the shape grows taller for it', () => {
  const tall = SHAPES.process.sizeOf({ label: 'Read the manifest from the cache root and check every digest it names' });
  assert.ok(tall.width <= 260);
  assert.ok(tall.height >= 36 + 15);
});

test('a decision is a diamond large enough to hold its question inside', () => {
  const size = SHAPES.decision.sizeOf({ label: 'Cached?' });
  assert.ok(size.width >= 2 * widthOf('Cached?', 'message') + 16);
  assert.equal(size.height, 2 * 15 + 18);
  assert.equal(SHAPES.decision.outline, 'diamond');
});

test('a process is a square-cornered box with its text in the middle', () => {
  const markup = drawn({ id: 'p', shape: 'process', label: 'Go', status: 'new', x: 10, y: 20, width: 96, height: 36 });
  assert.match(markup, /^<g class="node process leaf s-new"/);
  assert.match(markup, /<rect class="body" x="10" y="20" width="96" height="36" rx="2"\/><text class="message" x="58" y="41.5" text-anchor="middle">Go<\/text>/);
});

test('a terminator has rounded ends', () => {
  assert.match(drawn({ id: 't', shape: 'terminator', label: 'Start', x: 10, y: 20, width: 96, height: 36 }), /<rect class="body" x="10" y="20" width="96" height="36" rx="18"\/>/);
});

test('an input or output leans to the right', () => {
  assert.match(drawn({ id: 'i', shape: 'io', label: 'Read', x: 10, y: 20, width: 120, height: 36 }), /<path class="body" d="M22 20 H130 L118 56 H10 Z"\/>/);
});

test('a subprocess has a second line inside each side', () => {
  assert.match(drawn({ id: 's', shape: 'subprocess', label: 'Prune', x: 10, y: 20, width: 120, height: 36 }), /<rect class="body" x="10" y="20" width="120" height="36" rx="2"\/><path class="rule" d="M18 20 V56 M122 20 V56"\/>/);
});

test('a decision is a diamond with its question inside', () => {
  const markup = drawn({ id: 'd', shape: 'decision', label: 'Cached?', x: 10, y: 20, width: 120, height: 48 });
  assert.match(markup, /<path class="body" d="M70 20 L130 44 L70 68 L10 44 Z"\/><text class="message" x="70" y="47.5" text-anchor="middle">Cached\?<\/text>/);
});

test('wrapped text is centred on the middle of its shape', () => {
  const size = SHAPES.process.sizeOf({ label: 'Read the manifest from the cache root and check every digest it names' });
  const markup = drawn({ id: 'p', shape: 'process', label: 'Read the manifest from the cache root and check every digest it names', x: 0, y: 0, ...size });
  const lines = (markup.match(/<tspan/g) || []).length;
  assert.ok(lines >= 2);
  const first = Number(/<text class="message" x="[\d.]+" y="([\d.]+)"/.exec(markup)[1]);
  assert.ok(Math.abs((first - 11 + (lines * 15) / 2) - size.height / 2) <= 1);
});
