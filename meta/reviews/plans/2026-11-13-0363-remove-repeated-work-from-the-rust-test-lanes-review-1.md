---
type: "plan-review"
id: "2026-11-13-0363-remove-repeated-work-from-the-rust-test-lanes-review-1"
title: "Plan Review: Remove Repeated Work from the Rust Test Lanes Implementation Plan"
date: "2026-11-13T22:39:14+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-11-13-0363-remove-repeated-work-from-the-rust-test-lanes"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "performance", "standards", "documentation", "compatibility"]
review_number: 1
review_pass: 2
tags: ["build-system", "performance", "testing", "rust", "coverage"]
last_updated: "2026-11-14T09:36:51+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Remove Repeated Work from the Rust Test Lanes Implementation Plan

**Verdict:** REVISE

Phases 1 and 2 are strong. Phase 1's file-rotate claims check out against
the 0.7.6 source, and every mutation is caught. Phase 2 splits the work into
a pure, unit-tested domain module behind a thin task body that follows the
existing `npm_audit` pattern, and each step fails loudly. Most of the weight
falls on Phase 3. Its coverage floor sits within 0–1 pp of a single CI
sample. It is enforced in every environment, including local runs with
stale instrumented binaries, root runs that skip the permission tests, and
toolchain bumps. Its unit tests and docstring instruction are coupled to
Phase 2 in ways that will break.

### Cross-Cutting Themes

- **The floor is fragile across environments** (flagged by: correctness,
  compatibility, architecture). A whole-percent round-down of the lower CI
  leg leaves almost no headroom. Things outside the change under review
  move the totals: stale instrumented binaries left by `--no-clean`,
  `ACCELERATOR_TEST_SKIP_PERMISSION_TESTS`, toolchain or llvm-tools bumps,
  and new low-coverage crates. The "never lowered" rule leaves no sanctioned
  way to re-baseline.
- **Stale instrumented binaries under `--no-clean`** (flagged by:
  correctness, compatibility, performance). Profraw-only cleaning removes
  stale *profiles* but not stale *binaries*. After a lockfile or toolchain
  bump, superseded binaries may be picked up as report objects, which skews
  the totals and slowly grows report time and disk use. The plan's one
  equality check runs straight after a full clean, so it never covers this
  state.
- **Phase 3 docstring instruction points at removed text** (flagged by:
  documentation, code-quality). Phase 3 says to "replace 'report-only'", but
  Phase 2's rewrite has already removed that wording.
- **The report and nextest commands set their scope and features
  separately** (flagged by: architecture, code-quality, test-coverage).
  `_report_totals` restates `--manifest-path … --all-features` by hand. The
  report's scope rests on a dependency invariant with a test-support
  loophole, and no unit test pins either.
- **The metric trio is joined by reflection** (flagged by: code-quality,
  architecture). `CoverageFloor.shortfalls` relies on `asdict`/`getattr`
  with field names that merely match.
- **The docstring narrates the step sequence** (flagged by: documentation,
  standards). The opening sentence restates what the helper names already
  say.

### Tradeoff Analysis

- **A strict gate vs a reliable gate**: the user asked for a floor, and a
  tight floor catches regressions sooner. But with 0–1 pp of headroom and
  several environmental inputs, `default` (the CI mirror) can fail with no
  code change. Recommendation: keep the gate, but add explicit headroom and
  a documented re-baselining rule for toolchain bumps.
- **Self-healing staleness vs simplicity**: a lockfile or toolchain
  fingerprint that triggers a full clean adds machinery that "What We're NOT
  Doing" explicitly declines. Recommendation: at minimum, point the
  floor-failure message at the `clean --workspace` remedy and add a manual
  check after a dependency bump. Self-healing can wait until drift is
  observed.

### Findings

#### Major

- 🟡 **Correctness / Compatibility / Performance**: Stale instrumented
  binaries accumulate under `--no-clean` and skew the totals the floor judges
  **Location**: Phase 2: Task sequence / What We're NOT Doing; Phase 3
  Cargo's metadata hash changes on lockfile or toolchain bumps, leaving old
  binaries that `report` may pass to llvm-cov as objects. With the floor in
  force, local runs can fail while CI (where rust-cache prunes these
  artefacts) passes.
