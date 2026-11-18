---
type: "pr-description"
id: "230"
title: "[0364] Print each CI runner's CPU counts before the integration tests"
date: "2026-11-16T22:47:11+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0364"
parent: "work-item:0364"
relates_to: ["plan:2026-11-15-0364-parallelise-the-serial-test-suites"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/230"
pr_number: 230
tags: ["build-system", "ci", "testing", "pytest-xdist", "playwright"]
revision: "1fcfe2692c4dcb6187701a96fe5a0fc3dd10f202"
repository: "accelerator"
last_updated: "2026-11-16T22:47:11+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0364] Print each CI runner's CPU counts before the integration tests

## Summary

A new `mise run host:cpu-counts` task reports the CPU counts that size the
parallel test workers. Each integration job now runs it before its tests.
The 0364 work runs both suites with worker counts derived from the host's
CPUs. Until now the CI logs showed only the count that pytest-xdist picked,
so Phase 3 couldn't record the runners' logical counts next to it. Phase 7
needs both to judge whether the worker counts suit each runner.

## Changes

- **`tasks/host.py`** (new) adds the `host.cpu-counts` task. It prints
  four counts:
  - psutil's physical cores, which xdist's `-n auto` takes;
  - psutil's logical CPUs;
  - `os.process_cpu_count()`;
  - Node's `availableParallelism()`, which Phase 6's Playwright workers
    will follow.

  A count psutil can't determine is printed as `unknown`.
- **Registration**: the module is added to `tasks/__init__.py`, and
  `mise.toml` gets `host:cpu-counts`, which depends on
  `deps:install:python` for psutil. It reports rather than checks, so it
  stays out of `check` and `default`.
- **`.github/workflows/main.yml`**: `test-integration` runs
  `mise run host:cpu-counts` in a "Report the runner's CPU counts" step,
  before "Run integration tests".
- **The plan**: Phase 3's worker-count item is ticked, with the measured
  counts.

The first commit printed the counts from an inline script in the
workflow. The second moves that script into the task, because the step is
meant to stay.

## Context

- Work item: `meta/work/0364-parallelise-the-serial-test-suites.md`
- Plan: `meta/plans/2026-11-15-0364-parallelise-the-serial-test-suites.md`
  (Phase 3's open worker-count item, and Phase 7)
- Independent of #229. Both edit the plan, but in different sections.

## Measured counts

From this PR's run 43965178907:

| Host | Image | xdist banner | Physical | Logical | `os.process_cpu_count` | Node |
|---|---|---|---|---|---|---|
| engywook (local) | — | 16 | 16 | 16 | 16 | 16 |
| `ubuntu-26.04` | 20261111.196.1 | 2 | 2 | 4 | 4 | 4 |
| `macos-26-arm64` | 20261112.0504.1 | 3 | 3 | 3 | 3 | 3 |

- **On ubuntu**, `-n auto` takes half the runner's vCPUs, because it counts
  physical cores, while Playwright will see all four. If Phase 7's
  comparison fails on ubuntu, `-n logical` is the first lever.
- **On macOS** all four counts agree.

## Testing

- [x] `tests/unit/tasks/test_host_cpu_counts.py` (new), with only the
      `node` call stubbed:
  - the three host counts print as positive integers;
  - Node's figure is the one the task reports, and the task asks Node
    for `availableParallelism()`;
  - an unknown physical count prints as `unknown`.
- [x] `tests/unit/tasks/test_workflows.py`: the step comes before "Run
      integration tests" and runs `mise run host:cpu-counts`.
- [x] All the new tests failed before the change.
- [x] `mise run host:cpu-counts` on engywook prints 16 for every count.
- [x] Bare `mise run` exits 0 after each commit.
- [x] CI run 43965178907: the step succeeded on both runners and printed
      the counts above.

## Notes for Reviewers

- **The step only runs in `test-integration`.** `test-e2e` runs on the
  same runner types, so these counts apply to it too. Phase 6 can add the
  step there if the Playwright track wants its own record.
- **The plan's `last_updated` is unchanged** so that this PR and #229
  don't conflict on that line.
