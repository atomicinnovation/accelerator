---
type: "pr-description"
id: "220"
title: "[0364] Make the dev suite's timeout branches observable"
date: "2026-11-15T23:34:08+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0364"
parent: "work-item:0364"
relates_to: ["plan:2026-11-15-0364-parallelise-the-serial-test-suites", "work-item:0371"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/220"
pr_number: 220
tags: ["build-system", "testing", "pytest", "dev-task", "flakiness"]
revision: "5587c80fb2e112c5f924605e60345449ef4d30f1"
repository: "accelerator"
last_updated: "2026-11-15T23:34:08+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0364] Make the dev suite's timeout branches observable

## Summary

This PR implements Phase 1 of the 0364 plan, ahead of shortening the dev
integration suite's waits (Phase 2) and running it in parallel (Phase 3).
Until now a stop that escalated to SIGKILL looked exactly like a clean one.
`kill_arbiter` returns `"clean"` whichever path it took, and
`reap_descendants` logs nothing. So shorter waits could have hidden
escalations without any test noticing.

The suite now records evidence that makes escalation visible:

- every fake logs when it starts and when it receives SIGTERM;
- the driver logs every signal our code sends to a live process;
- every happy-path stop asserts that it was graceful;
- each negative-path test asserts output that only its own branch produces.

Timings are unchanged on purpose. Even so, the serial suite now takes
135–139 s, down from 163 s, because the teardown no longer runs a second
driver stop.

## Changes

### Fakes as a real module

- `tests/integration/dev/fake_process.py` (new) replaces the inline source
  templates. It has three behaviours: `server`, `frontend` and `child`.
- **Start-up order.** A fake installs its SIGTERM disposition, then appends
  `started <role> <pid> <create_time>` to `fakes.log`. If it spawns a child,
  it waits for the child's `started` line. Only then does it write its
  marker and `server-info.json`. So `started` always means "will log its
  SIGTERM, unless told to ignore it", and the readiness gate can't pass
  before both the server and its child are recorded.
- On SIGTERM a fake appends `sigterm …` and exits at once.
- It imports only the standard library and psutil, because test 6 and a
  self-test run it directly, without the repo root on `sys.path`.

### Driver

- **Launchers.** One launcher template replaces the four, and keeps the
  absolute-interpreter shebang the stripped-PATH test relies on.
- **`up` and `restart`** return only once every configured role has a
  running recorded fake, waiting up to 20 s. Circus reports the frontend
  watcher active before the fake has set up its SIGTERM handling, so
  without this wait a stop could race it.
- **`EscalationRecorder`** wraps `PsutilProcessOps` and appends to
  `escalations.log` on each `terminate`/`kill`, but only when the target is
  alive. `identity_matches` accepts zombies, so teardown routinely signals
  fakes that have already exited.
- **Shared parsing.** `RecordedFake` and `recorded_fakes` parse the events
  log. The tests import them from the driver.
- The duplicated comment above the timeout defaults is gone.

### Tests

- **`assert_stopped_cleanly`** checks four things in order, each with its
  own message:
  1. our code SIGKILLed no live process;
  2. our code SIGTERMed nothing but recorded fakes, which covers every
     arbiter;
  3. no recorded fake survived;
  4. every fake the test didn't kill out of band logged `sigterm`.

  It then checks that `dev.log` holds none of the timeout-branch evidence
  strings.
- **`stop_cleanly`** stops through the driver and then runs those checks.
  Tests 4, 5, 6, 10, 11 and 17 use it. The `workspace` fixture runs it for
  any stack still recorded at teardown, so tests that never stop are
  checked too.
- **Fixtures.** `unverified_stop_workspace` (tests 8, 9, 12, 16 and the
  stack-based self-test) keeps the old best-effort driver stop. Both
  fixtures finish with `_reap_survivors`, which kills surviving fakes and a
  surviving circusd, found through `dev.json` or `circusd.pid`.
- **Out-of-band kills.** `kill_out_of_band` lists a process's whole subtree
  and records it before killing anything. Tests 11 and 15 use it.
- **Test 6** builds its crafted server from `fake_process.py`, with
  `cwd=workspace`. It waits for the server and its child to be recorded,
  instead of sleeping 0.5 s.
- **Harness self-tests**, each pinned with `pytest.raises(match=…)`:
  - a forced SIGKILL from a real stack;
  - a SIGTERM of a non-fake;
  - a surviving fake;
  - an `ast` guard that fails if any evidence string stops occurring among
    `lifecycle.py`'s string constants (docstrings excluded).

## Negative-path record

| Test | Timeout branch | Output only that branch produces |
|---|---|---|
| `test_readiness_timeout_fails_and_tears_down` (8) | readiness budget expiry (`_readiness_gate`) | message `server did not write {server_info_path} in time`; `dev.log` line `server-info.json did not appear before the readiness timeout` |
| `test_daemon_startup_failure_reaps_handle_no_retry` (9) | pidfile budget expiry (`allocate_and_launch`, no pidfile) | message `the circus daemon did not start (no pidfile at`; `dev.log` line `circusd did not write a valid pidfile at` |
| `test_sigterm_ignoring_frontend_is_sigkilled` (16) | circus's own SIGTERM→SIGKILL escalation after `graceful_timeout` | the frontend was alive before the stop and dead after it, it logged no SIGTERM receipt, and our code recorded no escalation against it |

Tests 8 and 9 also assert that the other test's `dev.log` line is absent.

`test_recycled_pid_is_refused_real_psutil` (12) is a negative-path test,
but it covers identity refusal, not a timeout branch.

No test exercises four branches. Their outputs are part of every
happy-path absence check:

- `kill_arbiter`'s SIGKILL escalation (`still alive after SIGKILL`);
- `kill_arbiter`'s SIGTERM after `grace_quit` expires. It logs nothing, so
  the escalation record catches it instead;
- a pid-unknown arbiter that stays reachable (`endpoint still reachable
  after quit`);
- frontend-active budget expiry (`did not become active`).

All five evidence strings match `tasks/shared/dev/lifecycle.py`, and
`test_every_timeout_branch_evidence_line_is_still_logged` keeps it that way.

### Known gaps (production, out of scope)

Both are raised as **0371** (Linear PP-942), which does not block 0364:

- **Out-of-band kill.** When a watcher is SIGKILLed out of band, its
  children are reparented, and `reap_descendants` only lists the children
  of recorded PIDs. Tests that kill fakes out of band kill the whole
  subtree.
- **Lost `dev.json`.** Circus's `stop_children` lists a watcher's children
  only after signalling the watcher (`send_signal_process` →
  `send_signal_child`). A watcher that exits promptly therefore leaves its
  children unsignalled. On the normal path `reap_descendants` covers this
  through the recorded PIDs; the suite's escalation logs show its
  `terminate` lines naming only the children. Without `dev.json` there are
  no recorded PIDs, so the children survive.

## Context

- Work item: `meta/work/0364-parallelise-the-serial-test-suites.md`
- Plan: `meta/plans/2026-11-15-0364-parallelise-the-serial-test-suites.md`
  (Phase 1)
- Research: `meta/research/codebase/2026-11-15-0364-parallelise-the-serial-test-suites.md`
- Follow-up: `meta/work/0371-dev-stack-teardown-orphans-watcher-children.md`

Besides Phase 1, this branch carries the commits that lead up to it:

- 0363 marked done;
- a warm no-op build measurement taken after 0362 and 0363;
- the 0364 refinement and its review;
- the codebase research;
- the plan and its review.

## Testing

- [x] Red step observed before the harness existed: 9 tests failed and 7
      errored.
  - The missing escalation log failed every `stop_cleanly`.
  - Test 16 found no recorded frontend, because the events log was never
    written.
  - Running `fake_process.py` failed, because the module didn't exist yet.

  The evidence-guard and non-fake self-tests passed from the start, as
  expected, since neither needs the recorders.
- [x] `mise run test:integration:dev` passes serially: 21 passed, in
      135–139 s across runs (baseline 163 s).
- [x] `mise run build-system:check` and `mise run check` pass.
- [x] The suite writes nothing outside its `tmp_path`: the repo's
      `.accelerator/tmp/visualiser/server.log` is unchanged by a run.
- [x] Evidence inspected by hand:
  - Test 16 leaves `started frontend <pid> …` with no `sigterm` for that
    PID, and no `kill` line.
  - Test 4 leaves an escalation log with no `kill` line.

## Notes for Reviewers

- **Deviations from the plan.**
  - Test 10 runs childless watchers (`server_spawn_child` and
    `fe_spawn_child` false). It tests finding the arbiter after `dev.json`
    is lost, and children can't be reached on that path (see the known
    gaps above).
  - The recorded-fake parsing lives once, in the driver, and the tests
    import it. The plan had it duplicated in both.
- **Where to look.**
  - The order of the checks in `assert_stopped_cleanly`.
  - The alive-only rule in `EscalationRecorder`.
  - The recorded-fakes wait in the driver, which closes the race between
    circus reporting a watcher active and the fake installing its SIGTERM
    handling.
- **What's next.** Phase 2 lowers the driver's `probe_timeout` and
  `grace_kill` defaults and replaces the remaining settle sleeps (tests 5
  and 11). These assertions are what will catch any escalation the shorter
  waits cause.
