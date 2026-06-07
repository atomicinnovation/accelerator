---
type: plan
id: "2026-06-06-0101-unified-dev-task-for-visualiser"
title: "Unified Managed dev Task for Visualiser Server and Frontend Implementation Plan"
date: "2026-06-06T18:03:58+00:00"
author: Toby Clemson
producer: create-plan
status: accepted
work_item_id: "work-item:0101"
parent: "work-item:0101"
derived_from: ["codebase-research:2026-06-06-0101-unified-dev-task-for-visualiser"]
relates_to: ["work-item:0100"]
tags: ["dev-tooling", "mise", "invoke", "visualiser", "circus", "process-supervision"]
revision: "c893bc4638cdc31c029312742292e5abd6118ed9"
repository: "miscellaneous"
last_updated: "2026-06-06T22:48:59+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Unified Managed dev Task for Visualiser Server and Frontend Implementation Plan

## Overview

Add a single `mise run dev` task that starts and supervises both the visualiser
API server and the Vite frontend together as detached background processes under
a `circus` arbiter, plus `dev:stop` / `dev:restart` / `dev:status` lifecycle
tasks. Each process writes to its own log file; a developer brings the whole dev
stack up and down with one command instead of juggling two terminals. Re-running
`dev` while a healthy session is already up reuses it.

A late but load-bearing requirement: **multiple instances must coexist on one
machine** (one per jujutsu workspace). The circus control/pubsub endpoints are
therefore **per-workspace `ipc://` Unix-domain sockets** (not TCP ports), so two
workspaces' arbiters can never answer on the same endpoint — cross-workspace
isolation is *structural*, not probabilistic, and there is no endpoint port to
collide. Only the **Vite listen port** is a TCP port allocated free at `dev`
invocation time (and `--strictPort` makes a collision fail loud). The socket
paths, the frontend port + URL, the arbiter PID and its start-time, and the
server/frontend watcher PIDs are recorded in a **per-workspace state file** under
`.accelerator/tmp/dev/`. Because each jj workspace has its own working copy (and
hence its own `.accelerator/tmp/`), discovery and state are naturally isolated
with no shared global state.

**One supervised stack per workspace** is an intentional design limit: a single
`dev.json`/`dev.lock`/`circusd.pid`/`circus.ini` per workspace means a second
concurrent stack in the *same* workspace is impossible by construction (handled
as reuse / fail-fast, not a duplicate). Multiple stacks per workspace are out of
scope and would require keyed state files.

## Current State Analysis

The current dev workflow is a two-terminal, foreground split with no PID
tracking, no backgrounding, and no programmatic startup ordering.

- **`tasks/dev.py:14-39` `server()`** — `mkdir` the tmp dir, render config via
  `write-visualiser-config.sh --owner-pid 0` (captured to `config.json`), then
  run `{_SERVER_BIN} --config {_CONFIG_PATH}` with `pty=True` (**foreground,
  blocking**). The server binds a random loopback port and writes
  `server-info.json` + `server.pid` itself.
- **`tasks/dev.py:42-54` `frontend()`** — `npm --prefix {FRONTEND} run dev` with
  `env={"VISUALISER_INFO_PATH": …}`, `pty=True` (**foreground, blocking**). Sets
  no `VISUALISER_API_PORT`.
- **`frontend/vite.config.ts:26-51` `resolveApiPort()`** — reads `info.port` from
  `VISUALISER_INFO_PATH` **once at config-eval time**; absent/portless ⇒ proxy to
  `http://127.0.0.1:0` (deliberate fail-loud). So the server must have written
  `server-info.json` **before** Vite loads its config. Today only operator
  convention enforces this.
- **`mise.toml:38-46`** — `dev:server`/`dev:frontend` use `depends` (which are
  **completion barriers**) for `build:server:dev` / `deps:install:node`. A
  unified `dev` therefore cannot `depends` on the two foreground tasks; the
  orchestration must live in Python.
- **`tasks/__init__.py:33`** — `dev` is auto-exposed via
  `Collection.from_module(dev)`; a bare `invoke dev` default requires switching to
  the manual `Collection(...)` + `add_task(..., default=True)` pattern that
  `release` already uses (`tasks/__init__.py:20-30`).
- **`pyproject.toml:10-23`** — PEP 735 dependency groups managed by `uv`
  (`requires-python >=3.14`, `[tool.uv] prerelease = "allow"`). `circus` is
  **not present**; only `invoke` + build/test tooling.
- **Prior art** — `scripts/launch-server.sh` + `scripts/launcher-helpers.sh`
  implement reuse-before-lock, `(pid, start_time)` identity (with macOS `LANG=C`
  forcing), SIGTERM → 2 s grace → SIGKILL, and stale-state cleanup. The dev path
  does **not** use any of it today.

### circus behaviour verified against live docs/source (corrects the work item)

Web research against `circus.readthedocs.io` and `circus-tent/circus` source
surfaced three facts that change the design and **correct the work item's
mental model**:

1. **circus does NOT `killpg` the process group.** It `os.setsid()`s each
   process but signals **by PID**, only reaching descendants when the watcher
   sets **`stop_children = true`** (it then walks `psutil` children recursively).
   Without it, Vite's `node`/`esbuild` workers orphan — defeating Requirement 9.
   ⇒ `stop_children = true` is **mandatory** on both watchers.
2. **`graceful_timeout` defaults to 30 s**, but our contract requires a **2 s**
   SIGTERM grace. ⇒ set `graceful_timeout = 2` per watcher.
3. **`get_arbiter(...).start()` blocks the calling process** (owns the Tornado
   IOLoop) — it is *not* a detached daemon. We launch plain **`circusd --pidfile
   <file> <generated.ini>`** (no `--daemon`) via `Popen(start_new_session=True)`
   so the child detaches into its own session **and** the Popen PID is the real
   arbiter (whereas `--daemon` double-forks, leaving the handle pointing at a
   short-lived intermediate). The arbiter is then driven via
   `circus.client.CircusClient`.

Supporting facts: watcher states are `"active"` / `"stopped"`; the `status`
command returns `{"statuses": {name: state}}` for all watchers or `{"status":
state}` for one; circus supports `ipc://` endpoints as well as `tcp://`, and an
unreachable endpoint (no listener / absent socket) yields a *timeout* rather than
`circus.exc.CallError` (ZMQ connects lazily), so use a short client timeout for
liveness probes and treat the timeout as "stale/down"; stream classes in a
watcher passed over the wire must be dotted strings, but in an **INI file** the
natural `stdout_stream.class = FileStream` works; the arbiter leaves the pidfile
**and the `ipc://` socket files** on disk on crash (neither auto-removed), so
both are part of stale cleanup; `psutil` ships transitively with circus, giving
cross-platform `Process.create_time()` that sidesteps the macOS `LANG=C`
start-time pitfall.

> **`ipc://` socket-path length (budgeted, not assumed)**: Unix-domain socket
> paths are length-limited (~104 bytes on macOS `sun_path`, ~108 on Linux), and
> macOS `$TMPDIR` is itself a deep `/var/folders/…/T/` path (~49 chars), so the
> budget is tight. `ipc_socket_paths(workspace_root)` therefore:
> - resolves the temp base via **`tempfile.gettempdir()`** (honours `$TMPDIR`,
>   falls back to `/tmp` when unset — common on Linux cron/systemd/containers/CI),
>   never reading `$TMPDIR` directly;
> - creates a **per-workspace subdirectory** `acc-dev-<hash>/` where `<hash>` is a
>   **fixed 12-hex-char** digest of the canonicalised workspace root (collision-
>   negligible, length-bounded), holding `e.sock` / `p.sock` (short basenames);
> - computes the worst-case absolute path against the macOS ~104 limit (minus any
>   ZMQ suffix) and **hard-errors with a clear message if it would exceed** —
>   never silently truncating (which would break per-workspace uniqueness);
> - uses **filesystem-path** sockets only (no Linux-only abstract `@`/NUL
>   namespace), so the form is identical on both OSes.
>
> The resolved paths are recorded in dev-state *and* are deterministically
> recomputable, so discovery survives a lost state file. The per-workspace
> subdir is removed wholesale by `dev:stop` (along with the sockets); a unit test
> asserts the path is deterministic, distinct per workspace root, within the
> `sun_path` limit, a filesystem path (no leading `@`), and resolves correctly
> when `$TMPDIR` is unset.

These facts are docs/source-derived. circus is developed primarily on Linux, and
the two load-bearing cross-platform behaviours — the self-detached `circusd`
(`Popen(start_new_session=True)`) surviving the invoking shell under launchd, and
`stop_children` walking the `psutil` child tree to reach Vite's `node`/`esbuild`
workers — are **proven on macOS by the Phase 5 integration suite running on a
`macos-latest` CI leg** (plus a required interactive-shell manual check), not
assumed from documentation. The `psutil.create_time()` value's resolution also
differs by OS, so the identity-gate tolerance is validated per-platform there
(see Phase 2 §7).

### Desired End State

- `mise run dev` builds prerequisites, starts both processes detached under a
  circus arbiter, enforces server→frontend ordering, returns within 10 s of
  `server-info.json` being written, and reuses a healthy session if one exists.
- `mise run dev:stop` tears down both processes and all children and stops the
  arbiter, leaving no orphans.
- `mise run dev:restart` = stop then start.
- `mise run dev:status` reports each process's state, the frontend URL, and the
  resolved API port, exiting 0 (both) / 3 (one) / 4 (neither).
- Two instances run concurrently in two jj workspaces with no port or state
  collisions.
- An ADR records the `circus` dependency choice.

**Verification**: the Acceptance Criteria in `meta/work/0101-…md` all pass; the
new unit + integration suites are green on macOS and Linux.

### Key Discoveries

- `tasks/dev.py:7-11` — dev path constants are module-private; the server's tmp
  dir is `.accelerator/tmp/dev-server/` and it writes `server-info.json` there.
- `frontend/package.json:10` — the dev script is exactly `"vite"`, so
  `npm --prefix {FRONTEND} run dev -- --port <p> --strictPort --host 127.0.0.1`
  passes the listen port straight through to Vite. **No `vite.config.ts` change
  is needed** to make the frontend port deterministic.
