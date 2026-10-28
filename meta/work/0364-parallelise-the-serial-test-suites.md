---
type: "work-item"
id: "0364"
title: "Parallelise the serial test suites"
date: "2026-10-28T16:18:46+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0361"
source: "codebase-research:2026-10-28-full-build-duration-contention-and-slowness"
tags: ["build-system", "performance", "testing", "playwright", "pytest"]
last_updated: "2026-10-28T16:18:46+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0364: Parallelise the serial test suites

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Two suites run strictly serially even though most of their tests are
independent:
- **Playwright** runs all 356 tests on one worker, though 313 of them only
  read computed styles.
- **The dev-task integration suite** runs 17 tests in sequence, each spending
  about 10 s in fixed sleeps and timeouts.

Parallelising them, sized to the host, cuts each by more than half.

## Context

**Playwright (A5).** `playwright.config.ts` sets `workers: 1` for both
projects. The config comment justifies this for the kanban drag tests and the
visual snapshots. But 313 of the 356 tests belong to the `resolved-styles`
project, which reads computed styles from static showcase routes.

| Run, alone | Wall |
|---|---|
| Both projects, 1 worker (today) | 131 s |
| `resolved-styles`, 1 worker | 106 s |
| `resolved-styles`, 6 workers | 34 s (313/313 passed, one run) |
| `chromium`, 1 worker | 25 s |

`retries: 1` applies locally as well as in CI, which can hide flakes.

**Dev-task suite (A6).** `tests/integration/dev` drives real circusd. Each of
its 17 tests costs 4–20 s, mostly in fixed waits:
- `grace_kill`, 3.0 s on every stop;
- `probe_timeout`, 5.0 s whenever teardown probes an absent arbiter;
- several 0.5 s settle sleeps.

No pytest suite in the repo uses parallelism.
`uv run --with pytest-xdist pytest tests/integration/dev -n 6` passed 17/17
in 40 s, against 163 s serially, in one run.

The team decided that `default` mirrors CI, so both changes also run on CI's
runners, which have 3–4 vCPUs.

## Requirements

1. Run the `resolved-styles` project with a host-relative worker count, such
   as `workers: "50%"`. Keep the `chromium` project serial.
2. Parallelise `tests/integration/dev` with a host-relative worker count,
   such as `pytest -n auto`, or shrink `grace_kill` and `probe_timeout` for
   tests.
3. Before landing, validate both changes over repeated runs under a full
   `mise run` load.

## Acceptance Criteria

- [ ] Given a warm workspace, when `test:e2e:visualiser` runs alone, then it
      completes in under 75 s (131 s today).
- [ ] Given the `chromium` project, when it runs, then it still uses exactly
      one worker.
- [ ] Given a warm workspace, when `test:integration:dev` runs alone, then
      it completes in under 60 s (163 s today).
- [ ] Given 10 consecutive full `mise run` executions, when they finish,
      then both suites pass every time without consuming a Playwright retry.
- [ ] Given a CI runner with 3–4 vCPUs, when the suites run, then the worker
      counts derive from the host's CPU count rather than a fixed number.

## Open Questions

- Is `pytest-xdist` acceptable as a new dev dependency, or should the dev
  suite shrink its test timeouts instead?
- Should `retries: 1` stay on for local runs, given that it can hide flakes?
- Does any `resolved-styles` spec read state that the `chromium` drag tests
  mutate? The config comment says they must stay independent.

## Dependencies

- Blocked by: none
- Blocks: none

## Assumptions

- The `resolved-styles` specs are read-only, as the config comment and the
  research describe them.

## Technical Notes

- Worker settings: `cli/visualiser/frontend/playwright.config.ts:9-14`.
- Dev suite command: `tasks/test/integration.py:36`.
- Driver waits: `tests/integration/dev/dev_integration_driver.py:191-196`.
- The Playwright projects could run as two invocations or be split in the
  config. Workers are set globally in Playwright, not per project.

## Drafting Notes

- Grouped A5 and A6 because both use the same approach: host-relative
  workers, then repeated-run flake validation. Agreed in conversation.
- Replaced the fixed 6 workers from the experiments with host-relative
  counts, following the mirror-CI decision recorded in the research.
- The 75 s and 60 s targets are proposals derived from about 60 s projected
  for Playwright and 40 s measured for the dev suite.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- Related: 0361
