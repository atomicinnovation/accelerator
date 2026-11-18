---
type: "work-item"
id: "0365"
title: "Build the VCS test fixtures once per test run"
date: "2026-10-28T16:18:46+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0361"
blocks: ["work-item:0391"]
relates_to: ["work-item:0264", "work-item:0391"]
source: "codebase-research:2026-10-28-full-build-duration-contention-and-slowness"
tags: ["performance", "testing", "rust", "vcs"]
last_updated: "2026-11-18T13:28:54+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-957"
---

# 0365: Build the VCS test fixtures once per test run

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The VCS tests build two expensive fixture families with real `git` and `jj`
processes: the checkout fixture matrix and the status-log states. Each test
builds its own copy, so one `test:unit:cli` run builds the matrix 19 times
and the full status-log state set 6 times. Those tests are the slowest in
the nextest phase and account for about 870 of its 2,818 test-seconds.
Building each family once per run and sharing it with the tests that only
read it would shorten the phase that dominates the full build.

## Context

`test:unit:cli` is the longest task in a warm no-op `mise run`: a median of
255 s in `build_2026-11-17T070532Z_engywook_warm-no-op_devtools.json`, whose
whole build took a median of 277 s. Isolated, at `287b6279` on `main`, its
nextest phase has a median of 181.9 s (min 164.6, max 197.6) over 7,727
tests and 2,818 test-seconds
(`build_2026-11-18T123358Z_engywook_warm-no-op_devtools_isolated.json`).

The matrix consumers, 19 tests and 730 test-seconds in the second timed run:
- `vcs-adapters::classify`, 6 tests at 45–46 s each;
- `vcs-test-support::matrix`, 6 tests at 37–38 s each;
- `vcs-adapters::scrub`, 2 tests at 39–41 s each;
- `vcs-adapters::queries`, 5 tests at 30–31 s each.

Apart from `accelerator-migrate::guarded_resume`, these are the 17 slowest
tests in the run. One matrix build takes 3.3 s wall and 1.9 CPU-s when run
alone.

The full status-log state set is built by four tests in
`accelerator-vcs::status_log_goldens` (22–23 s each),
`vcs-adapters::conflicts` (29 s) and the `vcs-test-support::status_log_states`
self-test (20 s). Two more golden tests build the smaller plain-git subset.
The cap, bookmark and parity sets are built by four further tests at 1–6 s
each.

Both families already have an adopt path. `Matrix::build_or_adopt` and
`status_log::States::build_or_adopt` adopt a tree built earlier, named by
`ACCELERATOR_ZERO_SPAWN_MATRIX` and `ACCELERATOR_ZERO_SPAWN_STATUS_LOG`, and
build their own when neither is set. The zero-spawn lane that set those
variables was retired by 0264, so nothing sets them today and every build is
private.

The October research ruled out two other explanations. nextest's
process-per-test model is not the overhead: 338 tests took 10.8 s under
nextest and 11.1 s under `cargo test`. And the phase is not CPU-saturated:
CPU ran at about 50% busy when the phase ran alone.

## Requirements

1. Build the VCS fixture matrix once per test run, and have every matrix
   consumer in the audit below adopt it.
2. Build the full status-log state set, as `States::build_or_adopt` builds
   it, once per run. The tests whose states it covers adopt it: the four
   `build_states` golden tests, the two plain-git golden tests, the
   `status_log_states` self-test and, once it is read-only, `conflicts`.
   An adopting test gets a hermetic environment rooted at the shared build.
3. The cap, bookmark and parity sets stay private builds.
4. A test that writes into a shared tree either stops doing so without
   losing an assertion, or keeps a private build. The audit found two:
   - `scrub` writes `poison.gitconfig` into the matrix base;
   - `conflicts` runs `jj resolve --list` without `--ignore-working-copy`,
     so the jj CLI snapshots the working copy and records an operation.
5. A test run on its own, through a filtered nextest run, plain
   `cargo test` or the IDE, builds its own fixtures and passes.
6. A failed shared build is reported once with its cause, not as a failure
   in every adopting test.
7. Shared fixture trees do not accumulate across runs.
8. The variables and types that hand a shared fixture tree to a test are
   named for that purpose. Rename the `ACCELERATOR_ZERO_SPAWN_*` variables
   and `MATRIX_ROOT_VARIABLE` if they keep a role; they name a lane that no
   longer exists.
