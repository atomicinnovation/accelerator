const test = require('node:test');
const assert = require('node:assert/strict');

const { svgOf } = require('../lib/render.js');
const { SHAPES } = require('../lib/shapes.js');
const { widthOf } = require('../lib/text.js');

const groupHolding = (markup, attribute) => markup.slice(markup.lastIndexOf('<g ', markup.indexOf(attribute))).split('</g>')[0];
const drawn = node => groupHolding(svgOf({ width: 400, height: 200, nodes: [node], edges: [] }, { uid: 'd' }), `data-node="${node.id}"`);

test('a bubble is a circle wide enough to hold its words inside', () => {
  const size = SHAPES.bubble.sizeOf({ label: 'Verify the signature' });
  const words = SHAPES.bubble.textOf({ label: 'Verify the signature' });

  assert.equal(size.width, size.height);
  assert.ok(size.width >= Math.hypot(words.width, words.height) + 12);
  assert.equal(SHAPES.bubble.outline, 'disc');
  assert.deepEqual(SHAPES.bubble.sizeOf({ label: 'Go' }), { width: 64, height: 64 });
});

test('a bubble wraps its words rather than growing without limit', () => {
  const size = SHAPES.bubble.sizeOf({ label: 'Resolve the version the manifest pins and fetch it' });

  assert.ok(SHAPES.bubble.textOf({ label: 'Resolve the version the manifest pins and fetch it' }).lines.length > 2);
  assert.ok(size.width <= 150);
});

test('a bubble is drawn as a circle with its words centred in it', () => {
  const markup = drawn({ id: 'p', shape: 'bubble', label: 'Go', status: 'new', x: 10, y: 20, width: 64, height: 64 });

  assert.match(markup, /^<g class="node bubble leaf s-new"/);
  assert.match(markup, /<circle class="body" cx="42" cy="52" r="32"\/>/);
  assert.match(markup, /<text class="message" x="42" y="55.5" text-anchor="middle">Go<\/text>/);
});

test('a store is sized for its name', () => {
  assert.deepEqual(SHAPES.store.sizeOf({ label: 'Log' }), { width: 96, height: 36 });
  assert.ok(SHAPES.store.sizeOf({ label: 'Release manifest cache' }).width >= widthOf('Release manifest cache', 'message') + 28);
});

test('a store is two parallel lines, open at both ends, with its name between them', () => {
  const markup = drawn({ id: 's', shape: 'store', label: 'Log', tone: 'risk', x: 10, y: 20, width: 96, height: 36 });

  assert.match(markup, /^<g class="node store leaf t-risk"/);
  assert.match(markup, /<rect class="back" x="10" y="20" width="96" height="36"\/>/);
  assert.match(markup, /<path class="body" d="M10 20 H106 M10 56 H106"\/>/);
  assert.match(markup, /<text class="message" x="58" y="41.5" text-anchor="middle">Log<\/text>/);
});

test('an external entity is a plain rectangle sized for its name', () => {
  assert.deepEqual(SHAPES.external.sizeOf({ label: 'User' }), { width: 96, height: 36 });
  const markup = drawn({ id: 'e', shape: 'external', label: 'User', x: 10, y: 20, width: 96, height: 36 });

  assert.match(markup, /^<g class="node external leaf"/);
  assert.match(markup, /<rect class="body" x="10" y="20" width="96" height="36" rx="0"\/>/);
});

test('a linked data-flow shape opens its link from its words', () => {
  const markup = drawn({ id: 'p', shape: 'bubble', label: 'Go', link: 'https://example.test/go', x: 0, y: 0, width: 64, height: 64 });

  assert.match(markup, /<a href="https:\/\/example.test\/go"[^>]*><text class="message"/);
});
