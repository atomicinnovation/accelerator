const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const notation = require('../notation.js');
const { kitWith } = require('../../diagram-kit/lib/index.js');
const { SHAPES } = require('../../diagram-kit/lib/shapes.js');

const drawing = rules => notation.placedSceneOf({ id: 'g', kind: 'railroad-diagram', title: 'T', rules });
const one = is => drawing([{ name: 'r', is }]);
const part = (scene, id) => scene.nodes.find(each => each.id === id);
const linesOf = scene => part(scene, 'r/rails').lines;
const runsAlong = (scene, y, from, to) => linesOf(scene).some(line => line.some((point, i) => i > 0 && point[1] === y && line[i - 1][1] === y
  && Math.min(point[0], line[i - 1][0]) <= from && Math.max(point[0], line[i - 1][0]) >= to));
const problems = rules => notation.problemsIn({ id: 'g', kind: 'railroad-diagram', title: 'T', rules });
const middle = node => node.y + node.height / 2;

test('it draws diagrams of kind railroad-diagram', () => {
  assert.equal(notation.kind, 'railroad-diagram');
});

test('it places its own scene, with no arrows, inside its bounds', () => {
  const scene = one(['a', { rule: 'b' }]);

  assert.deepEqual(scene.edges, []);
  assert.deepEqual(notation.sceneOf({ id: 'g', kind: 'railroad-diagram', title: 'T', rules: [{ name: 'r', is: ['a'] }] }), one(['a']));
  assert.ok(scene.nodes.every(each => each.inert && each.x >= 0 && each.y >= 0 && each.x + each.width <= scene.width && each.y + each.height <= scene.height));
});

test('a rule is its name above a track that begins and ends in a pair of bars', () => {
  const scene = one('a'), name = part(scene, 'r'), token = part(scene, 'r/1'), rails = part(scene, 'r/rails');
  const upright = linesOf(scene).filter(line => line.length === 2 && line[0][0] === line[1][0]);

  assert.deepEqual([name.shape, name.label, rails.shape, rails.behind], ['rulename', 'r', 'rails', true]);
  assert.ok(name.y + name.height <= token.y);
  assert.equal(upright.length, 4);
  assert.ok(upright.every(line => (line[0][1] + line[1][1]) / 2 === middle(token)));
});

test('text is a terminal, drawn as a pill, and a reference to a rule is a rectangle', () => {
  const scene = one([{ terminal: 'v', tone: 'risk' }, { rule: 'number', status: 'new', link: 'src/grammar.rs:4' }]);

  assert.deepEqual([part(scene, 'r/1').shape, part(scene, 'r/1').label, part(scene, 'r/1').tone], ['token', 'v', 'risk']);
  assert.deepEqual([part(scene, 'r/2').shape, part(scene, 'r/2').label, part(scene, 'r/2').status, part(scene, 'r/2').link], ['ruleref', 'number', 'new', 'src/grammar.rs:4']);
  assert.deepEqual([part(scene, 'r/1').width, part(scene, 'r/1').height], Object.values(SHAPES.token.sizeOf({ label: 'v' })));
});

test('the parts of a sequence follow one another along one line, joined by rail', () => {
  const scene = one(['a', 'b', 'c']), [a, b, c] = ['r/1', 'r/2', 'r/3'].map(id => part(scene, id));

  assert.ok(a.x + a.width < b.x && b.x + b.width < c.x);
  assert.deepEqual([middle(b), middle(c)], [middle(a), middle(a)]);
  assert.ok(runsAlong(scene, middle(a), a.x + a.width, b.x));
  assert.ok(runsAlong(scene, middle(a), b.x + b.width, c.x));
});

test('the options of a choice are stacked, the first on the line, each joined to both ends', () => {
  const scene = one({ choice: ['a', 'bbb', 'c'] }), [a, b, c] = ['r/1', 'r/2', 'r/3'].map(id => part(scene, id));

  assert.deepEqual([b.x, c.x], [a.x, a.x]);
  assert.ok(a.y + a.height < b.y && b.y + b.height < c.y);
  for (const option of [b, c]) {
    assert.ok(linesOf(scene).some(line => line[0][1] === middle(a) && line[line.length - 1][1] === middle(option) && line[line.length - 1][0] === option.x));
    assert.ok(linesOf(scene).some(line => line[0][1] === middle(option) && line[0][0] === option.x + option.width && line[line.length - 1][1] === middle(a)));
  }
});

