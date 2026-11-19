---
type: "work-item"
id: "PP-956"
title: "Parallelise the serial test suites"
date: "2026-10-28T16:18:46+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "done"
kind: "task"
priority: "medium"
parent: "work-item:PP-953"
source: "codebase-research:2026-10-28-full-build-duration-contention-and-slowness"
relates_to: ["work-item:PP-954", "work-item:PP-960", "work-item:PP-961", "work-item:PP-962"]
tags: ["build-system", "performance", "testing", "playwright", "pytest", "flakiness"]
last_updated: "2026-11-18T10:03:29+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-956"
aliases: ["0364"]
---

# PP-956: Parallelise the serial test suites

**Kind**: Task
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Two suites run strictly serially even though most of their tests are
independent:
- **Playwright** runs all 356 tests on one worker, though 313 of them only
  read computed styles.
- **The dev-task integration suite** runs 17 tests in sequence, each spending
  about 10 s in fixed sleeps and timeouts.

This item runs both suites with worker counts sized to the host, and shortens
the dev suite's pure-delay waits on the happy path. It also turns Playwright
retries off, deliberately, in every environment `playwright.config.ts` runs
in, so that any flakiness is fixed rather than masked.

| Suite | Today | Target on the reference workstation |
|---|---|---|
| Playwright | 131 s | under 75 s |
| Dev-task suite | 163 s | under 60 s |

On CI's smaller runners the bar is "no slower, within a 5% noise tolerance".

The two suites are independent changes, so each may land on its own once its
own landing criteria pass. The item closes only when both suites have landed
and passed their CI comparison.

## Landing Status

- **Playwright: landed.** #232 merged on 2026-11-17, and the paired CI
  comparison passed on both OSes: −27.1% on ubuntu and −12.5% on macOS.
  #238 fixes the scroll-hash spec flake that the comparison exposed.
- **Dev-task suite: landed.** #222 merged on 2026-11-16, and the paired CI
  comparison passed on both OSes: −47.0% on ubuntu and −78.4% on macOS.
  #239 fixes the entrypoint footprint test that failed under the
  workers' load.

## Context

**Playwright (research recommendation A5).** `playwright.config.ts` sets
`workers: 1` for both projects. The comment above that setting justifies it
for the kanban drag tests and the visual-regression snapshots. The snapshot
specs have since moved to `playwright.docker.config.ts`, a separate config
with its own `workers` and `retries`, used only by
`test:e2e:visualiser:docker`. This item leaves that config and lane
unchanged.

313 of the 356 tests in `playwright.config.ts` belong to the
`resolved-styles` project, which reads computed styles from static showcase
routes. The remaining 43 belong to the `chromium` project, which holds the
drag tests.

| Run, alone | Wall |
|---|---|
| Both projects, 1 worker (today) | 131 s |
| `resolved-styles`, 1 worker | 106 s |
| `resolved-styles`, 6 workers | 34 s (313/313 passed, one run) |
| `chromium`, 1 worker | 25 s |

`retries: 1` applies locally as well as in CI, so a flaky test passes on its
second attempt and goes unnoticed.

Once `resolved-styles` has workers of its own, its tests run alongside each
other and alongside the `chromium` drag tests. The drag tests mutate shared
fixture files and then restore them. That overlap is only safe if no
`resolved-styles` probe measures an element that comes from a file another
spec running at the same time writes. Today the only guard is a config
comment asking that no `resolved-styles` spec read live work-item status,
not anything in the structure of the fixtures.

**Dev-task suite (research recommendation A6).** `tests/integration/dev`
drives a real circusd, the process manager behind the dev task, through its
arbiter, circusd's supervising process. The suite's fakes are stub frontend
and server processes that circusd supervises. Each of its 17 tests takes
4–20 s, mostly in fixed waits:
- `grace_kill`, 3.0 s on every stop. Teardown sends SIGTERM to the dev task's
  descendant processes, then sleeps for the whole grace period before sending
  SIGKILL to any that survive. The wait is a fixed delay, not an upper bound,
  so a fake that exits at once still costs the full 3.0 s. The driver sets
  3.0 s, above the production default of 2.0 s.
