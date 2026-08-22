import { test } from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

import { importPlaywright } from './playwright-loader.js';

const __dir = dirname(fileURLToPath(import.meta.url));
const fixtureNsRoot = (name) => resolve(__dir, `__fixtures__/${name}`);

test('importPlaywright exposes chromium from the driver tree', async () => {
  const result = await importPlaywright({
    nsRoot: fixtureNsRoot('fake-playwright'),
  });
  assert.ok('chromium' in result, 'chromium must be a named export');
  assert.equal(typeof result.chromium.launch, 'function');
});
