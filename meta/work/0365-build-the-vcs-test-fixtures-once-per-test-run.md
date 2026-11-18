---
type: "work-item"
id: "0365"
title: "Build the VCS test fixtures once per test run"
date: "2026-10-28T16:18:46+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "ready"
kind: "story"
priority: "medium"
parent: "work-item:0361"
blocks: ["work-item:0391"]
relates_to: ["work-item:0264", "work-item:0363", "work-item:0364", "work-item:0366", "work-item:0368", "work-item:0370", "work-item:0391"]
source: "codebase-research:2026-10-28-full-build-duration-contention-and-slowness"
tags: ["performance", "testing", "rust", "vcs"]
last_updated: "2026-11-18T18:21:11+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-957"
---

# 0365: Build the VCS test fixtures once per test run

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The VCS tests build two expensive fixture families with real `git` and `jj`
processes: the matrix and the status-log set. Each test makes its own
private build, so one `test:unit:cli` run builds the matrix 19 times and the
status-log set 6 times. Those tests are among the slowest in the nextest
phase and take about 870 of its 2,818 test-seconds. Test-seconds are the sum
of every test's own duration, which exceeds the phase's wall time because
tests run in parallel. The saving is the repeated fixture building inside
those 870 test-seconds, not all of it.

This item builds each family once per run, in a step of the `test:unit:cli`
task, and shares it. It also gives every multi-test `test:*` task a
`--filter` option, so that running a subset of tests goes through the tasks
rather than the underlying runners, and rewrites the guidance to match.

## Context

`test:unit:cli` is the longest task in a warm no-op `mise run` (a run with
caches warm and nothing changed): a median of 255 s in
`build_2026-11-17T070532Z_engywook_warm-no-op_devtools.json`, whose whole
build took a median of 277 s on the measurement host, engywook. Isolated
(`measure:build --isolate`, the task run on its own) at `287b6279` on
`main`, under the `devtools` host-settings profile, the task's duration has
a median of 192.8 s (min 174.8, max 207.9) and its nextest phase a median
of 181.9 s (min 164.6, max 197.6), over 7,727 tests and 2,818 test-seconds
(`build_2026-11-18T123358Z_engywook_warm-no-op_devtools_isolated.json`).

The matrix consumers, 19 tests and 730 test-seconds:
- `vcs-adapters::classify`, 6 tests at 45–46 s each;
- `vcs-test-support::matrix`, 6 tests at 37–38 s each;
- `vcs-adapters::scrub`, 2 tests at 39–41 s each;
- `vcs-adapters::queries`, 5 tests at 30–31 s each.

Apart from `accelerator-migrate::guarded_resume`, these are among the
slowest tests in the run. One matrix build takes 3.3 s wall and 1.9 CPU-s
when run alone; the October research measured the same tests at 35–47 s
under the full suite's load.

The status-log set is built by four tests in
`accelerator-vcs::status_log_goldens` (22–23 s each),
`vcs-adapters::conflicts` (29 s) and the `vcs-test-support::status_log_states`
self-test (20 s). Two more `accelerator-vcs::status_log_goldens` tests build
the smaller plain-git subset. The private sets (cap, bookmark and parity)
are built by four further tests at 1–6 s each.

Both families already have an adopt path. `Matrix::build_or_adopt` and
`status_log::States::build_or_adopt` adopt a tree built earlier, named by
`ACCELERATOR_ZERO_SPAWN_MATRIX` and `ACCELERATOR_ZERO_SPAWN_STATUS_LOG`, and
build their own when neither is set. The zero-spawn integration suite
(`test:integration:zero-spawn*`) set those variables; 0264 retired it, so
nothing sets them today and every build is private.

The October research ruled out two other explanations. nextest's
process-per-test model is not the overhead: 338 tests took 10.8 s under
nextest and 11.1 s under `cargo test`. And the phase is not CPU-saturated:
CPU ran at about 50% busy when the phase ran alone.

Today the root `CLAUDE.md` tells readers to drop to the underlying runner
to run one test, because the `mise run test:*` tasks have no filter.

## Requirements

Fixture families, as this item uses the terms:
- **the matrix**: the tree `Matrix::build_in` builds;
- **the status-log set**: the full set of states `States::build_or_adopt`
  builds, which contains the plain-git subset's states;
- **the private sets**: the cap, bookmark and parity sets. They belong to
  neither family and are always private builds.

