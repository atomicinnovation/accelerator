---
type: "plan-review"
id: "2026-11-15-0364-parallelise-the-serial-test-suites-review-1"
title: "Plan Review: Parallelise the Serial Test Suites Implementation Plan"
date: "2026-11-15T17:02:52+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-11-15-0364-parallelise-the-serial-test-suites"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "performance", "portability", "standards"]
review_number: 1
review_pass: 3
tags: ["build-system", "performance", "testing", "playwright", "pytest", "pytest-xdist"]
last_updated: "2026-11-15T20:23:46+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Parallelise the Serial Test Suites Implementation Plan

**Verdict:** REVISE

The plan is well structured. It has two independent tracks, the risky
worker change comes last in each, every phase can be merged on its own, the
test seams go through the existing `killer=` injection point with no change
to production code, and the worker allocation is a pure, unit-tested
function. The code references it cites are almost all accurate. But several
concrete details would stop the plan's own success criteria from working:

- Test 6 under `stop_cleanly` records a forced kill of a zombie.
- The `@vitest-environment node` pragma crashes the shared jsdom setup file.
- The `jq` overlap check reports overlap even for a serial run.
- The Phase 2 `rg` guard matches `time.sleep(60)` strings that are already
  in the file.

The Phase 1 safety net is also never shown to catch the failure it exists
for. The verdict is REVISE because there are well over three major findings;
no finding is critical.

### Cross-Cutting Themes

- **The forced-kill recorder records calls, not outcomes** (flagged by:
  correctness, architecture, test-coverage). `identity_matches` succeeds for
  zombies, so `reap_descendants` calls `kill()` on processes that have
  already exited. Test 6's pytest-owned `Popen` server will fail
  `stop_cleanly` every time. The recorder also overrides `signal_group`,
  which the dev lifecycle never calls, and misses the `terminate()` that
  `kill_arbiter` sends after `grace_quit` expires.
- **The safety net is unproven and full of gaps** (flagged by: test-coverage,
  correctness, code-quality). `stop_cleanly` ignores fakes that survive the
  stop. No positive control shows that it fails on a real escalation. The
  absence-only evidence strings can go vacuous without warning. A SIGTERM
  that arrives before the handler is installed causes false failures.
- **Fake behaviour is code built from nested string templates** (flagged by:
  code-quality, architecture, correctness). The recorder is spliced into
  templates and then embedded again with `repr`. The snippet already lacks
  `os`/`time` imports in the npm template and the child source.
- **The worker margin and chromium's worker count are one invariant held in
  two places** (flagged by: code-quality, architecture, portability). The
  Phase 7 lever "retune `CONCURRENT_PROJECT_MARGIN`" would only move the
  global cap.
- **Fixture isolation is asserted rather than demonstrated** (flagged by:
  architecture, test-coverage). The audit narrows Requirement 2's file-level
  definition of a conflict to a per-probe one without saying so. Nothing
  deterministic shows that `resolved-styles` survives mutated fixtures.
- **Sizing and measurement ignore other lanes on the host** (flagged by:
  performance, portability). `-n auto` uses physical cores and Playwright
  uses logical CPUs. Every gate is per-task and isolated, so cost pushed onto
  sibling lanes under `default` and in the CI roll-ups would go unseen.
- **A driver comment goes stale** (flagged by: standards, code-quality). The
  copy Phase 1 keeps says the defaults are "deliberately generous", which is
  false once Phase 2 lands.

### Tradeoff Analysis

- **Proof the evidence is strong vs keeping scope tight**: test-coverage
  wants positive controls and a mutated-fixture run. The plan explicitly
  excludes new tests for untested branches. Recommendation: add the controls
  as one-off steps recorded in the PR, not as permanent tests. That proves
  the net works without widening the suite.
- **Fast inner loop vs certainty under load**: performance wants measurements
  under `default` load and an A/B on `-n`. That lengthens validation, which
  is already ten `mise run`s. Recommendation: record one non-isolated
  `default` measurement after each worker phase, without making it a gate.

### Findings

#### Major

- 🟡 **Correctness / Architecture**: `stop_cleanly` records a forced kill of
  a zombie in test 6, and the recorder logs calls rather than outcomes.
  **Location**: Phase 1, Sections 2–3.
  `identity_matches` compares only `create_time`, so it succeeds for
  zombies. `reap_descendants` therefore `kill()`s test 6's unreaped `Popen`
  server, and the unconditional recorder logs it. Test 6 also never writes
  `fakes.log`.
- 🟡 **Correctness**: `@vitest-environment node` breaks the global jsdom
  setup file.
  **Location**: Phase 5, Section 1.
  `src/test/setup.ts` runs for every test and calls
  `Object.defineProperty(window, …)` at module level (verified, lines
  55–86). Under `node` there is no `window`, so the config test errors
  instead of going red.
- 🟡 **Correctness**: The overlap `jq` check truncates `startTime` to whole
  seconds.
  **Location**: Phase 6, Success Criteria.
  Fractional seconds are dropped but durations keep their milliseconds, so
  two tests run back to back within the same second count as overlapping.
  The check can't tell parallel runs from serial ones.
- 🟡 **Correctness**: The `rg` guard against fixed sleeps can never pass.
  **Location**: Phase 2, Success Criteria.
  The pattern matches the `time.sleep(60)` strings in test 6's `-c` source
  (verified, lines 230–231).
