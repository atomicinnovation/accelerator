---
date: "2026-05-18T18:30:00+01:00"
type: plan
skill: create-plan
work-item: "meta/work/0072-playwright-daemon-cjs-import-bug.md"
status: approved
---

# 0072 — Playwright daemon CJS-import bug: implementation plan

## Overview

Fix the `inventory-design` Playwright daemon so that every runtime/hybrid
crawl stops failing with
`Cannot read properties of undefined (reading 'executablePath')`. The root
cause is that `importPlaywright()` selects the CJS entry of the `playwright`
package; dynamic `import()` then hides the `chromium` named export under
`default`. This plan extracts the loader into a sibling module so it can be
unit-tested in isolation, drives the fix with fixture-based tests, and
folds in the GHSA-7mvr-c777-76hp security bump (`playwright` → `~1.55.1`)
in the same PR (delivered as two commits).

## Current State Analysis

- `skills/design/inventory-design/scripts/playwright/lib/daemon.js:27-39`
  defines `importPlaywright()` as a module-local function. It hard-codes
  `entryFile = 'index.js'` and then trusts `pkg.main`, both of which point
  at the CJS shim of `playwright`. Result: the returned namespace has only
  `default`; `chromium`, `firefox`, `webkit` are `undefined`.
- Both call sites — `ensureBrowser()` at `daemon.js:117-123` and the `ping`
  handler at `daemon.js:132-152` — destructure `chromium` from the loader's
  return value and explode immediately.
- The `ping` handler at `daemon.js:140-150` *also* re-derives
  `resolve(nsRoot, 'node_modules/playwright/package.json')` to read the
  installed version. The nested-cache layout is therefore known to two
  places in the same module today — the candidate "second call site" for
  any extracted helper already exists.
- `daemon.js:29` (the no-`nsRoot` branch) already works correctly because
  Node's package resolver evaluates `pkg.exports['.'].import` for ESM
  callers (`package.json` declares `"type": "module"`). The two branches
  should agree; today they diverge.
- `run.sh:58/91/107` always exports `ACCELERATOR_PLAYWRIGHT_NS_ROOT`
  before launching the daemon, so production never takes the working
  branch.
- Test runner is `node:test` (no `npm test` script, no Makefile). Lib
  tests live next to their sources (`lib/<name>.js` + `lib/<name>.test.js`)
  and share fixtures via `lib/__fixtures__/`. Sibling modules use static
  top-of-file imports for `node:*` builtins (see `lib/state.js`,
  `lib/path-guard.js`, `lib/lock.js`); the current
  `importPlaywright()` is an outlier with its dynamic `await import('node:*')`
  style inherited from CJS-era code.
- The existing `ping` test (`lib/daemon.test.js:70-90`) does assert
  `res.ok === true`, but is gated behind `resolvePlaywrightNsRoot()` which
  returns `null` when the lockhash-namespaced cache is absent (CI lacks
  it). So the regression has been invisible in CI and would only surface
  when a developer with a populated cache happens to run that test.
- There is **no** isolated unit test of `importPlaywright()`. A test that
  builds a tiny fake `node_modules/playwright/` fixture and asserts
  `'chromium' in result` would have caught this and would not need a real
  Chromium install.
