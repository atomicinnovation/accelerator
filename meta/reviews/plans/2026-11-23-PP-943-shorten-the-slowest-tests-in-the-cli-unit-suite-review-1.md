---
type: "plan-review"
id: "2026-11-23-PP-943-shorten-the-slowest-tests-in-the-cli-unit-suite-review-1"
title: "Plan Review: Shorten the Slowest Tests in the CLI Unit Suite Implementation Plan"
date: "2026-11-23T15:18:19+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-11-23-PP-943-shorten-the-slowest-tests-in-the-cli-unit-suite"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "performance", "standards"]
review_number: 1
review_pass: 3
tags: ["performance", "testing", "rust", "migrate", "vcs", "nextest"]
last_updated: "2026-11-23T16:46:38+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Shorten the Slowest Tests in the CLI Unit Suite Implementation Plan

**Verdict:** REVISE

The plan attacks the measured cause, serial `git`/`jj`/migrate spawns that
rebuild identical states. It does so by extending PP-957's
`SharedFamily`/`CopyableFamily`/`Workbench` machinery rather than inventing a
parallel path, and its phases are separable, test-first and backed by golden
measurement data. The weaknesses cluster in phases 5 and 6. The
stalled-migration family does not fit the `SharedFamily` contract, it drags
migrate's domain into `vcs-test-support`, and it turns a migrate regression
into a suite-wide build failure. Its reused-state test both checks the new
builder against itself and risks breaching the 20 s ceiling it serves. Several
stated counts are also wrong or tautological (26 vs 30 copies, the halt-count
assertions, the five-file corpus baseline).

### Cross-Cutting Themes

- **The migrate dependency doesn't fit the family abstractions** (flagged by:
  architecture, code-quality, standards) — `SharedFamily::build_privately` has
  no channel for the migrate path, so the path leaks into
  `StalledMigrationPart`, `SharedBuildRequest` and `build_for_sharing` through
  three different routes. `StallKey` is then passed twice.
- **Two colocation vocabularies in one crate** (flagged by: architecture,
  code-quality, standards) — `repository_state::Colocation { Git, JjAlone,
  JjColocated }` and `stalled_migration::JjColocation { Alone, Colocated }`,
  while the crate already says `pure`/`colocated`.
- **`copy_private_tree` sidesteps the copy guard** (flagged by: architecture,
  code-quality, standards, test-coverage) — it is a public unlogged wrapper
  over `copy_part`, protected only by a docstring, and the plan expects
  `test_fixture_builds.py` to pass unchanged.
- **Widening `Workbench::with` to a consumer** (flagged by: architecture,
  code-quality) — phase 6 adds `guarded_resume.rs` to the allow-list. A
  narrow, logged "build from scratch" constructor would keep the guard intact.
- **The phase 6 reused-state test** (flagged by: performance, test-coverage,
  code-quality) — it packs 4 private builds and 8 projections into one test,
  its oracle is the new `Stall::build` itself, and it re-masks files that
  `RepositoryState` already covers.
- **Copy count arithmetic** (flagged by: correctness, test-coverage) — phase
  6 expects 26 copies, but the reused-state test adds 4, so the run logs 30.
  "23 scenario runs" matches neither number.
- **`TestOutcome`/`TestMedian` naming** (flagged by: standards, code-quality,
  test-coverage) — the `Test*` prefix triggers pytest collection warnings.

### Tradeoff Analysis

- **Cohesion vs one builder**: architecture prefers the stall recipe to stay
  in `migrate-cli` (a second builder bin), so `vcs-test-support` remains
  VCS-only. The single-builder route keeps the build step simple and matches
  the plan's PP-957 lineage. Recommendation: keep the single builder, but
  record the tradeoff and isolate the migrate recipe in a
  `stalled_migration::corpus` submodule that shares one migrate-environment
  definition with `common::run`.
- **Fail-fast vs blast radius**: today a stall regression fails only
  `guarded_resume`. Under phase 5 it aborts every `test:unit:cli` run.
  Withholding the handover on failure keeps the signal local, but it adds a
  degraded mode to the build step. Recommendation: at minimum make the error
  name migrate's stderr and `guarded_resume`. Non-fatal handling is the
  author's call.
- **Oracle strength vs the 20 s ceiling**: test coverage wants the oracle to
  be the existing `Stalled::on` sequence plus absolute per-key facts, and
  performance wants fewer builds per test. Recommendation: split into one
  reused-state test per `StallKey`. That parallelises under nextest and leaves
  room for a stronger oracle.

### Findings

#### Major

- 🟡 **Architecture + Code Quality + Standards**: Stalled-migration family
  cannot honour the `SharedFamily` contract; the migrate path leaks into Part
  and request.
  **Location**: Phase 5: The family in Rust
  `build_privately(directory, hermetic, token)` cannot build a stall set
  without migrate's path, and the plan never says what it does.
  `StalledMigrationPart { key, migrate }` mixes part identity with a build
  input that `entries` and `rebased` ignore.
- 🟡 **Code Quality + Standards + Architecture**: Two overlapping colocation
  enums for one domain concept.
  **Location**: Phase 2: The projection / Phase 5: The family in Rust
  `Colocation::Git` is a VCS kind, not a colocation. Unify on one jj
  colocation type and model `RepositoryKind::Git | Jj(JjColocation)`.
