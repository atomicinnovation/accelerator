const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const notation = require('../notation.js');
const { kitWith } = require('../../diagram-kit/lib/index.js');

const DAY = 86400;
const SPEC = {
  id: 'claim-ages', kind: 'timeline', title: 'A claim protects its version for 14 days', scale: 'linear', from: 0, to: 30 * DAY,
  ticks: [{ at: 0, label: 'now' }, { at: 14 * DAY, label: '14 d' }, { at: 30 * DAY, label: '30 d' }],
  spans: [
    { from: 0, to: 14 * DAY, label: 'fresh: kept', tone: 'ok' },
    { from: 14 * DAY, to: 30 * DAY, label: 'stale: removed', tone: 'bad', link: 'src/claims.rs:159' },
    { from: 2 * DAY, to: 3 * DAY, label: 'a claim two days old is fresh', tone: 'info' },
  ],
  marks: [{ at: 20 * DAY, label: '1.24.0', tone: 'bad' }, { at: 2 * DAY, label: '1.25.0' }],
};
const resolveLink = link => ({ href: `#${link}`, label: `Open ${link}` });
const html = notation.htmlOf(SPEC, { resolveLink });
const number = (text, name) => Number(text.match(new RegExp(` ${name}="([-0-9.]+)"`))[1]);
const groups = cls => html.match(new RegExp(`<g class="${cls}[^"]*">.*?</g>`, 'g')) || [];
const problems = change => notation.problemsIn({ ...SPEC, ...change });

test('it draws diagrams of kind timeline, by itself', () => {
  assert.equal(notation.kind, 'timeline');
  assert.equal(typeof notation.htmlOf, 'function');
});

test('a sound timeline has no problems', () => {
  assert.deepEqual(notation.problemsIn(SPEC), []);
});

test('it is a drawing that scrolls sideways inside itself, named by its title', () => {
  assert.match(html, /^<div class="dk-scroll"><svg class="dk-timeline" xmlns="http:\/\/www.w3.org\/2000\/svg" width="760" height="\d+" viewBox="0 0 760 \d+" role="img" aria-label="A claim protects its version for 14 days">/);
  assert.match(html, /<\/svg><\/div>$/);
});

test('an axis runs the width of the drawing, with a labelled tick at each value given', () => {
  const ticks = groups('tick');

  assert.match(html, /<line class="axis" x1="24" y1="\d+" x2="736" y2="\d+"\/>/);
  assert.deepEqual(ticks.map(tick => tick.match(/<text[^>]*>(.*?)<\/text>/)[1]), ['now', '14 d', '30 d']);
  assert.deepEqual(ticks.map(tick => number(tick, 'x1')), [24, 24 + 712 * 14 / 30, 736].map(x => Math.round(x * 100) / 100));
});

test('a span is a bar from its start to its end, in its tone', () => {
  const [fresh, stale, young] = groups('span');

  assert.match(fresh, /^<g class="span t-ok">/);
  assert.deepEqual([number(fresh, 'x'), number(fresh, 'width')], [24, Math.round(712 * 14 / 30 * 100) / 100]);
  assert.match(stale, /^<g class="span t-bad">/);
  assert.match(young, /^<g class="span t-info">/);
});

test('spans that would overlap are drawn in lanes, one under another', () => {
  const [fresh, stale, young] = groups('span');

  assert.equal(number(stale, 'y'), number(fresh, 'y'));
  assert.ok(number(young, 'y') > number(fresh, 'y'));
});

test('a label too long for its bar is written under it', () => {
  const [fresh, , young] = groups('span');
  const baseline = span => number(span.match(/<text[^>]*>/)[0], 'y');

  assert.ok(baseline(fresh) < number(fresh, 'y') + 18);
  assert.ok(baseline(young) > number(young, 'y') + 18);
  assert.match(young, /text-anchor="start"/);
});

test('a mark is a dot on the axis with its label above, and neighbours\' labels at different heights', () => {
  const [old, recent] = groups('mark');

  assert.match(old, /^<g class="mark t-bad">/);
  assert.equal(number(old.match(/<circle[^>]*>/)[0], 'cx'), Math.round((24 + 712 * 20 / 30) * 100) / 100);
  assert.notEqual(number(old.match(/<text[^>]*>/)[0], 'y'), number(recent.match(/<text[^>]*>/)[0], 'y'));
});

test('a linked span or mark opens its link from its label', () => {
  assert.match(html, /<a href="#src\/claims.rs:159" aria-label="Open src\/claims.rs:159"><text[^>]*>stale: removed<\/text><\/a>/);
});

test('a log scale spaces values by their ratio', () => {
  const log = notation.htmlOf({ ...SPEC, scale: 'log', from: 60, to: 6000, spans: [], marks: [], ticks: [{ at: 60, label: 'a' }, { at: 600, label: 'b' }, { at: 6000, label: 'c' }] }, { resolveLink });
  const ticks = log.match(/<g class="tick">.*?<\/g>/g).map(tick => number(tick, 'x1'));

  assert.deepEqual(ticks, [24, 380, 736]);
});

test('the range is taken from what is drawn when it is not given', () => {
  const ranged = notation.htmlOf({ ...SPEC, from: undefined, to: undefined, ticks: [], marks: [{ at: 10, label: 'a' }, { at: 20, label: 'b' }], spans: [] }, { resolveLink });

  assert.deepEqual(ranged.match(/<circle[^>]*>/g).map(dot => number(dot, 'cx')), [24, 736]);
});

test('a timeline needs something on it, placed by numbers', () => {
  assert.deepEqual(problems({ spans: [], marks: [] }), ['no spans and no marks']);
  assert.deepEqual(problems({ scale: 'cubic', from: 'x', ticks: [{ at: '1', label: 'a' }, { at: 1 }], spans: [{ from: 5, to: 1, label: 'back' }, { from: 1, label: 'open' }], marks: [{ at: 1, label: 'm', tone: 'fine', link: 4 }, { label: 'lost' }] }), [
    'scale "cubic" is not one of linear, log',
    'from must be a number',
    'tick a: at must be a number',
    'tick 2: no label',
    'span back: ends before it starts',
    'span open: to must be a number',
    'mark m: tone "fine" is not one of ok, warn, bad, info, none',
    'mark m: link must be text',
    'mark lost: at must be a number',
  ]);
});

test('a log scale cannot place zero or less', () => {
  assert.deepEqual(problems({ scale: 'log' }), ['a log scale cannot place 0; start above zero']);
});

const EXAMPLES = path.join(__dirname, '..', 'examples');
for (const file of fs.readdirSync(EXAMPLES).filter(name => name.endsWith('.json'))) {
  test(`the example ${file} is sound and draws`, () => {
    const kit = kitWith(), example = JSON.parse(fs.readFileSync(path.join(EXAMPLES, file), 'utf8'));
    assert.deepEqual(kit.problemsIn(example), []);
    assert.match(kit.figureOf(example, null), /<div class="dk-drawn"><div class="dk-scroll"><svg class="dk-timeline"/);
  });
}
