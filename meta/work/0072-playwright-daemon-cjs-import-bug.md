---
work_item_id: "0072"
title: "Playwright daemon imports CJS entry, hiding chromium named export"
date: "2026-05-18T17:56:43+00:00"
author: Toby Clemson
type: bug
status: draft
priority: high
parent: ""
tags: [bug, inventory-design, playwright, daemon, esm, security]
---

# 0072: Playwright daemon imports CJS entry, hiding chromium named export

**Type**: Bug
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Every runtime/hybrid `inventory-design` crawl fails immediately because the
Playwright executor's daemon imports the CommonJS entry of the `playwright`
package via dynamic ESM `import()`, which hides the named exports
(`chromium`, `firefox`, `webkit`) under `default`. The first `ping` returns
`Cannot read properties of undefined (reading 'executablePath')` and every
browser-launching command (`navigate`, `snapshot`, `screenshot`, …) hits the
same destructure-of-undefined. The fix is a five-line change in
`importPlaywright()` to prefer the package's declared ESM entry. While we are
in the file, add the isolated regression test that would have caught this and
bump the pinned playwright version to close the high-severity SSL-verification
advisory GHSA-7mvr-c777-76hp surfaced during diagnosis.

## Context

Discovered during a `/accelerator:inventory-design current-app
http://127.0.0.1:52705/` session on 2026-05-18. Re-bootstrapping the
namespaced Playwright cache does not help; the defect is in the daemon code,
not the install. Full root-cause analysis with empirical verification and a
proposed minimal patch is in
[`meta/research/codebase/2026-05-18-0072-playwright-daemon-cjs-import-bug.md`](../research/codebase/2026-05-18-0072-playwright-daemon-cjs-import-bug.md);
session diagnostic at `/tmp/inventory-design-debug.log` on the investigating
machine.

The Bash-based Playwright executor exists because of an earlier decision (see
[`meta/research/codebase/2026-05-06-design-skill-localhost-and-mcp-issues.md`](../research/codebase/2026-05-06-design-skill-localhost-and-mcp-issues.md))
to route around the Claude Code sub-agent MCP-inheritance bug class. Today
the executor is the only path for runtime/hybrid crawls — this bug therefore
blocks the entire feature for every user.

## Requirements

### Reproduction

1. Run `/accelerator:inventory-design <source-id> http://127.0.0.1:<port>/`
   against any running web app on localhost.
2. The skill bootstraps Playwright successfully and pings the daemon.
3. The daemon's ping handler at
   `skills/design/inventory-design/scripts/playwright/lib/daemon.js:133`
   destructures `chromium` from `importPlaywright()`. `chromium` is
   `undefined` because the daemon loaded `playwright/index.js` (CJS) and
   Node's ESM/CJS interop wraps named exports under `default`.
4. The handler returns
   `{"protocol":1,"error":"internal-error","message":"Cannot read properties
   of undefined (reading 'executablePath')","category":"browser","retryable":false}`.
5. The skill's Step 5 (Confirm Executor Liveness) treats this as
   `executor-ping-failed` and (for `runtime` provisional mode) hard-fails.
   No inventory is written.

### Expected behaviour

