---
type: "codebase-research"
id: "2026-11-15-0364-parallelise-the-serial-test-suites"
title: "Codebase research for 0364: parallelise the serial test suites"
date: "2026-11-15T16:05:23+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0364"
parent: "work-item:0364"
relates_to: ["codebase-research:2026-10-28-full-build-duration-contention-and-slowness"]
topic: "What the Playwright and dev-task integration suites look like today, and what 0364's requirements will run into when parallelising them"
tags: ["research", "codebase", "build-system", "playwright", "pytest", "pytest-xdist", "visualiser", "dev-task", "circus", "testing", "performance"]
revision: "bb78a017090f89db493fd365182ea0c45a511f0d"
repository: "accelerator"
last_updated: "2026-11-15T16:05:23+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Codebase research for 0364: parallelise the serial test suites

**Date**: 2026-11-15 16:05 UTC
**Author**: Toby Clemson
**Git Commit**: bb78a017090f89db493fd365182ea0c45a511f0d (not pushed, so
references are local paths)
**Branch**: detached working copy (jj), `build-system` workspace
**Repository**: accelerator

## Research Question

For work item `meta/work/0364-parallelise-the-serial-test-suites.md`: what
do the Playwright visualiser suite and the dev-task integration suite look
like today? What will each of the item's requirements touch, depend on or
run into?

## Summary

Both changes are smaller in code than in validation. But four findings
change how the plan should approach them, and one of them contradicts a
premise of the work item.

1. **The fixture audit (Requirement 2) will find real conflicts.**
   - **What reads the written files.** `chromium` specs write work items 0001,
     0005, 0006 and 0003. `resolved-styles` specs visit `/kanban`,
     `/library/work-items`, `/library/work-items/0001-first-work-item`,
     `/library` and `/lifecycle*`, and all of those depend on those same
     files.
   - **A write that is never restored.** `default-pattern.spec.ts` PATCHes
     0001 to `ready` and never restores it.
   - **The opposite direction is empty.** `resolved-styles` itself writes
     nothing, so the second conflict class (one `resolved-styles` spec
     writing what another reads) is empty.
   - **Where to resolve.** The conflicts must be resolved on the
     `resolved-styles` side, as Requirement 2 says. The likely fix is to point
     those specs at fixtures no `chromium` spec writes, such as 0002, 0004,
     0099 or a new dedicated fixture.
   - **The hard part.** The kanban and listing routes aggregate every work
     item, so the audit has to settle what "reads" means for them. They are
     served from the server's in-memory index rather than by opening files
     per request.
2. **`probe_timeout` is not a pure delay.** The work item says happy-path
   teardown "probes an arbiter known not to be running". That holds for
   teardown, but the same `probe_timeout` is also the per-call budget for
   *live* arbiter calls: starting the frontend watcher, recording its PID,
   the quit request and `status`. If it is lowered too far, the slow-arbiter
   flake recorded in PR 46 comes back on loaded CI runners. The production
   default is 2.0 s and the driver's is 5.0 s.
3. **`grace_kill` is a fixed sleep in production code.** It lives in
   `reap_descendants`, which every clean teardown calls, even when nothing
   is left to reap. Lowering the driver's default is a test-side change, as
   the item intends. It also lowers the arbiter's SIGTERM→SIGKILL escalation
   budget in `kill_arbiter`, which is a poll, but happy-path tests never
   reach that branch.
4. **SIGKILL escalation leaves no trace today.**
   - **No output.** `kill_arbiter` returns `StopResult("clean")` whether the
     arbiter died on quit, on SIGTERM or on SIGKILL. `reap_descendants`
     logs nothing, and the driver prints only `{kind, pid}`. So the
     acceptance criterion "no stop escalates to SIGKILL" cannot be asserted
     without new observability.
   - **Shared substrings.** One negative-path test asserts a substring
     (`"circusd.log"`) that a sibling branch also produces.
   - **One test's branch emits nothing.** The test of a SIGTERM-ignoring
     frontend exercises circus's own escalation, which emits no string at
     all.
   - **A test-side option.** A test-side way to add the observability
     exists: let the fakes record SIGTERM receipt.

Beyond those, the build and CI wiring are simple:
- `tasks/test/integration.py:57` runs `uv run pytest tests/integration/dev
  -v`.
- `pyproject.toml`'s `dev` group gains `pytest-xdist`, and `uv.lock` must be
  regenerated because installs use `--frozen`.
- The worker-count function can live under `cli/visualiser/frontend/scripts/`,
  where `tsconfig.node.json` and Vitest already pick it up.
- The xdist state-isolation assumption holds: ports are never bound, and
  sockets and lock files are scoped to each workspace.

## Detailed Findings

### A. Playwright configuration

**`cli/visualiser/frontend/playwright.config.ts`**

The file is 55 lines and imports nothing local.
- `retries: 1` (line 9) applies in every environment.
- `workers: 1` (line 14). Its comment (lines 10–13) still cites the
  visual-regression snapshot specs, which no longer run under this config.
