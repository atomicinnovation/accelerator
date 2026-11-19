---
type: "pr-description"
id: "249"
title: "[0365] Build the status-log states once per run, copying them for tests that write"
date: "2026-11-19T18:58:43+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "PP-957"
parent: "work-item:PP-957"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/249"
pr_number: 249
tags: ["build-system", "testing", "rust", "vcs"]
revision: "ab4090f307f60b37e0a3a1d29847db9d561b74b7"
repository: "accelerator"
last_updated: "2026-11-19T20:30:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0365] Build the status-log states once per run, copying them for tests that write

## Summary

`test:unit:cli` built the `vcs status`/`vcs log` fixture states six or more
times per run, once in each test that needed them. It now builds them once,
in the build step that already builds the matrix, and hands them to the
tests. Tests that only read the states adopt the shared build. Tests that
change them, as anything that snapshots a jj working copy does, each get
their own copy. Every golden is byte-identical.

This is 0365's status-log increment: Phase 4 (the states record no absolute
path, so a copy reads as a fresh build would) and Phase 5 (sharing and
copying). Phase 6, the closing measurement, follows once this merges. #247,
the matrix increment, has merged, so this PR is now based on `main`.

## Changes

### Phase 4: states that record no build directory

- **A path scan.** `path_scan::files_naming(tree, needle)` lists every
  regular file under a tree whose bytes name a directory, and every symlink
  whose target does, in path order. It matches the needle both as given and
  as its canonical path, so `/var` and `/private/var` on macOS cannot hide
  each other.
- **Path-free status-log builders.** Each place a state recorded its build
  directory now records a relative path, or nothing:
  - `git-ahead` and `git-behind` are cloned with a command-scoped
    `-c core.logAllRefUpdates=false`, then pointed at `../origin.git`;
  - the bare `origin.git` points back at `../seed`;
  - `adversarial-git`'s `core.hooksPath` is `hostile-hooks`;
  - the secondary jj workspace is added as `../jj-secondary`, so jj's
    operation log names no absolute path.
- **The matrix's build split and a manifest type.** `States` builds through
  `build_with`, `build_in` and `build_for_sharing`, as `Matrix` does, and
  its manifest goes through `StatusLogManifest`.
- **A guard test.** `status_log_states::no_state_names_the_directory_it_was_built_in`
  scans every state, plus `seed`, `origin.git` and `jj-secondary-main`, for
  the directory it was built in.

### Phase 5: sharing and copying

- **The status-log set is a shared family.** `Family::StatusLog` joins the
  matrix on both sides of the contract (`--status-log`,
  `ACCELERATOR_SHARED_STATUS_LOG`), and the `shared-fixtures` builder builds
  both families concurrently. A filtered run builds only the families its
  selected tests consume.
- **The copy helper.** `copies::copy_part` copies named top-level entries of
  a shared tree, keeping every file's contents, mode and times to the
  nanosecond, so git's index and jj's working-copy state see nothing
  changed. Each copy is then scanned for the shared tree's path.
- **The workbench hands out three new kinds of fixture:**
  - `status_log_states()` adopts the shared set;
  - `writable_status_log_states(part)` copies `Everything`, `PlainGit` or
    `Conflicts` into the workbench, or builds that part privately when
    nothing is handed over;
  - `private_states(builder)` builds a cap, bookmark or parity set in the
    workbench, so it is logged under "private sets".

  A copy that still names the shared tree is logged with the offending files
  and fails the test.
- **The fixture log records copies.** A `copy` event carries the copy and
  its scan. The summary counts copies, the run fails on any copy whose scan
  found the shared path, and copies count towards the consumer checks.
- **The consumers move.** The six goldens tests and `conflicts` copy the
  states, the `status_log_states` self-test adopts them, and the cap,
  bookmark and parity tests build their sets through `private_states`. The
  full-set builders are `pub(crate)`, so no test can build the shared set
  behind the log's back. `ACCELERATOR_ZERO_SPAWN_STATUS_LOG`, `states_root`
  and `build_or_adopt` are gone.
- **New tests against real states.** `copy_fidelity` checks a copy of every
  state: the same modification times, the same `git status`/`jj status`
  output, and a jj snapshot that sees exactly the builder's own edits.
  `status_log_parts` checks each part's private build holds what the
  copied part holds.