1. Build the matrix once per `test:unit:cli` run. Every matrix consumer in
   the audit below uses that shared build, by adopting it or, under
   Requirement 7, by working in a copy of it.
2. Build the status-log set once per run. These tests use that shared
   build, by adopting it or, under Requirement 7, by working in a copy of
   it: the four `status_log_goldens` tests that build the full set, the two
   that build the plain-git subset (whose copies may cover only that part),
   the `status_log_states` self-test and `conflicts`.
3. `test:unit:cli`'s invoke task builds the shared fixtures in a step
   before nextest's test phase starts. The step:
   - builds the families the selected tests consume. A run without a
     filter builds both and may overlap with compiling the tests; a
     filtered run first finds the selected tests with `cargo nextest list`
     and skips any family none of them consumes;
   - may build the two families concurrently;
   - hands each shared build to the tests explicitly. A test adopts or
     copies only when it is handed a build, and builds its own when handed
     none;
   - hashes each shared tree, over relative paths, file contents and modes
     but not timestamps, just after building it and just before removing
     it, and scans each copy for the shared tree's absolute path as the
     copy is made. Copies are made through one helper so that each is
     scanned. A hash mismatch or a path match fails the run and names the
     tree or copy;
   - removes the shared trees and copies (Requirement 12) and prints the
     fixture summary (Requirement 13) when the run ends, pass or fail.

   The step works in every lane that runs these tests: coverage on
   (`cargo llvm-cov nextest`) and off (`ACCELERATOR_COVERAGE=off`), and
   CI's `test-unit` job on its `ubuntu-latest` and `macos-latest` legs.
4. `test:unit:cli` takes a `--filter <filterset>` option, declared by the
   invoke task and exposed by the mise task itself, so
   `mise run test:unit:cli --filter 'test(classify)'` works and the option
   appears in the task's help. The task rejects arguments it does not
   declare, and nextest rejects a malformed filterset before any fixture is
   built.
5. Every other leaf `test:*` task that runs more than one test takes a
   `--filter` option in the same way. Its value uses the syntax of the
   runner the task drives: pytest's `-k` expression, Vitest's test-name
   pattern, Playwright's `--grep`, or, for a task whose mise `run` calls an
   invoke task that drives its own runner, that runner's selection syntax.
   The task's help names the syntax, and the runner rejects a malformed
   expression where its syntax has one. Exempt are the single-check guards
   `test:integration:release-guard`, `test:integration:deny`,
   `test:integration:fixture-size`, `test:integration:measure`,
   `test:integration:skill-invocation` and `test:integration:conformance`,
   and the roll-ups `test:unit`, `test:integration` and `test:e2e`.
   Planning lists each in-scope task here, with its runner, its filter
   syntax and an example of a malformed expression, before the filters
   increment starts.
6. Each test that uses a shared build gets its own hermetic environment
   (the isolated home, XDG and jj configuration that `Hermetic` sets up),
   outside every shared tree and separate from every other test's.
7. A test that writes into a shared tree stops writing, if it can do so
   without losing an assertion, and adopts the shared build. Otherwise it
   works in its own copy, taken from the shared build rather than built
   afresh. A copy does not refer back to the shared tree, whether through
   linked worktrees, jj workspace paths or any other absolute path, and
   gives the test the view a private build gives it today, including the
   file timestamps that git's index and jj's working-copy state compare. A
   copy may cover only the part of the tree its test uses, provided that
   part looks as it would in a private build. Copy-on-write cloning, where
   the filesystem supports it, is the suggested approach; planning chooses
   the method. For each conditional row in the audit, planning decides
   whether the test adopts or copies, records which assertion adopting
   would lose if it copies, and writes the outcome into the audit table
   before that family's increment is checked.

   The audit found these writers:
   - `scrub` writes `poison.gitconfig` into the matrix's base directory,
     the root of the matrix tree;
   - `conflicts` runs `jj resolve --list` without `--ignore-working-copy`,
     so the jj CLI snapshots the working copy and records an operation;
   - the six `status_log_goldens` tests that use the status-log set run
     `jj status`, which takes the working-copy lock and writes store
     objects without committing an operation, so they work in copies;
   - the `status_log_states` self-test builds an extra adversarial state
     beyond the full set. It adopts if it can build that state outside the
     shared tree, and otherwise works in a copy.
