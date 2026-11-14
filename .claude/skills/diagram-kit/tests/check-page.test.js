const test = require('node:test');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const { chromePath } = require('../lib/browser.js');

const CLI = path.join(__dirname, '..', 'bin', 'diagram.js');
const CHECK = path.join(__dirname, '..', 'tools', 'check-page.js');
const NOTATIONS = path.join(__dirname, 'fixtures');
const SPEC = { id: 'd1', kind: 'boxes', title: 'Three boxes', boxes: [{ id: 'a', label: 'alpha' }, { id: 'b', label: 'beta', status: 'new' }, { id: 'c', label: 'gamma' }] };
const needsChrome = { skip: chromePath() ? false : 'needs Chrome or Chromium' };

function page(change = html => html) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'diagram-kit-check-'));
  fs.writeFileSync(path.join(dir, 'spec.json'), JSON.stringify(SPEC));
  const built = spawnSync('node', [CLI, 'render', 'spec.json', '--out', 'out.html', '--notations', NOTATIONS], { cwd: dir, encoding: 'utf8' });
  assert.equal(built.status, 0, built.stderr);
  const file = path.join(dir, 'out.html');
  fs.writeFileSync(file, change(fs.readFileSync(file, 'utf8')));
  return file;
}
function checked(file, ...args) {
  const result = spawnSync('node', [CHECK, file, ...args], { encoding: 'utf8' });
  return { status: result.status, report: JSON.parse(result.stdout || '{}'), stderr: result.stderr };
}
const narrowed = html => html.replace(/(data-node="a"[^>]*><rect class="body" x="[^"]*" y="[^"]*" width=")[^"]*"/, '$120"');
const stacked = html => html.replace(/(data-node="b"[^>]*><rect class="body" x="[^"]*" y=")[^"]*"/, (_, start) => `${start}${/data-node="a"[^>]*><rect class="body" x="[^"]*" y="([^"]*)"/.exec(html)[1]}"`);

test('a page the kit drew passes every check', needsChrome, () => {
  const { status, report, stderr } = checked(page());
  assert.equal(status, 0, stderr + JSON.stringify(report));
  assert.equal(report.ok, true);
  assert.deepEqual(report.diagrams.map(each => each.id), ['d1']);
  assert.equal(report.fontsLoaded, true);
  assert.deepEqual(report.diagrams[0].problems, []);
  assert.equal(report.tracing.traces && report.tracing.clears, true);
});

test('text wider than its box fails the check, naming the node', needsChrome, () => {
  const { status, report } = checked(page(narrowed));
  assert.equal(status, 1);
  assert.equal(report.ok, false);
  assert.ok(report.diagrams[0].problems.some(problem => /node a: text runs \d+px outside its box/.test(problem)), JSON.stringify(report.diagrams[0].problems));
});

test('a box drawn across another, or across an arrow, fails the check', needsChrome, () => {
  const { status, report } = checked(page(stacked));
  assert.equal(status, 1);
  assert.ok(report.diagrams[0].problems.some(problem => /^node (a|b): overlaps node (a|b)$/.test(problem)), JSON.stringify(report.diagrams[0].problems));
});

test('the check can keep a picture of each diagram', needsChrome, () => {
  const shots = fs.mkdtempSync(path.join(os.tmpdir(), 'diagram-kit-shots-'));
  const { status } = checked(page(), '--shots', shots);
  assert.equal(status, 0);
  assert.ok(fs.statSync(path.join(shots, 'd1.png')).size > 1000);
});

const overwritten = html => html.replace(/(data-node="a"[^>]*><rect class="body"[^>]*\/>)(<text class="name"[^>]*>[^<]*<\/text>)/, '$1$2$2');

test('one piece of a node\'s text written over another fails the check', needsChrome, () => {
  const { status, report } = checked(page(overwritten));
  assert.equal(status, 1);
  assert.ok(report.diagrams[0].problems.includes('node a: two pieces of its text overlap'), JSON.stringify(report.diagrams[0].problems));
});

const WORDS = { id: 'w1', kind: 'words', title: 'Two words', words: ['one', 'two'] };
function selfDrawnPage(change = html => html) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'diagram-kit-check-'));
  fs.writeFileSync(path.join(dir, 'spec.json'), JSON.stringify(WORDS));
  const built = spawnSync('node', [CLI, 'render', 'spec.json', '--out', 'out.html', '--notations', NOTATIONS], { cwd: dir, encoding: 'utf8' });
  assert.equal(built.status, 0, built.stderr);
  const file = path.join(dir, 'out.html');
  fs.writeFileSync(file, change(fs.readFileSync(file, 'utf8')));
  return file;
}
const overlapping = html => html.replace('<p class="words">', '<svg class="dk-timeline" width="200" height="40" viewBox="0 0 200 40"><text x="10" y="20">first label</text><text x="14" y="22">second label</text></svg><p class="words">');
const overwide = html => html.replace('<p class="words">', '<p class="words" style="width: 3000px">');

test('a figure a notation drew itself is checked and photographed too', needsChrome, () => {
  const shots = fs.mkdtempSync(path.join(os.tmpdir(), 'diagram-kit-shots-'));
  const { status, report, stderr } = checked(selfDrawnPage(), '--shots', shots);

  assert.equal(status, 0, stderr + JSON.stringify(report));
  assert.deepEqual(report.diagrams, [{ id: 'w1', selfDrawn: true, problems: [] }]);
  assert.ok(fs.statSync(path.join(shots, 'w1.png')).size > 0);
});

test('labels that lie on one another in a self-drawn drawing fail the check', needsChrome, () => {
  const { status, report } = checked(selfDrawnPage(overlapping));

  assert.equal(status, 1);
  assert.deepEqual(report.diagrams[0].problems, ['two of its labels overlap']);
});

test('a self-drawn figure wider than the page, with nothing to scroll it in, fails the check', needsChrome, () => {
  const { status, report } = checked(selfDrawnPage(overwide));

  assert.equal(status, 1);
  assert.deepEqual(report.diagrams[0].problems, ['it runs wider than the page']);
});