- `trace: "on-first-retry"` sits in the top-level `use` (line 20).
- `globalSetup` and `globalTeardown` are set at lines 15–16.
- There is no `reporter` and no `outputDir`, so Playwright's defaults
  apply: the list reporter and `test-results/`.
- `fullyParallel` is not set.

Projects (lines 22–42):

| Project | testDir | Specs | Tests |
|---|---|---|---|
| `resolved-styles` | `./tests/resolved-styles` | 17 | 313 |
| `chromium` | `./e2e` (inherited) | 16 | ~43 |

- **`resolved-styles`** has the comment at lines 24–29: "project order is
  unspecified under workers:1; do not add a resolved-styles spec that reads
  live work-item status".
- **`chromium`** has the comment at lines 35–38. It says the
  clean-fixture-before-drag guarantee now comes from the global
  setup/teardown restore, not from a project dependency.

`webServer` (lines 43–54):
- It runs `node e2e/start-server.mjs` against a health port taken from
  `E2E_HEALTH_PORT`, which the task allocates (lines 3–4).
- `reuseExistingServer: !process.env.CI && ALLOCATED_HEALTH_PORT ===
  undefined` (line 50). This is the only place `CI` is read, and it doesn't
  govern workers or retries.

**`playwright.docker.config.ts`** is a standalone `defineConfig` that
repeats `retries: 1` (26), `workers: 1` (27) and `trace: "on-first-retry"`
(32) by hand. It shares nothing with the main config, so changing the main
config leaves the Docker lane alone, as the item requires.

**How the suite is launched**
- `tasks/test/e2e.py:22-33` runs `npm --prefix FRONTEND run test:e2e`.
- `package.json:21` maps that script to `playwright test`.
- The task sets env through `playwright_environment` (`e2e.py:36-46`), and
  `tests/unit/tasks/test_e2e_visualiser.py` tests that function.
- No `--workers` flag is passed anywhere.
- `CI` is never set by the task. It reaches Playwright only from GitHub
  Actions' own `CI=true`.

**Version.** `package.json:40` asks for `^1.59.1`, and the lock file
resolves exactly 1.59.1. Per-project `workers` (1.52+) is available. Node
`>=22` (`package.json:7`) provides `os.availableParallelism()`.

