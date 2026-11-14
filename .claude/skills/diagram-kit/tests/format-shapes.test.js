const test = require('node:test');
const assert = require('node:assert/strict');

const { svgOf } = require('../lib/render.js');

const groupHolding = (markup, attribute) => markup.slice(markup.lastIndexOf('<g ', markup.indexOf(attribute))).split('</g>')[0];
const drawn = node => groupHolding(svgOf({ width: 400, height: 200, nodes: [node], edges: [] }, { uid: 'f' }), `data-part="${node.id}"`);

test('a field is a square-cornered cell with its name in the middle', () => {
  const markup = drawn({ id: 'f1', shape: 'field', inert: true, label: 'length', status: 'new', x: 10, y: 20, width: 120, height: 38 });

  assert.match(markup, /^<g class="node field s-new" data-part="f1">/);
  assert.match(markup, /<rect class="body" x="10" y="20" width="120" height="38" rx="0"\/>/);
  assert.match(markup, /<text class="name" x="70" y="43" text-anchor="middle">length<\/text>/);
  assert.doesNotMatch(markup, /class="break"/);
});

test('a field says what it is told to under its name', () => {
  const markup = drawn({ id: 'f1', shape: 'field', inert: true, label: 'payload', detail: '1024 bytes', x: 10, y: 20, width: 120, height: 38 });

  assert.match(markup, /<text class="name" x="70" y="36.5" text-anchor="middle">payload<\/text>/);
  assert.match(markup, /<text class="detail" x="70" y="50.5" text-anchor="middle">1024 bytes<\/text>/);
});

test('a field drawn shorter than it is has a break in each side', () => {
  const markup = drawn({ id: 'f1', shape: 'field', inert: true, label: 'payload', broken: true, x: 10, y: 20, width: 120, height: 38 });

  assert.match(markup, /<path class="break" d="M10 31 l-4 4 l8 8 l-4 4 M130 31 l-4 4 l8 8 l-4 4"\/>/);
});

test('a field with no name is an empty cell', () => {
  assert.doesNotMatch(drawn({ id: 'f1', shape: 'field', inert: true, label: '', x: 10, y: 20, width: 22, height: 38 }), /<text/);
});

test('a linked field opens its link from its name', () => {
  const markup = drawn({ id: 'f1', shape: 'field', inert: true, label: 'length', link: 'https://example.test/length', x: 10, y: 20, width: 120, height: 38 });

  assert.match(markup, /<a href="https:\/\/example.test\/length"[^>]*><text class="name"/);
});

test('a scale mark is a number with nothing drawn round it, centred or against its right edge', () => {
  const centred = drawn({ id: 'bit:7', shape: 'scale', inert: true, label: '7', x: 10, y: 0, width: 22, height: 14 });
  const ended = drawn({ id: 'row:32', shape: 'scale', inert: true, label: '32', align: 'end', x: 0, y: 20, width: 30, height: 38 });

  assert.match(centred, /^<g class="node scale" data-part="bit:7">/);
  assert.match(centred, /<rect class="body" x="10" y="0" width="22" height="14"\/><text class="key" x="21" y="10.5" text-anchor="middle">7<\/text>/);
  assert.match(ended, /<text class="key" x="30" y="42.5" text-anchor="end">32<\/text>/);
});
