const test = require('node:test');
const assert = require('node:assert/strict');

const { svgOf } = require('../lib/render.js');
const { SHAPES } = require('../lib/shapes.js');
const { widthOf } = require('../lib/text.js');

const SCENE = {
  width: 400, height: 300,
  nodes: [
    { id: 'frag', shape: 'fragment', operator: 'alt', guard: 'claim is young', dividers: [{ y: 150, guard: 'else' }], behind: true, inert: true, x: 20, y: 80, width: 360, height: 140 },
    { id: 'life:cli', shape: 'lifeline', behind: true, inert: true, x: 80, y: 44, width: 0, height: 240 },
    { id: 'act:1', shape: 'activation', behind: true, inert: true, status: 'new', x: 275, y: 100, width: 10, height: 60 },
    { id: 'cli', shape: 'box', label: 'launcher', x: 30, y: 10, width: 100, height: 32 },
    { id: 'user', shape: 'actor', label: 'Operator', x: 250, y: 4, width: 70, height: 56 },
    { id: 'note:1', shape: 'note', text: 'Young enough: stop here.', tone: 'risk', inert: true, x: 40, y: 230, width: 180, height: 30 },
    { id: 'end:1', shape: 'destroy', inert: true, x: 272, y: 262, width: 16, height: 16 },
  ],
  edges: [
    { id: 'm1', from: 'cli', to: 'user', label: '1: lstat the claim', head: 'filled', plate: false, link: 'https://example.test/m1', points: [[80, 120], [275, 120]], labelBox: { x: 100, y: 102, width: 150, height: 15 } },
    { id: 'm2', from: 'user', to: 'cli', label: 'Ok', head: 'open', line: 'dashed', plate: false, link: 'src/a.rs:4', points: [[275, 160], [80, 160]], labelBox: { x: 160, y: 142, width: 30, height: 15 } },
  ],
};
const svg = svgOf(SCENE, { uid: 's1', title: 'A sequence' });
const groupHolding = (markup, attribute) => markup.slice(markup.lastIndexOf('<g ', markup.indexOf(attribute))).split('</g>')[0];
const part = id => groupHolding(svg, `data-part="${id}"`);
const edge = id => groupHolding(svg, `data-edge="${id}"`);

test('a part that only supports the picture cannot be traced or focused', () => {
  assert.match(part('life:cli'), /^<g class="node lifeline" data-part="life:cli">/);
  assert.doesNotMatch(part('life:cli'), /data-node|tabindex|role=/);
  assert.match(svg, /data-node="cli" tabindex="0" role="button"/);
});

test('a lifeline is a line down from its place', () => {
  assert.match(part('life:cli'), /<line class="body" x1="80" y1="44" x2="80" y2="284"\/>/);
});

test('an activation is a bar that keeps its status', () => {
  assert.match(part('act:1'), /^<g class="node activation s-new"/);
  assert.match(part('act:1'), /<rect class="body" x="275" y="100" width="10" height="60"\/>/);
});

test('a fragment is a frame with its operator in a tab and its guard beside it', () => {
  const fragment = part('frag');
  assert.match(fragment, /<rect class="body" x="20" y="80" width="360" height="140"\/>/);
  assert.match(fragment, /<path class="tab" d="M20 80 H[\d.]+ V[\d.]+ L[\d.]+ 100 H20 Z"\/>/);
  assert.match(fragment, /<text class="operator" x="28" y="94" text-anchor="start">alt<\/text>/);
  assert.match(fragment, /<text class="guard" x="[\d.]+" y="94" text-anchor="start">\[claim is young\]<\/text>/);
});

test('a fragment divides its operands with a dashed line and the next guard', () => {
  const fragment = part('frag');
  assert.match(fragment, /<line class="divider" x1="20" y1="150" x2="380" y2="150"\/>/);
  assert.match(fragment, /<text class="guard" x="28" y="165" text-anchor="start">\[else\]<\/text>/);
});

test('an actor is a figure standing over its name', () => {
  const actor = groupHolding(svg, 'data-node="user"');
  assert.match(actor, /^<g class="node actor leaf" data-node="user"/);
  assert.match(actor, /<rect class="body" x="250" y="4" width="70" height="56"\/>/);
  assert.match(actor, /<circle class="figure" cx="285" cy="12" r="6"\/>/);
  assert.match(actor, /<path class="figure" d="M285 18 V34 M273 24 H297 M285 34 L276 44 M285 34 L294 44"\/>/);
  assert.match(actor, /<text class="name" x="285" y="56" text-anchor="middle">Operator<\/text>/);
});

test('a note is a sheet with a folded corner holding its text', () => {
  const note = part('note:1');
  assert.match(note, /^<g class="node note t-risk"/);
  assert.match(note, /<path class="body" d="M40 230 H212 L220 238 V260 H40 Z"\/><path class="fold" d="M212 230 V238 H220"\/>/);
  assert.match(note, /<text class="message" x="50" y="250" text-anchor="start">Young enough: stop here\.<\/text>/);
});

test('a destruction is a cross', () => {
  assert.match(part('end:1'), /<path class="body" d="M272 262 L288 278 M288 262 L272 278"\/>/);
});

test('supporting parts are drawn before the edges, and the rest after, in scene order', () => {
  const at = marker => svg.indexOf(marker);
  assert.ok(at('data-part="frag"') < at('data-part="life:cli"'));
  assert.ok(at('data-part="act:1"') < at('data-edge="m1"'));
  assert.ok(at('data-edge="m2"') < at('data-node="cli"'));
  assert.ok(at('data-node="user"') < at('data-part="note:1"'));
});

test('an edge label without a plate is plain text over its line', () => {
  assert.doesNotMatch(edge('m1'), /<rect/);
  assert.match(edge('m1'), /<text class="message" x="175" y="113" text-anchor="middle">1: lstat the claim<\/text>/);
});

test('an edge whose link is an address opens it from its label', () => {
  assert.match(edge('m1'), /<a href="https:\/\/example.test\/m1" aria-label="Open https:\/\/example.test\/m1"><text class="message"/);
});

test('an edge whose link nothing resolves tells it on hover', () => {
  assert.doesNotMatch(edge('m2'), /<a /);
  assert.match(edge('m2'), /<title>src\/a.rs:4<\/title>/);
});

test('the kit says how large an actor and a note must be', () => {
  const actor = SHAPES.actor.sizeOf({ label: 'Operator' });
  assert.ok(actor.width >= widthOf('Operator', 'name') + 8);
  assert.equal(actor.height, 56);
  const note = SHAPES.note.sizeOf({ text: 'Young enough: stop here, and say so in the report.' }, 180);
  assert.equal(note.width, 180);
  assert.equal(note.height, 2 * 15 + 14);
});
