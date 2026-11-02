---
type: "pr-description"
id: "177"
title: "[0368] Isolate and cold-start build measurements, and compare build records"
date: "2026-11-02T17:38:33+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0368"
parent: "work-item:0368"
relates_to: ["work-item:0361", "work-item:0362", "work-item:0363", "work-item:0364", "work-item:0365", "work-item:0366", "work-item:0367", "work-item:0370"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/177"
pr_number: 177
tags: ["build-system", "measurement", "performance", "mise", "macos"]
revision: "9015fd21f8699a0b4a5cabb1e5233b59513040c9"
repository: "accelerator"
last_updated: "2026-11-02T17:38:33+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0368] Isolate and cold-start build measurements, and compare build records

## Summary

Finishes 0368 on top of PR #175's `measure:build`. It commits the warm no-op
baseline the 0361 children were waiting for, adds `--isolate` and
`--cache-state` to `measure:build`, adds `measure:compare-builds` for reading
a before/after pair, and documents the macOS host settings in
`CONTRIBUTING.md`. Phases 5–9 of the 0368 plan land here. Phase 10, measuring
and citing the host settings' effect, moves to the new work item 0370, so 0368
closes as done.

## Changes

- **Baseline record** (Phase 5).
  `meta/measurements/build_2026-11-01T143523Z_engywook_warm-no-op_devtools.json`,
  measured at `007b2dd6` (the PR #175 merge plus a version bump), so it holds
  none of 0362–0367's changes. Median wall-clock is 315.1 s (303.3–317.0) on
  AC power with `devtools` host settings. `test_record.py` names it.
  Committing it discharges the baseline gate, so 0362–0364 and 0367 are no
  longer `blocked_by` 0368.
- **`--isolate <task>`** (Phase 6). Repeatable. It times each named task
  alone rather than the whole `default` closure. `TaskGraph` gains
  `isolated`, `unknown` and `closest_to`, and an unknown name is refused
  before the session lock with its closest match, e.g.
  `'tset' (closest: 'test')`. Run ids gain an `_isolated` suffix. The task
  graph is now parsed once, before the lock, and passed to the session.
- **`--cache-state warm-no-op|incremental|cold`** (Phase 7). The new
  `workspace.py` prepares the checkout before each timed run: `incremental`
  touches `cli/kernel/src/lib.rs`, and `cold` moves `cli/target` and the
  frontend's `node_modules` into `cold-trash-<n>/`, keeping their paths, then
  removes the trash. `cold` skips the priming run (`priming: null`) and
  refuses `--isolate`. Every check runs before the first move. A session
  stops if its trash cannot be removed, and the preflight and exit report
  warn of deletions, leftover trash and a missing dev launcher.
- **`measure:compare-builds <previous> <current> [--task …]`** (Phase 9).
  `comparison.py` reads two records and lists differing conditions first. It
  then reports wall-clock, idle load, process CPU, task-set and unit-count
  changes in seconds and as a ratio, each with a noise verdict under the
  citation rules. `--task` adds that task's duration and nextest phase.
  `read_record` refuses a missing or unreadable file with
  `UnreadableRecordError`. `pyproject.toml` allows `×` as a ruff confusable,
  since the ratios print as `×0.847`.
- **Host guidance** (Phase 8). `CONTRIBUTING.md` gains
  "Faster local builds on macOS". It covers the Developer Tools exemption for
  the terminal and IDE (effective only after a restart) and IDE exclusion of
  the three build-output folders. It also states the security cost and the
  file-watching limit (IJPL-2175). The `--host-settings` bullet in
  `tasks/README.md` links to it, and `test_contributing.py` pins the section.
- **Work-item edits.** 0365 and 0366 lose their `blocked_by` edges to 0368,
  and 0366 is now blocked by 0362 alone. Their Technical Notes name the
  `measure:compare-builds` command for their own before/after records. 0368 is
  `done`, with Requirement 9 pointing at the new 0370. 0370 is blocked by
  0362, 0363 and 0367.
- **pulldown-cmark 0.13.6** for RUSTSEC-2026-0433, which `deny.toml` requires
  us to take as an upgrade even though wikilinks are disabled. Every golden
  digest holds, so 0.13.6 is recorded against recipe 1 rather than bumping
  the recipe.
- **PR #175 follow-up.** The first review is recorded as
  `meta/reviews/prs/175-review-1.md`, and the PR #175 description's CI lane is
  ticked.

## Context

- Work item: `meta/work/0368-build-performance-measurement-and-host-guidance.md`
  (parent epic `meta/work/0361-speed-up-the-full-build-without-losing-confidence.md`)
- Plan: `meta/plans/2026-10-30-0368-build-performance-measurement-harness.md`,
  now `done`. Its Implementation Notes record where each of Phases 5–9
  departed from the plan.
- Follow-on: `meta/work/0370-measure-and-cite-the-macos-host-settings-effect-on-the-build.md`
- Previous PR: #175 (Phases 1–4)

## Testing

- [x] Harness, measure, contributing and mise-guard tests pass at the branch
  tip: 2559 passed
  (`uv run pytest tests/unit/tasks/shared/build_measurement tests/unit/tasks/test_{measure,contributing,mise}.py`)
- [x] `mise run build-system:check` passes at the branch tip (pyrefly: 0 errors)
- [x] Golden digests hold under pulldown-cmark 0.13.6
  (`cargo test -p work-adapters --test golden_digests`: 6 passed)
- [x] Full local CI mirror `mise run` passed at the end of Phases 6, 7, 8 and
  9, as the plan records. It has not been rerun since the pulldown-cmark bump
  and the final comment removal.
- [x] Manual, on the reference host, AC power:
  - `--isolate test:unit:cli --runs 1`: nextest phase 171.4 s.
  - `--cache-state cold --runs 1`: deleted a 39 GB `cli/target` and
    `node_modules`, rebuilt both in 435.5 s, and left no trash.
  - `--cache-state incremental --runs 1`: 334.1 s, and
    `cli/kernel/src/lib.rs` had no diff afterwards.
  - `measure:compare-builds` over the baseline and itself with
    `--task test:unit:cli` reported matching conditions and no change.
- [x] Manual: the TCC database's `kTCCServiceDeveloperTool` rows allow the
  terminal and IDEs, each started after its row changed, as the
  `CONTRIBUTING.md` steps describe.
- [ ] CI on both OSes. It has not run on Phases 6–9 yet.

## Notes for Reviewers

- **Where to start.** `comparison.py` is the largest new module (~540 lines)
  and decides what counts as a change. `workspace.py` is the only code here
  that deletes anything, so its pre-move checks are the part to scrutinise.
- **Line count.** The baseline record (~5.2k lines) is a real capture and
  makes up over half the diff.
- **Baseline settings.** The baseline is a `devtools` record, not the `none`
  record the README asks for first. 0370's `none` record therefore needs the
  exemption removed, which can need a reboot.
- **Restoring edges.** If this PR is abandoned, restore the `blocked_by`
  edges from 0365 and 0366 to 0368.
- **Deferred.** The PR #175 review's structural findings are still open and
  are listed in the plan's "Deferred to later phases". They include a
  condition value object, a `ports.py`, and splitting `tasks/measure.py`.