- `probe_timeout`, 5.0 s on every teardown. Teardown confirms the stack is
  down by probing the arbiter's status. Once the arbiter has gone, the probe
  waits out the whole `probe_timeout` before reporting it unreachable.
- several 0.5 s settle sleeps in `test_dev_integration.py`.

`grace_kill` and `probe_timeout` are the test driver's defaults, passed
through its `opts`. Negative-path tests already override them with small
values to reach their branches.

Most tests are happy-path tests: they start and stop the dev task cleanly.
The rest are *negative-path tests*: their expected outcome is a timeout or a
forced kill. Each one targets a *timeout branch*, meaning the code path taken
when a wait or budget runs out before the process it waits on responds. The
branches are:
- a stop that escalates to SIGKILL after `grace_kill`;
- a readiness, pidfile or frontend-active budget that expires.

An arbiter reported unreachable after `probe_timeout` is not a timeout
branch, because every happy-path teardown ends that way. If the record that
Requirement 6 asks for turns up another branch, it joins this list.

No pytest suite in the repo uses parallelism.
`uv run --with pytest-xdist pytest tests/integration/dev -n 6` passed 17/17
in 40 s, against 163 s serially, in one run. Shortening the waits alone can't
close the gap while the tests run serially: 17 stops × 3 s of `grace_kill`
already exceeds a minute.

**Measurement conditions.** All baselines above were measured on the
*reference workstation*: engywook, an Apple M4 Max with 16 cores (12
performance, 4 efficiency) and 128 GB, running macOS 26.3. It is the host of
0368's baseline record.

A *warm workspace* is `measure:build`'s `warm-no-op` cache state: one priming
run is made and discarded, and the timed runs follow with no source changes,
so build outputs and caches are present. Every local timing in this item, the
baselines and the targets alike, is taken in a warm workspace.

Both changes also run on CI, because in `.github/workflows/main.yml` the
`test-e2e` job runs `test:e2e:visualiser` and the `test-integration` job runs
`test:integration:dev`. Each job runs on an `ubuntu-latest` runner and a
`macos-latest` runner, each with 3–4 vCPUs. The team has also decided that
`default` will mirror CI. 0369 implements that decision, after this item.

## Requirements

1. Derive Playwright's worker counts from the host's logical CPU count, N:
   - the `resolved-styles` project gets half of N, rounded down, and at
     least one;
   - the `chromium` project stays at one worker;
   - the global `workers` cap is the `resolved-styles` count plus one, so the
     two projects can always run at once.

   For example, 16 CPUs gives 8 and 9; 4 vCPUs gives 2 and 3; 3 vCPUs gives
   1 and 2; 1 CPU gives 1 and 2. Express the derivation as a pure function
   of N, so it can be tested without the host having that many CPUs. Its
   one tunable parameter is the *share*, the fraction of N given to
   `resolved-styles` (one half). The *margin*, the number of workers the cap
   adds on top of that count, is the `chromium` project's single worker, so
   it stays at one.
2. Audit the Playwright fixtures. For each project, list the fixture files
   its specs write and the files they read. Find the files from the specs'
   own file access and from the files the visualiser server opens while that
   project runs alone, traced with `fs_usage` or an equivalent. A *conflict*
   is either of these:
   - a `resolved-styles` probe whose measured element, or that element's
     computed style, comes from a file a `chromium` spec writes, or a
     `resolved-styles` spec that writes such a file;
   - a `resolved-styles` probe whose measured element comes from a file
     another `resolved-styles` spec writes, since `resolved-styles` specs
     run concurrently with each other.

   A file only one spec touches is not a conflict. A page that aggregates
   every work item still reads the written files. That read is not a
   conflict when no probe on the page measures an element that comes from a
   written file. The audit lists such reads as accepted. Resolve each
   conflict on the `resolved-styles` side, by changing the spec or giving it
   its own fixture. Leave the drag tests as they are.
