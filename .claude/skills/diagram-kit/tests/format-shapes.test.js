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

const { SHAPES } = require('../lib/shapes.js');
const { widthOf } = require('../lib/text.js');

test('a token and a reference to a rule are as wide as their words and one height', () => {
  assert.deepEqual(SHAPES.token.sizeOf({ label: 'v' }), { width: Math.ceil(widthOf('v', 'name')) + 20, height: 26 });
  assert.deepEqual(SHAPES.ruleref.sizeOf({ label: 'number' }), { width: Math.ceil(widthOf('number', 'name')) + 20, height: 26 });
});

test('a token is a pill and a reference to a rule a rectangle, each with its words in the middle', () => {
  const token = drawn({ id: 't', shape: 'token', inert: true, label: 'v', tone: 'risk', x: 10, y: 20, width: 28, height: 26 });
  const reference = drawn({ id: 'r', shape: 'ruleref', inert: true, label: 'number', link: 'https://example.test/number', x: 10, y: 20, width: 64, height: 26 });

  assert.match(token, /^<g class="node token t-risk" data-part="t">/);
  assert.match(token, /<rect class="body" x="10" y="20" width="28" height="26" rx="13"\/><text class="name" x="24" y="37" text-anchor="middle">v<\/text>/);
  assert.match(reference, /<rect class="body" x="10" y="20" width="64" height="26" rx="0"\/><a href="https:\/\/example.test\/number"[^>]*><text class="name" x="42" y="37" text-anchor="middle">number<\/text><\/a>/);
});

test('rails are drawn as one path of lines that turn in rounded corners', () => {
  const rails = drawn({ id: 'rails', shape: 'rails', inert: true, behind: true, lines: [[[0, 10], [20, 10]], [[20, 10], [30, 10], [30, 40], [40, 40]]], x: 0, y: 0, width: 40, height: 40 });

  assert.match(rails, /<path class="body" d="M0 10 L20 10 M20 10 L25 10 Q30 10 30 15 L30 35 Q30 40 35 40 L40 40"\/>/);
});

test('the name of a rule is written from its left edge', () => {
  const name = drawn({ id: 'version', shape: 'rulename', inert: true, label: 'version', x: 0, y: 4, width: 60, height: 16 });

  assert.match(name, /<rect class="body" x="0" y="4" width="60" height="16"\/><text class="title" x="0" y="16" text-anchor="start">version<\/text>/);
});