The daemon's `ping` returns `{"protocol":1,"ok":true,"node":"…",
"playwright":"…","chromium":"…"}` and subsequent browser commands succeed.
Identical behaviour to running the daemon via the no-`nsRoot` branch (line
29), which already works because Node's package resolver correctly picks the
ESM conditional entry there.

### Deliverables

1. **Fix `importPlaywright()`** in
   `skills/design/inventory-design/scripts/playwright/lib/daemon.js:27-39`.
   Prefer `pkg.exports['.'].import` (resolves to `./index.mjs` for
   playwright 1.49.1 and 1.55.x+) over `pkg.main`. Fall back to `pkg.main`
   only when no ESM entry is declared. Update the surrounding comment to
   document why ESM-vs-CJS matters.
2. **Add an isolated unit test for `importPlaywright()`.** Create a fake
   `node_modules/playwright/` fixture under `__fixtures__/fake-playwright/`
   containing a stub `index.mjs` that exports `chromium`, a `package.json`
   with the conditional `exports['.'].import` map, and a stub `index.js` that
   would *not* expose `chromium` if accidentally selected. The test sets
   `ACCELERATOR_PLAYWRIGHT_NS_ROOT` to the fixture directory, calls
   `importPlaywright()`, and asserts:
   - `'chromium' in result === true`
   - `typeof result.chromium.launch === 'function'`
   The test must run without a real Playwright install and must not be
   skip-guarded on `existsSync(cacheRoot)`. Export `importPlaywright` (or
   move it into a sibling module) if needed to make it testable in isolation.
3. **Bump pinned playwright dependency** in
   `skills/design/inventory-design/scripts/playwright/package.json` from
   `~1.49.0` to a range covering `>=1.55.1`. Regenerate
   `package-lock.json`. This closes high-severity advisory
   [GHSA-7mvr-c777-76hp](https://github.com/advisories/GHSA-7mvr-c777-76hp)
   ("Playwright downloads and installs browsers without verifying the
   authenticity of the SSL certificate"). Re-bootstrap locally and confirm
   the new version works with the fixed `importPlaywright()` (the exports
   map shape is unchanged across the 1.49→1.55+ range, so no further
   adjustment expected).

### Out of scope

- Programmatic enforcement of the inventory-design crawl bounds (page cap,
  wall-clock, screenshot budget) — already noted as a separate concern.
- Strengthening `ping` to actually launch and close Chromium as a deeper
  health check — captured as Open Question 1 in the research doc; defer.
- Extracting a reusable `resolveEsmEntry()` helper — not yet justified
  (one call site); flag as a comment for future work.

## Acceptance Criteria

- [ ] `skills/design/inventory-design/scripts/playwright/lib/daemon.js`
      `importPlaywright()` selects `pkg.exports['.'].import` when present
      and otherwise falls back to `pkg.main`. A comment explains the
      ESM-vs-CJS reasoning.
- [ ] Given a developer runs
      `/accelerator:inventory-design <id> http://127.0.0.1:<port>/`
      against a local dev server after bootstrap, when the skill executes
      Step 5 (Confirm Executor Liveness), then the daemon's `ping` returns
      `{"protocol":1,"ok":true,…}` with `chromium` populated, and the
      crawl proceeds to write an inventory.
- [ ] A new test in
      `skills/design/inventory-design/scripts/playwright/lib/` (or a sibling
      test file) imports `importPlaywright`, points
      `ACCELERATOR_PLAYWRIGHT_NS_ROOT` at a fixture
      `__fixtures__/fake-playwright/` containing both an ESM `index.mjs`
      (with `chromium`) and a CJS `index.js` (without `chromium`), and
      asserts both that `'chromium' in result === true` and that
      `typeof result.chromium.launch === 'function'`.
- [ ] The new unit test runs as part of the default test suite — it does
      **not** skip when `~/.cache/accelerator/playwright/` is absent.
- [ ] If a regression is reintroduced (e.g. someone reverts the `exports`
      lookup to read `pkg.main`), the new test fails locally and in CI.
- [ ] `skills/design/inventory-design/scripts/playwright/package.json`
      `playwright` dependency range covers a version `>= 1.55.1`, and
      `package-lock.json` resolves the new version. `npm audit` in that
      directory no longer reports GHSA-7mvr-c777-76hp.
- [ ] A manual `/accelerator:inventory-design` run against a localhost URL
      produces an inventory artifact end-to-end on the upgraded
      playwright version.

## Open Questions