- 🟡 **Correctness**: `grace_kill` 0.5 also bounds `kill_arbiter`'s wait
  after SIGTERM.
  **Location**: Phase 2, Section 1.
  A SIGTERMed circusd stops its watchers gracefully for up to 2 s, so 0.5 s
  forces a SIGKILL whenever a busy arbiter misses the probe. That is more
  likely under xdist with `probe_timeout` at 2.0.
- 🟡 **Test Coverage**: `stop_cleanly` silently ignores fakes that survive
  the stop.
  **Location**: Phase 1, Section 3 (and the Phase 2 rationale).
  Only fakes that were running before the stop and dead after it are
  checked, but Phase 2's reasoning assumes survivors are flagged.
- 🟡 **Test Coverage**: The safety net is never shown to fail on the
  condition it guards.
  **Location**: Phase 1, TDD order.
  The red step only proves the log files are missing. No step shows a
  non-empty `forced_kills`, a matched timeout line, or a rejected
  unsignalled fake.
- 🟡 **Test Coverage**: Fixture independence is checked only by audit and by
  runs that may never hit the race.
  **Location**: Phase 4, Sections 3 and 5.
  The `rg` for four slugs can't see aggregate probes. A deterministic run of
  `resolved-styles` against pre-mutated fixtures would turn the audit claim
  into an observed result.
- 🟡 **Architecture**: The audit narrows Requirement 2's file-level
  definition of a conflict to a per-probe one without saying so.
  **Location**: Phase 4, Sections 3 and 5.
  Pinning probes can't empty the file-level conflict class for aggregates
  such as `/kanban` and `/library/work-items`, yet the record claims "both
  empty".
- 🟡 **Code Quality / Architecture**: The lifecycle recorder is code built
  from nested string templates.
  **Location**: Phase 1, Section 2.
  It goes through three levels of quoting and depends on fragment order
  (`{ignore}` after the recorder). It is never linted or type-checked.
- 🟡 **Code Quality / Architecture**: `CONCURRENT_PROJECT_MARGIN` is
  chromium's worker count under another name.
  **Location**: Phase 6, Section 1, and Phase 7, Step 5.
  The hard-coded `workers: 1` for chromium and the margin encode the same
  fact, so retuning the margin would only move the global cap.
- 🟡 **Code Quality**: Absence checks on duplicated log strings can go
  vacuous without warning.
  **Location**: Phase 1, Section 3 (`TIMEOUT_BRANCH_EVIDENCE`).
  `still alive after SIGKILL` and `frontend watcher did not become active`
  are only ever checked for absence. If either production message is
  reworded, the check keeps passing.
- 🟡 **Performance**: The new fixture teardown adds a second stop, against a
  dead arbiter, to every happy-path test.
  **Location**: Phase 1, Section 3.
  `_hard_cleanup` already runs a driver `stop` (verified), so each
  happy-path test pays an extra `probe_timeout`: about 5 s each in Phase 1,
  which contradicts the "wall time unchanged" criterion.
- 🟡 **Performance**: Success metrics are per-task, so cost shifted onto
  co-running lanes is invisible.
  **Location**: Desired End State, and Phases 3, 6 and 7.
  The source research showed `test:unit:visualiser` going from 64 s to
  162 s when the suites ran in parallel.
- 🟡 **Performance**: The worker pools are sized to the whole host, with no
  allowance for other lanes.
  **Location**: Performance Considerations, and Phase 6.
  16 xdist workers, each running a circusd and its fakes, plus 8 Chromium
  workers, overlap under `default`. On CI they compete with 10 sibling
  lanes.
- 🟡 **Standards / Code Quality**: The driver comment that is kept will
  contradict the shortened defaults.
  **Location**: Phase 1, Section 2, and Phase 2, Section 1.
  "Happy-path defaults are deliberately generous…" becomes false once
  `probe_timeout` is 2.0 and `grace_kill` is 0.5.

#### Minor

- 🔵 **Correctness / Test Coverage / Architecture**: The arbiter's SIGTERM
  after `grace_quit` expires escapes the recorder, and the `signal_group`
  override is dead code.
  **Location**: Phase 1, Section 2.
  `terminate(arbiter_pid)` at `lifecycle.py:738` isn't recorded, and the
  line-730 "endpoint still reachable after quit" message is missing from the
  evidence list.
- 🔵 **Correctness**: The recorder uses imports that the npm template and
  the child source don't have.
  **Location**: Phase 1, Section 2.
  `os` is missing from `_NPM_TEMPLATE`, and the child `-c` source has no
  imports at all.
- 🔵 **Correctness**: `started` is logged before the SIGTERM handler is
  installed.
  **Location**: Phase 1, Section 2.
  A SIGTERM that arrives in that window produces a false "stopped without
  SIGTERM". Install the handler first.
- 🔵 **Correctness**: "One chromium `parallelIndex`" doesn't capture "one
  chromium worker at a time".
  **Location**: Phase 6, Success Criteria.
  `parallelIndex` is a slot in the global pool, so a correct config can give
  chromium several indices. Assert instead that no two chromium results
  overlap in time.
- 🔵 **Correctness**: `stop_cleanly` identifies fakes by PID alone.
  **Location**: Phase 1, Section 3.
  A recycled PID under xdist churn can cause a false failure. Record
  `create_time` alongside the PID.
