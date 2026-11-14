---
type: "work-item"
id: "0363"
title: "Remove repeated work from the Rust test lanes"
date: "2026-10-28T16:18:46+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0361"
blocks: ["work-item:0370"]
source: "codebase-research:2026-10-28-full-build-duration-contention-and-slowness"
tags: ["build-system", "performance", "testing", "rust", "coverage"]
last_updated: "2026-11-13T13:38:47+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0363: Remove repeated work from the Rust test lanes

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Two Rust test lanes repeat work on every run:
- `test:unit:cli`, the coverage lane, cleans and recompiles every workspace
  crate before each run.
- The visualiser's `log::tests` rotation tests push millions of two-byte
  writes through the operating-system kernel, in each of the two lib-suite
  runs within `test:unit:visualiser`.

Removing both shortens `test:unit:cli`, the longest task in a local
`default` run, and cuts the user plus sys CPU time of `log::tests` to under
2 CPU-s. No test is dropped, coverage stays measured both locally and
in CI, and its totals stay correct. While touching the coverage command,
the item also makes its ineffective `--summary-only` flag take effect: the
lane deliberately stops printing the 523-row per-file table and prints a
totals line instead.

The two halves are independent and may merge separately.

## Context

The labels A2, A3 and A4 below are rows of the recommendations table in the
source research.

**Coverage lane (A2).** `test:unit:cli` runs `cargo llvm-cov nextest`, and
before every run cargo-llvm-cov does a full clean: `cargo clean --package` for
all 44 workspace members, then removes every `*.profraw` file. The clean
alone takes 61 s on one thread, then the crates recompile instrumented (30 s).
Warm and alone, the task takes 247 s; with `--no-clean` it took 160 s.

Bare `--no-clean` is not safe. It never deletes old profile data, so each
report merges counts from earlier runs. In one experiment, line coverage of
the `kernel` crate read 89.94% with stale files from earlier, different
runs present, against 77.99% after a clean. Running `cargo llvm-cov clean
--profraw-only` and then `--no-clean` gave the same figures as a full clean.

The task also passes `--summary-only`. cargo-llvm-cov only honours that with
`--json`, `--lcov` or `--cobertura`, so a full run prints a 523-row per-file
table.

The team decided that `default` mirrors CI, so instrumentation stays both
locally and in CI.

**Log-rotation tests (A3).** Three tests in `log::tests` push 2-byte lines
past the 5 MiB cap: `rotates_when_active_segment_crosses_5_megabytes`,
`retains_at_most_three_rotated_segments` and
`active_and_rotated_segments_are_owner_only`.
`retains_at_most_three_rotated_segments` takes 22.3 s, of which 20.8 s is
OS-kernel time, from about 13 M `write` syscalls. The research measured the
module at about 42 s of sys time alone each time the lib suite runs, which
makes that suite extremely sensitive to load. `test:unit:visualiser` runs
the lib suite twice, once with `--no-default-features --features
dev-frontend` and once with the default `embed-dist` features. The
research's quick-wins experiment turned coverage off locally
(`ACCELERATOR_COVERAGE=off`) and applied A1, A4, A5 and A6, none of which
touch the visualiser lib suite. In that run the `dev-frontend` lib-suite
run took 129 s and was the last task of `default` to finish.

Rotation is byte-based: `make_writer` configures
`ContentLimit::Bytes(MAX_BYTES)`, and file-rotate splits a single buffer at
the cap, rotating mid-write. A few large writes therefore cross the cap
exactly as millions of small ones do.

A4, the visualiser lib tests running a third time inside
`test:integration:visualiser`, has already been fixed separately by
switching that task to `--test '*'`.

## Requirements

1. Replace the coverage lane's full clean with this sequence, keeping
   instrumentation both locally and in CI:
   1. `cargo llvm-cov clean --profraw-only`
   2. `cargo llvm-cov nextest --no-clean --no-report`
   3. `cargo llvm-cov report --json --summary-only`

   Never use bare `--no-clean`. The `ACCELERATOR_COVERAGE=off` escape hatch
   to plain nextest stays as it is.
2. Make the coverage summary take effect: from the report's `totals`, print
   a single line of the form
   `coverage: lines 77.99%, functions 81.20%, regions 74.05%`, each value
   being `totals.{lines,functions,regions}.percent` rounded to two decimal
   places. Print no per-file rows and no raw JSON.