3. Set Playwright `retries` to 0 in every environment `playwright.config.ts`
   runs in, CI included, and keep traces on failure instead of on first
   retry. This is a deliberate policy change that goes beyond parallelism:
   flakes it exposes are fixed under Requirement 4, whether or not
   parallelism caused them.
4. Fix every flaky test that the parallelism and the loss of retries expose.
   Do not quarantine, skip or re-add retries to hide them. If a flake's fix
   needs changes outside its suite's tests, fixtures and test config, split it
   into its own work item. It then blocks only its own suite from landing.
5. Add `pytest-xdist` as a dev dependency, and have `test:integration:dev` run
   `tests/integration/dev` with `-n auto`.
6. Before changing any waits, record each negative-path dev test, the timeout
   branch it targets, and the error message, exit reason or log line that
   only that branch produces. Keep the record in the PR. Then shorten the
   pure-delay waits:
   - `grace_kill` and `probe_timeout`: lower the test driver's defaults in
     `dev_integration_driver.py`. The fakes exit promptly on SIGTERM, and
     teardown in happy-path tests probes an arbiter known not to be running.
   - The settle sleeps: shorten or replace them in `test_dev_integration.py`,
     where they are.

   Negative-path tests keep setting their own values explicitly. Leave the
   dev task's production defaults, and the readiness, pidfile and
   frontend-active budgets, unchanged. Those budgets protect loaded CI
   runners.
7. Before either suite lands, validate its change over 10 consecutive full
   `mise run` executions on the reference workstation, all on one recorded
   commit. A run counts if `mise run` exits 0, or if it fails only in a task
   unrelated to the suite, named in the record. Any change to the suite's
   tests, fixtures or config restarts the count at zero. If a sibling item
   that changes what `default` runs lands before the merge, rebase and
   repeat the validation.
8. Judge the CI effect of each suite's change as follows:
   - **Windows.** The *before window* is the last 10 main-branch runs of the
     suite's CI job before landing. The *after window* is the first 10
     after. For each window and each runner OS, record the run IDs, the
     median duration, the spread of the durations (interquartile range as a
     percentage of the median) and the runner image version.
   - **Contamination.** A window is *contaminated* in any of these cases: a
     sibling that changes the same CI job merges inside it; the runner image
     version changes between the windows; or the before window's spread
     exceeds 5%.
   - **Paired comparison.** When a window is contaminated, compare
     differently. Open two draft PRs from one main-branch commit, one with
     the suite's change and one with it reverted. Re-run the suite's job 10
     times on each with `gh run rerun`, then compare the medians on each OS.

## Acceptance Criteria

Landing criteria for Playwright:

- [x] Given `mise run measure:build --isolate test:e2e:visualiser
      --cache-state warm-no-op --runs 5 --host-settings devtools` on the
      reference workstation, when it finishes, then the median duration of
      `test:e2e:visualiser` is under 75 s (131 s today). The record is kept
      with `--keep`.
      Median 46.89 s, on `devtools` with no failed task:
      `meta/measurements/build_2026-11-16T230609Z_engywook_warm-no-op_devtools_isolated.json`.
- [x] Given a `test:e2e:visualiser` run with the JSON reporter, when its
      output is inspected, then no two `chromium` results' runs overlap in
      time, and at least one `chromium` result's run overlaps in time with a
      `resolved-styles` result's.
      Printed `true` then `true` in #232, and again on `main` at `52f5d20f`
      during validation, with 355 passed and none unexpected.
