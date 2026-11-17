---
type: "plan"
id: "2026-11-15-0364-parallelise-the-serial-test-suites"
title: "Parallelise the Serial Test Suites Implementation Plan"
date: "2026-11-15T16:47:37+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "in-progress"
work_item_id: "work-item:0364"
parent: "work-item:0364"
derived_from: ["codebase-research:2026-11-15-0364-parallelise-the-serial-test-suites"]
relates_to: ["work-item:0361", "work-item:0362", "work-item:0368", "work-item:0369", "work-item:0370", "work-item:0371"]
tags: ["build-system", "performance", "testing", "playwright", "pytest", "pytest-xdist", "flakiness"]
revision: "9737db7bc8c81586b1c512387e44b6da0121c318"
repository: "accelerator"
last_updated: "2026-11-17T08:21:09+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Parallelise the Serial Test Suites Implementation Plan

## Overview

Run the Playwright visualiser suite and the dev-task integration suite with
host-relative worker counts, instead of serially. Turn Playwright retries off
everywhere, and shorten the dev suite's pure-delay waits. The work runs as two
independent tracks of three phases each, plus a closing phase per suite.
Phases 1–6 are each their own PR and can be merged on their own. Phase 7
records its results in the PRs of Phases 3 and 6. It opens PRs only for a
paired comparison, a retune or a revert. In each track the worker change
goes last, so that phase is "the suite's change" for the 10-run validation
(Requirement 7) and for the CI windows (Requirement 8).

## Progress

| Phase | Track | PR | State |
|---|---|---|---|
| 1. Observable timeout branches | Dev | #220 | Merged 2026-11-16 |
| 2. Shorter pure-delay waits | Dev | #221 | Merged 2026-11-16 |
| 3. pytest-xdist workers | Dev | #222, #230 | #222 merged 2026-11-16, without a full CI before window; #230 (CI CPU counts) open |
| 4. Fixture isolation | Playwright | #225 | Merged 2026-11-16 |
| 5. Retries off | Playwright | #229 | Open |
| 6. Host-relative workers | Playwright | — | Validated on `c8cf7e5e`; PR not yet raised |
| 7. CI closing comparison | Both | — | Not started |

## Current State Analysis

**Playwright** (`cli/visualiser/frontend/playwright.config.ts`)
- **Settings.** `retries: 1` (line 9), global `workers: 1` (line 14) and
  `trace: "on-first-retry"` (line 20).
- **Stale comment.** The comment above `workers` (lines 10–13) cites the
  visual-regression specs, which now live in `playwright.docker.config.ts`.
- **Projects.** There are two: `resolved-styles` (313 tests in 17 files) and
  `chromium` (about 43 tests, including the drag tests).
- **Fixtures are served in place.** `e2e/start-server.mjs` serves
  `cli/visualiser/server/tests/fixtures/meta` directly. `global-setup.ts` and
  `global-teardown.ts` snapshot and restore `meta/work/*.md` once per run.
- **What `chromium` writes.** Its specs write 0001 (`kanban`,
  `kanban-click-vs-drag`, and `default-pattern`, which never restores it),
  0005 (`kanban`, `kanban-drag-overlay`), 0006 (`kanban-conflict`) and 0003
  (`legacy-schema`, an identical-content rewrite).
- **What `resolved-styles` reads.**
  - It opens 0001 through `DETAIL_ROUTE_SLUGS["work-items"]`
    (`tests/lib/detail-route-slugs.ts:22`).
  - It probes `/kanban` cards with `.first()`:
    `typography-resolved-sizes.spec.ts:136-151,165-170` (card title and id)
    and `radius-resolved-radii.spec.ts:134-140,153-155` (stage dots).
  - It also visits index-served aggregates: `/library`,
    `/library/work-items` and `/lifecycle*`.
  - It writes nothing.
- **Kanban cards are addressable.** Each card renders as
  `li[data-relpath="meta/work/<file>.md"]`
  (`src/routes/kanban/WorkItemCard.tsx:94`), and the stage dots sit inside
  the card
  (`WorkItemCardPresentation.tsx:67`).
- **Vitest picks up `scripts/`.** Vitest collects `**/*.test.ts` but excludes
  `e2e/` and `tests/` (`vite.config.ts:79-87`). `scripts/` is collected and
  covered by `tsconfig.node.json`, and
  `scripts/visual-diff-ciede2000{,.test}.ts` is the precedent.

**Dev-task suite** (`tests/integration/dev/`)
- **How it runs.** `tasks/test/integration.py:57` runs `uv run pytest
  tests/integration/dev -v`, serially.
- **Driver defaults.** The driver's defaults
  (`dev_integration_driver.py:191-196`) are `probe_timeout` 5.0 and
  `grace_kill` 3.0. Production uses 2.0 and 2.0
  (`tasks/shared/dev/lifecycle.py:76-86`).
- **Duplicated comment.** The 7-line comment above the defaults appears twice
  (lines 172–185).
- **`grace_kill` is a fixed dwell.** `reap_descendants` sleeps for the whole
  `grace_kill` on every clean teardown (`lifecycle.py:776`).
- **`probe_timeout` is spent per dead arbiter.** Each dead-arbiter probe costs
  the full `probe_timeout` (`_stop_without_state`, `lifecycle.py:869`). The
  same value also bounds live arbiter calls.
- **Escalation is invisible.** `kill_arbiter` returns `StopResult("clean")`
  whatever path it took, including SIGTERM after `grace_quit` and SIGKILL
  (`lifecycle.py:738-741`). The driver prints `{kind, pid}` for stop, and
  `reap_descendants` logs nothing. So escalation can't be seen today.
- **The killer can be swapped.** The driver injects
  `killer=PsutilProcessOps()` (driver 152). `DevDeps.killer` is typed as
  `ProcessOps`, and the dev lifecycle escalates only through
  `terminate(pid)` and `kill(pid)`. It never calls `signal_group`.
- **Zombies pass the identity check.** `identity_matches` compares only
  `create_time`, so it accepts a zombie. `is_alive` excludes zombies
  (`tasks/shared/processes.py:19-34,59-66`). `reap_descendants` therefore
  calls `kill` on fakes that have already exited but not yet been reaped.
- **Fakes are templated source.** The driver renders the fakes from Python
  source templates (`_SERVER_TEMPLATE`, `_NPM_TEMPLATE`, `_SPAWN`), and their
  children run an inline `-c` sleep.
- **Settle sleeps.** There are three fixed 0.5 s sleeps, at
  `test_dev_integration.py:210,235,309`.
- **Two weak negative-path assertions.**
  - Test 9's assertion (`"circusd.log" in message`, line 292) also matches its
    sibling branch.
  - Test 16 (line 384) asserts only that the frontend died.
- **Per-test isolation.** Each test owns `tmp_path / "ws"`, its IPC sockets
  are keyed by workspace, and no test binds a port. xdist isolation holds.
- **Dependencies.** The `pyproject.toml` `dev` group (lines 39–61) has no
  `pytest-xdist`, and installs use `uv sync --all-groups --frozen`.

## Desired End State

**Playwright**
- **Worker counts.** They are derived from `os.availableParallelism()`
  through a pure, unit-tested function. `resolved-styles` gets
  `max(1, ⌊N/2⌋)` workers, `chromium` gets 1, and the global cap is
  `resolved-styles + 1`.
- **Retries and traces.** `retries` is 0 and traces are kept on failure, with
  and without `CI`.
- **No fixture conflicts.** No `resolved-styles` probe measures an element
  that comes from a fixture a `chromium` spec writes. This uses Requirement
  2's per-probe definition of a conflict. The fixture audit and a run
  against mutated fixtures are recorded in the PR.
- **Speed.** `test:e2e:visualiser` has a median under 75 s on engywook (warm
  no-op, isolated, 5 runs, `devtools`).

**Dev-task suite**
- **Parallel run.** `test:integration:dev` runs `pytest -n auto` with
  `pytest-xdist` declared.
- **Shorter waits.** The driver's defaults are `probe_timeout` 2.0 and
  `grace_kill` 0.5. Every settle sleep is a condition wait.
- **Negative-path assertions.** Each negative-path test asserts output that
  only its own branch produces.
- **Happy-path assertions.** Every happy-path stop asserts four things:
  - no recorded fake survives;
  - every fake the test did not kill out of band logged its SIGTERM;
  - our code neither SIGKILLed a live process nor SIGTERMed anything but a
    recorded fake, which covers every arbiter the test ran;
  - no timeout-branch log line appears.

  Harness self-tests show that this check fails when it should.
- **Speed.** `test:integration:dev` has a median under 60 s on engywook.

**Both suites**
- **Validation.** Ten counted `mise run` executions on one recorded commit,
  as Requirement 7 defines them.
- **No masking.** No `skip`, `fixme`, retries or rerun markers are added.
- **CI.** CI medians are no more than 5% slower on ubuntu and on macOS, judged
  by the windows or the paired comparison.

### Key Discoveries:

- `tasks/shared/processes.py:19-34`: `pid_identity_matches` accepts zombies,
  so a recorder of escalations must check `is_alive` before it records.
- `cli/visualiser/frontend/src/test/setup.ts:55-86`: the shared Vitest setup
  writes to `window`, so frontend tests stay on `jsdom`.
