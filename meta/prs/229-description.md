---
type: "pr-description"
id: "229"
title: "[0364] Stop retrying Playwright tests and keep every failure's trace"
date: "2026-11-16T21:04:33+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0364"
parent: "work-item:PP-956"
relates_to: ["plan:2026-11-15-PP-956-parallelise-the-serial-test-suites"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/229"
pr_number: 229
tags: ["visualiser", "testing", "playwright", "flakiness"]
revision: "2f03844c4524925fd41118a117468cf17caadcf7"
repository: "accelerator"
last_updated: "2026-11-16T21:04:33+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0364] Stop retrying Playwright tests and keep every failure's trace

## Summary

This PR implements Phase 5 of the 0364 plan, the second phase of the
Playwright track. The visualiser's Playwright config now never retries a
failing test, and keeps the trace of every failure, with or without `CI`.
Until now `retries: 1` let a flaky test pass on its second attempt, and
the trace was recorded only on that retry.

The suite still runs on one worker. Any flake that losing the retry
exposes therefore shows up here, apart from any flake that Phase 6's
parallel workers might cause. Five full `mise run` executions found none.

## Changes

- **`playwright.config.ts`**: `retries: 1` becomes `retries: 0`, and
  `trace: "on-first-retry"` becomes `trace: "retain-on-failure"`.
- **`scripts/playwright-config.test.ts`** (new): a Vitest test that loads
  the config with `CI` unset and with `CI=true`. It asserts that the
  config never retries and keeps the trace of a failing test.
  - `defineConfig` is mocked as identity, so loading the config needs no
    Playwright runner.
  - Each case resets the module registry, so every load re-reads `CI`.
  - It runs under the default `jsdom` environment, because the shared
    setup writes to `window`.
- **The plan** records:
  - Phase 5's runs, per-file durations and `fullyParallel` decision;
  - that #222 and #225 merged;
  - the dev suite's CI before window, which was not met (see Notes).

## Context

- Work item: `meta/work/PP-956-parallelise-the-serial-test-suites.md`
- Plan: `meta/plans/2026-11-15-PP-956-parallelise-the-serial-test-suites.md`
  (Phase 5)
- Based on `main`, after #225 (Phase 4).
- This must merge before Phase 6 (host-relative workers). That way
  Phase 6's CI before window already includes the retries policy, and the
  CI comparison judges parallelism on its own.

## Red step

The config test ran before the config changed. All four cases failed on
the old values: `retries` was `1` and `use.trace` was `"on-first-retry"`,
under both `CI` values. After the change all four pass.

## Deliberate failure keeps its trace

I pointed `smoke.spec.ts`'s URL assertion at a wrong route and ran it on
its own. The run reported `1 failed` with no retry. It left
`test-results/smoke-app-loads-and-redirects-to-library-chromium/trace.zip`,
alongside `error-context.md`. I then restored the spec with `jj restore`,
so this PR doesn't contain it.

## Five serial runs under `mise run` load

All five full `mise run` executions exited 0 on `2d770eb2`, this PR's
config commit before it was rebased onto #225. The rebase changed only
the plan, not the code.

| Run | `mise run` wall | `test:e2e:visualiser` | Result |
|---|---|---|---|
| 1 | 502 s | 250.2 s | 355 passed, 1 skipped |
| 2 | 360 s | 256.4 s | 355 passed, 1 skipped |
| 3 | 445 s | 291.3 s | 355 passed, 1 skipped |
| 4 | 305 s | 194.0 s | 355 passed, 1 skipped |
| 5 | 337 s | 201.2 s | 355 passed, 1 skipped |

**No flakes were found.** The plan's known candidate was a first
navigation failing with `net::ERR_ABORTED` under full load. It never
appeared, so nothing needed a fix. The one skip is `legacy-schema`'s
existing lookup, which 0364 leaves alone.

## Per-file durations and `fullyParallel`

Playwright hands whole files to its workers. So the longest
`resolved-styles` file sets a floor on Phase 6's parallel wall time.

These figures come from one serial JSON-reporter run: 151.5 s wall time,
with the tests summing to 147.6 s.

| Project | File | Tests | Duration |
|---|---|---|---|
| `resolved-styles` | `dev-design-system-code-block-resolved-colours` | 56 | 25.5 s |
| `resolved-styles` | `aside-row-resolved-colours` | 31 | 20.8 s |
| `resolved-styles` | `non-regression-glyph-consumers` | 60 | 17.5 s |
| `resolved-styles` | `dev-design-system-glyph-resolved-fill` | 30 | 13.4 s |
| `resolved-styles` | `detail-eyebrow-resolved-colours` | 32 | 10.1 s |
| `resolved-styles` | `dev-design-system-chip-resolved-colours` | 20 | 9.3 s |
| `resolved-styles` | `fixture-coverage` | 30 | 8.6 s |
| `chromium` | `kanban-conflict` | 1 | 6.6 s |

- **The rest.** All the other files take under 5 s each.
- **Project totals.** `resolved-styles` sums to 123.5 s and `chromium` to
  24.1 s.
- **Decision.** The longest file, 25.5 s, fits well inside the 75 s
  target minus about 4 s of server start-up. So Phase 6 starts without
  `fullyParallel`, which stays in reserve as its first tuning lever.

## Testing

- [x] The config test fails before the change and passes after:
      `npm --prefix cli/visualiser/frontend test -- scripts/playwright-config.test.ts`.
- [x] `mise run test:unit:frontend`: 2619 passed, on this branch's tip.
- [x] `mise run frontend:check` passes, on this branch's tip.
- [x] `mise run test:e2e:visualiser`: 355 passed and 1 skipped in
      161.3 s, on one worker, on this branch's tip.
- [x] A deliberately failing test leaves its `trace.zip` and isn't
      retried. The change was then reverted.
- [x] Five `mise run` executions are green, with no flakes.
- [x] Anti-masking: the diff adds no `skip`, `fixme` or per-test
      `retries`.

## Notes for Reviewers

- **The global setup prints to stdout.** It prints a `[e2e] BASE_URL …`
  line before Playwright's own output. So
  `npx playwright test --reporter=json > results.json` doesn't produce
  valid JSON. Phase 6's overlap check needs that line dropped, or
  `PLAYWRIGHT_JSON_OUTPUT_NAME` set instead. This is noted in the plan.
- **The dev suite's CI before window wasn't met.** #222 merged through
  #225's merge commit, because #225 was stacked on it. At that point only
  three `main` runs fell after #221's merge, not ten, and the macOS runner
  image changed between them. The plan now records those three runs and
  their `test:integration:dev` times. It also records that Phase 7 runs
  the paired comparison for the dev suite. This PR changes neither suite's
  behaviour for that comparison.
- **Out of scope.** Worker counts are still 1, and the stale `workers`
  comment stays. Both are Phase 6.
