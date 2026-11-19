---
type: "pr-description"
id: "247"
title: "[0365] Build the VCS fixture matrix once per test:unit:cli run"
date: "2026-11-19T11:20:52+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "PP-957"
parent: "work-item:PP-957"
relates_to: ["work-item:PP-953", "work-item:PP-943"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/247"
pr_number: 247
tags: ["build-system", "testing", "rust", "vcs", "nextest", "mise"]
revision: "921e195a5501da357a33164dc29402f662d6e51a"
repository: "accelerator"
last_updated: "2026-11-19T12:45:11+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0365] Build the VCS fixture matrix once per test:unit:cli run

## Summary

`test:unit:cli` built the VCS fixture matrix 19 times per run, once in
every test that needed it. This PR builds it once, in a step of the invoke
task that runs before nextest's test phase. It hands the tree to the tests,
which adopt it in place, and records every build and adoption in a per-run
fixture log. It also gives `test:unit:cli` a `--filter` option that builds
only the fixtures the selected tests use. This is the matrix increment of
0365, Phases 1–3 of its plan. The status-log set follows in a later PR.

## Changes

### Test support: the workbench and the fixture log (`cli/vcs-test-support`)

- **`Workbench`** is how a test gets a shared fixture.
  `Workbench::for_this_test()` opens the test's own directory and hermetic
  root, then `workbench.matrix()` adopts the tree the run handed over. With
  nothing handed over, as under a plain `cargo nextest run`, it builds the
  matrix privately inside the workbench. A handed tree that is missing or
  cannot be adopted fails closed, naming the variable and the path, rather
  than falling back to a private build.
- **`Handover`** reads `ACCELERATOR_SHARED_MATRIX`,
  `ACCELERATOR_SHARED_WORKBENCHES` and `ACCELERATOR_FIXTURE_LOG`,
  canonicalising each path so a tree is adopted and logged under one
  spelling.
- **`FixtureLog`** appends one JSON line per `shared-build`, `adoption` or
  `private-build` event. Each event names its test from `NEXTEST_BINARY_ID`
  and `NEXTEST_TEST_NAME`.
- **`SharedFamily`** abstracts a family that can be adopted or built
  privately. Its `build_privately` takes a token only this crate can make,
  so no test can build a shared family except through the workbench, which
  logs it. The matrix builders are now `pub(crate)`.
- **The `Manifest`** of the matrix's rows is extracted into its own value.
  `Matrix::adopt` now fails on a missing, malformed or empty manifest.
- **`SharedBuildRequest`** builds each requested family on its own scoped
  thread and logs the `git` and `jj` versions it built with. A panic
  becomes a per-family failure.
- **`shared-fixtures`** is a new `[[bin]]` under `tests/fixtures/`, as
  `vcs-adapters`' fixture binaries already are. It is compiled by the same
  build as the tests, so it adds no build configuration.
- **Contract files** (`contract/fixture-log.jsonl` and
  `contract/variables.json`) hold the log format and the variable names.
  The Rust and Python suites both test against them, so neither side can
  rename a field or a variable alone.

### The consumers (`cli/vcs-adapters/tests`, `vcs-test-support/tests/matrix.rs`)

- The 19 matrix tests change only how they get the matrix: two lines
  opening the workbench and its matrix.
- `scrub` writes `poison.gitconfig` into its own workbench rather than
  into the matrix, which is now shared and must not change.
- No test is renamed, added or removed, and no assertion changes.

### The build step (`tasks/test/cli.py`, `tasks/shared/shared_fixtures/`)

- Before nextest, the task compiles the workspace tests, finds the
  `shared-fixtures` bin in that build's unit listing and runs it once.
  Under coverage, the bin runs instrumented and its profile data is written
  where the coverage report reads, so the builder code still counts
  towards the floor.
- **`runs.py`** gives each run a locked directory under
  `<temp>/accelerator-shared-fixtures-<uid>/`, mode `0700`. The root is
  refused if it is a symlink, foreign-owned, writable by others or inside a
  checkout. Each run first sweeps the directories of runs whose `flock`
  nobody holds, so a killed run's leftovers go on the next run.
- **`session.py`** owns a run's lifecycle: build, digest each tree, hand
  over, then, pass or fail, digest again, remove the run's directory, prune
  old logs and print the fixture summary. An interrupt only cleans up and
  skips judging, because tests killed mid-write leave misleading trees and
  log lines.
- **`log.py`** reads, summarises and judges the log. The run fails when a
  shared tree changed, when a test built a shared family privately, or
  when the log disagrees with the consumer list in `consumers.py`.
- **`hashing.py`** gains `tree_digest`, which hashes content and mode but
  not timestamps, and never opens a FIFO or device.
- **`locking.py`** extracts `workspace_lock`'s `flock` attempt into a
  primitive the run lock and the sweep share.
- The task passes `replace_env=True`, so an inherited hand-over variable
  never reaches the builder or nextest. It fails if nextest recompiles
  after the build step.

### `--filter` (`mise.toml`, `tasks/test/cli.py`, `tasks/shared/selection.py`, `tasks/shared/nextest_listing.py`)

- `mise run test:unit:cli --filter '<filterset>'` is declared in mise's
  `usage` and forwarded with `${usage_filter+…}`. mise leaves the variable
  unset when the flag is absent, so `--filter ''` reaches the task and is
  refused rather than meaning everything.
- A filtered run lists the selected tests with `cargo nextest list` first,
  then builds only the families whose consumer binaries have a selected
  test. Under coverage it skips the report, sweep and floor, and throws
  away the listing's own profile data.
- A malformed filter shows nextest's own parse error. A filter that selects
  no test, an undeclared word and an empty filter all fail before anything
  is built.
- `Selection` renders a filter for each runner, quoted once, so a value
  holding a space, `&` or `'` reaches nextest intact. The cargo, pytest
  and node lanes will reuse it.

### Docs and records (`tasks/README.md`, `meta/`)

- `tasks/README.md` gains a "Shared VCS fixtures" subsection and a "Task
  options" convention for declaring and forwarding options.
- The 0365 (now PP-957) work item is renamed from "fixture matrix" to
  "test fixtures", because it now covers both families. It is refined and
  reviewed, and the research, the plan with its review, and the outcomes
  of each manual check are recorded.
- 0391 (now PP-943) is raised for the slow guarded-resume tests, blocked
  by 0365.
- After rebasing onto main's re-key of work items onto Linear
  identifiers, one commit moves this branch's items and documents onto
  theirs: 0365 to PP-957 and 0391 to PP-943, with the 0365 plan, research
  and reviews renamed and their references re-pointed.
- Two isolated `test:unit:cli` measurements are recorded: the baseline
  and an interim record after this increment.

## Context

- Work item: `meta/work/PP-957-build-the-vcs-test-fixtures-once-per-test-run.md`
- Plan: `meta/plans/2026-11-18-PP-957-build-the-vcs-test-fixtures-once-per-test-run.md`
- Research: `meta/research/codebase/2026-11-18-PP-957-build-the-vcs-test-fixtures-once-per-test-run.md`
- Parent: PP-953 (0361), speeding up the full build without losing
  confidence.

## Testing

- [x] The bare `mise run` passes on engywook: 7,770 tests. The fixture
  summary shows one matrix shared build, 19 adoptions and no private
  build. Coverage is unchanged: lines 94.44%, functions 93.92%, regions
  93.18%.
- [x] The task and domain tests pass: 5,027 in `tests/unit/tasks`. Each
  piece was driven test-first.
- [x] The log shows 19 distinct hermetic roots, none inside the matrix
  tree, and matching digests before and after the run.
- [x] A test edited to write into the matrix fails the run, naming the
  tree and the added file.
- [x] With `ACCELERATOR_SHARED_FIXTURES_FAIL=matrix`, the run stops before
  nextest's test phase with one failure naming the matrix, and still
  prints the summary.
- [x] After a passing run, a failing test and a failed build, nothing is
  left under the shared root, and only nextest's own line matches
  `Summary [`.
- [x] SIGKILL: run A was held, then killed once run B was also held. Run C
  removed A's directory and kept B's, and B passed once released.
- [x] Each matrix test binary run directly with `cargo nextest run`
  passes, creating no shared directory and no log.
- [x] `--filter 'binary_id(vcs-adapters::classify)'` builds only the
  matrix, with coverage on and off. A filter selecting no consumer builds
  nothing. A malformed filter, `--bogus`, `extra` and a filter selecting
  no tests each fail before any fixture. `--help` lists `--filter`.
- [x] A filter holding a space, `&` and `'` reaches nextest intact on
  macOS, and under dash, run by forcing mise's inline shell to
  `/bin/dash` as Ubuntu's `sh` would.
- [x] CI's `ubuntu-latest` and `macos-latest` `test-unit` legs pass
  (7,770 and 7,771 tests), each summary showing one matrix shared build,
  19 adoptions and no private build.

## Notes for Reviewers

- **Speed so far.** Against the 18 November isolated baseline, the
  nextest-phase median fell from 181.9 s to 162.3 s (−19.6 s, beyond
  noise). The task-duration median fell from 192.8 s to 183.4 s (−9.3 s,
  within noise). The build step takes about 5 s. The record is noisy:
  idle load was 47% and the IDE used twice its baseline CPU. The 25 s
  target is judged after the status-log increment.
- **A broken builder now stops the whole lane** before any test runs,
  where before it failed only the tests that built that family. This is
  deliberate: fixture builds are deterministic, and the error names the
  family and its cause. To see the other tests while diagnosing, run
  `cargo nextest run` directly. `tasks/README.md` records this.
- **Consumers are listed per binary, not per test**, in
  `tasks/shared/shared_fixtures/consumers.py`. So a filtered run selecting
  only a consumer binary's fixture-free tests still builds the matrix. In
  exchange, renaming or adding a Rust test never touches the list. A
  missing consumer fails every unfiltered run, CI included, naming the
  binary.
- **Bypasses the type system cannot stop.** A `Workbench::with` carrying
  a test-local log could build privately without trace.
  `tests/unit/tasks/test_fixture_builds.py` allows it only in
  `vcs-test-support/tests/workbench.rs`. Plain `cargo test` is ruled out
  by guidance only.
- **Roll-ups ignore `--filter`.** `mise run test:unit --filter x` runs the
  whole roll-up unfiltered, even with an empty `usage`. "Task options" in
  `tasks/README.md` records this, and Phase 9 decides what to do about it.
- **A work-item criterion needs rewording.** It names
  `--filter 'test(classify)'`. That matches test names in other binaries
  and selects no matrix consumer, so it builds nothing. The check was run
  with `binary_id(vcs-adapters::classify)`, and the discrepancy is noted
  under the criterion.
- **A nextest bump** must recapture
  `tests/unit/tasks/fixtures/shared-fixtures/nextest-list.json`. A test
  compares its recorded version with the pin in `mise.toml`.
- **The two fault-injection switches**, `ACCELERATOR_SHARED_FIXTURES_FAIL`
  and `ACCELERATOR_SHARED_FIXTURES_HOLD`, exist for checking the build
  step by hand. The hold is refused when `CI` is set.