**Parallelism within files.** Playwright parallelises across *files* by
default. Tests in one file stay on one worker unless `fullyParallel` or
`test.describe.configure({ mode: "parallel" })` is set. Neither appears in
the 17 `resolved-styles` specs; the five uses of `mode: "parallel"` are all
in `tests/visual-regression/`. So eight workers over 17 files will be
uneven: the largest generated-loop specs (`fixture-coverage`,
`typography-resolved-sizes`, `root-resolved-tokens`) set the wall time. The
6-worker experiment (34 s) ran under the same file-level distribution. The
work item doesn't require `fullyParallel`, but per-project `fullyParallel:
true` on `resolved-styles` would be a tuning lever if 75 s is missed.

### B. Fixture writes and reads (Requirement 2 audit)

**How fixtures are served.**
- `e2e/start-server.mjs` serves `cli/visualiser/server/tests/fixtures` *in
  place*: `cwd: project`, `FIXTURES_PATH=…/fixtures/meta` (lines 27, 75,
  94–102). Nothing is copied.
- At startup it removes `.accelerator/tmp`, writes `.accelerator/config.md`
  and writes `frontend/.e2e-port`.

**Global writes**
- `e2e/global-setup.ts:17-43, 63-71` restores from any leftover
  `.e2e-fixture-snapshot.json`, then snapshots every `meta/work/*.md`.
- `e2e/global-teardown.ts:1-8` rewrites *every* `meta/work/*.md` from the
  snapshot, then deletes it.
- Both run once per invocation, outside any project.

**Per-spec writes (all `chromium`)**

| Spec | File | How | Restored |
|---|---|---|---|
| `kanban.spec.ts:14-57` | 0001 | drag → server PATCH | `writeFileSync` in `finally` after awaiting the PATCH |
| `kanban.spec.ts:59-108` | 0005 | drag → PATCH (with an SSE second tab) | `finally` |
| `kanban-click-vs-drag.spec.ts:20-41` | 0001 | none expected, but `finally` always writes | identical-content rewrite |
| `kanban-conflict.spec.ts:22-36,73-75` | 0006 | route interceptor writes `status: done`, waits 300 ms, `route.fetch()` (expects 412), writes back | `finally` too |
| `kanban-drag-overlay.spec.ts:71-114` | 0005 | API PATCH to `done`, retrying on 412 | API PATCH back to the status read from disk |
| `default-pattern.spec.ts:59-81` | 0001 (`docs[0]`, path-sorted) | API PATCH `status: ready`, accepts 204 or 412 | **not restored** until global teardown |
| `legacy-schema.spec.ts:46-78` | 0003 | the lookup uses `tests/fixtures/meta/work/0003…` (line 22), which probably fails to match the API's `meta/work/…` relPaths, so the test skips | `finally` still rewrites 0003 with identical content (line 76) |

- Mocked, with no writes: `kanban-keyboard`, `kanban-cross-config` (via
  `e2e/mock-board.ts`) and `kanban-columns`.
- Read-only: `cross-refs`, `dev-design-system`, `mermaid`, `wiki-links`,
  `navigation` and `smoke`.
- `project-pattern-custom-columns` sends PATCHes that are rejected with 400
  before any write.

**What `resolved-styles` reads.** A grep over `tests/` finds no
`writeFile` and no `request.patch`/`post`. The routes that touch written
files:
- `/kanban`, which shows all work items:
  - `typography-resolved-sizes.spec.ts:134-151`, on
    `.ac-kcard__title`/`__id` with `.first()`;
  - `radius-resolved-radii.spec.ts:133-140`, on `.ac-stagedots__dot`.
- `/library/work-items/0001-first-work-item`, from
  `DETAIL_ROUTE_SLUGS["work-items"]` (`tests/lib/detail-route-slugs.ts:22`).
  It is used by `fixture-coverage.spec.ts:17-27` and
  `detail-eyebrow-resolved-colours.spec.ts:11-13`. Detail bodies are read
  **from disk** on each request (`server/src/api/docs.rs:70-76`).
- The `/library/work-items` listing: `fixture-coverage.spec.ts:9-15` and
  `non-regression-glyph-consumers.spec.ts:41-43`.
- `/library` hub counts, and `/lifecycle` with `/lifecycle/first-plan`,
  which use clusters that include work items: `root-resolved-tokens`,
  `radius`, `typography`, `sidebar-meta-muted` and `eyebrow-unification`.
- `/library/work-items/0099-ac2-coverage` (`aside-row-resolved-colours`) and
  two decision detail pages. Nobody writes 0099, but its related rows come
  from clusters that are recomputed after any PATCH (`docs.rs:276-309`).

**How the server propagates a write**
- **PATCH.** An API PATCH updates the in-memory index, recomputes clusters
  and broadcasts SSE `DocChanged` straight away (`docs.rs:262-323`).
  `mark_self_write` suppresses the watcher's echo (`docs.rs:242-249`).
- **Direct file write.** A raw `writeFileSync`, including the
  identical-content restores and global teardown, goes through the `notify`
  watcher instead. That path has a 100 ms debounce per file, then re-index
  and broadcast (`server/src/watcher.rs:26-29, 36-43, 79-147, 164-189`).
- **Open pages.** Pages that are open subscribe over SSE and refetch.
- **Listings.** Listing and kanban data come from the index, not from
  re-reading files.

**Consequences for the audit**
- **Where the conflicts are.** `chromium` writes 0001, 0005, 0006 and 0003,
  and `resolved-styles` reads them through the 0001 detail page (which
  opens the file) and through the aggregate routes (the index). That is the
  conflict set.
- **Detail pages are clean to fix.** The detail-page conflict is easy: point
  `DETAIL_ROUTE_SLUGS["work-items"]` at a fixture no `chromium` spec writes,
  such as 0002, 0004 or 0099, or at a new one.
- **`fs_usage` misses most of it.** The aggregate routes are where
  Requirement 2's "trace the files the server opens" is inadequate on its
  own. While `resolved-styles` runs alone, the server opens every work item
  once at startup indexing, and then serves `/kanban` and the listings
  without opening anything. A trace therefore either shows everything
  (startup) or nothing (requests). The audit has to read the server's
  listing and cluster paths as well, and decide whether an index-served
  aggregate counts as a "read" of each file it includes.
- **What resolving the aggregates involves.** A conservative resolution
  makes the `/kanban` and listing assertions independent of which cards are
  present: select a card for a fixture nobody writes, rather than
  `.first()`. Status moves change column membership and stage dots, which
  is exactly what `.first()` would pick up.
- **0001 under `default-pattern`.** Because `default-pattern` leaves 0001
  at `ready`, 0001's status during the run depends on whether that test
  has run yet. That is a strong reason to keep `resolved-styles` away from
  0001 entirely.

### C. Dev-task integration suite

**How it runs**
- `tests/integration/dev/test_dev_integration.py` invokes
  `dev_integration_driver.py` as a subprocess for each action (`run_driver`,
  lines 32–57).
- Per-action `opts` are written to `workspace/opts-{action}.json`, then
  `_build_deps` (driver 129–197) turns them into a real `DevDeps`. That
  gives real circusd and psutil, a `Clock()`, and Python fake server and
  npm scripts.
- Opts apply to that one invocation only. The fixture's cleanup `stop`,
  `_hard_cleanup` (test 115–130), uses the driver defaults.
- There is no `conftest.py` in the directory.

**Budgets: driver defaults against production defaults**

| Budget | Driver (`dev_integration_driver.py:191-196`) | Production (`tasks/shared/dev/lifecycle.py:76-86`) | Kind |
|---|---|---|---|
| `probe_timeout` | 5.0 | 2.0 | per-call zmq poll; the full value is spent when no arbiter answers |
| `pidfile_timeout` | 20.0 | 10.0 | poll |
| `readiness_timeout` | 20.0 | 30.0 | poll |
| `frontend_active_timeout` | 20.0 | 15.0 | poll |
| `grace_quit` | 6.0 | 5.0 | poll until dead |
| `grace_kill` | 3.0 | 2.0 | **fixed sleep** in `reap_descendants`; poll in `kill_arbiter` |
| `frontend_settle` | (not overridden) | 0.5 | fixed sleep on every successful up (`lifecycle.py:457`) |

The comment above the budgets (driver 172–185) is a single paragraph
**duplicated word for word**. Lines 187–190 add a note on the
frontend-watcher flake.

**`grace_kill`**
- `reap_descendants` (`lifecycle.py:753-780`) SIGTERMs the
  identity-matched baseline, calls `_sleep(deps, deps.grace_kill)` (a
  deadline loop that never checks liveness, 840–843), then SIGKILLs any
  survivors.
- `teardown` calls it for every non-survivor result (`lifecycle.py:702`),
  even when the baseline is empty.
- `kill_arbiter` (736–750) uses `grace_kill` as the poll budget after
  SIGTERM and again after SIGKILL of the arbiter. That is reached only if
  `grace_quit` expires.
- `_wait_until_dead` (818–825) returns as soon as the process dies.

**`probe_timeout`**
- **Mechanism.** `_probe_status` (175–184) builds a circus client with
  `timeout=probe_timeout`. One zmq poll with no answer raises `CallError`,
  which is mapped to `SupervisorUnreachableError` and then `None`.
- **Call sites:**
  - `reuse_or_teardown` (208)
  - lock contention in `bring_up` (236–238)
  - the `start_frontend` client (423), whose timeout covers each start or
    status call
  - `_record_watcher_pid` (588)
  - `kill_arbiter`'s probe and quit (711, 714)
  - `_wait_until_unreachable`, on each probe (833)
  - `_stop_without_state` (869)
  - `do_status` (889)
- **Two roles.** The same value is the "how long to wait for a dead
  arbiter" cost *and* the "how long a live, loaded arbiter has to answer"
  budget.
- **Prior flake.** `meta/prs/46-description.md` records the slow-answer
  case flaking `test_restart_round_trip` on macOS CI. Frontend activation
  was made a poll that retries an unreadable status.

**Where the time goes at the driver defaults**
- **Cleanup `stop` on an already-stopped stack.** This goes through
  `_stop_without_state`, which spends a full `probe_timeout` (5 s) and no
  `grace_kill`. It applies to tests 4, 5, 6, 8, 10 and 11, and to the "a"
  workspace in test 17.
- **Cleanup `stop` on a live stack.** The probe answers, quit is sent and
  the poll exits early, but the fixed `grace_kill` (3 s) is still spent.
  It applies to tests 1, 2, 3, 7, 13, 14 and 15.
- **Dead-arbiter stops** (tests 5, 6 and 11, the stop in 10, and cleanup
  after 9 and 12). These spend `probe_timeout` + `grace_kill`.
- **Every successful `up`.** This spends the production `frontend_settle`
  of 0.5 s.

**The 17 tests**

| # | Test | Line | Path | Notes |
|---|---|---|---|---|
| 1 | `test_detach_readiness_and_log_routing` | 144 | happy | log markers via `wait_for_text` |
| 2 | `test_reuse_starts_no_duplicate` | 169 | happy | |
| 3 | `test_stale_info_does_not_satisfy_gate` | 178 | happy | `api_port: 8888` |
| 4 | `test_clean_teardown_leaves_no_orphans` | 189 | happy | |
| 5 | `test_orphan_reach_when_arbiter_already_dead` | 205 | happy (recovery) | settle sleep at 210 |
| 6 | `test_orphan_reach_with_only_server_recorded` | 219 | happy (crafted state) | settle sleep at 235 |
| 7 | `test_restart_round_trip` | 267 | happy | flaked historically (PR 46) |
| 8 | `test_readiness_timeout_fails_and_tears_down` | 278 | **negative: readiness budget** | `readiness_timeout: 2.0`, `server_write_info: False` |
| 9 | `test_daemon_startup_failure_reaps_handle_no_retry` | 287 | **negative: pidfile budget** | `pidfile_timeout: 2.0`, `fake_circusd_fail: True` |
| 10 | `test_discovery_survives_lost_state_file` | 295 | happy | |
| 11 | `test_stale_cleanup_after_out_of_band_kill` | 303 | happy (recovery) | settle sleep at 309 |
| 12 | `test_recycled_pid_is_refused_real_psutil` | 318 | negative, **not a timeout branch** (identity refusal) | |
| 13 | `test_frontend_resolves_npm_under_stripped_path` | 349 | happy | `strip_path` |
| 14 | `test_frontend_port_and_info_wiring` | 356 | happy | `free_port: 45678`, rendered only |
| 15 | `test_status_exit_codes` | 364 | happy | already polls for status 3 (376–380) |
| 16 | `test_sigterm_ignoring_frontend_is_sigkilled` | 384 | **negative: SIGKILL escalation**, but circus's own (`graceful_timeout = 2`), not `kill_arbiter`'s | `fe_ignore_sigterm`, `grace_quit: 8.0` |
| 17 | `test_cross_workspace_concurrency` | 393 | happy | two stacks side by side |

No test sets `probe_timeout`, `grace_kill` or `frontend_active_timeout`.

**Candidate entries for the negative-path record (Requirement 6)**

| Test | Branch | Output unique to the branch | What the test asserts now |
|---|---|---|---|
| 8 | readiness budget expiry, `_readiness_gate` (`lifecycle.py:398-418`) | message `"server did not write {server_info_path} in time; see {dev_dir}/server.log"`; log `"server-info.json did not appear before the readiness timeout"` | rc 1, `"server.log" in message`, no state. Adequate, but tighten to "did not write … in time" |
| 9 | pidfile budget expiry, `allocate_and_launch` with `pid is None` (`lifecycle.py:327-346`) | message `"the circus daemon did not start (no pidfile at {pidfile}); see {dev_dir}/circusd.log"`; log `"circusd did not write a valid pidfile at {pidfile}"` | rc 1, `"circusd.log" in message`. **Not unique**: the "pidfile PID not live" sibling (348–361, `"the circus daemon exited immediately; see …/circusd.log"`) also matches |
| 16 | SIGKILL escalation of a SIGTERM-ignoring frontend | **None.** Circus logs to `circusd.log` (its level wasn't checked); our code returns `"clean"` and logs nothing; `reap_descendants` would also SIGKILL it silently | only `not alive(frontend_pid)` |

The work item's branch list also names "a stop that escalates to SIGKILL
after `grace_kill`" (the `kill_arbiter` escalation) and "frontend-active
budget expiry". **No test exercises either one**:
- `kill_arbiter` escalation would produce the `"… still alive after SIGKILL"`
  survivor message, but only when even SIGKILL fails.
- Frontend-active expiry would produce `"the frontend watcher did not become
  active (…); see …/frontend.log"` (`lifecycle.py:452-453`).

Other branches that also have no test:
- frontend start refused (427–438)
- "became active then stopped" (458–461)
- pid-unknown survivor (730)
- lock contention (245–252)

**Observability for the happy-path acceptance criterion**

Happy-path tests check only exit codes, `kind`, state fields and psutil
liveness. "No stop escalates to SIGKILL" can't be observed today, for two
reasons:
- `StopResult` carries no escalation field.
- The driver prints only `{kind, pid}` for stop (driver 222).

There are two ways to add it:
- **Test-side, within the suite's own files.** Give the fake server and fake
  npm a SIGTERM handler that appends a marker to their log, then exits. Do
  the same for their `sleep` children, which today are bare
  `python -c 'time.sleep(3600)'` processes. A happy-path test then asserts
  that every fake logged its SIGTERM receipt. A process that never logged
  it was SIGKILLed.
- **Production-side.** Make `reap_descendants` or `kill_arbiter` log or
  report escalation. This changes `tasks/shared/dev/lifecycle.py`, which is
  outside the suite's tests, fixtures and config.

**The fakes** (driver 45–79):
- They keep Python's default SIGTERM disposition, so they exit at once.
  `server_ignore_sigterm`/`fe_ignore_sigterm` installs `SIG_IGN` instead
  (line 74).
- Spawned children get a fresh interpreter, so they don't inherit the
  ignore.
- After circus `quit` (SIGTERM with `stop_children`), every tracked process
  is normally dead well before the `reap_descendants` sleep. That sleep is
  pure dead time.

**Settle sleeps and their real conditions**

| Line | Test | Condition actually awaited | Ready-made predicate |
|---|---|---|---|
| 210 | 5 | the SIGKILLed arbiter is dead | `alive(arbiter)` (test 85–91) |
| 235 | 6 | the inline server has spawned its worker | `descendants(server.pid)` non-empty (test 106–112) |
| 309 | 11 | the arbiter, server and frontend are dead, so `status` sees "unreachable" | `alive(...)` on all three |

- **Local helpers.** The file already has `wait_for_text` (94–103) and an
  inline status poll (372–381). Both use `time.time()`, not `monotonic`.
- **No shared helper.** There is no shared `wait_until` in the repo. The
  closest shapes are:
  - `_deadline_wait` (`tests/unit/tasks/shared/test_processes.py:222-228`),
    which returns a bool;
  - `_wait_until` (`tests/unit/tasks/shared/build_measurement/test_samplers.py:30-34`),
    which asserts inside;
  - the production `tasks/shared/polling.py:wait_for_file`, which injects a
    clock.

**xdist isolation (the Assumption)**

The assumption holds.
- **Workspace.** Each test owns `tmp_path / "ws"` (test 133–138). All
  state lives under `workspace/.accelerator/tmp/{dev,visualiser}`.
- **Locks.** The lock is `fcntl.flock` on that workspace's `dev.lock`.
- **Sockets.** The IPC sockets share one directory,
  `tempfile.gettempdir()/acc-dev-<sha256(workspace)[:12]>/` (`tasks/shared/dev/endpoints.py:26-39`).
  Each name is keyed by the workspace path, so distinct `tmp_path`s don't
  collide. The 104-byte path limit doesn't depend on `tmp_path` depth.
- **Ports.** `api_port` (7777, 8888), `free_port: 45678` and
  `frontend_port: 54321` are rendered or written, **never bound**. The fake
  npm never binds its port.
- **Environment.** HOME is untouched, and nothing writes into the repo root.
- **One small risk.** `test_cross_workspace_concurrency` asserts that two
  consecutive `free_port()` bind-to-0 calls return different ports
  (line 401). Under xdist another worker could take and release a port in
  between, but the kernel rarely hands back the same ephemeral port
  immediately.

**What `-n auto` resolves to.** pytest-xdist's `auto` uses
`psutil.cpu_count(logical=False)` when psutil is importable. psutil is in
the `build` group, which `uv sync --all-groups` installs. Otherwise `auto`
falls back to the logical CPU count.
- **engywook.** It has 16 physical cores, so 16 workers for 17 tests. Each
  worker runs circusd plus four fake processes, and under `default` they
  share the host with every other lane.
- **CI.** On GitHub runners `auto` may resolve to fewer workers than there
  are vCPUs.

This is relevant to the CI closing criterion and to the "explicit
CPU-derived `-n`" retuning lever. Check it with `pytest -n auto --collect-only`
or the xdist startup banner on each host.

### D. Build-system wiring

- **`dev` task.** `tasks/test/integration.py:54-57` runs `context.run("uv run
  pytest tests/integration/dev -v")`. Every pytest-backed integration task
  uses the same shape.
- **Task test precedent.** No test pins any integration task's argv. The
  closest is `tests/unit/tasks/test_hooks_task_environment.py`
  (`mocker.MagicMock(spec=Context)`, then `context.run.call_args`), which
  0362 also used. This gives the red-green test for `-n auto`. These tests
  run under `test:unit:tasks`.
- **Dependencies.** `pyproject.toml:39-61` holds the `dev` group:
  `"pytest>=8"`, `"pytest-mock>=3.14"`, with `>=` floors carrying a
  one-line comment by convention. `[tool.pytest.ini_options]` (63–66) has
  only `--import-mode=importlib`, `pythonpath` and one filterwarning. There
  are no markers and no pytest-timeout.
- **Lock file.** `deps:install:python` runs `uv sync --all-groups --frozen`
  (`tasks/deps.py:20-22`), so `uv.lock` must be regenerated and committed
  with the `pyproject.toml` change.
- **mise wiring:**
  - `test:integration:dev` (`mise.toml:372-375`) depends only on
    `deps:install:python`.
  - `test:e2e:visualiser` (512–515) depends on `deps:install:playwright`,
    `build:frontend` and `build:server:dev`.
  - `tests/unit/tasks/test_mise.py:82` lists `test:integration:dev` in the
    launcher-dependency map. The golden `warm-no-op-tasks.golden.json` is a
    historical capture, so neither change touches it.
- **`default`** (`mise.toml:792-794`) runs everything concurrently, bounded
  by mise jobs (8 on engywook). `test:e2e:visualiser` and
  `test:integration:dev` overlap with each other, the unit lanes, docs,
  lint and types.

**`measure:build`** (`tasks/measure.py:3698-3763`; README "Build timing",
`tasks/README.md:392-519`)
- **Isolate mode.** `--isolate T` runs `mise run --output prefix
  --skip-deps T` (`tasks/shared/build_measurement/settings.py:96-105`).
- **Cache state.** `warm-no-op` makes one priming run that is discarded.
- **Environment.** `MISE_TASK_TIMINGS=1` is forced (`build_runner.py:23-31`).
- **Keep.** `--keep` writes `meta/measurements/build_<UTC>_<host>_<cache>_<settings>_isolated.json`.
- **Host settings.** `--host-settings` **defaults to `unspecified`**, which
  compares with nothing. The acceptance-criterion commands in 0364 omit
  it, but the Dependencies require records that show `devtools`, so every
  invocation must add `--host-settings devtools`.
- **Exit status.** The task exits 0 whenever it writes a record, even if a
  measured task failed, so the record's per-task outcome must be checked.

### E. CI

`.github/workflows/main.yml`:
- **`test-integration`** (63–110) runs `mise run test:integration`, a
  roll-up of 11 members that run concurrently, on `ubuntu-latest` and
  `macos-latest`.
- **`test-e2e`** (112–142) runs `mise run test:e2e`, a roll-up of only
  `test:e2e:visualiser`, on `ubuntu-24.04` (pinned for Playwright's
  Chromium) and `macos-latest`.
- **`CI`** is never set explicitly.
- **Concurrency.** There is no `MISE_JOBS`.

In the CI job log, the "Finished in" line for `test:integration:dev` is
one of 11 interleaved task timings. Requirement 8 extracts that per-task
line, not the job duration. `timeline.py:20` has the regex the measurement
harness uses.

## Code References

- `cli/visualiser/frontend/playwright.config.ts:9,14,20,22-42,43-54` — retries, workers, trace, projects, webServer
- `cli/visualiser/frontend/playwright.docker.config.ts:26-32` — the Docker lane's own copies (out of scope)
- `cli/visualiser/frontend/e2e/start-server.mjs:27,60-102,125-139` — fixtures served in place
- `cli/visualiser/frontend/e2e/global-setup.ts:17-43,63-71`, `e2e/global-teardown.ts:1-8` — snapshot/restore
- `cli/visualiser/frontend/e2e/default-pattern.spec.ts:59-81` — the unrestored PATCH of 0001
- `cli/visualiser/frontend/e2e/legacy-schema.spec.ts:22,57-59,76` — the probably-skipping lookup and the unconditional rewrite
- `cli/visualiser/frontend/e2e/kanban*.spec.ts` — the drag and conflict writes (table in B)
- `cli/visualiser/frontend/tests/lib/detail-route-slugs.ts:22` — the work-item detail slug used by `resolved-styles`
- `cli/visualiser/server/src/api/docs.rs:39-40,70-76,242-249,262-323` — index-served listings, disk-read details, PATCH propagation
- `cli/visualiser/server/src/watcher.rs:26-29,79-189` — debounced watcher re-index and SSE
- `cli/visualiser/frontend/vite.config.ts:66-88` — Vitest config (default include; excludes `e2e/`, `tests/`)
- `cli/visualiser/frontend/tsconfig.node.json:16-21` — covers `playwright.config.ts` and `scripts/**/*.ts`
- `cli/visualiser/frontend/scripts/visual-diff-ciede2000{,.test}.ts` — precedent for a Node-side helper with a sibling test
- `tests/integration/dev/dev_integration_driver.py:45-79,129-197,209-229` — fakes, deps, the duplicated comment, exit codes
- `tests/integration/dev/test_dev_integration.py:32-57,85-112,115-138,210,235,309,372-381` — harness, predicates, settle sleeps, status poll
- `tasks/shared/dev/lifecycle.py:76-86` — production defaults
- `tasks/shared/dev/lifecycle.py:175-184,398-418,327-361,452-461,685-750,753-780,818-843,857-871` — probe, branches, teardown, `reap_descendants` sleep
- `tasks/shared/dev/endpoints.py:26-39` — per-workspace IPC socket paths
- `tasks/shared/processes.py:61-67` — `is_alive` treats zombies as dead
- `tasks/test/integration.py:54-57` — the dev suite command
- `tasks/test/e2e.py:22-46` — the Playwright task and its env
- `tests/unit/tasks/test_hooks_task_environment.py` — task-argv test precedent
- `pyproject.toml:39-66`, `uv.lock`, `tasks/deps.py:20-22` — dev deps, pytest config, `--frozen`
- `tasks/measure.py:3698-3802`, `tasks/shared/build_measurement/settings.py:96-205`, `tasks/README.md:392-519` — measurement
- `.github/workflows/main.yml:63-142` — CI jobs

## Architecture Insights

- **Fixtures are shared state by design.** The visualiser e2e harness serves
  the committed fixture tree in place, because SSE tests need on-disk
  mutations the server really sees. Global snapshot/restore protects runs,
  not concurrent specs. Isolation is by convention: a config comment plus
  each spec restoring in `finally`. Parallelism makes that convention
  load-bearing, which is why the audit matters.
- **The server's index makes a "read" indirect.** Aggregate pages reflect
  every PATCH through the index and SSE. Any spec that asserts on `.first()`
  of a work-item list is order-sensitive to concurrent status writes, even
  if it never names a file.
- **The dev task gives one budget two roles.** In the dev task's
  dependency-injected deps, `probe_timeout` is both a wait for an absent
  peer and a deadline for a slow live peer. `grace_kill` is both a fixed
  dwell and an escalation budget. Lowering a single number therefore moves
  both behaviours, and the plan should reason about each one.
- **Tests aim at direction, not precise deadlines.** The driver comment
  says this suite asserts *direction*, "it happened", while unit tests in
  `test_dev.py` pin the exact poll and timeout arithmetic. Shortened waits
  should keep that split.
- **The repo's waits follow one shape.** Waits use an injected clock and
  `min(interval, deadline - now)` in production, and small local
  `time.monotonic` helpers in tests. There is no shared test-wait utility.
- **No pytest suite is parallel yet.** Every pytest lane is serial.
  `default` and the CI roll-ups get parallelism only from mise running
  lanes concurrently, so `-n auto` adds a second layer of concurrency on
  top of that.

## Historical Context

- `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`:
  - A5 and A6 (lines 309–339, 461–483) are the source of the baselines and
    experiments.
  - The combined what-if under full load (489–507) gave e2e 107 s rather
    than the projected ~60 s, so the savings shrink under `default`.
  - The mirror-CI decision (687–714) is why worker counts are
    host-relative.
  - The CI timings (722–729): dev 217 s on ubuntu and 190 s on macOS; e2e
    226 s on ubuntu and 305 s on macOS.
- `meta/plans/2026-06-12-0108-local-docker-visual-regression-baselines.md:49-60,675-736`:
  `workers: 1` originally existed so that snapshots captured clean fixtures
  before the drag tests mutated them. `resolved-styles` was split out under
  `tests/` with shared `tests/lib/` helpers.
- Known flake that is likely to resurface under more workers and no
  retries: `resolved-styles` `page.goto: net::ERR_ABORTED` on the first
  navigation under full load. It is recorded in:
  - `meta/plans/2026-10-06-0299-crate-dependencies-adr-0069.md:2102-2104`
  - `meta/validations/2026-09-26-0230-…-validation.md:52-61` (failed on both
    attempts)
  - `meta/plans/2026-06-20-0122-…md:253`
- `kanban-drag-overlay` Escape test: its watcher-timing flake was fixed by
  editing through the API (`meta/prs/159-description.md:202-204,341-347`).
  The watcher still ignores FSEvents "rescan".
- Health-port collisions from orphaned servers led to the per-run
  `E2E_HEALTH_PORT` (`meta/prs/149-description.md:122-123`).
- Dev suite:
  - `meta/plans/2026-06-06-0101-unified-dev-task-for-visualiser.md:1236-1257`
    sets out the isolation basis: tmp workspaces, per-workspace IPC and
    `free_port`.
  - `meta/prs/46-description.md:44-62,94,105` covers the
    `test_restart_round_trip` flake and why `probe_timeout` matters for
    live calls.
  - `meta/plans/2026-06-17-0114-…md:1174`: a 0.7 s post-SIGKILL sleep was
    too short under load.
  - `meta/plans/2026-09-25-0226-…md:2877-2881`: failures at load average 34.
- `meta/decisions/ADR-0041-circus-dev-process-supervision.md:86-92,140-142`
  sets production `graceful_timeout = 2` and `stop_children = true`. These
  stay unchanged.
- Siblings:
  - **0362** is done. It edited `tasks/test/integration.py` and the
    `test-integration` job, so check whether its merge falls inside the
    dev suite's CI before-window.
  - **0368** is done; it built `measure:build`.
  - **0369** is blocked by 0361.
  - **0370** toggles engywook's host settings. Don't schedule validation
    runs over it.
- A newer kept record exists:
  `meta/measurements/build_2026-11-15T100850Z_engywook_warm-no-op_devtools.json`.

## Related Research

- `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- `meta/reviews/work/0364-parallelise-the-serial-test-suites-review-1.md`

