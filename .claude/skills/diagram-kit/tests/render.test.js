const test = require('node:test');
const assert = require('node:assert/strict');

const { svgOf, legendOf, roundedPath, headOf } = require('../lib/render.js');

const SCENE = {
  width: 300, height: 260,
  nodes: [
    { id: 'launch', shape: 'group', label: 'launch', status: 'changed', parent: null, depth: 0, x: 10, y: 60, width: 280, height: 190 },
    { id: 'cli', shape: 'box', label: 'cli::main', parent: null, depth: 0, x: 100, y: 10, width: 100, height: 32 },
    { id: 'prune', shape: 'box', label: 'prune <T>', detail: 'removes "stale" trees', status: 'new', tone: 'risk', link: 'https://example.test/prune', parent: 'launch', depth: 1, x: 40, y: 190, width: 174, height: 46 },
    { id: 'clock', shape: 'box', label: 'Clock', link: 'src/clock.rs:4', parent: 'launch', depth: 1, x: 40, y: 100, width: 120, height: 32 },
  ],
  edges: [
    { id: 'cli->prune', from: 'cli', to: 'prune', label: 'runs & waits', line: 'dashed', head: 'open', status: 'new', points: [[150, 42], [150, 80], [120, 80], [120, 190]], labelBox: { x: 154, y: 50, width: 80, height: 18 } },
    { id: 'prune->clock', from: 'prune', to: 'clock', head: 'filled', points: [[100, 190], [100, 132]], labelBox: null },
  ],
};
const svg = svgOf(SCENE, { uid: 'd1', title: 'How <launch> fits' });
const groupHolding = (markup, attribute) => markup.slice(markup.lastIndexOf('<g ', markup.indexOf(attribute))).split('</g>')[0];
const element = id => groupHolding(svg, `data-node="${id}"`);
const edgeMarkup = id => groupHolding(svg, `data-edge="${id.replace('>', '&gt;')}"`);

test('the drawing is as large as its scene, with a margin, and says what it shows', () => {
  assert.match(svg, /^<svg class="dk" data-graph="d1"/);
  assert.match(svg, /viewBox="-8 -8 316 276"/);
  assert.match(svg, /width="316" height="276"/);
  assert.match(svg, /role="img" aria-label="How &lt;launch&gt; fits"/);
});

test('a box is drawn where the scene put it, with its label and detail escaped', () => {
  assert.match(element('prune'), /<rect class="body" x="40" y="190" width="174" height="46" rx="7"\/>/);
  assert.match(element('prune'), /class="name"[^>]*>prune &lt;T&gt;</);
  assert.match(element('prune'), /class="detail"[^>]*>removes &quot;stale&quot; trees</);
});

test('a label is centred in the room its link control leaves', () => {
  assert.match(element('prune'), /class="name" x="115" y="209" text-anchor="middle"/);
  assert.match(element('cli'), /class="name" x="150" y="30" text-anchor="middle"/);
});

test('status and tone are classes on the node', () => {
  assert.match(element('prune'), /class="node box leaf s-new t-risk"/);
  assert.match(element('cli'), /class="node box leaf"/);
  assert.match(element('launch'), /class="node group s-changed"/);
});

test('a group is titled in capitals at its top left', () => {
  assert.match(element('launch'), /<rect class="body" x="10" y="60" width="280" height="190" rx="9"\/>/);
  assert.match(element('launch'), /class="heading" x="24" y="80" text-anchor="start"[^>]*>LAUNCH</);
});

test('a node can be traced from the keyboard', () => {
  assert.match(element('cli'), /tabindex="0" role="button" aria-label="Trace the arrows of cli::main"/);
});

test('a link that is an address opens from its own control', () => {
  assert.match(element('prune'), /<a class="node-link" href="https:\/\/example.test\/prune" aria-label="Open https:\/\/example.test\/prune">/);
});

test('a link nothing resolves is told on hover, with no control', () => {
  assert.doesNotMatch(element('clock'), /node-link/);
  assert.match(element('clock'), /<title>src\/clock.rs:4<\/title>/);
});

test('the host decides where a link leads and what it carries', () => {
  const hosted = svgOf(SCENE, { uid: 'd1', resolveLink: ref => (ref.includes('.rs:') ? { href: '#files', attributes: { 'data-open': 'src/clock.rs', 'data-line': 4 }, label: `Open ${ref}` } : null) });
  const clock = groupHolding(hosted, 'data-node="clock"');
  assert.match(clock, /<a class="node-link" href="#files" data-open="src\/clock.rs" data-line="4" aria-label="Open src\/clock.rs:4">/);
});

