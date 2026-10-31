---
type: "work-item"
id: "0366"
title: "Cut Rust compile cost"
date: "2026-10-28T16:18:46+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0361"
blocked_by: ["work-item:0362", "work-item:0368"]
source: "codebase-research:2026-10-28-full-build-duration-contention-and-slowness"
tags: ["build-system", "performance", "rust"]
last_updated: "2026-10-30T13:09:40+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0366: Cut Rust compile cost

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

A cold `mise run` saturates the CPU. It compiles 456 distinct crates 2,015
times, across 15 compile configurations that cannot share artefacts. Merging
configurations where nothing requires them to differ, and trialling lighter
debug info, would reduce cold and incremental compile cost.

## Context

The research found 15 compile configurations. A configuration is a
combination of toolchain, target directory, profile, mode, wrapper, package
selection and unified features. They include:
- six stable debug builds, one per `--bin` root, plus a shim build inside
  pytest;
- two visualiser test configurations;
- three clippy check configurations;
- nightly rustdoc for 16 roots, and the nightly pup-driver check;
- a thin-LTO release build;
- the instrumented coverage build.

Resolver 3 unifies features only across the packages selected in a call, so
each `--bin` root gets its own feature closure. In `cli/target/debug/deps`,
`kernel` exists as 7 different artefacts and `syn` as 5.

In the cold run:
- Cargo did 1,164 `Compiling` units and 851 `Checking` or `Documenting`
  units, for 456 distinct crates.
- 33 cargo invocations shared `cli/target/debug/.cargo-lock`.
- Tasks spent about 440 task-seconds waiting on that lock.

The CPU was saturated, so the lever is less duplicated work, not more
parallelism. Separate target directories would remove the lock waits but
compile the shared dependency graph yet again.

Debug binaries are large; the launcher is 76 MB. Lighter debug info for
dependencies could shorten links and hashing, but that has not been measured.

## Requirements

1. Collapse the debug builds into one `cargo build` with unified features,
   where no lane needs a different feature set.
2. Drop the cargo builds inside pytest that mise already covers.
3. Align the clippy feature sets, so the visualiser and workspace clippy
   passes share more artefacts.
4. Trial `debug = "line-tables-only"` for dependencies in the dev profile.
   Measure cold build time, link time and binary size, and adopt it only if
   it measurably helps.

## Acceptance Criteria

- [ ] Given `measure:build` cold records at the commit before the change
      and after it, then the total of `Compiling`, `Checking` and
      `Documenting` units is lower after the change. The research counted
      2,015 at its commit; `--runs 1` is enough, since unit counts do not
      vary between runs.
- [ ] Given the tasks reachable from `default`, when their cargo invocations
      are grouped into compile configurations, then there are fewer than 15.
- [ ] Given the debug-info trial, when it completes, then its measurements
      are recorded as `measure:build` cold `--runs 3` records, and the
      setting is adopted only if it reduces the median cold build
      wall-clock.
- [ ] Every feature combination a test, lint or check exercises today
      (`test-loopback`, `dev-frontend`, `embed-dist`, `--all-features`) is
      still exercised.

## Open Questions

- Does any lane still need a separate per-binary debug build? `tasks/README.md`
  notes that two builds of one `target/debug/<bin>` with different features
  overwrite each other.
- Is losing debugger detail in dependencies acceptable, if the debug-info
  trial pays off?

## Dependencies

- Blocked by: 0368's `measure:build` harness and its warm no-op
  baseline. 0361 measures its 200 s target against that baseline, so it
  must be recorded before any part of this item merges. Work may start
  before then; only merging waits. This item also needs 0368's cold
  cache state (its Requirement 4) merged first, because its unit-count
  criterion and debug-info trial compare cold records. Remove the
  `blocked_by` edge once both the baseline and the cold state are in.
- Blocked by: 0362, which lands first. Collapsing the debug builds must
  keep `test:integration:hooks`'s launcher and `accelerator-vcs` builds
  working, and the debug-info trial changes the launcher size that 0362's
  speed ratios measure against. 0362 adds one in-pytest build, the debug
  shim through `build_shim()`; no mise task builds the shim today. If
  this item brings the debug shim under a mise task, the hooks,
  entrypoint and skill-invocation suites take that edge in place of
  `build_shim()`, and the shim keeps 0362's `minisign-verify` override.
- Blocks: none

## Assumptions

- Cold and incremental compile cost matters even though warm no-op runs are
  dominated by tests. The research measured incremental Rust work at about
  20 s on top of the test floor.

## Technical Notes

- One cargo build per binary group: `tasks/shared/dev_builds.py:68-78`.
- In-test builds: `tests/support/artefacts.py:56-93`.
- The two clippy feature passes: `tasks/lint/server.py:15`.
- The only profile override today is for release: `cli/Cargo.toml:260-262`.

## Drafting Notes

- Grouped B2 and B5, agreed in conversation. The B5 spike becomes a measured
  trial step within this task rather than a separate spike.
- The research gives no numeric target for fewer configurations or units.
  "Fewer than the recorded baseline" is the measurable form available.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- Related: 0361
