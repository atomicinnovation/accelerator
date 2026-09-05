---
type: "plan-review"
id: "2026-08-31-0203-third-party-attribution-artefact-review-1"
title: "Plan Review: Third-Party Attribution Artefact Implementation Plan"
date: "2026-08-31T21:59:18+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
parent: "plan:2026-08-31-0203-third-party-attribution-artefact"
target: "plan:2026-08-31-0203-third-party-attribution-artefact"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "compatibility", "security", "documentation", "safety"]
review_number: 1
review_pass: 2
tags: ["rust", "frontend", "licensing", "release", "vcs"]
last_updated: "2026-08-31T22:34:59+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Third-Party Attribution Artefact Implementation Plan

**Verdict:** REVISE

The plan is unusually well-grounded — every integration surface is verified
against real line numbers, the `RELEASE_MANIFEST`-over-`TREE_ARTIFACTS` decision
is correct, and the `public_api` byte-compare model is the right skeleton. It is
held back from approval by two clusters that go to the heart of the work item:
the drift gate is not wired to run anywhere it will actually execute with its
inputs provisioned (five lenses), and the artefact as specified does not
reliably discharge the very MPL-2.0 §3.2 duty it exists to close, nor is its
completeness tested (four lenses). Both are fixable in the plan without
restructuring; neither should reach implementation unaddressed.

### Cross-Cutting Themes

- **Provisioning and CI wiring of `notices:check`** (flagged by: architecture,
  correctness, compatibility, safety) — The `notices:check`/`notices:update`
  tasks declare no `depends`, so nothing guarantees `node_modules`
  (`license-checker-rseidelsohn`) or a warm cargo source cache (`cargo about
  generate --frozen` = `--offline`) exists when they run. Worse, compatibility
  found CI runs **per-component jobs, not the aggregate `check`/`default`** —
  so as wired the gate runs in no CI lane at all, and no single existing job
  provisions both closures. This is the single most-reinforced issue.
- **§3.2 source-availability is not actually discharged** (flagged by: security,
  documentation, correctness, test-coverage) — Reprinting the MPL-2.0 licence
  body is not §3.2 discharge; the plan never says *where* corresponding source
  is obtainable, the `about.hbs` block spec omits the statement, cargo-about's
  default feature/target resolution may omit `uluru` from the render entirely,
  and nothing asserts its presence automatically. The `deny.toml` comment would
  then claim a discharge that did not happen.
- **Frontend closure under-reports in the unsafe direction** (flagged by:
  security, correctness) — `--production` walks the declared prod-dependency
  graph, not the Vite bundle. A runtime dependency mis-declared under
  `devDependencies`, or inlined by the bundler, ships inside the signed
  visualiser binary with no notice and no check catching it. The AC asked for
  enumeration "over the built `dist/` bundle".
- **Byte-compare determinism across hosts** (flagged by: correctness,
  architecture, compatibility, test-coverage) — `about.toml` pins no `targets`
  or feature policy, so cargo-about's closure can differ macOS-vs-Linux;
  `node_modules` transitive resolution and verbatim CRLF/trailing-newline
  handling add further host variance. A green local `notices:update` can fail
  `notices:check` in CI with no dependency change.
- **`_render_frontend()` welds subprocess I/O to pure rendering** (flagged by:
  code-quality, architecture, test-coverage) — As sketched it shells out to npm
  *and* parses/sorts JSON in one function, yet the test plan wants to
  fixture-drive it. The pure transform must be a separate seam or it cannot be
  unit-tested without a live node tree.

### Tradeoff Analysis

- **Fast-`check`-lane speed vs drift-gate robustness**: The plan's design goal
  is keeping the drift check off the built `dist/` bundle so it stays in the
  fast read-only lane. That goal is what drives both the `--production`
  node_modules shortcut (which under-reports vs the bundle) and running real
  generators in `check` (which needs both closures provisioned and is
  host-sensitive). Recommendation: accept the completeness cost only if
  reconciled against the bundle at least in the `default` lane, and pay the
  provisioning cost explicitly via `depends` edges rather than hoping the lane
  warms the caches.
- **Over-approximation as house style vs the frontend's under-approximation**:
  The Rust side over-includes safely (praised by architecture and security) —
  but the `--production` frontend pass is the opposite direction (omission is
  the violation). The two halves do not share the safe posture; the frontend
  half needs to be pushed back to over-inclusion or bundle-reconciled.

### Findings

#### Critical

None. No lens raised a critical-severity finding; the REVISE verdict is driven
by the volume and reinforcement of major findings (well past the 3-major
threshold).

#### Major

- 🟡 **Compatibility / Architecture / Correctness / Safety**: `notices:check`
  runs in no provisioned CI lane and declares no `depends`
  **Location**: Phase 1 §4 (Task wiring); Migration Notes
  CI runs per-component jobs, not the aggregate `check`/`default`, so the gate
  never executes at pipeline time; and even run, the tasks declare no
  `deps:install:node` / cargo-fetch edge, so `--offline` cargo-about or a
  missing `node_modules` fails or false-greens. Mirror the `public-api:check`
  provisioning edge and add an explicit CI step.