- `cli/visualiser/frontend/src/routes/kanban/WorkItemCard.tsx:94`:
  `data-relpath` gives each kanban card a stable selector that probes can be
  pinned to.
- Work items 0002 (`done`), 0004 (`in-progress`) and 0007–0010 are written by
  no spec.
- `tasks/shared/dev/lifecycle.py:334-345,406-416,453,566,746-749`: the exact
  log lines and messages each timeout branch produces.
- `tests/unit/tasks/test_hooks_task_environment.py`: the precedent for
  asserting a task's argv with `mocker.MagicMock(spec=Context)`.
- `tasks/shared/build_measurement/settings.py:96-105`: `measure:build`
  defaults `--host-settings` to `unspecified`. Every invocation here must pass
  `--host-settings devtools`.

## What We're NOT Doing

- **The Docker lane.** No changes to `playwright.docker.config.ts` or
  `test:e2e:visualiser:docker`.
- **The drag tests.** No changes to the `chromium` specs, no per-test fixture
  copies for them, and no fix to `legacy-schema.spec.ts`'s likely-skipping
  lookup.
- **Automated audit guard.** No check that keeps the fixture audit true for
  future specs. The revised config comment is the reminder.
- **Production code.** No changes to `tasks/shared/dev/lifecycle.py`, its
  production defaults, or the readiness, pidfile and frontend-active budgets.
- **Tests for untested branches.** No new tests for production branches that
  nothing exercises today (`kill_arbiter` escalation, frontend-active expiry,
  lock contention). The record lists them, and the happy-path assertions
  check that their outputs never appear. Harness self-tests that prove the
  test-side evidence works are in scope.
- **Orphaned children after an out-of-band kill.** `reap_descendants` can't
  reach the children of a fake that was SIGKILLed out of band. This is a
  production gap, raised as 0371.
- **Other pytest lanes.** No parallelism for any other pytest lane, and no
  `pytest-timeout`.
- **Not upfront.** No `fullyParallel` and no explicit `-n`. Each is a tuning
  lever, used only if a measurement calls for it.

## Implementation Approach

**Two tracks, ordered so that each phase is safe on its own.**
- **Dev-task track.** Observability goes first, so the wait cuts in Phase 2
  have a safety net that would catch a masked escalation. Parallelism comes
  last, once the waits are short.
- **Playwright track.** Fixture isolation goes first, because it's harmless
  while the suite runs serially. Retries come off next, still on one worker,
  so flakes from losing retries surface apart from flakes caused by
  parallelism. Workers go last.

**Test-first throughout.**
- **Dev suite.** Its new assertions and harness self-tests are written first
  and fail against today's fakes.
- **Playwright worker counts.** The worker-count function and the config are
  driven by Vitest tests.
- **The `-n auto` flag.** It is driven by a task-argv unit test.

**Windows follow the worker phases.** Phases 2 and 5 merge before their
track's worker phase, so the CI before window (the last 10 main runs before
landing) already includes them. The CI comparison then judges parallelism on
its own, which matches the work item's revert rule: undo the workers and keep
the retries policy and the shorter waits.

---

## Phase 1: Make the dev suite's timeout branches observable

### Overview

Add test-side evidence of SIGTERM receipt and of escalation by our code.
Write the negative-path record, and make every test assert output that only
its own branch produces. Harness self-tests prove that the evidence fails
when it should. Timings stay unchanged.

### Changes Required:

#### 1. Negative-path record (written first, kept in the PR description)

| Test | Timeout branch | Output only that branch produces |
|---|---|---|
| `test_readiness_timeout_fails_and_tears_down` | readiness budget expiry (`_readiness_gate`) | message `server did not write {server_info_path} in time`; `dev.log` line `server-info.json did not appear before the readiness timeout` |
| `test_daemon_startup_failure_reaps_handle_no_retry` | pidfile budget expiry (`allocate_and_launch`, no pidfile) | message `the circus daemon did not start (no pidfile at`; `dev.log` line `circusd did not write a valid pidfile at` |
| `test_sigterm_ignoring_frontend_is_sigkilled` | circus's own SIGTERM→SIGKILL escalation after `graceful_timeout` | the frontend was alive before stop and dead after it, it logged no SIGTERM receipt, and our code recorded no escalation against it |

The record also notes:
- `test_recycled_pid_is_refused_real_psutil` is a negative-path test but
  targets no timeout branch: it covers identity refusal.
- No test exercises four branches:
  - `kill_arbiter`'s SIGKILL escalation (`still alive after SIGKILL`);
  - `kill_arbiter`'s SIGTERM after `grace_quit` expires (no log line; the
    escalation record catches it);
  - a pid-unknown arbiter that stays reachable (`endpoint still reachable
    after quit`);
  - frontend-active budget expiry (`did not become active`).

  Their outputs are part of the happy-path absence check.
- **A known gap, not a timeout branch.** After an out-of-band SIGKILL of a
  fake, its children are orphaned. `reap_descendants` enumerates the children
  of recorded PIDs, so it cannot reach them. This is production behaviour,
  so it is out of scope here. It is raised as 0371 (Linear PP-942), which
  does not block 0364. Tests that kill fakes out of band kill the whole subtree.
- **The same gap when `dev.json` is lost.** Circus's `stop_children` lists a
  watcher's children only after signalling the watcher, so a watcher that
  exits promptly leaves its children unsignalled. Normally
  `reap_descendants` reaches them through the recorded PIDs. Without
  `dev.json` there are none, so the children survive. Test 10 therefore runs
  childless watchers, and 0371 covers this case too.

Before Phase 2, the implementer confirms each string against
`lifecycle.py`. If the record turns up another branch, it joins the list and
gets the same treatment.

#### 2. The fakes become a real module

**File**: `tests/integration/dev/fake_process.py` (new)

**Changes**:
- **Command line.** `fake_process.py <behaviour> --role <label> --events-log
  <path> [flags]`.
  - `<behaviour>` is `server`, `frontend` or `child`.
  - `--role` is the label recorded in the events log. It defaults to the
    behaviour, and children are labelled `<parent role>-child`.
  - The flags are `--ignore-sigterm`, `--spawn-child` and `--marker`. The
    server also takes `--port` and `--write-info`.
- **Behaviour.** It keeps today's: the server writes its marker to
  `server.log` under its working directory's composed state dir and writes
  `server-info.json`, and the frontend prints its marker.
- **Order of start-up.** This is what makes the record trustworthy.
  1. `begin()` installs the SIGTERM disposition, then appends
     `started <role> <pid> <create_time>` to the events log. So `started`
     always means "will log its SIGTERM unless told to ignore it".
  2. If `--spawn-child` is set, the fake spawns its child by running this
     module again (`sys.executable`, `__file__`). It waits until the child's
     `started` line is in the events log.
  3. Only then does the fake write its marker and `server-info.json`.

  So when the readiness gate sees `server-info.json`, the server and its
  child are both recorded. A child never ignores SIGTERM. On SIGTERM, a fake
  appends `sigterm …` and exits with `os._exit(0)`.
- **Imports and entry point.** The module imports only the standard library
  and psutil, and it ends with `if __name__ == "__main__":
  main(sys.argv[1:])`. Test 6 and the survivor self-test run it directly,
  without the repo root on `sys.path`, so it can't import from `tasks` or
  `tests`. The parsing of recorded fakes lives in the driver, and the tests
  import it from there.
- **Shared names.** The module defines the file names the harness shares:
  `EVENTS_LOG = "fakes.log"`, `ESCALATIONS_LOG = "escalations.log"` and
  `KILLED_BY_TEST_LOG = "killed-by-test.log"`. The driver, the launchers and
  the tests import them from `tests.integration.dev.fake_process`, as
  `tests/unit/tasks/` already imports `tests.unit.tasks.shared.doubles`.

```python
class FakeLifecycle:
    def __init__(self, role: str, events_log: Path) -> None:
        self._role = role
        self._events_log = events_log
        self._identity = f"{os.getpid()} {psutil.Process().create_time()}"

    def begin(self, *, ignore_sigterm: bool) -> None:
        handler = signal.SIG_IGN if ignore_sigterm else self._on_sigterm
        signal.signal(signal.SIGTERM, handler)
        self._record("started")

    def spawn_recorded_child(self) -> None:
        child = subprocess.Popen([
            sys.executable, __file__, "child",
            "--role", f"{self._role}-child",
            "--events-log", str(self._events_log),
        ])
        while f"started {self._role}-child {child.pid} " not in self._log():
            time.sleep(0.01)

    def _on_sigterm(self, signum, frame) -> None:
        self._record("sigterm")
        os._exit(0)

    def _record(self, event: str) -> None:
        with self._events_log.open("a") as log:
            log.write(f"{event} {self._role} {self._identity}\n")

    def _log(self) -> str:
        return self._events_log.read_text()
```

**File**: `tests/integration/dev/dev_integration_driver.py`