- 🟡 **Correctness / Architecture**: Floor margin can be near zero and comes
  only from CI environments
  **Location**: Phase 3, Section 1: Choose the starting floors
  A total of 77.01% gives a floor of 77, a 0.01 pp margin. Local runs,
  cfg-gated paths and nondeterministic branches never fed into the floor.
- 🟡 **Compatibility**: Floor is coupled to the pinned Rust/LLVM and
  cargo-llvm-cov versions, with no re-baselining rule
  **Location**: Phase 3: Choose the starting floors / Docstring
  Region and function counts shift between rustc/LLVM releases. The
  "never lowered" rule leaves a routine toolchain bump with no sanctioned
  way through.
- 🟡 **Compatibility**: Floor will fail in root or container runs that set
  `ACCELERATOR_TEST_SKIP_PERMISSION_TESTS`
  **Location**: Phase 3: Enforce the floor in the task
  Skipped permission tests lower coverage of exactly the code they exercise,
  in a configuration the task explicitly supports.
- 🟡 **Test Coverage**: The real `COVERAGE_FLOOR` couples every sequence test
  to the production floor values
  **Location**: Phase 3, Section 3
  Only the two floor tests monkeypatch it. Every other coverage-on test
  would start raising `Exit` once the floor exceeds `EXPORT`'s totals, or
  when the floor is later raised.
- 🟡 **Architecture**: Report scope depends on a dependency-graph invariant
  with a test-support loophole
  **Location**: Key Discoveries / What We're NOT Doing (no
  `--ignore-filename-regex`)
  The lint allows test-support crates to depend on the visualiser. If one
  does, visualiser sources silently enter the totals that the floor gates
  on.
- 🟡 **Documentation**: Phase 3 tells the implementer to replace
  "report-only" text that Phase 2 already removed
  **Location**: Phase 3: Enforce the floor in the task (Docstring)
  The edit target won't exist, and between the two phases nothing says the
  lane is report-only.

#### Minor

- 🔵 **Correctness**: The `MAX_BYTES * 2` mutation-table row predicts the
  wrong outcomes for tests 2 and 3
  **Location**: Phase 1, Section 2: How each mutation is caught
  Test 2 gets `[10 MiB, 10 MiB]`, not an empty list. Test 3 counts 2 files,
  not 1. The mutation is still caught.
- 🔵 **Correctness**: The shortfall message can print a total equal to its
  floor
  **Location**: Phase 3, Section 2: `Shortfall.__str__`
  76.996 against a floor of 77 prints `lines 77.00% (floor 77%)`, because
  the comparison uses the unrounded value while the message rounds.
- 🔵 **Test Coverage**: The fate of `test_raises_when_inner_tests_fail` is
  unspecified, and a coverage-off failure is untested
  **Location**: Phase 2, Section 3
  After the change, that test's `exited=1` makes the *clean* step fail
  first, so its name lies. Nothing tests a failing plain nextest run.
- 🔵 **Test Coverage**: Manifest and feature flags on the clean and report
  commands aren't asserted
  **Location**: Phase 2, Section 3
  Dropping `--all-features` from the report, or the visualiser exclusion
  from nextest, would pass every unit test.
- 🔵 **Test Coverage**: The parser fixture is hand-built, not taken from real
  output
  **Location**: Phase 2, Section 2
  The work item's Technical Notes describe a different export shape
  ("totals object only") from the plan's `data[0].totals`. A trimmed real
  0.8.7 export would settle it.
- 🔵 **Test Coverage**: Floor tests don't confirm each metric is compared
  against its own floor
  **Location**: Phase 3, Section 2
  Parametrise the single-shortfall case over all three metrics, with
  distinct floors.
- 🔵 **Code Quality / Architecture**: Floor and totals are matched through
  reflection on field names
  **Location**: Phase 3, Section 2
  `asdict`/`getattr` couples two dataclasses by name alone. The trio is
  restated in four places.
- 🔵 **Code Quality / Architecture**: The report's `--all-features`
  duplicates the feature set in `_MANIFEST`
  **Location**: Phase 2, Section 3 (`_report_totals`)
  "Report and tests share a build" is stated in prose rather than in code.
- 🔵 **Code Quality**: Three slightly different ways to list log segments in
  one test module
  **Location**: Phase 1, Section 2
  `server.log.` and `server.log` differ by one dot. `count_segments` is
  kept conditionally.
- 🔵 **Code Quality**: A magic `3` plus an explanatory assertion message in
  the permissions test
  **Location**: Phase 1, Section 2
  Bind `rotations = 2` and assert `rotations + 1`.