- 🟡 **Security / Documentation / Correctness**: The artefact does not discharge
  MPL-2.0 §3.2 as specified
  **Location**: Phase 1 §2 (`about.hbs`); Success Criteria (Manual Verification)
  §3.2 requires making corresponding source available (or a written offer);
  reprinting the licence text does not. The plan names no source location, the
  block spec omits the statement, and cargo-about may omit `uluru` from the
  render entirely under default feature/target resolution — while the
  byte-compare still passes.

- 🟡 **Test Coverage**: The §3.2 statement and the entire Rust section have no
  automated test
  **Location**: Phase 1 §6 (unit tests); Testing Strategy
  Fixture tests cover only `_fold()`/`_render_frontend()`. The legally
  load-bearing §3.2 marker and the whole `about.hbs`-rendered Rust half (sort,
  verbatim text, copyright) are verified only by manual sampling, so a template
  regression ships green.

- 🟡 **Correctness / Security**: `--production` over `node_modules` can omit a
  bundled runtime dependency
  **Location**: Phase 1 §1 (`licenses:generate` script); What We're NOT Doing
  Vite bundles by import graph, not the deps/devDeps split. A runtime dep
  declared under `devDependencies` is bundled into the signed binary yet
  excluded by `--production` — the unsafe omission direction, and contrary to
  the AC's "over the built `dist/` bundle".

- 🟡 **Correctness / Architecture / Compatibility**: `about.toml` pins no
  `targets`/features, making the byte-compare host-dependent
  **Location**: Phase 1 §2 (`about.toml`); Performance Considerations
  cargo-about's closure and feature unification vary by host triple, so
  `_render_rust()` can emit different bytes on macOS vs Linux with an identical
  `Cargo.lock`. Pin `targets` to `deny.toml`'s four `[graph].targets`.

- 🟡 **Correctness**: Pruned `accepted` list can make cargo-about hard-error on
  a build/dev-dependency licence
  **Location**: Phase 1 §2 (`about.toml` `accepted`)
  cargo-about scans a broader manifest set than cargo-deny's pruned licence
  traversal (build scripts, proc-macros, possibly dev deps). A crate with a
  licence outside the copied ten-entry list aborts generation in the fast lane
  with an opaque error.

- 🟡 **Code Quality / Architecture**: `_render_frontend()` conflates the npm
  subprocess with pure rendering it claims to unit-test
  **Location**: Phase 1 §3 and §6
  The impure npm invocation is welded to the JSON parse/sort — the only branchy
  logic — so the fixture test either can't be written as described or silently
  exercises the real tree. Split an impure runner from a pure
  `renderer(json) -> str`.

- 🟡 **Code Quality**: Generator subprocess error handling dropped vs the
  `public_api` precedent
  **Location**: Phase 1 §3 (`_render_rust`/`_render_frontend` prose)
  `public_api.check` checks each `result.exited` and raises a specific Exit. The
  sketch omits this, so a generator/config failure masquerades as a confusing
  notices-drift error pointing at `notices:update`.

- 🟡 **Test Coverage**: `stage_notices` and its wiring into the prepare lanes
  are only manually verified
  **Location**: Phase 2 §2; Success Criteria (Manual Verification)
  The upload guard asserts enumeration, not that the file is copied. Omitting
  `build.stage_notices` from a lane passes every unit guard and fails only at
  release time — the exact silent-drop the guards were meant to prevent.

- 🟡 **Test Coverage**: `check()` isolation unspecified; missing-file branch
  untested; `_render_frontend` edge cases untested
  **Location**: Phase 1 §3 and §6
  `check()` shells out to real generators; the plan states no mocking/tmp
  redirect, so the drift/match tests risk being flaky or exercising the wrong
  path. The missing-file `Exit`, and missing-`licenseText`/duplicate-name
  frontend payloads, are not covered.

- 🟡 **Security / Compatibility**: cargo-about via `ubi:` is version-pinned, not
  hash-pinned — the "pinned under lockfile" claim is false
  **Location**: Phase 1 §1 (Tool pins); Current State Analysis
  `mise.lock` carries no checksum/url for `ubi:` backends (minisign, cosign) —
  only aqua entries hash-pin per platform. The tool that renders the legal
  artefact runs as a tag-mutable, unverified GitHub-release binary in CI.

- 🟡 **Documentation**: Notices-file content structure left implicit for a legal
  document
  **Location**: Phase 1 §2 and §3 (`_fold`)
  The header/preamble and per-block shape (including the §3.2 statement) are
  never defined, and the `about.hbs` spec lists only name/version/SPDX/
  copyright/text. The completeness of this file is the point of the work item;
  its structure should be planned, not discovered.

