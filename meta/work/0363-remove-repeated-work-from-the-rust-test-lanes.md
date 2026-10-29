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
blocked_by: ["work-item:0368"]
source: "codebase-research:2026-10-28-full-build-duration-contention-and-slowness"
tags: ["build-system", "performance", "testing", "rust", "coverage"]
last_updated: "2026-10-29T19:46:28+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0363: Remove repeated work from the Rust test lanes

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The Rust test lanes repeat three kinds of work on every run:
- The coverage lane cleans and recompiles every workspace crate.
- The visualiser lib tests run a third time, as a pure duplicate.
- The visualiser's log-rotation tests push millions of two-byte writes
  through the kernel.

Removing all three shortens the longest local task and the macOS unit-test CI
job, without dropping a single test or coverage figure.

## Context

**Coverage lane (A2).** `test:unit:cli` runs `cargo llvm-cov nextest`, and
before every run cargo-llvm-cov does a full clean: `cargo clean --package` for
all 44 workspace members, then removes every `*.profraw` file. The clean
alone takes 61 s on one thread, then the crates recompile instrumented (30 s).
Warm and alone, the task takes 247 s; with `--no-clean` it took 160 s.

Bare `--no-clean` is not safe. It never deletes old profile data, so each
report merges counts from earlier runs. In one experiment, kernel line
coverage read 89.94% with stale files present, against 77.99% after a clean.
Running `cargo llvm-cov clean --profraw-only` and then `--no-clean` gave the
same figures as a full clean.

The task also passes `--summary-only`. cargo-llvm-cov only honours that with
`--json`, `--lcov` or `--cobertura`, so a full run prints a 523-row per-file
table.

The team decided that `default` mirrors CI, so instrumentation stays both
locally and in CI.

**Duplicate lib run (A4).** The 338 visualiser lib unit tests run three times
per `default`:
- in `test:unit:visualiser`, under `dev-frontend`;
- in `test:unit:visualiser` again, under `embed-dist`;
- in `test:integration:visualiser`, through `cargo test --tests`, which
  includes the lib target with the same `dev-frontend` features as the first
  pass.

That third run took 33 s in a warm run and 238 s under load.

**Log-rotation tests (A3).** Three tests in `log::tests` push 2-byte lines
past the 5 MiB cap. `retains_at_most_three_rotated_segments` takes 22.3 s, of
which 20.8 s is kernel time, from about 13 M `write` syscalls. The module
costs about 42 CPU-s per pass, which makes the lib suite extremely sensitive
to load. Under the quick-wins experiment its `dev-frontend` pass took 129 s
and became the last task to finish.

## Requirements

1. Replace the coverage lane's full clean with `cargo llvm-cov clean
   --profraw-only` followed by `cargo llvm-cov nextest --no-clean`, keeping
   instrumentation both locally and in CI. Never use bare `--no-clean`.
2. Make the coverage summary take effect: pair `--summary-only` with
   `--json` reduced to a totals line, or remove it.
3. Change `test:integration:visualiser` from `--tests` to `--test '*'`, so it
   no longer runs the lib unit tests.
4. Make the `log::tests` rotation tests write in large chunks, or inject a
   smaller cap, once it is confirmed that rotation is byte-based.

## Acceptance Criteria

- [ ] Given a warm workspace, when `mise run --skip-deps test:unit:cli` runs
      alone, then it completes in under 180 s (247 s today).
- [ ] Given two consecutive coverage runs, when the second finishes, then
      the `*.profraw` files in `cli/target/llvm-cov-target` are only those
      the second run wrote.
- [ ] Given a full `mise run`, when the coverage lane reports, then it
      prints a totals summary rather than the 523-row per-file table.
- [ ] Given `test:integration:visualiser`, when it runs, then no
      `unittests src/lib.rs` target executes.
- [ ] Given `test:unit:visualiser`, when it runs, then the lib tests still
      run under both `dev-frontend` and `embed-dist`.
- [ ] Given the `log::tests` module, when it runs alone, then it uses under
      2 CPU-s (about 42 CPU-s today), and it still asserts rotation at the
      5 MiB cap and the retention limit.

## Open Questions

- Is rotation in `cli/visualiser/server/src/log.rs` byte-based? Chunked
  writes are only equivalent if it is.
- How often should a full `cargo llvm-cov clean --workspace` run locally to
  bound the risk from stale instrumented binaries?

## Dependencies

- Blocked by: 0368's `measure:build` harness and its warm no-op
  baseline. 0361 measures its 200 s target against that baseline, so it
  must be recorded before any part of this item merges. Work may start
  before then; only merging waits. Remove the `blocked_by` edge once that
  baseline is recorded.
- Blocks: none

## Assumptions

- CI's coverage numbers stay correct under the profraw-only clean, because
  its target directory is rebuilt from cache on each run.

## Technical Notes

- Coverage command and the `ACCELERATOR_COVERAGE` switch: `tasks/test/cli.py:11-36`.
- Visualiser lib passes: `tasks/test/unit.py:24-27`.
- Integration command: `tasks/test/integration.py:27`.
- Log tests and the cap: `cli/visualiser/server/src/log.rs:19-170`.
- cargo-llvm-cov warns that `--no-clean` "can cause false positives/false
  negatives due to old build artifacts". The profraw-only clean removes the
  stale-profile case, but stale instrumented binaries remain.

## Drafting Notes

- Grouped A2, A3 and A4 because each removes repeated work from a Rust test
  lane, as agreed in conversation.
- The A2 approach follows the decision that `default` mirrors CI. The
  research's earlier option of dropping instrumentation locally is
  superseded.
- The 180 s and 2 CPU-s targets are proposals derived from measurements of
  160 s with `--no-clean`, and about 42 CPU-s today.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- Related: 0361
