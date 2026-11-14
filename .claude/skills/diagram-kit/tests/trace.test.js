const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const GraphFocus = require(path.join(__dirname, '..', 'lib', 'trace.js'));

const EDGES = [
  { from: 'cli', to: 'fields' },
  { from: 'fields', to: 'wire' },
  { from: 'port', to: 'fields' },
  { from: 'port', to: 'wire' },
];

test('a focused node traces the edges leaving it as outbound', () => {
  assert.deepEqual(GraphFocus.focusOf(EDGES, 'fields').outbound, [1]);
});

test('a focused node traces the edges arriving at it as inbound', () => {
  assert.deepEqual(GraphFocus.focusOf(EDGES, 'fields').inbound, [0, 2]);
});

test('the nodes at the far end of a traced edge are its neighbours', () => {
  assert.deepEqual(GraphFocus.focusOf(EDGES, 'fields').neighbours.sort(), ['cli', 'port', 'wire']);
});

test('a node no edge touches traces nothing', () => {
  assert.equal(GraphFocus.focusOf(EDGES, 'alone'), null);
});

test('an edge from a node to itself is traced once, as outbound', () => {
  const focus = GraphFocus.focusOf([{ from: 'a', to: 'a' }], 'a');

  assert.deepEqual(focus, { outbound: [0], inbound: [], neighbours: [] });
});

test('tracing several boxes traces every edge leaving any of them as outbound', () => {
  assert.deepEqual(GraphFocus.focusOfAll(EDGES, ['cli', 'port']).outbound, [0, 2, 3]);
});

test('tracing several boxes traces every edge arriving at any of them as inbound', () => {
  assert.deepEqual(GraphFocus.focusOfAll(EDGES, ['fields', 'wire']).inbound, [0, 2, 3]);
});

test('an edge between two traced boxes is traced once, as outbound', () => {
  const focus = GraphFocus.focusOfAll(EDGES, ['cli', 'fields']);

  assert.deepEqual(focus.outbound, [0, 1]);
  assert.deepEqual(focus.inbound, [2]);
});

test('the neighbours of several traced boxes leave out the traced boxes themselves', () => {
  assert.deepEqual(GraphFocus.focusOfAll(EDGES, ['cli', 'fields']).neighbours.sort(), ['port', 'wire']);
});

test('tracing no boxes traces nothing', () => {
  assert.equal(GraphFocus.focusOfAll(EDGES, []), null);
});

test('a press on an untraced box adds it to the traced boxes', () => {
  assert.deepEqual([...GraphFocus.toggle(new Set(['cli']), 'port')].sort(), ['cli', 'port']);
});

test('a press on a traced box removes only that box', () => {
  assert.deepEqual([...GraphFocus.toggle(new Set(['cli', 'port']), 'cli')], ['port']);
});