8. The private sets stay private builds.
9. `mise run <task> --filter …` is the supported way to run a subset of
   tests. A VCS test run through `cargo nextest` directly is handed nothing,
   builds its own fixtures and passes. Plain `cargo test` is not a supported
   way to run the `cli/` tests.
10. The root `CLAUDE.md`'s "Running a single test" section and the
    test-running notes in `tasks/README.md` direct readers to each task's
    `--filter` option, name the exempt tasks, rule out `cargo test`, and
    treat running an underlying runner directly as a last resort.
11. The build step has two test-only switches: one makes a named family's
    shared build fail, and one holds the run after the build step until it
    is released. A failed shared build fails the build step, so the run
    stops before nextest's test phase and reports one failure with its
    cause.
12. Each run removes the shared trees and copies it created when it ends,
    whether it passed, a test failed or a shared build failed. A run also
    removes trees left by an earlier, interrupted run, but never those of
    a run still in progress. All of them live under one location or name
    prefix, which planning chooses.
13. A test writes fixture events only when it is given a fixture log
    location; the build step gives one, and a direct nextest run does not.
    The fixture log is kept after cleanup, at a location planning chooses,
    and records:
    - each shared build, with its family and the `git` and `jj` versions
      that built it;
    - each adoption, copy and private build, with its family (or "private
      sets"), the test's name and the test's hermetic root;
    - each shared tree's two hashes and each copy's path scan.

    When the run ends, pass or fail, `test:unit:cli` prints a summary with
    a heading for each family and one for the private sets, counting shared
    builds, adoptions, copies and private builds. No line of it matches
    nextest's `Summary [` line, which `measure:build` reads.
14. `ACCELERATOR_ZERO_SPAWN_MATRIX`, `ACCELERATOR_ZERO_SPAWN_STATUS_LOG`,
    `MATRIX_ROOT_VARIABLE` and `STATES_ROOT_VARIABLE` name a suite that no
    longer exists. If they keep a role in handing a shared tree to a test,
    rename them for that purpose. If they lose it, remove them and the
    code that reads them.
15. The item lands in three increments, each when every criterion tagged
    for it passes:
    - **matrix**: matrix sharing, with the build step, `test:unit:cli`'s
      `--filter`, the hash and path guards, the test-only switches, cleanup
      and the fixture log;
    - **status-log**: status-log sharing, after the matrix increment;
    - **filters**: the other tasks' `--filter` options and the guidance,
      after the matrix increment.

    A criterion tagged [each family] is checked for the matrix at the
    matrix increment and for the status-log set at the status-log
    increment. [fixtures closure] criteria are checked once both fixture
    increments have landed; that releases 0391 and gives 0361 its result.
    The item closes once the filters increment has also landed.

Mutation audit of today's consumers:

| Consumer | Tests | Builds | Effect on the fixtures | Uses the shared build by |
|---|---|---|---|---|
| `vcs-adapters::classify` | 6 | matrix | read-only, through `InProcessProbe` | adopting |
| `vcs-adapters::queries` | 5 | matrix | read-only; one test changes the process's working directory | adopting |
| `vcs-test-support::matrix` | 6 | matrix | read-only structural checks | adopting |
| `vcs-adapters::scrub` | 2 | matrix | writes `poison.gitconfig` into the matrix's base directory | adopting if it can stop writing, otherwise a copy |
| `accelerator-vcs::status_log_goldens` (full set and plain-git subset) | 6 | status-log set | runs the binary; `jj status` takes the working-copy lock and writes store objects but commits no operation | a copy |
| `vcs-adapters::conflicts` | 1 | status-log set | `jj resolve --list` snapshots and records an operation | adopting if it can stop snapshotting, otherwise a copy |
| `vcs-test-support::status_log_states` | 1 | status-log set plus adversarial state | reads the full set; builds an adversarial state | adopting if it builds that state outside the shared tree, otherwise a copy |
| `accelerator-vcs::status_log_goldens` (cap, bookmark), `status_log_parity` | 4 | private sets | read-only | private build; out of scope |

## Acceptance Criteria

- [ ] [matrix] Given a full `test:unit:cli` run, when it completes, then
      the fixture log shows one matrix shared build and, for each of the
      19 matrix consumers, an adoption or a copy as the audit table
      records, and no private build.
- [ ] [status-log] Given a full `test:unit:cli` run, when it completes,
      then the fixture log shows one status-log shared build and, for each
      test named in Requirement 2, an adoption or a copy as the audit table
      records, and no private build.
- [ ] [status-log] Given a full `test:unit:cli` run, when it completes,
      then the fixture log shows a private build under "private sets" for
      each private-set consumer.
- [ ] [each family] Given a full `test:unit:cli` run, when it completes,
      then the fixture log shows matching hashes for the family's shared
      tree and a clean path scan for each of its copies. Given a test made
      to write into the shared tree, the run fails and names the tree.
      Given a copy with the shared tree's absolute path planted in its VCS
      metadata, the scan fails the run and names the copy.
- [ ] [each family] Given a full `test:unit:cli` run, when it completes,
      then the fixture log shows a distinct hermetic root for each test
      using the family's shared build, none inside a shared tree.
- [ ] [each family] Given a copy made by the copy helper and a private
      build of the same family, when they are compared, then every file's
      modification time in the copy matches its source, and
      `git status --porcelain` and `jj status --ignore-working-copy` give
      the same output in both.
- [ ] [each family] Given every consumer in the audit table whose test
      code changes, when it is compared with `287b6279`, then its golden
      files are byte-identical, it keeps the same test names and count, and
      every assertion and golden comparison it made there is still made,
      as shown by the plan's mapping of each assertion to its new place.
- [ ] [matrix] Given `mise run test:unit:cli --filter 'test(classify)'`,
      when it runs, then the matrix alone is built, the `classify` tests
      adopt it, and they pass. Given a malformed filter or an undeclared
      argument, the task fails before any fixture is built. Given
      `mise run test:unit:cli --help`, `--filter` is listed.
- [ ] [status-log] Given `mise run test:unit:cli --filter 'test(conflicts)'`,
      when it runs, then the status-log set alone is built, `conflicts`
      adopts or copies it as the audit table records, and it passes.
- [ ] [each family] Given a filter that selects no consumer of the family,
      when `test:unit:cli` runs, then the family is not built.
- [ ] [each family] Given each consumer row in the audit table, run
      through `cargo nextest run -E <filter>` directly with a filter
      selecting only that row's tests, when it runs, then the tests pass,
      no shared fixture location is created and no fixture log is written.
- [ ] [each family] Given the test-only switch failing the family's shared
      build, when `test:unit:cli` runs, then the run stops before
      nextest's test phase with one failure naming that family and its
      cause, the fixture log has no event for that family's consumers, and
      the fixture summary is printed.
- [ ] [each family] Given a passing run, a run with a failing test and a
      run whose shared build fails, when each ends, then nothing it created
      remains under the shared fixture location, the fixture log remains
      with the `git` and `jj` versions on each shared-build event, the
      fixture summary was printed, and no output line outside nextest's
      own matches nextest's `Summary [` line.
- [ ] [each family] Given run A held after its build step and then killed
      with SIGKILL, and run B held after its build step, when run C
      starts, then A's trees are removed and B's remain, and B passes once
      released.
- [ ] [matrix] Given `cli/`, `tasks/`, `mise.toml` and `.github/`, when
      they are searched for `ACCELERATOR_ZERO_SPAWN_MATRIX` or
      `MATRIX_ROOT_VARIABLE`, then nothing matches.
- [ ] [status-log] Given `cli/`, `tasks/`, `mise.toml` and `.github/`,
      when they are searched for `ACCELERATOR_ZERO_SPAWN_STATUS_LOG` or
      `STATES_ROOT_VARIABLE`, then nothing matches.
- [ ] [each family] Given `test:unit:cli` with coverage on, with coverage
      off, and on CI's `ubuntu-latest` and `macos-latest` `test-unit`
      legs, when each runs, then it passes and its fixture summary shows
      the family built once.
- [ ] [filters] Given each task listed in Requirement 5, when it is run
      with `--help`, with a filter selecting a subset, and with its listed
      malformed expression, then its help lists `--filter` and names the
      syntax, fewer tests run and all of them pass, and the malformed
      expression fails the task.
- [ ] [filters] Given the root `CLAUDE.md` and `tasks/README.md`, when they
      are read, then they direct readers to each task's `--filter`,
      including `test:unit:cli`'s, name the exempt tasks, rule out
      `cargo test`, and present running an underlying runner directly only
      as a last resort.
- [ ] [fixtures closure] Given a full `test:unit:cli` run, when it
      completes, then nextest's summary line reports at least as many tests
      run as the before run's summary line (7,727 for the 18 November
      baseline).