- `tasks/test/integration.py:40-43` — integration tasks are thin `invoke`
  wrappers; `binary_acquisition` shells out to one script. A new
  `test:integration:dev` fits this pattern (we use a **pytest** harness with
  Python fake processes to stay cross-platform and avoid shellcheck/shfmt load).
- `mise.toml:86-89` — `test:unit:tasks` runs an **explicit file list**; a new
  `tests/tasks/test_dev.py` must be appended there or it won't run in CI.
- `tasks/shared/files.py:4-11` — `atomic_write_text` exists and is the right tool
  for writing the dev-state file atomically.
- Next ADR number is **ADR-0041** (`meta/decisions/` ends at ADR-0040).

## What We're NOT Doing

- **No `dev:logs` helper** (deferred to a follow-up per the work item's open
  question and the user's decision). Logs remain discoverable under
  `.accelerator/tmp/dev/`.
- **No crash auto-restart / file-watch supervision.** "Manage" = start / stop /
  restart / status only. Vite already provides HMR; server hot-reload is out of
  scope. (Watchers are configured so circus does **not** resurrect them —
  `respawn = false` — keeping behaviour to the agreed contract.)
- **Not removing `dev:server` / `dev:frontend`.** The two-terminal tasks remain;
  the unified path shares their underlying launch logic via extracted helpers.
- **No `vite.config.ts` change.** The frontend port is supplied via CLI args and
  recorded in dev-state; the existing API-port resolution is untouched.
- **No Windows support** (circus is POSIX-only; macOS + Linux are the targets).

## Implementation Approach

The arbiter is launched against a **generated INI** (so `FileStream` log config
and `stop_children`/`graceful_timeout` live in the file rather than being
marshalled over ZMQ). The INI declares two watchers: `server` (autostart) and
`frontend` (`autostart = false`). `dev` launches the daemon, polls for
`server-info.json`, then issues `start frontend` over a `CircusClient` — this is
how the server→frontend ordering gate is enforced.

> **Detach mechanism — self-detach, not `circusd --daemon`**: `circusd --daemon`
> double-forks, so the PID of the process we spawn is a short-lived intermediate
> that exits immediately after forking the real arbiter — a retained subprocess
> handle would point at the wrong process (and be useless for reaping on a
> startup failure). Instead, launch plain `circusd <generated.ini>` (no
> `--daemon`) via `subprocess.Popen(..., start_new_session=True)`: the child is
> detached into its own session (survives the invoking shell) **and** the Popen
> PID *is* the real arbiter, so the handle is a reliable reap target on the
> failure path. circusd still writes its `--pidfile`; the pidfile is the
> cross-command identity record, the handle is the launch-time reap target.

All cross-command discovery flows through a **per-workspace dev-state file**
(`.accelerator/tmp/dev/dev.json`) holding the resolved `ipc://` endpoint/pubsub
socket paths, the frontend port + URL, the arbiter PID + `psutil` start-time, the
**server and frontend watcher PIDs** (recorded *incrementally* — the server PID as
soon as the server watcher is active, the frontend PID as soon as it is — so
teardown can reach reparented orphans even after the arbiter dies, with no
launch-window gap), and the pidfile/INI paths. The `ipc://` socket paths are also
**deterministically recomputable** from the workspace root (the pure
`ipc_socket_paths` helper), so `dev:stop`/`dev:status` can recompute and probe
them even when the state file is missing/corrupt — dev-state is a discovery
*cache*, not the sole source of truth, and a truncated state file can never
orphan a live stack. Liveness is decided by probing the recorded (or recomputed)
endpoint with a short timeout; the recorded `(pid, start_time)` identity is used
only as a safety gate before any direct kill, never to drive the normal decision.

**Concurrency, freshness, and partial-launch robustness** (the design carries
several inherent races; the plan addresses each explicitly rather than assuming
them away):

- **Cross-workspace isolation is structural, not probabilistic**: because the
  control/pubsub endpoints are per-workspace `ipc://` sockets (Overview), a probe
  can only ever reach *this* workspace's arbiter or time out — there is no shared
  port space, so one workspace's `dev:stop` can never reach another's arbiter.
  This removes the cross-workspace endpoint-collision risk at the root rather than
  guarding against it after the fact. As belt-and-braces, the reuse gate still
  confirms the recorded `(arbiter_pid, start_time)` is alive before reusing.
- **Intra-workspace serialisation**: the reuse-gate → allocate → state-write →
  daemon-launch sequence is wrapped in a **non-blocking per-workspace lock**
  (Python `fcntl.flock` on a held fd over `.accelerator/tmp/dev/dev.lock` — `fcntl`
  is available on both macOS and Linux; a `mkdir`-with-recorded-owner fallback
  only for the rare filesystem where `flock` is unavailable). A second concurrent
  `dev` in the same workspace that cannot take the lock re-probes and reuses a now-
  healthy session, or fails fast, rather than racing into a duplicate arbiter.
- **Frontend-port TOCTOU**: only the Vite port is a TCP port; `free_port()`
  binds-to-0/reads/releases and Vite rebinds later, so it can be taken in the
  window. Vite's `--strictPort` makes that fail loud (surfaced in `frontend.log`),
  and because the endpoints are now `ipc://`, the daemon itself has no port to
  collide on — so `circusd` failing to start is a genuine error (bad INI, missing
  binary), handled as a bounded pidfile-poll-then-clear-error and a **reap of the
  launch handle** (the real arbiter PID, since we self-detach rather than
  `--daemon`), **not** a retry of a transient port race.
- **Recoverable partial launch**: a **provisional dev-state** (socket paths +
  frontend port/URL + pidfile + INI path; PIDs null) is written **before**
  `circusd` is launched, so even a daemon that starts but never writes its pidfile
  is discoverable for cleanup (and the launch handle is the real arbiter PID). The
  arbiter PID + start-time are filled once the pidfile appears and the PID is
  confirmed live; **each watcher PID is recorded the instant that watcher is
  active** (server PID right after the readiness gate, frontend PID right after
  `start frontend`), closing the window where an arbiter death between launch and
  recording would leave reparented children unreachable by identity.
- **Readiness freshness**: the readiness gate keys on `server-info.json`
  *existence*, but that file is server-owned and persists across runs, so any
  stale copy is **deleted before launch** (see Phase 3) — the gate must observe
  only the new server's freshly-written file, never a leftover.

Work is split so every phase leaves the repo green and is independently
mergeable, and each behavioural phase is written test-first.

---

## Phase 1: circus dependency + ADR-0041

### Overview

Add the runtime dependency and record the architectural decision. Independently
mergeable: the dep is declared and locked; the ADR is proposed.

### Changes Required

#### 1. Dependency declaration

**File**: `pyproject.toml`
**Changes**: add `circus` and `psutil` to the `build` dependency group (the
group that already backs the invoke tasks). `psutil` is declared explicitly
rather than relied upon transitively, since the identity check imports it
directly.

```toml
build = [
    "invoke>=2.2.1",
    "circus>=0.19.0",
    "psutil>=6",
    "keepachangelog>=2.0.0",
    # …existing…
]
```

**File**: `uv.lock`
**Changes**: regenerate via `uv lock` so CI's `--frozen` installs resolve.

> **Wheel-availability check (cross-platform)**: `circus` pulls in `pyzmq` and
> `tornado` (native components) and `psutil` is a C-extension; the project pins
> `requires-python >=3.14` with `[tool.uv] prerelease = "allow"`. Before locking,
> confirm these resolve to **installable artifacts for CPython 3.14 on both
> arm64 macOS and x86_64 Linux** (prebuilt wheels, or buildable sdists). If any
> falls back to a source build, record the required system toolchain/libraries
> (e.g. a C compiler, libzmq) as a prerequisite, or pin to a version that ships
> 3.14 wheels — otherwise a fresh CI runner or developer machine can fail
> `uv sync` on one platform but not the other.

#### 2. ADR

**File**: `meta/decisions/ADR-0041-circus-dev-process-supervision.md`
**Changes**: new ADR (status `proposed`) recording the choice of `circus` for dev
process supervision. Capture: the requirement (per-process logs, reuse detection,
group teardown, macOS+Linux); options considered (honcho — single combined
stream; supervisor — Unix-only XML-RPC + config-file model; raw
`subprocess`+`os.killpg`); the decision (circus: native `FileStream` per-process
logs, long-lived arbiter + Python `CircusClient`, **self-detach via plain
`circusd` (no `--daemon`) launched with `Popen(start_new_session=True)`** — `--daemon`
was **rejected** because its double-fork leaves the spawned PID a short-lived
intermediate, useless as a reap target; self-detach makes the Popen PID the real
arbiter); and the consequences (new runtime dep; not Windows-compatible;
**`stop_children` + `graceful_timeout` must be set explicitly** because circus
signals by PID, not by process group). Follow `ADR-0030`'s template and
`ADR-0029`'s sequential-id rule.

Also record, in the consequences, that the dev orchestration couples directly to
circus's API surface (INI dialect, `CircusClient` verbs, `circus.exc.CallError`
semantics) with **no abstraction boundary** — replacing circus later (with
`supervisor`, `honcho`, or raw `subprocess`+`os.killpg`) is a rewrite of the
supervision layer, not an adapter swap. This is acceptable lock-in for a dev-only
tool; capturing it makes the cost a conscious decision. To limit the blast
radius, the Phase 2 pure helpers (`render_circus_ini`, `evaluate_health`,
`status_exit_code`, `wait_for_file`, `pid_identity_matches`) stay circus-agnostic
wherever feasible.

### Success Criteria

#### Automated Verification

- [x] Lockfile resolves frozen: `uv sync --only-group build --frozen`
- [x] `circus` and `psutil` importable: `uv run python -c "import circus, psutil"`
- [x] `circus`/`pyzmq`/`tornado`/`psutil` resolve to installable artifacts for
      CPython 3.14 on **both** `ubuntu-latest` (x86_64) and `macos-latest`
      (arm64) — verified by the CI matrix added in Phase 5 (a clean
      `uv sync --only-group build --frozen` succeeds on both legs). Lockfile
      audit confirms prebuilt wheels exist for all three targets (incl. Linux
      aarch64); no source-build fallback on any platform.
- [x] ADR file exists and parses: `mise run test:unit:templates` (validates ADR
      frontmatter/schema)
