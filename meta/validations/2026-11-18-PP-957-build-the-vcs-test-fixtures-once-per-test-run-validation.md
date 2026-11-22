---
type: "plan-validation"
id: "2026-11-18-PP-957-build-the-vcs-test-fixtures-once-per-test-run-validation"
title: "Validation Report: Build the VCS Test Fixtures Once per Test Run Implementation Plan"
date: "2026-11-22T09:47:39+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-11-18-PP-957-build-the-vcs-test-fixtures-once-per-test-run"
tags: ["build-system", "testing", "nextest", "mise"]
last_updated: "2026-11-22T09:47:39+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Build the VCS Test Fixtures Once per Test Run

### Implementation Status

✓ Phase 1: Workbench and fixture log - Fully implemented (merged, PR #247)
✓ Phase 2: The shared build step in `test:unit:cli` - Fully implemented (PR #247)
✓ Phase 3: `--filter` on `test:unit:cli` - Fully implemented (PR #247)
✓ Phase 4: Status-log builders write no absolute paths - Fully implemented (PR #249)
✓ Phase 5: Status-log sharing - Fully implemented (PR #249)
✓ Phase 6: Fixtures closure - Measured and recorded in the work item
✓ Phase 7: pytest lanes - Fully implemented (PR #253, open)
✓ Phase 8: cargo and node lanes - Fully implemented (PR #253)
✓ Phase 9: Guidance - Fully implemented (PR #253)

Every success criterion in the plan is ticked, and all 21 in the work item.

### Automated Verification Results

✓ Full local run passes: `mise run` exited 0 at revision `20ffd986`
  (after both interception fixes), in 346.8 s; formatters changed nothing
✓ `test:unit:cli`: 8,113 passed, 2 skipped; fixture summary shows the
  matrix built once (19 adoptions) and the status-log set built once
  (7 adoptions, 10 copies), with 0 private builds of either
✓ PR #253 CI: every required job green on both legs, including `test-unit`
  and the integration lanes with the design-automation runtime lane (38/38)
✓ CI check set passes: `mise run check` exited 0
✓ Phase 7–9 suites: `test_selection`, `test_pytest_lanes`, `test_mise`,
  `test_dev_integration_task`, `test_hooks_task_environment`,
  `test_cargo_and_node_lanes`, `test_e2e_docker` and
  `test_design_automation_runtime_trees` (325 passed)
✓ Manual `--filter` spot checks, run on `test:unit:tasks`,
  `test:unit:design-automation` and `test:unit:frontend`:
  - `--help` lists `--filter` and names the runner's syntax;
  - a subset runs (9 passed, 5,279 deselected; node: one test, floors skipped);
  - malformed expressions fail (pytest exit 4, node exit 1);
  - a no-match filter fails with `selected no tests` naming the lane;
  - a value holding `'`, a space and `&` reaches pytest intact.
- Phases 1–6 were verified by their own merged PRs' CI and are not re-checked
  beyond the full run above.

### Code Review Findings

#### Matches Plan:

- `Selection` renders `pytest()`, `libtest()`, `vitest()`, `node_test()` and
  `playwright()` through one quoting function; empty filters are refused.
- `tasks/test/filters.py` centralises each runner's "nothing selected"
  detection: exit codes for pytest (5) and nextest (4), text for libtest,
  Vitest, node TAP and Playwright, as the README documents.
- The four bare pytest leaves are now invoke tasks with commands identical to
  their former `mise.toml` `run`.
- `node --test-name-pattern` precedes the suite files; filtered
  design-automation runs skip their floors and print the agreed line.
- `test_every_multi_test_leaf_declares_a_filter` guards the task tree, with
  a companion guard against stale exempt names.
- `CLAUDE.md` and `tasks/README.md` carry "Running a subset of tests".

#### Deviations from Plan:

- The design-automation runtime lane was repaired and added to the
  `test:integration` roll-up (and so to the default run), closing PP-805,
  PP-778 and PP-738. Recorded under Phase 8 "As built"; beyond the plan's
  scope.
- No roll-up refusal test: Phase 3 found roll-ups ignore extra arguments, so
  `test_every_roll_up_refuses_a_filter` was dropped as the plan allowed.
- Forwarding uses `${usage_filter+…}` rather than `:+`, so `--filter ''`
  reaches the task and is refused. Intended.

#### Potential Issues:

- Re-confirmed at `20ffd986`: `--filter ''` through mise fails in invoke's parser ("Flag <Argument:
  filter_ (f)> needed value and was not given one!"), not with the task's
  `--filter needs an expression`. It is the same on every lane, including
  `test:unit:cli`. The lane still refuses rather than running everything, but
  `tasks/README.md`'s "Task options" says the task itself refuses it.
- Fixed since first validation: a hop whose `Fetch.continueRequest` is
  refused was left paused until the 300 s wall clock; it is now failed with
  `Fetch.failRequest`. Pinned by "a hop whose continuation is refused is
  failed rather than left paused".
- Fixed since first validation: a failed interception setup left the daemon
  reusing a page with no navigation policy; the browser is now closed and the
  next command launches afresh. Pinned by "a browser whose request
  interception fails to start is closed and launched afresh", which drives a
  forked daemon against the `playwright-whose-first-interception-fails`
  fixture.
- Interception covers only the first page's own target. Popups, out-of-process
  iframes, workers and websockets skip the navigation policy (bearer fails
  closed).
- The bearer is attached by request URL whatever the initiator, so a
  cross-origin frame that `fetch`es the location origin borrows it. This
  likely predates the change (`page.route` behaved the same); it needs a
  security decision rather than a fix in this PR.

- The text-matched detections (libtest `test result:`, Vitest `Tests` line,
  node file-named TAP points, Playwright "No tests found") will break silently
  on a runner output change; the README flags them for re-checking on bumps.
- `test:integration` now materialises the vendored driver and browser trees,
  adding first-run download and disk cost to every full local run.

### Manual Testing Required:

1. Live tracker contract:
  - [ ] Run `test:integration:tracker-contract --filter '<test>'` with Jira,
        Linear or Shortcut credentials; never run with live credentials.

### Recommendations:

- Raise follow-up work for popup and OOPIF coverage and for the
  initiator-blind bearer.
- Correct the README's empty-filter sentence, or catch the empty value before
  invoke's parser does.
- Move work item PP-957 from `ready` to `done` once PR #253 merges.
- Merge PR #253.
