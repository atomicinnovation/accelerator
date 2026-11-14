const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const notation = require('../notation.js');
const { kitWith } = require('../../diagram-kit/lib/index.js');

const SPEC = {
  id: 'claim-name', kind: 'block-diagram', title: 'One claim per install, not per version',
  blocks: [
    { id: 'exe', name: 'current_exe()', detail: 'the launcher\'s own path', link: 'src/start_up.rs:12' },
    { id: 'identity', name: 'Identity', blocks: [
      { id: 'hash', name: 'hash', status: 'new' },
      { id: 'id', name: 'launcher id', detail: '16 hex digits', tone: 'risk' },
    ] },
    { id: 'claim', name: 'claim file', status: 'changed' },
  ],
  arrows: [
    { from: 'exe', to: 'hash', label: 'canonical path', link: 'src/start_up.rs:20' },
    { from: 'hash', to: 'id' },
    { from: 'id', to: 'claim', label: 'names', dashed: true, status: 'new', tone: 'risk' },
  ],
};
const scene = notation.sceneOf(SPEC);
const everyNode = nodes => nodes.flatMap(each => [each, ...everyNode(each.children || [])]);
const node = id => everyNode(scene.nodes).find(each => each.id === id);
const edge = id => scene.edges.find(each => each.id === id);
const problems = change => notation.problemsIn({ ...SPEC, ...change });

test('it draws diagrams of kind block-diagram', () => {
  assert.equal(notation.kind, 'block-diagram');
});

test('a sound diagram has no problems and nothing to remark on', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
  assert.deepEqual(notation.remarksOn(SPEC), []);
});

test('a block is a box with its name, what is said of it and its marks', () => {
  assert.deepEqual([node('exe').shape, node('exe').label, node('exe').detail, node('exe').link], ['box', 'current_exe()', 'the launcher\'s own path', 'src/start_up.rs:12']);
  assert.deepEqual([node('hash').status, node('id').tone], ['new', 'risk']);
  assert.equal(notation.sceneOf({ ...SPEC, blocks: [{ id: 'bare' }], arrows: [] }).nodes[0].label, 'bare');
});

test('a block that holds others is a group around them', () => {
  assert.deepEqual(scene.nodes.map(each => each.id), ['exe', 'identity', 'claim']);
  assert.deepEqual([node('identity').shape, node('identity').label, node('identity').children.map(each => each.id)], ['group', 'Identity', ['hash', 'id']]);
});

test('an arrow is a solid line with a filled head, or a dashed one, and may say what it carries', () => {
  assert.deepEqual([edge('exe->hash').from, edge('exe->hash').to, edge('exe->hash').label, edge('exe->hash').line, edge('exe->hash').head, edge('exe->hash').link],
    ['exe', 'hash', 'canonical path', 'solid', 'filled', 'src/start_up.rs:20']);
  assert.deepEqual([edge('hash->id').label, edge('hash->id').line], [undefined, 'solid']);
  assert.deepEqual([edge('id->claim').line, edge('id->claim').status, edge('id->claim').tone], ['dashed', 'new', 'risk']);
});

test('two arrows the same way between the same two blocks each keep an id of their own', () => {
  const twice = notation.sceneOf({ ...SPEC, arrows: [{ from: 'exe', to: 'hash', label: 'a' }, { from: 'exe', to: 'hash', label: 'b' }] });
  assert.deepEqual(twice.edges.map(each => each.id), ['exe->hash', 'exe->hash#2']);
});

test('it reads down the page unless told to read across', () => {
  assert.equal(scene.direction, 'down');
  assert.equal(notation.sceneOf({ ...SPEC, direction: 'right' }).direction, 'right');
});

test('a diagram needs blocks, each with an id of its own', () => {
  assert.deepEqual(problems({ blocks: [] }), ['no blocks']);
  assert.deepEqual(problems({ blocks: [{ name: 'x' }, { id: 'a', blocks: [{ id: 'a' }] }, { id: 'b', name: 4, detail: [], status: 'fresh' }], arrows: [], direction: 'up' }), [
    'direction "up" is not one of down, right',
    'block 1: no id',
    'block a: id is used twice',
    'block b: name must be text',
    'block b: detail must be text',
    'block b: status "fresh" is not one of new, changed, removed, existing',
  ]);
});

test('an arrow joins two blocks the diagram has, once', () => {
  assert.deepEqual(problems({ arrows: [{ from: 'exe' }, { from: 'exe', to: 'zz' }, { from: 'exe', to: 'exe' }, { from: 'exe', to: 'hash', label: 7, dashed: 'yes', tone: 'scary' }, { from: 'hash', to: 'id', label: 'x' }, { from: 'hash', to: 'id', label: 'x' }] }), [
    'arrow 1: no to',
    'arrow exe->zz: no block "zz"',
    'arrow exe->exe: an arrow does not return to where it began',
    'arrow exe->hash: label must be text',
    'arrow exe->hash: dashed must be true or false',
    'arrow exe->hash: tone "scary" is not one of risk, ok',
    'arrow hash->id: written twice',
  ]);
});

test('a block joined to nothing is remarked on, when the diagram has arrows', () => {
  assert.deepEqual(notation.remarksOn({ ...SPEC, blocks: [...SPEC.blocks, { id: 'lone', name: 'Clock' }] }), ['Clock is joined to nothing in the diagram']);
  assert.deepEqual(notation.remarksOn({ ...SPEC, arrows: [] }), []);
});

const EXAMPLES = path.join(__dirname, '..', 'examples');
for (const file of fs.readdirSync(EXAMPLES).filter(name => name.endsWith('.json'))) {
  test(`the example ${file} is sound and lays out`, async () => {
    const kit = kitWith(), example = JSON.parse(fs.readFileSync(path.join(EXAMPLES, file), 'utf8'));
    assert.deepEqual(kit.problemsIn(example), []);
    assert.deepEqual(kit.remarksOn(example), []);
    assert.equal((await kit.laidOut(example)).edges.every(each => each.points.length >= 2), true);
  });
}
