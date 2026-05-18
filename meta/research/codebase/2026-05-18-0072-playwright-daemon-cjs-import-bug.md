---
date: 2026-05-18T18:56:43+01:00
researcher: Toby Clemson
git_commit: ee201256c147a4f4e8b8f7427f9292fd79925767
branch: HEAD
repository: accelerator
topic: "inventory-design Playwright daemon — `importPlaywright()` loads CJS entry and loses `chromium` named export"
tags: [research, codebase, inventory-design, playwright, daemon, esm, cjs, bug]
status: complete
last_updated: 2026-05-18
last_updated_by: Toby Clemson
---

# Research: inventory-design Playwright daemon — `importPlaywright()` loads CJS entry and loses `chromium` named export

**Date**: 2026-05-18T18:56:43+01:00
**Researcher**: Toby Clemson
**Git Commit**: ee201256c147a4f4e8b8f7427f9292fd79925767
**Branch**: HEAD
**Repository**: accelerator

## Research Question

`/accelerator:inventory-design` against a `http://127.0.0.1` URL fails on every `ping` with:

```
{"protocol":1,"error":"internal-error",
 "message":"Cannot read properties of undefined (reading 'executablePath')",
 "category":"browser","retryable":false}
```

Rebootstrapping the Playwright namespace cache does not help. Where is the
defect, what is the smallest correct fix, and why did our test suite not
catch it?

A full empirical diagnostic captured during the session is at
`/tmp/inventory-design-debug.log` (local to the investigating machine; the
substantive content is reproduced in this document).

## Summary

The defect is in `skills/design/inventory-design/scripts/playwright/lib/daemon.js`
line 32. `importPlaywright()` selects `pkg.main` (`./index.js`, CommonJS)
from the playwright package manifest and feeds the resulting `file://` URL to
a dynamic `import()`. Node's ESM-importing-CJS interop wraps the entire CJS
module as `{ default: <module.exports> }` and does **not** propagate the
named exports — so `chromium`, `firefox`, `webkit`, `devices`, `selectors`
and `errors` are all `undefined` on the returned namespace. The two
destructuring call sites (`ensureBrowser()` at line 119 and the `ping`
handler at line 133) both do `const { chromium } = await importPlaywright()`,
get `undefined`, and explode on the first `chromium.…` call.

The fix is a five-line change in the same function: prefer
`pkg.exports['.'].import` (which resolves to `./index.mjs`, the genuine ESM
entry) and fall back to `pkg.main` only when no ESM entry is declared. No
caller needs to change. Verified empirically by re-running the same dynamic
`import()` against `index.mjs` — it returns
`['chromium','firefox','webkit', …]` as expected.

The bug went undetected because every test that would have exercised
`ensureBrowser()` is gated on the presence of the bootstrapped cache
directory (`existsSync(cacheRoot)` in `test-run.js:94`, equivalent guard in
`daemon.test.js:25`). CI lacks that directory, so the entire integration
suite is skipped. Even when the cache exists, the `ping` test only asserts
`typeof res.chromium === 'string'`, but never calls anything on the chromium
object — and it would also throw if it did, because the same destructure
hits `undefined`. There is no isolated unit test of `importPlaywright()`
that would assert `'chromium' in result`.