- [ ] [fixtures closure] Given the baseline
      `meta/measurements/build_2026-11-18T123358Z_engywook_warm-no-op_devtools_isolated.json`,
      or a fresh before-record taken under Dependencies, when
      `mise run measure:build -- --isolate test:unit:cli
      --host-settings devtools --runs 3 --keep` is recorded and compared
      with
      `mise run measure:compare-builds -- <baseline> <after> --task test:unit:cli`,
      then:
      - the first line reports no differing condition (both records were
        taken under the same host and build conditions);
      - the nextest-phase median is at least 25 s shorter (156.9 s or less
        against the 18 November baseline) with a noise verdict of
        "beyond" (outside the baseline's run-to-run spread);
      - the task-duration median is at least 25 s shorter (167.8 s or less
        against the 18 November baseline), also "beyond".

## Open Questions

Planning answers each of these:
- Do the fixtures contain linked worktrees, jj workspaces or other absolute
  paths that a copy would carry back to the shared tree? If they do, the
  copy helper rewrites those paths or copies by a method that keeps them
  relative.
- Does the status-log set contain the plain-git subset's states? If not,
  the two plain-git tests become private-set consumers and the saving
  shrinks slightly.
- For each conditional audit row, does the test adopt or copy
  (Requirement 7)?
- How are copies made (Requirement 7), where do the shared trees live
  (Requirement 12), and where is the fixture log kept (Requirement 13)?
- Are the adopt variables renamed or removed (Requirement 14)?
- What form does the fixture builders' entry point take?
- Which tasks are in scope for Requirement 5, with what syntax?

## Dependencies

- Blocked by: none.
- Blocks: 0391, which shortens the `guarded_resume` tests, the slowest left
  once this item lands. The edge is released once both fixture increments
  have landed, whatever the state of the filters increment, since removing
  the repeated fixture building changes the conditions those tests run in.
- Parent: 0361's closing comparison counts on this item's saving.
  Abandoning either fixture increment needs agreement on 0361.
- Coordinated with 0364, which changes some of the same test tasks
  (pytest-xdist in `tasks/test/integration.py`, Playwright worker counts).
  The filters increment rebases onto 0364's changes and does not merge
  inside 0364's CI measurement windows.
- Measurement ordering: 0363 requires this item's before and after
  timings to be taken once both of 0363's halves, coverage and visualiser,
  have landed. The 18 November baseline, taken at `287b6279` on `main`
  after 0362 and 0363 were done, meets that.
- Measurement environment: engywook with `devtools` host settings only.
  Recording must not overlap any other measurement or validation session
  on engywook, including 0364's and 0370's. If 0370 has finished and left
  other settings applied, restore `devtools`-only settings before
  recording or take a fresh before-record under the current settings. If
  0366, or any other change that affects `test:unit:cli`, lands on `main`
  before the after-record, take the fresh before-record at current `main`
  with this item's changes reverted, and capture its nextest summary
  line.
- Measured with: 0368's `measure:build` and `measure:compare-builds`. The
  nextest phase is read from nextest's `Summary [` line, so the build
  step falls outside it; the speed criterion bounds the task's duration
  as well.
- Compile configurations: the fixture builders' entry point reuses the
  tests' profile, features and target directory where possible, so it
  adds nothing to the number of compile configurations 0366 counts.
  0366's before and after records must not straddle this item's merge. If
  the entry point is a new binary or crate, it follows the matching
  checklist in `tasks/README.md`.
- Tooling: cargo-nextest and cargo-llvm-cov, pinned in `mise.toml`. The
  build step relies on `cargo nextest list` honouring the same filter as
  the run. Exposing `--filter` on mise tasks relies on mise's support for
  declaring task arguments, in the mise version used locally and in CI; a
  mise upgrade means re-checking the help output and argument rejection.
- Behaviour relied on: jj (pinned in `mise.toml`), jj-lib and gix (locked
  in `cli/Cargo.lock`), and `git`, which is not pinned: it comes from the
  Xcode command-line tools on engywook and the macOS runner, and from the
  runner image on `ubuntu-latest`. The mutation audit assumes jj-lib's
  loader and gix status write nothing, and copies rely on git's and jj's
  timestamp comparisons. Upgrading jj, jj-lib or gix, or a change in the
  `git` version the fixture log records, means re-running the audit and
  re-checking copies.
- External: GitHub-hosted `ubuntu-latest` and `macos-latest` runner images
  and the `test-unit` job in `.github/workflows/main.yml`. The copy method
  must work on each leg's filesystem, and a runner-image change means
  re-checking copies and cleanup.
- Related: 0264, which retired the zero-spawn suite that set the adopt
  variables.

## Assumptions

- A fixture tree built once gives each read-only test the same view a
  private build gives it today.
- gix status and jj-lib's loader write nothing to the fixtures. This was
  checked by reading the code; the hash guard in Requirement 3 now checks
  it on every run.
- Copying a shared tree costs much less than building one.

## Technical Notes

- Matrix adopt path: `MATRIX_ROOT_VARIABLE` and `matrix_root` at
  `cli/vcs-test-support/src/fixtures.rs:25,47-57`; `build_or_adopt` at
  `:109-116`; `build_in` at `:167`.
- Status-log adopt path: `STATES_ROOT_VARIABLE` at
  `cli/vcs-test-support/src/status_log.rs:24`; `build_or_adopt` at
  `:504-523`; `states_root` at `:568-579`.
- Writes into shared trees: `cli/vcs-adapters/tests/scrub.rs:96,133` and
  `cli/vcs-adapters/tests/conflicts.rs:49`.
- `Hermetic::rooted_at` writes `hermetic-home/`, `hermetic-xdg/` and
  `hermetic-jj.toml` under its base
  (`cli/vcs-test-support/src/hermetic.rs:46-58`), so an adopting test
  cannot root it at the shared tree.
- The fixture builders are Rust code in `vcs-test-support`, so the build
  step needs an entry point it can run.
- No mise task declares its own options today; exposing `--filter` on the
  mise tasks sets that pattern and belongs in `tasks/README.md`.
- `cargo nextest list` compiles the test binaries before listing, so a
  filtered run cannot overlap the fixture build with compilation.
- A copy's mtimes can match its source, but its ctimes and inode numbers
  cannot, which is why the copy criterion also compares `git status` and
  `jj status` output.

## Drafting Notes

- Brought the status-log set into scope alongside the matrix, as agreed in
  conversation. The private sets were left out because they cost 1–6 s
  each. The matrix lands first because it carries about 730 of the 870
  test-seconds with the least uncertainty. Splitting the status-log half
  into its own item was considered and declined; the increments are gated
  by tagged criteria instead.
- Set the speed target at 25 s off the isolated nextest-phase median, and
  the same off the task-duration median, so work moved into the build
  step cannot pass as a saving. The phase is not CPU-bound and
  `guarded_resume`'s 39 s tests become the slowest. `compare-builds` calls
  a change "beyond" noise only below the baseline's minimum (164.6 s for
  the phase, 174.8 s for the task), so 25 s clears the noise band for
  both.
