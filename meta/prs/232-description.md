---
type: "pr-description"
id: "232"
title: "[0364] Run the Playwright projects concurrently on host-relative workers"
date: "2026-11-17T08:22:16+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0364"
parent: "work-item:0364"
relates_to: ["plan:2026-11-15-0364-parallelise-the-serial-test-suites", "work-item:0369"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/232"
pr_number: 232
merge_commit: "a646f52775df912b9b4de1bc7d2516a428c09527"
tags: ["visualiser", "testing", "playwright", "performance"]
revision: "67d620ed634abbfa42927133c96842215e9cb72e"
repository: "accelerator"
last_updated: "2026-11-18T10:01:54+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0364] Run the Playwright projects concurrently on host-relative workers

## Summary

This PR implements Phase 6 of the 0364 plan, the last phase of the
Playwright track. The visualiser's Playwright suite no longer runs on one
worker. `resolved-styles` gets half the host's logical CPUs and runs
alongside `chromium`, which keeps a single worker because its drag specs
write shared fixtures in place. In isolation `test:e2e:visualiser` now
takes a median of **47.0 s** on engywook, against a 75 s target. It took
170.5 s on one worker after #229.

Phase 4 (#225) made this safe: no `resolved-styles` probe measures an
element that comes from a fixture a `chromium` spec writes. Phase 5 (#229)
turned retries off, so any flake that parallelism causes fails a run
instead of passing on a retry. None appeared.

## Changes

- **`scripts/worker-allocation.ts`** (new): `allocateWorkers(logicalCpus)`
  returns the three counts the config needs.
  - `resolved-styles` gets `max(1, ⌊N × 0.5⌋)` workers. The share,
    `RESOLVED_STYLES_SHARE`, is the one tuning lever.
  - `chromium` gets `CHROMIUM_WORKERS`, which is 1.
  - The global cap is their sum, so `chromium` always has its worker.
- **`scripts/worker-allocation.test.ts`** (new): 1, 3, 4 and 16 CPUs. They
  cover the floor of one worker, rounding down, and `chromium`'s single
  worker.
- **`playwright.config.ts`**:
  - It computes `allocateWorkers(availableParallelism())` once. The
    allocation sets the top-level `workers` and each project's `workers`.
  - The stale comment above `workers: 1` goes. It cited the
    visual-regression specs, which now live in
    `playwright.docker.config.ts`.
  - Each project's comment now states the invariant that matters while
    the projects run together: `resolved-styles` may not probe a fixture
    `chromium` writes, and `chromium` runs on one worker because its specs
    write shared fixtures in place.
- **`scripts/playwright-config.test.ts`**: under both `CI` values, with
  `availableParallelism` faked to 4, the config caps the run at 3 workers,
  gives `resolved-styles` 2 and `chromium` 1.
- **`meta/measurements/`**: the isolated timing record and the two
  load-measurement records.
- **The plan**: Phase 6's checks are ticked, and its measurements, counted
  runs and CI before window are recorded.

Per-host tuning is deliberately a code change. Unlike the dev suite's
`PYTEST_XDIST_AUTO_NUM_WORKERS`, the share has no environment override. A
different share changes what the counted runs and the CI windows
measured, so it belongs in a reviewed commit that restarts the count.

## Context

- Work item: `meta/work/0364-parallelise-the-serial-test-suites.md`
- Plan: `meta/plans/2026-11-15-0364-parallelise-the-serial-test-suites.md`
  (Phase 6)
- Based on `main` at `7ba3859c` (#233), after #229 (Phase 5) and #230.
  Validated on `main` at `902556ac` (#228); see "The count stands after
  two rebases".
- Phase 7 judges this PR's CI effect after it merges, so the suite hasn't
  landed until then.

## Red step

- The allocation test ran before `worker-allocation.ts` existed, and
  failed because the module was missing.
- The config test's six new cases ran before the config changed. They
  failed on the values: the top-level `workers` was `1`, and each
  project's `workers` was `undefined`.
- After the change, all 26 script tests pass.

## Faking the CPU count

The plan said to `vi.mock("node:os")`. That mock never reaches the config.
The frontend's Vitest runs under `jsdom`, so Vite applies its web
transform. That rewrites the config's `node:os` import to
`__vite-browser-external:node:os` and reads `availableParallelism` off
that module's `default`, which is the real `os` object. So the test
instead spies on that object with
`vi.spyOn(os, "availableParallelism").mockReturnValue(4)` in a
`beforeEach`. On engywook, the real count of 16 would give 9 and 8
workers, so passing at 3 and 2 proves the spy is what the config reads.

## The projects overlap, and `chromium` stays serial

From a JSON-reporter run with the task's environment, the plan's `jq`
check prints two lines:
- whether no two `chromium` tests overlap in time;
- whether some `chromium` test overlaps a `resolved-styles` test.

| | No two `chromium` tests overlap | `chromium` overlaps `resolved-styles` | Wall time |
|---|---|---|---|
| Before the config change | `true` | `false` | 188 s |
| After | `true` | `true` | 46 s |

Both runs had 355 expected, 0 unexpected, 1 skipped and 0 flaky.

## Isolated timing

`mise run measure:build --isolate test:e2e:visualiser --cache-state
warm-no-op --runs 5 --host-settings devtools --keep` reports a median of
**47.0 s** (min 46.4, max 54.3), and every run succeeded. It ran on
`83572065`, this PR's config commit before the rebase onto #228, which
changed no code. The idle load before it was 67.0%. The record is
`meta/measurements/build_2026-11-16T230609Z_engywook_warm-no-op_devtools_isolated.json`.

`fullyParallel` wasn't needed.

## Load measurement (not a gate)

Two non-isolated runs of `mise run measure:build --cache-state warm-no-op
--runs 5 --host-settings devtools --keep`, back to back:
- "without" ran on `main` at `902556ac`, which already contains the dev
  suite's workers (#222);
- "with" ran on this PR's config commit `399edd36`, rebased onto it.

Both working copies matched at start and end. The idle load was 36.9%
before "without" and 60.0% before "with".

| Figure | Without | With |
|---|---|---|
| `default` wall time | 274.3 s, IQR 12.6 | 276.7 s, IQR 2.3 |
| `test:unit:cli` | 202.0 s, IQR 6.2 | 255.0 s, IQR 5.1 |
| `test:e2e:visualiser` | 199.4 s, IQR 7.8 | 87.5 s, IQR 6.6 |
| `test:unit:tasks` | 124.3 s, IQR 26.1 | 189.3 s, IQR 9.1 |
| `test:integration:research` | 79.2 s, IQR 15.0 | 84.4 s, IQR 10.9 |

- **`default` is unchanged.** Its median rises by 2.4 s, inside the larger
  IQR.
- **The saving moves onto other lanes.** The e2e suite saves about 112 s.
  `test:unit:cli` and `test:unit:tasks` each slow by 50–65 s while it
  runs alongside them, and they now set the critical path. That
  contention belongs to 0369. The plan records it there only if `default`
  worsens by more than the larger IQR, which it didn't.
- **Unrelated-task failures**, all in "without" run 4:
  - `test:unit:cli`: `research-cli`'s `arxiv_pacing`
    `a_killed_calls_request_still_spaces_the_next` measured 2.983 s
    against its 3 s spacing assertion.
  - `lint:server:check`: the server's build script found no
    `frontend/dist/index.html` while `build:frontend` was rebuilding it.
- **Parsing artefacts.** The records also flag `deps:install:public-api`
  ("without" run 2) and `notices:check` ("with" run 4) as failed with no
  exit status. Each task's last log line was interleaved with another
  task's output, and both runs exited 0.
- **Records:**
  `meta/measurements/build_2026-11-17T063700Z_engywook_warm-no-op_devtools.json`
  and
  `meta/measurements/build_2026-11-17T070532Z_engywook_warm-no-op_devtools.json`.

### Two earlier attempts were discarded

- **The base moved.** In the first attempt, a `jj git fetch` and a
  `jj rebase -b @ -d 'trunk()'` from outside the session moved the
  "without" working copy from `ba876153` to `ed656159` mid-run.
- **Another session killed the browsers.** In the second, 9 of the 356
  e2e tests failed within 0.4 s of each other, one on every worker, with
  "Target page, context or browser has been closed". Another Claude Code
  session, working in the main checkout, ran
  `pkill -f chrome-headless-shell` at 06:27:02 UTC. macOS logged `pkill`
  launching at 06:27:04.137, and every one of the suite's browsers began
  exiting 42 ms later on SIGTERM (exit 143). The traces show all nine
  dying at once, including two tests that had already passed their
  assertions. This is outside interference, not a flake in the suite.

## Ten counted runs

Ten consecutive `mise run` executions on `c8cf7e5e`, whose code is
`399edd36` on top of `main` at `902556ac`. The commits above `399edd36`
touch only `meta/`. The working copy stayed clean, and the commit didn't
move. All ten count under Requirement 7.

| Run | Exit | Wall | Failed tasks | `test:e2e:visualiser` (9 workers) | Dev suite |
|---|---|---|---|---|---|
| 1 | 0 | 270 s | — | 355 passed, 1 skipped (1.2m) | 21 passed |
| 2 | 0 | 248 s | — | 355 passed, 1 skipped (1.1m) | 21 passed |
| 3 | 0 | 253 s | — | 355 passed, 1 skipped (1.2m) | 21 passed |
| 4 | 0 | 293 s | — | 355 passed, 1 skipped (1.3m) | 21 passed |
| 5 | 0 | 310 s | — | 355 passed, 1 skipped (1.1m) | 21 passed |
| 6 | 0 | 267 s | — | 355 passed, 1 skipped (1.1m) | 21 passed |
| 7 | 0 | 265 s | — | 355 passed, 1 skipped (1.1m) | 21 passed |
| 8 | 1 | 124 s | `test:unit:tasks` | 355 passed, 1 skipped (59.4s) | 21 passed |
| 9 | 0 | 246 s | — | 355 passed, 1 skipped (1.1m) | 21 passed |
| 10 | 0 | 218 s | — | 355 passed, 1 skipped (58.0s) | 21 passed |

Run 8 failed only `test:unit:tasks`, on a timing test of the measurement
harness. `test_the_build_stamps_lines_in_arrival_order` expects two lines
printed 0.5 s apart to be stamped more than 0.3 s apart. Under load the
first was stamped late, which left 0.25 s. It is unrelated to the suite.

The one skip is `legacy-schema`'s existing lookup, which 0364 leaves
alone.

### The count stands after two rebases

After the runs, this branch was rebased twice, both times by a sync
process outside this work:
1. onto `f7a8bd22`, a `[skip ci]` version bump;
2. onto `7ba3859c` (#233), which changes the visualiser server's
   idle-timeout parsing (`config.rs`), `tasks/notices.py` and its test.

The ten runs were not repeated:
- Neither change adds or removes a task in `default`.
- The e2e harness never sets the idle timeout, so the server runs on its
  default. A fault in parsing it would stop the server starting on every
  run, not cause a flake.
- This PR's CI on `aabc3ae1`, on the new base, passed every job. The e2e
  suite passed 355 with 1 skipped on both OSes: 149.03 s on 3 workers on
  ubuntu, and 296.68 s on 2 workers on macOS (run 44168400921). That
  matches the worker counts below.
- Phase 7's paired comparison runs the suite ten times per side on CI, so
  it would still catch a flake.

## CI before window for `test-e2e`

Only four `main` runs follow #229's merge (`2b3aa94f`, through #230). All
their E2E jobs passed, each on one worker.

| Run | Commit | ubuntu suite | ubuntu job | ubuntu image | macOS suite | macOS job | macOS image |
|---|---|---|---|---|---|---|---|
| 43988761983 | `2b3aa94f` (#229, #230) | 301.79 s | 384 s | 20261103.629 | 322.39 s | 412 s | 20261028.624 |
| 43989711594 | `f30a4f7c` (#224) | 222.33 s | 294 s | 20261103.629 | 313.67 s | 415 s | 20261103.629 |
| 43989762511 | `ba876153` (#226) | 336.53 s | 429 s | 20261103.629 | 320.72 s | 439 s | 20261028.624 |
| 44031742886 | `902556ac` (#228) | 224.70 s | 328 s | 20261103.629 | 341.73 s | 474 s | 20261103.629 |

- **Medians.** The suite's "Finished in" medians are 263.25 s on ubuntu
  (IQR 86.37 s, 32.8%) and 321.56 s on macOS (IQR 8.27 s, 2.6%).
- **No sibling touches the job.** #224, #226 and #228 change nothing
  under the visualiser, Playwright, the workflows, `mise.toml` or
  `tasks/test`.
- **The window is contaminated.** It holds four runs, not ten. The macOS
  runner image changes inside it, and ubuntu's IQR is far over 5%. So
  Phase 7 runs the paired comparison for Playwright, as it already must
  for the dev suite.
- **CI worker counts.** Node reports 4 CPUs on `ubuntu-latest` and 3 on
  `macos-latest` (#230's `host:cpu-counts`). On ubuntu `resolved-styles`
  therefore gets 2 workers under a cap of 3, and on macOS 1 under a cap
  of 2.

## Paired CI comparison

The CI before window was contaminated, so the CI rule was judged by a
paired comparison instead. Two draft PRs were opened from `main` at
`a646f527` (this PR's merge):
- #235 kept the workers.
- #236 reverted this PR's worker allocation and #222's `-n auto`.

Each arm's `Main` run was re-run 10 times: run 44189947764 for #235 and
run 44189949799 for #236. The medians are over those 10 reruns
(attempts 2–11) of `test:e2e:visualiser`'s "Finished in" line, leaving
out failed suites.

| OS | With workers (#235) | Serial (#236) | Change | 5% rule |
|---|---|---|---|---|
| ubuntu | 168.69 s (IQR 40.4%) | 231.44 s (IQR 33.0%) | −27.1% | Pass |
| macOS | 274.53 s, 9 runs (IQR 14.5%) | 313.88 s (IQR 34.2%) | −12.5% | Pass |

Each attempt's figure, with Playwright's worker count:

| Attempt | ubuntu, workers | ubuntu, serial | macOS, workers | macOS, serial |
|---|---|---|---|---|
| 1 | 148.79 (3w) | 241.92 (1w) | failed | 306.16 (1w) |
| 2 | 185.88 (3w) | 228.50 (1w) | 251.97 (2w) | 307.93 (1w) |
| 3 | 154.83 (3w) | 229.24 (1w) | 315.46 (2w) | 354.63 (1w) |
| 4 | 141.48 (3w) | 225.16 (1w) | 274.53 (2w) | 319.84 (1w) |
| 5 | 156.73 (3w) | 327.55 (1w) | 291.72 (2w) | 199.05 (1w) |
| 6 | 152.80 (3w) | 232.94 (1w) | 113.89 (3w) | 328.73 (1w) |
| 7 | 233.37 (3w) | 229.93 (1w) | 283.87 (2w) | 339.63 (1w) |
| 8 | 180.65 (3w) | 333.11 (1w) | 106.51 (3w) | 199.74 (1w) |
| 9 | 238.05 (3w) | 316.08 (1w) | failed | 303.79 (1w) |
| 10 | 148.27 (3w) | 272.12 (1w) | 331.49 (2w) | 204.92 (1w) |
| 11 | 235.89 (3w) | 225.96 (1w) | 270.91 (2w) | 340.67 (1w) |

- **Runner size dominates macOS.**
  - With workers, the two runs on a 5-CPU runner (3 workers) took
    106–114 s, against 252–331 s with 2 workers on 3 CPUs.
  - The E2E job doesn't print its CPU count. The serial arm's times fall
    into two groups, 199–205 s and 304–355 s, which suggests the same
    split.
  - Comparing only the 2-worker runs with the slower serial group gives
    283.87 s against 328.73 s, or −13.6%. That is close to the −12.5%
    from all the runs.
- **The whole job is faster (informational).** The E2E job's medians are
  332 s with workers against 524 s serially on ubuntu, and 386 s against
  436 s on macOS.
- **A flake the workers exposed.** `dev-design-system.spec.ts`'s "scrolling
  back to the top clears the hash to bare /dev" failed on attempts 1 and
  9 on macOS, and never on the serial arm.
  - It also failed once on this PR's own branch, in run 44062948001 on
    `cc46ef68` with 2 workers on macOS. That run is missing from the
    sections above. They recorded only the CI run on `aabc3ae1`, and
    said the suite showed no flake.
  - The spec scrolled to the top before the page's deep-link scroll to
    `#code` had run. When that scroll ran afterwards, the hash stayed on
    `#code`.
  - #238 fixes the spec by waiting for the landing scroll first. By
    decision, the fix didn't restart the 10-run count: it changes one
    spec's wait, not the config or the other specs.
- **The windows (informational).** The after window holds one `main` run
  so far, this PR's own (44182225437). It took 154.35 s on 3 workers on
  ubuntu and 206.77 s on 2 workers on macOS, against the contaminated
  before window's 263.25 s and 321.56 s.
- **A correction to the windows recorded above.** The "image" versions
  (20261103.629 and 20261028.624) are the Hosted Compute Agent's
  version, not the runner image's. The runner images (`ubuntu-24.04`
  20261111.127.1 and 20261111.370.1, `macos-26-arm64` 20261102.0473.1
  and 20261112.0504.1) also change inside the window, so it stays
  contaminated.

Playwright passes the CI rule on both OSes and has landed.

## Testing

- [x] The allocation and config tests fail before the change and pass
      after: `npm --prefix cli/visualiser/frontend test -- scripts/`, 26
      passed.
- [x] `mise run frontend:check` passes.
- [x] `mise run test:e2e:visualiser`: 355 passed and 1 skipped in 45.3 s,
      on 9 workers.
- [x] The projects overlap, and no two `chromium` tests do. The check
      printed `true` then `true`, against `true` then `false` before the
      change.
- [x] Timing target met: an isolated median of 47.0 s, against 75 s.
- [x] Ten counted `mise run` executions on `c8cf7e5e`, all counted under
      Requirement 7. Each one includes `frontend:check` and the frontend
      unit tests.
- [x] Anti-masking: the diff under `cli/` adds no `.skip`, `.fixme`,
      `.only`, `test.fail` or `retries`. Locally the suite showed no
      flake. On CI it showed one, which #238 fixes (see "Paired CI
      comparison").
- [x] The CI before window is recorded above. It is contaminated, so
      Phase 7 runs the paired comparison.
- [x] Load measurement recorded (not a gate).

## Notes for Reviewers

- **The test fakes the CPU count with a spy, not a module mock.** See
  "Faking the CPU count" above. A future test that fakes a `node:` import
  in the frontend's Vitest needs the same approach.
- **macOS CI gains less.** With 3 CPUs, `resolved-styles` gets one worker
  there, so the suite runs on two workers in total. The saving on macOS
  comes from running the two projects at the same time. Phase 7's paired
  comparison decides whether that is enough. If it isn't, the lever is
  `RESOLVED_STYLES_SHARE` or `fullyParallel`.
- **Never `pkill -f chrome-headless-shell` on a machine running the
  suite.** It kills the browsers of every Playwright run on the machine,
  not just the caller's. Kill only the processes you started.
- **Flakes seen outside this suite.** No work items were opened for them:
  - `research-cli`'s `arxiv_pacing` spacing test;
  - the measurement harness's `test_the_build_stamps_lines_in_arrival_order`;
  - the race between `lint:server:check` and `build:frontend` over
    `frontend/dist`;
  - `docs:build` timing out loading its Astro config, in a discarded
    attempt.
- **An unrelated docs-site audit ignore.** A new high advisory,
  GHSA-fc5q-39xc-vjpf (a `picomatch` ReDoS on crafted extglob patterns),
  started failing `docs:audit:check` on every branch. No patched
  `picomatch` exists: the advisory covers `<=4.0.7`, which is the latest
  release. So `docs-site/audit-ignores.toml` gains a dated ignore, to be
  reviewed by 2026-12-01. The docs site passes `picomatch` only globs that
  its own build config and tooling write, at build time for a static
  site.
- **How the CI rule was judged.** Both before windows were contaminated,
  so Phase 7 used the paired comparison above. It passed, so no revert
  or retune was needed.
