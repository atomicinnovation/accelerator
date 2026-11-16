const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const notation = require('../notation.js');
const { kitWith } = require('../../diagram-kit/lib/index.js');
const { widthOf } = require('../../diagram-kit/lib/text.js');

const SPEC = {
  id: 'header', kind: 'byte-field-diagram', title: 'The length comes before anything that depends on it',
  fields: [
    { name: 'version', size: 4, status: 'existing' },
    { name: 'flags', size: 4, link: 'src/frame.rs:12' },
    { name: 'kind', size: 8, tone: 'risk' },
    { name: 'length', size: 16, status: 'new' },
    { name: 'sequence', size: 48 },
    { name: 'reserved', size: 16, reserved: true },
    { name: 'payload', size: 'variable' },
    { name: 'checksum', size: 32 },
  ],
};
const scene = notation.placedSceneOf(SPEC);
const fields = scene.nodes.filter(each => each.shape === 'field');
const field = id => fields.find(each => each.id === id);
const marks = prefix => scene.nodes.filter(each => each.shape === 'scale' && each.id.startsWith(prefix));
const problems = change => notation.problemsIn({ ...SPEC, ...change });
const UNIT = field('f1').width / 4, LEFT = field('f1').x, TOP = field('f1').y, ROW = field('f1').height;

test('it draws diagrams of kind byte-field-diagram', () => {
  assert.equal(notation.kind, 'byte-field-diagram');
});

test('a sound layout has no problems and nothing to remark on', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
  assert.deepEqual(notation.remarksOn(SPEC), []);
});

test('it places its own scene, with no arrows, and the kit draws that', () => {
  assert.deepEqual(scene.edges, []);
  assert.deepEqual(notation.sceneOf(SPEC), scene);
  assert.ok(scene.nodes.every(each => each.inert && each.x >= 0 && each.y >= 0 && each.x + each.width <= scene.width && each.y + each.height <= scene.height));
});

test('fields sit side by side, each as wide as its size, 32 bits to a row', () => {
  assert.deepEqual(['f1', 'f2', 'f3', 'f4'].map(id => [field(id).label, (field(id).x - LEFT) / UNIT, field(id).width / UNIT, field(id).y]),
    [['version', 0, 4, TOP], ['flags', 4, 4, TOP], ['kind', 8, 8, TOP], ['length', 16, 16, TOP]]);
  assert.deepEqual([field('f1').status, field('f2').link, field('f3').tone, field('f4').status], ['existing', 'src/frame.rs:12', 'risk', 'new']);
});

test('a field too long for its row carries on in the next', () => {
  assert.deepEqual([field('f5').width / UNIT, field('f5').y, field('f5').label], [32, TOP + ROW, 'sequence']);
  assert.deepEqual([field('f5/2').x, field('f5/2').width / UNIT, field('f5/2').y, field('f5/2').label], [LEFT, 16, TOP + 2 * ROW, 'sequence']);
});

test('a reserved field is greyed', () => {
  assert.equal(field('f6').muted, true);
  assert.equal(field('f1').muted, undefined);
});

test('a field of no fixed size takes a row of its own, broken, and says it varies', () => {
  assert.deepEqual([field('f7').x, field('f7').width / UNIT, field('f7').y, field('f7').broken, field('f7').detail], [LEFT, 32, TOP + 3 * ROW, true, 'variable']);
  assert.deepEqual([field('f8').x, field('f8').y], [LEFT, TOP + 4 * ROW]);
});

test('the scale along the top numbers every bit, and names the unit', () => {
  assert.deepEqual(marks('at:').map(each => each.label), Array.from({ length: 32 }, (_, bit) => String(bit)));
  assert.deepEqual([marks('at:')[5].x, marks('at:')[5].width], [LEFT + 5 * UNIT, UNIT]);
  assert.equal(scene.nodes.find(each => each.id === 'unit').label, 'bit');
});

test('each row says the offset it starts at, until a field of no fixed size makes that unknown', () => {
  assert.deepEqual(marks('row:').map(each => [each.label, each.y, each.align]), [['0', TOP, 'end'], ['32', TOP + ROW, 'end'], ['64', TOP + 2 * ROW, 'end'], ['96', TOP + 3 * ROW, 'end']]);
});

test('a field many rows long is drawn as one broken row that says its size, and the offsets carry on past it', () => {
  const long = notation.placedSceneOf({ ...SPEC, fields: [{ name: 'tag', size: 8 }, { name: 'key', size: 24 }, { name: 'body', size: 256 }, { name: 'end', size: 8 }] });
  const body = long.nodes.find(each => each.id === 'f3');

  assert.deepEqual([body.width / UNIT, body.broken, body.detail], [32, true, '256 bits']);
  assert.deepEqual(long.nodes.filter(each => each.id.startsWith('row:')).map(each => each.label), ['0', '32', '288']);
});