## Open Questions

1. **What counts as a "read" for index-served routes?** Under
   Requirement 2, does `/kanban`, the listings or `/lifecycle` count as
   reading every work-item file it aggregates, given that the server
   serves them from memory? The answer decides whether the fix is to
   repoint the detail slugs and drop `.first()` on cards, or to give
   `resolved-styles` an aggregate-free fixture world.
2. **How is "no stop escalates to SIGKILL" made observable?** The
   test-side option (fakes record SIGTERM receipt) stays inside the
   suite's files. The production option (an escalation log line) touches
   `lifecycle.py`; the item says "production defaults unchanged", which
   doesn't strictly forbid that, but it does fall outside the
   tests/fixtures/config boundary of Requirement 4.
3. **Which negative-path tests are on the record?** Test 16 targets
   circus's own escalation, emits no unique output, and can't distinguish
   circus's SIGKILL from `reap_descendants`'s. Does it go on the
   negative-path record, and with what evidence? Test 12 (identity
   refusal) is negative but has no timeout branch. Test 9's assertion
   needs tightening to a substring only its branch produces.
4. **How far can `probe_timeout` safely drop?** Because it also bounds
   live calls, should the driver default stay at or above production's
   2.0 s? Alternatively, could negative and recovery paths override it
   lower, so that the 5 s of dead-arbiter waiting is cut without starving
   live calls on loaded CI?
5. **What does `-n auto` resolve to on each runner?** It may count
   physical cores via psutil, so check whether it under-provisions the
   CI runners compared with what the closing criterion expects.
6. **Does `resolved-styles` need `fullyParallel`?** File-level
   distribution across 17 uneven specs may limit the win from 8 workers.
   Whether per-project `fullyParallel` is needed to meet 75 s is answered
   by the first `measure:build --isolate` run.
7. **Is `legacy-schema.spec.ts` skipping in every run?** Its PATCH test
   probably skips every run because of a wrong relPath constant. That is
   outside this item's anti-masking criterion, which covers only additions,
   but its unconditional `writeFileSync` still counts in the audit.