test('an optional part hangs below a line that passes it by', () => {
  const scene = one(['a', { optional: 'b' }, 'c']), [a, b, c] = ['r/1', 'r/2', 'r/3'].map(id => part(scene, id));

  assert.ok(middle(b) > middle(a));
  assert.equal(middle(c), middle(a));
  assert.ok(runsAlong(scene, middle(a), b.x, b.x + b.width));
});

test('a part that repeats has a rail that returns beneath it, through its separator if it has one', () => {
  const scene = one({ oneOrMore: { rule: 'digit' }, separator: ',' }), digit = part(scene, 'r/1'), comma = part(scene, 'r/2');
  const bare = one({ oneOrMore: 'a' }), a = part(bare, 'r/1');

  assert.ok(comma.y > digit.y + digit.height);
  assert.ok(linesOf(scene).some(line => line[0][1] === middle(digit) && line[0][0] >= digit.x + digit.width && line[line.length - 1][1] === middle(comma)));
  assert.ok(linesOf(bare).some(line => line[0][0] >= a.x + a.width && line[line.length - 1][0] < a.x + 1 && line.some(point => point[1] > a.y + a.height)));
});

test('a part that may repeat or be absent is a repeat that can be passed by', () => {
  const scene = one(['a', { zeroOrMore: 'b' }]), [a, b] = ['r/1', 'r/2'].map(id => part(scene, id));

  assert.ok(middle(b) > middle(a));
  assert.ok(runsAlong(scene, middle(a), b.x, b.x + b.width));
  assert.ok(linesOf(scene).some(line => line.some(point => point[1] > b.y + b.height)));
});

test('rules are drawn one under another, and the drawing is as wide as the widest', () => {
  const scene = drawing([{ name: 'short', is: 'a' }, { name: 'long', is: ['a', 'b', 'c', 'd'] }]);

  assert.ok(part(scene, 'short/rails').y + part(scene, 'short/rails').height < part(scene, 'long').y);
  assert.ok(scene.width >= part(scene, 'long/rails').x + part(scene, 'long/rails').width);
});

test('a diagram needs rules, each with a name of its own and something it is', () => {
  assert.deepEqual(problems([]), ['no rules']);
  assert.deepEqual(problems([{ is: 'a' }, { name: 'r' }, { name: 's', is: 'a' }, { name: 's', is: 'a' }]), ['rule 1: no name', 'rule r: does not say what it is', 'rule s: name is used twice']);
});

test('each part of a rule is something the notation draws, well formed', () => {
  assert.deepEqual(problems([{ name: 'r', is: [
    '', [], { choice: ['a'] }, { loop: 'a' }, { terminal: 'a', rule: 'b' }, { terminal: 4 }, { optional: null }, { oneOrMore: 'a', separator: 7 }, { rule: 'x', status: 'fresh' }, { choice: ['a', { sequence: [] }] },
  ] }]), [
    'rule r: a terminal has no text',
    'rule r: a sequence is empty',
    'rule r: a choice needs at least two options',
    'rule r: {"loop":"a"} is not a terminal, rule, sequence, choice, optional, oneOrMore or zeroOrMore',
    'rule r: {"terminal":"a","rule":"b"} is more than one thing',
    'rule r: a terminal has no text',
    'rule r: an optional part is missing',
    'rule r: 7 is not a terminal, rule, sequence, choice, optional, oneOrMore or zeroOrMore',
    'rule r: rule x: status "fresh" is not one of new, changed, removed, existing',
    'rule r: a sequence is empty',
  ]);
});

test('a rule nothing uses is remarked on, the first rule apart', () => {
  const rules = [{ name: 'version', is: [{ rule: 'number' }] }, { name: 'number', is: 'n' }, { name: 'label', is: 'l' }];

  assert.deepEqual(notation.remarksOn({ rules }), ['label is defined, and no other rule uses it']);
  assert.deepEqual(notation.remarksOn({ rules: rules.slice(0, 2) }), []);
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
