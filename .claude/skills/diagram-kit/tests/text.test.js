const test = require('node:test');
const assert = require('node:assert/strict');

const { widthOf, wrapped, blockOf, ROLES } = require('../lib/text.js');
const { FONT_METRICS } = require('../lib/font-metrics.js');

const near = (actual, expected) => assert.ok(Math.abs(actual - expected) < 0.01, `${actual} is not ${expected}`);

test('a name is as wide as its characters in the monospaced face', () => {
  near(widthOf('start_up', 'name'), 8 * 0.6 * ROLES.name.size);
});

test('a detail is as wide as the sum of its characters in the proportional face', () => {
  const { usual, others } = FONT_METRICS['sans-400'];
  const em = c => others[c] ?? usual;
  near(widthOf('Wii', 'detail'), (em('W') + 2 * em('i')) * ROLES.detail.size);
});

test('a character the fonts do not cover takes the usual width of its face', () => {
  near(widthOf('→', 'name'), FONT_METRICS['mono-400'].usual * ROLES.name.size);
});

test('letter spacing widens every character of a heading', () => {
  const plain = { ...ROLES.heading, letterSpacing: 0 };
  near(widthOf('AB', 'heading') - widthOf('AB', plain), 2 * ROLES.heading.letterSpacing * ROLES.heading.size);
});

test('text that fits is one line', () => {
  assert.deepEqual(wrapped('identify, claim', 'edge', 400), ['identify, claim']);
});

test('text wider than its limit breaks between words', () => {
  const limit = widthOf('identify,', 'edge') + 1;
  assert.deepEqual(wrapped('identify, claim', 'edge', limit), ['identify,', 'claim']);
});

test('a word wider than the limit keeps a line to itself', () => {
  assert.deepEqual(wrapped('a version_claims b', 'name', 30), ['a', 'version_claims', 'b']);
});

test('no text is one empty line', () => {
  assert.deepEqual(wrapped('', 'name', 100), ['']);
  assert.deepEqual(wrapped(undefined, 'name', 100), ['']);
});

test('a block is as wide as its widest line and as tall as its lines', () => {
  const block = blockOf('identify, claim', 'edge', widthOf('identify,', 'edge') + 1);
  assert.deepEqual(block.lines, ['identify,', 'claim']);
  near(block.width, widthOf('identify,', 'edge'));
  assert.equal(block.height, 2 * ROLES.edge.lineHeight);
  assert.equal(block.role, 'edge');
});

test('an unknown role is refused', () => {
  assert.throws(() => widthOf('x', 'banner'), /no text role "banner"/);
});