- 🟡 **Architecture + Code Quality**: Migrate-specific domain knowledge moves
  into the VCS test-support crate.
  **Location**: Phase 5: corpus seeding
  The ledger, work-item corpus, migrate's env vars and the `MIGRATION STALLED`
  contract would live in `vcs-test-support`. That cohesion cost is
  unrecorded, and `Stall::build` duplicates `common::run`'s environment
  scrubbing.
- 🟡 **Architecture**: A migrate regression becomes a build-step failure for
  the whole CLI suite.
  **Location**: Phase 5 / Phase 6
  The stall assertions move from a test into `shared-fixtures`, whose failure
  aborts nextest before any crate runs.
- 🟡 **Code Quality + Architecture**: Allowing `Workbench::with` in a
  consumer test erodes the fixture-log guard.
  **Location**: Phase 6: Consumer registration
  Add a narrow logged constructor (e.g.
  `Workbench::built_from_scratch_for_this_test()`) instead.
- 🟡 **Test Coverage**: Stall reused-state test checks the new builder
  against itself.
  **Location**: Phase 6: Reused-state test
  Both sides use `Stall::build`. A dropped `--colocate` flag would pass
  unseen. Keep today's `Stalled::on` sequence as the oracle, and assert
  colocation and parent count per key.
- 🟡 **Test Coverage**: Halt-count assertions are tautological and do not pin
  the off-by-one.
  **Location**: Phase 3: halt count and iteration counts
  Looping `0..count` and asserting `count` iterations is true by
  construction. Add a boundary check: `HaltAt(count - 1)` halts and
  `HaltAt(count)` completes.
- 🟡 **Test Coverage**: Stall-building code lands in phase 5 with no test of
  its own.
  **Location**: Phase 5: Tests
  `Stall::build` and its not-stalled error path are first exercised in phase
  6. Drive it with stub migrate scripts (one exiting 1 with
  `MIGRATION STALLED`, one exiting 0).
- 🟡 **Correctness + Test Coverage**: The expected copy count of 26 ignores
  the reused-state test's 4 copies.
  **Location**: Phase 6: Success Criteria
  The log will show 30 copies. "All 23 scenario runs" is also inconsistent.
- 🟡 **Correctness**: The five-file reused-state comparison names a
  from-scratch build that cannot produce that state.
  **Location**: Phase 3, §4: Reused-state test
  Today the five-file corpus is two commits (`committed` then a second
  `commit_everything`, `rekey_run.rs:954-956`). `committed(vcs)` only builds
  four files. State the from-scratch side and whether the history stays at
  two commits.
- 🟡 **Performance**: The reused-state test packs 4 private builds into one
  test, risking the 20 s ceiling.
  **Location**: Phase 6: Reused-state test
  Today's 2-build tests already take 13.5–18 s under the suite. Split the
  test per `StallKey`.
- 🟡 **Performance**: Serial stall builds may become the build step's
  critical path.
  **Location**: Phase 5: `build_for_sharing`
  About 28 jj spawns plus 4 migrate runs run on one thread, and that wall
  time is added to `guarded_resume`'s 98.8 s figure. Build the four stalls on
  scoped threads.

#### Minor

- 🔵 **Architecture + Standards + Code Quality + Test Coverage**:
  `copy_private_tree` is an unguarded public route around the `copy_part`
  guard.
  **Location**: Phase 3: Copying a test's own tree
  Add it to `GUARDED_CALLS`, allowing only `work-adapters/tests/support`.
  Share the `named_by` formatting with `Workbench::copied`.
- 🔵 **Standards**: Invoke parameter names and defaults break the
  measure-task convention.
  **Location**: Phase 1: The task
  `logs` becomes `--logs`, not `--log`. Use `list[str] | None`,
  `iterable=[...]`, per-option `help` and a mise `description`.
- 🔵 **Standards**: `writable_stalled_migration` breaks the accessor naming
  pattern.
  **Location**: Phase 5: `workbench.rs`
  Match `StalledMigrations` (`writable_stalled_migrations`), or return the
  `Stall` directly.
- 🔵 **Standards**: The per-family build line uses the raw name instead of
  `heading`/`noun`.
  **Location**: Phase 5: `session.py`
- 🔵 **Code Quality + Standards + Test Coverage**: `TestOutcome`/`TestMedian`
  collide with pytest collection.
  **Location**: Phase 1: Log parsing and figures
- 🔵 **Code Quality**: The new duration parser does not reuse the `timeline`
  primitives (`read_stamped`, `attributed_lines`, `_NEXTEST_SUMMARY`,
  `median`).
  **Location**: Phase 1: Log parsing and figures
- 🔵 **Code Quality**: The reused-state test masks IDs a second time for
  files `RepositoryState` already covers.
  **Location**: Phase 6: Reused-state test
- 🔵 **Code Quality**: `committed(vcs)` and `Committed::build` become two
  ways to build the same repository.
  **Location**: Phase 3: The template
- 🔵 **Code Quality**: `RepositoryState::of` has no stated decomposition
  (per-VCS readers, ordering, ID masking).
  **Location**: Phase 2: The projection
