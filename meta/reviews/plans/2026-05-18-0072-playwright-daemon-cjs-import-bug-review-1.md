---
date: "2026-05-18T20:35:00Z"
type: plan-review
producer: review-plan
target: "plan:2026-05-18-0072-playwright-daemon-cjs-import-bug"
review_number: 1
verdict: APPROVE
lenses: [correctness, test-coverage, code-quality, architecture, security, compatibility]
review_pass: 2
status: complete
id: "2026-05-18-0072-playwright-daemon-cjs-import-bug-review-1"
title: "2026-05-18-0072-playwright-daemon-cjs-import-bug-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-18T20:35:00Z"
last_updated_by: Toby Clemson
---

## Plan Review: 0072 — Playwright daemon CJS-import bug

**Verdict:** REVISE

The plan correctly diagnoses a real ESM/CJS interop bug, proposes a five-line
fix supported by a defensible TDD extract-then-fix workflow, and bundles a
well-motivated security dependency bump. The seam choice (sibling
`playwright-loader.js` + `__fixtures__/fake-playwright/`) is consistent with
existing conventions and the lockhash-namespaced cache architecture composes
cleanly with the version bump. However, the fix itself is narrowly tailored
to a single `exports` map shape (silently regressing to the buggy CJS path
for other valid shapes), two automated success-criterion gates are
implemented incorrectly (a regex grep treating `?` as a quantifier, and a
lexicographic string version comparison), and the function still reads
`process.env` directly — undermining the testability rationale for the
extraction. Eight major findings cluster around four themes; the plan is
sound in intent and needs surgical edits before implementation.

### Cross-Cutting Themes

- **Narrow `exports` map handling** (flagged by: correctness, test-coverage,
  compatibility) — `pkg?.exports?.['.']?.import` matches only the shape
  playwright currently uses. String-form exports (`"exports": { ".":
  "./x.mjs" }`), nested condition trees (`{ import: { default: "..." } }`),
  and string-shorthand top-level exports all fall through to `pkg.main`
  (CJS), silently re-introducing the bug. The fixture exercises only the
  happy shape, so the regression test is blind to all variant shapes — yet
  the work item's Assumptions section explicitly cites the test as the
  guard against future shape changes.
- **Silent `catch {}` masks real failures** (flagged by: correctness,
  code-quality, architecture, security) — The empty catch around
  `readFileSync`/`JSON.parse` swallows ENOENT, EACCES, JSON parse errors,
  *and* fixture misconfigurations indiscriminately. This is the same class
  of silent-fallback bug that produced the issue under review.
- **Phase 1 commits a known-broken loader** (flagged by: code-quality,
  architecture) — The intermediate commit ships a test that intentionally
  fails plus a loader with an in-source "fixed in the next commit"
  apology. Bisectability is degraded and a code archaeologist landing on
  that commit sees a deliberately-shipped bug. The TDD ceremony is a
  developer-loop concern, not a commit-history concern.
- **Weak version/regex success-criteria gates** (flagged by: correctness)
  — Phase 2's grep treats `?` as a regex quantifier (won't match the
  literal optional-chaining operator); Phase 3's lockfile check uses
  lexicographic string comparison (`'1.55.10' < '1.55.2'` is true,
  `'1.6.0' > '1.55.1'` is true). Both can pass falsely or fail spuriously.

### Tradeoff Analysis

- **Atomic commit vs TDD ceremony**: Code Quality + Architecture lenses
  recommend collapsing Phases 1+2 into a single commit (preserves
  bisectability, drops the apology comment); Test Coverage + Correctness
  lenses value the explicit RED gate as proof the test exercises the bug.
  These are reconcilable: keep the phase split as a developer-loop
  checklist (write test red → confirm fail locally → apply fix → confirm
  pass) but commit the result as one atomic change.
- **Defence in depth vs narrow scope**: Compatibility flags the resolver
  as too narrow; Code Quality + Architecture favour minimal changes. A
  middle path: keep the resolver simple but throw loudly when
  `pkg.exports['.']` exists but no string `import` entry is reachable
  (loud failure beats silent regression), and document the narrow shape
  assumption in a code comment.

### Findings

#### Major
- 🟡 **Compatibility**: Resolver only handles one of several valid `exports` map shapes
  **Location**: Phase 2 — `lib/playwright-loader.js` resolver logic
  The fix matches only `exports['.'].import` as a string. Three other valid shapes (top-level string, subpath→string, nested condition trees) silently fall through to `pkg.main` (CJS), re-introducing the bug — exactly the silent-regression future-risk the work item flags as the test's job to guard.

- 🟡 **Correctness**: Phase 2 grep success criterion treats `?` as a regex quantifier
  **Location**: Phase 2, Success Criteria — `grep -q "pkg?.exports?\\.\\['.'\\]?\\.import" ...`
  Without `-F`, each `?` becomes an optional-quantifier and the pattern matches a string that doesn't appear in the source. The gate may pass falsely or fail spuriously, defeating its purpose as a regression guard.

- 🟡 **Correctness**: Lexicographic string comparison misorders semver versions
  **Location**: Phase 3, Success Criteria — `node -e "... if (v < '1.55.1') ...`
  String comparison says `'1.55.10' < '1.55.2'` and `'1.6.0' > '1.55.1'`. Within `~1.55.1` the realistic exposure is `1.55.10+`, which would fail the check incorrectly; a hypothetical downgrade to `1.6.0` would pass it incorrectly.