- 🔵 **Test Coverage**: The branch-unique assertions are never shown to
  reject the sibling branch.
  **Location**: Phase 1, TDD order (tests 8 and 9).
- 🔵 **Test Coverage**: The SIGTERM-receipt check is vacuous for several of
  the happy-path stops it lists.
  **Location**: Phase 1, Section 3 (tests 6 and 11, and
  `test_restart_round_trip`).
- 🔵 **Code Quality**: Assertions in teardown and `finally` can mask the
  primary failure, and `failing_workspace` misdescribes test 16.
  **Location**: Phase 1, Section 3.
- 🔵 **Portability**: The two suites count host CPUs on different bases,
  logical and physical.
  **Location**: Phase 3, and Performance Considerations.
  Consider `-n logical`, and mention `PYTEST_XDIST_AUTO_NUM_WORKERS`.
- 🔵 **Portability**: The fixture-audit trace method only works on macOS
  and needs sudo.
  **Location**: Phase 4, Section 5.
  Name `strace -f -e trace=openat` as the Linux equivalent.
- 🔵 **Portability**: The overlap check depends on an unstated "task env".
  **Location**: Phase 6, Success Criteria.
  Spell out `ACCELERATOR_VISUALISER_BIN` and `E2E_HEALTH_PORT`. Without
  them, the run can reuse a stray server on port 19087.
- 🔵 **Standards**: The Playwright project comments are only partly revised.
  **Location**: Phase 4, Section 4, and Phase 6, Section 2.
  Chromium's existing "snapshot project" block needs replacing, not adding
  to.
- 🔵 **Standards**: The pyproject comment convention the plan claims doesn't
  match the `dev` group.
  **Location**: Phase 3, Section 3.
  Plain pytest plugins carry no comment.
- 🔵 **Standards**: A library module imported by the config is placed in
  `scripts/`.
  **Location**: Phase 6, Section 1.
- 🔵 **Performance**: File-level imbalance is only discovered after the
  10-run validation has started.
  **Location**: Phase 6.
  Record per-file durations in Phase 5, and decide on `fullyParallel`
  before validating.

#### Suggestions

- 🔵 **Code Quality / Standards**: Derive the detail slug and the
  `data-relpath` from one `UNWRITTEN_WORK_ITEM` constant, and name the file
  it lives in.
  **Location**: Phase 4, Sections 1–2.
- 🔵 **Code Quality**: Drop the unused `opts` parameter on `stop_cleanly`.
  **Location**: Phase 1, Section 3.
- 🔵 **Test Coverage**: Mock `availableParallelism` in the config test and
  assert literal worker counts.
  **Location**: Phase 6, Section 2.
- 🔵 **Correctness**: List the exact `TIMEOUT_BRANCH_EVIDENCE` strings. The
  record lists two `dev.log` lines, not four.
  **Location**: Phase 1, Section 3.
- 🔵 **Performance**: Try `--dist worksteal` before retuning `-n`.
  **Location**: Phase 3.
- 🔵 **Portability**: Use `grep -En` instead of `rg`, which mise doesn't
  provision.
  **Location**: Phases 2 and 4.
- 🔵 **Portability**: Consider an environment-variable override for the CPU
  count, or document that per-host tuning is deliberately a code change.
  **Location**: Phase 6 and Phase 7.
- 🔵 **Portability**: State that the local targets are specific to engywook,
  and that Phase 7 is the gate across hosts.
  **Location**: Desired End State.

### Strengths

- ✅ Each track puts its risky change (workers) last, after a safety net.
  Every phase can merge on its own, and the CI before window isolates the
  effect of parallelism, which matches the revert-only-the-workers rule.
- ✅ Forced kills are captured through the existing `killer=` injection
  point, so production lifecycle code stays untouched.
- ✅ `allocateWorkers` is a pure function of the CPU count, with no host
  dependency. It is tested at the boundaries, and its arithmetic and table
  are correct.
- ✅ The code references are accurate: driver lines, lifecycle strings and
  defaults, config ranges, slugs, and `data-relpath`. xdist isolation holds,
  because the IPC sockets sit in a per-workspace hashed tempdir.
- ✅ Each production-facing change has a red-first test that follows an
  existing precedent (`test_hooks_task_environment.py`,
  `visual-diff-ciede2000`).
- ✅ Retries come off at one worker before parallelism, so flakes from losing
  retries are separated from flakes caused by parallelism.
- ✅ `wait_until` gathers the ad-hoc polling into one well-named primitive.
- ✅ Every `mise` task and `measure:build` flag the plan names exists, and the
  VCS commands use jj.
- ✅ The scope boundaries (Docker lane, production code, other pytest lanes,
  `fullyParallel` held back as a lever) are explicit. The plan also removes
  stale and duplicated comments.

### Recommended Changes

1. **Make the forced-kill recorder outcome-based, and fix test 6**
   (addresses: zombie forced kill; recorder logs calls; `terminate` escapes
   the recorder; `signal_group` dead code).
   - In `kill`, record only when `self.is_alive(pid)` is true, so zombies
     are excluded.
   - Also record `terminate(pid)` of the arbiter.
   - Drop the `signal_group` override.
   - Consider composing over a `ProcessOps` rather than subclassing.
   - Specify that `fake_events` returns `{}` when `fakes.log` is absent.
   - Add the line-730 message to the evidence list.
