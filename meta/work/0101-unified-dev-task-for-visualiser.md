---
type: work-item
id: "0101"
title: "Unified Managed dev Task for Visualiser Server and Frontend"
date: "2026-06-06T13:04:38+00:00"
author: Toby Clemson
producer: create-work-item
status: ready
kind: task
priority: medium
relates_to: ["work-item:0100"]
tags: ["dev-tooling", "mise", "invoke", "visualiser"]
last_updated: "2026-06-06T17:26:45+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0101: Unified Managed dev Task for Visualiser Server and Frontend

**Kind**: Task
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add a single `mise run dev` task that starts and manages both the visualiser
API server and the Vite frontend together, plus `dev:stop` / `dev:restart`
(and `dev:status`) lifecycle tasks. The processes run detached in the
background under a `circus` supervisor daemon, each writing to its own log
file, so a developer can bring the whole dev stack up and down with one command
instead of juggling two terminals. Re-running `dev` while a healthy session is
already up reuses it rather than starting a duplicate.

## Context

Today the dev workflow requires two tasks in two terminals:

- `mise run dev:server` → `invoke dev.server` — builds (via the
  `build:server:dev` dependency) and runs the Rust debug binary
  `skills/visualisation/visualise/server/target/debug/accelerator-visualiser`
  in the **foreground** under a pty. It writes config to
  `.accelerator/tmp/dev-server/config.json` (using `--owner-pid 0`, which
  disables owner-based auto-shutdown for the dev path) and the server writes
  its chosen port into `.accelerator/tmp/dev-server/server-info.json`.
- `mise run dev:frontend` → `invoke dev.frontend` — runs
  `npm --prefix skills/visualisation/visualise/frontend run dev` (Vite) in the
  **foreground**, with `VISUALISER_INFO_PATH` pointed at `server-info.json`.

All `mise` tasks are thin wrappers that delegate to Python `invoke` tasks
defined under `tasks/` (assembled in `tasks/__init__.py`; the dev tasks live in
`tasks/dev.py`). Neither dev task backgrounds its process or tracks a PID —
teardown is Ctrl-C in each terminal.

There **is** a real startup-ordering constraint. Vite's `resolveApiPort()` in
`frontend/vite.config.ts` reads the server port from `server-info.json` **at
config-eval time**. Its precedence is `VISUALISER_API_PORT` env → the port in
`server-info.json` → fall back to port 0 (which makes `/api` proxying fail
loudly). Because `dev.frontend` only sets `VISUALISER_INFO_PATH` (not
`VISUALISER_API_PORT`), the server must be up and have written
`server-info.json` before Vite starts. Currently this is enforced only by the
"start dev:server first" convention; there is no programmatic wait.

Strong prior art for managed background processes already exists in the
production `/visualise` launcher (not used by the dev tasks):
`scripts/launch-server.sh` and `scripts/launcher-helpers.sh` implement PID
files, `flock` locking, a reuse short-circuit for an already-running server,
SIGTERM → grace → SIGKILL escalation, and a start-time identity check that
refuses to kill a recycled PID. These are the patterns the dev task should
mirror.

No process-manager dependency currently exists — `pyproject.toml` provides only
`invoke` (plus build/test tooling). This item adds `circus` as the supervision
substrate (see the Decision subsection): a long-lived arbiter daemon supervises
both processes, gives native per-process log files via its `FileStream` stdout
stream, and exposes a Python `CircusClient` that the `invoke` tasks drive for
stop / restart / status / reuse. `mise` itself has no background-process
supervision and is not a fit for the lifecycle management — it remains only the
entry point.

## Requirements

1. **`mise run dev`** starts **both** the visualiser API server and the Vite
   frontend (the two managed processes) as **detached background processes**
   under the `circus` arbiter and returns promptly (the `dev` command launches
   the arbiter and detaches from it, returning to the shell while the arbiter
   keeps supervising both processes; it does not block streaming output). If a
   healthy `dev` session is already running, `dev` **reuses it**
   (reports success and does not start a duplicate), mirroring
   `launch-server.sh`'s reuse short-circuit. A session is **healthy** when the
   `circus` arbiter endpoint responds **and** both watchers (server and
   frontend) report a running/active state; an unreachable arbiter or a
   non-running watcher is treated as not-healthy and handled as stale state per
   Requirement 9.
