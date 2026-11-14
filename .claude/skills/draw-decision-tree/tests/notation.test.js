const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const notation = require('../notation.js');
const { kitWith } = require('../../diagram-kit/lib/index.js');

const SPEC = {
  id: 'keep-or-remove', kind: 'decision-tree', title: 'Only an old, unclaimed tree is removed',
  root: {
    question: 'Is the tree younger than the age backstop?', link: 'src/prune.rs:40',
    branches: [
      { answer: 'yes', then: { outcome: 'Keep', tone: 'ok' } },
      { answer: 'no', status: 'new', then: {
        id: 'claims', question: 'Does a claim inside the window name its version?', status: 'new',
        branches: [
          { answer: 'yes', then: { outcome: 'Keep', tone: 'ok', link: 'src/prune.rs:71' } },
          { answer: 'no', tone: 'risk', link: 'src/prune.rs:80', then: { id: 'gone', outcome: 'Remove', tone: 'risk', status: 'new' } },
        ],
      } },
    ],
  },
};
const scene = notation.sceneOf(SPEC);
const node = id => scene.nodes.find(each => each.id === id);
const problems = root => notation.problemsIn({ ...SPEC, root });
const remarks = root => notation.remarksOn({ ...SPEC, root });
const leaf = { outcome: 'Done' };
const ask = (question, ...branches) => ({ question, branches: branches.map(([answer, then]) => ({ answer, then })) });

test('it draws diagrams of kind decision-tree', () => {
  assert.equal(notation.kind, 'decision-tree');
});

test('a sound tree has no problems and nothing to remark on', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
  assert.deepEqual(notation.remarksOn(SPEC), []);
});

test('a question is a box and an outcome a rounded one, each saying what it is', () => {
  assert.deepEqual(scene.nodes.map(each => [each.id, each.shape, each.label]), [
    ['q', 'process', 'Is the tree younger than the age backstop?'],
    ['q.1', 'terminator', 'Keep'],
    ['claims', 'process', 'Does a claim inside the window name its version?'],
    ['claims.1', 'terminator', 'Keep'],
    ['gone', 'terminator', 'Remove'],
  ]);
});

test('questions and outcomes keep their marks and links', () => {
  assert.deepEqual([node('q').link, node('claims').status, node('q.1').tone, node('gone').tone, node('gone').status, node('claims.1').link], ['src/prune.rs:40', 'new', 'ok', 'risk', 'new', 'src/prune.rs:71']);
});

test('each answer is an arrow from its question to what follows, labelled with the answer', () => {
  assert.deepEqual(scene.edges.map(each => [each.id, each.from, each.to, each.label, each.line, each.head]), [
    ['q->q.1', 'q', 'q.1', 'yes', 'solid', 'open'],
    ['q->claims', 'q', 'claims', 'no', 'solid', 'open'],
    ['claims->claims.1', 'claims', 'claims.1', 'yes', 'solid', 'open'],
    ['claims->gone', 'claims', 'gone', 'no', 'solid', 'open'],
  ]);
});

test('an answer keeps its marks and link', () => {
  const [, taken, , fatal] = scene.edges;
  assert.equal(taken.status, 'new');
  assert.deepEqual([fatal.tone, fatal.link], ['risk', 'src/prune.rs:80']);
});

test('the tree runs down the page unless it says right', () => {
  assert.equal(scene.direction, 'down');
  assert.equal(notation.sceneOf({ ...SPEC, direction: 'right' }).direction, 'right');
});

test('a tree needs a root that is a question or an outcome, not both and not neither', () => {
  assert.deepEqual(notation.problemsIn({ ...SPEC, root: undefined }), ['no root']);
  assert.deepEqual(problems({ note: 'x' }), ['the root: is neither a question nor an outcome']);
  assert.deepEqual(problems({ question: 'Q?', outcome: 'O', branches: [] }), ['the root: is both a question and an outcome']);
});

test('a question needs answers, each with an answer and something that follows', () => {
  assert.deepEqual(problems({ question: 'Q?' }), ['the root: a question needs branches']);
  assert.deepEqual(problems({ question: 'Q?', branches: [{ then: leaf }, { answer: 'no' }, 'maybe', { answer: 'yes', then: leaf }] }), [
    'the root, branch 1: no answer',
    'the root, branch 2: nothing follows it',
    'the root, branch 3: is not an answer with what follows',
  ]);
});

test('problems further down are named by the answers that lead to them', () => {
  assert.deepEqual(problems(ask('Q?', ['yes', leaf], ['no', ask('R?', ['a', { outcome: 7 }], ['b', { question: 'S?', branches: [] }])])), [
    'after "no", "a": outcome must be text',
    'after "no", "b": a question needs branches',
  ]);
});

test('an id may be given once, and marks must be ones the notation has', () => {
  assert.deepEqual(problems(ask('Q?', ['yes', { id: 'x', outcome: 'A', status: 'fresh' }], ['no', { id: 'x', outcome: 'B' }])), [
    'after "yes": status "fresh" is not one of new, changed, removed, existing',
    'after "no": id "x" is used twice',
  ]);
});

test('direction outside what it may be is reported', () => {
  assert.deepEqual(notation.problemsIn({ ...SPEC, direction: 'up' }), ['direction "up" is not one of down, right']);
});

test('a question with one answer is remarked on', () => {
  assert.deepEqual(remarks(ask('Is it cached?', ['yes', leaf])), ['"Is it cached?" has only one answer, so it decides nothing']);
});

test('a question with the same answer twice is remarked on', () => {
  assert.deepEqual(remarks(ask('Is it cached?', ['yes', leaf], ['yes', { outcome: 'Wait' }], ['no', { outcome: 'Fetch' }])), ['"Is it cached?" has the answer yes twice']);
});

test('a question whose every answer leads to the same outcome is remarked on', () => {
  assert.deepEqual(remarks(ask('Is it cached?', ['yes', { outcome: 'Keep' }], ['no', { outcome: 'Keep' }])), ['Every answer to "Is it cached?" leads to Keep, so the question changes nothing']);
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