- 🟡 **Architecture**: Loader couples to `process.env` instead of accepting `nsRoot` as a parameter
  **Location**: Phase 1, Section 1 + Phase 1, Section 4 (test mutates `process.env`)
  Extracting a module for testability while leaving the env read inside the function undermines the seam's rationale; the test must mutate global state to drive the function. Change signature to `importPlaywright({ nsRoot } = {})`, default at the call site.

- 🟡 **Architecture**: Silent catch masks layout-coupling failures
  **Location**: Phase 2 — `playwright-loader.js`, `try { ... } catch {}` around `readFileSync`/`JSON.parse`
  Hard-coded `node_modules/playwright/package.json` path inside an empty catch is precisely the silent-fallback class of bug being fixed. Narrow the catch to ENOENT only and rethrow with a prefix; or log a structured warning on the failure path.

- 🟡 **Test Coverage**: Test does not directly verify the FIX's selection logic
  **Location**: Phase 1, Section 4 + Phase 2 success criteria
  Assertions check the resulting namespace shape, not which entry was loaded. A mutation flipping `exports['.'].import` to `exports['.'].default` (both strings in the fixture, both pointing at distinct files in production-shape) could survive. Add a second fixture pair with distinct ESM markers so the selection rule itself is asserted.

- 🟡 **Test Coverage**: `__cjsMarker` assertion may not reliably fire under the bug
  **Location**: Phase 1, Section 4 — `assert.equal(result.__cjsMarker, undefined, ...)`
  Whether `__cjsMarker` is surfaced as a named export under dynamic `import()` of CJS depends on Node's cjs-module-lexer behaviour, which is version- and shape-dependent. The `'chromium' in result` assertion is the reliable detector; the plan's framing of `__cjsMarker` as load-bearing is inverted.

- 🟡 **Compatibility**: playwright 1.49→1.55 API-stability deferred to manual Phase 4 only
  **Location**: Phase 3
  Bumping six minor versions with verification deferred to a single-URL manual smoke. The daemon uses `accessibility.snapshot()`, `screenshot({ mask })`, text-selectors, `headless: true`, click/type with refs — several of which have had behavioural tweaks across this range. Expand the Phase 4 checklist or make `daemon.test.js` gating in Phase 3.

#### Minor
- 🔵 **Correctness**: Silent `catch {}` could mask fixture misconfigurations in the test
  **Location**: Phase 1 & 2 — `try {} catch {}` in `playwright-loader.js`
  If the fixture path were wrong or the fixture's `package.json` malformed, the loader would silently keep the default `entryFile = 'index.mjs'` and the test could still pass for the wrong reason.

- 🔵 **Correctness**: Fix does not handle conditional `import` objects or shorthand string exports
  **Location**: Phase 2 — resolver
  See cross-cutting theme. Realistic future shapes (string shorthand, nested conditions) silently degrade to `pkg.main`.

- 🔵 **Correctness**: Re-revert manual step in Phase 2 duplicates Phase 1 RED gate
  **Location**: Phase 2, Manual Verification
  Phase 1 RED + Phase 2 GREEN already constitute the bisect proof. Re-reverting adds no information and introduces a window where the working tree contains the bug.

- 🔵 **Test Coverage**: Daemon ping test going green is asserted but only verifiable on developer laptops
  **Location**: Phase 2 success criteria + Testing Strategy
  CI lacks the populated cache, so the claim is unverifiable in automation. Either drop the claim or relax `daemon.test.js`'s `existsSync` skip to point at a fake-cache fixture.

- 🔵 **Test Coverage**: No test for nested-condition or string-form exports
  **Location**: Phase 1, Section 4 + work-item Assumptions
  The work item explicitly assigns the unit test as the guard against shape changes; the fixture doesn't actually exercise alternate shapes. Add at least one parametrised case.

- 🔵 **Test Coverage**: Silent `try/catch` exercised only indirectly
  **Location**: Phase 1, Section 4
  Add a no-manifest fixture case to pin the default-entry fallback behaviour explicitly.

- 🔵 **Test Coverage**: Manual-only Phase 4 gate for a regression-on-every-invocation bug
  **Location**: Phase 4
  The original bug shipped because there was no automated end-to-end smoke. The unit-test-only automation reproduces that gap at a different layer.

- 🔵 **Code Quality**: Phase 1 ships a known-broken loader as a standalone commit
  **Location**: Phase 1
  See cross-cutting theme. Collapse Phases 1+2 into one commit; keep the phase split as a developer-loop checklist only.