- 🟡 **Documentation**: `tasks/README.md` not updated for the new drift gate
  **Location**: Phase 1 §4 (missing README update)
  The repo's authoritative "learn once" task-tree map enumerates the standalone
  gates (`deny:check`, `pup:check`, `public-api:check`) and states no licence
  discharge exists — both go stale the moment `notices:check` lands.

#### Minor

- 🔵 **Architecture / Safety**: "Three independently mergeable phases" overstates
  the coupling
  **Location**: Implementation Approach; Phase 2
  Phase 2 references `RELEASE_NOTICES(_SOURCE)` and copies the committed file,
  both introduced in Phase 1, so it cannot land first. Reword to "sequentially
  mergeable (1 → 2 → 3)" and state the ordering constraint.

- 🔵 **Correctness / Compatibility**: `--frozen` = `--locked --offline`
  equivalence for cargo-about 0.9.2 is asserted, not verified
  **Location**: Key Discoveries
  If `--frozen` is not a `generate` flag in 0.9.2 the command errors; if it
  implies only `--offline`, a stale lockfile silently changes the closure.
  Confirm, else pass `--locked --offline` explicitly.

- 🔵 **Correctness**: Verbatim `licenseText` line-endings can cause byte-compare
  false positives
  **Location**: Phase 1 §3 (`_fold`/`check` comparison)
  Upstream CRLF or trailing-newline variance is copied byte-for-byte into the
  render while git/editor may normalise the committed file. Normalise line
  endings in `_fold()` and pin the artefact to LF via `.gitattributes`.

- 🔵 **Compatibility**: Byte-compare couples the artefact to exact tool-output
  formatting — coupling unacknowledged
  **Location**: Implementation Approach; Phase 1 §3
  A patch bump of either generator, or a `package-lock.json` refresh, fails
  `notices:check` with no dependency-policy change. Record the coupling in the
  Phase 3 rationale.

- 🔵 **Security**: `package-lock.json` regeneration for the new devDependency not
  called out
  **Location**: Phase 1 §1
  `deps:install:node` uses `npm ci`, which needs the lock in sync with
  `package.json`. Add an explicit install-and-commit step mirroring the
  `mise.lock` step.

- 🔵 **Security**: No in-pipeline integrity re-check of the unsigned compliance
  artefact
  **Location**: Phase 2 §3; What We're NOT Doing
  Unlike signed assets it gets no `_release_reverifies()` re-check before the
  draft→published flip; a truncated/stripped upload publishes green. Consider a
  post-upload digest re-check against the committed source.

- 🔵 **Documentation**: `deny.toml` comment rewrite is directional, with
  over-claim risk
  **Location**: Phase 2 §5
  No verbatim replacement is given; a loose rewrite risks implying the file is
  signed (it is not) or claiming §3.2 discharge that depends on the unresolved
  source-location gap. Draft the exact sentences.

- 🔵 **Safety**: Release lanes stage without re-verifying drift
  **Location**: Phase 2 §2
  Prepare lanes copy the committed file but do not run `notices:check`; currency
  rests wholly on branch protection enforcing `check` on `main`. Either run the
  check in the prepare lane as a backstop, or record the dependency explicitly.

- 🔵 **Code Quality**: Per-component block format duplicated across `about.hbs`
  and the Python renderer with no shared contract
  **Location**: Phase 1 §2 and §3
  The block shape lives twice with differing sort keys (Rust by `name`, frontend
  by `name@version`); the whole-file byte-compare cannot detect the two halves
  drifting in layout. Document the contract once and align the sort key.

- 🔵 **Architecture**: Byte-compare gate coupled to environment-variant
  `node_modules` resolution
  **Location**: Performance Considerations; Phase 1 §3
  Platform-specific optional/native transitive prod packages can vary the walked
  tree. Confirm `--production` determinism across supported platforms or scope
  to the lockfile-resolved set.

#### Suggestions

- 🔵 **Code Quality**: `check()` renders both generators before verifying the
  file exists — reorder to fail fast
  **Location**: Phase 1 §3 (sketched `check()`)
  Move the `RELEASE_NOTICES_SOURCE.exists()` guard above `_render()`, matching
  `public_api.check`'s ordering, so the missing-file path skips the dual-render
  cost.

- 🔵 **Code Quality**: Constant names lean on incidental "notices" over the
  domain's "attribution artefact"
  **Location**: Phase 1 §3 / Phase 2 §1 (paths.py constants)
  Consider `ATTRIBUTION_ARTEFACT(_SOURCE)` to mirror the work item's vocabulary
  and make the committed-vs-staged pair obvious. Low priority.

- 🔵 **Test Coverage**: Fold/render fixtures need mutation-killing inputs
  **Location**: Phase 1 §6
  Feed deliberately out-of-order crates and a multi-line licence body with
  quotes/newlines, asserting the verbatim substring appears unmodified — so a
  broken sort or text-mangling change actually fails.

### Strengths