- [x] Repo checks pass: `mise run check`

#### Manual Verification

- [ ] ADR reads coherently and the options/consequences match the as-built
      design (revisit at end of Phase 5 before flipping to `accepted`).
- [x] If any dependency falls back to a source build on either platform, the
      required system toolchain/libraries are documented (or a wheel-shipping
      version is pinned). — N/A: lockfile audit shows prebuilt wheels for all
      three targets (macOS arm64, Linux x86_64, Linux aarch64); documented in
      ADR-0041 "Wheel availability".

---

## Phase 2: Pure supervisor helpers (test-first)

### Overview

Introduce a `tasks/dev_supervisor.py` module of small, side-effect-isolated
functions, each unit-tested before implementation. No task wiring yet — this
phase merges as tested, currently-unused utilities (imported by Phase 3).

> Naming/placement note: a flat module `tasks/dev_supervisor.py` is used rather
> than a package, matching the existing flat `tasks/*.py` layout and the
> no-underscore-prefixed-module convention. To keep the functional-core/imperative-
> shell boundary a **file boundary** (not just a comment), the genuinely
> circus-agnostic *pure* helpers — `free_port`, `ipc_socket_paths`, `wait_for_file`,
> `pid_identity_matches`, `evaluate_health`, `status_exit_code` — **live under
> `tasks/shared/`** (alongside `files.py`/`paths.py`, e.g. a new `dev_helpers.py`),
> leaving `tasks/dev_supervisor.py` holding only the supervision concern (the
> `Supervisor` circus adapter, `DevState` I/O, locking, INI rendering, and the
> orchestrators). This prevents the pure core and the circus-coupled code drifting
> back together and keeps ADR-0041's blast-radius limit real.

### Changes Required

#### 1. Free-port allocation

**File**: `tasks/dev_supervisor.py`
**Changes**:

```python
def free_port() -> int:
    """Return a currently-free loopback TCP port (bind to 0, read, release)."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]
```

Only the **Vite frontend port** is a TCP port now (the circus endpoints are
`ipc://` sockets — see Overview), so a single `free_port()` is all that's needed.

> **Known race (documented, mitigated downstream)**: `free_port` releases the
> socket before Vite rebinds, so the port can be claimed in the window. This is
> *not* fixed here — Vite's `--strictPort` makes a collision fail loud (surfaced
> in `frontend.log`); the daemon itself has no port to collide on. There is no
> longer any endpoint/pubsub port to allocate.

Tests assert: `free_port()` returns a bindable loopback port.

A pure, **deterministic** `ipc_socket_paths(workspace_root) -> (endpoint, pubsub)`
helper derives the two per-workspace socket paths under
`tempfile.gettempdir()/acc-dev-<12-hex-hash>/{e,p}.sock` (see the budgeted
socket-path-length note). Being deterministic, it lets `dev:stop`/`dev:status`
recompute the paths when dev-state is missing/corrupt. Unit tests assert the paths
are distinct, deterministic for a given workspace root, **within the `sun_path`
limit** (with the worst-case computed, hard-error if exceeded), a **filesystem
path** (no leading `@`/NUL abstract namespace), and **resolved correctly when
`$TMPDIR` is unset** (via `tempfile.gettempdir()` falling back to `/tmp`).

#### 1b. Non-blocking per-workspace lock

**File**: `tasks/dev_supervisor.py`
**Changes**: `workspace_lock(path)` — a context manager taking a **non-blocking**
`fcntl.flock(fd, LOCK_EX | LOCK_NB)` on a held fd over
`.accelerator/tmp/dev/dev.lock`. `fcntl` is available on both macOS and Linux, so
this is the primary (and normally only) path; the `flock` fd is released
automatically if the holder dies. A `mkdir`-based fallback fires only on the rare
filesystem where `fcntl.flock` raises `OSError`/`AttributeError`, and that
fallback records an **owner `(pid, start_time)`** so a stale lock dir left by a
SIGKILLed holder can be reclaimed (mirroring the atomic-jsonl PID-owner reclaim)
rather than wedging the workspace. To avoid two contenders both observing the same
dead owner and both reclaiming, the fallback reclaim is **atomic**: stage a
uniquely-named owner dir then `os.rename` it into place (rename is atomic on
POSIX), and re-stat after the rename, backing off if the owner changed. Yields
whether the lock was acquired; releases on exit. Unit tests: acquire when free;
fail-fast (do not block) when held; release on context exit; **fallback-branch
reclaim of a stale (dead-owner) lock dir**; **two-contender dead-owner race
resolves to a single winner**. `dev:stop`/`dev:status` do **not** require the lock
(so a wedged lock can always be cleared by tearing the stack down).

#### 2. circus INI generation (pure)

**File**: `tasks/dev_supervisor.py`
**Changes**: `render_circus_ini(spec: ArbiterSpec) -> str` producing an INI with a
`[circus]` section (endpoint/pubsub from the spec, `check_delay`, no stats) and
two `[watcher:server]` / `[watcher:frontend]` sections. Frozen invariants the
unit tests assert:

```ini
[circus]
endpoint = ipc://{endpoint_socket}
pubsub_endpoint = ipc://{pubsub_socket}
check_delay = 1
pidfile = {pidfile}

[watcher:server]
cmd = {server_bin} --config {config_path}
numprocesses = 1
autostart = true
respawn = false
stop_children = true
graceful_timeout = 2
stdout_stream.class = FileStream
stdout_stream.filename = {dev_dir}/server.log
stderr_stream.class = FileStream
stderr_stream.filename = {dev_dir}/server.log

[watcher:frontend]
cmd = npm --prefix {frontend} run dev -- --port {fe_port} --strictPort --host 127.0.0.1
numprocesses = 1
autostart = false
respawn = false
stop_children = true
graceful_timeout = 2
stdout_stream.class = FileStream
stdout_stream.filename = {dev_dir}/frontend.log
stderr_stream.class = FileStream
stderr_stream.filename = {dev_dir}/frontend.log

[env:frontend]
VISUALISER_INFO_PATH = {server_info_path}
```

Tests assert: both watchers present; `stop_children = true` and
`graceful_timeout = 2` on **both**; `autostart = false` on frontend only;
`respawn = false` on both; `ipc://` endpoint/pubsub socket paths and the pidfile
interpolated; frontend `cmd` carries `--port … --strictPort`; the server `cmd`
**omits `--log-file`** (see below); `VISUALISER_INFO_PATH` set for frontend.

> **Single-writer per log file**: the server watcher's `FileStream` captures the
> server's stdout/stderr to `dev/server.log`. **Correction vs the legacy path**:
> today's `dev.server` *does* pass `--log-file {dev-server}/server.log`
> (`tasks/dev.py:34`) — so the shared config helper must **not** carry that flag
> into the unified path; the unified server is launched **without** `--log-file`
> so circus's captured stdout is the sole writer of `dev/server.log` (no
> dual-writer, and no name clash with the legacy `dev-server/server.log` used by
> the two-terminal flow). A unit assertion confirms the unified server `cmd`/config
> omits `--log-file`. State this in the directory-ownership comment (Phase 3 §1).

> **Toolchain on PATH in the detached daemon**: the frontend watcher shells out
> to `npm`/`node`, which on developer machines are often provided by a version
> manager (mise/asdf/nvm) via shims on the interactive shell's PATH. A detached
> `circusd` subprocess can run with a reduced environment where those shims are
> absent. The launch **must** make the daemon environment an explicit part of the
> launch contract: pass the resolved PATH (and `working_dir`) into the `circusd`
> subprocess env, **and resolve `npm`/`node` to absolute paths at launch and render
> those into the watcher `cmd`** (so the long-lived daemon is immune to later PATH
> drift / version-manager rehashes — a watcher started today keeps working even if
> the shim path changes), recording the resolved binary paths in dev-state for
> diagnosability. This is **exercised on the CI matrix**
> (Phase 5: a shim-`npm`-on-a-stripped-PATH integration case), not left to a
> manual check.

#### 3. dev-state read/write (atomic)

**File**: `tasks/dev_supervisor.py`
**Changes**: a typed `DevState` dataclass with explicit field types —
`endpoint: str`, `pubsub_endpoint: str` (full `ipc://<path>` strings, formatted
once at write time so callers never hand-build them), `frontend_port: int`,
`frontend_url: str`, `arbiter_pid: int | None`, `arbiter_start_time: float | None`
(both `None` in the provisional pre-launch state; filled once the pidfile PID is
confirmed live), `server_pid: int | None`, `server_start_time: float | None`,
`frontend_pid: int | None`, `frontend_start_time: float | None` (the two watcher
identities, `None` until both watchers are active — recorded so teardown can
reach reparented orphans by identity even after the arbiter dies), `pidfile: str`,
`ini_path: str`. `write_dev_state(path, state)` writes via `atomic_write_text` +
JSON; `read_dev_state(path) -> DevState | None` returns `None` if the file is
absent, unparseable, **or structurally present but schema-mismatched** (missing/
wrong-typed fields) — so every caller has a single simple contract. Round-trip +
missing-file + malformed-file + schema-mismatch + provisional-state (null PIDs)
unit tests.

#### 4. Health evaluation from a status response (pure)

**File**: `tasks/dev_supervisor.py`
**Changes**: `evaluate_health(statuses: dict[str, str]) -> Health` mapping a
circus `statuses` dict to an enum: `HEALTHY` (both `server` and `frontend`
`active`), `PARTIAL` (one active), `DOWN` (neither). Parametrised unit tests over
every combination including missing keys.

> **Startup-window caveat**: a circus watcher state alone cannot distinguish
> *frontend not yet started* (it is `autostart = false`, so `server=active,
> frontend=stopped` is the normal state during `up` between the readiness gate
> and `start frontend`) from *frontend died* (also ends `stopped`, since
> `respawn = false`). `evaluate_health` therefore stays a pure status→enum map;
> the **reuse/settle decision lives in Phase 3** and must not treat the
> legitimate startup window as a degraded `PARTIAL` (see Phase 3 §2 step 1).

#### 5. Status → exit-code mapping (pure)

**File**: `tasks/dev_supervisor.py`
**Changes**: `status_exit_code(health: Health) -> int` → `HEALTHY`→0,
`PARTIAL`→3, `DOWN`→4. Unit-tested.