- 🔵 **Code Quality**: One error message describes non-JSON output as "no
  totals"
  **Location**: Phase 2, Section 2
  Distinguish `JSONDecodeError` from a missing structure.
- 🔵 **Standards**: `FIVE_MEBIBYTES` is named after its value, and its MiB
  clashes with the test name's "megabytes"
  **Location**: Phase 1, Section 2
  The sibling constants are named by role.
- 🔵 **Standards / Documentation**: The docstring narrates steps the code
  already shows
  **Location**: Phase 2, Section 3
  Keep only the why: the full-clean cost, the merge hazard, the remedy and
  the off switch.
- 🔵 **Standards / Documentation**: Floor wording goes into the
  `ACCELERATOR_COVERAGE` env-var row
  **Location**: Phase 3: Descriptions (`tasks/README.md:778`)
  The variable doesn't control the floor. Document the floor policy (where
  it lives, how it's chosen, the ratchet rule) alongside the lane.
- 🔵 **Documentation**: The shortfall failure message gives no way to fix it
  **Location**: Phase 3, Section 3
  Name `COVERAGE_FLOOR`'s location and/or the `clean --workspace` remedy.
- 🔵 **Documentation**: Reversing the 0162 report-only decision is recorded
  only in this plan
  **Location**: Overview item 3
  Record it on the 0363 work item, a new work item, or an ADR.
- 🔵 **Performance**: The CPU budget mixes test cost with cargo and nextest
  start-up, and uses a different runner from the lane
  **Location**: Phase 1: Manual Verification
  Also time the prebuilt test binary or the lane's own `cargo test`.
- 🔵 **Performance**: Nothing watches artefact growth in `llvm-cov-target`
  **Location**: Phase 2 / What We're NOT Doing
  Report time and disk use may creep up with every lockfile update.

#### Suggestions

- 🔵 **Code Quality**: Floor judgement is inlined into `_run_instrumented`,
  breaking its step-per-helper shape. Extract `_enforce_floor`.
- 🔵 **Documentation**: The `coverage_enabled` docstring
  (`tasks/shared/rust.py:44`) says "coverage reported" and goes out of date.
- 🔵 **Standards**: The Rust sample exceeds 80 columns inside `mod tests`.
  Run `mise run fix` before `cli:check`.
- 🔵 **Test Coverage**: The mutation check sits under Automated
  Verification, but it is a manual step. Consider tightening the active
  segment check to `assert_eq!(active_len, OVERSHOOT)`.
- 🔵 **Performance**: The profraw-only clean cost is projected from 18
  files, but each run will delete about 9,200. Time it separately.
- 🔵 **Compatibility**: When recording the CI legs' totals, also confirm
  that each leg ran the three steps with no mismatched-data or
  missing-object warnings.

### Strengths

- ✅ The file-rotate 0.7.6 claims are verified against its source: the split
  at the cap, multiple caps in one write, rotation only past the cap, and
  `AppendCount::new(3)`. The 64 KiB overshoot never lands on a cap
  boundary.
- ✅ Phase 1 closes a real mutation gap. Literal-value assertions catch all
  four mutations, which the current tests miss for `MAX_FILES`. The red step
  is set up properly for a test-only change.
- ✅ Domain logic sits in a pure `tasks/shared/coverage.py` with its own
  tests, mirroring `npm_audit.py`. The task body only runs commands and maps
  failures to `Exit`.
- ✅ `_run_instrumented` reads as intention-named steps, each with one guard
  clause and its own error message. Failures short-circuit, so the failure
  path wastes no work.
- ✅ The sequence test pins `--profraw-only` before `--no-clean`. This guards
  against the bare `--no-clean` regression, which inflates totals (89.94% vs
  77.99%).
- ✅ The plan takes care over hidden coupling: `_MANIFEST` is shared with
  `integration.py`, `test_nextest_filter.py` bans the `--profile` substring,
  and the golden measurement fixture is deliberately left alone.
- ✅ Every flag was checked against the pinned cargo-llvm-cov 0.8.7. The
  floor is applied in Python, with no `--fail-under`, and a test pins that.
- ✅ Each phase can be merged and reverted on its own, which suits 0370's
  "two halves" contract.
- ✅ The changes target measured bottlenecks: the 61 s clean plus 30 s
  rebuild, and about 42 sys-CPU-s of 2-byte writes.