2. **Tighten `stop_cleanly` and prove that it bites** (addresses: survivors
   ignored; safety net unproven; vacuous SIGTERM checks; recycled PIDs).
   - Assert that every running fake is dead after the stop.
   - Record `create_time` in `fakes.log`.
   - In `test_restart_round_trip`, assert SIGTERM receipt for the first
     generation of fakes.
   - Add a positive-control step recorded once in the PR: run with
     `server_ignore_sigterm` and short graces, and show that `stop_cleanly`
     fails. Show the same for a sibling-branch swap between tests 8 and 9.
3. **Move the fake into a real module** (addresses: string templates;
   missing imports; handler ordering). Create
   `tests/integration/dev/fake_process.py` with flags for role, events log,
   ignoring SIGTERM, and spawning a child. Install the handler before logging
   `started`.
4. **Fix the success criteria that can't work** (addresses: `rg` matches
   `sleep(60)`; `jq` truncation; `parallelIndex`; `rg` not provisioned).
   - Anchor the sleep guard to statement starts, or name the three lines,
     and use `grep -En`.
   - Make the overlap check keep milliseconds.
   - Replace the `parallelIndex` count with a check that no two chromium
     results overlap.
   - Confirm that the check prints `false` against today's config.
5. **Drop the `@vitest-environment node` pragma** (addresses: jsdom setup
   crash; the new-convention suggestion). Node APIs work under jsdom.
6. **Remove the duplicated teardown stop** (addresses: extra stop against a
   dead arbiter). Split `_hard_cleanup` into a survivor-kill and socket
   removal step with no driver stop. Use the full fallback only when
   `stop_cleanly` didn't run or failed.
7. **Revisit `grace_kill` for the arbiter path** (addresses: `grace_kill`
   bounds `kill_arbiter`). Cover `kill_arbiter`'s use in the Phase 2
   rationale and risk notes. Either keep 0.5 for the reap, knowing it is the
   arbiter's budget too, or measure the terminate-branch hit rate under xdist
   before landing.
8. **Model the chromium workers in the allocation** (addresses: margin
   constant). Return `{ resolvedStyles, chromium, total }` and wire all three
   values from it. Drop `CONCURRENT_PROJECT_MARGIN` from the Phase 7 levers.
9. **Make the Playwright isolation claim explicit and observable**
   (addresses: narrowed conflict definition; non-deterministic check).
   - State the per-probe definition of a conflict and get Requirement 2
     aligned with it.
   - List the page-level aggregate reads as accepted non-conflicts.
   - Add a one-off run of `resolved-styles` against pre-mutated
     0001/0003/0005/0006.
   - Name `strace` as the Linux equivalent of `fs_usage`.
10. **Guard the absence-only evidence** (addresses: vacuous absence checks).
    Add a small unit test that each evidence string still occurs in
    `lifecycle.py`.
11. **Add measurements that account for other lanes** (addresses: per-task
    metrics; host-wide sizing; logical vs physical; file imbalance).
    - Record one non-isolated `default` measurement after each worker phase,
      plus the job wall-clock in Phase 7.
    - Compare `-n auto` with `-n logical` or half the CPUs under load.
    - Record per-file durations in Phase 5.
12. **Fix the comments and conventions** (addresses: stale driver comment;
    Playwright comments; pyproject convention; the `UNWRITTEN_WORK_ITEM`
    constant).
    - Rewrite or delete the surviving driver comment in Phase 2.
    - Replace chromium's comment block rather than adding a second one.
    - Add `pytest-xdist` with no comment.
    - Derive the slug and the relPath from one named constant.
    - Rename `failing_workspace` and drop the unused `opts`.

## Per-Lens Results

### Architecture

**Summary**: The plan is well structured, with two independent tracks of
phases that can each be merged and are ordered so every intermediate state
is safe. The weaknesses are evidence captured at the level of calls rather
than outcomes, and an audit that quietly narrows the work item's file-level
definition of a conflict.

**Strengths**: The risky change comes last in each track. Forced kills are
captured through the existing `killer=` injection point. Worker allocation
is a pure core with the I/O kept at the edge. Conflicts are resolved on the
`resolved-styles` side through a named constant. Scope boundaries are
explicit.

**Findings**:
- 🟡 major / medium — **ForcedKillRecorder records kill calls, not forced
  kills of live processes** (Phase 1, Section 2). `pid_identity_matches`
  succeeds for zombies, so a fake that exited on SIGTERM but hasn't been
  reaped yet is "forced-killed". Record only when `is_alive(pid)` is true.
- 🟡 major / medium — **The audit narrows Requirement 2's file-level
  definition of a conflict to a per-probe one without saying so** (Phase 4,
  Sections 3 and 5). Aggregates still read 0001/0003/0005/0006 after
  pinning. State the probe-level definition, align Requirement 2, and list
  the page-level reads as accepted.
- 🔵 minor / medium — **Fake behaviour is growing as nested source-string
  templates** (Phase 1, Section 2). Move it into a real `fake_process.py`
  module with CLI flags.
- 🔵 minor / medium — **Recorder subclasses the concrete PsutilProcessOps
  and covers SIGKILL only** (Phase 1, Section 2). Use a decorator that
  composes over a `ProcessOps`. Intercept `kill` and the arbiter's
  `terminate`, and drop `signal_group`.
- 🔵 suggestion / medium — **The chromium worker count and the margin encode
  one invariant in two places** (Phase 6). Return `chromium` from
  `allocateWorkers`.