- 🔵 **Code Quality**: Dynamic `await import('node:*')` is inconsistent with sibling modules
  **Location**: Phase 1 & 2 — `playwright-loader.js` body
  Every other lib/*.js module uses static top-of-file imports for `node:*`. The dynamic style is inherited verbatim from the CJS-era original and serves no purpose in an ESM module. The extraction is the natural moment to normalise.

- 🔵 **Code Quality**: Speculative `resolveEsmEntry` note in header comment doesn't earn its keep
  **Location**: Phase 2 — final-form header comment
  Forward-looking speculation about a refactor that may never happen. Drop the third paragraph; the plan/work-item record already captures it.

- 🔵 **Architecture**: Phase 1 commit ships a deliberately failing test in the suite
  **Location**: Phase 1 success criteria
  Breaks the green-main invariant and degrades bisectability for unrelated future investigations.

- 🔵 **Architecture**: Knowledge of nested cache layout still duplicated in two modules
  **Location**: daemon.js ping handler vs new loader
  The ping handler at `daemon.js:140-150` already constructs its own `resolve(nsRoot, 'node_modules/playwright/package.json')`. The "second call site" the plan defers `resolveEsmEntry` for already exists today. Consider exposing `resolvePlaywrightPkgJson(nsRoot)` from the loader and reusing it in the ping handler.

- 🔵 **Architecture**: Mixing security bump and bug fix in one PR
  **Location**: Implementation Approach
  The amortisation argument is reasonable, but split into two commits within the PR (loader fix; security bump) to preserve bisectability if the new playwright version turns out to have a regression.

- 🔵 **Security**: Open Question 1 (range choice) is not explicitly resolved in the plan
  **Location**: Phase 3, Section 1
  The work item flags the range as an Open Question. `~1.55.1` is the correct choice (1.55.0 is below the fix line) but the plan doesn't record the rationale. Add a one-line note so a future careless edit to `~1.55.0` doesn't silently re-admit the unpatched release.

- 🔵 **Security**: Lockfile check verifies `playwright` only, not transitive `playwright-core`
  **Location**: Phase 3 success criteria
  GHSA-7mvr-c777-76hp affects the installer which lives in `playwright-core`. The work item explicitly calls for checking both; the plan loses the `playwright-core` half. Add an automated assertion.

- 🔵 **Security**: Trust boundary on `ACCELERATOR_PLAYWRIGHT_NS_ROOT` is not documented
  **Location**: Phase 1, Section 1
  A hostile `nsRoot` could induce arbitrary module loading via the `import()` call. Production is safe because `run.sh` sets it from a lockhash, but the trust assumption should be a code comment so future callers don't accidentally widen the boundary.

- 🔵 **Security**: No integrity verification of regenerated lockfile beyond version checks
  **Location**: Phase 3, Section 2
  Add a manual-verification step confirming `resolved` URLs point at the expected registry and spot-checking `integrity` for `playwright`/`playwright-core`.

- 🔵 **Security**: Old 1.49.x cache is left on disk under its prior lockhash
  **Location**: Phase 3, Section 3 + Migration Notes
  Inert today but contains a vulnerable installer. Note in Phase 3 that `ACCELERATOR_PLAYWRIGHT_SWEEP=1` is recommended once after the bump.

- 🔵 **Compatibility**: Fixture's package.json doesn't reflect playwright 1.55's actual exports shape
  **Location**: Phase 3
  The fixture is fixed at the 1.49.1 shape. After regenerating the lockfile, confirm `node_modules/playwright/package.json` matches or update the fixture.

- 🔵 **Compatibility**: GHSA closure depends on transitive `playwright-core` resolving to ≥1.55.1
  **Location**: Phase 3 success criteria
  Duplicate of the security-lens finding from a compatibility angle. Same fix.

- 🔵 **Compatibility**: Dynamic `import()` of CJS named-export interop varies across Node 18/20/22
  **Location**: Phase 2
  The fix is Node-version-independent post-fix; add a one-line note to the loader comment so a future Node-floor change doesn't tempt a contributor to "simplify" the resolver back.

- 🔵 **Compatibility**: Cross-platform `pathToFileURL` + path-with-spaces
  **Location**: Phase 1 / Phase 2
  Low risk; verify the test passes from a path containing a space.

#### Suggestions
- 🔵 **Correctness**: `__cjsMarker` framing in plan is inverted
  Reorder the rationale: `'chromium' in result` is the primary regression guard; `__cjsMarker === undefined` is the secondary check. No code change required.

- 🔵 **Test Coverage**: Non-`nsRoot` branch is dead code in production but still untested
  Optional one-liner test gated on local playwright install presence.

- 🔵 **Code Quality**: `playwright-loader.js` name is broad for a single-function module
  Consider `import-playwright.js` for a tighter file/function correspondence.

- 🔵 **Architecture**: Loader module could plausibly live under a bootstrap-adjacent path
  Keep current placement; add a one-line layout-contract reference comment pointing at `ensure-playwright.sh`.

### Strengths

- ✅ Root cause analysis is empirically verified and the fix restores
  symmetry between the nsRoot and no-nsRoot branches rather than papering
  over the symptom.
- ✅ Sibling-module + co-located `.test.js` extraction is consistent with
  the existing lib/ convention (auth-header, lock, path-guard, state).
- ✅ Fixture under `lib/__fixtures__/fake-playwright/` follows the
  precedent set by `proc-stat-linux.txt`/`ps-lstart-macos.txt`; deliberately
  broken CJS `index.js` makes a wrong selection observable.
- ✅ The test removes the `existsSync` skip guard that hid the bug
  originally; runs in milliseconds with zero Chromium dependency.
- ✅ Defers reusable `resolveEsmEntry(pkgJson, conditions)` helper to a
  code comment until a second call site appears — aligned with YAGNI and
  the user's documented preference against premature abstractions.
- ✅ Lockhash-derived NS_ROOT ensures the security bump cannot be silently
  bypassed by a stale cache; `npm ci` + `npx playwright install chromium`
  re-run under the patched version automatically.
- ✅ `~1.55.1` is the correct security-driven lower bound (1.55.0 is below
  the GHSA fix line) and matches existing pinning discipline.
- ✅ Uses `npm install --package-lock-only` preserving the `npm ci`
  invariant for the bootstrap script.
- ✅ Out-of-scope list explicitly preserves architectural boundaries (no
  changes to `run.sh`, `ensure-playwright.sh`, or wider inventory-design
  skill).
- ✅ `pathToFileURL` is the cross-platform-correct way to convert paths to
  import URLs.

### Recommended Changes

Ordered by impact:

1. **Fix the two automated success-criterion gates** (addresses: "Phase 2
   grep treats `?` as a regex quantifier", "Lexicographic string
   comparison misorders semver")
   - Phase 2 grep: switch to `grep -qF "pkg?.exports?.['.']?.import"`.
   - Phase 3 lockfile check: parse the version into numeric components
     (e.g. `v.split('.').map(Number)` and component-wise compare) or use
     `npx semver -r '>=1.55.1' "$v"`.

2. **Decide on `exports` map breadth and document it loudly** (addresses:
   "Resolver only handles one of several valid exports map shapes",
   "Fix does not handle conditional import objects or shorthand string
   exports", "No test for nested-condition or string-form exports")
   - Recommend option (a): keep the resolver simple BUT throw loudly when
     `pkg.exports['.']` exists and no string `import` entry is reachable
     (loud failure beats silent regression to CJS).
   - Add an inline comment naming the assumed shape and the two
     unsupported variants (string-shorthand, nested conditions).
   - Add a parametrised test case with a fixture whose `exports` is a
     nested condition tree; assert the loader throws (per the chosen
     defensive behaviour) rather than silently selecting `pkg.main`.

3. **Pass `nsRoot` as a parameter, not via `process.env`** (addresses:
   "Loader couples to process.env")
   - Signature: `export async function importPlaywright({ nsRoot } = {})`.
   - In `daemon.js`, default at the call site:
     `importPlaywright({ nsRoot: process.env.ACCELERATOR_PLAYWRIGHT_NS_ROOT })`.
   - Test drops the env mutate/restore and passes `FAKE_NS_ROOT` directly.

4. **Narrow the silent catch** (addresses: "Silent catch masks
   layout-coupling failures", "Silent `catch {}` could mask fixture
   misconfigurations")
   - Catch only `err.code === 'ENOENT'` and re-throw other errors with a
     `playwright-loader: failed to read <path>` prefix.

5. **Collapse Phases 1 and 2 into a single atomic commit** (addresses:
   "Phase 1 ships a known-broken loader as a standalone commit", "Phase 1
   commit ships a deliberately failing test")
   - Keep the phase split as a developer-loop checklist (write test red,
     observe failure locally, apply fix, observe pass) but commit the
     extract + test + fix as one change.
   - Remove the "Known issue (fixed in the next commit)" interim comment
     entirely.
   - Drop the Phase 2 "re-revert the fix" manual verification step (it
     duplicates the Phase 1 RED gate that the developer-loop already
     confirms locally).

6. **Strengthen the version/exports verification on the real installed
   playwright** (addresses: "playwright 1.49→1.55 API-stability deferred
   to manual Phase 4", "Fixture's package.json doesn't reflect playwright
   1.55's actual exports shape", "Lockfile check verifies playwright only,
   not transitive playwright-core")
   - After regenerating the lockfile, inspect the installed
     `node_modules/playwright/package.json` and confirm its `exports`
     shape matches the fixture; if not, update the fixture or expand the
     resolver per change #2.
   - Add an automated check asserting `playwright-core` version is also
     `>= 1.55.1`.
   - Make `node --test lib/daemon.test.js` gating in Phase 3 (after
     rebootstrap), not just descriptive.
   - Expand the Phase 4 manual checklist to exercise each daemon command:
     navigate, snapshot, screenshot-with-mask, evaluate, click, type,
     wait_for — not just a single happy-path crawl.

7. **Document the loader's trust assumptions and security context**
   (addresses: "Trust boundary on ACCELERATOR_PLAYWRIGHT_NS_ROOT is not
   documented", "Open Question 1 is not explicitly resolved")
   - Add a one-paragraph trust-boundary comment to `playwright-loader.js`
     stating that `nsRoot` is fully trusted, citing `run.sh` as the only
     intended setter.
   - In Phase 3 (or `package.json`), record that the `.1` patch floor of
     `~1.55.1` is load-bearing because 1.55.0 is below the GHSA fix line.

8. **Refine the test's assertion structure** (addresses: "`__cjsMarker`
   assertion may not reliably fire under the bug", "Test does not directly
   verify the FIX's selection logic")
   - Promote `'chromium' in result` to the primary regression-guard
     assertion in both the plan narrative and the test source order.
   - Keep the `__cjsMarker` check as a belt-and-braces secondary
     assertion or replace it with a fixture-pair test where both entries
     are ESM but expose distinct named markers (`__esmEntry === 'mjs'`
     vs `__mainEntry === 'main'`) — this directly asserts the selection
     rule rather than its emergent consequence.

9. **Split the PR into two commits** (addresses: "Mixing security bump
   and bug fix in one PR")
   - Commit 1: loader extract + test + fix.
   - Commit 2: `package.json` + `package-lock.json` bump (+ trust-boundary
     comment if landed alongside).

10. **Minor cleanups** (addresses: minor + suggestion-tier findings)
    - Normalise the loader to static top-of-file imports for `node:*`
      (consistency with sibling modules).
    - Drop the speculative `resolveEsmEntry` paragraph from the
      header comment.
    - Add a Migration Notes line recommending
      `ACCELERATOR_PLAYWRIGHT_SWEEP=1` once after the bump.
    - Add an automated lockfile integrity check or manual verification
      step for `resolved` URLs and `integrity` fields.
    - Consider re-using the loader from `daemon.js`'s ping handler so
      the nested-cache layout is owned by one module (this is the second
      call site the plan describes as "not yet present" — it is in fact
      present today).

---
*Review generated by /review-plan*

## Per-Lens Results

### Correctness

**Summary**: The plan correctly identifies the root cause and proposes a
minimally invasive five-line fix wrapped in a defensible TDD red/green
workflow. The core logic of the fix is sound, and the Phase 1 RED state is
achievable via the primary `'chromium' in result` assertion. However, two
automated success-criterion checks contain correctness bugs (a regex grep
that mis-parses `?` as a quantifier, and a lexicographic version comparison
that misorders semver), the silent `catch {}` around fixture loading can
mask fixture misconfigurations, and the fix does not defend against the
realistic case where a future playwright `exports['.'].import` is itself a
conditional object rather than a string.

**Strengths**: Empirically verified root cause; fix falls back through a
defensible chain; Phase 1 RED is correctly load-bearing on the `chromium`
assertion; fixture design makes mis-selection observable; env-var
save/restore is correct.

**Findings**:
- 🟡 Phase 2 grep success criterion treats `?` as a regex quantifier
  (major, high)
- 🟡 Lexicographic string comparison misorders semver versions in
  lockfile check (major, high)
- 🔵 Fix does not handle conditional `import` objects or shorthand string
  exports (minor, high)
- 🔵 Silent `catch {}` masks ENOENT and JSON parse errors and may hide
  test-fixture misconfigurations (minor, medium)
- 🔵 Re-revert manual step duplicates information already established by
  Phase 1 RED (minor, high)
- 🔵 `__cjsMarker` assertion is not strictly load-bearing and may be
  redundant with `'chromium' in result` (suggestion, medium)

### Test Coverage

**Summary**: The plan adopts a sound TDD approach with a fixture-based
unit test that isolates the loader from a real Playwright install and
runs unconditionally. However, the load-bearing RED-test assertion is
fragile because Node's CJS named-export detection can surface
`__cjsMarker` either on the namespace directly or only under `default`,
making the test's failure semantics unclear. Several edge cases
(string-form exports, malformed package.json, nested condition trees)
are uncovered, leaving the fix's guard against future regressions
thinner than the plan implies.

**Strengths**: Explicit TDD sequencing; no `existsSync` skip; sibling-test
convention; fixture deliberately uses different shapes; Phase 2 manual
re-revert sanity check; success criteria distinguish RED and GREEN.

**Findings**:
- 🟡 `__cjsMarker` assertion may not reliably fire under the bug (major,
  medium)
- 🟡 Test does not directly verify the FIX's selection logic — only its
  outcome (major, high)
- 🔵 Daemon ping test going green is asserted but only verifiable on
  developer laptops (minor, high)
- 🔵 No test for nested-condition exports or string-form exports —
  future-regression risk acknowledged but unguarded (minor, high)
- 🔵 Silent `try/catch` around package.json read is exercised only
  indirectly (minor, medium)
- 🔵 Non-`nsRoot` branch is dead code in production but still untested
  (suggestion, high)
- 🔵 Manual-only Phase 4 gate for a regression-on-every-invocation bug
  (minor, medium)

### Code Quality

**Summary**: The plan is well-structured, applies the established
sibling-module + co-located test convention, and correctly defers a
`resolveEsmEntry` helper until a second call site emerges. The main
concerns are: (1) Phase 1 deliberately commits a known-broken loader,
trading bisectability for TDD ceremony; (2) dynamic `await
import('node:fs|path|url')` is inconsistent with every other lib/*.js
module; (3) the multi-paragraph header comment includes one speculative
paragraph that arguably doesn't earn its keep.

**Strengths**: Seam and fixture convention consistent with existing
modules; YAGNI-aligned deferral; comment captures non-obvious ESM/CJS
interop why; function remains short and single-purpose.

**Findings**:
- 🔵 Phase 1 ships a known-broken loader as a standalone commit (minor,
  high)
- 🔵 Dynamic `await import('node:*')` is inconsistent with sibling
  modules (minor, high)
- 🔵 Speculative `resolveEsmEntry` note in header comment does not earn
  its keep (minor, medium)
- 🔵 Silent `catch {}` swallows ENOENT, EACCES, and JSON parse errors
  indiscriminately (suggestion, medium)
- 🔵 `playwright-loader.js` name is broad for a single-function module
  (suggestion, low)

### Architecture

**Summary**: The plan extracts a small, focused loader module along an
established sibling-test convention and drives the fix via TDD. The seam
and scope are sensible, but the loader's interface leaks an
environment-variable dependency into its implementation, hard-codes the
nested cache layout that is also known to the ping handler and bootstrap
scripts, and the Phase 1/2 split leaves an intermediate commit with a
deliberately broken regression test. Overall the architecture is
appropriate for a localised bug fix; the main risks are coupling on
`process.env` and silent failure modes that the swallowed catch can mask.

**Strengths**: Seam matches existing convention; YAGNI-aligned scope
discipline; TDD ordering preserves intent-to-bisect; security bump
composes cleanly with lockhash architecture; out-of-scope list preserves
architectural boundaries.

**Findings**:
- 🟡 Loader couples to `process.env` instead of accepting nsRoot as a
  parameter (major, high)
- 🟡 Silent catch masks layout-coupling failures (major, high)
- 🔵 Phase 1 commit ships a deliberately failing test in the suite
  (minor, high)
- 🔵 Knowledge of nested cache layout still duplicated in two modules
  (minor, medium)
- 🔵 Mixing security bump and bug fix in one PR is a real but accepted
  tradeoff (minor, medium)
- 🔵 Loader module could plausibly live under a bootstrap-adjacent path
  (suggestion, low)

### Security

**Summary**: The plan correctly identifies GHSA-7mvr-c777-76hp and bumps
playwright out of the affected 1.49.x range to ~1.55.1. The lockhash-keyed
cache design means the bump automatically forces a fresh `npm ci` and
`npx playwright install chromium` under the patched version, so future
browser downloads will get TLS verification. The main residual concerns
are: Open Question 1 is unresolved in the plan; transitive
`playwright-core` not checked; loader trust boundary on
`ACCELERATOR_PLAYWRIGHT_NS_ROOT` undocumented.

**Strengths**: Correct version floor (1.55.1 is at the GHSA fix line);
lockhash invalidation ensures patched installer is used; `npm audit
--omit=dev` as the verification gate catches other open advisories;
Phase 3 success criteria include rebootstrap verification; fixture is
sandboxed to the test only.

**Findings**:
- 🔵 Open Question 1 (range choice) is not explicitly resolved in the
  plan (minor, high)
- 🔵 Lockfile check verifies `playwright` only, not transitive
  `playwright-core` (minor, high)
- 🔵 Trust boundary on `ACCELERATOR_PLAYWRIGHT_NS_ROOT` is not documented
  (minor, high)
- 🔵 No integrity verification of the regenerated lockfile beyond version
  checks (minor, medium)
- 🔵 Old 1.49.x cache is left on disk under its prior lockhash (minor,
  medium)

### Compatibility

**Summary**: The plan addresses a real ESM/CJS interop bug and pairs it
with a security-driven dependency bump. The fix targets the specific
exports-map shape playwright 1.49→1.55 uses, which is appropriate given
playwright is the only nsRoot-resolved package. However, the resolver is
narrowly tailored to `exports['.'].import` as a string and would silently
fall through to the buggy `pkg.main` for several valid exports-map
shapes; the version bump's API-stability claim across 1.49→1.55 is
asserted but not verified before Phase 4 manual testing.

**Strengths**: Correctly identifies the no-nsRoot branch parity goal;
uses `pathToFileURL` (cross-platform-correct); lockhash-namespaced cache
auto-invalidates; tilde-pinning discipline preserved; fixture test runs
unconditionally; `npm install --package-lock-only` preserves `npm ci`
invariant.

**Findings**:
- 🟡 Resolver only handles one of several valid `exports` map shapes
  (major, high)
- 🟡 playwright 1.49→1.55 API-stability assumption deferred to manual
  Phase 4 test (major, medium)
- 🔵 Fixture's package.json doesn't reflect playwright 1.55's actual
  exports shape (minor, high)
- 🔵 GHSA closure depends on transitive `playwright-core` resolving to
  ≥1.55.1, not just `playwright` (minor, medium)
- 🔵 Dynamic `import()` of CJS named-export interop varies across Node
  18/20/22 (minor, medium)
- 🔵 Cross-platform `pathToFileURL` + nested-fixture path is fine but
  worth confirming on path-with-spaces (minor, high)


## Re-Review (Pass 2) — 2026-05-18T20:35:00Z

**Verdict:** APPROVE

The plan revision addresses every pass-1 finding either fully or substantially.
Of the 8 major findings from pass 1, 6 are fully resolved and 2 are
partially resolved with surfaced edge cases. The revised plan is materially
stronger across all six lenses and is acceptable for implementation —
remaining concerns are minor refinements that can be addressed inline or
deferred to implementation review. Two new major findings emerged from the
deeper second pass: the `daemon.test.js` Phase 2 "gate" silently passes on
cache-absent environments (the test early-returns without assertion), and
the resolver's loud-throw branch is one shape short — the string-shorthand
form of `exports['.']` still silently falls through to `pkg.main` (the
exact bug class the throw was added to prevent).

### Previously Identified Issues

#### Resolved
- 🟡 **Correctness**: Phase 2 grep success criterion treats `?` as a regex
  quantifier — **Resolved**. Now uses `grep -qF` for fixed-string match.
- 🟡 **Correctness**: Lexicographic string comparison misorders semver —
  **Resolved**. Now uses numeric semver compare; a prerelease edge case
  remains but is a separate minor finding.
- 🟡 **Architecture**: Loader couples to `process.env` instead of
  accepting `nsRoot` as a parameter — **Resolved**. Signature is now
  `importPlaywright({ nsRoot } = {})`; test passes `nsRoot` directly.
- 🟡 **Architecture**: Silent catch masks layout-coupling failures —
  **Resolved**. Catch narrowed to ENOENT; other errors rethrow with a
  `playwright-loader:` prefix.
- 🟡 **Test Coverage**: Test does not directly verify the FIX's selection
  logic — **Resolved**. New `fake-playwright-distinct-entries` fixture
  uses ESM-vs-ESM with distinct `__selected` markers; assertion verifies
  the selection rule directly.
- 🟡 **Test Coverage**: `__cjsMarker` assertion may not reliably fire
  under the bug — **Resolved**. `__cjsMarker` removed; primary
  assertion is `'chromium' in result`.
- 🔵 **Code Quality**: Phase 1 ships a known-broken loader as a standalone
  commit — **Resolved**. Phases 1+2 collapsed into a single atomic
  commit; phase split retained only as developer-loop checklist.
- 🔵 **Code Quality**: Dynamic `await import('node:*')` inconsistent with
  sibling modules — **Resolved**. Static top-of-file imports.
- 🔵 **Code Quality**: Speculative `resolveEsmEntry` paragraph — **Resolved**.
  Removed from header comment.
- 🔵 **Architecture**: Phase 1 commit ships a deliberately failing test —
  **Resolved**. See atomic-commit resolution above.
- 🔵 **Architecture**: Knowledge of nested cache layout duplicated in two
  modules — **Resolved**. `resolvePlaywrightPkgPath(nsRoot)` helper
  exposed by the loader and reused in the ping handler.
- 🔵 **Architecture**: Mixing security bump and bug fix in one PR —
  **Resolved**. PR explicitly split into two commits (loader fix; bump).
- 🔵 **Security**: Open Question 1 (range choice) not resolved — **Resolved**.
  `.1` patch-floor rationale documented in Phase 2 description (though
  only in commit message rather than in `package.json` itself — see new
  minor finding).
- 🔵 **Security**: Lockfile check verifies `playwright` only —
  **Resolved**. Numeric semver check now covers both `playwright` AND
  `playwright-core`.
- 🔵 **Security**: Trust boundary on `ACCELERATOR_PLAYWRIGHT_NS_ROOT` not
  documented — **Resolved**. Trust-boundary paragraph added to the
  loader's header comment.
- 🔵 **Security**: No integrity verification of regenerated lockfile —
  **Resolved**. Phase 2 manual verification confirms `resolved` URLs +
  `integrity` fields + one cross-referenced hash via `npm view`.
- 🔵 **Security**: Old 1.49.x cache left on disk — **Resolved**. Migration
  Notes recommend `ACCELERATOR_PLAYWRIGHT_SWEEP=1` post-bump.
- 🔵 **Compatibility**: GHSA closure depends on transitive
  `playwright-core` — **Resolved**. Covered by the joint lockfile check.
- 🔵 **Test Coverage**: Daemon ping test only verifiable on developer
  laptops — **Partially Resolved**. Phase 2 promotes it to a hard gate
  but the gate is silently bypassable on cache-absent envs (see new
  major finding).
- 🔵 **Test Coverage**: Manual-only Phase 4 gate for a regression-on-
  every-invocation bug — **Partially Resolved**. Phase 3 expanded to
  exercise every daemon command, materially improving integration
  coverage; full automated end-to-end smoke still not added (out of
  scope per plan).
- 🔵 **Correctness**: Silent `catch {}` masks fixture misconfigurations —
  **Resolved**. Catch is now ENOENT-only with throw-on-other.
- 🔵 **Correctness**: Re-revert manual step duplicates Phase 1 RED —
  **Partially Resolved**. The manual step still exists but now as a
  developer-loop confirmation only, not preserved in commit history.

#### Partially Resolved
- 🟡 **Compatibility**: Resolver only handles one of several valid
  `exports` map shapes — **Partially Resolved**. The plan now throws
  loudly when `exports['.']` is an object without a string `import`
  (covering the nested-conditions case). However, the string-shorthand
  case (`"exports": { ".": "./index.mjs" }`) is still silently treated
  as "no exports" and falls through to `pkg.main`. See new major
  finding below.
- 🟡 **Compatibility**: playwright 1.49→1.55 API-stability deferred to
  manual Phase 4 only — **Partially Resolved**. Phase 3 manual checklist
  now exercises navigate/snapshot/screenshot-with-mask/evaluate/click/
  type/wait_for. Phase 2 also promotes `daemon.test.js` to a gate, but
  see new major finding about that gate's effectiveness.

### New Issues Introduced

#### Major
- 🟡 **Test Coverage**: Phase 2 `daemon.test.js` "gate" is silently
  bypassable on cache-absent environments
  The existing ping test in `daemon.test.js` does `if (!nsRoot) return;`
  — an early return with no assertion. On CI, fresh dev setups, or
  immediately after the version bump (before `run.sh` rebootstraps),
  the test passes vacuously. The plan claims it as "a hard gate, not
  descriptive" but the gate cannot distinguish "fix works" from "test
  never ran". Fix: add a precondition check that fails if `nsRoot` is
  not resolvable before running the test, or drop the "hard gate"
  framing.

- 🟡 **Correctness / Code Quality / Compatibility**: String-shorthand
  `exports['.']` form silently falls through to `pkg.main`
  The Node spec permits `"exports": { ".": "./index.mjs" }` (where the
  subpath is a string). The loader's check `typeof subpath === 'object'`
  is false for this shape, so it falls through to `else if (typeof
  pkg.main === 'string') entryFile = pkg.main`. That is exactly the
  silent-fall-through-to-CJS regression mode the loud-throw branch was
  added to prevent — but for a different shape. The nested-conditions
  fixture only catches the object-with-non-string-import case.
  Fix: add `if (typeof subpath === 'string') { entryFile = subpath; }`
  before the object check, and add a fourth fixture pinning the
  shorthand-shape behaviour.

#### Minor
- 🔵 **Correctness / Compatibility**: Prerelease versions pass the semver
  gate spuriously. `'1.55.1-beta.0'.split('.').map(Number)` yields
  `[1, 55, NaN, 0]`; the loop returns `NaN`, `NaN < 0` is false, and
  the gate passes. Add `if (v.includes('-')) { fail }` before the
  numeric compare.

- 🔵 **Correctness**: Missing `pkg.exports` falls through to `pkg.main`
  without warning. If a future package has no `exports` field, the
  loader silently uses `pkg.main` — same silent regression mode.
  Hypothetical for playwright today but inconsistent with the loud-
  failure design principle applied to the nested-conditions case.

- 🔵 **Test Coverage**: Three loader branches remain untested — no-`nsRoot`
  early return, ENOENT manifest fallback, `pkg.main` fallback when
  `exports['.']` absent. Each is cheap to cover with one extra fixture.

- 🔵 **Test Coverage**: Shape-change test's error-message regex
  (`/pkg\.exports\['\.'\]\.import is not a string/`) couples to wording
  rather than behaviour. Attach a stable `err.code` property and assert
  on that instead.

- 🔵 **Code Quality**: Four-paragraph header comment (~16 lines) is
  disproportionate to a ~45-line module. Sibling modules use single-line
  headers. Consider keeping paragraph 1 as the file header and inlining
  trust-boundary + layout-contract notes near the relevant code.

- 🔵 **Code Quality**: Redundant `subpath !== undefined &&` check —
  `typeof subpath === 'object'` already excludes undefined. The
  idiomatic check is `typeof subpath === 'object' && subpath !== null`.

- 🔵 **Architecture**: `resolvePlaywrightPkgPath` helper is asymmetric —
  only the `nsRoot` branch is folded; the no-`nsRoot` branch in the
  ping handler still hand-rolls a `new URL(...)`. Acceptable given the
  layouts differ; could be made explicit via a comment.

- 🔵 **Architecture**: Loud-failure throw message "Update the loader."
  propagates through the daemon's outer try/catch into client-visible
  `internal-error` envelopes. Audience is correct (developer running
  the daemon) but the imperative reads as a `// TODO`. Either rephrase
  descriptively or add an audience-marker comment.

- 🔵 **Security**: `.1` patch-floor rationale only lives in commit
  message + plan, not in `package.json` itself. A future maintainer
  relaxing to `~1.55.0` would not see the warning. Make the optional
  inline comment mandatory, or promote the numeric semver check to a
  permanent CI guard.

- 🔵 **Security**: Lockfile integrity spot-check is one-shot, not
  ongoing. Subsequent `npm install` regenerations won't be verified.
  Consider pinning `--registry` in `ensure-playwright.sh` or noting
  the spot-check procedure in Migration Notes for future bumps.

- 🔵 **Compatibility**: Phase 2 Step 3's example command for locating
  `nsRoot` references a non-existent `run.sh status` subcommand. The
  fallback comment (lockhash-cache path) is the working approach, but
  the developer may follow the broken example first. Replace with a
  direct lockhash-derived path: `LOCKHASH=$(shasum -a 256 ...
  package-lock.json | cut -c1-8); jq '.exports.\".\"' "$HOME/.cache/
  accelerator/playwright/$LOCKHASH/node_modules/playwright/package.json"`.

#### Suggestions
- 🔵 **Code Quality**: Add a one-line comment in the ENOENT catch
  explaining the intentional silent fall-through to default `index.mjs`.
- 🔵 **Code Quality**: Trim the Manual TDD-loop verification — the
  third fixture (loud-throw) already pins the silent-fall-through
  regression in code.
- 🔵 **Test Coverage**: Optional third marker in the selection-rule
  fixture to uniquely identify which path the loader took.
- 🔵 **Architecture**: Consider throwing on ENOENT too, since
  `ensure-playwright.sh`'s sentinel check should prevent that state.
- 🔵 **Compatibility**: Tighten audit-gate wording from "no high-severity
  GHSA-7mvr-c777-76hp" to "no high-severity advisories on `playwright`
  or `playwright-core`".

### Assessment

The plan is materially improved and acceptable for implementation. The two
new major findings (silently-bypassable Phase 2 gate, string-shorthand
exports fall-through) are worth addressing inline before merge — both are
small additions (a precondition check; a one-line `typeof subpath ===
'string'` branch + a fourth fixture). The minor findings can be addressed
during implementation review or deferred. No structural rework is needed;
the revised architecture, test strategy, and PR commit structure are all
sound.