- 🔵 **Code Quality**: The `matrix_private_build` helper duplicates the
  Workbench's adopt-or-build decision and opens an unused workbench.
  **Location**: Phase 4: The test
- 🔵 **Correctness**: IDs can leak through ref names (`refs/jj/keep/<hash>`),
  abbreviated IDs and content timestamps.
  **Location**: Phase 2: The projection
- 🔵 **Correctness**: The topological tie-break is not total, so positions
  can be nondeterministic.
  **Location**: Phase 2: Commits ordering
- 🔵 **Correctness**: There is no defined rule for adding the `rekey_run`
  reused-state test's time to the two per-test thresholds.
  **Location**: Phase 3: Test mapping; Phase 1
- 🔵 **Correctness**: The new `migrate` field breaks the
  `SharedBuildRequest` literal in `matrix_private_build.rs:32`, which the
  plan doesn't list.
  **Location**: Phase 5: `shared_build.rs`; Phase 4
- 🔵 **Correctness**: The parser spec leaves out nextest's padded fields,
  stamped family lines, and the `LEAK` and `TRY n PASS` statuses.
  **Location**: Phase 1: Log parsing
- 🔵 **Test Coverage**: The projection's differential cases miss commit
  parents, git refs or `HEAD`, `+`-joined merge run bases and tie-break
  determinism.
  **Location**: Phase 2: Tests
- 🔵 **Test Coverage**: The fallback branch runs only in a one-off direct
  `cargo nextest run`, never in CI.
  **Location**: Phase 4: The test
- 🔵 **Test Coverage**: The duration parser lacks edge cases (missing runs,
  FAIL/SLOW/LEAK, family medians) and a task-level output test.
  **Location**: Phase 1: Tests
- 🔵 **Performance**: The spawn-removal estimate treats migrate runs as equal
  to jj spawns and omits the added work.
  **Location**: Performance Considerations
- 🔵 **Performance**: Projection file-tree reads could multiply spawns unless
  batched (`jj log -T`, `git cat-file --batch`).
  **Location**: Phase 2: The projection
- 🔵 **Performance**: The `rekey_run` spawn/in-process split is unprofiled,
  so profile before switching.
  **Location**: Phase 3

#### Suggestions

- 🔵 **Performance**: "117 passes" is inconsistent with 26 + 13 (40 after
  the fix).
  **Location**: Performance Considerations: rekey_run
- 🔵 **Standards**: `_STALLED_MIGRATION` is used in phase 6 but never
  defined.
  **Location**: Phase 6: Consumer registration
- 🔵 **Architecture**: `_locate_builder` hard-codes a family-specific
  executable. Let each `Family` declare its tool needs instead.
  **Location**: Phase 5: The build step in Python
- 🔵 **Code Quality**: Use one counted-iteration helper rather than ad-hoc
  counters.
  **Location**: Phase 6: `for_each_colocation`
- 🔵 **Correctness**: Automate phase 5's absolute-path check with
  `files_naming` rather than a manual `rg`.
  **Location**: Phase 5: Manual Verification

### Strengths

- ✅ Targets the measured cause (serial spawns under contention) instead of
  tuning nextest scheduling.
- ✅ Extends PP-957's `SharedFamily`/`CopyableFamily`/`Workbench` and mirrors
  `status_log.rs`, so fixture-log events, the named-by check and the consumer
  rules apply for free.
- ✅ Respects the crate boundary: the migrate binary path is injected from
  `UnitListing`, and there is no dependency cycle.
- ✅ Has an explicit phase graph. Capability phases (1, 2, 5) are separate
  from landing phases (3, 4, 6), and rollback notes cover a phase 5 family
  with no consumer.
- ✅ `RepositoryState` makes "reused equals rebuilt" a first-class, testable
  invariant, with per-field differential tests and a no-snapshot test.
- ✅ Correctly diagnoses the 13 vs 14 halt-point gap, with a red-first test.
- ✅ Phase 1's golden test anchors the measurement tool to the baseline
  record's medians.
- ✅ Assertions are kept word for word, with test-mapping tables and a
  `nextest list` audit.
- ✅ Hoists the version checks out of per-copy paths, and adds no cache that
  outlives a run.
- ✅ Uses rich domain naming (`StallKey`, `ParentShape`, `Stalled::copied`,
  `Committed::fresh_copy`).

### Recommended Changes

1. **Model the migrate dependency once** (addresses: `SharedFamily`
   contract, Part type, accessor naming, `SharedBuildRequest` literals)
   - Keep `Part = StallKey`, and introduce `MigrateBinary(PathBuf)` as the
     family's build input.
   - State how `build_privately` behaves for this family.
   - Have `writable_stalled_migrations(key)` return the `Stall`.
   - List the `matrix_private_build.rs:32` literal, or add a defaulting
     constructor.
2. **Unify colocation** (addresses: two colocation enums) — one
   `JjColocation { Pure, Colocated }` with `flag()`, plus
   `RepositoryKind::Git | Jj(JjColocation)` in `RepositoryState`.
3. **Rework the phase 6 reused-state test** (addresses: self-referential
   oracle, 20 s ceiling, redundant masking, `Workbench::with` widening, copy
   count)
   - Make it one test per `StallKey`, with the oracle taken from today's
     `Stalled::on` steps.
   - Assert colocation and parent count per key.
   - Rely on `RepositoryState` for the state files.
   - Use a new logged `Workbench` constructor instead of the allow-list.
   - Expect 30 copies, and fix the "23" count.
