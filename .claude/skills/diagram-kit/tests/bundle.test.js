const test = require('node:test');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const { browserBundle } = require('../lib/bundle.js');
const { kitWith } = require('../lib/index.js');

const CLI = path.join(__dirname, '..', 'bin', 'diagram.js');
const FIXTURES = path.join(__dirname, 'fixtures');
const SKILLS = path.resolve(__dirname, '..', '..');
const BOXES = { id: 'd1', kind: 'boxes', title: 'Two boxes', boxes: [{ id: 'a', label: 'alpha', status: 'new' }, { id: 'b', label: 'beta' }] };
const exampleOf = file => JSON.parse(fs.readFileSync(path.join(SKILLS, file), 'utf8'));

function inABrowser(bundle) {
  const page = vm.createContext({ console, setTimeout, clearTimeout });
  vm.runInContext('var window = globalThis, self = globalThis, global = globalThis;', page);
  vm.runInContext(bundle, page);
  const taking = method => vm.runInContext(`(async (...written) => DiagramKit.live.${method}(...written.map(each => JSON.parse(each))))`, page);
  const within = method => (...values) => taking(method)(...values.map(value => JSON.stringify(value ?? null)));
  return { kinds: page.DiagramKit.live.kinds, laidOut: within('laidOut'), problemsIn: within('problemsIn'), remarksOn: within('remarksOn'), figureOf: within('figureOf') };
}
const plain = value => JSON.parse(JSON.stringify(value));
const everything = browserBundle();

test('the bundle gives a page the kit, with every notation beside it', () => {
  assert.deepEqual(plain(inABrowser(browserBundle(FIXTURES)).kinds()), ['boxes', 'words']);
  assert.deepEqual(plain(inABrowser(everything).kinds()), kitWith().kinds());
});

test('a page lays a diagram out exactly as a build does', async () => {
  const laid = await inABrowser(browserBundle(FIXTURES)).laidOut(BOXES);

  assert.deepEqual(plain(laid), plain(await kitWith(FIXTURES).laidOut(BOXES)));
  assert.ok(laid.nodes.every(node => node.width > 0));
});

for (const example of ['draw-class-diagram/examples/retention-classes.json', 'draw-sequence-diagram/examples/start-up-claims.json', 'draw-activity-diagram/examples/cold-start.json']) {
  test(`a page lays out ${path.basename(example)} as a build does`, async () => {
    const spec = exampleOf(example);

    assert.deepEqual(plain(await inABrowser(everything).laidOut(spec)), plain(await kitWith().laidOut(spec)));
  });
}

test('a page checks a diagram, remarks on it and draws its figure', async () => {
  const live = inABrowser(everything), spec = exampleOf('draw-data-flow-diagram/examples/launcher-bootstrap.json');

  assert.deepEqual(plain(await live.problemsIn({ ...spec, processes: [] })), ['diagram launcher-bootstrap: no processes; a data flow diagram shows what is done to data']);
  assert.match((await live.remarksOn(spec))[0], /2 flows cross a trust boundary/);
  assert.match(await live.figureOf(spec, await live.laidOut(spec)), /^<figure class="dk-figure" id="diagram-launcher-bootstrap">.*<svg class="dk"/);
});

test('the bundle needs nothing a browser lacks', () => {
  assert.doesNotMatch(everything, /require\(['"]node:/);
  assert.throws(() => browserBundle(path.join(__dirname, 'fixtures-needing-node')), /fs.*a browser cannot load it/);
});

test('the command line writes the bundle', () => {
  const printed = spawnSync('node', [CLI, 'bundle', '--notations', FIXTURES], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });

  assert.equal(printed.status, 0, printed.stderr);
  assert.equal(printed.stdout, browserBundle(FIXTURES));
});
