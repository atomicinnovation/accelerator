---
type: "work-item"
id: "0391"
title: "Shorten the guarded-resume migration tests"
date: "2026-11-18T13:28:33+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0361"
blocked_by: ["work-item:0365"]
relates_to: ["work-item:0365"]
external_id: "PP-943"
tags: ["performance", "testing", "rust", "migrate"]
last_updated: "2026-11-18T13:28:33+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0391: Shorten the guarded-resume migration tests

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The 14 tests in `accelerator-migrate::guarded_resume` take 322
test-seconds, and its ten jj tests run for 24–39 s each. Once 0365 stops
the VCS fixture tests rebuilding their fixtures, these become the slowest
tests in `test:unit:cli`'s nextest phase. That limits how far the phase,
and with it the full build, can shrink. Shortening them keeps every
scenario and colocation they cover.

## Context

In the isolated `test:unit:cli` baseline
(`build_2026-11-18T123358Z_engywook_warm-no-op_devtools_isolated.json`, at
`287b6279`), the second timed run gives `guarded_resume`:
- ten jj tests at 24.5–39.2 s each;
- three tests at 6.4, 6.4 and 4.2 s;
- 322 test-seconds in all.

In that run only the 19 fixture-matrix tests that 0365 targets were slower.

Each jj test loops over both colocations, `--no-colocate` and
`--colocate`. On each pass it builds a stalled migration from scratch, then
resumes it, rewrites its parents or edits its files. Building the stalled
state alone takes about ten `jj` CLI calls and a run of the migrate binary,
all serial within the test.

Unlike 0365's fixtures, these tests change their repositories, so one
shared build cannot serve them as it stands.

The baseline was taken under the suite's full load. How much of each test's
time is its own work and how much is contention is not yet known. 0365's
matrix build is the warning case: it takes 3.3 s alone and 45 s under load.

## Requirements

1. Establish how much of the guarded-resume tests' time is their own work
   and how much is contention with the rest of the suite. Time the binary
   alone and under the suite.
2. Shorten the slowest guarded-resume tests without dropping a colocation,
   a scenario or an assertion.
3. Every pairing of scenario and colocation that runs today still runs.
4. Research and planning choose the approach. Candidates include:
   - splitting each colocation into its own test so the two run in
     parallel;
   - building each stalled starting state once and giving each test a
     private copy;
   - spawning fewer `jj` processes.

## Acceptance Criteria

- [ ] Given an isolated `test:unit:cli` record
      (`mise run measure:build -- --isolate test:unit:cli --host-settings
      devtools --runs 3 --keep`) at the commit before this change and one
      after it, when `guarded_resume`'s per-test durations are read from
      each record's timed-run logs, then its median total test-seconds
      across the three runs is at most half the before figure, and its
      slowest test takes under 20 s.
- [ ] Given the change, when the suite runs, then every pairing of
      scenario and colocation that ran before still runs, with the same
      assertions.
- [ ] Given the two records, when
      `mise run measure:compare-builds -- <before> <after> --task test:unit:cli`
      runs, then its first line reports no differing condition and the
      nextest-phase median is no longer than before.

## Open Questions

- If most of the cost is contention, 0365 may remove most of it. Once 0365
  lands, re-measure. If `guarded_resume` is no longer the slowest binary in
  the phase, should this item be abandoned?

## Dependencies

- Blocked by: 0365. Its before figures are taken once 0365 has cut the
  suite's contention, and that measurement decides whether this item is
  still worth doing.
- Blocks: none.
- Related: 0365.

## Assumptions

- The 322 test-seconds come from the 18 November baseline, before 0365. The
  before and after figures for this item's criteria are taken at its own
  commits, not at that baseline.

## Technical Notes

- Tests: `cli/migrate-cli/tests/guarded_resume.rs`. The colocation loop is
  `COLOCATIONS` at `:125`. The stalled-state builder ends around `:150-180`.
- Shared helpers: `cli/migrate-cli/tests/common/mod.rs`.
- The per-test durations are the `PASS [ Ns ]` lines in each
  `run-N-test_unit_cli.log` under the record's
  `.accelerator-measure/build/<run-id>/` directory.

## Drafting Notes

- Raised as a child of 0361, split out while refining 0365, where
  `guarded_resume` showed up as the next-slowest binary.
- The targets (half the test-seconds, slowest test under 20 s) were agreed
  in conversation. They are not projections from a measurement.
- Made this item blocked by 0365, so its before figures include 0365's lower
  contention.

## References

- Baseline: `meta/measurements/build_2026-11-18T123358Z_engywook_warm-no-op_devtools_isolated.json`
- Related: 0361, 0365
