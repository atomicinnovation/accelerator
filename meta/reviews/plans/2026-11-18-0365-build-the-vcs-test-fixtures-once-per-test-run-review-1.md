---
type: "plan-review"
id: "2026-11-18-0365-build-the-vcs-test-fixtures-once-per-test-run-review-1"
title: "Plan Review: Build the VCS Test Fixtures Once per Test Run Implementation Plan"
date: "2026-11-18T20:12:06+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-11-18-0365-build-the-vcs-test-fixtures-once-per-test-run"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "performance", "safety", "portability", "usability"]
review_number: 1
review_pass: 4
tags: ["build-system", "performance", "testing", "rust", "vcs", "nextest", "mise"]
last_updated: "2026-11-19T00:40:07+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Build the VCS Test Fixtures Once per Test Run Implementation Plan

**Verdict:** REVISE

The plan is architecturally sound. It pairs a pure Python domain module with a
thin task, and a library `SharedBuildRequest` with a thin `[[bin]]`. The
hand-over is explicit and fails closed. It adds no compile configuration, lands
in incremental phases with cheap rollback, and is driven test-first throughout.
It does, however, contain several concrete logic errors that would break
specific steps:
- the `test(classify)` filter that the acceptance checks rely on selects none
  of the classify consumers;
- the builder lookup is ambiguous between the bin and its libtest harness;
- the consumer-drift guard cannot fire as specified;
- jj's operation log probably keeps an absolute path;
- node's `--test-name-pattern` would land after the file list.

Around these, the Python↔Rust fixture-log contract, the `finally`/interrupt
semantics and the lock/sweep edge cases need tightening.

### Cross-Cutting Themes

- **The consumer-drift guard does not work as claimed** (flagged by:
  correctness, usability, architecture, test-coverage). Phase 2 counts a
  private build as a problem only for a family the run *shares*. In a filtered
  run that skips the family, the missing consumer builds privately and the run
  passes. In an unfiltered run it simply adopts. So `CONSUMERS` drift is never
  caught, and where it surfaces at all, it lands on whoever filters next with a
  misleading message. The map is also pinned to individual Rust test names.
- **The fixture-log schema is an untested cross-language contract** (flagged
  by: architecture, code-quality, test-coverage). Rust's serde writes the log
  and Python reads it; the Python tests use a hand-written sample. The planned
  `#[serde(tag = "event", rename_all = "kebab-case")]` renames variants only,
  so the log would carry `hermetic_root` while the plan expects
  `hermetic-root`. Variable names are also duplicated across the two languages.
- **The `finally` sequence: interrupts, raising cleanup steps, missing trees**
  (flagged by: code-quality, architecture, safety, correctness). The `finally`
  turns `KeyboardInterrupt` into an `Exit`. A failing digest or `close()`
  skips the remaining cleanup and masks the original failure. And after a
  forced builder failure, digesting a tree that doesn't exist raises
  `FileNotFoundError`.
- **Lock/sweep edge cases** (flagged by: safety, architecture, correctness).
  - The flocked descriptor is missing from the `FixtureRun` interface.
  - Run-id uniqueness is unspecified, and `rename` replaces an existing file.
  - A sweep can delete a starting run's `*.lock.tmp` before it is flocked.
  - Ambiguous liveness errors (EACCES, a flock `OSError`) must count as "keep".
  - One undeletable leftover could block every later run.
- **The private-build fallback is never exercised in CI** (flagged by:
  test-coverage, architecture). Once the task hands trees over, the real
  `build_privately` paths for `PlainGit`, `Conflicts` and the matrix run only
  in manual checks. Yet the guidance tells developers to rely on them.
- **The `Family` model mixes concepts** (flagged by: architecture,
  code-quality). Rust's `Family` includes `PrivateSets`, which can't be handed
  over, while Python's has two values. Each family is also a hand-written field
  in `Handover` and `SharedBuildRequest`.
- **Temp-path assumptions** (flagged by: portability, safety). Python hands
  over the uncanonicalised `/var/...` while the builders write
  `/private/var/...`. The docs use `$TMPDIR`, which is unset on Linux. A
  shared `/tmp` root has no per-user component.
- **The hold switch hangs silently** (flagged by: usability, safety). It
  prints nothing and has no timeout.
- **Symlink timestamps in `copy_part`** (flagged by: portability,
  correctness). The copy needs `symlink_metadata` and `set_symlink_file_times`,
  and explicit directory modes.
- **A changed-tree failure doesn't say what changed** (flagged by: usability,
  architecture). There is one opaque digest, and the tree is deleted before
  anyone can inspect it.

### Tradeoff Analysis

- **Coupling vs over-building.**
  - Architecture argues for coarsening `CONSUMERS` to binary ids, accepting a
    few seconds of over-build on filtered runs.
  - Performance values building only what's needed, and suggests narrowing the
    pre-listing using `CONSUMERS`.
  - **Recommendation:** binary-level granularity. The status-log goldens
    binary is the one case where over-building costs a status-log build for a
    private-set test, and that is cheap.
- **Test-only switches vs a clean domain API.** Code-quality would move the
  forced failure out of `build()` and question the hold switch. Test-coverage
  and safety value both switches for testing failure and SIGKILL behaviour.
  **Recommendation:** keep both switches, but inject the forced failure in the
  bin's `main`, not in `build`'s signature. Give the hold switch a printed
  instruction and a timeout.

### Findings

#### Critical

None.

#### Major

- 🟡 **Correctness**: `test(classify)` selects none of the six classify
  consumer tests
  **Location**: Phase 3 Manual Verification; Phase 5 Manual Verification
  (SIGKILL check)
  nextest's `test()` is a substring match on the test name, and the classify
  tests say `classifies` or `misclassifies` (verified in `classify.rs`). The
  filter matches only non-consumer unit tests, so no matrix is built, and the
  SIGKILL check's runs B and C exercise nothing.
- 🟡 **Correctness**: The unit listing emits two `shared-fixtures` bin
  artifacts
  **Location**: Phase 2 §2 `builder_executable`
  A `[[bin]]` defaults to `test = true`, so cargo emits both the libtest
  harness and the real bin. Picking the harness makes every run fail at the
  build step, possibly intermittently.
