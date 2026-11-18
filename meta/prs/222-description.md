---
type: "pr-description"
id: "222"
title: "[0364] Run the dev-task suite across xdist workers"
date: "2026-11-16T10:16:49+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0364"
parent: "work-item:0364"
relates_to: ["plan:2026-11-15-0364-parallelise-the-serial-test-suites"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/222"
pr_number: 222
tags: ["build-system", "testing", "pytest", "pytest-xdist", "dev-task", "performance"]
revision: "30fc6872c9dd3afeee39c59687af0fd272ab6ea1"
repository: "accelerator"
last_updated: "2026-11-16T12:18:46+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0364] Run the dev-task suite across xdist workers

## Summary

This PR implements Phase 3 of the 0364 plan. It is stacked on #221, which
shortened the dev integration suite's waits. It declares `pytest-xdist`
and runs `test:integration:dev` with `-n auto`. In isolation the suite
now takes a median of **7.9 s** on engywook, against a 60 s target. It
took 61 s serially after #221, and 163 s before 0364 started.

The suite was already safe to parallelise. Each test owns its
`tmp_path` workspace, its IPC sockets are keyed by workspace, and no test
binds a port. #220's clean-stop checks make any escalation caused by
parallel load fail a test instead of passing unnoticed.

## Changes

- **`tasks/test/integration.py`**: `dev` now runs
  `uv run pytest tests/integration/dev -n auto -v`.
- **`tests/unit/tasks/test_dev_integration_task.py`** (new): asserts that
  the task's command includes `-n auto`. It follows the argv-test pattern
  of `test_hooks_task_environment.py`. It failed before the task change
  and passes after it.
- **`pyproject.toml` / `uv.lock`**: `pytest-xdist>=3` joins the `dev`
  group next to `pytest-mock`. The relock adds `pytest-xdist` 3.8.0 and
  `execnet` 2.1.2.
- **`meta/measurements/`**: the isolated timing record and the two
  load-comparison records.
- **The plan**: Phase 3's automated checks are ticked, and the load
  comparison's result is recorded.

## Context

- Work item: `meta/work/0364-parallelise-the-serial-test-suites.md`
- Plan: `meta/plans/2026-11-15-0364-parallelise-the-serial-test-suites.md`
  (Phase 3)
- Base: #221 (Phase 2), stacked on #220 (Phase 1). This PR must merge
  after both.

## Testing

- [x] The new argv test failed before the task change and passes after:
      `mise run test:unit:tasks`, 4781 passed.
- [x] The lock is consistent: `uv sync --all-groups --frozen`.
- [x] `mise run test:integration:dev` passes in parallel. The banner shows
      `created: 16/16 workers`, and 21 tests pass in 7.9 s.
- [x] `mise run build-system:check` passes.
- [x] Timing target met. `mise run measure:build --isolate
      test:integration:dev --cache-state warm-no-op --runs 5
      --host-settings devtools --keep` reports a median of 7.9 s (min 7.8,
      max 8.8), and no task failed in any run. The record is
      `meta/measurements/build_2026-11-16T004012Z_engywook_warm-no-op_devtools_isolated.json`.
- [x] Load comparison of `-n auto` against `-n 8` (not a gate; details
      below).
- [x] Ten consecutive counted `mise run` executions on `d29d7621`, whose
      parent is `main`'s #221 merge commit `e1f83907`. All ten exited 0, so
      no failure needed naming. The working copy stayed clean and the
      commit didn't move (details below).
- [x] No dev-suite flake appeared in any parallel run: no `stop_cleanly`
      failure, no null recorded PID and no missed `reused`. The diff adds
      no `skip`, rerun or flaky marker.
- [ ] Worker counts on both CI runners, next to psutil's physical and
      logical counts. The xdist banners are recorded below. The CI logs
      don't print psutil's counts, so the logical counts are still to be
      recorded.
- [ ] The CI before window for `test-integration` on each OS: the last 10
      main runs after #221 merges, with the median and IQR% of
      `test:integration:dev`'s "Finished in" line and the runner image
      version.

### Counted runs

| Run | `mise run` exit | Wall time | Dev suite (16 workers) |
|---|---|---|---|
| 1 | 0 | 361 s | 21 passed in 14.66 s |
| 2 | 0 | 320 s | 21 passed in 14.64 s |
| 3 | 0 | 270 s | 21 passed in 12.65 s |
| 4 | 0 | 300 s | 21 passed in 11.58 s |
| 5 | 0 | 293 s | 21 passed in 10.85 s |
| 6 | 0 | 319 s | 21 passed in 13.63 s |
| 7 | 0 | 280 s | 21 passed in 10.33 s |
| 8 | 0 | 278 s | 21 passed in 10.84 s |
| 9 | 0 | 315 s | 21 passed in 14.55 s |
| 10 | 0 | 345 s | 21 passed in 16.70 s |

0370 was not recording during these runs or the measurements.

### Worker counts

| Host | Runner image | xdist banner | psutil physical / logical | `test:integration:dev` |
|---|---|---|---|---|
| engywook | — | 16/16 | 16 / 16 | 7.9 s isolated |
| ubuntu | `ubuntu-26.04` 20261111.196.1 | 2/2 | not printed | 73.9 s |
| macOS | `macos-26-arm64` 20261102.0473.1 | 3/3 | not printed | 49.5 s |

The CI figures come from #222's run 43848534569. `-n auto` takes
psutil's physical count, so the banners give the CI physical counts. On
ubuntu, 2 workers is probably half the runner's vCPUs. If Phase 7's
comparison fails there, `-n logical` is the first lever to try.

### Load comparison

On engywook (16 cores, no SMT), `-n auto` and `-n logical` both resolve
to 16, so the comparison is `-n auto` against `-n 8`. The second set of
runs set `PYTEST_XDIST_AUTO_NUM_WORKERS=8`. Both were non-isolated `mise
run measure:build --cache-state warm-no-op --runs 5 --host-settings
devtools --keep` runs, back to back on commit `8d7966fb`, which does not
contain the Playwright worker phase. Every run succeeded, and the logs
show 16/16 and 8/8 workers.

| Median (IQR) | `-n auto` | `-n 8` |
|---|---|---|
| `default` wall time | 225.3 s (2.7) | 228.1 s (4.9) |
| `test:integration:dev` | 10.6 s (0.9) | 11.3 s (0.0) |
| `test:unit:cli` | 194.4 s (8.4) | 200.3 s (11.0) |
| `test:e2e:visualiser` | 147.5 s (0.7) | 158.7 s (12.3) |
| `test:unit:tasks` | 102.0 s (3.2) | 111.6 s (39.7) |

`-n 8` would have to beat `-n auto`'s `default` median by more than the
larger IQR, and it is slower instead. So `-n auto` stays, and nothing
restarts the 10-run count. The three slowest sibling tasks are no faster
with fewer dev-suite workers either. The idle load was 20.7% before the
`-n auto` run and 60.0% before the `-n 8` run. The records are
`meta/measurements/build_2026-11-16T081243Z_engywook_warm-no-op_devtools.json`
and
`meta/measurements/build_2026-11-16T083530Z_engywook_warm-no-op_devtools.json`.

## Notes for Reviewers

- **The `uv.lock` revision drops from 5 to 3.** The relock used the uv
  that `mise.toml` pins (0.11.6), which writes revision 3. Revision 5 came
  in with the Dependabot bump of 9 November, which used a newer uv.
  `--frozen` reads both, and nothing runs `--locked`. A later Dependabot
  bump will probably move it back to 5. That churn is harmless, but
  bumping the pinned uv would end it.
- **Physical against logical cores.** `-n auto` uses physical cores when
  psutil is importable, but Playwright counts logical CPUs. The two can
  differ on CI runners. The CI banners will show it, and
  `PYTEST_XDIST_AUTO_NUM_WORKERS` is the per-host override.
- **What a flake under load would look like.** If the 0.5 s `grace_kill`
  or the 2.0 s `probe_timeout` is too tight under parallel load, it shows
  up as one of these:
  - a `stop_cleanly` failure that names an arbiter's `terminate` or
    `kill`;
  - a null recorded PID;
  - a missed `reused`.

  The fix is to raise the budget, not to weaken the assertion. No flake
  appeared in the parallel runs so far.
- **What's next.** Phase 7 judges this PR's CI effect against the before
  window. A revert would remove only this PR's change and keep #221's
  shorter waits.
