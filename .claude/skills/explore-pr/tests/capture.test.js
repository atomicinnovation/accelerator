const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const { TILE_HEIGHT, tilesOf, withinDeadline } = require(path.join(__dirname, '..', 'scripts', 'capture.js'));

test('a page no taller than one tile is captured whole', () => {
  assert.deepEqual(tilesOf({ width: 1425, height: TILE_HEIGHT }), [
    { name: '', clip: { x: 0, y: 0, width: 1425, height: TILE_HEIGHT, scale: 1 } },
  ]);
});

test('a taller page is captured in numbered tiles that cover it exactly', () => {
  const tiles = tilesOf({ width: 1425, height: 2 * TILE_HEIGHT + 570 });

  assert.deepEqual(tiles.map(tile => tile.name), ['-1', '-2', '-3']);
  assert.deepEqual(tiles.map(tile => [tile.clip.y, tile.clip.height]), [
    [0, TILE_HEIGHT],
    [TILE_HEIGHT, TILE_HEIGHT],
    [2 * TILE_HEIGHT, 570],
  ]);
  assert.ok(tiles.every(tile => tile.clip.width === 1425 && tile.clip.x === 0));
});

test('no tile is taller than the height headless Chrome renders reliably', () => {
  const tiles = tilesOf({ width: 1425, height: 15570 });

  assert.ok(tiles.every(tile => tile.clip.height <= TILE_HEIGHT));
  assert.equal(tiles.reduce((sum, tile) => sum + tile.clip.height, 0), 15570);
});

test('a fractional page height is rounded up so its last pixels are kept', () => {
  const tiles = tilesOf({ width: 1425.5, height: TILE_HEIGHT + 0.25 });

  assert.deepEqual(tiles.map(tile => tile.clip.height), [TILE_HEIGHT, 1]);
});

test('a reply that arrives in time is passed through', async () => {
  assert.equal(await withinDeadline(Promise.resolve('reply'), 50, 'Page.navigate'), 'reply');
});

test('a reply that never arrives fails, naming the call it waited on', async () => {
  const never = new Promise(() => {});

  await assert.rejects(withinDeadline(never, 20, 'Page.captureScreenshot'), {
    message: /Page\.captureScreenshot gave no reply within 20 ms/,
  });
});

test('a reply that arrives in time leaves no timer to hold the process open', async () => {
  const before = process.getActiveResourcesInfo().filter(kind => kind === 'Timeout').length;

  await withinDeadline(Promise.resolve('reply'), 60000, 'Runtime.evaluate');

  const after = process.getActiveResourcesInfo().filter(kind => kind === 'Timeout').length;
  assert.equal(after, before);
});