- 🔵 suggestion / low — **Happy-path evidence assumes every workspace has
  the driver's fakes** (Phase 1, Section 3). Test 6 never writes
  `fakes.log`. Treat a missing file as empty.

### Code Quality

**Summary**: Each phase is small and testable on its own, and the names are
good domain names. The risks are the fake recorder generated from string
templates, a recorder method nothing calls, absence checks on duplicated log
strings, and a margin constant that hides chromium's worker count.

**Strengths**: `allocateWorkers` is pure and injected with its CPU count.
`wait_until` consolidates the polling loops. The relPath is held in a named
constant. Duplicated and stale comments are removed. `stop_cleanly` gathers
the assertions behind one intention-revealing name. The `killer=` injection
point is reused.

**Findings**:
- 🟡 major / high — **Lifecycle recorder is code generated from nested
  string templates** (Phase 1, Section 2). It has three levels of quoting
  and depends on fragment order, and the snippet already lacks the `os`
  import. Move it into a real module with an explicit `ignore_sigterm`
  parameter.
- 🟡 major / high — **The "margin" constant is chromium's worker count under
  another name** (Phase 6, Section 1). Model `{ resolvedStyles, chromium,
  total }` and drop the margin lever from Phase 7.
- 🟡 major / medium — **Absence checks on duplicated log strings go vacuous
  without warning** (Phase 1, Section 3). Guard that each string still
  occurs in `lifecycle.py`, or split exercised strings from unexercised
  ones.
- 🔵 minor / high — **`signal_group` override is dead code for the dev
  lifecycle** (Phase 1, Section 2). Drop it, and prefer a decorator.
- 🔵 minor / high — **The surviving driver comment becomes false once the
  defaults are cut** (Phase 2, Section 1). Rewrite or delete it.
- 🔵 minor / medium — **Assertions in teardown and `finally` can mask the
  primary failure, and the fixture name misdescribes test 16** (Phase 1,
  Section 3). Move test 17's `stop_cleanly(ws_b)` into the test body, and
  rename `failing_workspace`.
- 🔵 suggestion / medium — **Unused `opts` parameter on `stop_cleanly`**
  (Phase 1, Section 3).
- 🔵 suggestion / medium — **Slug and relPath name the same fixture
  independently** (Phase 4, Sections 1–2). Use one `UNWRITTEN_WORK_ITEM`
  constant.

### Test Coverage

**Summary**: The plan is strongly test-first, but the Phase 1 safety net is
never shown to catch the failure it exists for, and `stop_cleanly` lets
survivors through. Playwright fixture independence rests on a manual audit
and on runs that may never hit the race.

**Strengths**: Every production-facing change has a red-first test.
Observability comes first. The logs are cumulative per workspace.
`forced_kills` guards against the recorder not being wired. The two fixtures
turn a dirty stop into a teardown error. The overlap criterion is
mechanical. Retries come off before parallelism. Anti-masking criteria are
explicit.

**Findings**:
- 🟡 major / high — **`stop_cleanly` silently ignores fakes that survive the
  stop** (Phase 1, Section 3). Add `assert stopped == running`, and expect
  test 11's orphaned grandchildren to surface.
- 🟡 major / high — **The safety net is never shown to fail on the condition
  it guards** (Phase 1, TDD order). Add positive controls: a stop with
  `server_ignore_sigterm` and short graces, and an
  `assert_no_timeout_branch_output` that raises on the `dev.log` of tests 8
  and 9.
- 🟡 major / medium — **Fixture independence is checked by audit and
  probabilistic runs, not by a deterministic test** (Phase 4). Run
  `resolved-styles` against pre-mutated chromium-written fixtures.
- 🔵 minor / medium — **The arbiter's SIGTERM escalation after `grace_quit`
  is not observed** (Phase 1, Section 2). Record `terminate`.
- 🔵 minor / medium — **Branch-unique assertions are never shown to reject
  the sibling branch** (Phase 1, tests 8 and 9). Run a one-off swap mutation.
- 🔵 minor / medium — **SIGTERM-receipt check is vacuous for several listed
  happy-path stops** (tests 6 and 11, `test_restart_round_trip`).
- 🔵 suggestion / low — **Config worker assertions re-derive the expected
  value with the function under test** (Phase 6, Section 2). Mock
  `availableParallelism`.

### Correctness

**Summary**: Most code references check out against the source. But several
concrete details would make the plan's own success criteria fail on correct
code, or pass on incorrect code: the zombie forced kill in test 6, the node
environment crashing the jsdom setup, the `jq` truncation, and the `rg`
guard matching existing strings. The plan also underrates `grace_kill`'s
role in `kill_arbiter`.

**Strengths**: Line numbers, strings and defaults are accurate. The record's
strings match `lifecycle.py`. The recorder wraps the real `kill` entry
point. xdist isolation holds. The argv test matches its precedent. The
`allocateWorkers` arithmetic is correct.

**Findings**:
- 🟡 major / high — **`stop_cleanly` in test 6 records a forced kill of a
  zombie and fails** (Phase 1, Section 3). Log only kills of processes that
  are still alive, or reap the `Popen`, or keep test 6 off `stop_cleanly`.
  Have `fake_events` return `{}` when the file is missing.