### Recommended Changes

1. **Make the floor robust across environments** (addresses: floor margin
   near zero; coupled to toolchain versions; skip-permission runs; one
   workspace-wide floor)
   In Phase 3 §1, take the minimum across both CI legs *and* a local
   engywook run, including a run with `ACCELERATOR_TEST_SKIP_PERMISSION_TESTS`
   set, then subtract a fixed tolerance before rounding down. Alternatively,
   skip the floor (still printing the totals) when the skip variable is set,
   and pin that with a test. State that a toolchain, llvm-tools or
   cargo-llvm-cov bump is a sanctioned re-baselining point.
2. **Deal with stale instrumented binaries** (addresses: stale binaries skew
   totals; artefact growth)
   At minimum, add a Phase 2 manual check: bump one dependency, rerun, and
   compare the totals with a clean run's, recording the report time. Make
   the floor-failure message name `cargo llvm-cov clean --workspace`. If
   drift shows up, consider a fingerprint-triggered full clean, or passing
   `report` only this run's binaries.
3. **Isolate the task tests from the production floor** (addresses: real
   `COVERAGE_FLOOR` couples sequence tests)
   Have the `TestTestUnitCli` fixture monkeypatch `COVERAGE_FLOOR` to zero
   by default. Only the floor tests override it.
4. **Fix the Phase 3 docstring and README instructions** (addresses: the
   "report-only" text is already removed; floor wording in the env-var row;
   docstring narration; no remedy in the message)
   Give the exact Phase 3 docstring paragraph. Trim Phase 2's docstring to
   the why. Put the floor policy beside the lane in `tasks/README.md`, not
   in the env-var row. Update `tasks/shared/rust.py:44`.
5. **Make the report's scope and features explicit** (addresses: test-support
   loophole; duplicated `--all-features`; flags not asserted)
   Build the nextest and report commands from shared manifest and feature
   fragments, add `--ignore-filename-regex` for the visualiser (or a guard
   test), and assert the manifest, feature and exclude flags in the sequence
   test. Correct the Key Discoveries claim that the lint fully enforces the
   invariant.
6. **Tighten the floor domain model** (addresses: reflection on field names;
   rounding mismatch in the message; per-metric floor tests; inlined floor
   judgement)
   Use explicit comparisons or a `Metric` enum. Judge the rounded total, or
   print more precision. Parametrise the shortfall tests over each metric.
   Extract `_enforce_floor`.
7. **Correct and tidy Phase 1** (addresses: the `MAX_BYTES * 2` table row;
   segment-listing helpers; magic `3`; `FIVE_MEBIBYTES` naming; mutation
   check mislabelled; CPU budget)
   Fix the table row. Use one `log_segments` and one `rotated_segments`
   helper and delete `count_segments`. Bind `rotations`. Rename the constant
   by its role. Move the mutation check to Manual Verification. Also time
   the prebuilt test binary.
8. **Settle the export shape with a real fixture, and plan for the old
   test** (addresses: hand-built fixture; `test_raises_when_inner_tests_fail`)
   Commit a trimmed real 0.8.7 export as the parser fixture, which settles
   the plan-vs-work-item shape discrepancy. Repoint the old test at the
   coverage-off failure path.
9. **Record the 0162 reversal** (addresses: decision recorded only in the
   plan)
   Note it on the 0363 work item, or capture it as an ADR.

## Per-Lens Results

### Architecture

**Summary**: Structurally sound and consistent with codebase patterns,
with a clean domain/I-O split and independently revertible phases. Main
weakness: the report step's scope is undeclared and rests on a
dependency-graph invariant weaker than claimed, and Phase 3 makes pass or
fail depend on that scope and on environment-dependent inputs.

**Strengths**: Pure domain module matching `npm_audit.py`; intention-named
steps; `_MANIFEST` blast radius checked; independent phases; floor in Python
pinned by a test.