2. **`mise run dev:stop`** tears down both processes (the API server and the
   Vite frontend) and all their child processes, and stops the `circus`
   arbiter.
3. **`mise run dev:restart`** performs a stop followed by a start.
4. **`mise run dev:status`** reports whether each process is running and, when
   running, surfaces the frontend URL and the resolved API port. Its exit code
   conveys overall state: **0** when both processes are running, **3** when only
   one is running, **4** when neither is running — the same codes on macOS and
   Linux.
5. **Separate log files** — each process writes its stdout/stderr to its own
   file (e.g. under `.accelerator/tmp/dev/`, such as `server.log` and
   `frontend.log`), discoverable after `dev` returns.
6. **Implemented as `invoke` tasks** in Python (`tasks/dev.py`), with the
   `mise` tasks delegating to them as the existing tasks do. The tasks add and
   drive the `circus` library (arbiter + `CircusClient`) — a new dependency in
   `pyproject.toml`.
7. **Enforce startup ordering** — start the server, then poll for
   `server-info.json` to exist (every 100 ms, timing out after **30 s**) before
   starting the frontend, so Vite reliably resolves the API port and never falls
   through to port 0. If the server fails to write `server-info.json` within the
   30 s timeout, `dev` exits non-zero with a clear error naming the missing file
   and the server log path.
8. **Run prerequisites** — ensure the equivalents of `build:server:dev` and
   `deps:install:node` run before launch (e.g. via `mise` `depends` or task
   composition), matching current behaviour.
9. **Robust teardown** — the `circus` arbiter owns each process in its own group
   and escalates SIGTERM → grace period → SIGKILL on stop, so Vite's child
   node/esbuild workers do not orphan; `dev:stop` and `dev:status` must treat a
   stale arbiter endpoint or a dead/recycled daemon (e.g. a leftover socket or
   pidfile after a crash) as "not running" and clean it up rather than acting on
   an unrelated process. The grace-period and stale-state behaviours mirror the
   `launcher-helpers.sh` patterns: a **start-time identity check** (compare a
   PID's process start time before signalling it, so a reused PID belonging to
   an unrelated process is never killed) and SIGTERM → 2 s grace → SIGKILL.
10. **Cross-platform** — start/stop/restart/status behave identically on macOS
    and Linux (developer machines and CI).

### Decision (resolved during review)

- **Supervision approach: `circus`.** The dev tasks add the `circus` library and
  drive its arbiter via `CircusClient`. Rationale: (a) Requirement 5 needs
  per-process log files, which circus provides natively via its `FileStream`
  stdout stream — `honcho` only multiplexes to a single combined stream;
  (b) the long-lived arbiter makes "is a session already running?" detection,
  the reuse short-circuit (Requirement 1), and `dev:status` reporting natural;
  (c) the Python `CircusClient` fits the `invoke`-task control model more
  cleanly than `supervisor`'s XML-RPC + config-file model. circus runs on macOS
  and Linux, satisfying Requirement 10. Because this adds a runtime dependency,
  **an ADR should record this choice** (see Dependencies).
- **Cross-platform detach/signalling** is delegated to circus's arbiter; the
  acceptance criteria verify it behaves equivalently on macOS and Linux.

## Acceptance Criteria

- [ ] Given a clean checkout with dependencies already installed and the server
  debug binary built, when I run `mise run dev`, then both processes start in
  the background, the command returns within **10 s** of `server-info.json`
  being written, and the frontend successfully proxies `/api` to the server.
- [ ] Given a checkout where the server debug binary is **not** built and node
  deps are **not** installed, when I run `mise run dev`, then the
  `build:server:dev` and `deps:install:node` steps run automatically first and
  the stack still starts successfully.
