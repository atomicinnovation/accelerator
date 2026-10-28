---
type: "work-item"
id: "0368"
title: "Build-performance measurement and host guidance"
date: "2026-10-28T16:18:46+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0361"
source: "codebase-research:2026-10-28-full-build-duration-contention-and-slowness"
tags: ["build-system", "performance", "tooling", "documentation"]
last_updated: "2026-10-28T16:18:46+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0368: Build-performance measurement and host guidance

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The research measured the build with three ad-hoc scripts kept outside the
repository. Turning them into a `measure:build` task, and recording a
baseline, lets every other item in the epic show its saving. Alongside it,
documenting the host settings that cut background contention gives
contributors a faster local build at no cost to confidence.

## Context

The research harness is three scripts:
- **`timed-run.sh`** runs `mise run --output prefix` with every line
  timestamped, samples `iostat` and per-process cumulative CPU, and records
  wall-clock.
- **`iso.sh`** runs each task alone with `--skip-deps`.
- **`analyse.py`** reports each task's duration, queueing delay, cargo
  `Compiling`/`Checking` counts and lock waits, plus a 30-second profile of
  CPU busy and running tasks.

Their source is in the research document's appendix.

On the research machine, two host processes reacted to what the build wrote:
- **`syspolicyd`**, the XProtect/Gatekeeper assessment of new executables,
  used 44–78 CPU-s per run.
- **IntelliJ** used 270–380 CPU-s per run.

Each coverage run relinks 369 test binaries, and the bootstrap suites copy
binaries into temporary installations. After a build, both processes stayed
above 80% of a core for minutes. These effects were observed but not
separated out by experiment.

## Requirements

1. Commit the research harness as a `measure:build` task.
2. Record a baseline of a warm no-op `mise run` before the first speed
   change lands.
3. Document host settings for fast local builds:
   - add the terminal and IDE to macOS Developer Tools;
   - exclude `cli/target` and `node_modules` from IDE indexing.
4. Measure the effect of the host settings with `measure:build`, and record
   it in that documentation.

## Acceptance Criteria

- [ ] Given a workspace, when `mise run measure:build` runs, then it reports
      each task's duration, queueing delay, cargo unit counts and lock waits,
      and a 30-second CPU and concurrency profile, for a full `default` run.
- [ ] Given the epic, when the first speed change lands, then a warm no-op
      baseline from `measure:build` is already recorded.
- [ ] Given the contributor documentation, when a contributor reads it, then
      it describes the host settings and states their measured effect, or
      states plainly that it is unmeasured.
- [ ] Given `check` and `default`, when they run, then they do not run
      `measure:build`.

## Open Questions

- Where should baselines be recorded: under `meta/`, in the task's output
  directory, or both?
- Must `measure:build` run on Linux too? The harness currently relies on
  macOS `iostat` flags, `gdate`, and `ts` from moreutils.

## Dependencies

- Blocked by: none
- Blocks: none

## Assumptions

- `measure:build` follows the existing `measure:*` convention in
  `tasks/README.md`: it stays out of `check` and the bare `default` task,
  because a run needs a quiet host.

## Technical Notes

- Harness source: the research document's "Appendix: Measurement Harness".
- Existing `measure:*` tasks and their conventions: `tasks/README.md`
  ("The measure namespace").

## Drafting Notes

- Grouped C1 and C2, agreed in conversation. `measure:build` is also how the
  effect of the host settings gets measured.
- Kept `measure:build` out of `check` and `default`. This is an
  interpretation drawn from the existing `measure:*` convention; the
  research does not say so.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- Related: 0361