**Findings**:
- 🟡 major / medium — *Report scope depends on a dependency-graph invariant
  that has a test-support loophole* (Key Discoveries / What We're NOT Doing).
  `broken_rules` returns `[]` for test-support → composition-root edges, so a
  future test-support crate could pull visualiser sources into
  `llvm-cov-target`. Make scope explicit via `--ignore-filename-regex` or a
  guard test.
- 🔵 minor / high — *Report and test steps restate manifest and feature
  selection independently* (Phase 2 §3). Compose both from shared fragments
  and pin with a test.
- 🔵 minor / medium — *Single workspace-wide floor couples the gate to
  environment-dependent and unrelated inputs* (Phase 3). The permission-skip
  variable, OS cfgs, new crates and the stale-binary residual all move the
  total; acknowledge them and decide policy.
- 🔵 suggestion / medium — *Metric set duplicated across two dataclasses and
  joined by reflection* (Phase 3 §2). Use a `Metric` concept or a
  `percent(metric)` accessor.

### Code Quality

**Summary**: Well designed for maintainability; residual concerns are the
reflective metric matching, the duplicated feature flag, and three segment
listing variants.

**Strengths**: Follows capture-then-parse; intention-revealing helpers with
guard clauses; error chaining; named Rust helpers and literal constants;
care over hidden coupling.

**Findings**:
- 🔵 minor / high — *Floor and totals matched through reflection on field
  names* (Phase 3 §2).
- 🔵 minor / high — *Report's `--all-features` duplicates the feature set in
  `_MANIFEST`* (Phase 2 §3). Extract a shared constant.
- 🔵 minor / medium — *Three slightly different ways to list log segments*
  (Phase 1 §2). One helper per concept; delete `count_segments`.
- 🔵 minor / medium — *Magic `3` and explanatory assertion message* (Phase 1
  §2). Bind `rotations` and assert `rotations + 1`.
- 🔵 minor / medium — *Single error message misdescribes non-JSON output as
  "no totals"* (Phase 2 §2). Handle `JSONDecodeError` separately.
- 🔵 suggestion / medium — *Floor judgement inlined into `_run_instrumented`*
  (Phase 3 §3). Extract `_enforce_floor`.
- 🔵 suggestion / medium — *Phase 3 docstring edit references wording Phase 2
  already removed* (Phase 3 §3).

### Test Coverage

**Summary**: Strong overall: literal-value Rust assertions checked against
named mutations, a pure parser module with unit tests, and a full-sequence
rewrite of `TestTestUnitCli` with per-step early-stop tests. The main gap is
Phase 3 coupling the sequence tests to the real floor.

**Strengths**: Real mutation gap closed; proper red step for a test-only
change; pure-module unit tests incl. rounding and padding; ordering pin
against bare `--no-clean`; per-step failure tests; no-JSON-leak assertion;
two-sided boundary test; pre-existing test pitfalls identified.

**Findings**:
- 🟡 major / high — *Real `COVERAGE_FLOOR` couples every sequence test to
  production floor values* (Phase 3 §3). Default the fixture to a zero floor.
- 🔵 minor / high — *Fate of `test_raises_when_inner_tests_fail` unspecified;
  coverage-off failure untested* (Phase 2 §3).
- 🔵 minor / medium — *Manifest and feature flags on clean and report not
  asserted* (Phase 2 §3).
- 🔵 minor / medium — *Parser fixture is hand-built, not taken from real
  output* (Phase 2 §2). The work item describes a different shape; commit a
  trimmed real export.
- 🔵 minor / medium — *Floor tests don't confirm per-metric pairing* (Phase 3
  §2). Parametrise over metrics with distinct floors.
- 🔵 suggestion / medium — *Mutation check is a one-off manual step listed
  as automated* (Phase 1 Success Criteria). Also tighten to
  `active_len == OVERSHOOT`.

### Correctness

**Summary**: Phase 1 is verified against the file-rotate 0.7.6 source for
every test and mutation, though two mutation-table rows predict the wrong
failure mode. The Python sequencing and parser are sound. The main risk is
that stale instrumented binaries under `--no-clean`, combined with a floor
within 0–1 pp of measured totals, produce spurious failures.

**Strengths**: Rotation semantics confirmed; all three rewritten tests pass
unmutated and catch every mutation; parser exception coverage is complete;
step ordering and short-circuiting are correct; `COVERAGE_FLOOR` is read at
call time; no race on `llvm-cov-target`; `_MANIFEST` kept separate.

**Findings**:
- 🟡 major / medium — *Stale instrumented binaries accumulate under
  `--no-clean` and skew the totals the floor judges* (Phase 2 / Phase 3).
  Suggest fingerprint-triggered clean, this-run-only objects, or at minimum a
  dependency-bump manual check.
- 🟡 major / medium — *Floor margin can be near zero and is derived only from
  CI environments* (Phase 3 §1). Include a local total and subtract a
  tolerance.
