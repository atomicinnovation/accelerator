---
type: "work-item"
id: "0275"
title: "Runtime Preflight References the Deleted package-lock Namespace"
date: "2026-09-05T21:30:25+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "medium"
blocks: ["work-item:0208"]
relates_to: ["work-item:0248", "work-item:0196"]
external_id: "PP-805"
tags: []
last_updated: "2026-09-05T21:30:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0275: Runtime Preflight References the Deleted package-lock Namespace

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The `test:integration:design-automation` lane cannot start: its preflight
(`runtime-preflight.js`) still resolves the retired `package-lock.json` lockhash
cache namespace, which the vendored-runtime migration deleted. `requireRuntime()`
throws `ENOENT` before any suite runs, so every runtime suite fails — the
pre-existing ones and the new 0206 navigation-classification cases alike.

## Context

The vendored-runtime migration (0196) removed `ensure-playwright.sh` and the
lockhash namespace, and updated `tasks/test/integration.py` to export
`ACCELERATOR_TREE_DRIVER` from the vendored `bin/trees` driver. But
`runtime-preflight.js` and the daemon fork in `daemon-runtime.test.js` were never
migrated to consume it — so the task hands the suite a real driver tree and the
suite looks somewhere else entirely. 0208's Technical Notes wrongly state "The
preflight now resolves the vendored driver tree"; it does not.

## Requirements

Reproduction:
1. Run `mise run test:integration:design-automation` on a machine with the
   vendored driver tree available.
2. Expected: `test-run.js` and `daemon-runtime.test.js` execute against the
   vendored driver and browser.
3. Actual: every suite fails at `requireRuntime()` with
   `ENOENT … package-lock.json`, because `playwrightNsRoot()` hashes that
   deleted file; the assertion also targets the retired
   `<nsRoot>/node_modules/playwright/index.js`, whereas the daemon's loader now
   imports `<driver>/node_modules/playwright-core/index.mjs`.

Fix:
- `runtime-preflight.js` resolves the driver tree from `ACCELERATOR_TREE_DRIVER`
  and asserts the vendored layout (`node_modules/playwright-core/index.mjs`); the
  `package-lock.json` lockhash logic is removed.
- The daemon fork env in `daemon-runtime.test.js` sets
  `ACCELERATOR_PLAYWRIGHT_NS_ROOT`, `NODE_PATH=<driver>/node_modules`, and
  `ACCELERATOR_DESIGN_BROWSER_EXECUTABLE`, matching the executor's production
  spawn (`cli/design-cli/src/executor.rs`).
- The integration task provisions a browser tree (`cache ensure browser`) and
  exports its executable, not only the driver.

## Out of scope

- Wiring the lane into a scheduled CI build — that is 0208.
- Changing what the suites assert.

## Acceptance Criteria

- [ ] Given a provisioned vendored runtime (driver and browser), when
      `mise run test:integration:design-automation` runs, then all runtime
      suites execute and pass — no `ENOENT`, no zero-executed lane.
- [ ] Given `package-lock.json` is absent, when the preflight runs, then it
      never references that file and resolves via `ACCELERATOR_TREE_DRIVER`.
- [ ] Given the harness forks the daemon, when it launches, then Chromium starts
      via the vendored browser executable with no lockhash namespace.
- [ ] Given the 0206 `daemon-runtime.test.js` cases (redirect hop, per-request
      scope, sub-frame isolation, out-of-window redirect, click-triggered
      navigation), when the lane runs, then they execute and pass.

## Dependencies

- Blocks: 0208 — CI wiring cannot succeed until the preflight resolves the
  runtime.
- Relates to: 0248 (duplicate CI-coverage intent), 0196 (the migration that left
  the preflight stale).

## Assumptions

- CI and local runs are permitted to materialise the vendored browser tree
  (network on a cold cache), consistent with 0208's assumptions.

## Technical Notes

- `runtime-preflight.js` — `playwrightNsRoot()` / `requireRuntime()`.
- `tasks/test/integration.py` — `_resolve_driver_tree` (driver-only today),
  `design_automation`.
- `lib/playwright-loader.js` resolves `node_modules/playwright-core/index.mjs`;
  `lib/daemon.js` `ensureBrowser` reads `ACCELERATOR_DESIGN_BROWSER_EXECUTABLE`
  and `ACCELERATOR_PLAYWRIGHT_NS_ROOT`.
- `cli/design-cli/src/executor.rs` production spawn env is the reference shape.

## Drafting Notes

- Classified as a bug (a preflight referencing a deleted file), distinct from
  0208's "omission" framing; set to block 0208 rather than duplicate it.

## References

- Related: 0208, 0248, 0196
