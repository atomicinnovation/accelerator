---
type: "plan-validation"
id: "2026-11-15-PP-956-parallelise-the-serial-test-suites-validation"
title: "Validation Report: Parallelise the Serial Test Suites Implementation Plan"
date: "2026-11-18T10:39:08+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-11-15-PP-956-parallelise-the-serial-test-suites"
tags: ["build-system", "performance", "testing", "playwright", "pytest", "pytest-xdist", "flakiness"]
last_updated: "2026-11-18T10:39:08+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Parallelise the Serial Test Suites Implementation Plan

All seven phases are implemented as planned, and all 15 of work item
0364's acceptance criteria are met. Each is now ticked on the work item,
with its evidence. The validation ran on `main` at `52f5d20f`, after #239
merged, plus the records in #241.

`mise run` passes end to end. The first full run failed one task: the
coverage floor in `test:unit:cli`. Its coverage data was stale, and
clearing it, as the task itself advises, fixed it. See "Automated
Verification Results".

### Implementation Status

✓ Phase 1: Observable timeout branches (#220) - Fully implemented
✓ Phase 2: Shorter pure-delay waits (#221) - Fully implemented
✓ Phase 3: pytest-xdist workers (#222, #230) - Fully implemented
✓ Phase 4: Fixture isolation (#225) - Fully implemented
✓ Phase 5: Retries off (#229) - Fully implemented
✓ Phase 6: Host-relative workers (#232) - Fully implemented
✓ Phase 7: CI closing comparison (#235, #236, #238, #239, #241) - Fully
implemented

One plan box stays unchecked, by design. Phase 3's CI before window was
never met: only three `main` runs fell inside it. Requirement 8 covers
exactly that case, and the paired comparison decided the rule instead.

### Automated Verification Results

✓ `mise run` (full default) exits 0. The first run failed only on
  `test:unit:cli`'s coverage floor:
  - It measured lines at 76.66% against the 92% floor. No Rust changed in
    0364, and the task's message names stale profile data as the cause
    when nothing that affects coverage has changed.
  - After `cargo llvm-cov clean --workspace`, the next run passed its tests
    and the floor (94.41%), but failed its artefact sweep. The sweep's unit
    listing rebuilt the launcher crate, as the first build after a clean
    does.
  - A second run passed: 7,727 tests, coverage 94.41%, nothing to sweep.
  - The final full `mise run` then exited 0 in 310 s, with no failed task.
    The parent repository's conflicted `mise.toml` was ignored for it (see
    Recommendations).
✓ Dev suite in the final run: `test:integration:dev` 21 passed in 11.66 s.
✓ Playwright in the final run: `test:e2e:visualiser` 355 passed and 1
  skipped.
✓ Entrypoint lane in the final run: `test:integration:entrypoint` 79
  passed.
✓ Projects overlap, rerun on `main` during validation: the JSON-reporter
  check printed `true` then `true`, with 355 expected, 0 unexpected,
  1 skipped and 0 flaky.
✓ No fixed settle sleep of 0.5 s or more remains in
  `test_dev_integration.py`. The only sleep left is a 0.05 s poll
  interval.
✓ No `resolved-styles` spec names a written fixture: the plan's `grep`
  finds nothing.
✓ Production `tasks/shared/dev` has no diff from `main` before 0364
  (`a0d77504`).
✓ Isolated timing records, both on `devtools` with no failed task:
  - Playwright median 46.89 s (target under 75 s):
    `meta/measurements/build_2026-11-16T230609Z_engywook_warm-no-op_devtools_isolated.json`
  - Dev suite median 7.85 s (target under 60 s):
    `meta/measurements/build_2026-11-16T004012Z_engywook_warm-no-op_devtools_isolated.json`
✓ No masking: the added lines of every 0364 PR (#220–#239) contain no
  `.skip(`, `.fixme(`, `.only(`, `test.fail(`, per-test `retries`, or pytest
  skip, flaky, xfail or rerun marker. The only `retries` added is the
  config's `retries: 0`.
✓ Frontmatter validates for the plan, the work item, this report and the
  PR descriptions touched.

### Code Review Findings

#### Matches Plan:

- **Playwright config.** `playwright.config.ts` takes every worker count
  from `allocateWorkers(availableParallelism())`, sets `retries: 0` and
  `trace: "retain-on-failure"`, and states each project's invariant in one
  comment.
- **Worker allocation.** `scripts/worker-allocation.ts` is the plan's pure
  function, with `RESOLVED_STYLES_SHARE` 0.5 and `CHROMIUM_WORKERS` 1. Its
  test covers 1, 3, 4 and 16 CPUs exactly.
- **Config test.** `scripts/playwright-config.test.ts` loads the config
  under both `CI` values. It fakes 4 CPUs with a spy on the `os` module,
  and asserts retries, traces, the cap of 3 and the per-project workers
  2 and 1.
- **Dev suite fakes and recorders.** The fakes are the real module
  `tests/integration/dev/fake_process.py`, and the driver records
  escalations through `EscalationRecorder`.
- **Dev suite assertions.** `TIMEOUT_BRANCH_EVIDENCE` lists exactly the
  five strings the plan names, and an `ast` guard test pins them to
  `lifecycle.py`.
  - `assert_stopped_cleanly` makes the four checks in the plan's order,
    then checks for timeout-branch output.
  - The `workspace` and `unverified_stop_workspace` fixtures split as
    planned.
  - Harness self-tests show each clean-stop assertion failing with its
    own message.
- **Negative-path tests.** Tests 8 and 9 assert their branch's message
  and `dev.log` line, and that the sibling branch's line is absent.
  Test 16 asserts that the frontend received no SIGTERM, that our code
  never signalled it, and that `assert_stopped_cleanly` raises "stopped
  without SIGTERM".
- **Driver defaults.** `probe_timeout` went from 5.0 to 2.0 and
  `grace_kill` from 3.0 to 0.5. The pidfile, readiness and frontend-active
  budgets and `grace_quit` are unchanged.
- **Running the suite.** `pytest-xdist>=3` is in the `dev` group, and
  `test:integration:dev` runs `-n auto`.

#### Deviations from Plan:

- **One pair of PRs served both suites in Phase 7.** The plan reads as one
  pair per suite. The dev suite runs only in the integration job and
  Playwright only in the E2E job, on separate runners, so one pair (#235,
  #236) kept each suite's comparison clean and halved the CI reruns.
- **Two flakes were fixed after landing.** Phase 7 exposed both, and each
  was fixed in its own PR rather than within its phase.
  - #238 fixes Playwright's scroll-hash spec. It raced the page's
    font-deferred deep-link landing.
  - #239 fixes the entrypoint footprint test. It assumed a dispatch's
    best-effort prune finishes inside its 1–2 s budget.
  - #239 is outside 0364's two suites. It was fixed here because #222's
    load exposed it.
- **Playwright's 10-run count was not restarted.** By decision, #238
  changes one spec's wait, not the suite's config or its other specs. #232's
  two rebases were judged the same way, and recorded in #232 and the plan.
- **Corrections to the earlier records.** Phase 6's record said #232's CI
  passed and the suite showed no flake. It missed run 44062948001's macOS
  failure, which the plan and #232's description now record. The windows'
  "image" versions were the Hosted Compute Agent's; the correction doesn't
  change any contamination verdict.

#### Potential Issues:

- **macOS runner size varies.** It swings the timings more than the worker
  change does: 3- and 5-CPU runners. The paired medians pass either way,
  and the like-sized comparisons agree. Single-run CI timings on macOS
  should still be read with the runner size in mind.
- **The integration job is slightly slower with workers.** Its medians are
  934 s against 862 s on ubuntu, and 1,319 s against 1,272 s on macOS.
  Both differences are inside the IQR, so nothing went to 0369, but this
  is the cross-lane contention 0369 owns.
- **#239's test no longer covers dispatch-time pruning end to end.** It
  now asserts the retention rules through `cache prune`. That a dispatch
  prunes at all is covered only by the launcher's Rust unit tests.
- **The after windows are short.** They hold 7 `main` runs for the dev
  suite and 1 for Playwright. They are information only, because the
  paired comparison decided the rule.

### Manual Testing Required:

1. Already done during implementation, and recorded in the PRs:
  - [x] A deliberately failing Playwright test leaves its trace (#229).
  - [x] The fixture audit trace and the mutated-fixture run (#225).
  - [x] The events and escalation logs of
        `test_sigterm_ignoring_frontend_is_sigkilled` (#220).
  - [x] Ten counted `mise run` executions per suite (#222, #232).

2. Nothing further is needed to close 0364.

### Recommendations:

- **Watch the next `main` runs.** Both flake fixes merged on 2026-11-18,
  so the scroll-hash spec on macOS and the entrypoint lane on ubuntu
  should stay green. A recurrence would mean the diagnosis was incomplete.
- **If end-to-end coverage of dispatch-time pruning is wanted,** add a
  test that asserts only what the budget promises, rather than
  reintroducing a full-footprint check after one dispatch.
- **The parent repository's `mise.toml` holds jj conflict markers** from
  a rebase outside this work. Until it is resolved, every `mise run` in
  this workspace needs
  `MISE_IGNORED_CONFIG_PATHS=<parent>/mise.toml`.