- [ ] Given `mise run dev` has returned, when I inspect `.accelerator/tmp/dev/`,
  then two distinct, non-empty log files (`server.log` and `frontend.log`)
  exist, each containing its own process's startup output.
- [ ] Given a healthy `dev` session is already running (arbiter reachable and
  both watchers running, per Requirement 1), when I run `mise run dev` again,
  then it detects the existing session, reuses it (starts no duplicate
  processes), and reports success.
- [ ] Given `mise run dev` is running, when I run `mise run dev:stop`, then both
  processes and all their child processes terminate, the `circus` arbiter exits,
  and no process in the recorded server/frontend process groups survives
  (verified by listing the children of the recorded server/frontend PIDs, not a
  global process grep).
- [ ] Given `mise run dev` is running, when I run `mise run dev:restart`, then
  both processes are torn down and freshly started, and after restart the
  frontend successfully proxies `/api` to the (possibly new) server port (no
  port-0 fallback).
- [ ] Given the server takes time to boot, when `dev` starts the frontend, then
  the frontend launches only after `server-info.json` exists (no port-0
  fallback); and if the server does not write it within **30 s**, `dev` exits
  non-zero with a clear error message naming the missing file and the server log
  path.
- [ ] Given a stale arbiter endpoint or a dead/recycled supervisor daemon (e.g.
  a leftover socket or pidfile after a crash), when I run `dev:stop` or
  `dev:status`, then it is treated as "not running" and cleaned up rather than
  acting on an unrelated process.
- [ ] Given the tasks run on both macOS and Linux, then on each platform
  `dev:stop` leaves no orphaned node/server processes (verified via the
  process-group check above), `dev:status` reports the same set of fields with
  the same exit codes, and child-process teardown completes within the **2 s
  SIGTERM grace period** before SIGKILL escalation — i.e. observable behaviour
  is equivalent across the two platforms.
- [ ] Given `dev:status`, then it reports each process's state (plus the
  frontend URL and resolved API port when running) and exits **0** when both are
  running, **3** when one is running, and **4** when neither is running — the
  same codes on macOS and Linux.
- [ ] Given the `circus` dependency is added, then an ADR recording the choice
  exists (proposed or accepted) before the item is considered done.

## Open Questions

- Is a `dev:logs` tail helper wanted as part of this item, or deferred to a
  follow-up?

(Resolved during review: the supervision approach is `circus` — see the
Decision subsection; and `dev` **reuses** an already-running healthy session
rather than failing loudly — see Requirement 1.)

## Dependencies

- Blocked by: none.
- Blocks: none.
- Produces: an ADR recording the choice to add `circus` as a runtime dependency
  (tracked as an acceptance criterion, not a separate blocking work item).
- New dependency: adds the `circus` Python package to `pyproject.toml`. circus
  supports macOS and Linux (Requirement 10); it is not Windows-compatible, which
  is acceptable as the developer/CI targets are macOS and Linux. An ADR should
  record adding this dependency.
- Task prerequisites: depends on the existing `build:server:dev` and
  `deps:install:node` tasks running before launch (Requirement 8).
- Related: work item 0100 (Configurable Visualiser Auto-Shutdown) — adjacent
  visualiser server-lifecycle territory; the dev path uses `--owner-pid 0` to
  disable auto-shutdown. **No ordering dependency**: 0100 is independent, and
  this task only needs `--owner-pid 0` to keep disabling owner-based
  auto-shutdown. `dev:stop` tears the server down via circus regardless of
  0100's idle-timeout configuration, so the two remain consistent on stop
  semantics without sequencing.

## Assumptions

- The existing per-process `dev:server` / `dev:frontend` tasks **remain** — the
  unified `dev` task orchestrates the same underlying launch logic (or composes
  the existing tasks) rather than removing them. If this is wrong, scope
  expands to deprecating/removing them.
- "Manage" means start / stop / restart / status — **not** a long-running
  supervisor that auto-restarts processes on crash or on file change. (Vite
  already provides HMR; server hot-reload is out of scope.)
