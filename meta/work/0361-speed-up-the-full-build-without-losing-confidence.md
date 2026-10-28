---
type: "work-item"
id: "0361"
title: "Speed up the full build without losing confidence"
date: "2026-10-28T16:18:46+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "epic"
priority: "high"
relates_to: ["work-item:0369"]
source: "codebase-research:2026-10-28-full-build-duration-contention-and-slowness"
tags: ["build-system", "performance", "ci"]
last_updated: "2026-10-29T08:51:57+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0361: Speed up the full build without losing confidence

**Kind**: Epic
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

A bare `mise run` takes 317 s warm with no source changes (464 s on a cold
workspace). CI's critical path is a 23-minute integration job. Research traced
both to a handful of long-running test tasks and to redundant Rust
compilation, each with a specific, measured fix. This epic schedules those
fixes without weakening any check the build performs today.

## Context

Codebase research measured the full build in three cache states on one
workstation, isolated each long-running task, and confirmed every root cause
with a reverted experiment. The long poles are:

- **Debug-built verify shim:** in the bootstrap suites, it takes 1.15 s per
  call. On CI's ubuntu runner, skill-invocation alone takes 1,266 s.
- **Coverage lane:** it cleans and recompiles every workspace crate on every
  run, costing about 90 s.
- **Visualiser tests:** the lib tests run three times, and their log-rotation
  tests make about 13 M two-byte `write` syscalls.
- **Serial suites:** Playwright and the dev-task suite run strictly serially.
- **Hooks smoke test:** it downloads the published launcher from GitHub on
  every run.
- **Fixture rebuilds:** VCS fixture matrices are rebuilt in every nextest
  process.

Cold and incremental runs also pay for 15 compile configurations, 2,015 crate
compilations for 456 distinct crates, and rebuilds of unchanged inputs.

Applying five of the quick wins together, as a temporary experiment, cut the
warm no-op `default` from 317 s to 244 s.

The team has decided that `default` mirrors CI as far as possible. Every child
item therefore changes shared task definitions or code, and speeds up CI as
well as local runs.

## Requirements

Goals:
- Reduce the wall-clock of a warm no-op `mise run`.
- Shorten the CI workflow's critical path.
- Keep every gate that exists today, with no lighter local profile. One
  check moves rather than disappears: 0362 drops the hooks smoke test's
  per-pull-request check of the committed public key against a published
  signature, which `prerelease:finalise` and `release:finalise` keep.

Decomposition:
- 0362: Make the bootstrap test suites fast and hermetic (A1, A7)
- 0363: Remove repeated work from the Rust test lanes (A2, A3, A4)
- 0364: Parallelise the serial test suites (A5, A6)
- 0365: Build the VCS fixture matrix once per test run (B1)
- 0366: Cut Rust compile cost (B2, B5)
- 0367: Stop rebuilding unchanged inputs (B3, B4, B6)
- 0368: Build-performance measurement and host guidance (C1, C2)

## Acceptance Criteria

- [ ] Every child work item is done or explicitly abandoned.
- [ ] Given a `measure:build` baseline recorded before the first child
      lands, when all children are done, then a warm no-op `mise run` on
      the same machine completes in under 200 s.
- [ ] Given the main workflow's job timings, when all children are done,
      then no job in a pull-request run takes longer than 12 minutes.
- [ ] Every test that runs today still runs, except for the deliberately
      removed third run of the visualiser lib tests. Children may add
      tests.
- [ ] `default` still depends on everything it depends on today.

## Open Questions

- Which machine and conditions count as the reference for the 200 s target?
  The research machine had 22–30% background CPU load.
- Should the CI target be measured on a single pull-request run, or as a
  median over several?

## Dependencies

- Blocked by: none
- Blocks: none
- Related: 0369 "Make `default` mirror CI" (adds work to a local run;
  tracked separately)

## Assumptions

- Ratios from one Apple M4 Max carry over to other developer machines, even
  though the absolute figures will not.

## Technical Notes

The research document holds the measurement workflow, the harness scripts
and per-finding evidence. The combined what-if result is the baseline for
the 200 s target: 244 s with five quick wins, and 170–190 s projected once
the log-rotation tests are fixed.

## Drafting Notes

- Grouped the research's A/B/C recommendations into seven children, as
  agreed in conversation. The RUSTSEC-2026-0414 fix is deliberately excluded
  from this batch, and minor observations without a recommendation were
  dropped.
- Set priority to high because the research was prompted by the build
  becoming very long, and the CI critical path is 23 minutes. The source
  does not state a priority.
- Turned the research's projections into targets: under 200 s locally and
  under 12 minutes per CI job. The research projects 170–190 s and roughly
  11.5 minutes. Treat these as proposals, not commitments.
- Kept 0369 "Make `default` mirror CI" outside this epic as a related item,
  because it adds work rather than removing it.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- Related: 0362, 0363, 0364, 0365, 0366, 0367, 0368, 0369