- 🟡 major / high — **`@vitest-environment node` breaks the global jsdom
  setup file** (Phase 5, Section 1). Drop the pragma.
- 🟡 major / high — **Overlap check truncates `startTime` to whole seconds,
  so a serial run passes** (Phase 6). Keep the milliseconds, and confirm the
  check prints `false` on today's config.
- 🟡 major / high — **The `rg` guard matches existing `time.sleep(60)`
  strings and can never pass** (Phase 2). Anchor the pattern, or name the
  three lines.
- 🟡 major / medium — **`grace_kill` also bounds `kill_arbiter`'s wait after
  SIGTERM, and 0.5 s is below circus's 2 s graceful stop** (Phase 2,
  Section 1). Account for this in the rationale and measure how often the
  terminate branch is hit.
- 🔵 minor / medium — **"One chromium `parallelIndex`" doesn't capture "one
  chromium worker at a time"** (Phase 6). Assert that no chromium results
  overlap in time.
- 🔵 minor / high — **The SIGTERM of the arbiter after `grace_quit` expires
  escapes the recorder** (Phase 1). Record `terminate`, and add the line-730
  message.
- 🔵 minor / high — **Recorder needs imports the npm template and child
  source lack** (Phase 1, Section 2).
- 🔵 minor / medium — **`started` is logged before the SIGTERM handler is
  installed** (Phase 1, Section 2).
- 🔵 minor / low — **`stop_cleanly` identifies fakes by PID alone, so
  recycled PIDs can produce false failures** (Phase 1, Section 3).
- 🔵 suggestion / high — **"Four `dev.log` substrings from the record"
  doesn't match the record** (Phase 1, Section 3). Enumerate the exact
  strings.

### Performance

**Summary**: The plan targets the right bottlenecks. The weaknesses are a
new teardown that adds a stop against a dead arbiter to every test, worker
pools sized as if each suite had the host to itself, and success metrics
that can't see cost pushed onto sibling lanes.

**Strengths**: It separates the two roles of `probe_timeout`. Shorter waits
come before workers. `wait_until` is cheap. `fullyParallel` and `-n` are
held back as levers. The recorder overhead is negligible.

**Findings**:
- 🟡 major / high — **New fixture teardown adds a second, dead-arbiter stop
  to every happy-path test** (Phase 1, Section 3). Make `_hard_cleanup` a
  fallback only.
- 🟡 major / high — **Success metrics are per-task, so cost shifted onto
  co-running lanes is invisible** (Desired End State, Phases 3, 6 and 7).
  Add a non-isolated `default` measurement and the job wall-clock.
- 🟡 major / medium — **Worker pools sized to the whole host with no
  allowance for co-tenancy** (Performance Considerations, Phase 6). Compare
  `-n auto` with a smaller count under `default` load.
- 🔵 minor / medium — **File-level imbalance is discovered only after the
  10-run validation starts** (Phase 6). Record per-file durations in
  Phase 5.
- 🔵 suggestion / low — **Consider work-stealing distribution for few,
  uneven dev tests** (Phase 3). Try `--dist worksteal`.

### Portability

**Summary**: The plan works on ubuntu and macOS CI and on the macOS dev
host. Node 22 has `availableParallelism`, and the socket paths stay within
`sun_path` limits. The gaps are an inconsistent basis for counting CPUs, an
audit method that only works on macOS, and verification steps that depend
on an unstated environment.

**Strengths**: The worker derivation is pure. Socket paths are hashed and
length-guarded. The PR records what `-n auto` resolves to. The Docker lane
is out of scope. `jq` is pinned.

**Findings**:
- 🔵 minor / medium — **Two suites count host CPUs on different bases
  (logical vs physical)** (Phase 3). Consider `-n logical`, and mention
  `PYTEST_XDIST_AUTO_NUM_WORKERS`.
- 🔵 minor / high — **Fixture audit trace method is macOS-only and needs
  sudo** (Phase 4, Section 5). Name `strace`.
- 🔵 minor / high — **Overlap check depends on an unstated "task env"**
  (Phase 6). Spell out `ACCELERATOR_VISUALISER_BIN` and `E2E_HEALTH_PORT`.
- 🔵 suggestion / high — **Success criteria use ripgrep, which mise does not
  provision** (Phases 2 and 4).
- 🔵 suggestion / medium — **Worker share and margin are baked in with no
  runtime override** (Phases 6 and 7).
- 🔵 suggestion / low — **Local timing targets are calibrated to one host
  with mixed core types** (Desired End State).

### Standards

**Summary**: The plan follows the repo's conventions closely. Every mise
task and `measure:build` flag exists, the precedents are followed, and the
VCS commands use jj. The gaps are about the comment policy, plus small
inconsistencies in pyproject comments and module placement.

**Strengths**: The task names and flags are real. The new argv test mirrors
its precedent. The `scripts/` placement fits the Vitest and tsconfig setup.
The jj idioms are right. Stale comments are removed. The `dev` group and
`uv.lock` are handled correctly.

**Findings**:
- 🟡 major / high — **Kept driver comment will contradict the shortened
  defaults** (Phases 1–2). Rewrite or delete it in Phase 2.
- 🔵 minor / medium — **Playwright project comments are only partly
  revised** (Phases 4 and 6). Replace chromium's block with one statement of
  the invariant.
- 🔵 minor / medium — **Claimed pyproject comment convention doesn't match
  the `dev` group** (Phase 3, Section 3). Add the dependency with no
  comment.