test('bytes are laid eight to a row unless the layout says otherwise', () => {
  const bytes = notation.placedSceneOf({ ...SPEC, unit: 'byte', fields: [{ name: 'magic', size: 4 }, { name: 'length', size: 4 }, { name: 'digest', size: 32 }] });
  const digest = bytes.nodes.find(each => each.id === 'f3');

  assert.equal(bytes.nodes.filter(each => each.id.startsWith('at:')).length, 8);
  assert.equal(bytes.nodes.find(each => each.id === 'unit').label, 'byte');
  assert.deepEqual([digest.broken, digest.detail], [true, '32 bytes']);
  assert.equal(notation.placedSceneOf({ ...SPEC, perRow: 16 }).nodes.filter(each => each.id.startsWith('at:')).length, 16);
});

test('cells widen, up to a limit, so that names fit', () => {
  const wide = notation.placedSceneOf({ ...SPEC, fields: [{ name: 'acknowledged', size: 4 }, { name: 'rest', size: 28 }] });
  const cell = wide.nodes.find(each => each.id === 'f1');

  assert.ok(cell.width / 4 > UNIT);
  assert.ok(cell.width >= widthOf('acknowledged', 'name') + 10);
});

test('a name that still does not fit is replaced by a letter the scene spells out', () => {
  const flags = notation.placedSceneOf({ ...SPEC, fields: [{ name: 'urgent', size: 1 }, { name: 'ack', size: 1 }, { name: 'window', size: 30 }] });
  const label = id => flags.nodes.find(each => each.id === id).label;

  assert.deepEqual([label('f1'), label('f2'), label('f3')], ['a', 'b', 'window']);
  assert.deepEqual(flags.key, [['a', 'urgent'], ['b', 'ack']]);
  assert.equal(scene.key, undefined);
});

test('a name is left out of a part of its field too narrow for it, when another part shows it', () => {
  const split = notation.placedSceneOf({ ...SPEC, fields: [{ name: 'tag', size: 30 }, { name: 'sequence number', size: 34 }] });
  const label = id => split.nodes.find(each => each.id === id).label;

  assert.deepEqual([label('f2'), label('f2/2')], ['', 'sequence number']);
});

test('a field may be given an id of its own', () => {
  assert.ok(notation.placedSceneOf({ ...SPEC, fields: [{ id: 'len', name: 'length', size: 32 }] }).nodes.some(each => each.id === 'len'));
});

test('a layout needs fields, a unit it knows and a sensible row', () => {
  assert.deepEqual(problems({ fields: [] }), ['no fields']);
  assert.deepEqual(problems({ unit: 'nibble', perRow: 0 }), ['unit "nibble" is not one of bit, byte', 'perRow must be a whole number from 1 to 64']);
});

test('a field needs a name, a size and an id of its own', () => {
  assert.deepEqual(problems({ fields: [{ size: 4 }, { name: 'a', size: 0 }, { name: 'b', size: 'lots' }, { name: 'c' }, { id: 'x', name: 'd', size: 1 }, { id: 'x', name: 'e', size: 1 },
    { name: 'f', size: 1, reserved: 'yes', status: 'fresh' }] }), [
    'field 1: no name',
    'field a: size must be a whole number of bits, or "variable"',
    'field b: size must be a whole number of bits, or "variable"',
    'field c: size must be a whole number of bits, or "variable"',
    'field e: id x is used twice',
    'field f: reserved must be true or false',
    'field f: status "fresh" is not one of new, changed, removed, existing',
  ]);
});

test('bits that do not make whole bytes are remarked on', () => {
  assert.deepEqual(notation.remarksOn({ ...SPEC, fields: [{ name: 'a', size: 3 }, { name: 'b', size: 8 }] }), ['The fields total 11 bits, which is not a whole number of bytes']);
  assert.deepEqual(notation.remarksOn({ ...SPEC, unit: 'byte', fields: [{ name: 'a', size: 3 }] }), []);
});

const EXAMPLES = path.join(__dirname, '..', 'examples');
for (const file of fs.readdirSync(EXAMPLES).filter(name => name.endsWith('.json'))) {
  test(`the example ${file} is sound and lays out`, async () => {
    const kit = kitWith(), example = JSON.parse(fs.readFileSync(path.join(EXAMPLES, file), 'utf8'));
    assert.deepEqual(kit.problemsIn(example), []);
    assert.deepEqual(kit.remarksOn(example), []);
    assert.ok((await kit.laidOut(example)).nodes.length > 0);
  });
}
