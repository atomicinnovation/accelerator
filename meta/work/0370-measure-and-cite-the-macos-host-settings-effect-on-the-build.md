---
type: "work-item"
id: "0370"
title: "Measure and cite the macOS host settings' effect on the build"
date: "2026-11-02T11:40:46+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0361"
blocked_by: ["work-item:0362", "work-item:0363", "work-item:0367"]
relates_to: ["work-item:0368"]
source: "work-item:0368"
tags: ["build-system", "performance", "documentation", "macos"]
last_updated: "2026-11-02T11:40:46+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0370: Measure and cite the macOS host settings' effect on the build

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

`CONTRIBUTING.md`'s "Faster local builds on macOS" gives two host settings,
the Developer Tools exemption and the IDE exclusions, and their costs, but
not what they save. This item measures each setting with `measure:build`
on the reference host, commits the three condition records, and cites the
changes in `CONTRIBUTING.md`. It waits until 0362, 0363 and 0367 have each
landed or been abandoned, since those remove most of what the settings
save.

## Context

0368 built `measure:build`, recorded the warm no-op baseline and wrote the
host guidance. It split this measurement out because its condition records
must wait on three siblings, while the harness and the baseline had to
land before them.

The research attributed 44–78 CPU-s per run to `syspolicyd` and 270–380
CPU-s to IntelliJ, but never separated their effect by experiment. The
baseline (`devtools` settings, 315.1 s median at `007b2dd6`) is not one of
the conditions: it predates the siblings' changes.

## Requirements

1. Record three warm no-op `--runs 3` records on the reference host, at one
   commit on `main` taken after 0362, 0363 and 0367 have each landed or
   been abandoned, with the IDE open on the project throughout:
   - `none`: the terminal, the IDE and any tmux or zellij removed from
     Developer Tools, and the IDE exclusions cleared;
   - `devtools`: the Developer Tools entries added and the apps restarted;
   - `devtools+ide-exclusion`: the IDE exclusions added as well.
2. Retake a record under any of the retake conditions in `tasks/README.md`'s
   "Taking a baseline or condition record", or when `commit.matched` is
   false, or when `syspolicyd`, the IDE or `fsnotifier` is absent from any
   timed run.
3. Check comparability with `mise run measure:compare-builds -- <none>
   <devtools>` and `… <devtools> <devtools+ide-exclusion>`. Each must name
   exactly `host_settings` as differing. Commit the three records.
4. Add `### Measured effect` under `CONTRIBUTING.md`'s "Faster local builds
   on macOS", built from the two comparison reports:
   - the host (`engywook`, Apple M4 Max, macOS version) and the commit;
   - each step's change in median wall-clock and in each tracked process's
     median CPU-seconds, in seconds and as a ratio, with the IDE
     exclusion's effect stated as on top of the exemption;
   - "not distinguishable from noise on the reference host" wherever the
     report says so;
   - every change to or from absent named in words, with no ratio and no
     noise verdict;
   - every change from an idle median in seconds and in words, with no
     ratio;
   - the three record paths;
   - a sentence saying the ratios carry over to other Macs and the absolute
     seconds do not;
   - a pointer to `mise run measure:build -- --host-settings <label>` for
     measuring another Mac.

## Acceptance Criteria

- [ ] Given the three condition records, then they are labelled `none`,
      `devtools` and `devtools+ide-exclusion`, share one commit taken after
      0362, 0363 and 0367 each landed or were abandoned, hold three timed
      runs each, have `commit.matched` true and empty `fetch_failures`
      lists, and show `syspolicyd`, the IDE and `fsnotifier` present in
      every timed run.
- [ ] Given each consecutive pair of condition records, when
      `measure:compare-builds` compares them, then it names exactly
      `host_settings` as differing.
- [ ] Given `CONTRIBUTING.md`, when a contributor reads "Measured effect",
      then it cites each step's change in median wall-clock and
      per-process CPU-seconds, in seconds and as a ratio, naming the host
      and the commit. It states the IDE exclusion's effect as on top of
      the exemption, says which changes were within noise, and names a
      change to or from absent in words with no ratio or noise verdict.
- [ ] Given `tests/unit/tasks/test_contributing.py`, then:
      - `test_the_measured_effect_cites_three_condition_records` checks
        that each cited path exists under `meta/measurements/` and carries
        its label;
      - `test_every_cited_change_matches_the_comparison` runs
        `compare_records` over the cited records and finds each rendered
        change, its seconds, ratio and noise wording, in the section;
      - `test_the_measured_effect_points_at_measure_build` passes.

## Open Questions

None.

## Dependencies

- Blocked by: 0362, 0363 and 0367. Abandoning one discharges its edge.
- Blocks: none.
- External: GitHub releases, the RustSec advisory database and, if a cold
  record is ever wanted, the npm registry, as for 0368's records.
- Related: 0368, which built `measure:build`, `measure:compare-builds` and
  the host guidance this item cites into.

## Assumptions

- Ratios between the conditions carry over to other Macs; absolute
  figures do not.
- The guidance targets JetBrains IDEs, the only IDE observed.

## Technical Notes

- Record `none` first: undoing the Developer Tools exemption can need a
  reboot. Check the system TCC database's `kTCCServiceDeveloperTool` rows
  to confirm the exemption is gone or present.
- Keep the IDE open on the project for every condition, or its CPU never
  shows. Run on AC power: `--keep` warns about battery, but the record
  does not capture it.
- A warm `--runs 3` session takes about 20 minutes, so the three records
  take about an hour of operator time with the settings changes between
  them.
- The settings are measured cumulatively (neither, exemption, both), not
  as a 2×2 grid, so the IDE exclusion's effect without the exemption is
  not isolated.

## Drafting Notes

- Split from 0368's Requirement 9 and Phase 10 of its plan, at the
  operator's request, so 0368 could close before its three siblings land.

## References

- Split from: 0368
- Plan: `meta/plans/2026-10-30-0368-build-performance-measurement-harness.md`
  (Phase 10)
- Baseline: `meta/measurements/build_2026-11-01T143523Z_engywook_warm-no-op_devtools.json`
- Source: `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- Related: 0361, 0362, 0363, 0367, 0368
