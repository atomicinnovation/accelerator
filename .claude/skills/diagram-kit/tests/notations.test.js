const test = require('node:test');
const assert = require('node:assert/strict');

const { notations, markProblems } = require('../lib/notations.js');

const BOXES = {
  kind: 'boxes',
  problemsIn: spec => (spec.boxes || []).flatMap(box => markProblems(box, `box ${box.id}`)),
  sceneOf: spec => ({ nodes: spec.boxes.map(box => ({ ...box, shape: 'box' })), edges: [{ id: 'a->b', from: 'a', to: 'b', head: 'open' }] }),
};
const SPEC = { id: 'd1', kind: 'boxes', title: 'Two boxes', caption: 'What to notice.', boxes: [{ id: 'a', label: 'a', status: 'new' }, { id: 'b', label: 'b' }] };
const kit = () => notations([BOXES]);

test('a sound diagram has no problems', () => {
  assert.deepEqual(kit().problemsIn(SPEC), []);
});

test('a diagram needs an id, a kind and a title', () => {
  assert.deepEqual(kit().problemsIn({ kind: 'boxes', title: 't', boxes: [] }), ['diagram ?: no id']);
  assert.deepEqual(kit().problemsIn({ id: 'd1', title: 't' }), ['diagram d1: no kind']);
  assert.deepEqual(kit().problemsIn({ ...SPEC, title: ' ' }), ['diagram d1: no title; say the point the diagram makes']);
});

test('a diagram that is not an object is one problem', () => {
  assert.deepEqual(kit().problemsIn('boxes'), ['diagram ?: must be an object']);
});

test('a kind no notation draws is refused, naming the kinds that are drawn', () => {
  assert.deepEqual(kit().problemsIn({ ...SPEC, kind: 'blobs' }), ['diagram d1: no notation draws kind "blobs"; known kinds: boxes']);
});

test('a notation\'s own problems are reported under the diagram', () => {
  const marked = { ...SPEC, boxes: [{ id: 'a', status: 'fresh', tone: 'scary', link: 7 }] };
  assert.deepEqual(kit().problemsIn(marked), [
    'diagram d1: box a: status "fresh" is not one of new, changed, removed, existing',
    'diagram d1: box a: tone "scary" is not one of risk, ok',
    'diagram d1: box a: link must be text',
  ]);
});

test('a notation that throws on a malformed diagram is a problem, not a crash', () => {
  const problems = kit().problemsIn({ id: 'd1', kind: 'boxes', title: 't', boxes: 3 });
  assert.equal(problems.length, 1);
  assert.match(problems[0], /^diagram d1: could not be checked: .*flatMap is not a function$/);
});

test('a diagram is laid out into a scene the renderer can draw', async () => {
  const scene = await kit().laidOut(SPEC);
  assert.deepEqual(scene.nodes.map(node => node.id), ['a', 'b']);
  assert.ok(scene.width > 0 && scene.edges[0].points.length >= 2);
});

test('a diagram with problems is not laid out', async () => {
  await assert.rejects(kit().laidOut({ ...SPEC, title: '' }), /diagram d1: no title/);
});

test('a figure carries the title, the drawing, its legend and the caption', async () => {
  const figure = kit().figureOf(SPEC, await kit().laidOut(SPEC));
  assert.match(figure, /^<figure class="dk-figure" id="diagram-d1"><figcaption class="dk-title">Two boxes<\/figcaption>/);
  assert.match(figure, /<svg class="dk" data-graph="d1"[^>]*aria-label="Two boxes"/);
  assert.match(figure, /new in this change/);
  assert.match(figure, /<p class="dk-caption">What to notice\.<\/p><\/figure>$/);
});

test('a figure with edges says how to trace them', async () => {
  assert.match(kit().figureOf(SPEC, await kit().laidOut(SPEC)), /Press a box to trace its arrows/);
});

test('two notations may not draw the same kind', () => {
  assert.throws(() => notations([BOXES, BOXES]), /two notations draw kind "boxes"/);
});

test('a notation that lays itself out is drawn where it says, without the layout engine', async () => {
  const placed = { width: 40, height: 20, nodes: [{ id: 'a', shape: 'box', label: 'a', x: 0, y: 0, width: 40, height: 20 }], edges: [] };
  const rows = { kind: 'rows', problemsIn: () => [], placedSceneOf: spec => ({ ...placed, width: spec.width }) };
  const scene = await notations([BOXES, rows]).laidOut({ id: 'r1', kind: 'rows', title: 't', width: 99 });
  assert.deepEqual(scene, { ...placed, width: 99 });
});
