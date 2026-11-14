const test = require('node:test');
const assert = require('node:assert/strict');

const { svgOf, crowAt } = require('../lib/render.js');
const { SHAPES } = require('../lib/shapes.js');
const { widthOf } = require('../lib/text.js');

const groupHolding = (markup, attribute) => markup.slice(markup.lastIndexOf('<g ', markup.indexOf(attribute))).split('</g>')[0];
const ROWS = [{ key: 'PK', name: 'id', type: 'uuid' }, { key: 'FK', name: 'install_id', type: 'uuid', status: 'new' }, { name: 'refreshed_at', type: 'timestamp ?' }];

test('an entity is sized for its name and for its widest key, column and type together', () => {
  assert.deepEqual(SHAPES.entity.sizeOf({ label: 'claim' }), { width: 96, height: 30 });
  const size = SHAPES.entity.sizeOf({ label: 'claim', rows: ROWS });
  assert.equal(size.height, 30 + 8 + 3 * 16);
  assert.ok(size.width >= 26 + widthOf('refreshed_at', 'name') + 16 + widthOf('timestamp ?', 'name') + 16);
  assert.ok(SHAPES.entity.sizeOf({ label: 'claim', rows: [{ name: 'id', type: 'uuid' }] }).width < SHAPES.entity.sizeOf({ label: 'claim', rows: [{ key: 'PK', name: 'id', type: 'uuid' }] }).width);
});

test('an entity is a box with its name over a rule and a row for each attribute', () => {
  const entity = groupHolding(svgOf({ width: 300, height: 200, nodes: [{ id: 'claim', shape: 'entity', label: 'claim', rows: ROWS, status: 'changed', x: 10, y: 20, width: 240, height: 86 }], edges: [] }, { uid: 'e' }), 'data-node="claim"');
  assert.match(entity, /^<g class="node entity leaf s-changed"/);
  assert.match(entity, /<rect class="body" x="10" y="20" width="240" height="86" rx="3"\/><text class="title" x="130" y="40" text-anchor="middle">claim<\/text><line class="rule" x1="10" y1="50" x2="250" y2="50"\/>/);
  assert.match(entity, /<text class="key" x="18" y="67" text-anchor="start">PK<\/text><text class="column" x="44" y="67" text-anchor="start">id<\/text><text class="type" x="242" y="67" text-anchor="end">uuid<\/text>/);
  assert.match(entity, /<text class="key s-new" x="18" y="83" text-anchor="start">FK<\/text><text class="column s-new" x="44" y="83" text-anchor="start">install_id<\/text><text class="type s-new" x="242" y="83" text-anchor="end">uuid<\/text>/);
  assert.match(entity, /<text class="column" x="44" y="99" text-anchor="start">refreshed_at<\/text><text class="type" x="242" y="99" text-anchor="end">timestamp \?<\/text>/);
});

test('exactly one is two bars across the line', () => {
  assert.equal(crowAt('one', [50, 100], [50, 40]), 'M56 92 L44 92 M56 88 L44 88');
});

test('zero or one is a ring beyond a bar', () => {
  assert.equal(crowAt('zero-or-one', [50, 100], [50, 40]), 'M56 93 L44 93 M46 84 a4 4 0 1 0 8 0 a4 4 0 1 0 -8 0');
});

test('one or many is a bar beyond a crow\'s foot', () => {
  assert.equal(crowAt('one-or-many', [50, 100], [50, 40]), 'M50 90 L56 100 M50 90 L44 100 M56 86 L44 86');
});

test('zero or many is a ring beyond a crow\'s foot', () => {
  assert.equal(crowAt('zero-or-many', [50, 100], [50, 40]), 'M50 90 L56 100 M50 90 L44 100 M46 84 a4 4 0 1 0 8 0 a4 4 0 1 0 -8 0');
});

test('a crow\'s foot turns with its line', () => {
  assert.equal(crowAt('one-or-many', [100, 50], [40, 50]), 'M90 50 L100 44 M90 50 L100 56 M86 44 L86 56');
});

test('an edge draws how many at each end, and its line runs all the way to both entities', () => {
  const edge = { id: 'r', from: 'a', to: 'b', tail: 'one', head: 'zero-or-many', line: 'dashed', points: [[50, 40], [50, 100]], labelBox: null };
  const drawn = groupHolding(svgOf({ width: 100, height: 140, nodes: [], edges: [edge] }, { uid: 'e' }), 'data-edge="r"');
  assert.match(drawn, /^<g class="edge dashed"/);
  assert.match(drawn, /<path class="line" d="M50 40 L50 100"\/>/);
  assert.match(drawn, /<path class="tail crow" d="M44 48 L56 48 M44 52 L56 52"\/><path class="head crow" d="M50 90 L56 100 M50 90 L44 100 M46 84 a4 4 0 1 0 8 0 a4 4 0 1 0 -8 0"\/>/);
});

test('the key column widens for an attribute with several keys, so its name is not written over', () => {
  const wide = SHAPES.entity.keyColumnOf({ rows: [{ key: 'PK, FK', name: 'id' }] });
  assert.ok(wide >= widthOf('PK, FK', 'key') + 10);
  assert.equal(SHAPES.entity.keyColumnOf({ rows: [{ key: 'PK', name: 'id' }] }), 26);
  assert.equal(SHAPES.entity.keyColumnOf({ rows: [{ name: 'id' }] }), 0);
});