test('an edge says what it joins and how it changed', () => {
  assert.match(edgeMarkup('cli->prune'), /class="edge dashed s-new" data-edge="cli-&gt;prune" data-from="cli" data-to="prune"/);
});

test('an edge turns its corners on a curve', () => {
  assert.equal(roundedPath([[0, 0], [0, 20], [30, 20]], 6), 'M0 0 L0 14 Q0 20 6 20 L30 20');
  assert.equal(roundedPath([[0, 0], [0, 4], [30, 4]], 6), 'M0 0 L0 2 Q0 4 2 4 L30 4');
  assert.equal(roundedPath([[5, 5], [5, 50]], 6), 'M5 5 L5 50');
});

test('an open head is two strokes meeting at the end of the line', () => {
  assert.deepEqual(headOf('open', [[120, 80], [120, 190]]), { path: 'M115.5 181 L120 190 L124.5 181', cut: 0 });
  assert.match(edgeMarkup('cli->prune'), /<path class="head open" d="M115.5 181 L120 190 L124.5 181"\/>/);
  assert.match(edgeMarkup('cli->prune'), /<path class="line" d="M150 42 [^"]* L120 190"\/>/);
});

test('a filled head is a triangle, and the line stops at its base', () => {
  assert.deepEqual(headOf('filled', [[100, 190], [100, 132]]), { path: 'M95.5 141 L100 132 L104.5 141 Z', cut: 9 });
  assert.match(edgeMarkup('prune->clock'), /<path class="line" d="M100 190 L100 141"\/>/);
});

test('a head points the way its last run goes', () => {
  assert.equal(headOf('open', [[0, 5], [40, 5]]).path, 'M31 0.5 L40 5 L31 9.5');
  assert.equal(headOf('open', [[40, 5], [0, 5]]).path, 'M9 0.5 L0 5 L9 9.5');
});

test('an edge without a head has none', () => {
  assert.deepEqual(headOf('none', [[0, 0], [0, 9]]), { path: '', cut: 0 });
});

test('an edge label sits where the scene put it', () => {
  assert.match(edgeMarkup('cli->prune'), /<rect x="154" y="50" width="80" height="18" rx="9"\/><text class="edge-text" x="194" y="63" text-anchor="middle"[^>]*>runs &amp; waits</);
});

test('groups are drawn first, then edges, then boxes', () => {
  const at = marker => svg.indexOf(marker);
  assert.ok(at('data-node="launch"') < at('data-edge="cli-&gt;prune"'));
  assert.ok(at('data-edge="prune-&gt;clock"') < at('data-node="cli"'));
});

test('the legend names each status and tone the scene uses, once', () => {
  const legend = legendOf(SCENE);
  assert.equal((legend.match(/new in this change/g) || []).length, 1);
  assert.match(legend, /changed/);
  assert.match(legend, /risk/);
  assert.doesNotMatch(legend, /removed/);
});

test('a scene that marks nothing has no legend', () => {
  assert.equal(legendOf({ nodes: [{ id: 'a' }], edges: [] }), '');
});

test('the last corner before a head is tightened to leave the head a straight run', () => {
  assert.equal(roundedPath([[0, 0], [0, 20], [12, 20]], 6, 9), 'M0 0 L0 17 Q0 20 3 20 L12 20');
  assert.equal(roundedPath([[0, 0], [0, 20], [8, 20]], 6, 9), 'M0 0 L0 20 L8 20');
  assert.equal(roundedPath([[0, 0], [0, 20], [30, 20]], 6, 9), 'M0 0 L0 14 Q0 20 6 20 L30 20');
});

test('what a group is called is written again over the edges, so an arrow never hides it', () => {
  const at = marker => svg.indexOf(marker);
  const captions = svg.slice(at('<g class="captions"'), svg.indexOf('</g>', at('<g class="captions"')));
  assert.match(captions, /^<g class="captions" aria-hidden="true"><text class="heading"[^>]*>LAUNCH<\/text>$/);
  assert.ok(at('data-edge="prune-&gt;clock"') < at('<g class="captions"'));
  assert.ok(at('<g class="captions"') < at('data-node="cli"'));
});

test('a scene with no groups has no captions to write again', () => {
  assert.doesNotMatch(svgOf({ width: 10, height: 10, nodes: [{ id: 'a', shape: 'box', label: 'a', x: 0, y: 0, width: 10, height: 10 }], edges: [] }, {}), /captions/);
});
