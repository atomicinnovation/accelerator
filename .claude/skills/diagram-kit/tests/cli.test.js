const test = require('node:test');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const CLI = path.join(__dirname, '..', 'bin', 'diagram.js');
const NOTATIONS = path.join(__dirname, 'fixtures');
const SPEC = { id: 'd1', kind: 'boxes', title: 'Two boxes', caption: 'c', boxes: [{ id: 'a', label: 'alpha', status: 'new' }, { id: 'b', label: 'beta' }] };

function workdir(spec = SPEC) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'diagram-kit-cli-'));
  fs.writeFileSync(path.join(dir, 'spec.json'), typeof spec === 'string' ? spec : JSON.stringify(spec));
  return dir;
}
const run = (dir, ...args) => spawnSync('node', [CLI, ...args, '--notations', NOTATIONS], { cwd: dir, encoding: 'utf8' });

test('checking a sound diagram succeeds quietly', () => {
  const result = run(workdir(), 'check', 'spec.json');
  assert.equal(result.status, 0);
  assert.equal(result.stderr, '');
});

test('checking a diagram with problems lists them and fails', () => {
  const result = run(workdir({ ...SPEC, title: '', boxes: [{ id: 'a', status: 'fresh' }] }), 'check', 'spec.json');
  assert.equal(result.status, 1);
  assert.match(result.stderr, /warning: diagram d1: no title/);
  assert.match(result.stderr, /warning: diagram d1: box a: status "fresh"/);
});

test('a file that is not JSON is reported by name, not by a stack trace', () => {
  const result = run(workdir('{'), 'check', 'spec.json');
  assert.equal(result.status, 1);
  assert.match(result.stderr, /^warning: spec\.json: not valid JSON/);
  assert.doesNotMatch(result.stderr, /at .*\.js:\d+/);
});

test('layout writes where every node and edge goes', () => {
  const dir = workdir();
  const result = run(dir, 'layout', 'spec.json', '--out', 'scene.json');
  assert.equal(result.status, 0, result.stderr);
  const scene = JSON.parse(fs.readFileSync(path.join(dir, 'scene.json'), 'utf8'));
  assert.deepEqual(scene.nodes.map(node => node.id), ['a', 'b']);
  assert.equal(scene.edges[0].points.length >= 2, true);
});

test('rendering to .svg writes a drawing that carries its own styles', () => {
  const dir = workdir();
  const result = run(dir, 'render', 'spec.json', '--out', 'out.svg');
  assert.equal(result.status, 0, result.stderr);
  const svg = fs.readFileSync(path.join(dir, 'out.svg'), 'utf8');
  assert.match(svg, /^<svg class="dk"/);
  assert.match(svg, /<style>[\s\S]*\.dk text/);
  assert.match(svg, /font-face/);
});

test('rendering to .html writes a page that needs nothing from the network', () => {
  const dir = workdir();
  const result = run(dir, 'render', 'spec.json', '--out', 'out.html');
  assert.equal(result.status, 0, result.stderr);
  const page = fs.readFileSync(path.join(dir, 'out.html'), 'utf8');
  assert.match(page, /^<!doctype html>/);
  assert.match(page, /<title>Two boxes<\/title>/);
  assert.match(page, /<figure class="dk-figure" id="diagram-d1">/);
  assert.match(page, /src:url\(data:font\/woff2;base64,/);
  assert.match(page, /attachTracing\(document\)/);
  assert.doesNotMatch(page, /https?:\/\/(?!www\.w3\.org)/);
  assert.doesNotMatch(page, /org\.eclipse\.elk/);
});

test('a page shows what a notation remarks on', () => {
  const three = { ...SPEC, boxes: [...SPEC.boxes, { id: 'c', label: 'gamma' }] };
  const dir = workdir(three);
  run(dir, 'render', 'spec.json', '--out', 'out.html');
  assert.match(fs.readFileSync(path.join(dir, 'out.html'), 'utf8'), /<ul class="dk-remarks"><li>More than two boxes\.<\/li><\/ul>/);
});

test('rendering a diagram with problems writes nothing and fails', () => {
  const dir = workdir({ ...SPEC, kind: 'blobs' });
  const result = run(dir, 'render', 'spec.json', '--out', 'out.html');
  assert.equal(result.status, 1);
  assert.match(result.stderr, /no notation draws kind "blobs"/);
  assert.equal(fs.existsSync(path.join(dir, 'out.html')), false);
});

test('an unknown command explains how the tool is used', () => {
  const result = run(workdir(), 'draw', 'spec.json');
  assert.equal(result.status, 2);
  assert.match(result.stderr, /usage: diagram\.js/);
});

test('preparing a list of diagrams lays out the sound ones and reports the rest, without failing', () => {
  const dir = workdir([SPEC, { ...SPEC, id: 'd2', title: '' }, { id: 'd3', kind: 'blobs', title: 't' }, { ...SPEC, id: 'd4', boxes: [...SPEC.boxes, { id: 'c', label: 'gamma' }] }]);
  const result = run(dir, 'prepare', 'spec.json');
  assert.equal(result.status, 0, result.stderr);
  const prepared = JSON.parse(result.stdout);
  assert.deepEqual(prepared.kinds, ['boxes']);
  assert.deepEqual(prepared.diagrams.map(each => each.id), ['d1', 'd2', 'd3', 'd4']);
  const [sound, untitled, unknown, remarked] = prepared.diagrams;
  assert.deepEqual(sound.problems, []);
  assert.deepEqual(sound.scene.nodes.map(node => node.id), ['a', 'b']);
  assert.match(sound.legend, /new in this change/);
  assert.deepEqual(untitled.problems, ['no title; say the point the diagram makes']);
  assert.equal(untitled.scene, null);
  assert.match(unknown.problems[0], /^no notation draws kind "blobs"/);
  assert.deepEqual(remarked.remarks, ['More than two boxes.']);
});

test('preparing an empty list says which kinds can be drawn', () => {
  const result = run(workdir([]), 'prepare', 'spec.json');
  assert.deepEqual(JSON.parse(result.stdout), { kinds: ['boxes'], diagrams: [] });
});

test('rendering a file with no diagrams in it is refused, not a crash', () => {
  const result = run(workdir([]), 'render', 'spec.json', '--out', 'out.html');
  assert.equal(result.status, 1);
  assert.match(result.stderr, /^warning: spec\.json: no diagrams in it/);
  assert.doesNotMatch(result.stderr, /at .*\.js:\d+/);
});