3. Make the three `log::tests` rotation tests reach the real 5 MiB cap in a
   few large writes instead of 2-byte lines. The tests assert against the
   literal production values (5 MiB and 3 segments), not against
   `MAX_BYTES` and `MAX_FILES` by name, so that a change to either constant
   fails them.
4. State in the `test:unit:cli` task's docstring that `cargo llvm-cov clean
   --workspace` is the remedy when instrumented binaries are suspected
   stale. The docstring need not mention that no periodic full clean is
   scheduled.

## Acceptance Criteria

Timings are taken on the host that recorded 0368's baseline, as the median
of three runs. "Warm" means after one completed `test:unit:cli` run with no
source changes since.

**Coverage lane**

- [ ] Given a warm workspace, when `mise run --skip-deps test:unit:cli` runs
      with no other task running, then it completes in under 180 s. The
      same command is first timed on the unchanged code under the same
      procedure to record the before figure (247 s in the research).
- [ ] Given a warm workspace, when `mise run --skip-deps test:unit:cli`
      runs, then its totals line equals the totals from the same lane run
      made straight after `cargo llvm-cov clean --workspace`.
- [ ] Given a completed full `test:unit:cli` run, when the lane's three
      commands are run by hand with `-p kernel` added to each, then the
      `kernel` totals equal those of the same restricted commands run
      straight after `cargo llvm-cov clean --workspace`. As a control, the
      same restricted run with bare `--no-clean` (skipping the
      profraw-only clean) after a fresh full run reports different
      `kernel` totals, showing the setup can expose stale profiles.
- [ ] Given two consecutive coverage runs, when the second finishes, then
      `cli/target/llvm-cov-target` holds at least one `*.profraw` file, and
      every one has a modification time later than the second run's start.
- [ ] Given `test:unit:cli` before and after the change, when each run
      finishes, then nextest reports the same number of tests run, all
      passing.
- [ ] Given a `--json --summary-only` export with known `totals` values,
      when the task formats it, then the printed line matches the format in
      Requirement 2, with each percentage rounded to two decimal places.
- [ ] Given a full `mise run`, when the coverage lane reports, then it
      prints one totals line with line, function and region percentages,
      no per-file rows and no JSON.
- [ ] Given `ACCELERATOR_COVERAGE=off`, when `test:unit:cli` runs, then it
      runs plain `cargo nextest` with no `cargo llvm-cov` invocation (no
      clean and no report), and the same number of tests pass.
- [ ] Given the CI `test-unit` job on both its `ubuntu-latest` and
      `macos-latest` legs, when it runs `test:unit:cli`, then it still runs
      `cargo llvm-cov` and prints the totals line.
- [ ] Given the `test:unit:cli` task's docstring, when read, then it names
      `cargo llvm-cov clean --workspace` as the remedy for suspected stale
      instrumented binaries.

**Log-rotation tests**

- [ ] Given the lib test binary already built (`cargo nextest run -p
      accelerator-visualiser --lib --no-default-features --features
      dev-frontend --no-run`), when `cargo nextest run -p
      accelerator-visualiser --lib --no-default-features --features
      dev-frontend log::tests` is timed with `/usr/bin/time -l` on an
      otherwise idle host, then user plus sys time is under 2 CPU-s. The
      same command is first run on the unchanged code to record the
      before figure, which replaces the research's sys-only 42 s as the
      baseline.
- [ ] Given the rewritten `log::tests`, when they run, then all three
      rotation tests named in Context still run and pass, and together they
      assert that:
      - each full rotated segment is exactly 5 MiB;
      - after crossing the cap, the active segment is smaller than 5 MiB;
      - after more than three rotations, exactly three rotated segments
        are kept;
      - a single write that crosses the cap triggers rotation;
      - the active segment and every rotated segment have owner-only
        permissions.
- [ ] Given each of the following mutations applied alone, when
      `log::tests` runs, then at least one rotation test fails: `MAX_BYTES`
      halved; `MAX_BYTES` doubled; `MAX_FILES` set to 2; `MAX_FILES` set to
      4.

## Dependencies

- Blocked by: none. This item waited on 0368 recording a warm no-op
  baseline, so that its before and after timings stay comparable with the
  baseline 0361 measures its 200 s target against. That baseline is
  recorded at
  `meta/measurements/build_2026-11-01T143523Z_engywook_warm-no-op_devtools.json`.
- Blocks: 0370, which measures the macOS host settings once this item,
  0362 and 0367 have each landed or been abandoned. For 0370, this item
  counts as landed only when both halves have each landed or been
  explicitly abandoned.
- Verification host: the timing criteria need the host that recorded
  0368's baseline (engywook). Its macOS settings must not be changed, for
  example by 0370's experiments, until this item's timings are taken.
- Tooling: cargo-llvm-cov 0.8.7 and cargo-nextest 0.9.138, both pinned in
  `mise.toml` and used by both local and CI runs. Requirements 1, 2 and 4
  rely on this cargo-llvm-cov version's `clean --profraw-only`,
  `clean --workspace` and `--no-clean` flags, and on the shape of the
  `--json --summary-only` export. Upgrading it means re-checking all three.
- Library: file-rotate 0.7.6 (locked in `cli/Cargo.lock`). Requirement 3
  relies on its byte-based split of a single buffer at the cap. Upgrading
  it means re-checking that the chunked-write tests still cross the cap
  mid-write.
- CI cache: the `Swatinem/rust-cache` step in the `test-unit` job of
  `.github/workflows/main.yml` decides which instrumented binaries CI
  restores, and so bounds CI's residual stale-binary risk. A change to its
  key or scope should be re-checked against this item.
- Measurement ordering with 0365 (B1): 0365 times `test:unit:cli`'s
  nextest phase, which this item's coverage half changes, and targets the
  floor the research projects once this item's visualiser half lands.
  0365's before and after timings should both be taken after both halves
  have landed. 0365 records the same ordering.

## Assumptions

- CI coverage figures stay correct because the profraw-only clean runs
  before every report and deletes any profile data that the CI cache
  restores. Correctness does not depend on what that cache contains.
- Nothing in CI parses the per-file coverage table, so replacing it with a
  totals line loses no consumer.
- The shorter macOS `test-unit` CI job, and `test:unit:visualiser` no
  longer becoming the tail of `default`, are expected side effects, not
  verified outcomes of this item. 0370 measures macOS separately.

## Technical Notes

- Coverage command and the `ACCELERATOR_COVERAGE` switch:
  `tasks/test/cli.py:20-48`.
- Log tests and the cap: `cli/visualiser/server/src/log.rs:19-170`; the
  writer is built at `log.rs:22-43`.
- cargo-llvm-cov warns that `--no-clean` "can cause false positives/false
  negatives due to old build artifacts". The profraw-only clean removes the
  stale-profile case. Stale instrumented binaries remain possible, but cargo
  rebuilds any whose sources change, so the residual risk is left to the
  manual full clean.
- `cargo llvm-cov report --json --summary-only` drops the per-line
  segments but still lists a summary for every file. The totals sit at
  `data[0].totals`, beside those per-file summaries. The task captures the
  export, extracts the line, function and region percentages from
  `data[0].totals` and prints them as the totals line.
- Merging a second copy of the same run's profiles leaves percentages
  unchanged, which is why the coverage-equality criterion seeds a
  different, wider prior run.

## Drafting Notes

- Grouped A2 and A3 because each removes repeated work from a Rust test
  lane, as agreed in conversation. A4 was dropped from scope because it had
  already shipped.
- The A2 approach follows the decision that `default` mirrors CI. The
  research's earlier option of dropping instrumentation locally is
  superseded.
- A3 uses chunked writes rather than an injected smaller cap, so the tests
  keep exercising the production cap without a test-only seam.
- The full-clean cadence question was settled as "manual remedy only",
  because cargo's own rebuild-on-change bounds the stale-binary risk.
- The 180 s and 2 CPU-s targets are proposals derived from measurements of
  160 s with `--no-clean`, and about 42 s of sys time today.

## References

- Source: `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- Baseline: `meta/measurements/build_2026-11-01T143523Z_engywook_warm-no-op_devtools.json`
- Parent: 0361
- Related: 0362, 0365, 0367, 0368, 0370
