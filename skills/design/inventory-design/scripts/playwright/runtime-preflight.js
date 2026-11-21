// Locating the vendored Playwright runtime, for the suites that need one.
//
// Shared by the two root-level suites rather than copied into each: they must
// hand the daemon its runtime exactly as `accelerator design executor` does,
// and two copies of that would be two chances to drift from it.
//
// Nothing here skips. Both callers run only under a task whose preflight has
// already resolved the driver and browser trees, so an absent one is a
// visible failure.

import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';

const DRIVER_TREE_VAR = 'ACCELERATOR_TREE_DRIVER';
const BROWSER_TREE_VAR = 'ACCELERATOR_TREE_BROWSER';
const PLAYWRIGHT_CORE_ENTRY = 'node_modules/playwright-core/index.mjs';
const CHROMIUM_SHELL = 'chrome-headless-shell';

function requireTree(variable, member) {
  const tree = process.env[variable];
  assert.ok(
    tree,
    `${variable} is not set. Run this suite through ` +
      '`mise run test:integration:design-automation`; it does not skip.'
  );
  assert.ok(
    existsSync(resolve(tree, member)),
    `${variable} (${tree}) holds no ${member}. Run ` +
      '`accelerator cache repair`; this lane does not skip.'
  );
  return resolve(tree, member);
}

// The environment the executor gives the daemon: the driver tree as the root
// playwright-core is imported from, and the bundled browser to launch.
export function requireRuntime() {
  requireTree(DRIVER_TREE_VAR, PLAYWRIGHT_CORE_ENTRY);
  return {
    ACCELERATOR_PLAYWRIGHT_NS_ROOT: process.env[DRIVER_TREE_VAR],
    ACCELERATOR_DESIGN_BROWSER_EXECUTABLE: requireTree(
      BROWSER_TREE_VAR,
      CHROMIUM_SHELL
    ),
  };
}
