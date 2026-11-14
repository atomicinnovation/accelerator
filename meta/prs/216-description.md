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
revision: "405c2a0d6584459591f439307e8494e5a1149dc8"
repository: "accelerator"
last_updated: "2026-11-14T14:42:02+00:00"
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

This PR implements Phases 1 and 2 of the 0363 plan:

- The rotation tests now reach the cap in a few large writes and assert the
  real production values. Their CPU time drops from about 31 CPU-s to
  under 0.2.
- The coverage lane clears only the previous run's profile data, keeps its
  instrumented build, and prints a single totals line. Its warm median
  drops from 268.63 s to 174.39 s.

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

### Coverage lane (Phase 2)

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
- `tests/unit/tasks/test_rust.py`:
  - `TestTestUnitCli` now checks the whole call sequence:
    - command order and flags;
    - that only the report's output is captured;
    - that each failing step stops before the next;
    - that the docstring names the stale-binary remedy.
  - New `TestVisualiserHasNoDependents` fails if any workspace member
    depends on `accelerator-visualiser`, by any kind of dependency.
    `cargo llvm-cov report` can't exclude a package, so a dependent would
    pull the visualiser into the totals.
- The `mise.toml` description, `tasks/README.md` and the
  `coverage_enabled` docstring are updated to match.

### Planning documents

- The 0363 work item, research, plan and their reviews.
- Notes in 0361 (A4 shipped separately) and 0365 (take timings only after
  0363 lands).
- The two-line "Mark 0362 as done" status change, which hadn't reached
  `main` yet.

## Context

- Work item: `meta/work/0363-remove-repeated-work-from-the-rust-test-lanes.md`
- Plan: `meta/plans/2026-11-13-0363-remove-repeated-work-from-the-rust-test-lanes.md`
- Research: `meta/research/codebase/2026-11-13-0363-remove-repeated-work-from-the-rust-test-lanes.md`

The plan has two more phases, which this PR does not implement:

- **Phase 3:** sweep superseded instrumented artefacts, to bound disk
  growth.
- **Phase 4:** a coverage floor.

## Testing

All timings were taken on engywook with the host otherwise busy
(1-minute load averages of 12 to 54).

- [x] Unit tests pass: `uv run pytest tests/unit/tasks/test_coverage.py
      tests/unit/tasks/test_rust.py tests/unit/tasks/test_cli_permission_skip.py
      tests/unit/tasks/test_nextest_filter.py` (71 passed)
- [x] `mise run build-system:check`, `mise run server:check` and
      `mise run cli:check` pass
- [x] The full `mise run` passes. It prints one `coverage:` line, with no
      per-file rows and no JSON.
- [x] `log::tests`, user + sys CPU, median of three:
      `cargo test` 30.59 → 0.14 CPU-s, `cargo nextest run` 35.86 → 0.39
      CPU-s. Each of the four mutations fails the new tests.
- [x] `test:unit:cli`, warm, median of three: 268.63 s → 174.39 s (target
      under 180 s), with 7,401 tests passing before and after. Each step
      timed on its own: clean 1.30 s (10,286 leftover profiles), nextest
      169.45 s, report 4.53 s, with no mismatched-data or missing-object
      warnings.
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
- [x] Stale binaries: after a `smallvec` 1.15.2 → 1.15.1 bump, the totals
      are unchanged, both on their own and after a full clean. The report
      took a median of 4.27 s with the superseded binaries present and
      4.06 s without them.
- [x] `ACCELERATOR_COVERAGE=off` runs no `cargo llvm-cov` command and
      passes all 7,401 tests.
- [x] CI: `test-unit` on both `ubuntu-latest` and `macos-latest` runs the
      three steps, prints the totals line, and its report shows no
      mismatched-data or missing-object warnings:
      - macOS (`macos-26-arm64`): 7,401 tests,
        `coverage: lines 94.19%, functions 93.57%, regions 92.88%`.
      - Ubuntu (`ubuntu-26.04`): 7,400 tests,
        `coverage: lines 94.18%, functions 93.57%, regions 92.87%`. Its
        first attempt failed on the timing-sensitive
        `arxiv_pacing a_killed_calls_request_still_spaces_the_next` and
        passed on a rerun.

## Notes for Reviewers

- **The plan had two flag combinations cargo-llvm-cov 0.8.7 rejects.**
  Both are recorded in the plan and work item:
  - `--no-clean` beside `--no-report` is refused. `--no-report` on its
    own already skips the full clean, so the nextest step passes it alone,
    and a test forbids `--no-clean` there.
  - `report` rejects every feature flag, even though its `--help` lists
    them. It reads the objects already built, so it needs none, and a test
    stops anyone adding one.
- **Disk growth is the known cost.** Keeping the instrumented build leaves
  superseded artefacts behind after a dependency or toolchain change. The
  `smallvec` bump grew `cli/target/llvm-cov-target` from 11G to 18G, and a
  full clean brought it back to 12G. Phase 3 of the plan sweeps them on
  every run, for about 1 s of unit listing plus the deletion.
- **Totals can move by 0.01 between identical runs** (one of three runs
  read regions 92.89% instead of 92.88%). This matters for Phase 4's
  floor, which leaves a full percentage point of headroom.
- **The coverage fixture's paths are neutralised.** The fixture
  (`tests/unit/tasks/fixtures/coverage/llvm-cov-summary.json`) uses
  `/workspace` in place of the checkout path, so no machine path is
  committed.
- **The Ubuntu E2E job fails for reasons outside this PR.** `ubuntu-latest`
  now runs Ubuntu 26.04, and `deps:install:playwright` fails there with
  "Playwright does not support chromium on ubuntu26.04-x64". It passed on
  `main` on 24.04.
- **Report scope rests on the dependency guard.** The report has no
  package filter, so it covers today's files only because nothing depends
  on the visualiser. `TestVisualiserHasNoDependents` keeps that true.

https://claude.ai/code/session_01HEhrwWSAMeTH1BGPYLkEZP