**Changes**:
- **Thin launchers.** `_SERVER_TEMPLATE`, `_NPM_TEMPLATE`, `_IGNORE` and
  `_SPAWN` are replaced by one launcher template.
  - It keeps the absolute-interpreter shebang, so the stripped-PATH test
    still holds.
  - It puts the repo root on `sys.path` and calls `fake_process.main` with an
    argv built from `opts`.
  - Circus's own arguments (`serve --owner-pid 0`, `--prefix … run dev`)
    are ignored.
  - The events log is `workspace / EVENTS_LOG`.
- **`up` and `restart` return only once every fake is recorded.** Circus
  reports the frontend watcher active before the fake has run `begin()`.
  So after `bring_up` or `do_restart` returns `started` or `reused`, the
  driver waits up to 20 s, polling every 50 ms, for every configured role
  to have a running recorded fake: `server` and `frontend`, plus each one's child unless
  `*_spawn_child` is false. If a role is missing when the wait expires, the
  action fails and names it. A SIGTERM-ignoring fake is therefore always
  ignoring SIGTERM before any test stops it.
- **Remove the duplicate.** Delete the second copy of the duplicated comment
  paragraph. Phase 2 rewrites the copy that remains.

```python
_LAUNCHER = """\
#!{interp}
import sys
sys.path.insert(0, {repo_root!r})
from tests.integration.dev.fake_process import main
main({argv!r})
"""
```

#### 3. The driver records escalation by our code

**File**: `tests/integration/dev/dev_integration_driver.py`

**Changes**:
- Add an `EscalationRecorder` that wraps a `ProcessOps` and delegates every
  method to it.
- `terminate(pid)` and `kill(pid)` append a line to
  `workspace / ESCALATIONS_LOG` only when the target is alive. This rule is
  needed because `identity_matches` accepts zombies, so `reap_descendants`
  routinely signals fakes that have already exited.
- The file is created when the recorder is built.
- Inject `EscalationRecorder(PsutilProcessOps(), …)` as `killer=`.

```python
class EscalationRecorder:
    def __init__(self, inner: ProcessOps, record: Path) -> None:
        self._inner = inner
        self._record = record
        record.touch()

    def terminate(self, pid: int) -> None:
        self._note_if_alive("terminate", pid)
        self._inner.terminate(pid)

    def kill(self, pid: int) -> None:
        self._note_if_alive("kill", pid)
        self._inner.kill(pid)

    def _note_if_alive(self, signal_name: str, pid: int) -> None:
        if self._inner.is_alive(pid):
            with self._record.open("a") as record:
                record.write(f"{signal_name} {pid}\n")
```

The other four `ProcessOps` methods delegate unchanged.

#### 4. The tests assert branch-unique output and clean stops

**File**: `tests/integration/dev/test_dev_integration.py`

**Changes**:
- **Evidence.** `TIMEOUT_BRANCH_EVIDENCE` lists exactly these `dev.log`
  substrings:
  - `server-info.json did not appear before the readiness timeout`
  - `circusd did not write a valid pidfile at`
  - `still alive after SIGKILL`
  - `endpoint still reachable after quit`
  - `did not become active`
- **Helpers.**
  - `wait_until(condition, timeout=5.0)` polls on `monotonic`. Phase 2 uses
    it too.
  - `recorded_fakes(workspace)` parses the events log into `RecordedFake`s.
    Each has a role, an identity `(pid, create_time)` and its events. It
    returns `[]` when the log is absent.
    - `RecordedFake.is_running()` delegates to `PsutilProcessOps`:
      `identity_matches` and `is_alive`, so a zombie counts as not running.
    - `RecordedFake.is_one_of(identities)` matches by PID and by start time
      within `START_TIME_TOLERANCE`, because Linux can shift `create_time`
      between readers.
  - `escalations(workspace)` parses the escalation log into
    `Escalation(signal, pid)`s. It fails if the file is missing, which proves
    the recorder is wired.
  - `kill_out_of_band(workspace, pid)` first lists the process and its
    `children(recursive=True)` with their identities, and records them in
    the killed-by-test log. Only then does it SIGKILL each one. Listing first
    matters, because once the parent is dead its children are reparented and
    can no longer be listed.
  - `killed_by_test(workspace)` reads those identities back. It returns `[]`
    when the log is absent.
  - `assert_no_timeout_branch_output(workspace)` checks that `dev.log`
    contains no string from `TIMEOUT_BRANCH_EVIDENCE`.
- **`assert_stopped_cleanly(workspace)`.** It asserts four things, in this
  order, each with its own message:
  1. our code SIGKILLed no live process (no `kill` line);
  2. our code SIGTERMed nothing but recorded fakes, so no `terminate` names
     a PID outside the events log. Every legitimate `terminate` target in
     `reap_descendants` is a fake or a fake's child, so any other target is
     an arbiter. This covers every arbiter the test ran, including a
     restart's first generation and a stack whose `dev.json` was deleted;
  3. no recorded fake is still running;
  4. every recorded fake that the test did not kill out of band logged
     `sigterm`;

  and then that there is no timeout-branch output. `stop_cleanly(workspace)`
  stops through the driver, asserts success, then calls
  `assert_stopped_cleanly`. The events log is cumulative, so the restart's
  internal stop of the first generation is checked too.

```python
def stop_cleanly(workspace: Path) -> None:
    assert_ok(run_driver(workspace, "stop"))
    assert_stopped_cleanly(workspace)


def assert_stopped_cleanly(workspace: Path) -> None:
    fakes = recorded_fakes(workspace)
    ours = escalations(workspace)
    sigkilled = [e for e in ours if e.signal == "kill"]
    assert not sigkilled, f"our code SIGKILLed: {sigkilled}"
    fake_pids = {fake.pid for fake in fakes}
    non_fakes = [
        e for e in ours if e.signal == "terminate" and e.pid not in fake_pids
    ]
    assert not non_fakes, f"our code SIGTERMed a non-fake: {non_fakes}"
    survivors = [fake for fake in fakes if fake.is_running()]
    assert not survivors, f"survived the stop: {survivors}"
    killed = killed_by_test(workspace)
    unsignalled = [
        fake for fake in fakes
        if not fake.is_one_of(killed) and "sigterm" not in fake.events
    ]
    assert not unsignalled, f"stopped without SIGTERM: {unsignalled}"
    assert_no_timeout_branch_output(workspace)
```

- **Two fixtures.**
  - **`workspace`, for happy-path tests.**
    - If a stack is still recorded at teardown, it runs `stop_cleanly`.
    - In `finally` it runs `_reap_survivors`. That SIGKILLs any recorded
      fake that is still running. It also SIGKILLs a surviving arbiter: the
      one recorded in `dev.json` if its identity matches, or otherwise the
      PID in the workspace's `circusd.pid`, but only if that process is a
      circusd. Then it removes the IPC sockets. The pidfile matters for test
      10, which deletes `dev.json` on purpose, and only a clean stop
      removes the pidfile. A failed stop therefore can't leak a circusd
      whose sockets have gone.
    - It never stops through the driver, so no test pays for a second stop
      against a dead arbiter.
  - **`unverified_stop_workspace`, for tests 8, 9, 12 and 16 and the
    stack-based harness self-tests.** Its teardown runs `_hard_cleanup`. That
    is a driver stop, skipped when neither `dev.json` nor the workspace's
    endpoint socket exists, followed by `_reap_survivors`.
- **Happy-path stops.** Every explicit happy-path `stop` uses `stop_cleanly`.
  That covers tests 4, 5, 6, 10, 11 and 17. Tests that never stop are
  checked by the fixture.
- **Out-of-band kills.**
  - Test 11 and test 15 SIGKILL fakes through `kill_out_of_band`.
  - Test 5 still kills only the arbiter, with `os.kill`. The arbiter is not
    a fake, and its children stay reachable through the recorded PIDs.
- **Test 6.**
  - Build the crafted server from `fake_process.py server --spawn-child`,
    in place of the inline `-c` source.
  - Run it with `cwd=workspace`, so its `server.log` stays inside the
    workspace, and with `workspace / EVENTS_LOG` as its events log.
  - Replace the 0.5 s sleep and the `assert child_pids` with
    `assert wait_until(lambda: {"server", "server-child"} <= {fake.role for
    fake in recorded_fakes(workspace)})`. This waits for the record, not
    just for the process to exist. It has to happen in this phase: the new
    fake starts more slowly, so a child that the stop reached before its
    `begin()` would be SIGTERMed as a non-fake.
  - Compute `child_pids` after the wait.
  - Its SIGTERM receipt is then checked like any other fake's.
- **Tests 8 and 9.**
  - Assert the message substring and the `dev.log` line from the record, in
    place of the bare `"server.log"` and `"circusd.log"` checks.
  - Assert that the sibling branch's `dev.log` line is absent.
  - Assert that `assert_no_timeout_branch_output` raises against their
    `dev.log`.
- **Test 16.**
  - Record `frontend_pid` and assert it is alive.
  - Stop with `{"grace_quit": 8.0}` through `run_driver`. Then assert:
    - the frontend is dead;
    - it logged `started` and no `sigterm`;
    - the escalation record does not name it;
    - `assert_stopped_cleanly` raises with "stopped without SIGTERM".
- **Test 17.** `test_cross_workspace_concurrency` calls `stop_cleanly(ws_a)`
  where it stops today, and `stop_cleanly(ws_b)` at the end of the body. Its
  `finally` only runs `_reap_survivors` for both, so a cleanup failure can't
  replace the test's own failure.