- ✅ Rejects the work item's `TREE_ARTIFACTS` route in favour of the
  `RELEASE_MANIFEST` single-file template — a correct boundary decision that
  avoids the per-platform satellite tables (smoke-checks, `pins.toml` digests,
  `TreeSpec`, the exact-set manifest gate), verified against `github.py:258-277`.
- ✅ The unconditional `uploads.append(RELEASE_NOTICES)` is correct and is seen
  by both coverage guards despite their differing `tree_tokens`; `stage_notices`
  runs in the same job as finalise, so the pre-flight existence assertion never
  fires on a non-staged file.
- ✅ The upload append fails safe: `upload_and_verify_release` computes `missing`
  and raises before the `--clobber` loop, so a missing file fails the whole
  release rather than causing a partial upload.
- ✅ Both positive upload-presence assertions are genuinely red-first, and the
  attest-glob coverage rides the existing per-path loop, so the notices file is
  proven glob-covered with no bespoke glob logic.
- ✅ Faithfully reuses the `public_api` byte-compare model and the
  `context.cd(CLI_DIR)` idiom; error messages name the exact remediation
  command; no comments introduced, honouring the repo's low-comment posture.
- ✅ The `about.toml` `accepted` list is a precise superset of `deny.toml`'s
  allow-list plus MPL-2.0, and re-sorting the license-checker JSON neutralises
  JS key-ordering nondeterminism — the right instincts for reproducibility.
- ✅ Keeping the file unsigned but attested (rides the `accelerator-*` SLSA
  glob) is a defensible scope decision: the launcher resolves nothing against
  it, so it does not widen the verified trust surface.

### Recommended Changes

1. **Wire and provision the drift gate for real** (addresses: `notices:check`
   runs in no provisioned CI lane; no `depends` edge; no job with both
   closures). Add `depends = ["deps:install:node", <cargo-fetch/cli edge>]` to
   both `notices:check` and `notices:update`; add an explicit `mise run
   notices:check` step to a named CI job that provisions both `node_modules`
   (via `npm ci`) and the cargo source cache, following the `public-api:check`
   precedent rather than aggregate-task membership.

2. **Make §3.2 actually dischargeable and tested** (addresses: §3.2 not
   discharged; source location unspecified; `uluru` may be omitted; no §3.2
   test). Decide and document the corresponding-source mechanism (crate
   repository URL / written offer), add it to the `about.hbs` block spec, pin
   cargo-about's `targets`/features to the release closure so `uluru` is
   rendered, and add an automated assertion that the Rust section contains
   `uluru`/`MPL-2.0` with a source marker.

3. **Close the frontend under-reporting gap** (addresses: `--production` misses
   bundled dev-declared deps). Reconcile the `--production` set against the
   actual Vite bundle (bundle-time plugin or a diff against `dist/` imports),
   at least in the `default` lane, or document and justify why no
   production-bundled module can originate from a `devDependency`.

4. **Pin byte-compare determinism** (addresses: host-dependent `about.toml`;
   line endings; node_modules variance). Pin `about.toml` `targets` to
   `deny.toml`'s four triples and fix the feature set; normalise line endings /
   trailing newline in `_fold()` and add a `.gitattributes` LF pin; confirm
   `--production` resolves identically across platforms or scope to the lock.

5. **Split the frontend generator seam** (addresses: `_render_frontend`
   conflation; dropped error handling). Separate an impure
   `_run_license_checker(context) -> str` from a pure `_render_frontend(payload)
   -> str`; check each generator's exit status and raise a distinct Exit before
   the byte comparison; point fixture tests at the pure function.

6. **Fill the remaining test and doc gaps** (addresses: Rust-section coverage,
   `stage_notices` wiring, `check()` isolation, `tasks/README.md`, `deny.toml`
   wording, `package-lock.json`). Add Rust-section/structural assertions over
   the committed file; a spy test that both prepare lanes invoke
   `stage_notices`; specify `check()` isolation and the missing-file case;
   update `tasks/README.md`; draft the exact `deny.toml` replacement sentences;
   add the `npm install` + `package-lock.json` commit step.

7. **Correct the two overstated claims** (addresses: "independently mergeable";
   "pinned under lockfile"). Reword phases as sequentially mergeable (1 → 2 →
   3), and state that the `ubi:` pin is version-not-hash pinning like
   minisign/cosign; verify cargo-about 0.9.2 publishes ubi-resolvable assets for
   all three platforms.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: Architecturally well-grounded — correctly rejects `TREE_ARTIFACTS`
for the `RELEASE_MANIFEST` template, the unconditional append is seen by both
guards, and the over-approximation posture matches house style. The one material
structural weakness: the two new drift tasks declare no provisioning
dependencies and silently rely on sibling gates' side effects to warm their
inputs, undermining the DAG in the fast `check` lane.

**Strengths**:
- Rejects `TREE_ARTIFACTS` for the `RELEASE_MANIFEST` single-file template — a
  well-reasoned boundary decision avoiding the per-platform satellite machinery.