- `package.json:8` pins `playwright: ~1.49.0`. `package-lock.json` resolves
  `playwright@1.49.1`, which is covered by high-severity advisory
  GHSA-7mvr-c777-76hp ("Playwright downloads and installs browsers without
  verifying the authenticity of the SSL certificate"); fixed in `1.55.1`.
  The `.1` patch floor is load-bearing — 1.55.0 is below the fix line.
- Bumping the version changes `package-lock.json`, which changes the
  lockhash computed in `run.sh:34` and `ensure-playwright.sh:43`, which
  triggers a fresh `NS_ROOT` directory, fresh `npm ci`
  (`ensure-playwright.sh:336`), and fresh `npx playwright install
  chromium` (`ensure-playwright.sh:349`) on the next invocation.

## Desired End State

- `lib/playwright-loader.js` exports `importPlaywright({ nsRoot } = {})`
  and `resolvePlaywrightPkgPath(nsRoot)`. `nsRoot` is passed in by the
  caller, not read from `process.env` inside the loader. The loader
  prefers `pkg.exports['.'].import` and throws loudly if `exports['.']`
  declares conditions but no string `import` is reachable (avoiding the
  silent-fall-through-to-CJS regression mode the original bug exhibited).
  Static top-of-file imports for `node:*` builtins, matching sibling
  modules.
- `lib/daemon.js` imports both helpers from the loader and passes
  `process.env.ACCELERATOR_PLAYWRIGHT_NS_ROOT` explicitly at each call
  site. The `ping` handler reuses `resolvePlaywrightPkgPath` rather than
  re-deriving the nested-cache layout.
- `lib/playwright-loader.test.js` runs unconditionally (no skip guard).
  Three fixtures under `lib/__fixtures__/` exercise: (1) the primary
  CJS-vs-ESM regression — assert `'chromium' in result`; (2) the
  selection rule itself — distinct ESM markers in `index.mjs`
  vs `pkg.main` files, asserting the loader picked `exports['.'].import`
  over `pkg.main`; (3) a nested-conditions shape — assert the loader
  throws.
- `package.json` `playwright` range is `~1.55.1`; `package-lock.json`
  resolves both `playwright` AND `playwright-core` to `>= 1.55.1`;
  `npm audit --omit=dev` reports no high-severity advisories for the
  playwright dependency tree.
- After the bump, the installed `node_modules/playwright/package.json`
  `exports['.']` shape is confirmed to match the fixture (or the
  fixture is updated to match the installed shape).
- A manual `/accelerator:inventory-design <id> http://127.0.0.1:<port>/`
  run against a local dev server exercises navigate + accessibility
  snapshot + screenshot-with-mask + evaluate + click + type + wait_for
  commands without the destructure-undefined error and produces an
  inventory artifact end-to-end on the upgraded playwright version.

### Key Discoveries

- `lib/playwright-loader.js`/`playwright-loader.test.js` is the natural
  seam — sibling-module convention already in use (`auth-header.js`,
  `lock.js`, `path-guard.js`, etc., each with a `.test.js` peer).
- Fixture convention is established: `lib/__fixtures__/` already holds
  `proc-stat-linux.txt` and `ps-lstart-macos.txt` for sibling tests.
  New `__fixtures__/fake-playwright*/` directories will sit alongside.
- The exports-map shape (`exports['.'].import === './index.mjs'`) has
  held for playwright 1.49.1 (verified in the research doc). The plan
  re-verifies this shape against the installed 1.55.x package.json
  after lockfile regeneration rather than asserting cross-version
  invariance ahead of time.
- `run.sh` consults the lockhash, so changing `package-lock.json`
  automatically routes the daemon to a freshly bootstrapped cache; no
  manual `rm -rf` is needed. Old `1.49.x` cache directories remain
  inert under their prior lockhash — `ACCELERATOR_PLAYWRIGHT_SWEEP=1`
  removes them if desired.
- `package.json` has no `"scripts"` block, so automated checks must invoke
  `node --test` directly against the specific test files.

## What We're NOT Doing

- Strengthening `ping` to actually `launch()`/`close()` Chromium as a
  deeper health check. (Open Question 1 in the research; deferred.)
- Programmatic enforcement of the inventory-design crawl bounds (page
  cap, wall-clock, screenshot budget). Tracked separately.
- Touching the wider `inventory-design` skill, `run.sh`, or
  `ensure-playwright.sh`. The fix is confined to the daemon's loader,
  its tests, the playwright dependency, and the `ping` handler's reuse
  of the new helper.
- Forcing cleanup of prior lockhash cache directories.
  `ensure-playwright.sh` already supports this via
  `ACCELERATOR_PLAYWRIGHT_SWEEP=1`; the plan recommends a one-time run
  in Migration Notes but does not automate it.

## Implementation Approach

Drive the fix with TDD as a developer-loop discipline — write the test
first, observe RED locally, apply the fix, observe GREEN — then commit
the result as a **single atomic commit** containing the extract, the
tests, and the fix. The security bump lands in the **same PR** as a
**second commit** (separated for bisectability) because both edits
touch the adjacent `package.json` / `package-lock.json` pair and share
a single bootstrap cycle.

PR commit structure:

- **Commit 1** — loader extract + fixtures + tests + fix (this is the
  Phase 1 deliverable below).
- **Commit 2** — `playwright ~1.55.1` security bump and lockfile
  regeneration (Phase 2 below).

## Phase 1: Extract, test, and fix the loader

### Overview

A single atomic commit. The developer workflow is: (a) sketch the new
loader and tests with the test asserting the fixed behaviour; (b) make
the source apply the bug temporarily and confirm the tests go RED; (c)
apply the actual fix and confirm GREEN; (d) stage and commit the final
state. The repository never sees an intermediate commit containing a
known-broken loader.

### Changes Required

#### 1. New module — `lib/playwright-loader.js`

**File**: `skills/design/inventory-design/scripts/playwright/lib/playwright-loader.js`
**Changes**: New file exposing two named exports:
`importPlaywright({ nsRoot } = {})` and
`resolvePlaywrightPkgPath(nsRoot)`. Uses static top-of-file imports
matching sibling modules.

```js
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

// Loads the `playwright` package from a namespaced cache root when given,
// otherwise falls back to normal package resolution.
//
// We must select the ESM entry (advertised by pkg.exports['.'].import,
// typically ./index.mjs). pkg.main points at the CJS shim, and a dynamic
// import() of CJS wraps the entire module.exports under `default` — so
// the named exports chromium/firefox/webkit/devices/selectors/errors
// would all be undefined on the returned namespace.
//
// Trust boundary: `nsRoot` is treated as fully trusted. In production it
// is set by run.sh from a lockhash digest under
// $HOME/.cache/accelerator/playwright. The value drives an arbitrary
// import() call; any new caller must derive it from an equally trusted
// source.
//
// Layout contract: the path `node_modules/playwright/package.json`
// relative to `nsRoot` is owned by ensure-playwright.sh — keep in sync.

export function resolvePlaywrightPkgPath(nsRoot) {
  return resolve(nsRoot, 'node_modules/playwright/package.json');
}

export async function importPlaywright({ nsRoot } = {}) {
  if (!nsRoot) return import('playwright');

  const pkgPath = resolvePlaywrightPkgPath(nsRoot);
  let pkg;
  try {
    pkg = JSON.parse(readFileSync(pkgPath, 'utf8'));
  } catch (err) {
    if (err.code !== 'ENOENT') {
      throw new Error(
        `playwright-loader: failed to read ${pkgPath}: ${err.message}`
      );
    }
  }

  let entryFile = 'index.mjs';
  if (pkg) {
    const subpath = pkg.exports?.['.'];
    if (subpath !== undefined && subpath !== null && typeof subpath === 'object') {
      if (typeof subpath.import === 'string') {
        entryFile = subpath.import;
      } else {
        // exports['.'] declares conditions but no string `import` entry is
        // reachable (e.g. nested condition tree, removed condition). Fail
        // loudly rather than silently falling through to pkg.main (CJS) —
        // that fall-through is what produced the original bug.
        throw new Error(
          `playwright-loader: pkg.exports['.'].import is not a string at ${pkgPath}; ` +
          `the exports map shape may have changed. Update the loader.`
        );
      }
    } else if (typeof pkg.main === 'string') {
      entryFile = pkg.main;
    }
  }

  const entryUrl = pathToFileURL(
    resolve(nsRoot, 'node_modules/playwright', entryFile)
  ).href;
  return import(entryUrl);
}
```

#### 2. Update `lib/daemon.js`

**File**: `skills/design/inventory-design/scripts/playwright/lib/daemon.js`
**Changes**:
- Delete the existing `importPlaywright` function and its preceding
  comment block (lines 24-39).
- Add `import { importPlaywright, resolvePlaywrightPkgPath } from './playwright-loader.js';`
  near the top of the file, alongside the other relative imports.
- Both daemon call sites pass `nsRoot` explicitly. `ensureBrowser()`
  becomes (roughly):

  ```js
  const { chromium } = await importPlaywright({
    nsRoot: process.env.ACCELERATOR_PLAYWRIGHT_NS_ROOT,
  });
  ```

- The `ping` handler (`daemon.js:132-152`) passes the same `nsRoot` to
  `importPlaywright` and replaces its hand-rolled
  `resolve(nsRoot, 'node_modules/playwright/package.json')` with
  `resolvePlaywrightPkgPath(nsRoot)`. The no-`nsRoot` branch in the
  ping handler (`new URL('../node_modules/playwright/package.json', import.meta.url).pathname`)
  is preserved as-is.

#### 3. New fixtures under `lib/__fixtures__/`

Three fixtures, each laid out so the fixture directory itself is a
valid `nsRoot` (the loader joins `nsRoot` with
`node_modules/playwright/package.json`).

**3a.** `lib/__fixtures__/fake-playwright/node_modules/playwright/` — primary
regression fixture. CJS `index.js` deliberately omits `chromium` so an
accidental CJS load is distinguishable from a correct ESM load.

- `package.json`:

  ```json
  {
    "name": "playwright",
    "version": "0.0.0-fixture",
    "main": "index.js",
    "exports": {
      ".": {
        "import": "./index.mjs",
        "require": "./index.js",
        "default": "./index.js"
      }
    }
  }
  ```

- `index.mjs`:

  ```js
  export const chromium = {
    launch: async () => ({ close: async () => {} }),
    executablePath: () => '/fake/chromium',
  };
  ```

- `index.js`:

  ```js
  'use strict';
  module.exports = { __cjsMarker: true };
  ```

**3b.** `lib/__fixtures__/fake-playwright-distinct-entries/node_modules/playwright/`
— selection-rule fixture. BOTH entries are ESM with distinct named
markers, so the test directly asserts which file the loader selected
rather than relying on the emergent CJS-vs-ESM difference.

- `package.json`:

  ```json
  {
    "name": "playwright",
    "version": "0.0.0-fixture",
    "main": "main-entry.mjs",
    "exports": {
      ".": {
        "import": "./esm-entry.mjs",
        "default": "./main-entry.mjs"
      }
    }
  }
  ```

- `esm-entry.mjs`: `export const __selected = 'exports-import';`
- `main-entry.mjs`: `export const __selected = 'pkg-main';`

**3c.** `lib/__fixtures__/fake-playwright-nested-conditions/node_modules/playwright/`
— shape-change fixture. `pkg.exports['.'].import` is an object, not a
string; the loader must throw.

- `package.json`:

  ```json
  {
    "name": "playwright",
    "version": "0.0.0-fixture",
    "main": "index.js",
    "exports": {
      ".": {
        "import": {
          "node": "./index.mjs",
          "default": "./index.mjs"
        }
      }
    }
  }
  ```

- `index.mjs`: empty file (never reached because the loader throws).
- `index.js`: empty file (never reached).

#### 4. New test file — `lib/playwright-loader.test.js`

**File**: `skills/design/inventory-design/scripts/playwright/lib/playwright-loader.test.js`
**Changes**: New file using `node:test`, matching the existing lib-test
style. The test computes each fixture's `nsRoot` from `import.meta.url`
and passes it to `importPlaywright` as a parameter — no `process.env`
mutation.

```js
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
```

The first test is the primary regression guard. The second test
directly verifies the selection rule. The third pins down the loud-
failure behaviour so a future change that silently re-introduces
`pkg.main` fall-through breaks the test.

### Success Criteria

#### Automated Verification

- [x] Loader module exists: `test -f skills/design/inventory-design/scripts/playwright/lib/playwright-loader.js`
- [x] All three fixtures exist with their files:
      `test -f skills/design/inventory-design/scripts/playwright/lib/__fixtures__/fake-playwright/node_modules/playwright/{package.json,index.mjs,index.js}`
      and equivalents for `fake-playwright-distinct-entries/` and
      `fake-playwright-nested-conditions/`.
- [x] `lib/daemon.js` no longer declares `importPlaywright` locally:
      `! grep -q 'function importPlaywright' skills/design/inventory-design/scripts/playwright/lib/daemon.js`
- [x] `lib/daemon.js` imports the loader (fixed-string match to avoid
      regex pitfalls):
      `grep -qF "from './playwright-loader.js'" skills/design/inventory-design/scripts/playwright/lib/daemon.js`
- [x] Loader prefers the ESM entry (fixed-string match for the literal
      optional-chaining operator):
      `grep -qF "pkg.exports?.['.']" skills/design/inventory-design/scripts/playwright/lib/playwright-loader.js`
- [x] All three new tests pass:
      `node --test skills/design/inventory-design/scripts/playwright/lib/playwright-loader.test.js`
      exits 0 with 3 tests passing.
- [x] All other lib tests pass or skip identically to pre-change baseline:
      `node --test skills/design/inventory-design/scripts/playwright/lib/*.test.js`.
- [x] Shell-level structural harness still passes:
      `bash skills/design/inventory-design/scripts/playwright/test-run.sh`
      (skipping playwright-dependent checks gracefully if the cache is
      absent — matches pre-change behaviour).

#### Manual Verification (developer-loop TDD)

Performed locally before committing; not preserved in the commit
history. The single atomic commit contains the final fixed state.

- [ ] Temporarily revert the loader fix (e.g. `entryFile = pkg.main`
      branch only) and run the new test file: all three tests should
      fail. This confirms the tests exercise the bug.
- [ ] Re-apply the fix and run again: all three tests pass.
- [ ] Inspect the diff: loader module is the new file, daemon.js drops
      the local function and gains two imports, three fixtures plus the
      test file are added.

---

## Phase 2: Bump playwright to `~1.55.1` (security)

### Overview

Close GHSA-7mvr-c777-76hp by bumping the pinned playwright range out of
`1.49.x`. Regenerate the lockfile. Re-bootstrap the daemon's
namespaced cache. Confirm both the loader's fix and the daemon's
behaviour hold on the new version. Lands as a separate commit within
the same PR.

### Changes Required

#### 1. Update `package.json`

**File**: `skills/design/inventory-design/scripts/playwright/package.json`
**Changes**: Change `"playwright": "~1.49.0"` to `"playwright": "~1.55.1"`.
Include a one-line context note in the commit message (and optionally
as a brief `// ` comment near the dependency or in the commit body)
that the `.1` patch floor is load-bearing: 1.55.0 is below the
GHSA-7mvr-c777-76hp fix line.

#### 2. Regenerate `package-lock.json`

**File**: `skills/design/inventory-design/scripts/playwright/package-lock.json`
**Changes**: Regenerated by running, in that directory:

```bash
npm install --package-lock-only
```

(Or equivalently `npm install` if the local `node_modules/` is desired
for development convenience — but the runtime cache lives under the
lockhash-namespaced directory, not next to `package.json`, so
`--package-lock-only` is enough to ship the change.)

Confirm the resolved version is `>= 1.55.1` for both `playwright` and
the transitive `playwright-core`.

#### 3. Confirm installed exports shape matches the fixture

**Action**: After lockfile regeneration but before commit, inspect the
installed playwright's `package.json` to confirm its `exports['.']`
shape still matches what the primary fixture assumes:

```bash
NS=$(bash -c 'source skills/design/inventory-design/scripts/playwright/run.sh status' 2>/dev/null | awk '/ns_root/{print $2}')
# Or after first invocation triggers bootstrap, find under ~/.cache/accelerator/playwright/
jq '.exports."."' "$NS/node_modules/playwright/package.json"
```

If `exports['.']` exposes a string `import` entry, no fixture change is
needed. If the shape has changed (e.g. nested conditions, string
shorthand), update the loader and the fixtures accordingly before
proceeding.

#### 4. Rebootstrap the namespaced cache (developer environment)

**Action**: Run any inventory-design command that invokes `run.sh` —
the new lockhash will be missing from `~/.cache/accelerator/playwright/`,
which triggers `ensure-playwright.sh:336` (`npm ci`) and
`ensure-playwright.sh:349` (`npx playwright install chromium`).
No code change; this is bookkeeping the developer does locally
before running the acceptance step in Phase 3.

### Success Criteria

#### Automated Verification

- [x] `package.json` carries the new range:
      `grep -qF '"playwright": "~1.55.1"' skills/design/inventory-design/scripts/playwright/package.json`
- [x] Lockfile resolves `playwright` `>= 1.55.1` (numeric semver
      compare, not lexicographic):
      ```bash
      node -e '
        const p = require("./skills/design/inventory-design/scripts/playwright/package-lock.json");
        const cmp = (a, b) => {
          const ap = a.split(".").map(Number), bp = b.split(".").map(Number);
          for (let i = 0; i < 3; i++) if (ap[i] !== bp[i]) return ap[i] - bp[i];
          return 0;
        };
        for (const dep of ["node_modules/playwright", "node_modules/playwright-core"]) {
          const v = p.packages[dep].version;
          if (cmp(v, "1.55.1") < 0) { console.error(dep + " is " + v + ", need >= 1.55.1"); process.exit(1); }
        }
      '
      ```
- [x] Audit reports no `playwright` / `playwright-core` advisories:
      `cd skills/design/inventory-design/scripts/playwright && npm audit --omit=dev` (no
      high-severity GHSA-7mvr-c777-76hp; other dev-only advisories
      acceptable).
- [x] Loader unit tests still pass against the fixtures (fixtures are
      version-agnostic):
      `node --test skills/design/inventory-design/scripts/playwright/lib/playwright-loader.test.js`.
- [x] **Gating** — after rebootstrap, the daemon test passes against
      the new cache:
      `node --test skills/design/inventory-design/scripts/playwright/lib/daemon.test.js`.
      This is a hard gate, not descriptive: Phase 2 is not complete
      until this test exits zero on a populated cache.

#### Manual Verification

- [x] Lockfile diff review: only `playwright`, `playwright-core`, and
      their transitive deps change; no unexpected dependency churn.
- [x] Spot-check `package-lock.json` for the updated entries: `resolved`
      URLs point at the configured registry (typically
      `https://registry.npmjs.org/`) and `integrity` fields are present
      for both `playwright` and `playwright-core`. Cross-reference one
      hash against the registry's published value via
      `npm view playwright@<resolved-version> dist.integrity`.
- [x] Installed `node_modules/playwright/package.json` `exports['.']`
      shape matches the primary fixture (Step 3 above). If divergent,
      reconcile before continuing.
- [x] Bootstrap log shows a fresh `npm ci` and `npx playwright install
      chromium` succeeding for the new lockhash.

---

## Phase 3: End-to-end acceptance

### Overview

Confirm the full `/accelerator:inventory-design` flow produces an
inventory artifact against a real local dev server on the upgraded
playwright version, exercising the daemon commands actually used by the
skill — not just the happy-path navigate. This is the closing
acceptance criterion from the work item and the integration-level
guard against playwright API drift across the 1.49 → 1.55 range.

### Changes Required

No code changes. This phase is execution-only:

1. Start any local dev server reachable at `http://127.0.0.1:<port>/`,
   ideally one whose pages include interactive controls (a button to
   click, an input to type into, an element that appears after a
   delay).
2. From inside the workspace, run
   `/accelerator:inventory-design <source-id> http://127.0.0.1:<port>/`.
3. Observe Step 5 (Confirm Executor Liveness): the daemon's `ping`
   must return `{"protocol":1,"ok":true,…}` with `chromium` populated.
4. Allow the crawl to complete and write an inventory artifact under
   the configured inventory directory.

### Success Criteria

#### Manual Verification

- [x] The `ping` step succeeds with `"ok":true`.
- [x] An inventory artifact is written for the supplied source-id.
- [x] No `Cannot read properties of undefined (reading 'executablePath')`
      error appears in the session log.
- [x] The crawl exercises and completes the following daemon commands
      without error (verify via the session diagnostic log):
      - `navigate` (page load + `waitUntil: 'domcontentloaded'`)
      - `snapshot` (accessibility snapshot; verify the structure is
        non-empty)
      - `screenshot` with at least one `mask` array (full-page on
        primary, masked on a sensitive region)
      - `evaluate` (any inline expression result returned)
      - `click` on a ref captured from the snapshot
      - `type` on an input ref
      - `wait_for` against a text or element selector
- [x] Repeating the run against a second local URL on the same server
      reuses the (already-bootstrapped) namespaced cache without
      re-installing chromium.

---

## Testing Strategy

### Unit Tests

- `lib/playwright-loader.test.js` — three fixture-based cases:
  1. **Regression**: `'chromium' in result` against the
     `fake-playwright/` fixture (ESM + CJS-without-chromium). Primary
     guard against re-introducing the original bug.
  2. **Selection rule**: `result.__selected === 'exports-import'`
     against the `fake-playwright-distinct-entries/` fixture (two ESM
     files with distinct markers). Directly asserts the loader picks
     `exports['.'].import` over `pkg.main` rather than relying on
     CJS-vs-ESM emergent behaviour.
  3. **Shape-change loud failure**: `assert.rejects` against the
     `fake-playwright-nested-conditions/` fixture. Pins down the
     loud-failure behaviour so a future change that silently restores
     `pkg.main` fall-through breaks the test.

  All tests pass `nsRoot` as a parameter — no `process.env` mutation.
  All run in milliseconds with no real Chromium dependency, no
  `existsSync` skip.

### Integration Tests

- Existing `lib/daemon.test.js` — its `ping` test will now pass in
  environments with a populated namespaced cache. **Gating in Phase 2**
  after rebootstrap. CI still lacks the cache so this only adds value
  on developer machines; the loader unit tests + the Phase 3 manual
  acceptance are the cross-environment coverage.
- Existing `test-run.js` / `test-run.sh` — continue to gate on
  `existsSync(cacheRoot)`. Out of scope to tighten their skip guards.

### Manual Testing Steps

1. Developer-loop TDD verification before committing Phase 1:
   temporarily revert the loader fix, observe the new tests fail, then
   re-apply the fix. (Done locally; not preserved in commit history.)
2. After Phase 2 lockfile regeneration, confirm installed playwright
   `exports['.']` shape matches the primary fixture.
3. Phase 3 end-to-end acceptance against a real dev server, exercising
   every daemon command listed above.

## Performance Considerations

None. The fix removes a runtime failure; it does not add a new code
path. The ESM `index.mjs` re-exports the same `playwright-core` API
as the CJS `index.js` shim, so command latency is unchanged. The
playwright version bump (`1.49 → 1.55+`) is verified manually in
Phase 3 against every daemon command rather than asserted as
universally API-stable.

## Migration Notes

- The lockhash-namespaced cache strategy means the version bump
  automatically produces a fresh cache directory without any cleanup.
  Prior `1.49.x` caches remain on disk under their old lockhash; they
  are inert (nothing references them) but still contain the vulnerable
  installer.
- **Recommended cleanup**: developers should run with
  `ACCELERATOR_PLAYWRIGHT_SWEEP=1` once after rebootstrapping the new
  cache to remove the old `1.49.x` cache directory and its bundled
  installer. Not enforced; documented here so the dormant artefact
  doesn't outlive its usefulness.
- No data migration. No skill behaviour change other than "runtime/
  hybrid crawls work again".

## References

- Work item: `meta/work/0072-playwright-daemon-cjs-import-bug.md`
- Research: `meta/research/codebase/2026-05-18-0072-playwright-daemon-cjs-import-bug.md`
- Background research:
  `meta/research/codebase/2026-05-06-design-skill-localhost-and-mcp-issues.md`
- Background plan:
  `meta/plans/2026-05-06-design-skill-localhost-and-mcp-issues.md`
- Advisory: GHSA-7mvr-c777-76hp
  (https://github.com/advisories/GHSA-7mvr-c777-76hp)
- Bug seam:
  `skills/design/inventory-design/scripts/playwright/lib/daemon.js:27-39`
- Second call site for the nested-cache layout:
  `skills/design/inventory-design/scripts/playwright/lib/daemon.js:140-150`
- Production NS_ROOT injection:
  `skills/design/inventory-design/scripts/playwright/run.sh:58,91,107`
- Bootstrap install:
  `skills/design/inventory-design/scripts/ensure-playwright.sh:336,349`
- Fixture convention precedent:
  `skills/design/inventory-design/scripts/playwright/lib/__fixtures__/`