4. **Test `Stall::build` in phase 5** (addresses: untested stall code,
   absolute paths) — use stub migrate scripts for the stall and not-stalled
   paths, and add an automated `files_naming` check over a built stall.
5. **Record the cohesion and blast-radius tradeoffs** (addresses:
   migrate-in-`vcs-test-support`, build-step failure)
   - Isolate the corpus in `stalled_migration::corpus`.
   - Share the migrate env definition with `common::run`.
   - Make a family build failure name migrate's stderr and `guarded_resume`.
6. **Parallelise the stall builds** (addresses: serial critical path) — use
   `thread::scope` inside `build_for_sharing`.
7. **Pin the halt count by its boundary** (addresses: tautological
   assertions) — `HaltAt(count - 1)` halts and `HaltAt(count)` completes.
   Replace `checked > 0` with the exact set of halts.
8. **Clarify the five-file corpus** (addresses: the from-scratch mismatch,
   reused-time attribution)
   - Keep two commits, or justify one.
   - Make the from-scratch side `Committed::build(vcs, seed)`.
   - Define how the reused-state test's time is attributed to the tail-test
     thresholds.
9. **Guard `copy_private_tree`** (addresses: copy guard bypass) — add it to
   `GUARDED_CALLS` with an allow-list, red first.
10. **Tighten phase 1** (addresses: invoke naming, `Test*` names, timeline
    reuse, parser edge cases, family line wording)
    - Use the `log`/`binary` list params, and `PassedTest`/`MedianTiming`
      names.
    - Build on `read_stamped` and `_NEXTEST_SUMMARY`.
    - Add cases for padding, `LEAK`, `TRY n PASS`, missing runs and stamped
      family lines.
    - Print `family.heading`.
11. **Harden `RepositoryState`** (addresses: ID leaks, tie-break, missing
    differential cases, spawn cost, decomposition)
    - Exclude `refs/jj/*`.
    - Use the parents' positions as the final tie-break.
    - Add parent, ref and `+`-joined cases.
    - Batch reads.
    - Name the internal seams.
12. **Small fixes**
    - Correct "117 passes".
    - Define `_STALLED_MIGRATION`.
    - Add a projected spawn budget, and profile `rekey_run` before
      switching.
    - Make `committed(vcs)` delegate to `Committed::build`.

## Per-Lens Results

### Architecture

**Summary**: The plan reuses PP-957's shared-family, workbench and
fixture-log machinery, keeps each binary's landing commit independent, and
states most of its tradeoffs. The main structural risk is the
stalled-migration family. The single-builder route moves migrate-specific
domain knowledge into `vcs-test-support`, and it makes the whole
`test:unit:cli` run depend on migrate behaving correctly during the build
step. The plan records neither consequence. Smaller issues: an unlogged copy
helper, duplicated colocation modelling, and a build dependency threaded
awkwardly through `Part` and `SharedFamily`.

**Strengths**:
- Extends PP-957's abstractions and mirrors `status_log.rs`, so logging,
  named-by and consumer rules apply for free.
- Respects the crate boundary by injecting the migrate binary path.
- Has an explicit phase dependency graph and per-commit rollback.
- `RepositoryState` is a reusable projection in the shared crate.
- `rekey_run` template reuse stays local to its test binary.
- Builds once per test with no cross-run cache, which honours PP-957's
  constraints.

**Findings**:
- 🟡 major / high — *Migrate-specific domain knowledge moves into the VCS
  test-support crate* (Phase 5). The corpus, ledger, `work_item` and the
  migrate CLI contract would live in `vcs-test-support`. The second-builder
  route is unrecorded, and `Stall::build` duplicates `common::run`'s
  environment scrubbing. Record the tradeoff, isolate the recipe, and share
  one migrate-environment definition.
- 🟡 major / high — *A migrate regression becomes a build-step failure for
  the whole CLI suite* (Phases 5–6). Acknowledge the blast radius. Consider
  withholding the handover on failure, so consumers fall back and fail
  locally, or at least name migrate's stderr and `guarded_resume` in the
  error.
- 🟡 major / medium — *The family's build dependency doesn't fit the
  `SharedFamily`/`CopyableFamily` abstractions* (Phase 5). There are two
  injection channels, `build_privately` cannot be implemented, and the key is
  passed twice. Use a `StalledMigrationRecipe { migrate }`, keep `Part` as
  `StallKey`, and return the `Stall` directly.
- 🔵 minor / high — *Public unlogged copy helper is a side door around the
  workbench boundary* (Phase 3). Extend the guard, or refuse sources under
  the shared root.
- 🔵 minor / high — *Colocation is modelled twice in the same crate*
  (Phases 2/5). Define one type.
- 🔵 minor / medium — *Widening the `Workbench::with` guard to a consumer
  test erodes a deliberate boundary* (Phase 6). Add a logged forced-private
  API.
- 🔵 suggestion / medium — *The single-builder locator now hard-codes a
  second, family-specific executable* (Phase 5 Python). Let each `Family`
  declare the executables it needs.

### Code Quality