- What precise playwright range should we adopt? `~1.55.0` keeps us inside
  a single minor like the current pin; `^1.55.0` floats with minor bumps;
  `>=1.55.1 <2` is the broadest safe range given the advisory cut-off.
  Recommend `~1.55.1` to match prior pinning discipline unless there's a
  reason to track minors more aggressively.

## Dependencies

- Blocked by: none
- Blocks: any pending or future inventory-design usage that needs runtime
  or hybrid mode — currently every documented `/accelerator:inventory-design`
  invocation against a URL.

## Assumptions

- The exports-map shape (`exports['.'].import === './index.mjs'`) holds for
  every playwright version we'd realistically upgrade to. Validated for
  1.49.1; consistent with playwright's release history. If a future major
  changes the shape, the fallback to `pkg.main` would re-introduce the bug
  silently, so the unit test must remain in place as a guard.
- Bumping playwright across the 1.49 → 1.55+ range does not require
  changes to other daemon code (`browser.newContext()`, `page.goto`,
  `page.screenshot`, etc.). Public API has been stable across that range.
  To be confirmed during the acceptance run.
- The new unit test fixture's stub `chromium.launch` can be a noop function
  — the test asserts shape, not behaviour. If a future change tries to
  actually launch the stub, that's a sign the test scope has crept.

## Technical Notes

Proposed patch to `daemon.js:27-39` (verified empirically — see research doc):

```js
async function importPlaywright() {
  const nsRoot = process.env.ACCELERATOR_PLAYWRIGHT_NS_ROOT;
  if (!nsRoot) return import('playwright');
  const { resolve } = await import('node:path');
  const { pathToFileURL } = await import('node:url');
  // Prefer the ESM entry advertised by pkg.exports['.'].import.
  // pkg.main is the CJS shim; dynamic import() of CJS hides named
  // exports under `default` (chromium becomes undefined).
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

Files touched:

- `skills/design/inventory-design/scripts/playwright/lib/daemon.js` —
  the patch above (lines 27–39).
- `skills/design/inventory-design/scripts/playwright/lib/daemon.test.js`
  (or a new sibling) — new `importPlaywright` fixture test.
- `skills/design/inventory-design/scripts/playwright/__fixtures__/fake-playwright/`
  — new directory with stub `package.json`, `index.mjs`, `index.js`.
- `skills/design/inventory-design/scripts/playwright/package.json` — bump
  `playwright` range.
- `skills/design/inventory-design/scripts/playwright/package-lock.json` —
  regenerated.

## Drafting Notes

- Bundled all three deliverables (fix + regression test + security bump)
  into one work item at the user's explicit direction during scoping.
  Alternative was three separate items.
- Priority set to **high** because every runtime/hybrid crawl is broken
  today and `inventory-design` is the entry point for the
  design-convergence workflow. Not **critical** because there is a manual
  workaround (don't run runtime mode) and the skill itself has been
  shipped only recently.
- Treated the GHSA advisory as in-scope for the same PR because both the
  fix and the bump edit the same `package.json` / `package-lock.json`
  pair, so reviewer overhead is amortised. If reviewer feedback prefers
  separating them, the security bump is a trivial split.
- Chose `~1.55.1` as the recommended range in Open Questions — matches the
  prior `~1.49.0` pinning style; user to confirm before implementation.
- Work item ID 0072 was assigned manually by the user, overriding the
  default allocator. The research document was renamed in lockstep so
  both artifacts share the same `0072` token.

## References

- Research: `meta/research/codebase/2026-05-18-0072-playwright-daemon-cjs-import-bug.md`
- Background: `meta/research/codebase/2026-05-06-design-skill-localhost-and-mcp-issues.md`
- Background: `meta/plans/2026-05-06-design-skill-localhost-and-mcp-issues.md`
- Advisory: GHSA-7mvr-c777-76hp (https://github.com/advisories/GHSA-7mvr-c777-76hp)
- Session diagnostic (local only): `/tmp/inventory-design-debug.log`