- 🟡 **Correctness / Usability / Architecture / Test Coverage**: The
  private-build check cannot catch a consumer missing from `CONSUMERS`
  **Location**: Phase 2 §2 `problems()`; Phase 3 §2
  The check covers only families the run shares. A filtered run that skips a
  family lets a missing consumer build privately and pass, and in an
  unfiltered run the consumer just adopts. The message ("although the run
  shares it") would also be wrong in the drift case.
- 🟡 **Correctness**: jj's operation log probably records `jj workspace add`'s
  absolute path
  **Location**: Phase 4 §2
  `build_jj_states` passes an absolute path (verified, `status_log.rs:157-167`),
  and jj stores each operation's command line in its `args` tag. If so, the
  red scan test stays red, and every `Everything` copy fails its path scan in
  Phase 5.
- 🟡 **Correctness**: An appended `--test-name-pattern` lands after the file
  list
  **Location**: Phase 8 §1–2
  The lanes run `node --test --test-reporter=tap {discovered}` (verified), so
  appending puts the pattern after the positional file arguments. The
  floor-skip branch would then hide the fact that nothing was filtered.
- 🟡 **Architecture / Code Quality / Test Coverage**: The fixture-log schema
  has no shared contract
  **Location**: Phase 1 §1; Phase 2 §2
  A field rename passes both suites, and the serde attributes as planned
  already disagree with the documented field names.
- 🟡 **Code Quality / Architecture / Safety / Correctness**: Exception
  handling in the `finally` is under-specified
  **Location**: Phase 2 §3 step 7
  It converts interrupts, lets cleanup failures mask the real failure, and
  digests trees that may not exist.
- 🟡 **Safety**: The sweep's liveness check must fail closed on anything but
  a successful lock
  **Location**: Phase 2 §2 `sweep_abandoned()`
  Only a successful `LOCK_EX | LOCK_NB` should mark a run abandoned. Any other
  error should keep the run, so that a live run's trees can't be deleted.
- 🟡 **Portability**: The hand-over uses the uncanonicalised temp spelling
  while the builders write canonical paths
  **Location**: Phase 2 §2 `SHARED_FIXTURES_ROOT`; Phase 1 §2 `Handover`
  On macOS the containment checks, such as "hermetic root not inside
  `<run>/matrix`", can pass trivially.
- 🟡 **Test Coverage / Architecture**: The real private-build fallback is only
  verified manually
  **Location**: Phase 5 §2; Testing Strategy
  No automated lane runs `StatusLogPart::build_privately` or the matrix's
  private build. Nothing pins that a part built privately has the same keys as
  a copy of that part.
- 🟡 **Test Coverage**: Nothing enforces that the full-set goldens test still
  makes all 38 comparisons
  **Location**: Phase 5 §3 Assertion map
  A filter that dropped a state would still pass. Asserting the set of
  compared names against the golden files would close this.
- 🟡 **Code Quality**: Fixture lifetime depends on keeping the `Workbench`
  alive, but nothing enforces it
  **Location**: Phase 1 §2, §4
  `let (_, matrix) = matrix()?` drops the workbench and deletes a private
  tree. That breaks only on direct nextest runs, never under the task.
- 🟡 **Code Quality**: The "pure domain module" carries about seven
  responsibilities and duplicates existing helpers
  **Location**: Phase 2 §2
  It duplicates `tasks/shared/locking.py`, `tasks/shared/hashing.py` and
  `instrumented_build.UnitListing.parse`. It also does filesystem I/O, so
  "pure" is inaccurate.
- 🟡 **Usability**: A changed-tree failure gives no clue which file changed or
  which test wrote it
  **Location**: Phase 2 §2 `tree_digest` / `problems`
- 🟡 **Usability**: Filters that match nothing are unspecified and behave
  differently from lane to lane
  **Location**: Phase 3 §3; Phases 7–8
  nextest reports "cli tests failed", while libtest, node and Vitest exit 0
  having run nothing.

#### Minor

- 🔵 **Test Coverage**: The manifest write/adopt round trip has no direct
  test, contrary to the plan's claim (Phase 1 §3).
- 🔵 **Test Coverage**: The new `status_log_states` test changes the test
  count that the acceptance criterion pins (Phase 4 §2).
- 🔵 **Test Coverage**: Process and timing tests need readiness handshakes and
  timeouts (Phase 2 §2).
- 🔵 **Test Coverage**: The status-log cases for consumer selection are not
  listed (Phase 5 §4).
- 🔵 **Correctness**: Nothing says who creates `<run>/matrix` and
  `<run>/workbenches`. `assert_no_repository_ancestor` canonicalises, so the
  base must already exist (Phase 2 §2–3).
- 🔵 **Safety / Correctness**: A concurrent sweep can delete another run's
  `*.lock.tmp` before it is flocked (Phase 2 §2).
- 🔵 **Safety / Architecture**: Run-id uniqueness, a no-replace rename and the
  lock descriptor's lifetime are not specified (Phase 2 §2).
- 🔵 **Safety**: One undeletable leftover could block every later run
  (Phase 2 §2).
- 🔵 **Safety**: The private-build guard passes when its evidence is missing:
  an empty log or unparseable lines (Phase 2 §2).
- 🔵 **Safety / Usability**: The hold switch waits forever, with no message
  (Phase 2 §2–3).
- 🔵 **Safety**: Hand-over variables inherited from the shell are not cleared
  (Phase 3 §3).
- 🔵 **Architecture / Code Quality**: Rust's `Family` mixes the shared
  families with the private-sets category, and adding a family touches about
  eight places (Phase 1, Phase 2 §1).
- 🔵 **Architecture**: The tree digests are not recorded in the fixture log,
  as Requirement 13 asks (Phase 2 §2–3).
- 🔵 **Code Quality**: Three inconsistent private-build signatures, and an
  undefined `SharedCopyable` / `writable::<F>` (Phase 1 §2, Phase 5 §2).
- 🔵 **Code Quality**: Test-only switches sit in production code paths
  (Phase 2 §1, §3).
- 🔵 **Code Quality**: `FixtureLogs.create` prunes as a hidden side effect,
  and the `scan` result is a free-form string (Phase 2 §2, Phase 5 §2).
- 🔵 **Portability**: The path scan bridges only the `/var/folders` spelling
  (Phase 4 §1).
- 🔵 **Portability / Safety**: The docs and manual steps assume `$TMPDIR` is
  set, so on Linux `rm -rf "$TMPDIR/..."` targets `/` (Migration Notes).
- 🔵 **Portability**: A single, unscoped directory in a shared Linux `/tmp`
  is not per-user (Phase 2 §2).
- 🔵 **Correctness / Portability**: Copying needs `lstat`- and
  `lutimes`-aware handling for symlinks, and explicit directory modes
  (Phase 5 §1).
- 🔵 **Performance**: The wall-time saving is first measured only after both
  increments land, so there's no interim data point at Phase 3 (Phase 6).
- 🔵 **Performance**: The serial overhead on the critical path is not
  budgeted, and the status-log build time has not been measured
  (Performance Considerations).
- 🔵 **Performance**: A filtered run lists every test binary twice (Phase 3).
- 🔵 **Performance**: Nothing automated guards against nextest recompiling in
  the coverage-off and filtered lanes (Performance Considerations).
- 🔵 **Usability**: nextest's own filterset parse error must reach the user
  (Phase 3 §3).
- 🔵 **Usability**: The test-only switches don't fit the "Contributor
  environment variables" framing (Phase 2 §3).
- 🔵 **Usability**: mise's help omits the fact that a filtered run skips
  coverage (Phase 3 §3).
- 🔵 **Usability**: What `--filter` does on the roll-ups has not been checked
  (Phase 9).
- 🔵 **Usability**: The rule "`cargo test` is not supported for `cli/`"
  conflicts with the visualiser lanes (Phase 9 §2).

#### Suggestions

- 🔵 **Architecture**: Consider making the adopted trees read-only so that a
  writer fails at the point of the write (Phase 2 §3).
- 🔵 **Architecture / Code Quality**: `Selection` gathers every runner's
  syntax, plus the npm `--` separators and leading spaces. `Family.ALL` on a
  `StrEnum` becomes a member (Phases 7–8, Phase 2 §3).
- 🔵 **Performance**: Parallelise each family's independent groups if the
  build step turns out to be material (Phase 2, Phase 5 §4).
- 🔵 **Portability**: Option forwarding was verified on one shell only; check
  dash on ubuntu (Phases 3, 7, 8).
- 🔵 **Portability**: Record the nextest and mise coupling in the pin-bump
  procedure (Phase 3 §2).
- 🔵 **Test Coverage**: A `jj status` run with `--ignore-working-copy` can't
  detect working-copy divergence (Phase 5 §1).
- 🔵 **Test Coverage**: Pin the exact unfiltered command for the four new
  invoke tasks (Phase 7 §2).
- 🔵 **Safety**: Point `test_cli_permission_skip.py` at temporary roots, or
  make that redirection autouse (Phase 2 §3).
- 🔵 **Usability**: Give a concrete example in every lane's help; say so when
  a filtered run builds no family; reject an unrecognised
  `ACCELERATOR_SHARED_FIXTURES_FAIL` value.

### Strengths

- ✅ It has a clean functional core and imperative shell on both sides: a pure
  `shared_fixtures.py` behind a thin `cli.py`, and `SharedBuildRequest` behind
  a thin `[[bin]]` that is the only part reading the environment.
- ✅ The hand-over is explicit and can be injected: `Workbench::with(Handover)`
  plus a cheap marker-file family make the workbench tests fast, free of git,
  and independent of process-global environment.
- ✅ A broken hand-over fails closed, and private builds of shared families
  become run failures. The efficiency invariant is enforced rather than hoped
  for.
- ✅ The lock lifecycle is ordered: the lock file is locked under a temporary
  name and renamed before the directory exists, and `close()` removes the
  directory before the lock. The sweep therefore never sees a live run's
  directory without its lock.
- ✅ No new compile configuration: the builder is a `[[bin]]` under
  `tests/fixtures/`, following an established precedent and staying outside
  the pup and store-duplication lints. The listing moves earlier rather than
  being added.
- ✅ Phase 4 makes the status-log set path-free, which is an improvement in
  its own right and is protected by byte-identical goldens and a red-first
  scan test.
- ✅ Copies set every mtime from the source, directories last, so git's and
  jj's stat caches behave the same in a copy.
- ✅ Consumer counts were verified: 19 matrix adoptions; six copies, one copy
  and two adoptions for the status-log set; four private-set builds. The
  `with_goldens()` keys give the same 38 comparisons.
- ✅ The fixture summary is tested against the real `timeline` regexes, which
  protects `measure:build`.
- ✅ There is one consistent `--filter` name with runner-specific help, and
  the Phase 9 guard spells out the exempt set.
- ✅ The trade-offs are stated openly, the speed criterion bounds the task
  duration as well as the nextest phase, and rollback is cheap.

### Recommended Changes

1. **Fix the classify filter** (addresses: `test(classify)` selects none of
   the classify consumers). Replace `test(classify)` with
   `binary_id(vcs-adapters::classify)` throughout the manual checks, the
   SIGKILL check, the test examples in Phase 3 §3 and the help-text example.
   For the test case for "only classify tests match", use the binary id.
2. **Disambiguate the builder artifact** (addresses: two `shared-fixtures`
   artifacts). Set `test = false` on the `[[bin]]`, and/or require
   `profile.test == false` in `builder_executable`. Add a harness-shaped
   artifact to the listing fixture.
3. **Make private builds of a shared family unconditional problems, and
   detect drift in unfiltered runs** (addresses: the consumer-drift guard,
   missing evidence, the wrong message).
   - Any `private-build` of `matrix` or `status-log` is a problem whenever a
     log exists.
   - In an unfiltered run, an adoption or copy by a test outside `CONSUMERS`
     is a problem.
   - So is a requested family with no `shared-build` event.
   - So is an empty log or an unparseable line.

   Consider coarsening `CONSUMERS` to binary ids.
4. **Drive the jj workspace path into the Phase 4 red test** (addresses: the
   jj operation log's absolute path). Add `jj-secondary-main` to the expected
   failures, and pass a relative path to `jj workspace add`. Check jj 0.43's
   workspace store.
5. **Put `--test-name-pattern` before `{discovered}`** (addresses: node
   argument order). Have the lane test assert the order.
6. **Pin the fixture-log contract** (addresses: the cross-language schema).
   - Use `rename_all_fields = "kebab-case"`.
   - Have a Rust test serialise one of each `Event` and compare it with a
     checked-in golden JSONL, and have the Python tests parse that same file.
   - Add a check that the variable names agree.
   - Optionally add a `tree-digest` event.
7. **Specify the `finally` policy** (addresses: exception handling,
   interrupts, missing trees, who creates directories).
   - A non-`Exit` `BaseException` closes the run and is re-raised unchanged.
   - Each cleanup step is best-effort and records its own failure as a
     problem.
   - Digest only the families whose pre-build digest was taken.
   - `start()` creates `<run>/workbenches` and the tree roots.
   - Put the "outcome plus problems gives an `Exit`" logic in a pure function.
8. **Tighten the lock and sweep specification** (addresses: sweep liveness,
   run id, `lock.tmp` race, descriptor lifetime, undeletable leftovers,
   per-user root).
   - `FixtureRun` owns the open descriptor and `close()` releases it last.
   - Run ids use a uuid, and the rename refuses to replace an existing file.
   - The sweep opens locks without `O_CREAT` and treats any error as "keep".
   - It ignores young `*.lock.tmp` files and is best-effort over other runs'
     debris.
   - Reuse the primitives in `tasks/shared/locking.py`.
   - Scope the root per user, or create it with mode 0700.
9. **Canonicalise the temp root and the handed paths** (addresses: path
   spellings, `$TMPDIR` docs). Use `Path(tempfile.gettempdir()).resolve()`,
   and canonicalise in `Handover::from_environment`. Write
   `${TMPDIR:-/tmp}` in the docs, or print the resolved root in the summary.
   Derive the path scan's alternative spellings with `canonicalize`.
10. **Add automated private-build coverage** (addresses: fallback untested in
    CI, the manifest round trip, the 38 comparisons).
    - Add a Rust test per `StatusLogPart` that builds privately in
      `Workbench::with(Handover::default())` and checks its keys against
      `entries()`.
    - Add a manifest write→adopt round-trip test.
    - Assert that the compared golden names equal the golden files.
11. **Enforce the workbench lifetime in the types** (addresses: lifetime
    trap). Return a named struct or a guard that borrows the workbench.
12. **Split `shared_fixtures.py` along its domain seams** (addresses: god
    module). Separate the run lifecycle, the fixture log and consumer
    selection. Reuse `locking.py`, `hashing.py` and
    `UnitListing.executable()`.
13. **Make tree-changed failures diagnosable** (addresses: the opaque digest).
    Digest per entry and list the added, removed and modified paths.
14. **Define what a filter that matches nothing does** (addresses: zero-match
    inconsistency). `test:unit:cli` should fail early using the listing it
    already makes. Record the behaviour for each runner in the Phase 7 table.
15. **Smaller items:**
    - Separate `Family` from the log category.
    - Give the hold switch a printed instruction and a timeout.
    - Clear inherited hand-over variables.
    - Use `symlink_metadata` and `set_symlink_file_times` in `copy_part`.
    - Take an interim measurement at Phase 3 and budget the serial overhead.
    - Show nextest's parse stderr.
    - Align the mise and invoke help.
    - Scope the `cargo test` rule to exclude the visualiser.
    - Record the change in the `status_log_states` test count in the
      work-item amendment.

## Per-Lens Results

### Architecture

**Summary**: The plan is structurally sound: it pairs a functional core with
an imperative shell, injects the hand-over explicitly, and lands in
incremental phases with easy rollback. The main risk is the new contract
between the Python build system and the Rust test suite. The fixture-log
schema and the family vocabulary are written separately in two languages, and
`CONSUMERS` pins individual Rust test names. Secondary concerns:
- the `Family` model mixes concepts;
- the lock handle is missing from the `FixtureRun` interface;
- the post-run digest delays cleanup on interrupt;
- the private-build fallback goes unexercised in CI.

**Strengths**: a functional core and imperative shell on both sides; a
hand-over that can be injected; fail-closed adoption; run-scoped directories
with flock liveness; reuse of the precedent for fixture bins without new
build configuration; a path-free status-log set; trade-offs stated openly;
cheap rollback.

**Findings**:
- 🟡 major / high, **The cross-language fixture-log schema has no shared
  contract** (Phase 1 §1, Phase 2 §2). The serde `rename_all` on a tagged enum
  renames variants only. Make one side own the schema and add a golden JSONL
  contract test.
- 🟡 major / medium, **`CONSUMERS` couples the build system to individual Rust
  test names** (Phase 3 §2, Phase 5 §4). Drift is found late, and by the wrong
  person. Coarsen it to binary ids, or have the Rust side declare consumption.
- 🔵 minor / high, **The `Family` model mixes shared families with log
  categories** (Phase 1, Phase 2 §1). Key `Handover` and `SharedBuildRequest`
  by family.
- 🔵 minor / high, **The tree digests are not recorded in the fixture log**
  (Phase 2 §2–3). Add a `tree-digest` event.
- 🔵 minor / medium, **The lock handle is missing from `FixtureRun`**
  (Phase 2 §2).
- 🔵 minor / medium, **The `finally` ordering and exception conversion weaken
  interrupt handling** (Phase 2 §3).
- 🔵 minor / medium, **The private-build fallback is never exercised in CI**
  (Phase 5 §2).
- 🔵 suggestion / low, **The digest guard cannot say which test wrote**.
  Consider read-only adopted trees.
- 🔵 suggestion / low, **`Selection` gathers every runner's syntax in one
  class**.

### Code Quality

**Summary**: The domain vocabulary is strong, and there are good seams for
testing. The weak points are:
- the fixture-log contract is duplicated across Rust and Python, and the
  serde naming already disagrees;
- a Python module calling itself pure domain carries many responsibilities;
- the merging of errors in the `finally` is under-specified;
- smaller API-shape issues.

**Strengths**: rich, consistent domain language; `Workbench::with(Handover)`
as a seam for injection; `build` takes the log and the forced failure as
parameters; symmetric `build_in` / `build_for_sharing` / `adopt` replacing
dead environment paths; `_instrumented_environment` lifted out; one
`Selection` for quoting; a consumer list that polices itself (but see
correctness).

**Findings**:
- 🟡 major / medium, **The fixture-log schema and family names are duplicated
  with no contract check** (Phase 1 §1, Phase 2 §2).
- 🟡 major / medium, **The merging of exceptions in the `finally` is
  under-specified** (Phase 2 §3 step 7).
- 🟡 major / medium, **Fixture lifetime depends on keeping the `Workbench`
  alive** (Phase 1 §2, §4).
- 🟡 major / high, **The "pure domain module" carries about seven
  responsibilities and duplicates helpers** (Phase 2 §2).
- 🔵 minor / high, **Rust's `Family` mixes shared families with private
  sets**.
- 🔵 minor / medium, **There are three inconsistent private-build
  signatures**, and an undefined `SharedCopyable`.
- 🔵 minor / medium, **Each family is a hand-written field, so adding one is
  shotgun surgery**.
- 🔵 minor / medium, **Test-only switches live in production code paths**.
- 🔵 minor / medium, **`create` has a hidden side effect, and the scan result
  is a string**.
- 🔵 suggestion / low, **`Family.ALL` on a `StrEnum`, and `Selection`
  renderings with embedded spaces and `--`**.

### Test Coverage

**Summary**: The plan is strongly test-led. It has explicit red conditions,
fault-injection switches, real SIGKILL tests, byte-identical goldens and a
standing digest guard. The gaps:
- nothing automated checks that the fixture log or `CONSUMERS` is complete;
- the private-build fallback runs in no automated lane;
- a few claims that behaviour is preserved are not enforced (the 38
  comparisons, the manifest round trip).

**Strengths**: a named failing test before each change; cheap fake families;
fault injection; summary tests against the real regexes; regenerated goldens
diffed; a standing guard on every run; an exempt set spelled out in the
Phase 9 guard; existing assertions kept.

**Findings**:
- 🟡 major / high, **No full-run check that the fixture log and `CONSUMERS`
  are complete** (Phase 3 §2, Phase 2).
- 🟡 major / high, **The real private-build fallback is verified only
  manually** (Phase 5 §2).
- 🟡 major / medium, **Nothing enforces all 38 golden comparisons**
  (Phase 5 §3).
- 🔵 minor / high, **The manifest round trip has no direct test**
  (Phase 1 §3).
- 🔵 minor / high, **The new `status_log_states` test changes the pinned test
  count** (Phase 4 §2).
- 🔵 minor / medium, **Process and timing tests need deterministic
  synchronisation and timeouts** (Phase 2 §2).
- 🔵 minor / medium, **The status-log selection cases are not listed**
  (Phase 5 §4).
- 🔵 suggestion / low, **`jj status --ignore-working-copy` cannot detect
  working-copy divergence** (Phase 5 §1).
- 🔵 suggestion / medium, **Pin the exact unfiltered command for the four new
  invoke tasks** (Phase 7 §2).

### Correctness

**Summary**: The core design holds, and most factual claims check out.
Several logic errors would break specific steps:
- `test(classify)` matches no classify consumer;
- the builder lookup is ambiguous;
- the drift guard cannot fire;
- jj's operation log probably keeps an absolute path;
- node's pattern argument would land after the files.

**Strengths**: mtime preservation keeps the stat caches equivalent; the lock
and directory ordering avoids TOCTOU; the hand-over fails closed; the
invariant is checked on every run; consumer counts and the 38 comparisons
verified; the `fixture_tempdir_path` mask doesn't depend on the directory
prefix.

**Findings**:
- 🟡 major / high, **`test(classify)` selects none of the six classify
  consumers** (Phase 3 and Phase 5 Manual Verification).
- 🟡 major / high, **Two `shared-fixtures` bin artifacts are emitted**
  (Phase 2 §2).
- 🟡 major / medium, **The private-build check cannot catch `CONSUMERS`
  drift** (Phase 2 §2 vs Phase 3 §2).
- 🟡 major / medium, **jj's operation log probably records `jj workspace
  add`'s absolute path** (Phase 4 §2).
- 🟡 major / medium, **`--test-name-pattern` is read as file arguments**
  (Phase 8 §1–2).
- 🔵 minor / medium, **Nothing says who creates the tree and workbench
  directories, or what a missing tree digests to** (Phase 2 §2–3).
- 🔵 minor / medium, **The combined `Exit` from the `finally` turns an
  interrupt into a failure** (Phase 2 §3).
- 🔵 minor / medium, **A concurrent sweep can delete a `*.lock.tmp` before it
  is flocked** (Phase 2 §2).
- 🔵 minor / low, **Copy fidelity needs symlink- and mode-aware handling**
  (Phase 5 §1).

### Performance

**Summary**: The plan targets the right bottleneck: repeated, spawn-bound
fixture builds inside the slowest tests. It replaces them with one concurrent
build, zero-copy adoption and small partial copies, and adds no compile
configuration. The gaps are in measuring and budgeting:
- there is no interim measurement before both increments have landed;
- the serial build-step overhead is not budgeted;
- filtered runs list twice;
- there is no automated guard against a repeated compile.

**Strengths**: targets the measured cost; zero-copy matrix adoption; no repeated
compile; the instrumented environment kept for filtered runs; concurrent
families and selective builds; an append-only log with no locks; the task
duration is bounded too.

**Findings**:
- 🔵 minor / medium, **The wall-time saving is first measured only after both
  increments** (Phase 3, Phase 6).
- 🔵 minor / medium, **The serial overhead on the critical path is not
  budgeted, and the status-log build time has not been measured**
  (Performance Considerations).
- 🔵 suggestion / medium, **Parallelise the independent groups within each
  family if needed**.
- 🔵 minor / low, **A filtered run lists every test binary twice**
  (Phase 3).
- 🔵 minor / medium, **Nothing automated guards against a recompile in the
  coverage-off and filtered lanes**.

### Safety

**Summary**: The plan takes accidental harm seriously. Deletion is confined to
one dedicated root, flock gives liveness, the hand-over fails closed, digests
and scans guard the trees, and rollback is cheap. The remaining risks are at
the edges:
- an ambiguous liveness check in the sweep;
- `finally` steps that can skip cleanup;
- a guard that passes when its evidence is missing;
- a hold switch with no timeout.

**Strengths**: the lock lifecycle ordering; deletion confined to one root; the
hand-over fails closed; guards in both directions; writes moved out of shared
state; failure and recovery tested directly; cheap rollback.

**Findings**:
- 🟡 major / medium, **The sweep's liveness check must fail closed**
  (Phase 2 §2).
- 🔵 minor / medium, **Run-id uniqueness, a no-replace rename and the
  descriptor's lifetime are unspecified**.
- 🔵 minor / medium, **Sweeping unlocked `*.lock.tmp` files races with a run
  that is just starting**.
- 🔵 minor / medium, **A raising `finally` step skips cleanup and masks the
  original failure**.
- 🔵 minor / medium, **One undeletable leftover could block every later
  run**.
- 🔵 minor / medium, **The private-build guard passes when its evidence is
  missing**.
- 🔵 minor / high, **The hold switch waits forever with no timeout**.
- 🔵 minor / medium, **Hand-over variables inherited from the shell are not
  cleared**.
- 🔵 suggestion / medium, **Task tests not redirected to temporary roots act
  on the real temp root and logs**.
- 🔵 suggestion / low, **The manual cleanup command misses the Linux location**.

### Portability

**Summary**: Broadly portable. The trees live under system temp, liveness is
scoped to the run id, and the status-log set becomes path-free. The main gap
is the macOS path spelling: Python hands over `/var/...` while the builders
write `/private/var/...`. Smaller issues are a `/tmp` root that isn't scoped
per user, docs that assume `$TMPDIR`, and coupling to nextest and mise
interfaces.

**Strengths**: shared state outside the checkout; per-run liveness for
concurrent workspaces; a path-free status-log set; filesystem differences
accounted for; a fallback to direct nextest; tool versions recorded; both CI
legs in the criteria.

**Findings**:
- 🟡 major / high, **The hand-over paths use the uncanonicalised temp
  spelling** (Phase 2 §2, Phase 1 §2).
- 🔵 minor / medium, **The path scan bridges only the `/var/folders`
  spelling** (Phase 4 §1).
- 🔵 minor / high, **The docs and manual steps assume `$TMPDIR` is set**.
- 🔵 minor / medium, **A single, unscoped directory in a shared Linux `/tmp`
  is not per-user**.
- 🔵 suggestion / low, **Option forwarding was verified on one host's
  `/bin/sh`**.
- 🔵 suggestion / medium, **Symlink timestamps must be set without following
  the link**.
- 🔵 suggestion / medium, **Record the coupling to nextest and mise in the
  pin-bump procedure**.

### Usability

**Summary**: There is one consistent `--filter` across every lane, help that
names each runner's syntax, early failure on a malformed filter, and messages
that name the variable or path to edit. The weak points are diagnostics:
- a changed-tree failure doesn't say where;
- drift errors reach the wrong developer;
- filters that match nothing are inconsistent;
- the hold switch hangs silently;
- the coverage skip is missing from mise's help.

**Strengths**: one option name; help specific to each runner; announced skips;
a fail-closed hand-over with an actionable message; the summary printed
whether the run passes or fails, with the log path; direct nextest still
works; the Phase 9 guard forces a decision for each new lane.

**Findings**:
- 🟡 major / high, **A tree-changed failure gives no clue which file changed
  or which test wrote it** (Phase 2 §2).
- 🟡 major / high, **`CONSUMERS` drift is detected only on filtered runs, so
  the error reaches the wrong developer** (Phase 3 §2).
- 🟡 major / medium, **Filters that match nothing are unspecified and
  inconsistent** (Phase 3 §3, Phases 7–8).
- 🔵 minor / medium, **Make sure nextest's own filterset parse error reaches
  the user**.
- 🔵 minor / high, **The hold switch waits silently, with no instruction on
  how to release it**.
- 🔵 minor / high, **The test-only switches don't fit the "Contributor
  environment variables" framing**.
- 🔵 minor / high, **mise's help omits the fact that a filtered run skips
  coverage**.
- 🔵 minor / medium, **What `--filter` does on the roll-ups has not been
  checked**.
- 🔵 minor / high, **"`cargo test` is not supported for `cli/`" conflicts
  with the visualiser lanes**.
- 🔵 suggestion / medium, **Give a concrete example in every lane's help**.
- 🔵 suggestion / medium, **Say so when a filtered run builds no shared
  family**.
- 🔵 suggestion / low, **Reject an unrecognised
  `ACCELERATOR_SHARED_FIXTURES_FAIL` value**.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-11-18T23:07:03+00:00

**Verdict:** COMMENT

All eight lenses ran again. Every critical and major finding from pass 1
is resolved, except two that are partly resolved and called out below. The
seven design decisions held up under review. Two new major findings were
introduced by the edits, so the plan is acceptable as it stands but could
be improved; see the new issues below.

### Previously Identified Issues

- 🟡 **Correctness**: `test(classify)` selects none of the classify consumers — Resolved
- 🟡 **Correctness**: Two `shared-fixtures` bin artifacts — Resolved
- 🟡 **Correctness / Usability / Architecture / Test Coverage**: The private-build check cannot catch `CONSUMERS` drift — Resolved
- 🟡 **Correctness**: jj's operation log records an absolute path — Resolved (driven into the red test)
- 🟡 **Correctness**: An appended `--test-name-pattern` lands after the file list — Resolved
- 🟡 **Architecture / Code Quality / Test Coverage**: The fixture-log schema has no shared contract — Resolved
- 🟡 **Code Quality / Architecture / Safety / Correctness**: Exception handling in the `finally` is under-specified — Partially resolved. Failures before the yield and `SharedFixturesError` are not classified, and on the interrupt path a failing `close()` can replace the interrupt.
- 🟡 **Safety**: The sweep's liveness check must fail closed — Resolved
- 🟡 **Portability**: The hand-over uses the uncanonicalised temp spelling — Resolved
- 🟡 **Test Coverage / Architecture**: The private-build fallback is only verified manually — Partially resolved. `PlainGit` and `Conflicts` are now covered. The private dispatch for the matrix and `Everything` is not, and the claim that `Everything`'s private path is `build_in` is inaccurate.
- 🟡 **Test Coverage**: Nothing enforces all 38 golden comparisons — Resolved
- 🟡 **Code Quality**: Fixture lifetime is unenforced — Partially resolved (`private_states` still returns a bare map)
- 🟡 **Code Quality**: The god module duplicates helpers — Resolved
- 🟡 **Usability**: The changed-tree failure gives no clue — Partially resolved (the paths are listed, but there is no route to the test that wrote them)
- 🟡 **Usability**: Filters that match nothing are inconsistent — Resolved
- 🔵 Minor and suggestion findings: nearly all resolved. Still partly open: the serial-overhead budget, automated recompile detection outside unfiltered coverage-on runs, the roll-up "ignore" outcome, and the nextest pin-bump note (which lacks `NEXTEST_*`).

### New Issues Introduced

- 🟡 **Test Coverage / Correctness**: The new copy and part tests build the full status-log set directly. `copies.rs` (about five tests) and `status_log_parts.rs` each call `States::build_in` in their own process, outside any `Workbench`. That adds back roughly the builds the increment removes, and neither the log nor the private-build guard can see them.
- 🟡 **Safety**: The sweep deletes any unlocked directory under a root that has not been validated and whose symlinks are followed. The sweep runs before `start()` checks the root's owner and mode, works on the canonical (symlink-followed) root, and removes directories with any name.
- 🔵 **Correctness / Test Coverage**: The "listed consumer used no fixture" rule fails every happy-path task test as specified, because the fake nextest call writes no adoptions. `test_cli_permission_skip.py`'s `MockContext` creates no tree at all.
- 🔵 **Correctness / Safety**: On the interrupt path, `close()` is not isolated, and a second signal can abort cleanup (`unwind_then_defer` exists in `signals.py`).
- 🔵 **Code Quality / Architecture**: The run lifecycle (about a dozen steps and the isolation policy) sits in `cli.py`, where only fakes of `context.run` can test it. A `SharedFixtureSession` in the package would fix that. `RunOutcome` and `prune` sit in the wrong modules or test files, `prune` needs knowledge of run liveness, and `test_consumers.py` is marked "(new)" twice.
- 🔵 **Code Quality**: Rust `Family` declares `StatusLog` in Phase 1, while Python adds it in Phase 5. `States` lacks a `build_with(base, env)` that takes a given `Hermetic`. `Handover: Default` needs a default `TestName`.
- 🔵 **Correctness**: Rebasing a part copy may expose keys that were never copied. Stray events from tests that don't use a test-local log can satisfy or trip the evidence checks. The check that a `shared-build` event exists should also match the tree and apply to filtered runs.
- 🔵 **Architecture**: A builder failure now blocks the whole cli suite, and the plan does not state that as a trade-off.
- 🔵 **Portability**: The permission-based sweep tests need the repo's `geteuid() == 0` skip, or injected primitives. Zero-match detection depends on runner output text; use exit codes where they exist (nextest 4, pytest 5).
- 🔵 **Usability**: The pytest lanes' `usage` snippet is invalid TOML (escaped `'` inside a literal string). The private-build message gives consumer-list advice in unfiltered runs, where a stale list can't be the cause.
- 🔵 **Performance**: A filtered coverage-on listing runs instrumented binaries, which write profile data. Binary-level consumption over-builds for some filters. The two digests run one after the other.
- 🔵 Suggestions: the root-name check would be better as a shared variables file; the summary's `trees:` path has already been deleted when it prints; an empty `--filter ''` falls back silently to the full suite; the "selected no tests" message gives no next step; lock-holding child processes in the tests may be orphaned; directory modes should be applied after their contents; record the nextest version in the `nextest-list.json` capture.

### Assessment

Pass 1's revisions fixed every correctness bug, settled the cross-language
contract, the consumer drift and the lock/sweep semantics, and made
`--filter` consistent. The plan is now structurally sound. Two new major
issues are worth fixing before implementation, and both are small edits:
1. Give `copies.rs` and `status_log_parts.rs` a logged, shared or narrowly
   scoped source instead of a direct `States::build_in`.
2. Validate the root (lstat, no symlink, owner and mode) before sweeping,
   and limit deletion to names in the run-id grammar.

The new minor findings are mostly about how well sketch and prose agree:
the cleanup policy before the yield, the fakes in the task tests, the TOML
quoting, where `Family::StatusLog` first appears, and `States::build_with`.
These can be fixed in the same pass or during implementation.

## Re-Review (Pass 3) — 2026-11-19T00:02:00+00:00

**Verdict:** COMMENT

All eight lenses ran again against the pass-2 findings. Both pass-2 majors
are resolved. Every lens rates the plan structurally sound. Two new majors,
both in the new `copy_fidelity.rs` design, keep the verdict at COMMENT.

### Previously Identified Issues

- 🟡 **Test Coverage / Correctness**: the new copy and part tests built the full status-log set directly — Resolved. `copies.rs` now uses hand-written trees, `copy_fidelity` and `status_log_parts` go through workbenches, and a direct-build guard is added.
- 🟡 **Safety**: the sweep ran over a root that had not been checked and that could be a symlink — Resolved. `FixtureRuns.open` checks the root with lstat, and deletion is limited to the run-id grammar.
- 🟡 **Code Quality / Architecture / Safety / Correctness**: the `finally` was under-specified — Resolved. A single `SharedFixtureSession` policy now covers it. Some edge cases remain; see the new issues.
- 🟡 **Test Coverage / Architecture**: the private-build fallback was verified only manually — Resolved. The `build_with` symmetry is in place and the part tests cover it.
- 🟡 **Code Quality**: the workbench lifetime was not enforced — Resolved. `private_states` now returns a `Fixture`.
- 🟡 **Usability**: the changed-tree failure gave no route to the writer — Resolved. The message now names the consumers to rerun.
- 🔵 Pass-2 minor findings and suggestions — nearly all Resolved. Still partly open:
  - `RunOutcome` still sits in `runs.py`, and `test_consumers.py` is still marked "(new)" twice;
  - empty-`--filter` handling has not reached the later lanes;
  - the throwaway profile file for the filtered listing.

### New Issues Introduced

- 🟡 **Code Quality / Correctness**: when the tests run directly, a workbench's fixtures collide. With nothing handed over, `status_log_states()` builds privately into `workbench.path()`. `copy_fidelity` then copies, and `status_log_parts` builds its parts, into that same directory, so they overwrite or fail on existing states. The copies made directly with `copy_part` are also not logged, although Phase 5's manual check expects to find them in the log.
- 🟡 **Correctness**: the jj snapshot assertion cannot hold. `dirty-jj`, `deleted-jj` and `rename-jj` carry edits that have never been snapshotted, and no `JJ_TIMESTAMP` or `JJ_RANDOMNESS_SEED` is set. Any snapshot therefore rewrites `@` with a fresh commit id, and "identical output, no new change" fails on those three states. (Verified in `status_log.rs`.)
- 🔵 **Safety / Correctness**: the plan mis-describes `unwind_then_defer`, which drops later signals rather than re-raising them (verified in `signals.py`). A first signal during cleanup on the `Exit` path is not classified. `close()` lacks the sweep's make-writable retry.
- 🔵 **Architecture / Code Quality / Correctness**: the session classifies on invoke's `Exit`. The pattern elsewhere in `tasks/shared` raises a domain error, which the task translates. The hold-under-CI refusal is raised outside the session, so it is never translated, and its session test cannot be written as specified.
- 🔵 **Architecture / Test Coverage / Correctness**: the direct-build guard misses `build_plain_git_states`, `build_conflict_states` and `build_adversarial_state`. It lands in Phase 5, after the matrix consumers move in Phase 1. Two bypasses are not fenced: `Workbench::with` with a local log, and direct `copy_part`. Making the builders `pub(crate)` would let the compiler enforce the rule.
- 🔵 **Correctness**: a builder failure may add a spurious "log records no shared build" problem. Inherited variables can only be removed with `replace_env=True`. `Handover::from_environment` should treat an empty value as unset.
- 🔵 **Test Coverage**: the canonicalisation test reads the run's real variables. The `close()`-failure test depends on a fault that the retry would defeat. No test covers a raising `close()` or `prune` on the `Exit` path.
- 🔵 **Usability**:
  - the pytest-lane test asserts a shortened no-match message;
  - empty-`--filter` forwarding and the `ValueError` are not carried into the lanes;
  - builder-failure, root-refusal and sweep warnings give no remedy;
  - the "Handover did not come from the environment" message gives no fix.
- 🔵 **Portability**:
  - two read-only tests lack the root skip;
  - the run lock's behaviour on a filesystem without flock or hard links is unspecified;
  - the root is resolved at import time.
- 🔵 **Performance**: the coverage-on lane keeps a redundant no-op cargo listing pass for the sweep, which the budget doesn't count.
- 🔵 Suggestions:
  - move the contract files out of the Python test tree;
  - add `family.rs` and `errors.py` modules;
  - define `DigestPhase` and `TreeChanges`, and pass the summary its root explicitly;
  - have `copy_part` return `named_by` as data;
  - remove the duplicated part and workbench APIs;
  - prune only logs named by run id;
  - keep the guard-test directory out of the checkout;
  - handle two first runs creating the root at once;
  - clear the hold variable in the task-test fixture.

### Assessment

Every lens rates the plan structurally sound. The cross-language contract,
the consumer checks, the lock and sweep, the session lifecycle and the
`--filter` surface have all converged. Both remaining majors are local to
`copy_fidelity.rs` and the workbench layout:
1. Give each fixture a workbench its own subdirectory, or one fixture per
   workbench, so private builds and copies never share a path.
2. Restate the snapshot check as "a snapshot in each copy picks up the same
   changes". For example, compare `jj diff --summary`, or pin `JJ_TIMESTAMP`
   and `JJ_RANDOMNESS_SEED` and expect the same new operation.

The minors are mostly precision fixes: the signal wording, error
translation at the boundary, the guard's name list, and message remedies.
None blocks implementation.

## Re-Review (Pass 4) — 2026-11-19T00:30:28+00:00

**Verdict:** COMMENT

All eight lenses ran again. No critical or major findings remain: both
pass-3 majors are resolved, and every lens rates the plan structurally
sound. What is left is minor findings and suggestions, so the verdict stays
COMMENT. APPROVE needs no findings at all.

### Previously Identified Issues

- 🟡 **Code Quality / Correctness**: workbench fixtures collided in direct runs — Resolved. Each workbench now holds one fixture, and `copy_fidelity` and `status_log_parts` use separate workbenches.
- 🟡 **Correctness**: the jj snapshot assertion could not hold — Resolved as a false failure, but the replacement check is weak (see below).
- 🔵 Error boundary, hold-under-CI, signal wording, `close()` retry, spurious "no shared build", `replace_env`, empty values, canonicalisation seam, cleanup-failure tests, root skips, flock/link behaviour, lazy root, messages and remedies, contract location, module split, `named_by` as data, prune scope, root race — Resolved.
- 🔵 Direct-build guard — Partially resolved. The builders are `pub(crate)`, but the public `build_privately` and `build_part_privately` trait methods, and `StatusLogPart::build_privately`, still let a test bypass the log.
- 🔵 Duplicated workbench accessors — Partially resolved. They are accepted as sugar, but the tests mix the two spellings.

### New Issues Introduced

- 🔵 **Correctness**: invoke catches `KeyboardInterrupt` inside `context.run` and forwards `\x03` to the child (verified, `runners.py:490`). A Ctrl-C during the build or nextest therefore reaches the session as a command failure, not as an interrupt, and the session's unconditional `deferred_signals` overrides the task's `_hold`. Pass the task's `Interruption` to the session, and enter `deferred_signals` only when no signal has arrived, as `build_runner.py:93-96` does.
- 🔵 **Correctness / Test Coverage**: the `jj diff --summary` check compares two copies with each other, so it cannot detect a copy that differs from its source. Compare each state's summary with its known edits instead (`A new-file.txt`, `D gone.txt`, `D old.txt` plus `A new.txt`, otherwise empty). Copy the whole `Everything` part each time, so `jj-secondary` keeps its store.
- 🔵 **Correctness**: the "signal during cleanup" test injects a `KeyboardInterrupt` that the per-step `except Exception` isolation does not catch. Deliver a real signal from the fake instead.
- 🔵 **Code Quality / Architecture**: the session's constructor sketch lacks the seams its tests inject (hold factory, removal, pruning, output). Phase 5 still names `fixture_log.rs` and `handover.rs` for `Family`. `build_conflict_states` is described as both `pub` and `pub(crate)`.
- 🔵 **Test Coverage**: Phase 1 workbench tests call `writable()` and `private_states()`, which only exist from Phase 5. The build-error test should assert that `src/` is unchanged, with `build_in` checking ancestors before rooting its `Hermetic`. `selection_from` has no test file or lane case.
- 🔵 **Safety**:
  - concurrent prunes race on `unlink`; use `missing_ok`;
  - the sweep should decide a directory is an orphan by opening its lock at decision time, not from the listing;
  - `tree_digest` must not open FIFOs or other non-regular files;
  - the make-writable walk should touch only directories.
- 🔵 **Usability**:
  - `selection_from` is specified only in a README bullet;
  - the `<task>` in `nothing_selected` must be the mise name;
  - the recompile and failing-list messages should name their likely causes;
  - the one-fixture error could say what the workbench already holds.
- 🔵 **Portability**: the `os.link` error mapping should cover every `OSError` except `EEXIST`, not only `EPERM` and `ENOTSUP`.
- 🔵 **Performance**: count the private part builds of `status_log_parts` in the cost accounting, and note the cost of running the direct-run consumers directly.
- 🔵 Suggestions:
  - rename `tasks/test/selection.py`, or put it in `helpers.py`;
  - the private-set builders are callable unlogged (accept this, or guard them);
  - the session hard-codes the cli nextest command, so move that hint to the task;
  - rename `_fixture_runs`;
  - consider a consuming `Workbench` API.

### Assessment

The plan has converged. No lens raises anything that would cause wrong
behaviour across the suite or need significant rework. The one finding with
real behavioural effect is how interrupts are classified under invoke, and
that is a local fix modelled on `build_runner.py`. The rest are precision
fixes that can be made now or during implementation. The plan is ready for
`/accelerator:implement-plan`.

## Verdict Update — 2026-11-19T00:40:07+00:00

**Verdict:** APPROVE

After pass 4, every remaining minor finding and suggestion was applied to
the plan, except the optional consuming-`Workbench` API. The reviewer
approved the plan, and it is marked ready for implementation.