- [x] Given the worker-count function, when a unit test calls it with 1, 3,
      4 and 16 CPUs, then the `resolved-styles` count and the global cap are
      1 and 2, 1 and 2, 2 and 3, and 8 and 9 respectively.
      `scripts/worker-allocation.test.ts` covers exactly these four cases
      (#232).
- [x] Given `playwright.config.ts`, when it is loaded with and without `CI`
      set, then `retries` is 0 and traces are kept on failure. The
      `resolved-styles` project's `workers` and the global `workers` equal
      the worker-count function's outputs for `os.availableParallelism()`,
      and the `chromium` project's `workers` is 1.
      `scripts/playwright-config.test.ts` loads the config under both `CI`
      values with 4 CPUs faked. It checks `retries` 0, `retain-on-failure`, a
      cap of 3, 2 `resolved-styles` workers and 1 `chromium` worker (#229,
      #232).
- [x] Given a test made to fail deliberately, when the suite runs, then a
      trace for that test is left in the output directory. This is checked
      once and recorded in the PR.
      Recorded in #229: a deliberately failed `smoke.spec.ts` left its
      `trace.zip`.
- [x] Given the fixture audit, when it is recorded in the PR, then it lists
      each project's written and read files, says how each list was traced,
      and shows no conflict as defined in Requirement 2.
      Recorded in #225. It was traced with `fs_usage` and by reading the
      aggregate routes' code, and found no conflict once the probes were pinned.

Landing criteria for the dev-task suite:

- [x] Given `mise run measure:build --isolate test:integration:dev
      --cache-state warm-no-op --runs 5 --host-settings devtools` on the
      reference workstation, when it finishes, then the median duration of
      `test:integration:dev` is under 60 s (163 s today). The record is kept
      with `--keep`.
      Median 7.85 s, on `devtools` with no failed task:
      `meta/measurements/build_2026-11-16T004012Z_engywook_warm-no-op_devtools_isolated.json`.
- [x] Given the project's dev dependencies, when they are inspected, then
      `pytest-xdist` is declared and `test:integration:dev` passes `-n auto`.
      `pytest-xdist>=3` is in `pyproject.toml`'s `dev` group, and the task runs
      `pytest tests/integration/dev -n auto -v` (#222).
- [x] Given the negative-path record from Requirement 6, when each
      negative-path test runs after the change, then it asserts the message,
      exit reason or log line of its recorded branch.
      Tests 8 and 9 assert their branch's message and `dev.log` line, and that
      the sibling branch's line is absent. Test 16 asserts the frontend logged
      no SIGTERM and our code never signalled it (#220).
- [x] Given the same record, when each happy-path dev test runs after the
      change, then it asserts that none of the recorded timeout-branch outputs
      appear. In particular, no stop escalates to SIGKILL. Reporting the
      arbiter unreachable at teardown is expected and is not checked.
      `stop_cleanly` and the `workspace` fixture assert that our code sent no
      SIGKILL and that no timeout-branch line appears. Harness self-tests and an
      `ast` guard show the check fails when it should (#220).
- [x] Given `dev_integration_driver.py`, when its defaults are compared with
      today's, then `grace_kill` and `probe_timeout` are lower. The
      readiness, pidfile and frontend-active budgets and the dev task's
      production defaults are unchanged.
      `probe_timeout` went from 5.0 to 2.0 and `grace_kill` from 3.0 to 0.5. The
      other budgets are unchanged, and `tasks/shared/dev` has no diff across
      0364 (#221).
- [x] Given `test_dev_integration.py`, when its settle sleeps are compared
      with today's, then each one is either a fixed delay below 0.5 s or a
      condition wait that returns as soon as its condition holds.
      The three 0.5 s sleeps are now condition waits. Only a 0.05 s poll
      interval remains (#220, #221).

Landing criteria for each suite:

- [x] Given 10 counted runs under Requirement 7, when they finish, then the
      suite passes in every one. The PR records each run's outcome and the
      commit they ran on, and that commit is the PR's merge base or has been
      re-validated after rebasing.
      Dev suite: ten exits of 0 on `d29d7621` (#222). Playwright: the suite
      passed in all ten runs on `c8cf7e5e`; one run failed only an unrelated
      `test:unit:tasks` timing test (#232). #232's two rebases and #238's
      one-spec fix didn't restart the count, for the reasons recorded there and
      in the plan.
- [x] Given the change's diff to the suite's specs, tests, fixtures and
      config, when it is inspected, then it adds none of the following:
      `skip`, `fixme`, a per-test or per-describe `retries`, or a pytest
      rerun or flaky marker. The PR lists each flake found during validation
      with its fix or its split-out work item.
      No 0364 PR adds any of them; the only `retries` added is the config's
      `retries: 0`. Two flakes were found after landing and are listed in #222
      and #232: the scroll-hash spec, fixed by #238, and the entrypoint
      footprint test, fixed by #239.

Closing criterion for each suite, checked after it lands:

- [x] Given the suite's per-task "Finished in" duration in its CI job
      (`test-e2e` for Playwright, `test-integration` for the dev suite), when
      the after window's median is compared with the before window's, then
      it is no more than 5% higher. The comparison is made on ubuntu and on
      macOS separately, and the windows are as defined in Requirement 8. The
      before window's figures are recorded in the PR before merge, and the
      after window's when it completes.

      If either window is contaminated, use the paired comparison instead.
      It passes on the same rule: on each OS, the median with the change is
      no more than 5% above the median without it. Its run IDs are recorded
      in the PR.

      If the check fails, the implementer decides whether to retune or
      revert. Retuning means changing the derived worker counts: the
      Playwright share, or an explicit CPU-derived `-n` in place of
      `-n auto` for the dev suite. A revert undoes the suite's worker changes
      only, keeping `retries: 0` and the shortened waits, and resets that
      suite's Landing Status to "not landed". A re-landing carries the
      retuned worker change alone. It repeats the timing, 10-run and
      anti-masking landing criteria, then this check with new windows.

      Both before windows were contaminated, so the paired comparison decided
      it: #235 and #236, runs 44189947764 and 44189949799, recorded in #222 and
      #232. With the change, the dev suite is 47.0% faster on ubuntu and 78.4%
      faster on macOS, and Playwright is 27.1% and 12.5% faster.

## Open Questions

- Will the CI runners' 3–4 vCPUs give the suites enough workers to be
  noticeably faster, or only no slower? This is answered by measurement after
  landing. The closing criterion requires only "no slower, within 5%".

## Dependencies

- Blocked by: none. 0368's warm no-op baseline, which the parent 0361 needed
  recorded before any timing change, now exists at
  `meta/measurements/build_2026-11-01T143523Z_engywook_warm-no-op_devtools.json`.
  This item's own targets are measured against the baselines in Context. Any
  flake split out under Requirement 4 is added here as a blocker for its
  suite when it is created.
- Blocks: no item directly. 0361's closing comparison counts on this item's
  savings: the research projects about 70 s from the Playwright change and
  about 120 s from the dev suite change. That comparison in turn gates 0369.
  Abandoning either suite therefore needs agreement on 0361.
- Environment: the reference workstation, engywook, with the `devtools` host
  settings of 0368's baseline record. Each 10-run validation needs about an
  hour of otherwise idle time on it. 0370 changes engywook's host settings
  while it records its conditions. This item's measurements and validations
  must therefore not overlap that recording, and each `measure:build` record
  must show `devtools`.
- Tooling: local timings come from `measure:build --isolate`, which already
  exists. CI timings come from the "Finished in" lines in GitHub Actions job
  logs. The pre-landing medians must be captured before log retention removes
  them. `pytest-xdist` is a new third-party package; adding it updates
  `pyproject.toml` and `uv.lock`. `main.yml` has no `workflow_dispatch`
  trigger, so the paired comparison uses pull-request runs and
  `gh run rerun`. That needs write access to the repository's Actions.
- External: GitHub-hosted runners. A change to their runner image
  contaminates a suite's CI windows (Requirement 8).
- Coordinated with: the other children of 0361 (0362, 0363, 0365, 0366, 0367,
  0370). A sibling that changes the `test-e2e` or `test-integration` job
  contaminates any of this suite's windows it merges inside (Requirement 8).
  The closest overlap is 0362, which also edits `tasks/test/integration.py`
  and reshapes the `test-integration` job. Preferably, such siblings don't
  merge between the start of a suite's before window and the end of its
  after window.
- Related: 0369 "Make `default` mirror CI" sits outside 0361 and merges only
  after 0361 closes, so it cannot land during this item's validation.

## Assumptions

- Under `-n auto` the dev suite's tests don't share state: each uses its own
  `tmp_path` workspace and `free_port` allocations. If a test turns out to
  share state, fixing it is in scope (Requirement 4).
- Fixing the flaky tests exposed here is bounded work within this item, not a
  separate investigation. Requirement 4 sets the boundary for when a flake is
  split out instead.

## Technical Notes

- Worker and retry settings: `cli/visualiser/frontend/playwright.config.ts`
  (`retries`, `workers`, `trace`, and the `projects` array). Node's
  `os.availableParallelism()` gives the logical CPU count.
- Playwright supports a per-project `workers` setting from 1.52; the repo is
  on 1.59.1, so one config file with two projects is enough.
- Dev suite command: `tasks/test/integration.py:57`.
- Driver waits: `tests/integration/dev/dev_integration_driver.py:191-196`.
  The comment there explains why the readiness budgets are generous; the
  pure-delay waits are a different matter.
- The fake server's `api_port` defaults to a fixed `7777` but is only
  written into its info file, not bound, so it should not collide under
  xdist.
- `measure:build`'s isolation mode, its cache states and
  `measure:compare-builds` are documented in `tasks/README.md` under "Build
  timing".

## Drafting Notes

- Grouped A5 and A6 because both use the same approach: host-relative
  workers, then repeated-run flake validation. Agreed in conversation. After
  review, each suite was allowed to land on its own so that a flake in one
  does not hold back the other. The Landing Status section shows which
  suites have landed.
- Kept the wait shortening (Requirement 6) here rather than in a follow-up,
  and named it in the Summary. It only touches test-side values.
- Kept kind `task` despite the validation and CI-window work. Each suite
  lands as a task-sized change, and splitting the item was considered and
  declined.
- Replaced the fixed 6 workers from the experiments with host-relative
  counts, following the mirror-CI decision recorded in the research.
- The 75 s and 60 s targets are proposals derived from about 60 s projected
  for Playwright and 40 s measured for the dev suite.
- Took "no retries" to mean `retries: 0` in CI as well as locally, and
  switched `trace` from `on-first-retry` (which would never fire) to
  `retain-on-failure`. The Docker visual-regression config keeps its own
  settings; changing it is out of scope.
- Took "make independent" to mean no spec reads or writes a file that a
  concurrently running spec writes, resolved on the `resolved-styles` side.
  Plan review narrowed this to a per-probe definition (Requirement 2).
  Pages that list every work item necessarily read the drag tests' files,
  and a computed-style probe pinned to an unwritten fixture measures the
  same thing whatever those files say. Giving the drag tests per-test
  fixture copies is a larger change and is not required here. Neither is an
  automated guard that keeps the audit true for future specs; the revised
  config comment is the reminder.
- Took the margin to be the `chromium` project's single worker rather than a
  second tuning lever. Raising it would only lift the global cap, since
  `chromium` stays at one worker.
- Kept the retries policy change here rather than landing it first on its
  own. It is the reason flakes surface during this item's validation.
- Took "no slower than today, ideally faster" to mean the CI criterion
  compares 10-run medians before and after, with a 5% tolerance for runner
  noise. "Faster" is an aim rather than a gate.

## References

- Source: `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- Related: 0361, 0368, 0369
