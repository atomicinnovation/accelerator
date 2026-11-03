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
revision: "a5929988a93abc78a9b68329899f42d8f7d5035f"
repository: "accelerator"
last_updated: "2026-11-03T13:36:51+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0368] Isolate and cold-start build measurements, and compare build records

## Summary

Finishes 0368 on top of PR #175's `measure:build`. It commits the warm no-op
baseline the 0361 children were waiting for, adds `--isolate` and
`--cache-state` to `measure:build`, adds `measure:compare-builds` for reading
a before/after pair, and documents the macOS host settings in
`CONTRIBUTING.md`. Phases 5–9 of the 0368 plan land here, together with fixes
for the first review's findings. Phase 10, measuring and citing the host
settings' effect, moves to the new work item 0370, so 0368 closes as done.

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
  graph is now parsed once, before the lock, and passed to the session. An
  isolated task that fails without printing its exit status takes the
  invocation's, through `TaskFigures.failed_silently_with`. Tests pin both
  guards: a full build's failed task never takes the invocation's status, and
  a printed status always wins.
- **`--cache-state warm-no-op|incremental|cold`** (Phase 7). The new
  `workspace.py` prepares the checkout before each timed run: `incremental`
  touches `cli/kernel/src/lib.rs`, and `cold` moves `cli/target` and the
  frontend's `node_modules` into `cold-trash-<n>/`, keeping their paths, then
  removes the trash. Both paths, and the trash prefix, live in
  `locations.py`. Deletion is all-or-nothing: every check runs before the
  first move, and a failed or interrupted move puts back whatever it had
  already moved. `cold` skips the priming run (`priming: null`) and refuses
  `--isolate`, giving `--skip-deps` as the reason and `warm-no-op`,
  `incremental` or a full cold build as the alternatives. A session stops if
  its trash cannot be removed and lists that trash once, as unremoved. The
  preflight warns of deletions, leftover trash and a dev launcher that will
  need rebuilding. The exit report repeats the dev-launcher warning only once
  something has been deleted, and logs a marker it could not read. The notice
  types live in `session_notices.py`.
- **`measure:compare-builds <previous> <current> [--task …]`** (Phase 9).
  `comparison.py` models the comparison and `comparison_report.py` renders
  it, sharing one plural with `reports.py`. The report lists differing
  conditions first. It then reports wall-clock, idle load, process CPU,
  task-set and unit-count changes in seconds and as a ratio, each with a noise
  verdict under the citation rules. A zero previous median gives seconds
  without a ratio. Unit totals are compared only when every run in each record
  agrees on them; otherwise the report names the record whose runs vary.
  `--task` adds that task's duration and nextest phase, matching names through
  `TaskGraph`'s closest-match lookup. When the host-settings labels differ,
  each record line is introduced by its label. `read_record` refuses a
  missing, unreadable or non-UTF-8 file with `UnreadableRecordError`.
  `pyproject.toml` allows `×` as a ruff confusable, since the ratios print as
  `×0.847`.
- **Host guidance** (Phase 8). `CONTRIBUTING.md` gains
  "Faster local builds on macOS". It covers the Developer Tools exemption for
  the terminal and IDE (effective only after a restart) and IDE exclusion of
  the three build-output folders. It also states the security cost and the
  file-watching limit (IJPL-2175). The `--host-settings` bullet in
  `tasks/README.md` links to it, and `test_contributing.py` pins the section.
- **Build-timing docs.** The lead of `tasks/README.md`'s "Build timing" covers
  `--isolate` and `cold`. The comparison paragraph says that `compare-builds`
  takes record file paths and leaves a one-run previous record unjudged for
  noise. The `tasks/measure.py` docstring and the `measure:build` description
  in `mise.toml` describe the namespace as it now is.
- **Work-item edits.** 0365 and 0366 lose their `blocked_by` edges to 0368,
  and 0366 is now blocked by 0362 alone. Their Technical Notes name the
  `measure:compare-builds` command for their own before/after records. 0368 is
  `done`, with Requirement 9 pointing at the new 0370. 0370 is blocked by
  0362, 0363 and 0367. 0367 names the one `node_modules` that `cold` deletes.
  Every edited item has `last_updated` bumped and `blocks:` beside `parent:`.
- **pulldown-cmark 0.13.6** for RUSTSEC-2026-0433, which `deny.toml` requires
  us to take as an upgrade even though wikilinks are disabled. Every golden
  digest holds, so 0.13.6 is recorded against recipe 1 rather than bumping
  the recipe.
- **Review records.** The first reviews of PR #175 and of this PR are recorded
  as `meta/reviews/prs/175-review-1.md` and `meta/reviews/prs/177-review-1.md`,
  and the PR #175 description's CI lane is ticked.

## Context

- Work item: `meta/work/0368-build-performance-measurement-and-host-guidance.md`
  (parent epic `meta/work/0361-speed-up-the-full-build-without-losing-confidence.md`)
- Plan: `meta/plans/2026-10-30-0368-build-performance-measurement-harness.md`,
  now `done`. Its Implementation Notes record where each of Phases 5–9
  departed from the plan.
- Follow-on: `meta/work/0370-measure-and-cite-the-macos-host-settings-effect-on-the-build.md`
- Review: `meta/reviews/prs/177-review-1.md`
- Previous PR: #175 (Phases 1–4)

## Testing

- [x] Harness, measure, contributing and mise-guard tests pass at the branch
  tip: 2586 passed
  (`uv run pytest tests/unit/tasks/shared/build_measurement tests/unit/tasks/test_{measure,contributing,mise}.py`)
- [x] `mise run build-system:check` passes at the branch tip (pyrefly: 0 errors)
- [x] Golden digests hold under pulldown-cmark 0.13.6 at the rebased tip
  (`cargo test -p work-adapters --test golden_digests`: 6 passed)
- [x] A failed second move puts the first back, tested on a real `tmp_path` in
  `test_workspace.py`.
- [x] Full local CI mirror `mise run` passed at the end of Phases 6, 7, 8 and
  9, as the plan records. It has not been rerun since the pulldown-cmark bump,
  the review fixes or the rebase onto `0d0b16bd`.
- [x] Manual, on the reference host, AC power, before the review fixes:
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
- [ ] CI on both OSes. It passed at `1bf83b88`, the tip before the review
  fixes. At `4e1c0bf6` the check jobs pass and the test jobs are running.

## Notes for Reviewers

- **Where to start.** `workspace.py` (~140 lines) is the only code here that
  deletes anything, so its pre-move checks and its put-back on a failed move
  are the part to scrutinise. `comparison.py` (~340 lines) decides what counts
  as a change, and `comparison_report.py` (~270 lines) renders it.
- **Line count.** The baseline record (~5.2k lines) is a real capture and
  makes up over half the diff.
- **Baseline settings.** The baseline is a `devtools` record, not the `none`
  record the README asks for first. 0370's `none` record therefore needs the
  exemption removed, which can need a reboot.
- **Restoring edges.** If this PR is abandoned, restore the `blocked_by`
  edges from 0365 and 0366 to 0368.
- **Open review findings.** Three of the first review's findings are not
  addressed here. A session that stops on unremoved trash still writes no
  record of the timed runs it finished. `compare-builds` still exits 0
  whenever both records parse, so it cannot gate a script. Nothing checks for
  a live dev server or rust-analyzer using the directories `cold` deletes.
- **Deferred.** The PR #175 review's structural findings are still open and
  are listed in the plan's "Deferred to later phases". They include a
  condition value object, a `ports.py` that the new `Workspace` port would
  join, and splitting `tasks/measure.py`.