- **Harness self-tests** (permanent). Each one pins the assertion it expects
  with `pytest.raises(AssertionError, match=…)`.
  - **`test_the_clean_stop_check_sees_our_forced_kill`**
    (`unverified_stop_workspace`):
    1. Bring the stack `up` with `server_ignore_sigterm`.
    2. SIGKILL only the arbiter, with `os.kill` (not `kill_out_of_band`,
       which would take the server down with it), and wait until it is dead.
    3. Stop with `{"grace_kill": 0.5}`.
    4. Assert that `escalations` contains both
       `Escalation("terminate", server_pid)` and
       `Escalation("kill", server_pid)`, which proves the recorder writes
       both signals from a real stack. Then assert that
       `assert_stopped_cleanly` raises with "our code SIGKILLed".
  - **`test_the_clean_stop_check_sees_a_sigterm_of_a_non_fake`** (plain
    `tmp_path`, no stack):
    1. Write an events log holding one fake that is dead and logged
       `started` and `sigterm`.
    2. Write an escalation log holding `terminate 4242`, a PID that is not
       recorded.
    3. Assert that `assert_stopped_cleanly` raises with "our code SIGTERMed
       a non-fake".
  - **`test_the_clean_stop_check_sees_a_survivor`** (plain `tmp_path`, no
    stack):
    1. Create an empty escalation log.
    2. Start a `fake_process.py child` that logs to the workspace's events
       log, and `wait_until` its `started` line appears.
    3. Assert that `assert_stopped_cleanly` raises with "survived the stop".
    4. Kill the child in `finally`.
  - **`test_every_timeout_branch_evidence_line_is_still_logged`:**
    1. Parse `tasks/shared/dev/lifecycle.py` with `ast`.
    2. Collect every string constant, including the literal parts of
       f-strings, and leave out docstrings.
    3. Assert that each `TIMEOUT_BRANCH_EVIDENCE` string occurs in one of
       them.

    If a production message is reworded, the absence check then fails
    loudly instead of going vacuous. The docstring at `lifecycle.py:532`
    can't keep it green.

**TDD order**
1. Write the helpers, the tightened assertions and the harness self-tests.
2. Run the suite. It fails for these reasons:
   - The escalation log is missing, so every `stop_cleanly` fails.
   - The events log is never written, so test 16's `started` check fails.
   - `fake_process.py` doesn't exist, so the self-tests fail.

   The evidence-guard test and the self-test for a SIGTERM of a non-fake
   pass from the start, because neither needs the recorders. Record that in
   the PR.
3. Add `fake_process.py`, the launchers, the readiness wait and the
   `EscalationRecorder` until the suite is green.

### Success Criteria:

#### Automated Verification:

- [x] The dev suite passes serially: `mise run test:integration:dev`
- [x] Build-system checks pass, including ruff on `fake_process.py`, which
      pyrefly doesn't cover because it only checks `tasks/**`:
      `mise run build-system:check`
- [x] `mise run check` passes
- [x] Red step observed and noted in the PR: the new assertions and
      self-tests fail before the recorders exist.
- [x] The suite writes nothing outside its `tmp_path`: the repo's
      `.accelerator/tmp/visualiser/server.log` is unchanged by a run.

#### Manual Verification:

- [x] Each test's evidence holds:
  - A run of `test_sigterm_ignoring_frontend_is_sigkilled` leaves the events
    log with `started frontend <pid> …` and no `sigterm` for that PID.
  - A happy-path test leaves an escalation log with no `kill` line.
- [x] The negative-path record is in the PR description and each string
      matches `lifecycle.py`. It is in PR #220's description, along with
      the red-step note.
      `test_every_timeout_branch_evidence_line_is_still_logged` pins the
      strings.
- [x] The follow-up work item for orphaned children after an out-of-band
      kill exists and is linked from the PR. It is 0371 (PP-942), linked
      from PR #220, and also covers the lost-`dev.json` case.
- [x] Suite wall time is unchanged or lower. No timing change is intended,
      and the teardown no longer runs a second driver stop.

---

## Phase 2: Shorten the dev suite's pure-delay waits

### Overview

Lower the driver's `probe_timeout` and `grace_kill` defaults. Replace the
fixed settle sleeps with condition waits. Phase 1's assertions catch any
escalation that the shorter grace would cause.

### Changes Required:

#### 1. Driver defaults

**File**: `tests/integration/dev/dev_integration_driver.py`

**Changes**:
- **`probe_timeout` 5.0 → 2.0.** This is the production value, and it
  already bounds live calls in real use. It bounds more than the
  dead-arbiter probe. Three other calls fail silently, with no `dev.log`
  line and no escalation, when a busy arbiter misses the deadline:
  - **The reuse gate (`reuse_or_teardown`).** A slow status reply leads to
    a teardown and relaunch instead of `reused`.
  - **Watcher-PID recording (`_record_watcher_pid`).** An unreachable reply
    leaves `server_pid` or `frontend_pid` null, which narrows the reach of
    `reap_descendants` without any signal.
  - **The frontend start and status polls.**

  So a Phase 3 failure in a test that expects `reused` (test 2), or that
  reads a recorded PID and finds it null (tests 1, 13, 15 and 16), counts
  as a `probe_timeout` failure. The fix is to raise `probe_timeout`, not to
  weaken the assertion. The same rule applies to `grace_kill` below.
- **`grace_kill` 3.0 → 0.5.** It bounds two waits, and both are covered:
  - **`reap_descendants`.** After circus `quit` with `stop_children`, the
    fakes are already dead before the reap runs. A fake that does survive
    is flagged by `stop_cleanly`, not SIGKILLed unnoticed.
  - **`kill_arbiter`.** When the status probe times out on a live but busy
    arbiter, no `quit` is sent. `grace_quit` then expires, our code SIGTERMs
    circusd, and it waits only `grace_kill`. A circusd that has been
    SIGTERMed stops its watchers gracefully for up to 2 s, so 0.5 s turns
    this path into a SIGKILL. Both the SIGTERM and the SIGKILL are in the
    escalation record, so the failure is visible, never masked. The Phase 3
    validation records how often it happens. If it happens at all, the fix
    is to raise `grace_kill` to 2.5, not to weaken the assertion.
- **Unchanged.** `pidfile_timeout`, `readiness_timeout`,
  `frontend_active_timeout` and `grace_quit` keep their values.
  Negative-path tests keep their explicit values.
- **The surviving comment.** Rewrite it to say only what stays true: the
  readiness, pidfile and frontend-active budgets are generous because real
  processes start on loaded runners, while the probe and grace budgets are
  tight because `stop_cleanly` makes escalation visible. Keep the
  frontend-watcher note.

#### 2. Settle sleeps become condition waits

**File**: `tests/integration/dev/test_dev_integration.py`

**Changes**: rebuild `wait_for_text` and the status poll in
`test_status_exit_codes` on Phase 1's `wait_until`, and replace the two
remaining settle sleeps. Phase 1 has already replaced test 6's.

| Test | Replacement |
|---|---|
| `test_orphan_reach_when_arbiter_already_dead` | `assert wait_until(lambda: not alive(state["arbiter_pid"]))` |
| `test_stale_cleanup_after_out_of_band_kill` | `assert wait_until(lambda: not any(fake.is_running() for fake in recorded_fakes(workspace)))` |

### Success Criteria:

#### Automated Verification:

- [x] The dev suite passes serially: `mise run test:integration:dev`
- [x] No fixed settle sleep of 0.5 s or more remains:
      `grep -En '^[[:space:]]*time\.sleep\((0\.[5-9]|[1-9])' tests/integration/dev/test_dev_integration.py`
      returns nothing
- [x] Production defaults are untouched: `jj diff --from 'fork_point(trunk() | @)' --to @ -- tasks/shared/dev` is empty
- [x] `mise run build-system:check` passes

#### Manual Verification:

- [x] Five serial runs under `mise run` load are green, with no
      `stop_cleanly` failure from the 0.5 s `grace_kill`. All 21 tests
      passed in each run, on commit `b9866538`.
- [x] The serial wall time falls well below 163 s. Record the figure in the
      PR. It is 61 s isolated, down from 135–139 s after Phase 1, and
      92–120 s (median 95 s) under `mise run` load. Both figures and the
      five load runs are in PR #221's description. PR #221 was stacked on
      PR #220. Both merged on 2026-11-16, and `main`'s merge commit is
      `e1f83907`.

---

## Phase 3: Run the dev suite under pytest-xdist (dev suite lands)

### Overview

Add `pytest-xdist`, run the suite with `-n auto`, validate it, and capture
the CI before window.

### Changes Required:

#### 1. Task argv test (red first)

**File**: `tests/unit/tasks/test_dev_integration_task.py` (new)

```python
from invoke import Context

from tasks.test.integration import dev


def test_the_dev_suite_spreads_its_tests_across_workers(mocker):
    context = mocker.MagicMock(spec=Context)

    dev(context)

    assert "-n auto" in context.run.call_args.args[0]
```

#### 2. Task command

**File**: `tasks/test/integration.py:57`
**Changes**: `context.run("uv run pytest tests/integration/dev -n auto -v")`.

