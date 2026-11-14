const test = require('node:test');
const assert = require('node:assert/strict');

const { svgOf } = require('../lib/render.js');
const { SHAPES } = require('../lib/shapes.js');
const { notations } = require('../lib/notations.js');

const groupHolding = (markup, attribute) => markup.slice(markup.lastIndexOf('<g ', markup.indexOf(attribute))).split('</g>')[0];

test('a bar lies across the flow, and stands upright when the flow runs sideways', () => {
  assert.deepEqual(SHAPES.bar.sizeOf({}), { width: 84, height: 6 });
  assert.deepEqual(SHAPES.bar.sizeOf({ upright: true }), { width: 6, height: 84 });
});

test('a bar is a solid stroke, announced by what it does', () => {
  const drawn = svgOf({ width: 100, height: 20, nodes: [{ id: 'fork', shape: 'bar', x: 8, y: 7, width: 84, height: 6 }], edges: [] }, { uid: 'a' });
  assert.match(groupHolding(drawn, 'data-node="fork"'), /^<g class="node bar leaf" data-node="fork" tabindex="0" role="button" aria-label="Trace the arrows of the bar"><rect class="body" x="8" y="7" width="84" height="6" rx="2"\/>/);
});

test('the end of one flow is a ring with a cross in it', () => {
  assert.deepEqual(SHAPES.flowfinal.sizeOf({}), { width: 20, height: 20 });
  assert.equal(SHAPES.flowfinal.outline, 'disc');
  const drawn = svgOf({ width: 40, height: 40, nodes: [{ id: 'stop', shape: 'flowfinal', x: 10, y: 10, width: 20, height: 20 }], edges: [] }, { uid: 'a' });
  assert.match(groupHolding(drawn, 'data-node="stop"'), /<circle class="body" cx="20" cy="20" r="10"\/><path class="cross" d="M13 13 L27 27 M27 13 L13 27"\/>/);
});

test('a notation may place only some diagrams itself and leave the rest to the layout engine', async () => {
  const boxes = {
    kind: 'boxes', problemsIn: () => [],
    placedSceneOf: spec => (spec.placed ? { width: 1, height: 1, nodes: [], edges: [] } : null),
    sceneOf: () => ({ nodes: [{ id: 'a', shape: 'box', label: 'a' }], edges: [] }),
  };
  const kit = notations([boxes]);
  assert.deepEqual(await kit.laidOut({ id: 'x', kind: 'boxes', title: 't', placed: true }), { width: 1, height: 1, nodes: [], edges: [] });
  assert.equal((await kit.laidOut({ id: 'x', kind: 'boxes', title: 't' })).nodes[0].id, 'a');
});