**Summary**: The plan is well structured. It follows the `CopyableFamily`
and `Workbench` patterns, puts tests first, and keeps phases small. The main
risks are making the new family fit abstractions that assume no external
input, weakening fixture-log guards for consumer tests, and adding types and
parsers that overlap existing ones.

**Strengths**:
- Phase 5 mirrors the `States`/`StatusLogPart`/`StatusLogManifest` shape.
- Uses rich domain naming throughout.
- Phase 1 is a pure module with I/O in the task.
- Test-mapping tables keep assertions word for word.
- `copy_private_tree` reuses `copy_entry`, so time fidelity is defined once.

**Findings**:
- 🟡 major / high — *Stalled-migration family cannot honour the
  `SharedFamily` contract; the migrate path leaks into Part and request*
  (Phase 5). Introduce `MigrateBinary(PathBuf)`, split or document
  `build_privately`, and take the key once.
- 🟡 major / high — *Allowing `Workbench::with` in a consumer test erodes
  the fixture-log guard* (Phase 6). Add
  `Workbench::built_from_scratch_for_this_test()`.
- 🟡 major / high — *Two overlapping colocation enums for the same domain
  concept* (Phases 2/5). Use `JjColocation` plus
  `RepositoryKind::Git | Jj(_)`.
- 🔵 minor / high — *`copy_private_tree` is an unguarded public route around
  the `copy_part` guard* (Phase 3). Guard it, and extract the `named_by`
  formatter.
- 🔵 minor / high — *New duration parser does not reuse the existing
  timeline parsing primitives* (Phase 1).
- 🔵 minor / medium — *`TestOutcome`/`TestMedian` names collide with pytest
  collection* (Phase 1).
- 🔵 minor / high — *Reused-state test masks IDs a second time for files
  `RepositoryState` already covers* (Phase 6).
- 🔵 minor / high — *`committed(vcs)` and `Committed::build` become two ways
  to build the same repository* (Phase 3).
- 🔵 minor / medium — *`RepositoryState::of` has no stated decomposition
  despite substantial logic* (Phase 2).
- 🔵 minor / medium — *`matrix_private_build` test duplicates the
  Workbench's adopt-or-build decision* (Phase 4).
- 🔵 minor / medium — *Migrate-specific corpus moves into the generic VCS
  fixture crate* (Phase 5). Use a scoped `stalled_migration::corpus`
  submodule.
- 🔵 suggestion / medium — *Use one counted-iteration helper rather than
  ad-hoc counters* (Phase 6).

### Test Coverage

**Summary**: The plan takes test coverage seriously, with reused-state tests,
a golden measurement test, word-for-word assertions and red-first contract
tests. The weak spots are the oracles and the counts. The `guarded_resume`
reused-state test checks the new builder against itself, the halt-count
assertions are largely tautological, and phase 5 lands the stall code with
no test of its own.

**Strengths**:
- Each reused state gets a copy-vs-scratch test.
- The golden test reproduces the baseline medians.
- The projection has per-field differential cases and a no-snapshot test.
- Test-mapping tables and a `nextest list` audit protect scenario coverage.
- Red-first contract tests keep the Rust and Python mirrors in step.
- The halt-count fix is ordered to start red.

**Findings**:
- 🟡 major / high — *Stall reused-state test checks the new builder against
  itself, not against today's `Stalled::on`* (Phase 6). Use the old sequence
  as the oracle, and assert colocation and parent count per key.
- 🟡 major / high — *Halt-count assertions are tautological and do not pin
  the off-by-one they fix* (Phase 3). Add the `count - 1` / `count` boundary
  check, and assert the exact checked halts.
- 🟡 major / medium — *Stall-building production code lands in phase 5 with
  no test of its own* (Phase 5). Drive it with stub migrate scripts.
- 🔵 minor / high — *Expected copy count of 26 omits the reused-state test's
  four copies* (Phase 6).
- 🔵 minor / medium — *`copy_private_tree` bypasses the copy guard*
  (Phase 3).
- 🔵 minor / medium — *The projection's differential cases miss commit-graph
  parents, git refs and masking edge cases* (Phase 2).
- 🔵 minor / medium — *The fallback branch is checked only by a one-off
  direct `cargo nextest run`* (Phase 4).
- 🔵 minor / medium — *The duration parser lacks edge-case and task-output
  tests* (Phase 1).
- 🔵 suggestion / medium — *`Test`-prefixed production class names will
  draw pytest collection warnings* (Phase 1).

### Correctness

**Summary**: The build-once-then-copy design is sound, since copies keep IDs
and nanosecond mtimes, and the halt-count fix is correct. The risk is in the
details. The phase 6 copy count is wrong, the phase 3 five-file comparison
names a baseline that cannot produce that state, the projection can leak IDs
or nondeterminism, and the measurement task has no rule for attributing
`rekey_run`'s new test cost.

**Strengths**:
- Builds on `copy_part`'s fidelity guarantees, so copies read as unchanged.
- Requires `--ignore-working-copy` for reads of unsnapshotted stalls.
- Correctly diagnoses the 13 vs 14 halt gap.
- `Stalled::copied` keeps borrow and lifetime order correct.
- Times each family inside Rust, because the threads are parallel.

**Findings**:
- 🟡 major / high — *Expected copy count of 26 ignores the reused-state
  test's four copies* (Phase 6). It should be 30. Also fix "23 scenario
  runs".
