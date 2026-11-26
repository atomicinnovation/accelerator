---
type: "work-item"
id: "PP-943"
title: "Shorten the slowest tests in the CLI unit suite"
date: "2026-11-18T13:28:33+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "ready"
kind: "task"
priority: "medium"
parent: "work-item:PP-953"
relates_to: ["work-item:PP-957", "work-item:PP-958", "work-item:PP-959", "work-item:PP-960", "work-item:PP-962"]
external_id: "PP-943"
tags: ["performance", "testing", "rust", "migrate", "vcs"]
last_updated: "2026-11-26T08:17:24+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
aliases: ["0391"]
---
# PP-943: Shorten the slowest tests in the CLI unit suite

**Kind**: Task
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

After PP-957 (legacy alias 0365), the slowest tests left in
`test:unit:cli`'s nextest phase belong to three binaries, chosen for two
reasons. `accelerator-migrate::guarded_resume` spends the most
test-seconds (the sum of its tests' durations): 229, with a slowest test of
30–35 s. Three tail tests, one in `vcs-test-support::matrix_private_build`
and two in `work-adapters::rekey_run`, finish last in the phase, at 16–36 s
each. Shortening `guarded_resume` takes load off the suite. Shortening the
tail tests aims to shorten the tail the phase ends on; the criteria require
each to halve and record whether the phase's end moved. Both keep every
scenario they cover. The work serves the parent, PP-953 (legacy alias 0361):
speeding up the full build without losing confidence.

## Context

The reference figures come from the isolated `test:unit:cli` reference
record `build_2026-11-20T130237Z_engywook_warm-no-op_devtools_isolated.json`,
at `a89415bf1e`, taken after PP-957 landed. Its nextest-phase median is
143 s.

**`guarded_resume`.** Across the record's three timed runs:
- 221, 229 and 242 test-seconds in all;
- a slowest test of 30.1, 34.7 and 35.4 s, each time
  `moving_onto_new_parents_starts_a_fresh_jj_run`;
- six of the eight slowest tests in the phase.

Before PP-957, in `build_2026-11-18T123358Z_…_isolated.json` at
`287b6279`, the binary took 322 test-seconds and its slowest test 39.2 s.
PP-957 cut about 30% by reducing contention, and left `guarded_resume` the
slowest binary.

`accelerator-migrate` is the package name of the `cli/migrate-cli` crate.
A stalled migration is one halted part-way, which the migrate binary
resumes or refuses. Ten of the binary's 13 tests are jj tests that loop
over both colocations,
`--no-colocate` and `--colocate`. The other three, one Git and two jj, run
once without a colocation. On each pass a jj test builds a stalled
migration from scratch, then resumes it, rewrites its parents or edits its
files. Building the stalled state alone takes about ten `jj` CLI calls and
a run of the migrate binary, all serial within the test. These tests change
their repositories, so one shared build cannot serve them as it stands.
They finish around positions 1,370–1,780 of 8,113, counting the completion
counter on nextest's `PASS` lines, long before the phase ends. So they add
load to the suite rather than set when it ends.

**The tail tests.** Three tests finish last in the phase, at positions
8111–8113. A rekey run rewrites work-item IDs as a sequence of file
mutations; a halt point is the index of the mutation at which it is
killed. The private and shared matrix builds are PP-957's terms for a
test's own fixture build and the one a test run shares.

| Test | Durations, runs 1 / 2 / 3 (s) | Expensive work |
|---|---|---|
| `matrix_private_build` `a_workbench_handed_no_matrix_builds_the_one_a_run_would_share` | 32.0 / 29.7 / 35.8 | builds the fixture matrix twice, privately and via `shared_build`, to compare them |
| `rekey_run` `in_a_real_repository::a_run_killed_at_every_write_resumes_over_its_own_writes` | 22.4 / 20.9 / 23.7 | a fresh committed repository per halt point, for every mutation of a whole run, under Git and jj |
| `rekey_run` `in_a_real_repository::a_resume_refuses_an_edit_since_the_halt_to_a_path_not_yet_written` | 16.0 / 15.9 / 18.7 | a fresh committed Git repository per halt point |

How much of each test's time is its own work and how much is contention
is not yet known. PP-957's VCS fixture matrix is the warning case: one
build of it takes 3.3 s alone and 35–47 s under the full suite.

## Requirements

1. Establish, for each target test (the three tail tests and every
   `guarded_resume` test), how much of its time is its own work and how
   much is contention with the rest of the suite. The baseline record and
   its alone runs answer this before planning; any rebaseline record and
   the after record repeat it with their own alone runs.
2. Shorten the `guarded_resume` binary's total test-seconds and its slowest
   test without dropping a colocation, a scenario or an assertion.
3. Every `guarded_resume` scenario that runs today still runs, and so does
   every pairing of scenario and colocation. A scenario is one of today's
   test functions.
4. Research and planning choose the approach for each of the three
   binaries. Candidates for `guarded_resume` include:
   - splitting each colocation into its own test so the two run in
     parallel, which shortens the slowest test but not total test-seconds;
   - building each stalled starting state once and giving each test a
     private copy;
   - spawning fewer `jj` processes.

   No candidates have been weighed for the tail tests; research starts from
   the "Expensive work" column above.
5. Shorten the three tail tests without dropping a VCS, a halt point or an
   assertion. The `matrix_private_build` test still compares a private
   build with a shared one.
6. The `rekey_run` test
   `a_resume_refuses_an_edit_since_the_halt_to_a_path_not_yet_written`
   halts at every mutation of the run it actually performs. Today it counts
   its halt points from the four-file fixture, but it commits a fifth file,
   `meta/notes/synced-only.md`, so its last halt point never runs.
7. Each binary's change can land on its own; none waits on another. The
   timing targets are judged once, from the before record and the after
   record.
8. A reused or more cheaply built starting state is the same state a
   from-scratch build gives.

## Acceptance Criteria

The criteria judge these records, each an isolated `test:unit:cli` record
(`mise run measure:build -- --isolate test:unit:cli --host-settings
devtools --runs 3 --keep`) on the measuring host:

- **Baseline record**: at `main` when planning starts, alongside three
  alone runs of each binary.
- **Rebaseline record**: taken only if a change that affects
  `test:unit:cli` runtime lands on `main` before the after record, at
  current `main` with PP-943's changes reverted, alongside fresh alone
  runs. A change affects runtime if it touches test sources, Cargo
  dependencies, the dev profile or a toolchain pin; the implementer makes
  the call and the author confirms it.
- **Before record**: the rebaseline record if one was taken, otherwise the
  baseline record.
- **After record**: at `main` once all three binaries' changes have landed,
  alongside alone runs.

Every figure is combined within each timed run first, then taken as the
median across a record's three runs. A test's duration in a run is its
`PASS [ Ns ]` duration, counting only the final attempt of a retried test.
A starting state that target tests share is built from scratch on every
run, inside the measured `test:unit:cli` task, by extending PP-957's build
step; neither a dependency task nor a cache that survives between runs may
build it. Its build time, from its own entry in the run log, is added to
the figure of the tests it serves: to `guarded_resume`'s total
test-seconds, or to that tail test's duration. `guarded_resume`'s total
test-seconds sums the tests the plan maps from its before tests, wherever
they live. A state-equality test the change adds counts towards the
figure of the target tests whose state it checks.

- [ ] **Comparable records.** Given the before and after records, when
      `mise run measure:compare-builds -- <before> <after> --task
      test:unit:cli` runs, then its first line reads `conditions match`.
- [ ] **`guarded_resume` timing.** Given the comparable records, when
      `guarded_resume`'s durations are read, then its median total
      test-seconds across the three runs is at most half the before figure,
      and the largest of its tests' durations is under 20 s.
- [ ] **Tail-test timing.** Given the comparable records, when each tail
      test's duration is read, then each `rekey_run` tail test is at most
      half its before duration. The `matrix_private_build` test's median
      alone duration is at most half its median alone duration alongside
      the before record, and its after duration is no longer than its
      before duration.
- [ ] **Split tests.** Given a target test that the change splits into
      successor tests, when its after duration is read, then it is the
      median of each run's longest successor. For a tail test, the median across the
      three runs of the successors' summed durations is also no longer than
      the largest of the test's three before durations.
- [ ] **Merged tests.** Given several target tests that the change merges
      into one test, when its after duration is read, then for tail tests it
      is at most half the sum of their before durations, and for
      `guarded_resume` tests it counts as one test under the 20 s ceiling.
- [ ] **Own work and contention.** Given the baseline record, any
      rebaseline record and the after record, when each binary runs alone
      three times
      (`mise run test:unit:cli --filter 'binary(<name>)'`), then each target
      test's median alone duration, read from the `PASS` lines of each
      run's output, and its duration in the matching record are recorded in
      the implementation plan.
- [ ] **Coverage.** Given the change, when the suite runs, then each item
      of the coverage baseline in Technical Notes still runs and passes with
      the same assertions, checked by code review against the
      **test mapping**:
      - every `guarded_resume` scenario, and every pairing of scenario and
        colocation;
      - every VCS and halt point in the two `rekey_run` tests;
      - the `matrix_private_build` test's comparison of a private build
        with a shared one.
- [ ] **Test mapping.** Given `cargo nextest list` output for the three
      binaries at the before and after records' commits, when each test in
      the before list is looked up in the implementation plan, then the
      plan maps it, and each of its assertions, to the successor tests that
      cover it, every successor appears in the after list, and each
      `rekey_run` test's halt points are derived from the mutations of a
      whole run over the corpus that test runs, rather than fixed or
      sampled. A test that loops over colocations, VCSs or halt points
      asserts the number of iterations it completed against
      `COLOCATIONS.len()`, its VCS list or that mutation count.
- [ ] **Reused state.** Given a target test whose starting state the change
      reuses or builds with fewer steps, when the suite runs, then a test
      asserts that the state matches one built from scratch, under each
      colocation and VCS the target test uses it with. The comparison
      covers at least the commit graph's shape and descriptions, each
      commit's file tree, bookmarks and refs, the working copy's parents and
      contents, the colocation mode, and the migration's stall state on
      disk. It excludes change IDs, commit hashes, operation IDs and
      timestamps. The plan may add to this list, but not remove from it.
- [ ] **Independent landing.** Given each binary's change landed in its own
      commit, when `mise run test:unit:cli` runs at that commit, then it
      passes, and the plan records the three landing commits.
- [ ] **Phase guard.** Given the comparable records, when
      `mise run measure:compare-builds -- <before> <after> --task
      test:unit:cli` runs, then the nextest-phase median is either no
      longer than before or, as `compare-builds` reports it, within noise:
      inside the before record's [min, max].
- [ ] **Recorded figures.** Given the before and after records, when the
      item closes, then the implementation plan records both run IDs, each
      figure the timing criteria compute beside its threshold, any setup
      cost added to a figure, and, for each after run, the tests at the last
      three completion positions with the phase's end time.

## Open Questions

- How much of each target test's time is its own work, and how much is
  contention? Requirement 1 answers it from the baseline record before
  planning commits to an approach.

## Dependencies

- Parent: PP-953 (0361), whose closure, and through it PP-961's merge,
  waits on this item being done or abandoned.
- Blocked by: none. PP-957 (0365) is done, and its closure measurement
  (2026-11-20) showed `guarded_resume` still the slowest binary.
- Possibly blocked by: the author, who reviews a target before planning if
  its median alone figure is under half its under-suite figure (see
  Assumptions).
- Blocks: none.
- Related: PP-957 (0365).
- Measurement tooling: `measure:build` and `measure:compare-builds`, from
  PP-960 (done), as the criteria invoke them.
- Concurrent work: PP-958's trial of `debug = "line-tables-only"` for
  dependencies in the dev profile changes the test binaries, so it can
  change `test:unit:cli` runtime and would call for a rebaseline record.
  In turn, a builder or test binary this item adds changes the compile
  units PP-958 counts. PP-958 takes its before record below its own first
  commit, on the same base as its after record, so this item's merge falls
  on both sides and neither item constrains the other's merge.
  PP-959's rebuild work does not reach a warm no-op record, which measures
  with every input already built. No open PP-953 sibling changes the shared
  helpers or `shared_build`.
- Fixture infrastructure: a shared starting state extends PP-957's build
  step in `test:unit:cli`, its copy helper and its fixture log, not a
  nextest setup script, which PP-957 rejected as experimental. No fixture
  output line may match nextest's `Summary [` line, which `measure:build`
  reads, and PP-957's own criteria keep passing.
- Behaviour relied on: `jj` (pinned in `mise.toml`), `git` (unpinned, from
  the Xcode command-line tools or the runner image) and nextest's
  `PASS [ Ns ]` line format. A change to any of them during the item calls
  for a rebaseline record and a fresh check of the **reused state**
  comparison.
- Measurement environment: `engywook` with `devtools` host settings only
  (the Developer Tools entries added, per PP-962). Recording must not
  overlap any other measurement session on `engywook`, including PP-962's.
  If PP-962 has left other settings applied, restore `devtools`-only
  settings before recording, or take a rebaseline record under the current
  settings.

## Assumptions

- The reference figures come from the reference record, after PP-957. The
  criteria's figures come from this item's own records, not from that one.
- The halving and 20 s targets assume a test's own work is at least half
  its time under the suite. A target is reviewed with the author before
  planning if its median alone figure is under half its figure in the
  baseline record, compared:
  - per tail test, for its halving target;
  - by `guarded_resume`'s total test-seconds, for the binary's halving
    target;
  - by its slowest test's durations, for the 20 s ceiling.

  An agreed revision is written into the **`guarded_resume` timing** or
  **tail-test timing** criterion, with a Drafting Note giving the reason.
  If a rebaseline record is taken, the rule is checked again against it,
  and a target whose outcome changes is reviewed again.
- Both records come from the same host with the same host settings. The
  reference records came from `engywook` (Apple M4 Max) with `devtools`
  settings, so the figures cited here compare only with records from that
  host.

## Technical Notes

- `guarded_resume`: `cli/migrate-cli/tests/guarded_resume.rs`. The
  colocation loop is `COLOCATIONS` at `:125`. The stalled-state builder
  ends around `:150-180`. Shared helpers are in
  `cli/migrate-cli/tests/common/mod.rs`, used by 6 of `migrate-cli`'s test
  binaries.
- `matrix_private_build`:
  `cli/vcs-test-support/tests/matrix_private_build.rs:49`, the binary's
  only test.
- `rekey_run`: `cli/work-adapters/tests/rekey_run.rs:928` and `:949`. Both
  loop over `mutations_of_a_whole_run()`, building a fresh repository with
  `committed` on each pass. Shared helpers are in
  `cli/work-adapters/tests/support/mod.rs`, used across `work-adapters`'
  tests.
- `shared_build` lives in `vcs-test-support`, which the tests of 12 other
  `cli/` crates depend on.
- Changes to the shared helpers or to `shared_build` are in scope as far as
  a chosen approach needs them; a broader redesign is a follow-up item.
  Their other consumers must still pass, and `matrix_private_build` must
  still compare a private build with a shared one.
- Coverage baseline, at the time of drafting. A scenario run is a scenario
  under one colocation, or its single run for the three without one.
  - `guarded_resume`: 13 scenarios; 10 loop over both colocations and 3 run
    once, 23 scenario runs in all.
  - `rekey_run` `a_run_killed_at_every_write_resumes_over_its_own_writes`:
    every halt point in `0..mutations_of_a_whole_run()`, under Git and jj.
  - `rekey_run`
    `a_resume_refuses_an_edit_since_the_halt_to_a_path_not_yet_written`:
    every halt point in `0..mutations_of_a_whole_run()`, under Git. After
    the change (Requirement 6): every halt point of a whole run over its own
    five-file corpus, one more than today.
  - `matrix_private_build`: the private matrix's shapes equal
    `shared_build`'s, and exactly one `private-build` event is logged for
    the `matrix` fixtures.
- The per-test durations are the `PASS [ Ns ]` lines in each
  `run-N-test_unit_cli.log` under the record's
  `.accelerator-measure/build/<run-id>/` directory.

## Drafting Notes

- Raised as a child of PP-953 (0361), split out while refining PP-957
  (0365), where `guarded_resume` showed up as the next-slowest binary.
- The `guarded_resume` targets (half the test-seconds, slowest test under
  20 s) were agreed in conversation. They are not projections from a
  measurement.
- The three tail tests were brought into scope during enrichment, after the
  post-PP-957 record showed them finishing last in the phase. The title
  changed to match.
- The tail tests' target, half the before duration, mirrors the
  `guarded_resume` target and was agreed in conversation.
- The nextest-phase criterion stays a guard against regression rather than
  a required reduction, although shortening the tail tests should now
  shorten the phase; the recorded last finishers show whether it did.
- The open question, whether to abandon the item if PP-957 removed most of
  the cost, was answered by the reference record: PP-957 cut about 30% and
  `guarded_resume` stayed the slowest binary.
- Research
  (`meta/research/codebase/2026-11-23-PP-943-shorten-the-slowest-tests-in-the-cli-unit-suite.md`)
  found the edit-refusal test's last halt point never runs. The author chose
  on 2026-11-23 to fix it here (Requirement 6). The fix adds one pass to a
  tail test, so that test's after figure covers one more halt point than its
  before figure. The halving target stays as it is, and the plan records the
  extra pass beside the figure.
- The baseline record
  (`build_2026-11-23T140904Z_engywook_warm-no-op_devtools_isolated`, at
  `f253d980e9`) and its alone runs put four targets under the half-ratio
  line: the `guarded_resume` slowest test (13.6 s alone, 28.2 s under the
  suite), `matrix_private_build` (7.5 s, 33.2 s), and the `rekey_run` kill
  (11.6 s, 23.3 s) and edit-refusal (6.6 s, 17.9 s) tests. The author
  reviewed them on 2026-11-23. The 20 s ceiling and both `rekey_run`
  halving targets stay: the chosen approaches remove most of those tests'
  `git` and `jj` spawns, which also cuts the contention they suffer.
  `matrix_private_build`'s target moved to its alone duration, because
  contention is 77% of its time under the suite and the change controls
  only its own work; its under-suite duration must not grow.
- The three binaries stay in one item: they share a measurement method and
  one before/after record pair. A delay in one binary delays the item's
  closure.

## References

- Reference record: `meta/measurements/build_2026-11-20T130237Z_engywook_warm-no-op_devtools_isolated.json`
- Pre-PP-957 baseline: `meta/measurements/build_2026-11-18T123358Z_engywook_warm-no-op_devtools_isolated.json`
- Per-test logs: `run-N-test_unit_cli.log` under
  `.accelerator-measure/build/<run-id>/`, gitignored and kept only on the
  measuring host
- Parent: `meta/work/PP-953-speed-up-the-full-build-without-losing-confidence.md`
- Related: `meta/work/PP-957-build-the-vcs-test-fixtures-once-per-test-run.md`
- Concurrent and tooling items: `meta/work/PP-958-cut-rust-compile-cost.md`,
  `meta/work/PP-959-stop-rebuilding-unchanged-inputs.md`,
  `meta/work/PP-960-build-performance-measurement-and-host-guidance.md`,
  `meta/work/PP-962-measure-and-cite-the-macos-host-settings-effect-on-the-build.md`