A separate, lower-severity issue was incidentally surfaced during the
rebootstrap: `playwright@1.49.1` (currently pinned) has a high-severity
advisory **GHSA-7mvr-c777-76hp** ("Playwright downloads and installs
browsers without verifying the authenticity of the SSL certificate"), fixed
in `1.55.1`. Bumping out of the `~1.49.0` range fixes the advisory and is
strictly safer than keeping the current pin. This is recommended but
strictly orthogonal to the import bug — the import bug exists at every
playwright version on this code path.

## Detailed Findings

### The defect

`skills/design/inventory-design/scripts/playwright/lib/daemon.js:24-39`:

```js
// ESM import() does not honour NODE_PATH. When run via run.sh the daemon
// receives ACCELERATOR_PLAYWRIGHT_NS_ROOT (the namespaced cache dir) and we
// construct a file:// URL so the import resolver can find playwright.
async function importPlaywright() {
  const nsRoot = process.env.ACCELERATOR_PLAYWRIGHT_NS_ROOT;
  if (!nsRoot) return import('playwright');
  const { resolve } = await import('node:path');
  const { pathToFileURL } = await import('node:url');
  let entryFile = 'index.js';                                     // <-- CJS
  try {
    const { readFileSync } = await import('node:fs');
    const pkg = JSON.parse(readFileSync(resolve(nsRoot, 'node_modules/playwright/package.json'), 'utf8'));
    if (typeof pkg.main === 'string') entryFile = pkg.main;       // <-- still CJS
  } catch {}
  return import(pathToFileURL(resolve(nsRoot, 'node_modules/playwright', entryFile)).href);
}
```

`playwright@1.49.1`'s manifest:

```json
"main": "index.js",
"exports": {
  ".": {
    "types": "./index.d.ts",
    "import": "./index.mjs",
    "require": "./index.js",
    "default": "./index.js"
  },
  …
}
```

`pkg.main` is the CommonJS entry. When the daemon hands a `file://…/index.js`
URL to dynamic `import()`, Node loads the CJS module via its CJS-interop
shim and returns `{ default: <module.exports> }`. Named exports are
**not** unwrapped from the CJS exports object — that is correct
Node behaviour and is why the package ships a separate `index.mjs` (whose
own contents re-export `chromium`/`firefox`/`webkit` as proper ESM bindings).

Empirically demonstrated in `/tmp/inventory-design-debug.log:91-110`:

| Entry imported                  | `Object.keys(m)` (first few)          | `m.chromium` |
| ------------------------------- | ------------------------------------- | ------------ |
| `…/playwright/index.js` (CJS)   | `['default']`                         | `undefined`  |
| `…/playwright/index.mjs` (ESM)  | `['_android','_electron','chromium',…]` | `object`     |

### Call sites that break

Both call sites destructure `chromium` from the unwrapped namespace.

**`ensureBrowser()` at `daemon.js:117-123`** — invoked the first time any
browser-touching command (`navigate`, `snapshot`, `screenshot`, `evaluate`,
`click`, `type`, `wait_for`) arrives:

```js
async function ensureBrowser() {
  if (browser) return;
  const { chromium } = await importPlaywright();
  browser = await chromium.launch({ headless: true });
  …
}
```

**`ping` handler at `daemon.js:132-152`** — used by the launcher and tests
to confirm the daemon can reach a real browser binary:

```js
if (cmd === 'ping') {
  const { chromium: cr } = await importPlaywright();
  const execPath = cr.executablePath();
  …
}
```

No other call site exists. Neither path uses `firefox`, `webkit`,
`devices`, `selectors`, or `errors`, so the fix can be confined to making
`chromium` available as a named export.

### Smallest correct fix

Replace lines 27–39 with:

```js
async function importPlaywright() {
  const nsRoot = process.env.ACCELERATOR_PLAYWRIGHT_NS_ROOT;
  if (!nsRoot) return import('playwright');
  const { resolve } = await import('node:path');
  const { pathToFileURL } = await import('node:url');
  // Prefer the ESM entry advertised by pkg.exports['.'].import.
  // pkg.main is the CJS shim; dynamic import() of CJS hides named
  // exports under `default` (chromium/firefox/webkit become undefined).
  // index.mjs re-exports them as proper ESM bindings.
  let entryFile = 'index.mjs';
  try {
    const { readFileSync } = await import('node:fs');
    const pkg = JSON.parse(readFileSync(resolve(nsRoot, 'node_modules/playwright/package.json'), 'utf8'));
    const esm = pkg?.exports?.['.']?.import;
    if (typeof esm === 'string') entryFile = esm;
    else if (typeof pkg.main === 'string') entryFile = pkg.main;
  } catch {}
  return import(pathToFileURL(resolve(nsRoot, 'node_modules/playwright', entryFile)).href);
}
```

Rationale: this restores the same resolution Node would do for the
bare-specifier branch (`import('playwright')` at line 29 already resolves
via `pkg.exports['.'].import` because `daemon.js` is ESM —
`package.json:5` declares `"type": "module"`). The `nsRoot` branch becomes
behaviourally consistent with the no-`nsRoot` branch instead of diverging
from it. The default-entry guess flips from `index.js` to `index.mjs` so
even if a future playwright drops `exports['.'].import` we'd still grab the
ESM file when present.

### Why the bare-specifier branch (line 29) works fine

Line 29 — `if (!nsRoot) return import('playwright');` — uses Node's normal
package resolution, which consults `pkg.exports['.']` and matches the
`import` condition because the importer (`daemon.js`) is an ESM module.
That branch is exercised by tests that fork `run.js` directly without
setting `ACCELERATOR_PLAYWRIGHT_NS_ROOT` (`daemon.test.js:96-101`, similar
patterns at lines 117, 138, 161). It works correctly today.

In production, however, `run.sh` always sets `ACCELERATOR_PLAYWRIGHT_NS_ROOT`
to the namespaced cache directory before launching the daemon (`run.sh:28-37`)
— so the buggy `nsRoot` branch is the only one users ever hit. The
asymmetry between the two branches is the load-bearing observation: the
working branch already does what the broken branch should be doing.

### Why the test suite did not catch this

Every browser-launching test is skipped without the bootstrapped cache.

- `daemon.test.js:15-27` — `resolvePlaywrightNsRoot()` returns `null` if the
  cache `node_modules/playwright/index.js` is absent. Tests that depend on
  this skip themselves silently.
- `daemon.test.js:70-90` — the `ping` test runs the **real** daemon and
  calls `ping`, but only asserts `typeof res.chromium === 'string'` (line
  85). That string is `cr.executablePath()` which would also throw on the
  same `undefined` destructure — meaning even this "passing" test would
  fail in a clean local environment with the cache present. It hasn't
  been observed failing, which suggests either (a) it hasn't been run
  recently against an actual cache, or (b) someone has the cache but
  the test was previously passing on a Playwright version where `index.js`
  did expose named exports (true for some older versions).
- `test-run.js:94` — `skip: !playwrightInstalled` where
  `playwrightInstalled` is `existsSync(cacheRoot)`. CI lacks the cache,
  so every integration test (`navigate`, `snapshot`, `screenshot`, …) is
  skipped.
- There is **no isolated unit test for `importPlaywright()`** that asserts
  the return value has named exports. A one-line assertion
  (`assert.ok('chromium' in result)`) would have failed loudly.

### Lockfile and dependency surface

`skills/design/inventory-design/scripts/playwright/package.json:1-13`:

```json
"type": "module",
"engines": { "node": ">=20" },
"dependencies": { "playwright": "~1.49.0" },
"devDependencies": { "pngjs": "^7.0.0" }
```

`skills/design/inventory-design/scripts/playwright/package-lock.json` pins
`playwright@1.49.1` and the transitive `playwright-core@1.49.1`. The
`~1.49.0` range will float forward to `1.49.x` only; bumping out of
`1.49.x` requires editing `package.json` and regenerating the lockfile
(which is what the bootstrap log shows `npm audit fix --force` would do —
to `1.60.0`).

### Incidental: GHSA-7mvr-c777-76hp (high)

Captured at `/tmp/inventory-design-debug.log:159-166`:

> playwright <1.55.1 — Playwright downloads and installs browsers without
> verifying the authenticity of the SSL certificate.

This is a property of the installer (the `npx playwright install chromium`
step in `ensure-playwright.sh:349`), not the runtime. We download via that
exact path. Bumping `playwright` to `>=1.55.1` closes the advisory without
affecting the import-bug fix one way or the other; it is a separate
hardening change worth doing in the same PR or a successor.

## Code References

- `skills/design/inventory-design/scripts/playwright/lib/daemon.js:27-39` — the buggy function
- `skills/design/inventory-design/scripts/playwright/lib/daemon.js:117-123` — `ensureBrowser()` call site
- `skills/design/inventory-design/scripts/playwright/lib/daemon.js:132-152` — `ping` handler call site
- `skills/design/inventory-design/scripts/playwright/run.sh:28-37` — sets `ACCELERATOR_PLAYWRIGHT_NS_ROOT` in production
- `skills/design/inventory-design/scripts/playwright/lib/daemon.test.js:15-27, 70-90` — `ping` test plus skip guard
- `skills/design/inventory-design/scripts/playwright/test-run.js:94, 98-115, 137-181` — integration tests, all gated on `existsSync(cacheRoot)`
- `skills/design/inventory-design/scripts/playwright/package.json:1-13` — Node `>=20`, `"type": "module"`, `playwright ~1.49.0`
- `skills/design/inventory-design/scripts/playwright/package-lock.json` — pins `playwright@1.49.1`, transitive `playwright-core@1.49.1`
- `skills/design/inventory-design/scripts/ensure-playwright.sh:349` — `npx playwright install chromium` (the path covered by the SSL advisory)

## Architecture Insights

- **The two branches of `importPlaywright()` should agree.** The
  no-`nsRoot` branch (line 29) relies on Node's package resolution, which
  picks the ESM entry via `exports['.'].import`. The `nsRoot` branch
  bypasses package resolution and replicates it manually — but the manual
  replication picks the CJS entry. Any future divergence between the two
  branches is a smell; the fix makes them match.
- **Manual `pkg.main` resolution is generally wrong for modern packages.**
  Any package with `"type": "module"` peer or with a conditional
  `exports['.']` map needs the conditions evaluated, not `pkg.main`
  read literally. Today only `playwright` is imported this way; if we
  add more nsRoot-style dynamic imports later, lifting a small helper
  out (`resolveEsmEntry(pkgJson, conditions = ['import','default'])`)
  would prevent the same bug from being re-introduced.
- **Skip-by-default integration tests hide bootstrap regressions.** The
  `existsSync(cacheRoot)` guard is reasonable for CI portability, but it
  means the only environment that ever exercises `ensureBrowser()` is the
  developer's laptop on the days they happen to have the cache populated.
  A cheap mitigation: add a `importPlaywright`-level unit test that
  builds a tiny fake `node_modules/playwright/` fixture (a real
  `index.mjs` exporting a stub `chromium`, a stub `package.json` with the
  exports map) and asserts the function returns an object where
  `'chromium' in result` and `typeof result.chromium.launch === 'function'`.
  That test runs in seconds, has no Chromium dependency, and would have
  caught this regression at PR time.
- **The `ping` command's "is chromium reachable?" check has a gap.**
  `ping` reads `chromium.executablePath()` and `fs.access()`s the result.
  That asserts the *binary* exists, but doesn't assert
  `chromium.launch()` works. Worth considering whether `ping` should
  perform a no-op `chromium.launch().close()` to give a stronger health
  signal — though only if the latency budget allows.

## Historical Context

- `meta/research/codebase/2026-05-06-design-skill-localhost-and-mcp-issues.md`
  — research note that explicitly chose "Option B: Bash-based Playwright
  executor" to sidestep the MCP sub-agent inheritance bug class. The
  `run.sh` / `run.js` / `daemon.js` machinery being debugged here is the
  realisation of that decision. The same doc's "Open Questions" #1 asked
  what the executor protocol shape should be — `importPlaywright()` is one
  of the seams that question landed on, but the choice between CJS and
  ESM resolution was never discussed in the followup plan.
- `meta/plans/2026-05-06-design-skill-localhost-and-mcp-issues.md`
  — plan that implemented the executor; lays out the namespaced cache
  (`~/.cache/accelerator/playwright/<sha8>/`) and the bootstrap script.
  The plan does not mention `exports` map handling or the CJS/ESM
  distinction.
- `meta/plans/2026-05-03-design-convergence-workflow.md` — original
  inventory-design plan, predates the Bash-executor decision.
- No ADR covers the Playwright daemon, the namespaced cache strategy, or
  ESM/CJS module loading in plugin scripts. This bug-fix work could
  motivate one ADR-sized question: *how should plugin scripts that load
  npm packages from namespaced caches resolve entry points?* — but that's
  a future concern, not a blocker for the fix.

## Related Research

- [`meta/research/codebase/2026-05-06-design-skill-localhost-and-mcp-issues.md`](../codebase/2026-05-06-design-skill-localhost-and-mcp-issues.md)
- [`meta/research/codebase/2026-05-02-design-convergence-workflow.md`](../codebase/2026-05-02-design-convergence-workflow.md)

## Open Questions

1. **Should `ping` actually launch the browser briefly?** The current
   `ping` reads `executablePath()` and `fs.access()`s the file but does
   not call `chromium.launch()`. A no-op launch+close would catch
   environment failures (e.g. missing libnss) that pure file-existence
   does not. Worth evaluating against the per-op wall-clock budget.
2. **Add a fixture-based unit test for `importPlaywright()`?** Build a
   minimal fake `node_modules/playwright/` (with `index.mjs`, `index.js`,
   and a `package.json` carrying the exports map) under
   `__fixtures__/fake-playwright/`, then assert `'chromium' in result`
   and `typeof result.chromium.launch === 'function'`. Runs in
   milliseconds with no Chromium dependency. This is the highest-leverage
   regression guard for this class of bug.
3. **Bump playwright out of `~1.49.0` to ≥`1.55.1` to close
   GHSA-7mvr-c777-76hp?** Same PR as the import fix, or a successor?
   Recommend same PR — both touch the same lockfile and the advisory is
   high-severity.
4. **Extract a `resolveEsmEntry()` helper?** Premature today (one call
   site), but worth signposting in a comment as the obvious refactor if
   any second site appears.