- 🟡 major / high — *Five-file reused-state comparison names a from-scratch
  build that cannot produce that state* (Phase 3 §4). Use
  `Committed::build(vcs, seed)`, and justify or keep the two-commit history.
- 🔵 minor / medium — *IDs can leak through reference names, abbreviated IDs
  and timestamps in file content* (Phase 2).
- 🔵 minor / medium — *Topological tie-break is not total, so positions can
  be nondeterministic* (Phase 2).
- 🔵 minor / high — *No defined way to add the rekey reused-state test's
  time to the tail-test figures* (Phases 1/3).
- 🔵 minor / high — *Adding a `migrate` field breaks existing
  `SharedBuildRequest` struct literals the plan does not list* (Phases 4/5).
- 🔵 minor / medium — *Parser spec leaves out nextest's padded fields and
  stamped family lines* (Phase 1).
- 🔵 suggestion / low — *Only a manual step checks that the migrate run
  leaves no absolute paths in the stall* (Phase 5).

### Performance

**Summary**: The plan goes after the real bottleneck, serial spawns that cost
2–5× more under suite load. It builds once and copies, hoists the version
checks, and drops a redundant matrix build. It does not account for the work
it adds (the reused-state test's 4 private builds plus projections), or for
where the new family's build sits (4 stalls on one thread, possibly the
build step's critical path). Both put `guarded_resume`'s 20 s ceiling and
98.8 s budget at risk.

**Strengths**:
- Targets spawns rather than scheduling.
- Hoists the version checks.
- `matrix_private_build` stops building a second matrix.
- Nanosecond mtimes avoid snapshot and rehash work.
- Acknowledges the unprofiled `rekey_run` in-process share.
- Adds no cross-run cache.

**Findings**:
- 🟡 major / medium — *The reused-state test packs four private stall builds
  into one test, risking the 20 s per-test ceiling* (Phase 6). Use one test
  per `StallKey`.
- 🟡 major / medium — *Serial stall builds may become the build step's
  critical path and inflate the counted family time* (Phase 5). Use
  `thread::scope`.
- 🔵 minor / medium — *The spawn-removal estimate treats migrate runs as
  equal to jj spawns and omits the added work* (Performance Considerations).
- 🔵 minor / medium — *The projection's per-commit file-tree reads could
  multiply jj spawns unless batched* (Phase 2).
- 🔵 minor / low — *Copying only pays off if spawns, not in-process work,
  dominate each pass, and that split is unprofiled* (Phase 3).
- 🔵 suggestion / high — *The `rekey_run` pass count in Performance
  Considerations is inconsistent* (117 vs ~40).

### Standards

**Summary**: The plan mostly follows established conventions: the family
traits tables, manifest and part pattern, contract files, consumer registry,
and a pure `build_measurement` module. The gaps are naming drift (two
colocation enums, accessor naming, invoke option names, the per-family line
wording) and an unlogged copy helper outside the guard.

**Strengths**:
- `Family::StalledMigration` follows the traits table exactly.
- The manifest, Part and sharing-test filenames mirror the status-log ones.
- `measure:test-durations` is registered beside its siblings.
- The pure module and its test paths respect the layering.
- The guard allow-lists and consumer sets are updated red-first.
- The new output line is checked against `_NEXTEST_SUMMARY` and
  `_FINISHED_IN`.

**Findings**:
- 🟡 major / high — *Two overlapping colocation vocabularies in the same
  crate* (Phases 2/5). Use the crate's `Pure`/`Colocated` terms.
- 🔵 minor / high — *Invoke parameter names and defaults break the
  measure-task convention and the documented `--log` flag* (Phase 1).
- 🔵 minor / high — *New unlogged copy helper is not covered by the copy
  guard* (Phase 3).
- 🔵 minor / medium — *`writable_stalled_migration` breaks the workbench
  accessor naming pattern* (Phase 5).
- 🔵 minor / medium — *Part type carries a build input rather than only
  selecting a part* (Phase 5).
- 🔵 minor / medium — *Per-family build line uses the raw family name
  instead of the `heading`/`noun` wording* (Phase 5).
- 🔵 suggestion / medium — *`TestOutcome` dataclass name collides with
  pytest's `Test*` class collection* (Phase 1).
- 🔵 suggestion / low — *`_STALLED_MIGRATION` constant is used before any
  phase defines it* (Phase 6).

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-11-23T16:03:58+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Architecture + Code Quality + Standards**: Stalled-migration family
  cannot honour the `SharedFamily` contract — Resolved (`type Inputs`,
  `MigrateBinary`, `Part = StallKey`)
- 🟡 **Code Quality + Standards + Architecture**: Two overlapping colocation
  enums — Resolved (one `JjColocation { Pure, Colocated }`,
  `RepositoryKind`)
- 🟡 **Architecture + Code Quality**: Migrate domain knowledge in
  `vcs-test-support` — Resolved (author decision; `stalled_migration::corpus`,
  tradeoff recorded)
- 🟡 **Architecture**: Migrate regression fails the whole suite — Resolved
  (author decision; fail-fast with a diagnostic error, tradeoff recorded)
- 🟡 **Code Quality + Architecture**: `Workbench::with` allow-list widened —
  Resolved (no `Workbench::with(` call added)
- 🟡 **Test Coverage**: Stall reused-state test checks the builder against
  itself — Resolved (independent `Stalled::on` oracle, four per-key tests,
  absolute facts)
- 🟡 **Test Coverage**: Tautological halt counts — Resolved (exact-halt-count
  boundary test)
- 🟡 **Test Coverage**: `Stall::build` untested in phase 5 — Resolved (stub
  migrate tests)
- 🟡 **Correctness + Test Coverage**: Copy count of 26 — Resolved (30)
- 🟡 **Correctness**: Five-file corpus from-scratch mismatch — Resolved
  (two-commit template and oracle)
- 🟡 **Performance**: Reused-state test risks the 20 s ceiling — Resolved
  (one oracle build per test)
- 🟡 **Performance**: Serial stall builds — Resolved (`thread::scope`)
- 🔵 **All four lenses**: `copy_private_tree` unguarded — Resolved
- 🔵 **Standards**: Invoke parameter names — Resolved
- 🔵 **Standards**: Accessor naming — Partially resolved (see new findings)
- 🔵 **Standards**: Per-family line wording — Partially resolved (phase 5
  criterion still quotes the old wording)
- 🔵 **Code Quality + Standards + Test Coverage**: `Test*` class names —
  Resolved
- 🔵 **Code Quality**: Timeline reuse; redundant masking; duplicate
  committed builders; `RepositoryState` decomposition — Resolved
- 🔵 **Code Quality**: `matrix_private_build` duplicates adopt-or-build —
  Partially resolved
- 🔵 **Correctness**: ID leaks via refs; `SharedBuildRequest` literals;
  parser padding; rekey time attribution — Resolved
- 🔵 **Correctness**: Non-total tie-break — Partially resolved
- 🔵 **Test Coverage**: Projection differential gaps; parser edge cases —
  Resolved
- 🔵 **Test Coverage**: Fallback branch only in a direct run — Still
  present (accepted by the author, recorded in phase 4)
- 🔵 **Performance**: Spawn estimate; batched projection reads; rekey
  profiling; pass count — Resolved
- 🔵 **Standards / Architecture / Code Quality suggestions**:
  `_STALLED_MIGRATION`, per-family executables, counted-iteration helper —
  Resolved (helper now duplicated; see new findings)

### New Issues Introduced

- 🟡 **Performance**: The four oracle tests and the family build may consume
  the `guarded_resume` halving headroom. A rough count puts the figure at
  ~115–120 s against ≤ 98.8 s, and the plan names no lever if the projected
  budget comes out over.
- 🟡 **Performance**: The `rekey_run` profile gate ("spawns under half a
  pass") is too lax. A 50/50 pass already misses both targets once the 14th
  halt and the charged reused-state tests count.
- 🔵 **Correctness**: Thresholds are rounded up (98.75 → 98.8, 11.65 → 11.7,
  8.95 → 9.0), so they are looser than "at most half".
- 🔵 **Correctness**: The family build time is added as a sum of two medians,
  where the work item asks for the median of per-run sums.
- 🔵 **Correctness**: `tests/workbench.rs`'s `Marked` family also implements
  `SharedFamily`/`CopyableFamily` and needs `type Inputs = ()`, but it is not
  listed.
- 🔵 **Correctness**: The tie-break still cannot order commits that differ
  only by the refs or children pointing at them.
- 🔵 **Architecture + Code Quality + Standards**: `writable_stalled_migrations`
  returns an owned `Stall`, breaking the `Held<'_, F>` accessor pattern and
  its lifetime guarantee. Its plural name also misdescribes a single stall.
- 🔵 **Architecture**: Routing `common::run` through `migrate_command` moves
  the invocation contract of all 11 migrate test binaries into
  `vcs-test-support`.
- 🔵 **Code Quality + Architecture**: `SharedBuildRequest.migrate` is a
  family-specific field on the generic request. Resolve inputs through a
  per-family hook instead.
- 🔵 **Code Quality + Standards + Correctness**: `each_of` is defined twice
  (work-adapters and migrate-cli), and its count check cannot fail. Define it
  once in `vcs-test-support`, with an expected count.
- 🔵 **Code Quality**: The `test_durations` sketch references an undefined
  `MedianTiming` and gives `--charge` no home in the pure module.
- 🔵 **Code Quality**: `SharedMatrix::shapes` takes a directory that only one
  arm uses.
- 🔵 **Test Coverage**: The five-file halt-count test's first red is a
  missing API, not the off-by-one. Build it without copying.
- 🔵 **Test Coverage**: `writable_with`'s private-build path has no test of
  its own in CI.
- 🔵 **Standards + Correctness**: The phase 5 criterion quotes
  `stalled-migration built in X s`, not
  `shared VCS fixtures: stalled-migration set built in X s`.
- 🔵 **Standards**: The new test names say "without colocation" and "merge",
  not `Pure` and `MergeWithSibling`.
- 🔵 **Standards**: `stall_build.rs` breaks the `stalled_migration_*` test
  file naming.
- 🔵 **Suggestions**: SIGTERM and malformed `--charge` tests;
  `preflight_vcs_parity.rs` keeps its own `COLOCATIONS`; `SharedMatrix`
  variants could use the `Acquisition` terms (`Adopted`, `Built`).

### Assessment

Every previous critical-path finding is resolved or was settled by an
explicit author decision. The structural design is now sound across all six
lenses. Two major findings remain, both performance budget arithmetic:
whether `guarded_resume` and `rekey_run` can meet their halving targets once
the added verification work is charged. Neither blocks implementation,
since phases 3 and 6 now gate on profiles. But the gates should be derived
from the thresholds, and the plan should name a fallback lever. The rest are
minor consistency fixes, most of them mechanical.

## Re-Review (Pass 3) — 2026-11-23T16:30:28+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Performance**: The oracle tests and family build consume the
  `guarded_resume` headroom — Resolved. Per-colocation oracles share a
  copied prefix, projections are capped at 2 spawns, and an escalating
  budget gate is in place. The estimate is now ~85 s against ≈ 98.75 s.
- 🟡 **Performance**: `rekey_run` profile gate too lax — Resolved
- 🔵 **Performance**: Family build charged at its contended wall time —
  Still present, but conservative (it can only over-charge), so it is not
  re-raised.
- 🔵 **Correctness**: Thresholds rounded up; sum of medians; `Marked` not
  listed; tie-break not total; `each_of` cannot fail; criterion wording —
  Resolved
- 🔵 **Architecture + Code Quality + Standards**: Owned `Stall` breaks the
  `Held` pattern — Resolved (`Held<'_, StalledMigrations>`, `Stalled<'w>`)
- 🔵 **Architecture + Code Quality**: Family-specific request field —
  Resolved (`inputs_from_environment`)
- 🔵 **Architecture**: Migrate invocation contract in `vcs-test-support` —
  Resolved (accepted and recorded)
- 🔵 **Code Quality + Standards**: `each_of` duplicated — Resolved
  (`vcs_test_support::counted`)
- 🔵 **Code Quality**: `SharedMatrix` argument; figure types and charging —
  Resolved (`RunFigure` named but not sketched)
- 🔵 **Test Coverage**: Halt-count red from a missing API; `writable_with`
  untested; SIGTERM and malformed `--charge` — Resolved
- 🔵 **Standards**: Test names; `stall_build.rs`; second colocation idiom
  (scoped out); `SharedMatrix` vocabulary — Resolved

### New Issues Introduced

- 🟡 **Correctness**: Phase 6 depends on phase 3 (`copy_private_tree` and
  its guard entry), and phase 5 on phase 2 (`JjColocation`,
  `RepositoryState`). The graph and the rollback notes claim otherwise.
- 🟡 **Performance**: The plan does not say whether the oracle tests are
  charged to each scenario's 20 s ceiling or only to the binary total. If
  per scenario, the slowest test could breach 20 s, and the gate projects
  only the total.
- 🔵 **Correctness**: `--serves …=stalled-migration` names the family, but
  the build lines carry the heading `stalled-migration set`. The key that
  `family_builds` uses is unstated.
- 🔵 **Correctness**: `Committed` holds no `Hermetic`, but
  `then_committing`, `committed(vcs)`'s `(repo, env)` callers and the
  projections all need one.
- 🔵 **Standards + Correctness**: The `SharedFamily` signatures leave out
  the `PrivateBuild` token and misstate `writable`'s bound
  (`CopyableFamily<Inputs = ()>`).
- 🔵 **Standards**: `--serves <binary>=<family>` reads backwards from
  `served_by`.
- 🔵 **Standards**: Stub tests in an integration test would need a public,
  unlogged `Stall::build`. Drive them through `writable_with` instead.
- 🔵 **Code Quality**: `StalledMigrations::stall(key)` has no failure path,
  unlike `States::get`.
- 🔵 **Architecture**: `ACCELERATOR_SHARED_FIXTURES_MIGRATE` is defined in
  three places, with no Python-side contract check.
- 🔵 **Test Coverage**: Several gaps:
  - `each_of` has no over-count test;
  - each tie-break key has no test of its own;
  - `--charge` and `--serves` have no success-path test at task level;
  - the split oracle is never run under the scenarios before they move to
    copies.
- 🔵 **Performance**: The derivation of per-class (`jj` and migrate)
  under-suite rates is underdetermined.
- 🔵 **Suggestions**:
  - a `TestId` value instead of `(str, str)`;
  - sketch `RunFigure`;
  - price the reused-state test at the binary-wide under-suite ratio.

### Assessment

Pass 2's findings are all resolved or accepted, and the design is
converging. No lens raised a structural concern about the approach. Two
major findings remain. One is a factual error in the phase dependency
graph, fixed by moving `copy_private_tree` into phase 2 or by correcting
the graph. The other is an unstated interpretation of how the oracle tests
count against the 20 s per-test ceiling, which needs the author's call.
The minor findings are specification precision fixes, most of them
mechanical.

## Verdict Update — 2026-11-23T16:46:38+00:00

**Verdict:** APPROVE

The author approved the plan after pass 3's findings were addressed. The
two major findings were the phase dependency graph and how the oracle
tests count against the 20 s ceiling. The first was fixed by moving
`copy_private_tree` into phase 2. The second was settled by an agreed
interpretation recorded in the plan. The minor findings were applied as
specification fixes.