#### 3. Dependency

**File**: `pyproject.toml` (`dev` group), `uv.lock`
**Changes**: add `"pytest-xdist>=3"` next to `pytest-mock`. Like its
sibling pytest plugins, it gets no comment. Run `uv lock` and commit the
regenerated `uv.lock`, since `deps:install:python` uses `--frozen`.

### Success Criteria:

#### Automated Verification:

- [x] The new unit test fails before step 2 and passes after:
      `mise run test:unit:tasks`
- [x] The lock is consistent: `uv sync --all-groups --frozen`
- [x] The suite passes in parallel: `mise run test:integration:dev`, and the
      xdist banner shows the worker count. It shows 16 workers on
      engywook, and all 21 tests pass in 7.9 s.
- [x] `mise run build-system:check` passes
- [x] Timing target met: in
      `mise run measure:build --isolate test:integration:dev --cache-state warm-no-op --runs 5 --host-settings devtools --keep`,
      the record's per-task outcome is success and the median is under
      60 s. It is 7.9 s (min 7.8, max 8.8), recorded in
      `meta/measurements/build_2026-11-16T004012Z_engywook_warm-no-op_devtools_isolated.json`.

#### Manual Verification:

- [x] Worker counts recorded in the PR: what `-n auto` resolves to on
      engywook and on both CI runners, from the xdist banner in the job log,
      next to psutil's physical and logical counts. #222's banners were
      recorded in #222. The CI logs didn't print psutil's counts, so #230
      added `mise run host:cpu-counts` before the integration tests. Its
      run 43965178907 measured:

      | Host | Image | xdist banner | Physical | Logical | `os.process_cpu_count` | Node `availableParallelism` |
      |---|---|---|---|---|---|---|
      | engywook | — | 16 | 16 | 16 | 16 | 16 |
      | `ubuntu-26.04` | 20261111.196.1 | 2 | 2 | 4 | 4 | 4 |
      | `macos-26-arm64` | 20261112.0504.1 | 3 | 3 | 3 | 3 | 3 |

      On ubuntu, `-n auto` takes half the runner's vCPUs, while Playwright
      will see all four. If Phase 7's comparison fails on ubuntu, `-n
      logical` is the first lever. On macOS the counts agree.
- [x] Load comparison recorded in the PR (not a gate). On engywook (16
      cores, no SMT), `-n auto` and `-n logical` both resolve to 16, so the
      comparison is between `-n auto` and `-n 8`:
  - Run one non-isolated
    `mise run measure:build --cache-state warm-no-op --runs 5 --host-settings devtools --keep`
    for each, setting `PYTEST_XDIST_AUTO_NUM_WORKERS=8` for the second.
    Run the two back to back on one recorded commit. Record in the PR
    whether the Playwright track's worker phase is in that commit.
  - Record, for each, the median and IQR of the `default` wall time and of
    the three slowest sibling tasks.
  - The `default` wall-time median decides. The sibling-task figures are
    context only. `-n 8` wins only if its `default` median beats
    `-n auto`'s by more than the larger IQR. A win makes switching a tuning change, which restarts the
    10-run count.
  - If the median or IQR is borderline, try `--dist worksteal` before
    retuning `-n`. `PYTEST_XDIST_AUTO_NUM_WORKERS` is the documented
    per-host override.
  - These runs are separate from the 10 counted runs. The comparison varies
    the config, and the counted runs must use the final config on one
    commit.
  - The question of physical versus logical cores is answered by the CI
    banners, where the two counts can differ.
  - **Result, recorded in PR #222.** Both runs used commit
    `8d7966fb`, which does not contain the Playwright worker phase. Every
    run succeeded, and the logs show 16/16 and 8/8 workers.

    | Figure | `-n auto` | `-n 8` |
    |---|---|---|
    | `default` wall time | 225.3 s, IQR 2.7 | 228.1 s, IQR 4.9 |
    | `test:integration:dev` | 10.6 s, IQR 0.9 | 11.3 s, IQR 0.0 |
    | `test:unit:cli` | 194.4 s, IQR 8.4 | 200.3 s, IQR 11.0 |
    | `test:e2e:visualiser` | 147.5 s, IQR 0.7 | 158.7 s, IQR 12.3 |
    | `test:unit:tasks` | 102.0 s, IQR 3.2 | 111.6 s, IQR 39.7 |

    `-n 8` loses on the `default` median, so `-n auto` stays and the count
    is not restarted. The idle load was 20.7% before the `-n auto` run and
    60.0% before the `-n 8` run. The records are
    `meta/measurements/build_2026-11-16T081243Z_engywook_warm-no-op_devtools.json`
    and
    `meta/measurements/build_2026-11-16T083530Z_engywook_warm-no-op_devtools.json`.
- [x] Any `stop_cleanly` failure across the 10 runs that names an
      arbiter's `terminate` or `kill`, or any null recorded PID or missed
      `reused`, is handled under Phase 2's `probe_timeout` and `grace_kill`
      rule, and is listed in the PR. None occurred: the dev suite passed
      21/21 in every counted run.
- [x] Ten counted `mise run` executions on one recorded commit, per
      Requirement 7. All ten exited 0 on `d29d7621`, whose parent is the
      #221 merge commit `e1f83907` on `main`. The working copy stayed clean.
      Each run took 270–361 s, and the dev suite took 10.3–16.7 s on 16
      workers. The outcomes are in PR #222:
  - The PR records each outcome. A failure in an unrelated task is named.
  - Any change to the suite restarts the count.
  - The commit is the merge base, or the validation is repeated after a
    rebase.
- [x] Anti-masking: the diff adds no `skip`, rerun or flaky marker. Every
      flake found is listed with its fix or its split-out work item, and a
      split-out item is added to 0364's Dependencies as a blocker. No flake
      was found.
- [ ] The CI before window for `test-integration` is recorded in the PR
      before merge. For each OS:
  - the last 10 main-branch run IDs, all after Phase 2's merge;
  - the median and IQR% of the `test:integration:dev` "Finished in" line
    (parsed with the regex in `tasks/shared/build_measurement/timeline.py:20`);
  - the runner image version from the job's "Set up job" step.

  Also check whether any sibling (0362 especially) merged inside the window.

  This was not met. #222 merged at 2026-11-16 20:46 UTC, through #225's
  merge commit `929fae79`, because #225 was stacked on it. Only three
  `main` runs fall between #221's merge (10:24 UTC) and that commit. In
  each, both `test-integration` jobs succeeded:

  | Run | Commit | ubuntu dev suite | ubuntu image | macOS dev suite | macOS image |
  |---|---|---|---|---|---|
  | 43848526711 | `e1f83907` (#221) | 163.39 s | 20261103.629 | 198.84 s | 20261028.624 |
  | 43913843321 | `242096a9` (#188) | 171.11 s | 20261103.629 | 166.83 s | 20261103.629 |
  | 43924565386 | `36e3d747` (#227) | 163.22 s | 20261103.629 | 260.10 s | 20261028.624 |

  - The medians are 163.39 s on ubuntu (range 163.2–171.1) and 198.84 s on
    macOS (range 166.8–260.1). Three samples give no meaningful IQR.
  - The whole job took 666–756 s on ubuntu and 1247–1471 s on macOS.
  - No sibling in the window touches the job. #188 changes no build or
    test wiring, and #227 only orders `test:unit:frontend`.
  - The macOS runner image changes inside the window.

  Phase 7 therefore treats the dev suite's before window as contaminated
  and runs the paired comparison for it. Later `main` runs can't extend
  the window, because every one of them contains #222.

  The after window starts with run 43953328694, on `929fae79`. Its
  integration and E2E jobs passed on both OSes. Only `Create prerelease`
  failed, on a DNS error during provenance attestation
  (`getaddrinfo ENOTFOUND run-actions-3-azure-eastus.actions.githubusercontent.com`),
  which is unrelated to 0364.
- [x] Validation and measurement don't overlap 0370's host-settings
      recording.

---

## Phase 4: Isolate `resolved-styles` from the drag tests' fixture writes

### Overview

Carry out the Requirement 2 audit under its per-probe definition of a
conflict. A probe conflicts when the element it measures, or that element's
computed style, comes from a fixture another concurrently running spec
writes. Resolve each conflict on the `resolved-styles` side, and show the
result with a run against mutated fixtures. The suite still runs on one
worker.

Pages that aggregate every work item (`/kanban`, `/library/work-items`,
`/lifecycle*`) still read the written fixtures. That is accepted. These specs
measure computed styles, so a probe pinned to an unwritten fixture measures
the same thing whatever the written fixtures say. Playwright locators
re-resolve after a re-render driven by SSE, and a re-rendered pinned element
has the same computed style.

### Changes Required:

#### 1. Name the unwritten work item once

**File**: `cli/visualiser/frontend/tests/lib/stable-fixtures.ts` (new)

**Changes**:
- Export `UNWRITTEN_WORK_ITEM = "0002-second-work-item"`, which is a `done`
  item that no spec writes.
- Export `UNWRITTEN_WORK_ITEM_CARD`, the
  `li[data-relpath="meta/work/${UNWRITTEN_WORK_ITEM}.md"]` selector derived
  from it.

#### 2. Repoint the work-item detail slug

**File**: `cli/visualiser/frontend/tests/lib/detail-route-slugs.ts:22`

**Changes**:
- Set `"work-items": UNWRITTEN_WORK_ITEM`.
- Confirm that `fixture-coverage` and `detail-eyebrow-resolved-colours`
  still assert the same things against a `done` item.

#### 3. Pin the `/kanban` card probes to the unwritten item

**Files**:
- `tests/resolved-styles/typography-resolved-sizes.spec.ts:136-151`
- `tests/resolved-styles/radius-resolved-radii.spec.ts:134-140`

**Changes**: scope the card selectors to `UNWRITTEN_WORK_ITEM_CARD` (for
example `… .ac-kcard__title`, `… .ac-kcard__id`, `… .ac-stagedots__dot`).
`.first()` then resolves inside a card no spec moves.

#### 4. Audit and resolve the remaining aggregate probes

Go through every `resolved-styles` probe on `/library`,
`/library/work-items`, `/lifecycle`, `/lifecycle/first-plan` and
`/library/work-items/0099-ac2-coverage`. List the probes whose measured
element comes from a work item that `chromium` writes (0001, 0003, 0005,
0006). Examples are a row, a count, a cluster member, or a related row that
is recomputed after a PATCH (`server/src/api/docs.rs:276-309`). Pin each one
to the unwritten item in the same way.

Probes on page chrome that doesn't depend on work items (headings, sidebar,
tokens) are recorded as not conflicting. So are the page-level aggregate
reads, with the reasoning above.

#### 5. Audit record (in the PR description)

For each project, list the fixture files it writes and reads, and how each
list was traced:
- **The specs' own file access.** Grep for `writeFileSync`, `readFileSync`,
  `request.patch` and `request.post`.
- **The server's file opens.** Trace the whole run of
  `npx playwright test --project=<p>` on its own, from before the server
  starts, so that start-up indexing is captured too. Then filter to
  `tests/fixtures/meta`.
  - On macOS, start `sudo fs_usage -w -f filesys accelerator-visualiser`
    before the run. This filters by process name.
  - On Linux, run `strace -f -e trace=open,openat -o trace.log npx
    playwright test --project=<p>`. The server is then a traced descendant,
    so Ubuntu's default `ptrace_scope` needs no root.
- **The server's index-served aggregate routes.** Read their code. This is
  needed because the trace shows only startup indexing for them.

The record then lists:
- the conflicting probes found, and the pin that resolved each one;
- the accepted page-level aggregate reads;
- the two conflict classes under the per-probe definition, both empty.

#### 6. Mutated-fixture run (in the PR description)

Rewrite 0001, 0003, 0005 and 0006 into states the drag and conflict specs
can leave behind:
- move 0001 and 0005 to another stage;
- change 0006's body and version;
- rewrite 0003.

Then run `npx playwright test --project=resolved-styles` with the task's env
(see "The task's env" in Phase 6). Restore the fixtures with `jj restore
cli/visualiser/server/tests/fixtures/meta/work`. The run must pass
unchanged. Record the mutations and the result in the PR.

### Success Criteria:

#### Automated Verification:

- [x] The suite passes: `mise run test:e2e:visualiser`. 355 passed and 1
      was skipped (`legacy-schema`'s existing lookup), on one worker.
- [x] `mise run frontend:check` passes
- [x] No `resolved-styles` spec names a written fixture:
      `grep -rEn "0001-first|0003-third|0005-sse|0006-conflict" cli/visualiser/frontend/tests`
      returns nothing

#### Manual Verification:

- [x] The audit record is in the PR, with the trace method for each list and
      no conflicts under the per-probe definition. The audit found one
      conflict beyond those listed above. `/lifecycle` sorts its cluster
      cards by most recent change, so a write moves the written item's
      cluster to the top, where radius's `.first()` card probe measured it.
      That probe is pinned to 0002's cluster card
      (`UNWRITTEN_LIFECYCLE_CARD`). The `fs_usage` trace shows that
      `resolved-styles` writes nothing, and opens 0001, 0003, 0005 and
      0006 only in the server's start-up index.
- [x] The mutated-fixture run passed, and the fixtures are restored.
      `resolved-styles` passed 313/313 against the mutations.
- [x] A spot-check shows that each pinned probe still selects an element. A
      changed slug should not have turned a probe into a no-op that passes
      vacuously. Every pinned probe either `waitFor`s its element or
      evaluates it through `.first()`, so a missing element times out.

---

## Phase 5: Turn Playwright retries off and keep traces on failure

### Overview

Set `retries: 0` and `trace: "retain-on-failure"` in every environment, and
fix whatever flakes appear while the suite still runs serially.

### Changes Required:

#### 1. Config test (red first)

**File**: `cli/visualiser/frontend/scripts/playwright-config.test.ts` (new)

**Changes**:
- Run the file under the default `jsdom` environment, as
  `visual-diff-ciede2000.test.ts` does. The shared `src/test/setup.ts` writes
  to `window`, so the `node` environment would crash it. Node APIs such as
  `process.env` and `node:os` work under jsdom.
- Mock `@playwright/test`'s `defineConfig` as identity, so that importing
  the config needs no Playwright runner.
- For both `CI` unset and `CI=true`: run `vi.stubEnv`, then `vi.resetModules()`,
  then import `../playwright.config` dynamically. Assert `retries === 0` and
  `use.trace === "retain-on-failure"`.

```ts
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("@playwright/test", () => ({ defineConfig: <T>(c: T) => c }));