- **Guards.** `tests/unit/tasks/test_fixture_builds.py` now also refuses
  `copy_part(` outside the copy's own tests, and a direct call to a
  private-set builder anywhere in a test. The goldens test asserts that it
  compared exactly the 38 golden files on disk.
- **Python side.** `consumers.py` lists the five status-log consumer
  binaries, and the captured nextest listing grows to cover them.
- **Docs, plan and work item.** `tasks/README.md`'s "Shared VCS fixtures"
  subsection covers copies, the path scan and the private sets. The plan's
  Phase 4 and 5 checks are ticked, with an "as built" note on each phase,
  and the work item records every status-log outcome.

## Context

- Work item: `meta/work/PP-957-build-the-vcs-test-fixtures-once-per-test-run.md`
- Plan: `meta/plans/2026-11-18-PP-957-build-the-vcs-test-fixtures-once-per-test-run.md`,
  Phases 4 and 5
- Builds on #247 (merged), which builds the matrix once per run and hands
  it to the tests.

## Testing

- [x] Each step written test first; the Phase 4 guard test failed before
      the builder changes, naming the files the plan predicted.
- [x] Regenerating the goldens changes nothing, and
      `jj diff --stat cli/vcs-test-support/fixtures` is empty.
- [x] Run directly (nothing handed over, so every test builds privately),
      `cargo nextest run --manifest-path cli/Cargo.toml -p vcs-test-support -p accelerator-vcs -p vcs-adapters`
      passes (340 tests), and writes no fixture log.
- [x] `mise run` passes end to end with coverage on: 8,113 tests in
      `test:unit:cli`, coverage above the floor (lines 93.64%, functions
      93.14%, regions 92.22%). The fixture summary reads:
      - matrix: 1 shared build, 19 adoptions, 0 copies, 0 private builds;
      - status-log set: 1 shared build, 7 adoptions, 10 copies, 0 private
        builds;
      - private sets: 4 private builds.
- [x] Coverage off passes with the same summary.
- [x] The log shows 17 distinct hermetic roots for the status-log set,
      none inside a shared tree, every copy's scan clean, and matching
      before-and-after digests.
- [x] A test made to write into the adopted set fails the run, naming the
      tree and the added file. A shared path planted in
      `conflict-git/.git/config` through the hold switch fails the run,
      naming the copy and the file.
- [x] `--filter 'binary_id(vcs-adapters::conflicts)'` and
      `--filter 'test(conflicts)'` build the status-log set alone and pass;
      `--filter 'binary_id(vcs-adapters::classify)'` builds the matrix
      alone.
- [x] `ACCELERATOR_SHARED_FIXTURES_FAIL=status-log` stops the run before
      any test, with one failure naming the status-log set.
- [x] Passing, failing-test and forced-failure runs leave nothing under the
      shared root, and only nextest's own line matches `Summary [`.
- [x] The SIGKILL check: run C removed killed run A's directory and lock,
      kept held run B's, and B passed once released.
- [x] Phase 4's push: the `ubuntu-latest` unit leg passed.
- [x] Both CI `test-unit` legs pass, each summary showing both families
      built once (8,112 tests on `ubuntu-latest`, 8,113 on `macos-latest`).

## Notes for Reviewers

- **Copies are writable.** The plan asked for copies with exactly their
  source's modes. Since it was written, the build step started sealing each
  shared tree read-only, so an exact copy could not be written. `copy_part`
  keeps every mode and adds the owner's write permission.
- **Why the matrix is never copied.** It records absolute paths throughout,
  and every matrix consumer only reads it, so all of them adopt. Only the
  status-log set has a copy path.
- **A filter can build a set its test doesn't use.** Families are chosen
  per consumer binary, so selecting only `the_log_is_capped_at_five_entries`
  (a private-set test in the goldens binary) still builds the status-log
  set: 2.0 s of a 6.5 s task, recorded in the work item.
- **Smaller departures from the plan,** each noted in its "as built"
  section:
  - the workbench's fixture type is `Held`, not `Fixture`;
  - jj summarises `rename-jj`'s snapshot as `R {old.txt => new.txt}`;
  - `copy_fidelity`'s planted-path test copies rather than adopts, so the
    run shows 7 adoptions, not 8.
- **Still to come.** Phase 6 measures `test:unit:cli` against the
  18 November baseline once this lands; the filters increment follows.