9. Research and planning choose how the fixtures are built once per run.
   The mechanism also works under the coverage lane's
   `cargo llvm-cov nextest`.

Mutation audit of today's consumers:

| Consumer | Tests | Builds | Effect on the fixtures |
|---|---|---|---|
| `vcs-adapters::classify` | 6 | matrix | read-only, through `InProcessProbe` |
| `vcs-adapters::queries` | 5 | matrix | read-only; one test changes the process's working directory |
| `vcs-test-support::matrix` | 6 | matrix | read-only structural checks |
| `vcs-adapters::scrub` | 2 | matrix | writes `poison.gitconfig` into the matrix base |
| `status_log_goldens` | 6 | full set or plain-git subset | runs the binary; `jj status` takes the working-copy lock and writes store objects but commits no operation |
| `vcs-adapters::conflicts` | 1 | full set | `jj resolve --list` snapshots and records an operation |
| `status_log_states` | 1 | full set plus adversarial state | read-only |
| `status_log_goldens`, `status_log_parity` | 4 | cap, bookmark and parity sets | read-only; out of scope |

## Acceptance Criteria

- [ ] Given a full `test:unit:cli` run, when it completes, then the matrix
      was built once and none of the 19 matrix consumers built its own.
- [ ] Given a full `test:unit:cli` run, when it completes, then the full
      status-log state set was built once, and every test named in
      Requirement 2 adopted it or keeps a private build under
      Requirement 4.
- [ ] Given `scrub` and `conflicts` running against shared fixtures, when
      they finish, then nothing in the shared trees has changed and each
      still makes every assertion it makes today.
- [ ] Given any matrix or status-log consumer run alone with plain
      `cargo test`, when it runs, then it builds its own fixtures and
      passes.
- [ ] Given a completed run, pass or fail, when it ends, then no shared
      fixture tree from that run remains on disk.
- [ ] Given a full `test:unit:cli` run, when it completes, then nextest
      reports at least the baseline's 7,727 tests run.
- [ ] Given the baseline
      `meta/measurements/build_2026-11-18T123358Z_engywook_warm-no-op_devtools_isolated.json`,
      when `mise run measure:build -- --isolate test:unit:cli
      --host-settings devtools --runs 3 --keep` is recorded after the
      change and compared with
      `mise run measure:compare-builds -- <baseline> <after> --task test:unit:cli`,
      then the first line reports no differing condition, and the
      nextest-phase median is at least 25 s shorter (156.9 s or less)
      with a noise verdict of "beyond".

## Open Questions

- Can tests running `jj status` concurrently share one jj state? Its
  snapshot takes the working-copy lock and writes store objects without
  committing an operation. If not, the golden tests that run it keep
  private builds and the saving shrinks.

## Dependencies

- Blocked by: none. The baseline record is
  `meta/measurements/build_2026-11-18T123358Z_engywook_warm-no-op_devtools_isolated.json`.
- Blocks: 0391, which shortens the `guarded_resume` tests, the slowest
  left once this item lands. It takes its before figures after this item,
  since this item reduces the suite's contention.
- Related: 0264, which retired the zero-spawn lane that set the adopt
  variables.

## Assumptions

- A fixture tree built once gives each read-only test the same view a
  private build gives it today.
- gix status and jj-lib's loader write nothing to the fixtures. This was
  checked by reading the code, not by test.

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
  `hermetic-jj.toml` under its base (`cli/vcs-test-support/src/hermetic.rs:46-58`).
- nextest setup scripts sit behind its `experimental` flag.

## Drafting Notes

- Brought the status-log states into scope alongside the matrix, as agreed
  in conversation. The cap, bookmark and parity sets were left out because
  they cost 1–6 s each.
- Set the speed target at 25 s off the isolated nextest-phase median.
  About 870 test-seconds go, but the phase is not CPU-bound and
  `guarded_resume`'s 39 s tests become the slowest. `compare-builds` calls
  a change "beyond" noise only below the baseline's 164.6 s minimum, so
  25 s clears the noise band.
- Dropped the zero-spawn acceptance criterion: the lane no longer exists.
- Replaced the October figures with the 18 November baseline, taken at
  `main` once 0362, 0363 and 0364 had landed.

## References

- Source: `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- Baseline: `meta/measurements/build_2026-11-18T123358Z_engywook_warm-no-op_devtools_isolated.json`
- Related: 0361, 0264, 0391