- Unconditional `uploads.append(RELEASE_NOTICES)` mirrors `RELEASE_MANIFEST` and
  is correctly visible to both guards despite differing `tree_tokens` — verified
  against `github.py:258-277`.
- Over-approximation posture consistent with the codebase's fail-closed licence
  conventions (`write_notices`, `deny.toml` pruning).
- Pure `_fold()` separated from generator I/O and fixture-tested, mirroring
  `public_api`.
- Naming (`accelerator-` prefix, flat) treated as a load-bearing constraint
  keeping the three attest globs untouched.

**Findings**:
- **major/high** — `notices:check` declares no provisioning dependencies and
  relies on sibling-gate side effects (Phase 1 §4). mise resolves `check.depends`
  as an unordered concurrent set; nothing guarantees `cli:check` warms the cargo
  cache before `notices:check`. Give the tasks explicit `depends` edges.
- **minor/medium** — `_render_frontend()` couples subprocess I/O with pure
  rendering it claims to unit-test (Phase 1 §3). Split imperative shell from
  functional core.
- **minor/high** — Phase 2 hard-depends on Phase 1, so "independently mergeable"
  overstates the coupling (Implementation Approach). Only Phase 3 is order-free.
- **minor/low** — Byte-compare gate coupled to environment-variant `node_modules`
  resolution (Performance Considerations). Confirm `--production` determinism
  across platforms.

### Code Quality

**Summary**: Closely and faithfully models the `public_api` update/check pair
with guard-clause error handling, actionable Exit messages, and no comments
introduced. Main risk in `tasks/notices.py`: the frontend generator conflates an
impure npm subprocess with the pure JSON→text rendering the tests want to
fixture-drive, and the subprocess error-handling `public_api` performs is
dropped. The block format is also duplicated across template and renderer with
no single source of truth.

**Strengths**:
- Faithfully reuses the `public_api.py` byte-compare precedent and the `deny.py`
  `context.cd(CLI_DIR)` idiom.
- Error paths use guard clauses with specific messages naming `mise run
  notices:update`.
- Pure `_render`/`_fold` with side-effecting `update`/`check` at the edges — the
  right instinct for testability.
- No comments introduced, honouring the repo's low-comment tolerance.

**Findings**:
- **major/high** — `_render_frontend` conflates the npm subprocess with pure JSON
  rendering, defeating the stated fixture test (Phase 1 §3, §6). Split
  `_run_license_checker` from a pure `_render_frontend(payload)`.
- **major/medium** — Generator subprocess error handling dropped relative to the
  `public_api` precedent (Phase 1 §3). A generator failure masquerades as a
  drift error. Capture each result and raise a distinct Exit.
- **minor/medium** — Per-component block format duplicated across `about.hbs` and
  the Python renderer with no shared contract, differing sort keys (Phase 1 §2,
  §3). Document once, align sort key.
- **suggestion/high** — `check()` runs both generators before verifying the file
  exists — reorder to fail fast (Phase 1 §3).
- **suggestion/low** — Constant names lean on incidental "notices" rather than the
  domain's "attribution artefact" (paths.py constants).

### Test Coverage

**Summary**: Genuinely test-driven for the wiring it touches — the two
upload-presence guards are red-first and ride the existing attest-glob loop, and
`_CHECK_GATES` pins the drift gate into `check`. But coverage is thin exactly
where compliance risk is highest: the §3.2 statement and the entire Rust-section
rendering are verified only by manual sampling, `about.hbs`/`_render_rust` get no
automated test, and the release-staging wiring has no unit coverage. `check()`
isolation is unspecified.

**Strengths**:
- The two positive upload-presence assertions are genuinely red-first.
- Attest-glob coverage rides the existing loop — no bespoke glob logic to get
  wrong.
- The `_CHECK_GATES` parametrised gate-placement test is a strong, cheap
  regression guard.
- Fold/render logic factored as pure functions and fixture-tested at the right
  pyramid level.
- The `update && check` round-trip maps onto the AC's reproducibility criterion.

**Findings**:
- **major/high** — The §3.2 MPL source statement has no automated test (Phase 1
  §6; Testing Strategy). Add a substring guard over the committed artefact.
- **major/high** — Rust-section rendering (`about.hbs`, `_render_rust`) has zero
  automated coverage (Phase 1 §2, §6). Render over a fixture crate set or assert
  structural invariants.
- **major/medium** — `stage_notices` and its wiring into the prepare lanes are
  only manually verified (Phase 2 §2). Add a spy test.
- **major/medium** — `check()` unit-test isolation is unspecified; missing-file
  branch untested (Phase 1 §3, §6). Specify monkeypatching and a third case.
- **major/medium** — `_render_frontend` edge cases untested — missing
  licenseText/copyright, duplicates, empty payload (Phase 1 §6).
- **minor/medium** — Verbatim-preservation and sort assertions need
  mutation-killing fixtures (Phase 1 §6).
- **minor/medium** — Byte-compare determinism guarded only by a single round-trip
  (Testing Strategy).