#### 6. Readiness poll (injected clock)

**File**: `tasks/dev_supervisor.py`
**Changes**: `wait_for_file(path, *, timeout=30.0, interval=0.1, sleep=time.sleep,
now=time.monotonic) -> bool`. Define the loop precisely to pin the boundary
semantics: **check at t=0**, then while `now() < deadline` sleep
`min(interval, deadline - now())` and re-check, with a **final check** after the
loop. Reused by Phase 3 for the bounded pidfile poll. Tests inject fake
`sleep`/`now` to assert: returns `True` as soon as the path exists; returns
`False` after `timeout` with an **exact** poll count (not "≈300") derived from
the defined loop; never sleeps past the deadline (the last sleep is clamped).

#### 7. PID identity check (psutil)

**File**: `tasks/dev_supervisor.py`
**Changes**: `pid_identity_matches(pid: int, expected_start: float, *,
tolerance: float) -> bool` using `psutil.Process(pid).create_time()`, returning
`False` on `psutil.NoSuchProcess`. Tests mock `psutil.Process` to cover match,
drift within tolerance, mismatch (recycled PID), and dead PID.

> **Cross-platform tolerance (must be validated, not inherited)**: the shell
> launcher's ±1 s was tuned for a `/proc`-vs-`ps lstart` whole-second boundary
> that does **not** apply here — `create_time()` is captured and compared from
> the *same* psutil source. But psutil's resolution differs by OS (Linux derives
> from `/proc` jiffies at `CLK_TCK` granularity; macOS reads `kinfo_proc` at
> microsecond granularity), so a value correct on one platform can be too loose
> (admits a recycled PID) or too tight (rejects a genuine match) on the other.
> **Empirically validate the recycle/drift behaviour on both macOS and Linux**
> (via the Phase 5 real-PID integration assertion, run on both CI legs), pick
> `tolerance` accordingly — defaulting to an **exact / sub-second** match since
> capture and comparison share the psutil clock — and document why the chosen
> value is correct on each platform. `tolerance` is an explicit argument (no
> baked-in ±1 s default) so the validated value is passed deliberately.

### Success Criteria

#### Automated Verification

- [x] New unit suite passes: `uv run pytest tests/tasks/test_dev.py -v`, covering
      every helper above incl. `free_port` bindable, `ipc_socket_paths`
      (distinct / deterministic / within `sun_path` limit / filesystem-path /
      `$TMPDIR`-unset resolution), `workspace_lock` (acquire / fail-fast-when-held /
      release / atomic dead-owner reclaim), `DevState` schema-mismatch and
      provisional-state round-trips, the exact `wait_for_file` poll count, and
      `pid_identity_matches` drift/recycle/dead-PID
- [x] `test_dev.py` registered in `mise.toml` `test:unit:tasks` file list and the
      task is green: `mise run test:unit:tasks`. **All** fake-driven orchestration
      tests (Phase 3) also live in this same `tests/tasks/test_dev.py` so they are
      wired into CI via the explicit file list (a file in neither explicit list
      runs locally but is silently skipped in CI)
- [x] Repo checks pass: `mise run check`

#### Manual Verification

- [ ] Helper signatures read naturally for the Phase 3/4 call sites (review the
      orchestrator sketch below against them).

---

## Phase 3: `dev` (up) + `dev:stop` + `dev:restart`

### Overview

Wire the orchestrator using Phase 2 helpers, plus reuse, the readiness gate, and
robust teardown. After this phase `mise run dev` / `dev:stop` / `dev:restart`
work and a session is demoable. Test-first at the orchestration seam using a
fake `CircusClient` and fake subprocess launcher.

> **Injection seam (so the orchestration is unit-testable)**: invoke `@task`
> functions are called by the framework with only `(context)`, so they cannot
> receive test doubles directly. The orchestration logic therefore lives in
> plain, injectable functions in `tasks/dev_supervisor.py`. The injected
> collaborators **and the resolved dev paths** are grouped into one frozen
> `DevDeps` dataclass (`client_factory`, `launcher`, `killer`, `clock`,
> `config_renderer`, `state_path`, `lock_path`, `dev_dir`, `pidfile`, `ini_path`,
> `server_info_path`), default-constructed by the `@task` adapters to the real
> `CircusClient`/subprocess/`time`/`os.kill`/`tasks.dev` config-render wiring and
> the `_DEV_*` constants. `bring_up` thus never references `tasks/dev.py`
> module-level constants or its private config helper — the render-then-launch
> ordering and all directory side-effects (mkdir, stale-file deletion) are encoded
> in one place via injected values, so a new caller cannot skip them. Signatures
> stay tame: `bring_up(deps) -> UpResult`,
> `do_stop(deps) -> StopResult`, `do_status(deps) -> StatusResult`. Each result is
> its own small typed type — `UpResult` (started | reused | a typed failure with
> a message + artifact path), `StopResult` (`clean` | `refused` | `survivor`,
> with the offending PID), `StatusResult` (health + rendered fields + exit code).
> Each `@task def up/stop/restart/status(context)` is a **one-line adapter** that
> calls the function and maps the result to printed output / `invoke.Exit`. The
> Phase 3 fake-driven tests target these functions.
>
> `client_factory` yields an object satisfying a **minimal `Supervisor` protocol**
> (`status() -> dict[str, str]`, `start(name)`, `quit()`, raising a local
> `SupervisorUnreachable` rather than leaking `circus.exc.CallError`), implemented
> by a thin circus adapter. The orchestrators depend on the protocol, not on
> circus's wire verbs/exceptions — making the "limit the blast radius" intent from
> ADR-0041 a real (if small) boundary, and letting the test fake align to a
> contract rather than to circus internals.
>
> **bring_up is itself decomposed** (not a monolith): it reads as a short
> sequence over named injectable helpers — `reuse_or_teardown(state, deps) ->
> ReuseOutcome` (the lock-gated reuse/stale/degraded gate, returning reuse |
> proceed), `allocate_and_launch(deps) -> LaunchedArbiter` (provisional-state →
> launch → bounded pidfile poll → live-PID confirm), the readiness gate, and
> `start_frontend(deps)` (record watcher PIDs). Each is unit-testable on its own.
>
> **Shared teardown helper** (itself decomposed, mirroring `bring_up`): a single
> `teardown(state, deps) -> StopResult` in `dev_supervisor.py` reads as a short
> sequence over named sub-helpers — `snapshot_targets(state) -> TargetSet`
> (identity-establish + descendant snapshot), `kill_arbiter(targets, deps)`,
> `reap_descendants(targets, deps)`, `remove_artifacts(state, result)` — each
> unit-testable in isolation. It **returns the `clean`/`refused`/`survivor`
> result**, and all three call sites (the `up` reuse-gate's degraded branch, the
> `up` stale/readiness-failure path, and `do_stop`) delegate to it and **honour
> its result** (the reuse-gate branches fail-fast on `survivor`/`refused` rather
> than launching a competing arbiter), so the recycled-PID safety gate and the
> no-orphan reaping have exactly one implementation and one result contract. All
> grace/escalation waits use the **injected clock** (the same `sleep`/`now` seam as
> `wait_for_file`) so the unit tests assert escalation deterministically, leaving
> only the integration "it happened, generous margin" assertion on real time.

### Changes Required

#### 1. Extract shared server-config helper