async function loadConfig(ci: string | undefined) {
  vi.resetModules();
  vi.stubEnv("CI", ci);
  return (await import("../playwright.config")).default;
}

afterEach(() => vi.unstubAllEnvs());

describe.each([undefined, "true"])("with CI=%s", (ci) => {
  it("never retries a failing test", async () => {
    expect((await loadConfig(ci)).retries).toBe(0);
  });

  it("keeps the trace of a failing test", async () => {
    expect((await loadConfig(ci)).use?.trace).toBe("retain-on-failure");
  });
});
```

#### 2. Config

**File**: `cli/visualiser/frontend/playwright.config.ts`
**Changes**: `retries: 0`; `use.trace: "retain-on-failure"`.

#### 3. Flake fixes

Run the suite repeatedly at one worker and fix each failure that appears.
The known candidate is `resolved-styles`' first-navigation
`net::ERR_ABORTED` under full load. Per Requirement 4, a fix that needs
changes outside the suite's specs, fixtures and config is split into its own
work item, which blocks only Playwright's landing.

#### 4. Per-file durations (in the PR description)

Playwright hands whole files to workers, so the longest `resolved-styles`
file sets a floor on the parallel wall time. From one serial run's JSON
reporter output, record each file's duration. Compare the longest with the
75 s budget, minus server start-up. If the longest file can't fit, turn on
per-project `fullyParallel: true` for `resolved-styles` in Phase 6 from the
start, so that the 10-run count isn't spent and then restarted.

### Success Criteria:

#### Automated Verification:

- [x] The config test fails before step 2 and passes after:
      `npm --prefix cli/visualiser/frontend test -- scripts/playwright-config.test.ts`.
      Before, all four cases failed on the values (`1` and
      `"on-first-retry"`); after, all four pass.
- [x] Frontend unit tests pass: `mise run test:unit:frontend`
- [x] `mise run frontend:check` passes
- [x] The suite passes: `mise run test:e2e:visualiser`. 355 passed and 1
      was skipped, in 170.5 s on one worker.

#### Manual Verification:

- [x] A test made to fail deliberately leaves `trace.zip` under
      `test-results/<test>/`. This is done once, recorded in the PR and then
      reverted. `smoke.spec.ts`'s URL assertion was pointed at a wrong
      route. The run reported `1 failed` with no retry, and left
      `test-results/smoke-app-loads-and-redirects-to-library-chromium/trace.zip`.
      The spec was restored with `jj restore`.
- [x] Five serial runs under `mise run` load are green. Each flake found is
      listed in the PR with its fix. All five exited 0 on `2d770eb2`, each
      with 355 passed and 1 skipped. The suite took 194–291 s (median
      250 s), and each `mise run` took 305–502 s. No flake appeared, so the
      `net::ERR_ABORTED` candidate needed no fix.
- [x] The per-file durations and the `fullyParallel` decision are in the PR.
      From one serial JSON-reporter run (151.5 s wall, 147.6 s summed
      tests), the longest `resolved-styles` files are
      `dev-design-system-code-block-resolved-colours` at 25.5 s,
      `aside-row-resolved-colours` at 20.8 s and
      `non-regression-glyph-consumers` at 17.5 s. `resolved-styles` sums
      to 123.5 s and `chromium` to 24.1 s. The longest file fits well
      inside 75 s minus about 4 s of start-up, so Phase 6 starts without
      `fullyParallel`. The global setup prints its `BASE_URL` line to
      stdout, so a JSON report piped from stdout needs that line dropped,
      or `PLAYWRIGHT_JSON_OUTPUT_NAME` set instead.

---

## Phase 6: Host-relative Playwright workers (Playwright lands)

### Overview

Derive the per-project and global worker counts from the host's logical CPU
count, verify that the projects overlap, then validate and capture the CI
before window.

### Changes Required:

#### 1. Worker allocation (red first)

**Files**:
- `cli/visualiser/frontend/scripts/worker-allocation.ts`
- `cli/visualiser/frontend/scripts/worker-allocation.test.ts`

**Why `scripts/`.** It is the one directory that Vitest collects and that
`tsconfig.node.json`, where `playwright.config.ts` and the Node types live,
also covers. Putting the module there is a deliberate choice of location.

**The margin.** The work item's *margin* is the `chromium` project's single
worker. The allocation names it as `chromium`, so the config takes all
three counts from one place. That leaves the share as the only tuning
lever.

```ts
export interface WorkerAllocation {
  resolvedStyles: number;
  chromium: number;
  total: number;
}

