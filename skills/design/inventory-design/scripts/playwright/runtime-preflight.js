// Locating the bootstrapped Playwright namespace, for the suites that need one.
//
// Shared by the two root-level suites rather than copied into each: they must
// resolve the namespace exactly as the launcher does, and two copies of that
// arithmetic would be two chances to drift from it.
//
// Nothing here skips. Both callers run only under a task whose preflight has
// already guaranteed a runtime, so an absent one is a visible failure.

import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';

const LOCK_FILE = new URL('./package-lock.json', import.meta.url).pathname;

// The namespace `ensure-playwright.sh` populates: the cache root, then the
// first eight hex characters of the lockfile's sha256.
export function playwrightNsRoot() {
  const cacheRoot =
    process.env.ACCELERATOR_PLAYWRIGHT_CACHE ||
    `${process.env.HOME}/.cache/accelerator/playwright`;
  const lockhash = createHash('sha256')
    .update(readFileSync(LOCK_FILE))
    .digest('hex')
    .slice(0, 8);
  return resolve(cacheRoot, lockhash);
}

// Asserting on the installed package rather than the cache root: an empty or
// half-populated namespace would satisfy a directory-existence check and then
// fail every browser command with something far less diagnosable.
export function requireRuntime() {
  const nsRoot = playwrightNsRoot();
  assert.ok(
    existsSync(resolve(nsRoot, 'node_modules', 'playwright', 'index.js')),
    `Playwright is not installed for this lockhash namespace (${nsRoot}). ` +
      'Run ensure-playwright.sh; this lane does not skip.'
  );
  return nsRoot;
}
