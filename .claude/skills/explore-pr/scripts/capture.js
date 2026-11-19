// Headless Chrome never answers a captureBeyondViewport screenshot of a page
// about 15,000 px tall, so a tall page is captured a tile at a time.
const TILE_HEIGHT = 5000;
const CALL_DEADLINE_MS = 120000;

function tilesOf({ width, height }) {
  const whole = Math.ceil(height);
  const count = Math.max(1, Math.ceil(whole / TILE_HEIGHT));
  return Array.from({ length: count }, (_, index) => {
    const y = index * TILE_HEIGHT;
    return {
      name: count === 1 ? '' : `-${index + 1}`,
      clip: { x: 0, y, width, height: Math.min(TILE_HEIGHT, whole - y), scale: 1 },
    };
  });
}

function withinDeadline(reply, ms, call) {
  let timer;
  const expiry = new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error(`${call} gave no reply within ${ms} ms`)), ms);
  });
  return Promise.race([reply, expiry]).finally(() => clearTimeout(timer));
}

module.exports = { TILE_HEIGHT, CALL_DEADLINE_MS, tilesOf, withinDeadline };
