const test = require('node:test');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const { kitWith, notations } = require('../lib/index.js');

const CLI = path.join(__dirname, '..', 'bin', 'diagram.js');
const FIXTURES = path.join(__dirname, 'fixtures');
const kit = kitWith(FIXTURES);
const BOXES = { id: 'd1', kind: 'boxes', title: 'Two boxes', boxes: [{ id: 'a', label: 'alpha', link: '#a' }, { id: 'b', label: 'beta' }] };
const WORDS = { id: 'w1', kind: 'words', title: 'Words', words: ['one', 'two'] };
const entries = markup => markup.match(/<li class="dk-(?:used|unused)">.*?<\/li>/g) || [];
const named = markup => entries(markup).map(entry => [entry.match(/class="dk-(used|unused)"/)[1], entry.match(/class="dk-meaning"><b>(.*?)<\/b>/)[1]]);

function run(specs, ...args) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'diagram-kit-legend-'));
  fs.writeFileSync(path.join(dir, 'spec.json'), JSON.stringify(specs));
  return { dir, ...spawnSync('node', [CLI, ...args, '--notations', FIXTURES], { cwd: dir, encoding: 'utf8' }) };
}

test('a legend is closed until it is asked for', async () => {
  const legend = kit.symbolsOf(BOXES, await kit.laidOut(BOXES));

  assert.match(legend, /^<details class="dk-symbols"><summary>Legend <span class="dk-notation">Box diagram<\/span><\/summary>/);
  assert.doesNotMatch(legend, /<details[^>]* open/);
});

test('it lists every symbol the notation has, those this diagram uses first', async () => {
  const legend = kit.symbolsOf(BOXES, await kit.laidOut(BOXES));

  assert.deepEqual(named(legend), [['used', 'Box'], ['used', 'Follows'], ['unused', 'Group'], ['unused', 'Owns']]);
});

test('each symbol is drawn as the kit draws it, beside what it means', async () => {
  const [box, follows] = entries(kit.symbolsOf(BOXES, await kit.laidOut(BOXES)));

  assert.match(box, /<span class="dk-sample"><svg class="dk dk-sample"[^>]*aria-hidden="true">.*<g class="node box[^"]*" data-part="sample">.*<\/svg><\/span><span class="dk-meaning"><b>Box<\/b> One thing\.<\/span>/);
  assert.match(follows, /<g class="edge dashed"[^>]*>.*<path class="head open"/);
  assert.doesNotMatch(box, /data-graph|data-node|tabindex/);
});

test('the symbols a diagram does not use wait behind a switch that says how many there are', async () => {
  const legend = kit.symbolsOf(BOXES, await kit.laidOut(BOXES));

  assert.match(legend, /<label class="dk-more"><input type="checkbox"> Show the 2 symbols this diagram does not use<\/label>/);
  assert.doesNotMatch(kit.symbolsOf({ ...WORDS, words: ['#one'] }, null), /dk-more/);
});

test('a diagram that uses every symbol but one says so in the singular', async () => {
  const most = notations([{ ...require('./fixtures/draw-boxes/notation.js'), symbols: require('./fixtures/draw-boxes/notation.js').symbols.slice(0, 3) }]);

  assert.match(most.symbolsOf(BOXES, await most.laidOut(BOXES)), /Show the 1 symbol this diagram does not use/);
});

test('how to use the diagram is said in its legend, and only what applies', async () => {
  const linked = kit.symbolsOf(BOXES, await kit.laidOut(BOXES));
  const bare = { ...BOXES, boxes: [{ id: 'a', label: 'alpha' }] };

  assert.match(linked, /<p class="dk-usage">Press a box to trace its arrows.*Esc.*<\/p>/);
  assert.match(linked, /<p class="dk-usage">↗ on a box opens/);
  assert.doesNotMatch(kit.symbolsOf(bare, await kit.laidOut(bare)), /dk-usage/);
});

test('a notation that draws itself says which of its symbols a diagram uses', () => {
  assert.deepEqual(named(kit.symbolsOf(WORDS, null)), [['used', 'Word'], ['unused', 'Link']]);
  assert.deepEqual(named(kit.symbolsOf({ ...WORDS, words: ['#one'] }, null)), [['used', 'Word'], ['used', 'Link']]);
  assert.match(kit.symbolsOf(WORDS, null), /<span class="dk-sample"><b>word<\/b><\/span>/);
});

test('a notation with no name of its own is named by its kind', async () => {
  const unnamed = notations([{ ...require('./fixtures/draw-boxes/notation.js'), name: undefined }]);

  assert.match(unnamed.symbolsOf(BOXES, await unnamed.laidOut(BOXES)), /<summary>Legend <span class="dk-notation">boxes<\/span><\/summary>/);
});

test('a notation with no symbols to explain has no legend', async () => {
  const plain = notations([{ ...require('./fixtures/draw-boxes/notation.js'), symbols: undefined }]);

  assert.equal(plain.symbolsOf(BOXES, await plain.laidOut(BOXES)), '');
});

test('a figure carries its legend only when asked, and then says how to use it there', async () => {
  const scene = await kit.laidOut(BOXES);
  const without = kit.figureOf(BOXES, scene), withLegend = kit.figureOf(BOXES, scene, { legend: true });

  assert.doesNotMatch(without, /dk-symbols/);
  assert.match(without, /dk-trace-hint/);
  assert.match(withLegend, /<\/div><details class="dk-symbols">/);
  assert.doesNotMatch(withLegend, /dk-trace-hint/);
  assert.match(kit.figureOf(WORDS, null, { legend: true }), /<\/div><details class="dk-symbols">/);
});

test('the command line draws legends on a page when asked', () => {
  const plain = run([BOXES], 'render', 'spec.json', '--out', 'plain.html'), keyed = run([BOXES, WORDS], 'render', 'spec.json', '--out', 'keyed.html', '--legend');

  assert.equal(keyed.status, 0, keyed.stderr);
  assert.doesNotMatch(fs.readFileSync(path.join(plain.dir, 'plain.html'), 'utf8'), /<details class="dk-symbols">/);
  assert.equal(fs.readFileSync(path.join(keyed.dir, 'keyed.html'), 'utf8').match(/<details class="dk-symbols">/g).length, 2);
});

test('preparing diagrams for a page hands over each legend when asked', () => {
  const plain = JSON.parse(run([BOXES], 'prepare', 'spec.json').stdout), keyed = JSON.parse(run([BOXES, WORDS, { ...BOXES, id: 'd2', title: '' }], 'prepare', 'spec.json', '--legend').stdout);

  assert.equal('symbols' in plain.diagrams[0], false);
  assert.deepEqual(keyed.diagrams.map(each => /^<details class="dk-symbols">/.test(each.symbols)), [true, true, false]);
});
