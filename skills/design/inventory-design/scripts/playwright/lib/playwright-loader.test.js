import { test } from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

import { importPlaywright } from './playwright-loader.js';

const __dir = dirname(fileURLToPath(import.meta.url));
const fixtureNsRoot = (name) => resolve(__dir, `__fixtures__/${name}`);

test('importPlaywright exposes chromium from the namespaced cache (regression)', async () => {
  const result = await importPlaywright({
    nsRoot: fixtureNsRoot('fake-playwright'),
  });
  assert.ok('chromium' in result, 'chromium must be a named export');
  assert.equal(typeof result.chromium.launch, 'function');
});

test('importPlaywright selects exports["."].import over pkg.main', async () => {
  const result = await importPlaywright({
    nsRoot: fixtureNsRoot('fake-playwright-distinct-entries'),
  });
  assert.equal(
    result.__selected,
    'exports-import',
    'loader must pick exports["."].import, not pkg.main'
  );
});

test('importPlaywright throws when exports["."].import is not a string', async () => {
  await assert.rejects(
    importPlaywright({
      nsRoot: fixtureNsRoot('fake-playwright-nested-conditions'),
    }),
    /pkg\.exports\['\.'\]\.import is not a string/
  );
});
