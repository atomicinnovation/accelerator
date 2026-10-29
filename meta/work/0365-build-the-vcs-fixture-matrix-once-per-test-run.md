---
type: "work-item"
id: "0365"
title: "Build the VCS fixture matrix once per test run"
date: "2026-10-28T16:18:46+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0361"
blocked_by: ["work-item:0368"]
source: "codebase-research:2026-10-28-full-build-duration-contention-and-slowness"
tags: ["performance", "testing", "rust", "vcs"]
last_updated: "2026-10-30T01:47:37+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0365: Build the VCS fixture matrix once per test run

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Every test in the VCS adapter suites builds its own copy of the full git/jj
fixture matrix, spawning real `git` and `jj` processes in each nextest
process. These are the longest tests in the coverage lane. Building the
matrix once per run and sharing it with the tests that only read it would
shorten the nextest phase's tail.

## Context

The nextest phase of `test:unit:cli` runs 6,407 tests across 369 binaries,
2,296 test-seconds in all. The slowest 25 tests, at 20–47 s each, are all VCS
fixture tests. The heaviest binaries are:
- `vcs-adapters::classify`, 277 test-seconds;
- `vcs-test-support::matrix`, 208;
- `vcs-adapters::queries`, 172.

Thirteen tests across `classify`, `queries` and `scrub` each call
`Matrix::build_in(base.path())`. One matrix build takes 3.3 s wall and
1.9 CPU-s when run alone, but the same tests take 35–47 s under the suite's
load.

A sharing mechanism already exists: `Matrix::build_or_adopt` adopts a prebuilt
matrix named by `ACCELERATOR_ZERO_SPAWN_MATRIX`. Only the zero-spawn lane uses
it.

Two other explanations were ruled out. nextest's process-per-test model is
not the overhead: 338 tests took 10.8 s under nextest and 11.1 s under
`cargo test`. And the phase is not CPU-saturated: CPU ran at about 50% busy
when the phase ran alone.

## Requirements

1. Build the fixture matrix once per test run, for example from a nextest
   setup script.
2. Make every test that only reads the matrix adopt it through
   `build_or_adopt` and `ACCELERATOR_ZERO_SPAWN_MATRIX`.
3. Audit each matrix consumer for mutation. Tests that modify fixtures keep
   building their own private matrix.

## Acceptance Criteria

- [ ] Given a full nextest run, when it completes, then no test that only
      reads the matrix has called `Matrix::build_in`.
- [ ] Given a test that modifies fixtures, when it runs, then it still
      builds a private matrix.
- [ ] Given a warm workspace, when `measure:build` runs `test:unit:cli` in
      isolation mode, then the median nextest phase it reports is at least
      20 s shorter than at the commit before the change.
- [ ] Given the zero-spawn lane, when it runs, then it still passes with the
      matrix handed over through `ACCELERATOR_ZERO_SPAWN_MATRIX`.

## Open Questions

- Which matrix consumers modify the fixtures? This has not been audited.
  `scrub` poisons the environment rather than the fixtures, but that needs
  confirming.
- Should the status-log state builders (`status_log::States::build_or_adopt`)
  be shared the same way? `accelerator-vcs::status_log_goldens` is among the
  heaviest binaries, but the research recommends this change only for the
  matrix.

## Dependencies

- Blocked by: 0368's `measure:build` harness and its warm no-op
  baseline. 0361 measures its 200 s target against that baseline, so it
  must be recorded before any part of this item merges. Work may start
  before then; only merging waits. This item also needs 0368's isolation
  mode (its Requirement 3) merged first, because its speed criterion
  compares an isolated nextest phase against the commit before the
  change. Remove the `blocked_by` edge once both the baseline and
  isolation mode are in.
- Blocks: none

## Assumptions

- A matrix built once per run gives each read-only test the same view it
  gets from a private build today.

## Technical Notes

- Variable, `matrix_root` and `build_or_adopt`:
  `cli/vcs-test-support/src/fixtures.rs:25-116`.
- A per-test rebuild: `cli/vcs-adapters/tests/classify.rs:177`.

## Drafting Notes

- Kept B1 as its own item, separate from 0363, because it is medium-sized
  and needs the mutation audit first. Agreed in conversation.
- The "at least 20 s shorter" target comes from the research's projection of
  20–40 s off nextest. It was not measured.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- Related: 0361
