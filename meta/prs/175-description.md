---
type: "pr-description"
id: "175"
title: "[0368] Add an operator-run full-build timer with host analysis"
date: "2026-10-31T19:03:13+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0368"
parent: "work-item:0368"
relates_to: ["work-item:0361", "work-item:0362", "work-item:0363", "work-item:0364", "work-item:0365", "work-item:0366", "work-item:0367", "work-item:0369"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/175"
pr_number: 175
tags: ["build-system", "measurement", "performance", "mise", "macos"]
revision: "244760759693f68adb242796d2400dbf4287d1f3"
repository: "accelerator"
last_updated: "2026-10-31T19:03:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0368] Add an operator-run full-build timer with host analysis

## Summary

Adds `mise run measure:build`, an operator-run, macOS-only task that times a
full `mise run default` across one priming run and N timed runs, and writes a
JSON record of per-task figures, CPU contention and host-process CPU. It is the
instrument the 0361 build-speed epic needs: 0362–0367 are blocked on a warm
no-op baseline that this harness records. This PR covers phases 1–4 of the 0368
plan; the baseline, isolation mode, cache states, `measure:compare-builds` and
host guidance follow in later phases.

## Changes

- **`measure:build` task** (`tasks/measure.py`, `mise.toml`). Flags: `--runs`
  (default 3), `--keep`, `--host-settings none|devtools|devtools+ide-exclusion`
  and repeatable `--ide-pattern`. It refuses a non-macOS host before writing
  anything, and exits 0 whenever it writes a record, even if a measured task
  failed. It stays out of `check` and `default`, and the closure guard in
  `tests/unit/tasks/test_mise.py` now names it.
- **Task timeline analysis** (`tasks/shared/build_measurement/timeline.py`,
  `task_graph.py`, `fetch_failures.py`). Parses mise's `--output prefix` log
  against `mise tasks --json`. Per task it gives duration, queueing delay,
  cargo unit counts, lock wait, nextest phase and an `outcome` of `succeeded`,
  `failed`, `skipped`, `not-run` or `silent` (a roll-up). It also classifies
  launcher, advisory-database and npm-registry fetch failures.
- **Host activity analysis** (`cpu_load.py`, `tracked_processes.py`,
  `samplers.py`, `across_runs.py`). `iostat` and `ps` sample once a second in
  the background. CPU busy is bucketed into 30-second windows, and CPU-seconds
  are attributed to `syspolicyd`, `XprotectService`, the IDE and `fsnotifier`.
  Each figure carries a `presence` of `measured`, `flagged` or `absent`, and a
  spread across runs.
- **Provenance and record** (`provenance.py`, `history.py`, `host.py`,
  `record.py`, `record_model.py`). The measured commit is the nearest
  first-parent commit that changes anything outside `meta/measurements/`, so
  committing a kept record does not move it. It compares each commit with its
  first parent because jj's `files()` treats a clean merge as empty. Records
  are validated with `pydantic` (new, pinned exactly to `2.14.2`) and carry a
  `conditions` comparability key and a schema version.
- **Session orchestration** (`session.py`, `measured_build.py`,
  `build_runner.py`, `build_group.py`, `session_lock.py`, `wiring.py`). The
  domain runs over injected ports, with real ports composed in one place. It
  holds a per-workspace session lock, runs the build in its own process group
  and records that group's id in `build.pgid` so a later session can detect an
  orphan. On Ctrl-C it reaps the whole group. The build runner drops inherited
  `MISE_TASK_*` and sets `MISE_TASK_TIMINGS=1`. Without that, an enclosing
  `mise run` silences every `Finished in` line.
- **Shared extractions, no behaviour change.**
  - Signal unwinding moves into `tasks/shared/signals.py`.
  - Tree and process-group reaping move into `tasks/shared/processes.py`; the
    dev host server now calls `reap_tree`.
  - Diagnostic runners move into `tasks/shared/diagnostics.py`, and
    `repository_kind` into `tasks/shared/repository.py`.
- **Docs and meta.** `tasks/README.md` documents build timing, run directories,
  how to read a record, schema evolution and the operator protocol for baseline
  runs. The PR also adds the 0361–0369 work items, the two research documents,
  the 0368 plan and the plan and work-item reviews.

## Context

- Work item: `meta/work/0368-build-performance-measurement-and-host-guidance.md`
  (parent epic `meta/work/0361-speed-up-the-full-build-without-losing-confidence.md`)
- Plan: `meta/plans/2026-10-30-0368-build-performance-measurement-harness.md`,
  with phases 1–4 implemented here
- Research: `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`,
  `meta/research/codebase/2026-10-30-0368-build-performance-measurement-harness.md`

## Testing

- [x] Harness, signal, process, repository, measure and mise-guard tests pass
  locally: 2374 passed, 2 skipped
  (`uv run pytest tests/unit/tasks/shared/build_measurement tests/unit/tasks/shared/test_{signals,processes,repository}.py tests/unit/tasks/test_{measure,mise}.py`)
- [x] `mise run build-system:check` passes (ruff, format, pyrefly strict: 0 errors)
- [x] Full local CI mirror `mise run` passed at the end of each phase, as
  recorded in the plan (not re-run for this description)
- [x] Analysis is checked against a real captured warm no-op run on the
  reference host (`test_captured_run.py` over the `warm-no-op-*` golden
  fixtures)
- [x] Manual on the reference host: `--runs 1 --keep` writes the record to both
  places with every field. Ctrl-C (single and double) leaves no `mise`, `cargo`,
  `nextest` or `iostat` process and writes no record. `tail -f` on the printed
  log follows the build.
- [ ] Tasks unit lane on both CI OSes (ubuntu and macOS). Real-`jj`/`git` tests
  are gated by `require`, which fails in CI rather than skipping, so they need
  this PR's CI run.

## Notes for Reviewers

- **Where to start.** `session.py` holds the flow, `timeline.py` the densest
  parsing, and `record_model.py` the persisted shape.
- **Fixtures.** `warm-no-op-build.golden.gz` (~400 KB) and
  `warm-no-op-tasks.golden.json` (~5.6k lines) are real captures, not
  hand-written. They make up most of the line count.
- **Known limits.** Per-process CPU is a lower bound: one-second sampling
  misses short-lived processes, and the tail after a build ends is not counted.
  Raw run directories under `.accelerator-measure/build/` are never pruned.
- **Out of scope here.** Linux support, per-task CPU attribution and any CI
  gate. The baseline record and the 0362–0367 `blocked_by` edits come in
  phase 5.