- A test that writes works in a copy of the shared build, not a private
  build, so it keeps most of the saving. The goldens always work in copies,
  so the content hash can stay strict, and the hash runs on every run as
  a guard rather than once at acceptance.
- Chose a build step in `test:unit:cli`'s invoke task over a nextest setup
  script or a build made by the first test. A failed build then stops the
  run before any test starts, cleanup runs in one place whatever the
  outcome, both coverage lanes behave alike, and nothing depends on
  nextest's experimental setup scripts.
- Widened the scope, by decision, to give every multi-test `test:*` task a
  `--filter` option, so subsetting goes through the tasks for every suite.
  It lands as its own increment after the matrix. It does not hold back
  the fixture result, 0391 or 0361. Splitting it into its own item was
  considered and declined. `cargo test` is ruled out by guidance only;
  enforcement was considered and not pursued.
- Changed the kind from task to story, given three increments and the
  widened scope.
- Dropped the zero-spawn acceptance criterion: the suite no longer exists.
- Replaced the October figures with the 18 November baseline, taken at
  `main` once 0362 and 0363 had landed. 0364 had not landed; it changes only
  the Playwright and dev-task suites, so it does not affect the isolated
  `test:unit:cli` record. Removed per-test claims whose source could not be
  confirmed.

## References

- Source: `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- Baseline: `meta/measurements/build_2026-11-18T123358Z_engywook_warm-no-op_devtools_isolated.json`
- Related: 0361, 0264, 0363, 0364, 0366, 0368, 0370, 0391