- Log files belong under the existing `.accelerator/tmp/` convention already
  used for `dev-server`.

## Technical Notes

- mise → invoke wiring: `mise.toml` `[tasks."dev:server"]` runs
  `invoke dev.server`; `[tasks."dev:frontend"]` runs `invoke dev.frontend`.
  invoke converts underscores to hyphens in task names. New tasks would add
  `[tasks."dev"]`, `[tasks."dev:stop"]`, `[tasks."dev:restart"]`,
  `[tasks."dev:status"]` delegating to corresponding `tasks/dev.py` functions.
- Current `tasks/dev.py` launches both processes with
  `context.run(..., pty=True)` (foreground). Backgrounding cannot rely on
  invoke's `run(..., disown=True)` (it gives no stream/fd control) — the new
  tasks start a `circus` arbiter (two watchers: server and frontend) and control
  it via `CircusClient`.
- circus wiring: configure one watcher per process with a `FileStream` stdout
  stream pointed at `.accelerator/tmp/dev/server.log` / `frontend.log`
  (Requirement 5); the arbiter owns each watcher's process group, so its stop
  flow escalates SIGTERM → grace → SIGKILL across the whole tree. `dev:stop`
  quits the arbiter; `dev:restart` and `dev:status` issue the corresponding
  `CircusClient` commands.
- Readiness gate: between starting the server watcher and the frontend watcher,
  poll for `.accelerator/tmp/dev-server/server-info.json` (the file the server
  writes its port into) every 100 ms, timing out after 30 s; this is the same
  signal `launch-server.sh` polls for (up to ~5s in the production path).
- Reuse / identity / stale-state: on `dev`, query the arbiter (or its endpoint)
  to detect an already-running healthy session and reuse it; treat an
  unreachable endpoint with a leftover socket/pidfile as a dead daemon and clean
  it up. `launcher-helpers.sh` `start_time_of` / `start_time_matches` (PID
  start-time, forced `LANG=C`) and `stop_server_stop` (identity check → SIGTERM
  → 2s grace → SIGKILL) remain the reference for the grace-period and
  recycled-PID behaviours.
- Key files: `mise.toml`, `tasks/dev.py`, `tasks/build.py`, `tasks/__init__.py`,
  `tasks/shared/paths.py`, `pyproject.toml`,
  `skills/visualisation/visualise/frontend/vite.config.ts`,
  `skills/visualisation/visualise/scripts/launch-server.sh`,
  `skills/visualisation/visualise/scripts/launcher-helpers.sh`.

## Drafting Notes

- The supervision approach was resolved during review 1 to `circus` (a library
  with native per-process log files and a Python control client), and the
  already-running behaviour was resolved to reuse rather than fail-loud. See the
  Decision subsection and Requirement 1.
- Inferred the audience is repo developers running the visualiser locally, and
  that "dev mode" means the existing debug-binary server plus Vite-with-proxy
  setup.
- Added `dev:status` to the requested `dev` / `dev:stop` / `dev:restart` set as
  the natural lifecycle complement; flagged a `dev:logs` helper as an open
  question rather than assuming it in scope.
- Linked work item 0100 as related because both touch visualiser server
  lifecycle; a reviewer may judge the link too weak to keep.

## References

- Related: work item 0100 (`meta/work/0100-configurable-visualiser-auto-shutdown.md`)
- Codebase: `tasks/dev.py`, `mise.toml`,
  `skills/visualisation/visualise/scripts/launch-server.sh`,
  `skills/visualisation/visualise/scripts/launcher-helpers.sh`,
  `skills/visualisation/visualise/frontend/vite.config.ts`
- Process-manager research (2024–2026): honcho v2.0.0 (foreground multiplexer,
  no per-file logs), circus 0.19.0 (daemon + FileStream per-process logs +
  Python `CircusClient`), supervisor 4.3.0 (XML-RPC control, Unix-only),
  `subprocess` + `os.killpg` process-group teardown; mise has parallel-run and
  `watch` (watchexec) but no background supervision. **circus 0.19.0 selected**
  — see the Decision subsection.