**File**: `tasks/dev.py`
**Changes**: factor the `write-visualiser-config.sh --owner-pid 0` →
`config.json` logic out of `server()` into a config-rendering helper so both
`dev.server` (unchanged behaviour) and the new arbiter path render identical
config. To avoid a leading-underscore cross-module call, config rendering stays
owned by `tasks/dev.py` and is injected into `bring_up` as the
**`DevDeps.config_renderer`** collaborator (defaulted by the `up` adapter to
`tasks/dev.py`'s helper) — so `dev_supervisor` never reaches into
`dev._write_server_config`, and the render-then-launch ordering is encoded in the
orchestrator rather than relying on each call site to render first. New dev
constants (passed into `DevDeps` by the adapters): `_DEV_DIR = REPO_ROOT /
".accelerator/tmp/dev"`, `_DEV_STATE = _DEV_DIR / "dev.json"`, `_LOCK = _DEV_DIR /
"dev.lock"`, `_PIDFILE = _DEV_DIR / "circusd.pid"`, `_INI = _DEV_DIR /
"circus.ini"`. `server-info.json` stays in `dev-server/` (server-owned, path
unchanged).

> **Directory-ownership boundary** (state this as a module-level comment so it
> survives future refactors): `.accelerator/tmp/dev-server/` is **server-owned**
> (the server writes `config.json` + `server-info.json` + its own pidfile there);
> `.accelerator/tmp/dev/` is **orchestration-owned** (lock, dev-state, circus INI,
> pidfile, captured logs, and the `ipc://` sockets live under a short
> `$TMPDIR`-rooted hashed base recorded in dev-state). `server-info.json` is the
> **sole cross-directory contract** between them. The legacy `dev.server` runs the
> binary with `--log-file {dev-server}/server.log` (`tasks/dev.py:34`); the unified
> path deliberately **omits** `--log-file` so circus's `FileStream` capture is the
> single writer of `dev/server.log` (distinct from the legacy `dev-server/server.log`,
> no dual-writer, no same-path clash). A cleanup task that wipes one directory must
> not assume the other.

#### 2. `up` orchestrator (the `dev` task)

**File**: `tasks/dev.py` (thin `@task` adapter) + `tasks/dev_supervisor.py`
(`bring_up` + the named helpers from the Overview).
**Changes**: `@task(default=True) def up(context)` renders the server config,
calls `bring_up(deps)`, and prints its `UpResult` / raises `invoke.Exit`.
`bring_up` reads as a short sequence:

0. **Take the per-workspace lock** (`workspace_lock(_LOCK)`, non-blocking). If it
   cannot be acquired, another `dev` is mid-launch in this workspace: re-read
   dev-state and probe once — reuse if it is now HEALTHY; otherwise fail fast with
   a recovery-pointing message ("another `dev` is starting in this workspace;
   retry in a moment, or run `mise run dev:status` / `mise run dev:stop` if this
   persists"). The whole reuse→launch→state-write critical section runs under this
   lock.

1. **`reuse_or_teardown(state, deps)`** — `read_dev_state(_DEV_STATE)`; if present,
   probe `CircusClient(endpoint, timeout=2)` `status`. Because the endpoint is a
   **per-workspace `ipc://` socket**, a reachable probe can *only* be this
   workspace's arbiter (no shared port space), so there is no cross-workspace
   responder to mis-trust; as belt-and-braces the gate also confirms the recorded
   `(arbiter_pid, start_time)` is alive (`pid_identity_matches`) before reusing.
   Then:
   - reachable + identity-confirmed + `HEALTHY` → **reuse** (return a `reused`
     `UpResult`; the adapter prints the reuse block, step 6).
   - reachable + `PARTIAL`/`DOWN` → **degraded** → `teardown(state, deps)`; if it
     returns `survivor`/`refused`, **fail fast** (do not launch a competitor)
     surfacing the survivor; else continue. (A PARTIAL observed here is *either* a
     settled degraded session *or* a watcher that died asynchronously after a
     healthy start — `respawn = false` means neither recovers — and teardown is
     the correct action for both. The legitimate `server=active, frontend=stopped`
     startup window is *not* a source here: it occurs only while *this* call holds
     the lock, so it is never externally observed mid-flight.)
   - timeout/unreachable → stale → `teardown(state, deps)` (clears artifacts incl.
     the stale `ipc://` socket files), continue.

2. `_DEV_DIR.mkdir(parents=True, exist_ok=True)`; **delete any stale
   `_SERVER_INFO_PATH` and the server's own pidfile in `dev-server/`** (mirroring
   `launch-server.sh:41`) so the readiness gate in step 4 cannot pass against a
   leftover file.

3. **`allocate_and_launch(deps)`**:
   a. Derive the two per-workspace `ipc://` socket paths and `free_port()` for the
      Vite port; render + write the INI.
   b. **Write the provisional dev-state** (socket paths + frontend port/URL +
      pidfile + INI path; all PIDs null) **before** launching, so a daemon that
      starts but never writes its pidfile is still discoverable for cleanup.
   c. Launch plain `circusd --pidfile {_PIDFILE} {_INI}` (no `--daemon`) via
      `Popen(..., start_new_session=True)` (env carrying the resolved PATH/cwd and
      absolute `npm`/`node` — see Phase 2 §2). Self-detaching this way means the
      **Popen PID is the real arbiter** (a reliable reap target), unlike
      `--daemon`'s double-fork. Keep the handle.
   d. **Bounded pidfile poll** via `wait_for_file(_PIDFILE, timeout=…)`. Because
      the endpoints are `ipc://` (no port to collide on), a non-appearing pidfile
      is a **genuine startup failure** (bad INI, missing binary), not a transient
      port race: **kill the launch handle** (the real arbiter PID — and reap any of
      its descendants), then `invoke.Exit` naming `{_DEV_DIR}/server.log` and the
      missing pidfile (no retry — a retry cannot help a deterministic failure).
   e. When the pidfile appears, **parse defensively**: an empty/non-integer
      pidfile (created-but-not-yet-written) is "not ready", keep polling within
      the bound. Once a PID is read, confirm it is **live**
      (`psutil.Process(pid)`), capture `create_time()`, and update dev-state with
      the real arbiter PID + start-time. (The pidfile PID should equal the launch
      handle PID since we self-detach; a mismatch is a startup anomaly →
      kill-handle + exit as in (d).)

4. **Readiness gate**: `wait_for_file(_SERVER_INFO_PATH, timeout=30,
   interval=0.1)` — meaningful because step 2 cleared any stale file. On success,
   **immediately record the server watcher PID + start-time into dev-state** (the
   server watcher is now active) so an arbiter death from here on still leaves the
   server subtree reachable by identity. On failure → `teardown(state, deps)`,
   `invoke.Exit` naming `server-info.json` **and** `{_DEV_DIR}/server.log`.

5. **`start_frontend(deps)`**: `send_message("start", name="frontend")` only
   confirms the command was *accepted*, not that the watcher reached or stayed
   `active` — and with `respawn = false` a frontend that starts then immediately
   dies (e.g. `--strictPort` exit on a stolen port, or `npm`/`node` failing) goes
   `active`→`stopped` without a client error. So **bound the transition**: poll
   (injected-clock seam) for the frontend watcher to reach `active`; on a client
   error, a timeout, or an observed `active`→`stopped` within the window →
   `teardown`, `invoke.Exit` naming `{_DEV_DIR}/frontend.log` (never print "stack
   ready" with a dead frontend). Only once `active` is confirmed, **record the
   frontend watcher PID + start-time into dev-state** (incrementally — not batched
   with the server PID — so there is no launch-window gap where a watcher is live
   but unrecorded; read from circus / the watcher pidfile). teardown can then reach
   reparented orphans by identity even if the arbiter dies.

6. **Print the success block** and return (well within 10 s of step 4 succeeding):

   ```text
   Visualiser dev stack ready.
     Frontend: http://127.0.0.1:<fe_port>
     API:      http://127.0.0.1:<api_port>
     Logs:     .accelerator/tmp/dev/server.log
               .accelerator/tmp/dev/frontend.log
   ```

   On the **reuse** path the same block is printed under a heading that makes the
   staleness consequence explicit: "Dev stack already running (reused) — code
   changes since it started are NOT live; run `mise run dev:restart` to apply
   them." (Frontend URL from dev-state; API URL/port from `server-info.json`.)

Every failure branch raises `invoke.Exit` with a message naming the relevant
artifact (missing pidfile, daemon/server log, frontend log) — not a bare
traceback — so a developer can self-diagnose from the printed line.

#### 3. `stop` task

**File**: `tasks/dev.py` (thin adapter) + `tasks/dev_supervisor.py`
(`do_stop`/`teardown`).
**Changes**: `@task def stop(context)` calls `do_stop(deps)` and prints its
`StopResult`; `do_stop` delegates to the shared `teardown(state, deps) ->
StopResult`. The teardown sequence, ordered so identity is established **before**
any process is enumerated or signalled:

- `read_dev_state`; if `None` → return a `clean` result ("Dev stack not running.").
- **Establish the safe target set first** (before any signalling, and guarded
  against null/dead PIDs):
  - If the recorded `arbiter_pid` is non-null, build a live `psutil.Process`
    handle and **verify identity** (`pid_identity_matches(arbiter_pid,
    arbiter_start_time)`). On `NoSuchProcess` → the arbiter is already gone (skip
    arbiter kill). On a live-but-mismatched PID → **refused** (the recorded PID is
    now an unrelated process); never enumerate or signal it.
  - If `arbiter_pid` is **null** (the provisional/interrupted-launch case — e.g.
    `dev` was Ctrl-C'd after the self-detached `circusd` started but before its PID
    was confirmed), there is no PID to gate on. Fall back to the **endpoint as the
    liveness authority** (consistent with the design's "liveness = probe the
    endpoint" philosophy): if the endpoint is reachable, `quit` it and **confirm
    death by re-probing until the endpoint goes unreachable** (channel gone == dead
    → `clean`), bounded by the same generous wait; if it stays reachable past the
    bound → `survivor`. If the endpoint is already unreachable, treat as stale →
    `clean` after artifact removal.
  - While the arbiter handle is live, **snapshot its `children(recursive=True)`**
    as `(pid, create_time)` pairs (the in-tree descendants, captured *before*
    `quit`). Separately, take live handles for the recorded **`server_pid` /
    `frontend_pid`** (and their current descendants) — these remain reachable by
    identity even after the arbiter dies and its children reparent to init/launchd
    (where they are no longer `children()` of the dead arbiter). All snapshot
    reads are wrapped against `NoSuchProcess`.
- Probe endpoint: reachable → `send_message("quit")` (circus runs SIGTERM → 2 s
  `graceful_timeout` → SIGKILL with `stop_children` across the node/esbuild tree,
  then the arbiter exits). Confirm death by polling for the arbiter PID to exit
  (or, when `arbiter_pid` is null, by re-probing the endpoint per the null-PID
  fallback above) with a wait **generous enough to cover circus's own 2 s
  `graceful_timeout` plus a reaping margin** — never escalate to a direct kill
  while circus is still gracefully tearing down.
- If the arbiter is still alive after that wait (and identity-confirmed): SIGTERM
  → 2 s → SIGKILL the arbiter PID directly.
- **Reap surviving descendants, re-enumerated post-grace and individually
  identity-gated**: after the grace window, **re-walk the live descendants of the
  recorded `server_pid`/`frontend_pid`** (a *fresh* enumeration, not only the
  pre-quit snapshot) — this catches workers Vite/the server spawned lazily during
  teardown that the pre-quit snapshot missed. The pre-quit `(pid, create_time)`
  snapshot is the **recycled-PID baseline**: before signalling any PID, re-verify
  its `(pid, create_time)` (a child can exit and its PID be recycled during the
  grace window — the gate applies to children too), then SIGTERM → 2 s → SIGKILL.
  Skip any whose identity no longer matches. *(A sub-millisecond TOCTOU between
  the final `create_time()` read and the `kill` syscall is irreducible; signal via
  the already-held live `psutil.Process` handle and keep the read→kill gap to one
  statement — accepted as proportionate for a local dev tool, documented like the
  `free_port` race.)*
- **Artifact removal is gated on confirmed death, and never severs a live
  arbiter's control channel**: on a fully-confirmed-dead teardown, remove
  `_DEV_STATE`, `_PIDFILE`, `_INI`, **and the `ipc://` socket files** (mirroring
  `stop_server_stop`). On **survivor** (arbiter not confirmed dead) or **refused**
  (identity mismatch), **keep dev-state AND the pidfile AND the `ipc://` sockets**
  — unlinking a live-but-wedged arbiter's sockets would sever the only channel a
  later `dev:stop` could `quit` through, forcing a SIGKILL that orphans children;
  so socket/pidfile removal is explicitly **excluded** from "best-effort removal"
  whenever the arbiter is not confirmed gone. Surface a recovery-pointing message
  ("arbiter PID <n> did not match the recorded identity; not killing — investigate
  `\.accelerator/tmp/dev/dev.log` and re-run `mise run dev:stop`" / "arbiter <n>
  still alive after SIGKILL; left dev-state + sockets in place — see
  `.accelerator/tmp/dev/dev.log`, then re-run `mise run dev:stop`").
- **Teardown error contract**: artifact removal uses `missing_ok` and is
  best-effort across the *removable* set (dev-state/INI on confirmed death; the
  full set incl. sockets only on confirmed death) — a kill failure must not leave
  a half-removed removable set, but must also not remove a live arbiter's control
  endpoint (above). An unexpected `CallError` from `quit` (other than the expected
  unreachable case) is recorded and falls through to the identity-gated direct-kill
  path rather than aborting teardown.
- **Stale-socket removal is identity-gated too** (lock-free `dev:stop`/`dev:status`
  take no lock, and the `ipc://` paths are deterministic, so a stale-path teardown
  could otherwise race a concurrent `dev` that just rebound the same path): only
  unlink the socket files when the arbiter is confirmed gone / the endpoint is
  confirmed stale (probe times out AND the recorded identity is dead) — never
  remove a socket a newer launch may have just bound. The identity gate closes the
  rebind-*before*-probe case; to also close the narrow probe-*then*-rebind
  sub-window, the unlink **opportunistically takes the non-blocking workspace lock
  and skips unlinking if it cannot acquire it** (a concurrent `dev` holds the lock
  while binding), so a stale teardown never removes a socket a just-launched
  arbiter owns. (The residual is then the same proportionate, accepted class as the
  read→kill TOCTOU.)
- **Diagnostic sink**: teardown diagnostics (unexpected `CallError`, refused,
  survivor) go through **one `log_diagnostic(msg)` helper** (timestamp + stable
  `dev:` prefix) that prints to the command's stderr **and** appends to
  `.accelerator/tmp/dev/dev.log`; that path is named in the survivor/refused
  recovery messages (above) so the orchestration log is discoverable. `dev.log`
  lives under the `dev:stop`-cleaned dir and is **truncated on each `dev` (up)**
  so it does not accumulate across sessions. (The watchers' own output is in
  `server.log`/`frontend.log`; `dev.log` is the orchestration log.)

`teardown` **returns** the `clean`/`refused`/`survivor` `StopResult`; `do_stop`
and both `bring_up` reuse-gate call sites consume it (the reuse gate fails fast on
`survivor`/`refused`).

#### 4. `restart` task

**File**: `tasks/dev.py`
**Changes**: `@task def restart(context)` runs `do_stop` then `bring_up`, with an
explicit contract on the seam:

- On `do_stop` result `clean` (dev-state confirmed removed) → proceed to
  `bring_up` from a guaranteed-clean slate.
- On `survivor` (could not confirm the arbiter dead) → **abort** with a non-zero
  `invoke.Exit` and a message pointing at the survivor and the recovery step
  ("investigate and re-run `mise run dev:stop`, then `mise run dev`"), rather than
  launching a second arbiter to compete for ports.
- On `refused` (the recorded arbiter PID is now an unrelated live process) →
  **same conservative handling as `do_stop`: keep dev-state and abort** with a
  recovery-pointing message, rather than silently clearing state and relaunching
  (which could abandon whatever the recorded arbiter became). This keeps `do_stop`
  and `restart` consistent on `refused`. A subsequent `mise run dev` then
  reconciles via its lock + reuse/stale gate.

`bring_up`'s lock + reuse/stale gate remains authoritative, so even if a prior
`do_stop` left a still-live unhealthy arbiter, `up` reconciles it (tears it down,
honouring the teardown result) before launching rather than double-launching. The
`up` invariant — *on a clean return dev-state reflects exactly one healthy
arbiter, and on any teardown path dev-state is either absent or describes a live
arbiter* — is documented and unit-tested so the compose-by-call boundary is
explicit.

#### 5. Namespace + mise wiring

**File**: `tasks/__init__.py`
**Changes**: replace `ns.add_collection(Collection.from_module(dev))` with a
manual collection so `invoke dev` (bare) maps to `up`:

```python
ns_dev = Collection("dev")
ns_dev.add_task(dev.up, default=True)
ns_dev.add_task(dev.stop)
ns_dev.add_task(dev.restart)
ns_dev.add_task(dev.status)      # added in Phase 4
ns_dev.add_task(dev.server)
ns_dev.add_task(dev.frontend)
ns.add_collection(ns_dev)
```

Each task's **docstring carries the mental model** (surfaced via
`invoke dev <task> --help`). `up`'s docstring: "Start both processes detached in
the background under a circus arbiter; returns once ready. The arbiter keeps
supervising after this command exits — use `dev:stop` to tear it down, or
`dev:server`/`dev:frontend` for the manual two-terminal flow." `status`'s
docstring states the **exit-code legend** ("0 = both running, 3 = one running,
4 = neither — identical on macOS and Linux").

**File**: `mise.toml`
**Changes**: add tasks (existing `dev:server`/`dev:frontend` unchanged). The
descriptions set expectations (detached/background; the manual alternative; the
exit-code legend) so the model is discoverable from `mise tasks` without reading
the plan:

```toml
[tasks."dev"]
description = "Start + supervise the visualiser server + frontend, detached in the background (use dev:stop to tear down; dev:server/dev:frontend for the manual two-terminal flow)"
depends = ["build:server:dev", "deps:install:node"]
run = "invoke dev"

[tasks."dev:stop"]
description = "Stop the supervised dev server + frontend and the circus arbiter"
run = "invoke dev.stop"

[tasks."dev:restart"]
description = "Restart the supervised dev stack (stop then start)"
depends = ["build:server:dev", "deps:install:node"]
run = "invoke dev.restart"
```

### Success Criteria

#### Automated Verification

- [x] Orchestration unit tests pass against `bring_up`/`do_stop`/`do_status` and
      the named helpers (`reuse_or_teardown`, `allocate_and_launch`, `teardown`)
      via the injected `DevDeps` (fake `CircusClient` + fake launcher + fake killer
      + injected clock), all in `tests/tasks/test_dev.py`: reuse short-circuit;
      (`do_status` orchestration tests added in Phase 4 — done):
      **lock-held → re-probe → reuse** *and* **lock-held → fail-fast** (both
      branches); degraded-teardown that **honours a `survivor`/`refused` result by
      failing fast**; stale-cleanup (incl. removing stale `ipc://` sockets);
      **stale-info deletion before launch**; **pidfile-never-appears → reap-by-handle
      → clear error (no retry)**; **empty/partially-written pidfile keeps polling**;
      **provisional-state-before-launch**; **watcher-PID recording after both
      active**; readiness-timeout error path; arbiter identity-gated kill;
      **per-descendant identity re-check before reaping** (a recycled child PID is
      skipped); **orphan reaping via recorded `server_pid`/`frontend_pid` when the
      arbiter is already dead** (reparented children no longer under the arbiter);
      **null/dead arbiter PID guarded** (no enumeration/crash); **artifact removal
      gated on confirmed death** (refused/survivor keep state); restart
      `clean`/`survivor`/`refused` handling (refused keeps state + aborts); and the
      printed success/reuse blocks (incl. the reuse "changes NOT live" heading) —
      `mise run test:unit:tasks`
- [x] `invoke dev --help` lists `dev` (default), `dev.stop`, `dev.restart`, and
      each docstring carries the mental model / exit-code legend
- [x] Repo checks pass: `mise run check`

#### Manual Verification

- [ ] `mise run dev` on a clean build returns promptly and prints the labelled
      `Frontend:`/`API:`/`Logs:` block; `server.log` + `frontend.log` appear under
      `.accelerator/tmp/dev/`; the frontend proxies `/api` (no port-0 fallback).
- [ ] With a stale `server-info.json` left from a prior run, `mise run dev` still
      proxies `/api` to the **new** server (the stale file did not satisfy the
      gate).
- [ ] Re-running `mise run dev` prints the verbatim reuse heading ("Dev stack
      already running (reused) — code changes since it started are NOT live; run
      `mise run dev:restart` to apply them.") and starts no duplicate processes.
- [ ] `mise run dev:stop` leaves no `node`/server processes among the children of
      the recorded PIDs; `mise run dev:restart` comes back on a (possibly new)
      port with working `/api`.
- [ ] On a clean checkout (binary unbuilt, node deps uninstalled), `mise run dev`
      actually triggers `build:server:dev` + `deps:install:node` first and the
      stack still starts (the behavioural half of the prerequisite AC, which the
      fake-process integration suite cannot drive via the real task graph).
- [ ] Two jj workspaces run `mise run dev` concurrently with no collision (distinct
      ports + state files); a second `mise run dev` in the *same* workspace while
      one is mid-launch does not spawn a duplicate arbiter.

---

## Phase 4: `dev:status`

### Overview

Add reporting with the contractual exit codes and field set. Test-first on the
formatting/exit-code seam.

### Changes Required

#### 1. `status` task

**File**: `tasks/dev.py` (thin adapter) + `tasks/dev_supervisor.py` (`do_status`).
**Changes**: `@task def status(context)` calls `do_status(...)` and exits with its
code. `do_status`:

- `read_dev_state`; if `None` or endpoint unreachable (`CallError`) → both DOWN.
- Else `send_message("status")` → `evaluate_health`.
- Report each process's state; when the frontend watcher is `active`, print the
  frontend URL from dev-state; when the server watcher is `active`, print the API
  URL + port from `server-info.json`. **Always print the two log paths**
  (`.accelerator/tmp/dev/server.log` / `frontend.log`) — including on a HEALTHY
  stack, since tailing logs on a *running* stack is the most common reason to run
  `dev:status` and, with no `dev:logs` helper, the printed path is the only
  discovery route.
- Exit `status_exit_code(health)` (0 / 3 / 4) via `raise invoke.Exit(code=…)`.
  The printed wording correlates with each state (both-running / one-running /
  neither) so the exit code is inferable from the output, and the
  `dev:status` docstring states the legend explicitly.

> The brief `server=active, frontend=stopped` startup window (between `up`'s
> readiness gate and `start frontend`) can momentarily read as `PARTIAL`/exit 3.
> The status output labels it "(starting)" — but the label must be gated on
> **"the frontend watcher PID has never been recorded in dev-state"** (i.e. the
> stack is genuinely mid-first-launch), **not** merely on "frontend port
> populated". The port persists after a successful start, so a frontend that
> started and *later died* (`respawn = false`) would otherwise be mislabelled
> "(starting)" forever; with the watcher-PID gate, a settled-but-dead frontend
> correctly renders as degraded (exit 3), while only the true pre-first-start
> window shows "(starting)". The exit code is still `status_exit_code(health)`;
> "(starting)" is a display refinement, not a fourth state.

#### 2. mise wiring

**File**: `mise.toml`
**Changes**:

```toml
[tasks."dev:status"]
description = "Report dev server + frontend state, frontend URL, and resolved API port (exit 0 = both running, 3 = one, 4 = neither)"
run = "invoke dev.status"
```

### Success Criteria

#### Automated Verification

- [x] Exit-code mapping + field-rendering unit tests pass, including: the
      **"(starting)"** label is emitted for `server=active, frontend=stopped` only
      when the **frontend watcher PID has never been recorded** in dev-state; the
      complementary negative case — a frontend whose PID *was* recorded but is now
      dead (port still populated) renders as **degraded (exit 3)**, not
      "(starting)"; and log paths print on HEALTHY too: `mise run test:unit:tasks`
- [x] Repo checks pass: `mise run check`

#### Manual Verification

- [ ] With both up: `mise run dev:status` prints both states + frontend URL + API
      port and `echo $?` ⇒ `0`.
- [ ] After `dev:stop`: `echo $?` ⇒ `4`.
- [ ] Kill one watcher out-of-band: `echo $?` ⇒ `3`.

---

## Phase 5: End-to-end integration harness

### Overview

A pytest integration suite driving the **real** `invoke dev.*` tasks against
real `circusd`, using lightweight **Python fake processes** (a fake "server" that
writes `server-info.json` then spawns a child and sleeps; a fake "frontend" that
spawns a child and sleeps) so the lifecycle is exercised without building Rust or
booting Vite, and identically on macOS and Linux. Registered as
`test:integration:dev`.

### Changes Required

#### 1. Integration suite

**File**: `tests/tasks/test_dev_integration.py`
**Changes**: tests, each in an isolated tmp workspace dir, asserting. Timing-
sensitive cases assert **direction, not magnitude** (the precise poll-count /
deadline maths is pinned by the deterministic Phase 2 unit tests), and the
readiness timeout is **parametrised small** for the integration variant (~2 s,
not 30 s) so the suite does not hinge on a 30 s wall-clock window under loaded
parallel CI:

- **Detach + readiness**: `dev` returns; the arbiter survives the invoking shell
  (still running after the call returns); `server.log`/`frontend.log` non-empty and
  **correctly routed** — the fake server and fake frontend each emit a
  distinguishable startup marker, and the test asserts each marker appears only in
  its own log (no cross-wired streams — AC3's "its own process's output"); dev-state
  written with a frontend port and distinct per-workspace `ipc://` socket paths, and
  the recorded `server_pid`/`frontend_pid` populated.
- **Reuse**: second `dev` starts no new arbiter (PID unchanged) and reports reuse.
- **Stale-info does not satisfy the gate**: pre-create a stale `server-info.json`;
  a fake server that writes a *different* port must be the one the frontend wires
  to (the gate keyed on the fresh file, not the leftover).
- **Teardown without orphans (clean quit)**: record the server/frontend PIDs and
  their `psutil` children *before* stop; after `dev:stop` assert none survive
  (children-of-recorded-PIDs check, **not** a global grep) and the arbiter is gone.
- **Teardown without orphans (arbiter already dead)**: SIGKILL the arbiter
  out-of-band leaving its watcher children alive (now reparented away from the
  dead arbiter), then `dev:stop` → assert the reap-by-recorded-`server_pid`/
  `frontend_pid` path (identity-gated, re-enumerating their live descendants
  post-grace) kills the orphaned children too. This is the case
  `children(recursive=True)` on the dead arbiter alone would miss.
- **Orphan reach with only the server recorded**: SIGKILL the arbiter in the
  window after the readiness gate but **before** `start frontend` (only
  `server_pid` recorded, `frontend_pid` still null), leaving the server child
  alive; assert `dev:stop` reaps the server subtree via the incrementally-recorded
  `server_pid` — covering the launch-window gap the incremental recording closes.
- **Restart round-trip**: bring the stack up, capture the arbiter PID; run
  `dev:restart`; assert a **new** arbiter PID, both watchers active/HEALTHY, and a
  populated (possibly different) frontend port — a real stop→start round-trip
  against `circusd`, not just the unit seam (covers work-item AC6).
- **Readiness timeout**: a fake server that never writes `server-info.json` makes
  `dev` exit **non-zero** with a message naming the file + server log (assert the
  outcome + message, not the elapsed time; uses the small parametrised timeout).
- **Daemon startup failure (no retry)**: render an unstartable INI (e.g. a missing
  server binary) so the pidfile never appears; assert `dev` reaps the launched
  `circusd` via its launch handle — which, because we launch **without `--daemon`**
  (`Popen(start_new_session=True)`), is the **real** arbiter PID, not a double-fork
  intermediate — exits **non-zero** naming the daemon log, and leaves **no**
  orphaned `circusd`/child (assert via the handle PID and a sweep of its session).
  (With `ipc://` endpoints there is no port collision, so a non-appearing pidfile
  is a genuine failure, not a transient to retry.)
- **Discovery survives a lost state file**: bring the stack up, then truncate/delete
  `dev.json`; assert `dev:stop` still finds and tears down the arbiter by
  **recomputing** the deterministic `ipc://` paths (dev-state is a cache, not the
  sole source of truth) — no orphan.
- **Stale cleanup**: kill the arbiter out-of-band leaving the state file + stale
  `ipc://` sockets; `dev:stop` and `dev:status` treat it as not-running and clean
  up the sockets/state.
- **Recycled-PID safety (real psutil)**: record a real arbiter PID + create_time,
  let it exit, then assert `pid_identity_matches` returns `False` for that PID and
  `dev:stop` **refuses** to kill it and keeps dev-state (run on both CI legs, so
  the cross-platform `create_time` semantics behind the tolerance are exercised
  for real, not just mocked).
- **Cross-workspace concurrency**: bring up two arbiters in two isolated tmp
  workspace dirs; assert distinct frontend ports **and distinct `ipc://` socket
  paths**, both HEALTHY, and that `dev:stop` in one does not affect the other —
  the latter now structurally guaranteed by the per-workspace sockets. (Needs no
  Rust/Vite — the fakes suffice — so the concurrency AC is automated, not manual.)
- **npm/node PATH under the detached daemon**: place a fake `npm` only on a
  version-manager-style shim path and launch with a **stripped** PATH; assert the
  frontend watcher still resolves it (i.e. the daemon inherited the resolved
  PATH/cwd and/or the absolute-path render), proving the Phase 2 §2 mitigation on
  both CI legs rather than by manual check.
- **Frontend port/info wiring**: assert the frontend watcher's launch carries the
  allocated `--port <fe_port> --strictPort` and `VISUALISER_INFO_PATH` pointed at
  the (fresh) `server-info.json` — i.e. the resolved-port plumbing AC1/AC6 depend
  on is wired correctly. (The *real* Vite `resolveApiPort()`→`/api` proxy is not
  bootable in this fake harness; it stays a manual-verification step. The success
  criteria are worded as "the ordering gate fires and the frontend receives the
  resolved port" for the automated layer, with the end-to-end `/api` proxy
  confirmed manually — see Phase 3 Manual Verification.)
- **Status exit codes**: 0 / 3 / 4 across both-up / one-up / none-up.
- **2 s grace**: a fake process that ignores SIGTERM is **SIGKILLed** — assert the
  escalation *happened* (the process is gone and was killed, not graceful) with a
  generous upper margin, not a tight window.

> **Adapter fidelity vs the fake**: the unit tests fake the **`Supervisor`
> protocol** (a clean `status()/start()/quit()` + `SupervisorUnreachable`), so the
> risk shifts to the thin **circus adapter** that implements the protocol — it
> must correctly parse real circus wire shapes (`{"statuses": {name: state}}` for
> all watchers vs `{"status": state}` for one) and translate an unreachable/timed-
> out endpoint into `SupervisorUnreachable`. Those exact adapter branches are
> **cross-checked against real `circusd`** by the reuse and stale-cleanup
> integration cases above, so neither an over-friendly fake nor a mis-coded
> adapter can keep the unit tests green while production breaks.

#### 2. invoke task + mise wiring

**File**: `tasks/test/integration.py`
**Changes**: add `@task def dev(context)` running
`uv run pytest tests/tasks/test_dev_integration.py -v`. The real source of CI
contention is the **mise-level `test:integration` aggregate running its member
tasks in parallel** (the documented cause of this repo's shell-suite flakiness),
not intra-file parallelism — the project configures no pytest-xdist. The
de-flaking basis is therefore: per-test **isolated tmp workspace + per-workspace
`ipc://` sockets + `free_port()`** (no cross-test collisions) plus **generous,
direction-only timing margins** and a small parametrised readiness timeout.

Also add a cheap **config-shape unit test** (in `tests/tasks/test_dev.py`)
asserting the `dev` and `dev:restart` mise task definitions carry
`depends = ["build:server:dev", "deps:install:node"]`, so the prerequisite-auto-run
acceptance criterion (which the fake-process integration suite cannot exercise via
the real task graph) cannot silently regress.

**File**: `mise.toml`
**Changes**: add `[tasks."test:integration:dev"]` with
`depends = ["deps:install:python"]` (matching every other pytest-backed task —
without it the `circus`/`psutil` import fails on a clean runner, notably the new
macOS leg) and `run = "invoke test.integration.dev"`; append
`"test:integration:dev"` to the `test:integration` aggregate `depends`.

#### 2b. macOS CI matrix (enforce cross-platform parity)

**File**: `.github/workflows/main.yml`
**Changes**: the headline requirement is identical macOS/Linux behaviour, but CI
currently runs `test-unit`/`test-integration`/`test-e2e`/`check` on
`ubuntu-latest` only (macOS runs only in prerelease/release). Matrixing those
aggregate jobs wholesale would drag every unrelated suite (visualiser, frontend,
templates, config, hooks, github, migrate) onto a ~10×-priced macOS runner.
Instead add **one dedicated job, `test-cross-platform`**, with
`strategy.matrix.os: [ubuntu-latest, macos-latest]`, running **exactly**
`mise run test:unit:tasks` + `mise run test:integration:dev` on both legs. This is
the concrete change (not an either/or): it enforces the Requirement 10 parity
claim and proves the `circus`/`pyzmq`/`tornado`/`psutil` wheel resolution under
`requires-python >=3.14` on arm64 macOS, while keeping the added macOS minutes
scoped to the two relevant suites. Phase 1's wheel check and Phase 5's parity
success criteria reference this named job. **Add `test-cross-platform` to the
`prerelease` job's `needs:` list** (alongside the existing
`test-unit`/`test-integration`/`test-e2e`/`check`) so a red macOS leg actually
**gates** the release chain — otherwise the parity check is advisory, not
enforcing, and a macOS-only regression (e.g. a `sun_path` breach) could ship.
(`release` already gates transitively via `needs: prerelease`, so it needs no
direct edit.)

> **Deviation (as-built, maintainer decision):** rather than a dedicated
> `test-cross-platform` job scoped to the two dev suites, the existing
> `test-unit`/`test-integration`/`test-e2e` jobs were each turned into
> `strategy.matrix.os: [ubuntu-latest, macos-latest]` jobs, so **all** suites run
> cross-platform (not only the dev-task ones). This accepts the extra macOS
> minutes in exchange for catching macOS-only regressions across the whole repo,
> and removes the special-case job. `check` stays Linux-only (shell format/lint is
> platform-independent). `prerelease.needs` lists the three matrix jobs (a `needs`
> on a matrix job waits for every leg), so a red macOS leg still gates the release
> chain. This also dissolves the "`test:integration:dev` must run independently"
> constraint, since the matrix runs the whole `test:integration` aggregate.

> **Architecture coverage caveat**: the matrix proves wheel resolution + behaviour
> on `ubuntu-latest` (x86_64) and `macos-latest` (arm64). **aarch64 Linux**
> (Apple-silicon Docker, ARM CI/containers) is *not* exercised; circus/pyzmq/
> tornado/psutil are native packages whose CPython-3.14 aarch64-manylinux wheels
> are not guaranteed. Note aarch64 Linux as a known-unverified target in the
> Phase 1 wheel-check (documenting the C-toolchain + libzmq source-build fallback),
> or confirm aarch64 wheels exist for the pinned versions.

#### 3. Flip ADR-0041 to accepted

**File**: `meta/decisions/ADR-0041-circus-dev-process-supervision.md`
**Changes**: once the as-built behaviour matches, transition `proposed` →
`accepted` (per `ADR-0031` immutability rules).

### Success Criteria

#### Automated Verification

- [~] The `test-unit`/`test-integration`/`test-e2e` jobs are matrixed over
      `[ubuntu-latest, macos-latest]`, so every suite (incl. `test:unit:tasks` +
      `test:integration:dev`) runs on **both** legs with identical exit codes,
      field set, and no orphans — the cross-platform parity claim is enforced, not
      manual (this also proves the py3.14 wheel resolution for
      circus/pyzmq/tornado/psutil on arm64 macOS). Matrix added to
      `.github/workflows/main.yml`; green on darwin locally — the live CI run on
      both legs is pending push.
- [x] The matrixed `test-unit`/`test-integration`/`test-e2e` jobs are in the
      `prerelease`/`release` `needs:` list, so a red macOS leg of any suite
      **blocks** release (a `needs` on a matrix job waits for every leg; `release`
      gates transitively via `needs: prerelease`)
- [x] Integration suite includes the restart round-trip, lost-state-file
      discovery, orphan-reach-with-only-server-recorded, and daemon-startup-failure
      (real-handle reap) cases above, all green (on darwin locally; both legs via CI)
- [x] `test:integration:dev` is part of `mise run test:integration`
- [~] Full suite green: `mise run test` (dev unit + integration suites green;
      full Rust/e2e suite not re-run locally — covered by the existing CI jobs)
- [x] Repo checks pass: `mise run check`

#### Manual Verification

- [ ] **Required (outstanding human step)**: `mise run dev` exercised by hand
      from a *real interactive shell* on macOS confirms the detached arbiter
      outlives the terminal closing
      (a stronger condition than surviving a CI step's non-interactive shell under
      launchd), and `dev:stop` then leaves no orphaned `node`/server children.
- [x] ADR-0041 is `accepted` and matches the shipped design. (The server-log
      capture deviation is an implementation detail outside the ADR's
      circus-choice scope; recorded in the plan + code comments + memory.)

---

## Testing Strategy

### Unit Tests (`tests/tasks/test_dev.py` — single file, wired into `test:unit:tasks`)

- INI generation invariants (stop_children, graceful_timeout=2, autostart,
  respawn=false, `ipc://` endpoint/pubsub interpolation, frontend
  `--port/--strictPort`, `VISUALISER_INFO_PATH`).
- `free_port` bindable; `ipc_socket_paths` distinct / deterministic / within the
  `sun_path` limit / filesystem-path (no `@`) / `$TMPDIR`-unset resolution;
  `workspace_lock` acquire / fail-fast / release / atomic dead-owner reclaim /
  two-contender race resolves to one winner.
- dev-state round-trip / missing / malformed / schema-mismatch / provisional
  (null PIDs) — incl. the per-watcher incremental-PID writes.
- `evaluate_health` and `status_exit_code` over all combinations; `do_status`
  "(starting)" label **gated on frontend-PID-never-recorded** (a settled-but-dead
  frontend renders degraded, not starting); log paths printed on HEALTHY too.
- `wait_for_file` with injected clock (success, timeout, **exact** poll count, no
  overshoot); defensive pidfile parse (empty/partial → keep polling).
- `pid_identity_matches` (match, within-tolerance drift, recycle mismatch, dead
  PID).
- Orchestrator seams (`bring_up`/`do_stop`/`do_status` + `reuse_or_teardown` /
  `allocate_and_launch` / `start_frontend`; `teardown` + its `snapshot_targets` /
  `kill_arbiter` / `reap_descendants` / `remove_artifacts` sub-helpers) via injected
  `DevDeps` (incl. injected clock for grace/escalation waits): reuse short-circuit;
  lock-held reuse *and* fail-fast; degraded-teardown honouring `survivor`/`refused`;
  stale-cleanup with identity-gated socket removal; stale-info deletion; self-detach
  launch + daemon-startup-failure reap-by-real-handle + clear error (no retry);
  provisional-state-before-launch; incremental watcher-PID recording; readiness-
  timeout error; arbiter identity-gated kill; **per-descendant identity re-check**
  (recycled child skipped); **post-grace re-enumeration** of recorded-watcher
  descendants; orphan reaping via recorded watcher PIDs when the arbiter is dead;
  null/dead-PID guard; artifact removal gated on confirmed death with **sockets/
  pidfile retained on survivor/refused**; `config_renderer` injection; restart
  `clean`/`survivor`/`refused` (refused keeps state + aborts); success/reuse output
  blocks (verbatim strings).
- Config-shape: `dev`/`dev:restart` mise tasks declare the build/deps `depends`;
  unified server launch omits `--log-file`.

### Integration Tests (`tests/tasks/test_dev_integration.py`)

- Real `circusd` + Python fake processes; the end-to-end scenarios in Phase 5
  (detach, reuse, stale-info, both teardown-without-orphans variants, orphan-reach-
  with-only-server-recorded, restart round-trip, readiness timeout, daemon-startup-
  failure real-handle reap, lost-state-file discovery, stale cleanup, real-psutil
  recycled-PID safety, cross-workspace concurrency, npm/node PATH, frontend
  port/info wiring, status exit codes, 2 s grace), run on both the Linux and macOS
  CI legs. Timing-sensitive cases assert direction with generous margins; the
  readiness timeout is parametrised small.

### Manual Testing Steps

1. `mise run dev` on a clean checkout (no build, no node deps) → prerequisites run,
   stack starts, `/api` proxy works.
2. Re-run `mise run dev` → reuse.
3. `mise run dev:status` → fields + exit 0; stop → exit 4; kill one → exit 3.
4. `mise run dev:restart` → `/api` works on the new port.
5. Two jj workspaces concurrently → no collision.

## Performance Considerations

The readiness gate is the only material added latency; the contract caps it at
10 s after `server-info.json` is written (well under the 30 s server-boot
timeout). Frontend-port allocation has a negligible bind/close cost, and the
`ipc://` endpoints need no allocation at all. The non-blocking workspace lock is a
single `fcntl.flock` syscall on the happy path. circus's `check_delay = 1` keeps
the idle arbiter cheap.

## Migration Notes

No data migration. `dev:server` / `dev:frontend` continue to work unchanged for
the two-terminal workflow. New ephemeral state (dev-state, lock, INI, pidfile,
logs) lives under the existing `.accelerator/tmp/dev/` convention (ADR-0019);
the circus `ipc://` socket files live under a short `$TMPDIR`-rooted base keyed by
a hash of the workspace root (to stay within the `sun_path` length limit) and are
recorded in dev-state. Nothing is committed; all of it is removed by `dev:stop`.

## References

- Original work item: `meta/work/0101-unified-dev-task-for-visualiser.md`
- Research: `meta/research/codebase/2026-06-06-0101-unified-dev-task-for-visualiser.md`
- Binding spec: `meta/reviews/work/0101-unified-dev-task-for-visualiser-review-1.md`
- Prior art: `skills/visualisation/visualise/scripts/launch-server.sh`,
  `skills/visualisation/visualise/scripts/launcher-helpers.sh`
- Vite port resolution: `frontend/vite.config.ts:26-51`
- Current dev tasks: `tasks/dev.py:14-54`; mise wiring: `mise.toml:38-46`
- Namespace pattern to mirror: `tasks/__init__.py:20-30` (release collection)
- circus docs/source: `circus.readthedocs.io` (circusd, configuration),
  `circus-tent/circus` (`watcher.py`, `process.py`, `client.py`, `__init__.py`)
- Related: work item 0100 (`--owner-pid 0` coupling only; no ordering dependency)