- 🔵 minor / high — *`MAX_BYTES * 2` row predicts wrong outcomes for tests 2
  and 3* (Phase 1 §2). Test 2 → `[10 MiB, 10 MiB]`, test 3 → 2 files.
- 🔵 minor / high — *Shortfall message can show a total equal to or above
  its floor* (Phase 3 §2). Judge the rounded value, or print more precision.

### Performance

**Summary**: Targets the measured bottlenecks with minimal changes, and the
algorithmic and I/O reasoning holds. Remaining concerns are about
measurement: the CPU budget conflates tool start-up, one cost is projected
from a much smaller sample, and growth of the report cost is unwatched.

**Strengths**: Targets the dominant costs; relies on a verified library
property; captures output in memory; short-circuits failures; profraw clean
prevents unbounded profile build-up.

**Findings**:
- 🔵 minor / medium — *CPU budget conflates test cost with cargo and nextest
  start-up, and uses a different runner from the lane* (Phase 1 Manual
  Verification).
- 🔵 minor / medium — *Nothing bounds or watches instrumented-artefact growth
  in `llvm-cov-target`* (Phase 2 / What We're NOT Doing).
- 🔵 suggestion / medium — *Profraw-only clean cost projected from 18 files
  but will delete about 9,200 per run* (Performance Considerations).

### Standards

**Summary**: Closely follows repo conventions (module layout, error class,
frozen dataclasses, `hide="out"`, `Exit(code=1)`, substring guard,
`_MANIFEST` blast radius). Small issues in Rust constant naming, docstring
narration, and where the README floor wording lands.

**Strengths**: Mirrors the `npm_audit` layout; `_MANIFEST` not widened;
`--profile` guard respected; all description sites updated together; capture
pattern matches `docs.py`; Rust tests keep existing names and patterns.

**Findings**:
- 🔵 minor / medium — *Constant named for its value, with units inconsistent
  with the test name* (Phase 1 §2).
- 🔵 minor / medium — *Docstring narrates steps the code already shows*
  (Phase 2 §3).
- 🔵 minor / low — *Floor wording goes into the env-var row the variable
  doesn't govern* (Phase 3 §3).
- 🔵 suggestion / low — *Rust sample exceeds 80 columns once nested in
  `mod tests`* (Phase 1 §2).

### Documentation

**Summary**: Every surface describing the lane is identified and the
stale-binary remedy is pinned by a test. But the Phase 3 docstring
instruction references removed wording, the floor policy has no
discoverable home, and the 0162 reversal isn't recorded.

**Strengths**: All description sites enumerated and the golden fixture
reasoned about; the remedy is pinned by a test; the docstring explains why;
no new comments in `log.rs`.

**Findings**:
- 🟡 major / high — *Phase 3 tells the implementer to replace "report-only"
  text that Phase 2 already removed* (Phase 3 §3).
- 🔵 minor / medium — *Floor behaviour and raising procedure squeezed into
  the env-var row* (Phase 3: Descriptions).
- 🔵 minor / medium — *Shortfall failure message gives no remediation
  pointer* (Phase 3 §3).
- 🔵 minor / medium — *Reversal of the 0162 report-only decision recorded
  only in this plan* (Overview / Phase 3).
- 🔵 suggestion / medium — *Docstring opening restates the step sequence*
  (Phase 2 §3).
- 🔵 suggestion / low — *`coverage_enabled` docstring goes out of date*
  (`tasks/shared/rust.py:44`).

### Compatibility

**Summary**: Command shapes match the pinned cargo-llvm-cov 0.8.7, and the
same binary runs on both CI legs. The rotation tests rely only on
lockfile-pinned file-rotate behaviour. The main gap is the Phase 3 floor's
thin, single-sample headroom against toolchain bumps, root/container runs,
and local stale binaries.

**Strengths**: Flags verified against 0.8.7; floor from the lower CI leg;
the parser tolerates schema additions; permission test under `cfg(unix)`,
unaffected by umask; no `--fail-under` dependency.

**Findings**:
- 🟡 major / medium — *Floor coupled to pinned Rust/LLVM and cargo-llvm-cov
  versions, with no re-baselining rule* (Phase 3).
- 🟡 major / medium — *Floor will fail in root/container runs that set
  `ACCELERATOR_TEST_SKIP_PERMISSION_TESTS`* (Phase 3 §3).
- 🔵 minor / medium — *Local totals can drift from CI's because
  `--no-clean` keeps stale binaries* (Phase 2 / Phase 3). Point the failure
  message at the remedy.
- 🔵 suggestion / low — *Flag verification was on the local macOS binary
  only; validate the Linux leg too* (Phase 2 Manual Verification).

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-11-14

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Correctness / Compatibility / Performance**: Stale instrumented
  binaries accumulate under `--no-clean` and skew the totals the floor judges
  — Resolved. The dependency-bump manual check comes with a stop-and-revisit
  rule, and the remedy is named in the docstring and the floor message.
  Performance notes that disk growth across many bumps is still unwatched.
- 🟡 **Correctness / Architecture**: Floor margin can be near zero and comes
  only from CI environments — Resolved. The floor is now the minimum of
  four runs, less 1 pp, rounded down.
- 🟡 **Compatibility**: Floor coupled to toolchain versions with no
  re-baselining rule — Resolved. Toolchain, llvm-tools and cargo-llvm-cov
  bumps are a sanctioned reset point.
- 🟡 **Compatibility**: Floor fails in root runs that skip the permission
  tests — Resolved. A root skip-permission run is one of the four samples.
- 🟡 **Test Coverage**: The real `COVERAGE_FLOOR` couples the sequence tests
  — Resolved. The fixture defaults to a zero floor.
- 🟡 **Architecture**: Report scope rests on a lint with a test-support
  loophole — Resolved. The `TestVisualiserHasNoDependents` guard closes it.
- 🟡 **Documentation**: Phase 3 replaces text Phase 2 already removed —
  Resolved. Phase 3 replaces an exact sentence that Phase 2 writes.
- 🔵 All minor findings and suggestions from pass 1 — Resolved, apart from
  artefact growth in `llvm-cov-target` (partially resolved).

### New Issues Introduced

- 🔵 **Documentation / Standards / Code Quality**: The Phase 2 `cli.py`
  sample silently drops the existing comment explaining why the visualiser
  is excluded. The plan should say whether it is kept, and whether it should
  now also mention the report's scope.
- 🔵 **Code Quality / Architecture**: The guard hand-parses `Cargo.toml`,
  while `tasks/shared/cargo_metadata.workspace_packages()` already gives
  cargo's resolved dependency list, including `workspace = true`
  inheritance. Key Discoveries should also say that `edges()` drops
  dev-dependencies.
- 🔵 **Test Coverage / Code Quality / Architecture**: Nothing checks that
  the real `COVERAGE_FLOOR` covers every `Metric` with a value from 0 to
  100. A `__post_init__` completeness check, or one cheap test, would fix
  this.
- 🔵 **Test Coverage**: If the guard keeps its TOML walker, its
  parametrisation lacks plain `[dependencies]` and `[build-dependencies]`
  cases and a negative case.
- 🔵 **Correctness**: Appending `-p kernel` to a nextest command that
  already carries `--workspace --exclude` won't restrict the run. Spell out
  the restricted commands, and confirm that `report` accepts `-p`.
- 🔵 **Compatibility**: The root skip-permission run doesn't pin its
  toolchain or platform. It should use the mise-provisioned tools and record
  its OS, architecture and `cargo llvm-cov --version`.
- 🔵 **Performance**: Also record `du -sh cli/target/llvm-cov-target` in
  the dependency-bump check.
- 🔵 **Documentation**: The References section cites
  `crate_dependencies.py:350-397`. It should cite `369-401`.

### Assessment

All seven major findings and nearly all minor ones are resolved, and the
edits introduced only minor issues and suggestions. The plan is acceptable
to implement as it stands. The `-p kernel` command fix and the floor
completeness check are the most worthwhile follow-ups. Switching the guard
to `workspace_packages()` would remove the TOML walker and its extra test
cases.

### Follow-up Edits After Pass 2

All eight new minor issues and suggestions above were addressed in the
plan. The guard now uses `workspace_packages()` with synthetic-package
branch tests. `CoverageTotals` and `CoverageFloor` check that every metric
is present, and the floor that its values are in range. The restricted
`kernel` commands are spelled out. The root run's environment is pinned.
`du -sh` is recorded. The visualiser-exclusion comment is kept. References
are corrected.

### Final Verdict

**Verdict:** APPROVE — the reviewer approved the plan after the pass-2
follow-up edits (2026-11-14T09:36:51+00:00).