### Correctness

**Summary**: The upload-set integration is logically sound — the unconditional
append correctly mirrors `RELEASE_MANIFEST`, and `stage_notices` runs in the same
job as finalise, so the existence assertion is satisfied on every path. The risk
concentrates in the two generators' determinism and closure-completeness:
cargo-about's feature/target resolution is unpinned, so the drift check can be
host-dependent and can under-report the very MPL/gix closure the artefact exists
to discharge.

**Strengths**:
- The unconditional append is correct — `stage_notices` is wired into both
  prepare lanes, both in the same runner/job as the later upload.
- Re-sorting the license-checker JSON by `name@version` neutralises key-ordering
  nondeterminism.
- The `accepted` list is a precise superset of `deny.toml`'s allow-list plus
  MPL-2.0.
- Reusing the `public_api` model is the right structural choice.

**Findings**:
- **major/medium** — cargo-about feature/target resolution may under-report the
  linked closure, silently omitting the `uluru`/gix MPL sub-closure (Phase 1 §2;
  Manual Verification). Pin resolution to the release build and assert `uluru`
  appears.
- **major/medium** — Unpinned targets/features make the byte-compare
  host-dependent, causing drift-check false positives across platforms (Phase 1
  §2, §3). Pin `targets` to `deny.toml`'s four triples.
- **major/high** — `notices:check` has no `depends`, so it can run before
  node_modules/cargo sources exist (Phase 1 §4).
- **major/medium** — `accepted` pruned to deny's linked closure can make
  cargo-about hard-error on a build/dev-dependency licence (Phase 1 §2).
