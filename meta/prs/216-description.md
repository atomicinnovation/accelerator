---
type: "pr-description"
id: "216"
title: "[0363] Stop repeating work in the Rust test lanes"
date: "2026-11-14T14:00:45+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0363"
parent: "work-item:0363"
relates_to: ["plan:2026-11-13-0363-remove-repeated-work-from-the-rust-test-lanes"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/216"
pr_number: 216
tags: ["build-system", "testing", "rust", "coverage"]
revision: "5b2801353f6867af5e907a76c436ad1efc76d160"
repository: "accelerator"
last_updated: "2026-11-14T22:51:48+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0363] Stop repeating work in the Rust test lanes

## Summary

Two Rust test lanes were redoing work on every run.

- **Visualiser log-rotation tests:** they reached the 5 MiB rotation cap
  through millions of 2-byte writes.
- **Coverage lane (`test:unit:cli`):** cargo-llvm-cov cleaned and rebuilt
  every workspace crate before each run, then printed a 523-row per-file
  table.

This PR implements all four phases of the 0363 plan:

1. The rotation tests reach the cap in a few large writes and assert the
   real production values. Their CPU time drops from about 31 CPU-s to
   under 0.2.
2. The coverage lane clears only the previous run's profile data, keeps its
   instrumented build, and prints a single totals line.
3. After the report, the lane deletes instrumented artefacts the current
   build no longer uses, so keeping the build no longer grows the disk.
   With Phases 2 and 3, the lane's warm median drops from 268.63 s to
   156.48 s.
4. The lane fails when line, function or region coverage falls below a
   floor: 92%, 92% and 91% to start.

Phases 3 and 4 go beyond the work item's original scope, at the user's
request. Phase 4 reverses the "No coverage gating" decision of the 0162
plan.

## Changes

### Visualiser `log::tests` (Phase 1)

- `cli/visualiser/server/src/log.rs`: the three slow rotation tests now
  write buffers of 5 MiB + 64 KiB. file-rotate splits each one exactly at
  the cap, so one write gives exactly one rotation.
- They assert literal values, not the `MAX_BYTES` and `MAX_FILES`
  constants:
  - rotated segments are exactly 5,242,880 bytes;
  - the active segment is 64 KiB;
  - five rotations keep exactly three segments;
  - every segment is `0o600`.
- The old tests let all four mutations through (`MAX_BYTES` halved or
  doubled, `MAX_FILES` set to 2 or 4), because their writes scaled with
  the constants. The new tests catch each one. Production code is
  unchanged.

### Coverage lane steps (Phase 2)

- `tasks/test/cli.py`: the single `cargo llvm-cov nextest --summary-only`
  becomes three steps:
  1. `cargo llvm-cov clean --profraw-only`
  2. `cargo llvm-cov nextest … --no-report`
  3. `cargo llvm-cov report --json --summary-only`, captured and printed
     as one line, such as
     `coverage: lines 94.19%, functions 93.57%, regions 92.88%`

  A failure in any step, or a report with no usable totals, fails the
  task. `ACCELERATOR_COVERAGE=off` still runs plain `cargo nextest run`.
- `tasks/shared/coverage.py` (new): a pure parser that turns the export's
  `data[0].totals` into a `CoverageTotals` value, already rounded to two
  decimal places. It raises `CoverageReportError` on text that isn't JSON
  or carries no totals. Its tests use a trimmed real 0.8.7 export as the
  fixture.
- `tests/unit/tasks/test_rust.py`: `TestTestUnitCli` checks the whole call
  sequence (command order and flags, which output is captured, that each
  failing step stops the next). The new `TestVisualiserHasNoDependents`
  fails if any workspace member depends on `accelerator-visualiser`, by any
  kind of dependency. `cargo llvm-cov report` can't exclude a package, so
  a dependent would pull the visualiser into the totals.

### Sweep of superseded instrumented artefacts (Phase 3)

- `tasks/shared/instrumented_build.py` (new) decides which artefacts in
  `cli/target/llvm-cov-target/debug` the current build still uses:
  - `UnitListing` parses `cargo test --no-run --message-format=json` into
    the unit hashes the build uses, the hash-free binaries cargo uplifts,
    each crate's unit count, and how many units the listing rebuilt.
  - `live_units` recovers each uplifted binary's hash from the
    byte-identical `<crate>-<hash>` copy beside it in `deps`. cargo reports
    a binary only by its uplifted copy, so without this the sweep deleted
    every live binary and the next run relinked them.
  - `superseded_entries` picks the entries in `deps`, `build` and
    `.fingerprint` whose unit hash is not live.
  - `superseded_incremental` keeps each crate's newest `incremental`
    sessions, as many as it has units. Those directories are named for
    rustc's crate id, which cargo never reports, so they can't be matched
    by hash.
- `tasks/test/cli.py`: after the totals line, the lane reads
  `cargo llvm-cov show-env`, lists the units with that environment and
  `CARGO_TARGET_DIR` pointed at the instrumented build, deletes the
  superseded entries, and prints a line such as
  `swept 2195 superseded artefacts (4.34 GB)`. It sweeps nothing and fails
  if the listing rebuilt any unit (its environment no longer matches the
  build), found no units, or found a binary with no matching copy.

### Coverage floor (Phase 4)

- `tasks/shared/coverage.py`: `CoverageFloor` holds a percentage per
  metric. It rejects a missing metric or one outside 0 to 100, and returns
  a `Shortfall` (`lines 69.50% (floor 70%)`) for each total below its
  floor, in metric order.
- `tasks/test/cli.py`: `COVERAGE_FLOOR` is lines 92, functions 92, regions
  91. The lane checks it last, after the totals and sweep lines, and fails
  with a message naming each shortfall and the
  `cargo llvm-cov clean --workspace` remedy. The floor is applied in
  Python, not with `--fail-under-*`, and a test pins that.
- `TestTestUnitCli` sets a zero floor for every test, so raising the
  production floor never breaks them. Only the floor tests override it,
  checking a total below and exactly at its floor.
- `tasks/README.md` gains a "Coverage floor" subsection: what the floor
  gates, how to set it, the rule that floors only go up (except a reset
  after a toolchain, llvm-tools or cargo-llvm-cov bump), and the first
  step when it trips for no reason.

### Descriptions and planning documents

- The `mise.toml` description, `tasks/README.md`, the `test:unit:cli`
  docstring and the `coverage_enabled` docstring match the new lane.
- The 0363 work item, research, plan and their reviews, with each phase's
  recorded outcomes.
- Notes in 0361 (A4 shipped separately) and 0365 (take timings only after
  0363 lands), and the "Mark 0362 as done" status change.

## Context

- Work item: `meta/work/0363-remove-repeated-work-from-the-rust-test-lanes.md`
- Plan: `meta/plans/2026-11-13-0363-remove-repeated-work-from-the-rust-test-lanes.md`
- Research: `meta/research/codebase/2026-11-13-0363-remove-repeated-work-from-the-rust-test-lanes.md`
- Reversed decision: "No coverage gating" in
  `meta/plans/2026-07-02-0162-rust-toolchain-guard-rails.md`

## Testing

Timings were taken on engywook. Phases 1 and 2 were timed with the host
otherwise busy (1-minute load averages of 12 to 54); Phase 3's final lane
timings on a quieter host (8.8 to 12.8).

- [x] Unit tests pass: `uv run pytest tests/unit/tasks/test_coverage.py
      tests/unit/tasks/test_rust.py tests/unit/tasks/test_instrumented_build.py
      tests/unit/tasks/test_cli_permission_skip.py
      tests/unit/tasks/test_nextest_filter.py` (129 passed)
- [x] `mise run build-system:check`, `mise run server:check` and
      `mise run cli:check` pass
- [x] The full `mise run` passes with the floor in force, printing
      `coverage: lines 94.19%, functions 93.57%, regions 92.89%`. Its first
      attempt failed on a race outside this PR (see Notes for Reviewers).
- [x] `log::tests`, user + sys CPU, median of three:
      `cargo test` 30.59 → 0.14 CPU-s, `cargo nextest run` 35.86 → 0.39
      CPU-s. Each of the four mutations fails the new tests.
- [x] `test:unit:cli`, warm, median of three:
      - 268.63 s before, 174.39 s after Phase 2 on the busy host;
      - 156.48 s after Phase 3 on the quieter host, each run compiling
        nothing and sweeping 0. The sweep step alone takes about 2.3 s.
- [x] The totals line equals the line from a run straight after
      `cargo llvm-cov clean --workspace`.
- [x] Leftover profiles are cleared:
      - Restricted to `kernel`, the totals (lines 77.99%, functions
        84.38%, regions 75.59%) are the same with a profile clear and
        after a full clean.
      - The control, with no profile clear after a full run, gives 89.94%,
        96.88% and 86.22%. So leftover profiles would inflate the totals,
        and the profile clear prevents it.
      - After two runs, every profile postdates the second run's start.
- [x] Superseded artefacts are swept:
      - The first run swept exactly the 2,551 entries (2.28 GB) a probe
        found outside the live set, then 380 `incremental` directories
        (3.10 GB). The next run compiled nothing and swept 0.
      - A `smallvec` 1.15.2 → 1.15.1 bump rebuilt 88 crates, swept 26,554
        entries (8.41 GB) and left `llvm-cov-target` at 11G, the same as
        after a full clean. Without the sweep the same bump grew it from
        11G to 18G. The totals were unchanged.
      - After a restricted `-p kernel` run, the next lane swept its 192
        entries (85.19 MB) and compiled nothing.
- [x] The floors come from four runs, each with cargo-llvm-cov 0.8.7. Each
      floor is the minimum less one point, rounded down:

      | Run                                   | Lines  | Functions | Regions |
      |---------------------------------------|--------|-----------|---------|
      | CI `macos-26-arm64`                   | 94.19% | 93.57%    | 92.88%  |
      | CI `ubuntu-26.04` (x86_64)            | 94.18% | 93.57%    | 92.87%  |
      | Local, macOS 26.3 arm64               | 94.19% | 93.57%    | 92.88%  |
      | Root, Ubuntu 26.04.1 x86_64 container | 93.96% | 93.43%    | 92.62%  |
      | **Floor**                             | **92** | **92**    | **91**  |
- [x] Setting the regions floor to 99 for one run fails the lane with
      `coverage below COVERAGE_FLOOR in tasks/test/cli.py: regions 92.88%
      (floor 99%)`, after the totals and sweep lines.
- [x] `ACCELERATOR_COVERAGE=off` runs no `cargo llvm-cov` command and
      passes all 7,401 tests.
- [x] CI before the floor: both `test-unit` legs printed the totals and
      sweep lines and passed, with no mismatched-data or missing-object
      warnings:
      - macOS: 7,401 tests, `swept 2195 superseded artefacts (4.34 GB)`.
      - Ubuntu: 7,400 tests, `swept 1981 superseded artefacts (5.48 GB)`.
- [x] CI with the floor in force: both `test-unit` legs pass, twice, at
      `a5cfcfa0`. macOS reads regions 92.89% then 92.88%; Ubuntu reads
      `coverage: lines 94.18%, functions 93.57%, regions 92.87%` both
      times.

## Notes for Reviewers

- **The plan had two flag combinations cargo-llvm-cov 0.8.7 rejects.**
  Both are recorded in the plan and work item:
  - `--no-clean` beside `--no-report` is refused. `--no-report` on its
    own already skips the full clean, so the nextest step passes it alone,
    and a test forbids `--no-clean` there.
  - `report` rejects every feature flag, even though its `--help` lists
    them. It reads the objects already built, so it needs none, and a test
    stops anyone adding one.
- **The sweep's riskiest judgements are binaries and `incremental`.**
  A binary counts as live only through a byte-identical copy in `deps`,
  which holds whether cargo hard-links or copies (on macOS it copies).
  With no such copy, the lane sweeps nothing and fails rather than delete
  a binary it might need. An `incremental` directory judged wrongly costs
  only one non-incremental rebuild of its crate.
- **The floor's headroom.** Totals can move by 0.01 between identical runs,
  and the root run sits about 0.25 below the others. A full point below
  the lowest of the four runs leaves room for both.
- **Running the tests as root fails 28 of them.**
  `ACCELERATOR_TEST_SKIP_PERMISSION_TESTS` guards only
  `store::test_support`'s sealed directories and files. Other tests seal
  paths by hand, so as root the lane stops before its report. The root run
  took its totals from the lane's steps run by hand with
  `--no-fail-fast` (7,372 passed, 28 failed), and the README's recipe now
  says so. Guarding those tests is follow-up work.
- **A flaky test outside this PR.**
  `design-adapters::spawn_properties
  the_child_s_streams_are_redirected_to_the_bootstrap_log` failed once under
  a full `mise run`. It reads the bootstrap log as soon as it is non-empty,
  before the child's stderr line has always arrived. It passed five times
  alone and in the rerun.
- **Two CI flakes outside this PR.** macOS's `test:unit:frontend` can pass
  every test and still fail on `[vitest-worker]: Timeout calling
  "onTaskUpdate"`; its duration here (185 to 347 s) matches `main`'s (240
  to 321 s). The Ubuntu integration lane's bootstrap test
  `test_concurrent_cold_cache_slow_downloader_all_succeed` once saw 8
  downloads where the lock should allow 2. Both passed on this PR's
  earlier runs, and neither touches its code.
- **The coverage fixtures' paths are neutralised.** Both fixtures under
  `tests/unit/tasks/fixtures/coverage/` use `/workspace` in place of the
  checkout path, so no machine path is committed.
- **The Ubuntu E2E job fails for reasons outside this PR.** `ubuntu-latest`
  now runs Ubuntu 26.04, and `deps:install:playwright` fails there with
  "Playwright does not support chromium on ubuntu26.04-x64". It passed on
  `main` on 24.04.
- **Report scope rests on the dependency guard.** The report has no
  package filter, so it covers today's files only because nothing depends
  on the visualiser. `TestVisualiserHasNoDependents` keeps that true.

https://claude.ai/code/session_01HEhrwWSAMeTH1BGPYLkEZP