export const RESOLVED_STYLES_SHARE = 0.5;
export const CHROMIUM_WORKERS = 1;

export function allocateWorkers(logicalCpus: number): WorkerAllocation {
  const resolvedStyles = Math.max(
    1,
    Math.floor(logicalCpus * RESOLVED_STYLES_SHARE),
  );
  return {
    resolvedStyles,
    chromium: CHROMIUM_WORKERS,
    total: resolvedStyles + CHROMIUM_WORKERS,
  };
}
```

```ts
it.each([
  [1, 1, 2],
  [3, 1, 2],
  [4, 2, 3],
  [16, 8, 9],
])("gives %i CPUs %i resolved-styles workers under a cap of %i",
  (cpus, resolvedStyles, total) => {
    expect(allocateWorkers(cpus)).toEqual({
      resolvedStyles,
      chromium: 1,
      total,
    });
  });
```

#### 2. Config wiring (extend the Phase 5 config test first)

**File**: `cli/visualiser/frontend/playwright.config.ts`

**Changes**:
- Compute `allocateWorkers(availableParallelism())` once.
- Set the top-level `workers: allocation.total`.
- In `resolved-styles`, set `workers: allocation.resolvedStyles`.
- In `chromium`, set `workers: allocation.chromium`.
- Delete the stale comment at lines 10–13.
- Replace `resolved-styles`' comment (lines 24–29) with the invariant that
  now matters, in one or two lines: no probe may measure an element that
  comes from a fixture a `chromium` spec writes, because the two projects
  run concurrently. This belongs here, not in Phase 4, because only this
  phase makes it true.
- Import `availableParallelism` by name from `node:os`, so the test's mock
  of that named export is the one the config reads.
- Replace `chromium`'s existing "snapshot project" comment block with one
  statement of its invariant: it runs on one worker because its drag specs
  mutate shared fixtures in place, and `globalSetup` and `globalTeardown`
  restore them.

**Per-host tuning is deliberately a code change.** The Playwright share has
no environment override, unlike the dev suite's
`PYTEST_XDIST_AUTO_NUM_WORKERS`. A different share changes what the 10-run
validation and the CI windows measured, so it belongs in a reviewed commit
that restarts the count.

**Test additions**:
- Make `availableParallelism` return 4 by spying on the `os` module object
  (`vi.spyOn(os, "availableParallelism")`) in a `beforeEach`. A
  `vi.mock("node:os")` does not reach the config: under `jsdom` Vite's web
  transform rewrites the config's import to
  `__vite-browser-external:node:os` and reads the export off that module's
  `default`, which is the real `os` object the spy patches.
- Under both `CI` values, assert the literal counts:
  - top-level `workers` is 3;
  - `resolved-styles` is 2;
  - `chromium` is 1.

### Success Criteria:

#### Automated Verification:

- [x] The worker-allocation and config tests fail before the change and pass
      after: `npm --prefix cli/visualiser/frontend test -- scripts/`. Before,
      the allocation test failed on the missing module, and the config test's
      six worker cases failed on the values (`workers` 1, project workers
      `undefined`); after, all 26 script tests pass.
- [x] `mise run frontend:check` passes
- [x] The suite passes: `mise run test:e2e:visualiser`. 355 passed and 1
      was skipped, in 45.3 s on 9 workers.
- [x] Projects overlap. Before the config change the check printed `true`
      then `false` (188 s wall); after, `true` then `true` (46 s wall). Both
      runs had 355 expected, 0 unexpected, 1 skipped and 0 flaky. Run `npx playwright test --reporter=json >
      results.json` from the frontend directory, with the task's env, and
      then this `jq` check.

      **The task's env** is what `playwright_environment` in
      `tasks/test/e2e.py` sets. Run these from the repo root:
      ```
      export ACCELERATOR_VISUALISER_BIN="$PWD/cli/target/debug/accelerator-visualiser"
      export E2E_HEALTH_PORT="$(uv run python -c 'from tasks.shared.ports import free_port; print(free_port())')"
      ```
      Without `E2E_HEALTH_PORT`, the run can adopt a stray server on 19087.

      The check keeps millisecond precision, and prints two lines:
      - `true` when no two `chromium` results overlap in time;
      - `true` when some `chromium` result overlaps a `resolved-styles`
        result.
      ```
      jq '
        def epoch: capture("^(?<base>[^.]+)(\\.(?<frac>[0-9]+))?Z$")
          | (.base + "Z" | fromdate) + ("0." + (.frac // "0") | tonumber);
        [.. | objects | select(has("projectName")) | .projectName as $p | .results[]
          | (.startTime | epoch) as $s | {p: $p, s: $s, e: ($s + .duration / 1000)}] as $rs
        | [$rs[] | select(.p == "chromium")] as $c
        | [$rs[] | select(.p == "resolved-styles")] as $r
        | ($c | sort_by(.s) | [range(1; length) as $k | .[$k - 1].e <= .[$k].s] | all),
          any($c[]; . as $x | any($r[]; .s < $x.e and $x.s < .e))
      ' results.json
      ```
      Before the config change, the same check prints `true` then `false`.
      Record both outputs in the PR.
- [x] Timing target met: in
      `mise run measure:build --isolate test:e2e:visualiser --cache-state warm-no-op --runs 5 --host-settings devtools --keep`,
      the per-task outcome is success and the median is under 75 s. It is
      47.0 s (min 46.4, max 54.3) on `83572065`, with every run successful
      and an idle load of 67.0%, recorded in
      `meta/measurements/build_2026-11-16T230609Z_engywook_warm-no-op_devtools_isolated.json`.

#### Manual Verification:

- [x] Ten counted `mise run` executions on one recorded commit, per
      Requirement 7. The PR records each outcome and the commit. All ten
      count, on `c8cf7e5e`, whose code is `399edd36` on top of `main` at
      `902556ac` (#228); the commits above `399edd36` touch only `meta/`.
      The working copy stayed clean.
  - Nine exited 0, in 218–310 s. Run 8 exited 1 after 124 s, failing only
    `test:unit:tasks`: `test_the_build_stamps_lines_in_arrival_order`
    expects two lines printed 0.5 s apart to be stamped more than 0.3 s
    apart, and under load the first was stamped late, leaving 0.25 s. It is
    a timing test of the measurement harness, unrelated to the suite.
  - `test:e2e:visualiser` passed 355 with 1 skipped in every run, in
    58–78 s on 9 workers. The dev suite passed 21/21 in every run.
- [x] Anti-masking: the diff adds no `skip`, `fixme` or per-test/describe
      `retries`. Every flake is listed with its fix or its split-out item.
      The diff under `cli/` adds no `.skip`, `.fixme`, `.only`, `test.fail`
      or `retries`. The suite showed no flake. The one e2e failure seen
      during this phase was the outside `pkill` recorded with the load
      measurement below.
- [x] The CI before window for `test-e2e` is recorded in the PR before merge,
      as in Phase 3, with every run after Phase 5's merge. Only four `main`
      runs follow #229's merge (`2b3aa94f`, through #230). All their E2E
      jobs passed, each on one worker:

      | Run | Commit | ubuntu suite | ubuntu job | ubuntu image | macOS suite | macOS job | macOS image |
      |---|---|---|---|---|---|---|---|
      | 43988761983 | `2b3aa94f` (#229, #230) | 301.79 s | 384 s | 20261103.629 | 322.39 s | 412 s | 20261028.624 |
      | 43989711594 | `f30a4f7c` (#224) | 222.33 s | 294 s | 20261103.629 | 313.67 s | 415 s | 20261103.629 |
      | 43989762511 | `ba876153` (#226) | 336.53 s | 429 s | 20261103.629 | 320.72 s | 439 s | 20261028.624 |
      | 44031742886 | `902556ac` (#228) | 224.70 s | 328 s | 20261103.629 | 341.73 s | 474 s | 20261103.629 |

      - The suite's medians are 263.25 s on ubuntu (IQR 86.37 s, 32.8%) and
        321.56 s on macOS (IQR 8.27 s, 2.6%).
      - No sibling in the window touches the job: #224, #226 and #228 change
        nothing under the visualiser, Playwright, the workflows, `mise.toml`
        or `tasks/test`.
      - The macOS runner image changes inside the window, and ubuntu's IQR
        is far over 5%.

      Phase 7 therefore treats the Playwright before window as contaminated
      and runs the paired comparison for it, as it does for the dev suite.
      Node reports 4 CPUs on ubuntu and 3 on macOS, so on CI the change
      gives `resolved-styles` 2 workers on ubuntu (cap 3) and 1 on macOS
      (cap 2).
- [x] If 75 s is missed, the first lever is per-project
      `fullyParallel: true` on `resolved-styles`, unless Phase 5 already
      turned it on. It is a config change, so it restarts the 10-run count.
      Not needed: the isolated median is 47.0 s.
- [x] Load measurement recorded in the PR (not a gate): one non-isolated
      `mise run measure:build --cache-state warm-no-op --runs 5 --host-settings devtools --keep`
      without the worker change and one with it. Run them back to back,
      from one recorded base commit. Record whether the dev track's worker
      phase is in that base. Record the median and IQR of the `default` wall time and of the
      three slowest sibling tasks. These runs are separate from the 10
      counted runs.
  - **Result.** The base is `main` at `902556ac` (#228), which contains the
    dev track's worker phase (#222). "Without" ran on `902556ac` and "with"
    on the Phase 6 config commit `399edd36` rebased onto it, back to back.
    Both working copies matched at start and end. The idle load was 36.9%
    before "without" and 60.0% before "with".

    | Figure | Without | With |
    |---|---|---|
    | `default` wall time | 274.3 s, IQR 12.6 | 276.7 s, IQR 2.3 |
    | `test:unit:cli` | 202.0 s, IQR 6.2 | 255.0 s, IQR 5.1 |
    | `test:e2e:visualiser` | 199.4 s, IQR 7.8 | 87.5 s, IQR 6.6 |
    | `test:unit:tasks` | 124.3 s, IQR 26.1 | 189.3 s, IQR 9.1 |
    | `test:integration:research` | 79.2 s, IQR 15.0 | 84.4 s, IQR 10.9 |

    The `default` median rises by 2.4 s, inside the larger IQR, so it is
    unchanged. The e2e suite's saving moves onto `test:unit:cli` and
    `test:unit:tasks`, which now set the critical path. That is the
    cross-lane contention 0369 owns.
  - **Unrelated-task failures.** "Without" run 4 failed `test:unit:cli`
    (`research-cli`'s `arxiv_pacing`
    `a_killed_calls_request_still_spaces_the_next`, whose 3 s spacing
    assertion measured 2.983 s) and `lint:server:check` (the server's
    build script found no `frontend/dist/index.html` while `build:frontend`
    was rebuilding it). The records also flag `deps:install:public-api`
    and `notices:check` as failed with no exit status. Both are parsing
    artefacts: their last log line was interleaved with another task's
    output, and their runs exited 0.
  - **Two earlier attempts were discarded.** In the first, a fetch and
    `jj rebase -b @ -d 'trunk()'` from outside this session moved the
    "without" base mid-run. In the second, another Claude Code session ran
    `pkill -f chrome-headless-shell` at 06:27:04 UTC, which SIGTERMed all
    nine of the e2e suite's browsers at once (exit 143) and failed 9 tests.
    Neither is a flake in the suite.
  - The records are
    `meta/measurements/build_2026-11-17T063700Z_engywook_warm-no-op_devtools.json`
    and
    `meta/measurements/build_2026-11-17T070532Z_engywook_warm-no-op_devtools.json`.

---

## Phase 7: CI closing comparison (per suite, after it lands)

### Overview

Judge each suite's CI effect under Requirement 8, and close the item when
both suites have passed.

### Steps

1. **Collect the after window.** Take the first 10 main-branch runs after the
   suite's worker phase merged, with the same figures as the before window.
   Also record the wall-clock of the whole `test-integration` or `test-e2e`
   job in both windows, so that cost moved onto sibling lanes shows up. Add
   them to the PR. These figures, like the `default` measurements from
   Phases 3 and 6, are informational. They don't enter the pass rule.
   Contention between lanes belongs to 0369. If the job wall-clock or the
   `default` median gets worse by more than the larger IQR, record it on
   0369 so that its lane scheduling can weigh it.
2. **Check for contamination.** Look for three things: a sibling that changes
   the same job merged inside either window; the runner image version changed
   between the windows; or the before window's IQR is over 5%.
3. **If a window is contaminated, run the paired comparison.**
   - Open two draft PRs from one main commit: one with the worker change, one
     with it reverted.
   - Run `gh run rerun <id>` 10 times on each.
   - Compare the medians per OS, and record the run IDs.
4. **Apply the pass rule.** On ubuntu and on macOS, the median after the
   change is no more than 5% above the median before it.
5. **If the rule fails,** the implementer either retunes or reverts.
   - **Retuning** means changing `RESOLVED_STYLES_SHARE`, or putting an
     explicit CPU-derived `-n` (or `-n logical`) in place of `-n auto`. The
     load comparisons recorded in Phases 3 and 6 point at which way to go.
   - **Reverting** removes the worker phase only, and resets that suite's
     Landing Status to "not landed".
   - **Re-landing** repeats Phase 3's or Phase 6's timing, 10-run and
     anti-masking criteria, then this phase with new windows.
6. **On a pass,** update 0364's Landing Status and close the item once both
   suites have passed.

### Success Criteria:

#### Manual Verification:

- [ ] Both windows (or the paired run IDs) are recorded in each suite's PR.
- [ ] The 5% rule passes on both OSes for both suites.
- [ ] The work item's Landing Status and status are updated.

---

## Testing Strategy

### Unit Tests:

- `allocateWorkers` at 1, 3, 4 and 16 CPUs. The cases cover the floor of one
  worker, rounding down, and chromium's single worker.
- `playwright.config.ts` loaded with and without `CI`, with
  `availableParallelism` mocked to 4, checking retries, traces, the literal
  per-project worker counts and the global cap.
- The `test:integration:dev` argv includes `-n auto`.

### Integration Tests:

- The dev suite's own tests carry the new evidence:
  - Negative-path tests assert output unique to their branch, and the
    absence of their sibling branch's output.
  - Happy-path stops assert four things: no recorded fake survives, every
    fake not killed out of band received SIGTERM, our code did not escalate,
    and there is no timeout-branch output.
  - Harness self-tests show each clean-stop assertion failing with its own
    message: a forced kill, a SIGTERM of a non-fake, a survivor, and an
    unsignalled fake (in test 16). An `ast`-based guard test pins every
    evidence string to a string constant in `lifecycle.py`.
- The Playwright suite runs with both projects at once. The JSON-reporter
  check proves they overlap and that no two `chromium` tests overlap.
- `resolved-styles` passes once against mutated copies of the fixtures that
  `chromium` writes.

### Manual Testing Steps:

1. Run a deliberately failing Playwright test and confirm that its trace
   remains.
2. Run `test_sigterm_ignoring_frontend_is_sigkilled` and inspect the
   workspace's `fakes.log` and `escalations.log`.
3. Run the `fs_usage` (macOS) or `strace` (Linux) trace for each project
   during the audit, and the mutated-fixture run.
4. Run the 10-run `mise run` validation for each suite on engywook, idle,
   outside 0370's recording.

## Performance Considerations

- **Dev suite under `-n auto`.**
  - `-n auto` uses physical cores when psutil is importable: 16 on engywook,
    possibly fewer than the vCPU count on CI. Playwright counts logical
    CPUs. Phase 3 records both counts from the CI banners, where they can
    differ, and compares `-n auto` with `-n 8` under load on engywook.
  - Under `default`, each worker's circusd and four fakes compete with the
    other lanes. Under that load the 0.5 s `grace_kill` is the most likely
    source of flakes, through the fakes' reap and through `kill_arbiter`'s
    wait after SIGTERM. `stop_cleanly` makes such a flake visible rather
    than silent.
- **Cost moved onto co-running lanes.** The per-task gates can't see it. The
  source research saw `test:unit:visualiser` go from 64 s to 162 s when the
  suites ran in parallel. Phases 3 and 6 record a non-isolated `default`
  measurement, and Phase 7 records the job wall-clock. None of these is a
  gate.
- **Playwright's spread.** File-level distribution across 17 uneven specs
  limits what eight workers can achieve, and the largest generated-loop
  specs set the wall time. `fullyParallel` is held back as a lever. Phase 5's
  per-file durations decide whether to use it before the 10-run validation.
- **Local targets are specific to engywook.** It has 12 performance cores and
  4 efficiency cores, and both counts treat them alike. The Phase 7 CI
  comparison is the gate across hosts.
- **Smaller savings under `default`.** The earlier research measured e2e at
  107 s under full load, against a projected 60 s. The local targets are
  isolated measurements, so `default` will show smaller savings.

## Migration Notes

There's no data migration. Phases 2 and 5 merge before their track's worker
phase, so each CI before window already contains them. A revert in Phase 7
removes only the worker phase.

## References

- Original work item: `meta/work/0364-parallelise-the-serial-test-suites.md`
- Codebase research: `meta/research/codebase/2026-11-15-0364-parallelise-the-serial-test-suites.md`
- Source research: `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- Prior dev-suite flake: `meta/prs/46-description.md`
- Task-argv test precedent: `tests/unit/tasks/test_hooks_task_environment.py`
- Node-side helper precedent: `cli/visualiser/frontend/scripts/visual-diff-ciede2000{,.test}.ts`