- 🔵 minor / medium — **Config-imported library module placed in
  `scripts/`** (Phase 6, Section 1). Either justify the location or use a
  `config/` directory.
- 🔵 suggestion / medium — **First use of a per-file `@vitest-environment`
  pragma** (Phase 5, Section 1). Drop it if jsdom works, which it does.
- 🔵 suggestion / low — **File for the shared stable-fixture constant not
  named** (Phase 4, Section 2).

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-11-15

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Correctness / Architecture**: `stop_cleanly` records a forced kill of
  a zombie in test 6 — Resolved
- 🟡 **Correctness**: `@vitest-environment node` breaks the jsdom setup —
  Resolved
- 🟡 **Correctness**: The `jq` overlap check truncates `startTime` —
  Resolved
- 🟡 **Correctness**: The `rg` sleep guard can never pass — Resolved
- 🟡 **Correctness**: `grace_kill` 0.5 also bounds `kill_arbiter`'s wait
  after SIGTERM — Resolved
- 🟡 **Test Coverage**: `stop_cleanly` ignores survivors — Resolved
- 🟡 **Test Coverage**: The safety net is never shown to fail — Partially
  resolved (the escalation-record assertions are still never the ones that
  fail)
- 🟡 **Test Coverage**: Fixture independence isn't deterministic — Resolved
- 🟡 **Architecture**: The audit narrows Requirement 2 silently — Resolved
- 🟡 **Code Quality / Architecture**: The lifecycle recorder is built from
  string templates — Resolved
- 🟡 **Code Quality / Architecture**: The margin constant is chromium's
  worker count — Resolved in the plan (the work item still disagrees; see
  below)
- 🟡 **Code Quality**: Absence checks can go vacuous — Partially resolved
  (`did not become active` also appears in a docstring at
  `lifecycle.py:532`)
- 🟡 **Performance**: The teardown runs a second stop against a dead
  arbiter — Resolved
- 🟡 **Performance**: Per-task metrics hide cost moved onto sibling lanes —
  Partially resolved (the figures are recorded, but no rule acts on them)
- 🟡 **Performance**: Pools are sized to the whole host — Partially resolved
  (see the new finding on the `-n` comparison)
- 🟡 **Standards / Code Quality**: The driver comment goes stale — Resolved
- 🔵 **Correctness / Test Coverage / Architecture**: The arbiter SIGTERM
  after `grace_quit` escapes the recorder — Partially resolved (only the
  arbiter in `dev.json` at stop time is checked)
- 🔵 **Portability**: The fixture audit is macOS-only — Partially resolved
  (the `strace -p` attach needs root under Yama and misses start-up opens)