- **major/medium** — `--production` over node_modules misses a runtime dep
  mis-declared in devDependencies — the unsafe omission direction (Phase 1 §1;
  What We're NOT Doing).
- **minor/medium** — `--frozen` = `--locked --offline` equivalence for cargo-about
  0.9.2 asserted, not verified (Key Discoveries).
- **minor/medium** — Embedded verbatim licenseText line-endings and git
  normalisation can cause byte-compare false positives (Phase 1 §3).

### Compatibility

**Summary**: Additive and consumer-safe on the release surface — a new unsigned
upload rides the existing attest glob and `RELEASE_MANIFEST` template with no
contract changes, and exact version pins plus `npm ci` give the drift check a
reproducible input. The real gaps are provisioning and cross-environment
execution: CI runs per-component checks in isolated jobs (not the aggregate
`check`), no single job carries both the warm cargo registry and `node_modules`,
the new tasks declare no `deps:install:node`, and the plan overstates what `mise
lock` pins for the ubi backend.

**Strengths**:
- Additive upload only — reuses the existing attest glob and `RELEASE_MANIFEST`
  template, no consumer-facing contract change.
- Both generators pinned to exact versions; frontend closure resolved via `npm
  ci` from a committed lock.
- Correctly follows the `ubi:` precedent (minisign/cosign) for a tool absent from
  the aqua registry.

**Findings**:
- **major/high** — `notices:check` wired only into the aggregate `check`/`default`,
  which CI never runs as a single job (Phase 1 §4; Migration Notes). Add an
  explicit CI step mirroring `public-api:check`.
- **major/high** — No CI job provisions both the warm cargo registry and
  node_modules the dual generator requires (Migration Notes; Performance). Name a
  host job that provisions both.
- **major/high** — notices tasks declare no `deps:install:node` (or cargo fetch)
  dependency (Phase 1 §4).
- **minor/medium** — `mise lock` does not hash-pin the ubi backend across
  platforms — the claimed cross-platform pinning does not apply (Key Discoveries;
  Phase 1 §1).
- **minor/medium** — Byte-compare couples the committed artefact to exact
  tool-output formatting — coupling not acknowledged (Phase 1 §3).

### Security

**Summary**: Framed as a supply-chain / licence-compliance change, the biggest
exposures are an unverified new build tool and a legal artefact that can
under-report an undischarged obligation. The `ubi:cargo-about` pin is
version-only — `mise.lock` carries no checksum for `ubi:` backends — so the tool
generating the legal artefact runs as an unverified GitHub-release binary in CI.
The frontend enumeration and the described §3.2 handling are both plausibly
incomplete. The unsigned-but-attested decision itself is defensible.

**Strengths**:
- The artefact rides the existing `accelerator-*` SLSA attest glob, so it
  genuinely receives build provenance.
- The `public_api`-style byte-compare gives tamper/drift detection against silent
  edits.
- Both generators fail in the safe over-inclusive direction (cargo-about fails
  closed; node_modules over-approximates the bundle).
- Unsigned but not a trust anchor is a reasonable scope decision — the launcher
  resolves nothing against it.

**Findings**:
- **major/high** — cargo-about pinned via ubi is not hash-pinned — an unverified
  binary generates the legal artefact (Phase 1 §1; Current State Analysis).
  Prefer aqua if available, or document the accepted unverified surface and
  correct the "pinned under lockfile" claim.
- **major/medium** — Reproducing MPL-2.0 licence text does not discharge §3.2
  corresponding-source availability (Phase 1 §2; Manual Verification). Emit a
  concrete source pointer per MPL crate and assert its presence.
- **major/medium** — Enumerating production node_modules can under-report code
  Vite inlines from devDependencies (Phase 1 §1; What We're NOT Doing).
  Reconcile against the built bundle.
- **minor/medium** — No in-pipeline integrity re-check of the compliance artefact
  before the draft→published flip (Phase 2 §3). Add a post-upload digest check.
- **minor/medium** — `package-lock.json` regeneration for the new devDependency
  is not called out (Phase 1 §1). `npm ci` requires the lock in sync.

### Documentation

**Summary**: Strong on process documentation (drift-check Exit messages name the
regeneration command; the Phase 3 rationale note satisfies AC bullet 5 well) but
under-specifies the content of the two documentation surfaces that matter most.
The generated notices file is itself a legal-compliance document whose header,
§3.2 statement, and the source it must resolve to are all left implicit — a gap
against AC bullet 2. And the plan never updates `tasks/README.md`.

**Strengths**:
- Drift-check Exit messages name the exact regeneration command.
- Phase 3's rationale note enumerates the substantive decisions — a complete
  answer to AC bullet 5.
- Phase 1 manual-verification bullets check for verbatim text, copyright, and a
  §3.2 statement per MPL component.

**Findings**:
- **major/high** — Notices-file content structure left implicit for a legal
  document (Phase 1 §2, §3). Specify header/preamble and per-block shape;
  add §3.2 to the `about.hbs` spec.
- **major/high** — §3.2 "means of obtaining source" location never specified
  (Phase 1 Manual Verification; Desired End State). Decide and document the
  source mechanism and the exact statement wording.
- **major/high** — Task-tree documentation not updated for the new notices drift
  gate (Phase 1 §4). Extend `tasks/README.md`'s gate enumeration and
  licence-mechanism paragraph.
- **minor/medium** — `deny.toml` comment rewrite is directional, with over-claim
  risk (Phase 2 §5). Draft the exact replacement sentences.

### Safety

**Summary**: Wires the artefact into the signed-release path in the safe
direction — the append is guarded by an atomic pre-flight existence check that
runs before any `--clobber` upload, so a missing file fails the release cleanly,
and `stage_notices` is a deterministic copy of a `check`-guarded committed file.
The chief unaddressed weakness is the drift guard itself: `notices:check`
declares no provisioning `depends`, so it can run against a stale `node_modules`
and pass green against an understated closure — failing open.

**Strengths**:
- The upload append is protected by an atomic pre-flight gate (`missing` computed
  before the `--clobber` loop), so a missing file can never cause a partial
  upload.
- `stage_notices` only copies a committed, `check`-guarded source; accidental
  deletion is caught in `check`.
- Unsigned, not a trust anchor, rides the existing attest glob with no `main.yml`
  edit.
- Idempotent staging via `shutil.copy2`.

**Findings**:
- **major/medium** — `notices:check` declares no provisioning depends edge — the
  compliance guard can misfire or false-green on a partial/stale node_modules
  (Phase 1 §4, §3). Add `depends` and require `npm ci`.
- **minor/medium** — Release lanes stage the committed artefact without
  re-verifying drift (Phase 2 §2). Run `notices:check` in the prepare lane or
  record the branch-protection dependency.
- **minor/high** — Unconditional append makes the committed text file a
  release-critical pre-flight asset; failure surfaces after the tag is pushed
  (Phase 2 §3). Keep phase ordering strict; assert every `stage_*` step is
  invoked by both prepare lanes.

## Re-Review (Pass 2) — 2026-08-31

**Verdict:** APPROVE

The revision resolved the entire pass-1 REVISE cluster — the drift gate is
provisioned and CI-wired, §3.2 is discharged by per-crate source URLs with an
automated assertion, the frontend under-report is guarded, byte-compare
determinism is pinned, and the generator seam is split. Pass 2 surfaced a fresh
batch of findings, but they were implementation-detail defects concentrated in
the pass-1 edits themselves (a broken CI-job spec, an unsound spy test, a guard
in a non-building lane), not design gaps — all have been fixed in-place. No
design-level blocker remains; the plan is ready to implement.

### Previously Identified Issues (Pass 1)

All pass-1 findings resolved, most fully:

- 🟡 **Compatibility/Architecture/Correctness/Safety**: `notices:check` runs in
  no provisioned CI lane / no `depends` — **Resolved**. Dedicated
  `check-attribution` job; `deps:install:node` + `deps:install:cargo-sources`
  edges; added to `prerelease.needs`.
- 🟡 **Security/Documentation/Correctness**: §3.2 not discharged — **Resolved**.
  Per-crate repository + immutable crates.io download URL; targets pinned so
  `uluru` renders; automated MPL-generic §3.2 assertion.
- 🟡 **Test Coverage**: §3.2 and Rust section untested — **Resolved**.
  `_render_rust` fixture, generic committed-artefact §3.2 parse, LF-normalisation
  and generator-failure cases added.
- 🟡 **Correctness/Security**: `--production` omits bundled dev-declared deps —
  **Resolved**. Bundled-import guard, now correctly laned and with a negative
  fixture.
- 🟡 **Correctness/Architecture/Compatibility**: unpinned `about.toml` targets —
  **Resolved**. Pinned to the four shipped triples (five-vs-four corrected).
- 🟡 **Correctness**: pruned `accepted` may hard-error — **Resolved** (scoped;
  reconciliation flagged for implementation).
- 🟡 **Code Quality/Architecture**: `_render_frontend` conflation — **Resolved**
  (functional-core/imperative-shell split).
- 🟡 **Code Quality**: generator error handling dropped — **Resolved** (distinct
  `Exit` per generator, now with tests).
- 🟡 **Test Coverage**: `stage_notices` wiring / `check()` isolation / frontend
  edge cases — **Resolved** (integration spy; isolation + missing-file;
  edge-case fixtures).
- 🟡 **Security/Compatibility**: ubi not hash-pinned — **Resolved** (language
  corrected; job-isolation rationale recorded; `package-lock.json` step added).
- 🟡 **Documentation**: content structure / §3.2 location / `tasks/README.md` —
  **Resolved** (header, per-block shape, source location, README edits, exact
  `deny.toml` wording).
- 🔵 Minors (phase mergeability wording, `--frozen` proviso, line endings,
  tool-output coupling) — **Resolved**.

### New Issues Introduced (Pass 2) — all fixed in-place

- 🟡 **Compatibility**: `check-attribution` omitted `RUSTUP_HOME` routing and a
  distinct `cache_key_prefix` (breaks on cache-hit) — **Fixed**: both added, job
  now mirrors `check-architecture` completely.
- 🟡 **Test Coverage/Correctness**: the `test_release.py` spy was unsound
  (drove heavy prepare lanes with one collaborator stubbed; invented
  `monkeypatch_spy`) — **Fixed**: moved to extend the existing
  `TestPrereleasePrepare` integration harness.
- 🟡 **Compatibility/Correctness**: `cargo fetch` may not extract sources the
  offline read needs — **Fixed**: flagged for verification with a `--locked`
  fallback.
- 🟡 **Architecture/Correctness**: cargo-registry provisioning not a declared
  task edge — **Fixed**: added `deps:install:cargo-sources`.
- 🟡 **Architecture/Safety/Security**: bundled-import guard in a non-building
  lane — **Fixed**: hosted in the `test:unit` group that builds the bundle
  (already gated), structured enumeration, fail-loud, negative fixture.
- 🟡 **Safety/Compatibility**: drift gate not in the release `needs` chain —
  **Fixed**: added `check-attribution` to `prerelease.needs`; branch-protection
  required-check update called out.
- 🟡 **Documentation**: README step targeted nonexistent text; missing CI-job
  table row — **Fixed**: reworded against real text; row added.
- 🟡 **Correctness/Compatibility**: `deny.toml` has five targets, plan said four
  — **Fixed** everywhere.
- 🟡 **Test Coverage/Code Quality**: generator-failure branch and `_fold` LF
  normalisation untested — **Fixed**: cases added; "verbatim unmodified" wording
  corrected to content-preserved/endings-normalised.
- 🔵 Minors (crates.io download endpoint, generic MPL §3.2 guard, `deny.toml`
  line citation, header-preamble claim, frontend §3.2 assumption, block-contract
  home, optional-dep determinism invariant) — **Fixed/noted**.

### Deliberately not changed

- 🔵 **Code Quality**: `ATTRIBUTION_ARTEFACT` spelling and the `_SOURCE`
  suffix — **resolved after the re-review**: standardised on British spelling
  (the sibling `TREE_ARTIFACTS` is renamed to `TREE_ARTEFACTS`, Phase 2 §6), and
  the suffix flipped so the committed file is the base `ATTRIBUTION_ARTEFACT` and
  the staged copy is `ATTRIBUTION_ARTEFACT_STAGED`.
- 🔵 **Code Quality**: the impure `_render` orchestrator shares a stem with the
  pure `_render_*` functions — a low-confidence naming nicety, left to
  implementation.

### Assessment

The plan is ready to implement. The pass-2 verdict rests on fixes applied
in-place after the re-review agents ran, so the residual risk is that those
fixes were not themselves re-reviewed; each is a narrow, verifiable correction
rather than a design change. Two open items are genuinely the implementer's to
confirm against the live toolchain — whether `cargo fetch` alone satisfies
offline cargo-about, and cargo-about 0.9.2's dependency-kind filtering for the
`accepted` list — both flagged inline. The naming/spelling nit is now settled
(British throughout, suffix flipped).

---
*Re-review generated by /accelerator:review-plan*