- 🔵 **Portability**: Worker share and margin have no runtime override —
  Partially resolved (the plan doesn't say a code change is deliberate)
- 🔵 All other minor findings and suggestions from pass 1 — Resolved

### New Issues Introduced

- 🟡 **Test Coverage**: The escalation-record assertions are never the ones
  a self-test makes fail. In the forced-kill self-test, the "stopped without
  SIGTERM" assertion fires first. The `kill` and `terminate` assertions have
  no messages, so removing either one leaves the suite green.
- 🟡 **Correctness / Test Coverage**: The survivor self-test asserts before
  the child can have logged `started`, so `recorded_fakes` is empty. The
  check then fails on the missing `escalations.log`, not on the survivor.
- 🟡 **Correctness / Test Coverage**: Nothing `up` waits for depends on the
  fakes' `begin()` or their children's `started`. Test 16's
  SIGTERM-ignoring frontend can die on the default disposition, and an
  unrecorded child can make the SIGTERM check vacuous or the survivor check
  flaky.
- 🟡 **Standards**: The work item's Requirement 1 and closing criterion
  still call the margin a tunable lever.
- 🟡 **Standards**: The work item's Playwright acceptance criterion still
  asks for one chromium `parallelIndex`.
- 🟡 **Performance**: On engywook (16 cores, no SMT), `-n auto` and
  `-n logical` both resolve to 16 workers, so the load comparison only
  measures noise. Compare against a smaller count such as `-n 8`.
- 🔵 **Architecture / Correctness**: The check that the arbiter was never
  SIGTERMed only covers the arbiter in `dev.json` at stop time. Assert
  instead that every `terminate` names a recorded fake.
- 🔵 **Code Quality / Architecture / Test Coverage**: The evidence guard can
  be satisfied by a docstring. Match only string constants (via `ast`), or
  use a more specific fragment.
- 🔵 **Correctness / Test Coverage**: Test 6's crafted server writes
  `server.log` into the repository's working directory. Pass
  `cwd=workspace`.
- 🔵 **Correctness**: `_reap_survivors` never kills a surviving circusd,
  which can leak arbiters after a failed test.
- 🔵 **Correctness**: `kill_out_of_band` must list descendants before it
  kills the parent.
- 🔵 **Correctness**: `probe_timeout` also bounds the reuse gate and the
  watcher-PID recording, and a timeout there fails silently.
- 🔵 **Code Quality**: The fake_process CLI shape (positional role or
  `--role`) is inconsistent. `stop_cleanly` defaults a missing arbiter to
  `None`. The evidence file names are a contract spread across three
  modules. `killed_by_test` is missing from the helpers list.
- 🔵 **Code Quality**: Phase 4's config comment describes concurrency that
  only arrives in Phase 6.
- 🔵 **Standards**: The work item's Context and Drafting Notes still state
  the file-level rule. The amended Requirement 2 has an 81-column line.
- 🔵 **Performance**: Three-run measurements are too noisy to declare a
  winner, and sibling-lane figures feed no rule.
- 🔵 **Portability**: Identity floats are compared exactly when they should
  use `START_TIME_TOLERANCE`. The `\s` in `grep -E` is a GNU extension; use
  `[[:space:]]`. There is no portable command to pick a free port.
- 🔵 **Performance**: The `unverified_stop_workspace` teardown still pays a
  full `probe_timeout` when no stack exists.
- 🔵 **Correctness**: The `node:os` partial mock must match the config's
  import style.

### Assessment

Pass 2 resolves almost all of pass 1. Every critical-path design problem is
fixed: the zombie forced kill, the jsdom crash, the `jq` and `grep`
criteria, the template fakes, the duplicated stop, and the margin invariant.
The six new major findings are narrower. Three are race or ordering details
in the new test harness, which would surface as harness flakes rather than
masked failures. Two are about bringing the work item into line with the
plan. One is a measurement that can't discriminate on engywook. All six are
mechanical fixes. The verdict stays REVISE under the major-count threshold,
but the plan should be close to ready after one more round of edits.

## Re-Review (Pass 3) — 2026-11-15

**Verdict:** COMMENT

Lenses re-run: test coverage, correctness, standards and performance. These
are the lenses that raised pass 2's major findings.

### Previously Identified Issues

- 🟡 **Test Coverage**: The escalation-record assertions are never the ones
  that fail — Resolved
- 🟡 **Correctness / Test Coverage**: The survivor self-test asserts too
  early — Resolved
- 🟡 **Correctness / Test Coverage**: The fakes' readiness isn't ordered
  against `up` — Resolved
- 🟡 **Standards**: The work item still calls the margin tunable — Resolved
- 🟡 **Standards**: The work item asks for one chromium `parallelIndex` —
  Resolved
- 🟡 **Performance**: `-n auto` and `-n logical` can't be told apart on
  engywook — Resolved (the comparison is now against `-n 8`)
- 🔵 **Architecture / Correctness**: The arbiter-SIGTERM check only covers
  the arbiter in `dev.json` — Resolved
- 🔵 **Code Quality / Architecture / Test Coverage**: The evidence guard can
  be satisfied by a docstring — Resolved
- 🔵 **Correctness / Test Coverage**: Test 6 writes into the repo's working
  directory — Resolved
- 🔵 **Correctness**: `_reap_survivors` doesn't kill a surviving circusd —
  Partially resolved (an arbiter whose `dev.json` is gone is still
  unreachable)
- 🔵 **Correctness**: `kill_out_of_band` ordering; `probe_timeout`
  consumers; `node:os` mock style — Resolved
- 🔵 **Performance**: Sibling-lane figures feed no rule; three-run noise;
  the `unverified_stop_workspace` teardown probe — Resolved
- 🔵 **Standards**: The work item's Context, Drafting Notes and line width —
  Resolved

### New Issues Introduced

- 🔵 **Test Coverage / Correctness**: Test 6 keeps its 0.5 s sleep and its
  `assert child_pids` through Phase 1, against a fake that is slower to
  start. Under load, the child can be SIGTERMed before its `begin()` runs.
  Move the record-based wait into Phase 1.
- 🔵 **Correctness**: `_reap_survivors` can't reach an arbiter whose
  `dev.json` is gone (test 10) but still removes its sockets. Also read
  `circusd.pid`, and check the process is a circusd before killing it.
- 🔵 **Correctness**: The plan doesn't say that `fake_process.py` imports
  only the standard library and psutil, or that it has a `__main__` entry.
  Test 6 and the survivor self-test run it directly, without the repo root
  on `sys.path`.
- 🔵 **Standards**: In the work item, part of the "make independent"
  Drafting Note is spliced onto the margin bullet and says "existing config
  comment", where the plan says it is revised.
- 🔵 **Standards**: The work item's timing criteria leave out
  `--host-settings devtools`.
- 🔵 **Standards**: Phase 1 claims that types are checked on
  `fake_process.py`, but pyrefly only covers `tasks/**`.
- 🔵 **Performance**: The two arms of a load comparison aren't pinned to
  one commit or to the other track's state.
- 🔵 Suggestions:
  - **Test Coverage**: assert `Escalation("terminate", server_pid)` in the
    forced-kill self-test.
  - **Performance**:
    - say that the `default` median decides the `-n 8` comparison;
    - give the driver's post-`up` wait a short poll interval;
    - consider counting the measurement runs towards the 10-run validation.
  - **Standards**:
    - fix a few over-80 lines;
    - change "ten consecutive green" to "ten counted" runs.

### Assessment

No major or critical findings remain, so the verdict moves to COMMENT. The
plan is sound and ready to implement. What remains is small and mechanical:
the test 6 wait in Phase 1, the `circusd.pid` fallback, and some wording and
consistency fixes in the plan and the work item. It is worth applying these
before implementation, but they don't call for another review pass.

## Verdict Change — 2026-11-15

**Verdict:** APPROVE

The remaining pass 3 minors and suggestions have been applied to the plan
and the work item. The plan's author approved it for implementation, and
the plan's status is now `ready`.
