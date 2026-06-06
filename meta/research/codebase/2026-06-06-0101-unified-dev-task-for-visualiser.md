---
type: codebase-research
id: "2026-06-06-0101-unified-dev-task-for-visualiser"
title: "Research: Unified Managed dev Task for Visualiser Server and Frontend (0101)"
date: "2026-06-06T17:34:38+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0101"
parent: "work-item:0101"
relates_to: ["codebase-research:2026-04-17-meta-visualiser-implementation-context"]
topic: "Unified mise run dev task supervising the visualiser API server and Vite frontend under circus"
tags: [research, codebase, visualiser, dev-tooling, mise, invoke, circus, process-supervision, lifecycle]
revision: "0bbab3114c0dbc1639ec4a2297e23cc4fec1ccf7"
repository: "miscellaneous"
last_updated: "2026-06-06T17:34:38+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Unified Managed dev Task for Visualiser Server and Frontend (0101)

**Date**: 2026-06-06T17:34:38+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: 0bbab3114c0dbc1639ec4a2297e23cc4fec1ccf7
**Branch**: HEAD (jj working copy)
**Repository**: miscellaneous (Accelerator plugin)

## Research Question

For work item 0101 (`meta/work/0101-unified-dev-task-for-visualiser.md`): how does the
current visualiser dev workflow work, and what existing patterns must a unified
`mise run dev` task (plus `dev:stop` / `dev:restart` / `dev:status`) mirror when it
adds a `circus` supervisor to manage both the Rust API server and the Vite frontend
as detached background processes?

## Summary

The current dev workflow is a **two-terminal, foreground** split with **no PID tracking,
no backgrounding, and no programmatic startup ordering**:

- `mise run dev:server` → `invoke dev.server` builds (`build:server:dev`) and runs the
  Rust debug binary in the foreground under a pty, writing config (with `--owner-pid 0`)
  to `.accelerator/tmp/dev-server/config.json`. The **server itself** writes
  `server-info.json` (its chosen port) and `server.pid` into that dir.
- `mise run dev:frontend` → `invoke dev.frontend` runs Vite in the foreground with
  `VISUALISER_INFO_PATH` pointed at `server-info.json`.

The **load-bearing startup constraint** is real: Vite's `resolveApiPort()` reads the
port from `server-info.json` **once, at config-eval time**, with no re-read. If the
server hasn't written that file before Vite loads its config, the `/api` proxy is
permanently wired to port 0 (ECONNREFUSED). Today only the "start the server first"
convention enforces this — there is no wait.

Strong prior art exists in the **production launcher** (`launch-server.sh` +
`launcher-helpers.sh`), which 0101 must mirror: reuse-before-lock ordering, non-blocking
`flock` (with `mkdir` fallback), spawn-detached-then-poll-for-`server-info.json`
(5 s / 0.1 s), `(pid, start_time)` identity checks (with macOS `LANG=C` forcing) before
any kill, and SIGTERM → 2 s grace → SIGKILL escalation with a synthesised
`server-stopped.json`. **None of this is used by the dev path today** — the dev path is
foreground-only against a *different* tmp dir (`dev-server/` vs production's
`visualiser/`).

The work item and its review have already **resolved the key decisions**: supervision =
`circus`; already-running behaviour = **reuse** (gated on a "healthy session" =
arbiter-reachable **AND** both watchers running); plus a fixed numeric contract (30 s
readiness / 100 ms poll / 10 s return / 2 s SIGTERM grace / status exit codes 0/3/4).
`circus` is **not yet a dependency** — `pyproject.toml` only ships `invoke` (+ build/test
tooling), and an ADR recording the `circus` choice is a required acceptance criterion.

## Detailed Findings

### 1. Current dev tasks (`tasks/dev.py`)

The dev tasks are thin invoke functions, both **foreground** (`pty=True`), with **no PID
tracking** and **no `pre=` dependencies** (ordering lives in mise `depends`).

Module-level path constants — note these are **private to `dev.py`**, not exported from
`shared/paths.py` (`tasks/dev.py:7-11`):

```python
_TMP_DIR = REPO_ROOT / ".accelerator/tmp/dev-server"
_CONFIG_PATH = _TMP_DIR / "config.json"
_SERVER_INFO_PATH = _TMP_DIR / "server-info.json"
_SERVER_BIN = SERVER / "target/debug/accelerator-visualiser"
_WRITE_CONFIG = VISUALISER / "scripts/write-visualiser-config.sh"
```

**`server()`** (`tasks/dev.py:14-39`):
1. `mkdir -p .accelerator/tmp/dev-server/` (`:27`).
2. Reads plugin version from `.claude-plugin/plugin.json` (`:28`).
3. Runs `write-visualiser-config.sh` with `--owner-pid 0` (`hide=True`, captures stdout)
   (`:29-37`) — the **task** then `write_text`s the JSON to `config.json` (`:38`). Plain
   `write_text`, **not** `atomic_write_text` (which exists at `tasks/shared/files.py:4-11`
   but is unused here).
4. Runs `{_SERVER_BIN} --config {_CONFIG_PATH}` with `pty=True` (`:39`) — **blocks**. Sets
   **no env vars**.

`--owner-pid 0` disables the owner-liveness auto-shutdown watcher (the server's watch loop
only checks owner liveness `if owner_pid > 0`); idle-timeout shutdown still applies. The
`write-visualiser-config.sh` default is also `OWNER_PID=0`, so passing it is belt-and-braces.

**`frontend()`** (`tasks/dev.py:42-54`):
```python
context.run(
    f"npm --prefix {FRONTEND} run dev",
    env={"VISUALISER_INFO_PATH": str(_SERVER_INFO_PATH)},
    pty=True,
)
```
Sets only `VISUALISER_INFO_PATH` (never `VISUALISER_API_PORT`), foreground/blocking. There
is **no programmatic wait** for `server-info.json` — ordering is operator convention.

### 2. Build / install prerequisites

- **`build:server:dev`** → `build.server_dev` (`tasks/build.py:109-119`):
  `cargo build --manifest-path {CARGO_TOML} --no-default-features --features dev-frontend`.
  Output lands at the default debug path `server/target/debug/accelerator-visualiser` —
  exactly `_SERVER_BIN`. The `dev-frontend` feature serves the frontend from the filesystem
  rather than the embedded `embed-dist` default.
- **`deps:install:node`** → `deps.install_node` (`tasks/deps.py:21-24`):
  `npm --prefix {FRONTEND} ci`.

Requirement 8 (run prerequisites) maps onto these two existing tasks.

### 3. mise → invoke wiring (`mise.toml`)

```toml
[tasks."dev:server"]                  # mise.toml:38-41
description = "..."
depends = ["build:server:dev"]
run = "invoke dev.server"

[tasks."dev:frontend"]                # mise.toml:43-46
description = "..."
depends = ["deps:install:node"]
run = "invoke dev.frontend"
```

Pattern: colon-namespaced quoted task name; `depends = [...]` runs (and **completes**)
prerequisites first; `run = "invoke <module>.<task>"` delegates (invoke converts
underscores → hyphens, e.g. `server_dev` → `build.server-dev`).

**Critical wiring constraint for the unified task**: mise `depends` **block to completion**
before `run`. A unified `dev` therefore **cannot** simply `depends = ["dev:server",
"dev:frontend"]` — those foreground tasks would never return. The orchestration
(background-launch server → poll for `server-info.json` → launch frontend) must live
**inside `tasks/dev.py`**, with mise `depends` used only for the non-blocking build/install
prerequisites (`build:server:dev`, `deps:install:node`).

### 4. Namespace assembly (`tasks/__init__.py`)

`dev` is added wholesale via `ns.add_collection(Collection.from_module(dev))`
(`tasks/__init__.py:33`), which auto-exposes **every** `@task` in `dev.py`. So new
`stop` / `restart` / `status` (and a `dev` orchestrator) functions are picked up **without
touching `__init__.py`**. To get a bare `invoke dev` default (vs `invoke dev.dev`), switch
`dev` to the manual `Collection("dev")` + `add_task(..., default=True)` pattern that
`release` uses (`tasks/__init__.py:20-30`).

### 5. Shared paths (`tasks/shared/paths.py`)

Exports `REPO_ROOT`, `VISUALISER`, `SERVER`, `CARGO_TOML`, `FRONTEND`, `PLUGIN_JSON`,
`BIN_DIR`, etc. (`tasks/shared/paths.py:3-13`). **Gap**: there is **no** shared constant for
`.accelerator/tmp/`, the dev-server dir, `server-info.json`, or the debug binary — those are
private to `dev.py`. If `dev:stop`/`dev:status` need the info/pid paths, the existing
pattern is module-private constants in `dev.py`; promoting them to `shared/paths.py` is a
small optional refactor. `atomic_write_text` already exists (`shared/files.py:4-11`) if the
new tasks want atomic config writes.

### 6. Vite API-port resolution — the startup-ordering constraint
(`frontend/vite.config.ts`)

`resolveApiPort()` (`vite.config.ts:26-49`) precedence:
1. `VISUALISER_API_PORT` env (if present and `Number.isFinite`) (`:27-28`).
2. Else read `VISUALISER_INFO_PATH` synchronously, `JSON.parse`, return `info.port` only if
   `typeof info.port === 'number'` (`:30-41`). Parse/read errors are caught and warned.
3. Else **fall back to port 0** (`:43-48`).

**Resolution happens at config-eval time**: `const apiPort = resolveApiPort()` runs at module
top level (`:51`) and is interpolated once into the proxy target
`` target: `http://127.0.0.1:${apiPort}` `` (`:58`). There is **no re-read after startup**.
If `server-info.json` is absent/portless when Vite loads, `/api` proxies to
`http://127.0.0.1:0` → ECONNREFUSED (deliberate "fail loudly", per `:46` and
`frontend/README.md:26`).

The frontend reads **only** the `port` field; the full `ServerInfo` struct (written by the
Rust server) is at `server/src/server.rs:150-162`, populated `:289-298`, with
`port: local.port()` (the real bound port) and `url: format!(...)`.

**Frontend URL for `dev:status`**: `vite.config.ts` sets only `proxy`, not `server.host`/
`server.port`, so Vite uses defaults (`http://localhost:5173`, auto-incrementing if taken).
No artefact records the Vite dev URL — a `dev:status` task must assume the default (or set
`server.port` explicitly). The only structured URL is the **backend** `url` in
`server-info.json`.

`VISUALISER_INFO_PATH` has **no default** in the Vite config — if unset the whole block is
skipped. Nothing in the repo sets it for plain `npm run dev`; the dev task is what supplies it.

### 7. Production launcher patterns to mirror (`scripts/launch-server.sh`,
`scripts/launcher-helpers.sh`)

These implement the supervision surface the work item points at. **The launcher does not
write the pidfile/info file — the spawned binary does.**

- **PID file** (`launch-server.sh:20`): `PID_FILE="$TMP_DIR/server.pid"`; readers sanitise
  via `tr -cd '0-9'`. The launcher spawns detached and polls for the file's appearance
  (`:196-203`); it never persists the child PID itself.
- **flock** (`launch-server.sh:57-72`): non-blocking `exec 9>"$LOCK"; flock -n 9` on
  `launcher.lock`, held for the launcher's lifetime (released on fd close — no trap).
  Portable **`mkdir "$LOCK.d"` fallback** for macOS (no `flock`), released via EXIT trap.
- **Reuse short-circuit BEFORE the lock** (`launch-server.sh:26-42`): if `server-info.json`
  **and** `server.pid` exist → read PID → `kill -0` liveness → **start-time identity match**
  vs the `start_time` in `server-info.json` → validate `url` against
  `^http://127\.0\.0\.1:[0-9]+/?$` → print URL and `exit 0`. Any failure falls through to
  `rm -f "$INFO" "$PID_FILE"` (stale cleanup) and proceeds.
- **Readiness poll** (`launch-server.sh:200-207`): `for _ in $(seq 1 50); … sleep 0.1` =
  **5 s** total, waiting for **both** `server-info.json` and `server.pid`; on timeout
  `die_json` naming the log file.
- **SIGTERM → grace → SIGKILL** in `stop_server_stop` (`launcher-helpers.sh:167-190`):
  `kill` (SIGTERM) → poll 20 × 0.1 s = **2 s grace** → `kill -9` → 0.1 s settle → fail if
  still alive. Post-condition: `server-stopped.json` must exist; if forced/missing,
  synthesise it via `write_server_stopped … "forced-sigkill"` (atomic mktemp + rename,
  0600).
- **start-time identity** (`launcher-helpers.sh:45-74`): `start_time_of` reads
  `/proc/<pid>/stat` field 22 + `btime`/`CLK_TCK` on Linux; on macOS reads
  `ps -p <pid> -o lstart=` then `date -j -f` — **both forced under `LANG=C LC_ALL=C`**
  because the daemon records its start-time under `LANG=C`; a locale mismatch makes every
  reuse check fail and silently respawns the daemon. `start_time_matches`
  (`launcher-helpers.sh:84-91`) tolerates ±1 s drift, but note the **as-shipped** reuse
  short-circuit, `stop_server_stop`, and `stop_server_status` use **exact equality** inline,
  not the tolerant helper — a new dev task wanting drift tolerance should call
  `start_time_matches`.
- **Stale-state cleanup**: layered `rm -f "$INFO" "$PID_FILE"` on dead-PID / identity-mismatch
  / malformed-URL paths; `rm -f "$STOPPED"` on fresh launch. There is **no unix socket** —
  the server binds a TCP loopback port, so "stale socket" = "stale `server-info.json` +
  `server.pid`".
- **Cross-platform**: `darwin`/`linux` gate; `/proc` vs `ps` for start-time and ppid;
  `flock` vs `mkdir` lock; `sha256sum` vs `shasum`.

### 8. Dependency state (`pyproject.toml`)

Uses PEP 735 `[dependency-groups]` managed by **uv** (`uv.lock` present; `requires-python
>=3.14`):

```toml
[dependency-groups]
build = ["invoke>=2.2.1", "keepachangelog>=2.0.0", "rich>=14.2.0",
         "semver>=3.0.4,<4", "tomlkit>=0.13,<0.14", "ziglang>=0.16.0",
         "cargo-zigbuild>=0.22.3"]
dev = ["pytest>=8", "pytest-mock>=3.14"]
```

`circus` is **not present**. Since it backs the build/dev invoke tasks, it would naturally
join the `build` group. `[tool.uv] prerelease = "allow"`. The ADR acceptance criterion (work
item AC) requires recording the `circus` choice before the item is done — and no existing ADR
covers dev tooling / process supervision / dependency selection (closest adjacent:
`ADR-0019-ephemeral-file-separation-via-paths-tmp.md`).

### 9. Resolved decisions & fixed contract (from review 1)

`meta/reviews/work/0101-unified-dev-task-for-visualiser-review-1.md` (final verdict
**APPROVE**; item now `status: ready`) locked in:

- **Supervision = `circus`** (supervisor vetoed as Unix-only; investigation phase removed).
- **Reuse, gated on "healthy session"** = arbiter reachable **AND** both watchers running;
  unhealthy → not reused, handled as stale state.
- **`dev` launches the arbiter and detaches** from it.
- Numeric contract: **30 s** readiness timeout, **100 ms** poll interval, **10 s** return
  (re-anchored to `server-info.json` being written), **2 s** SIGTERM grace, status exit codes
  **0** (both) / **3** (one) / **4** (neither), identical macOS/Linux.
- **Teardown scope** = children of the recorded server/frontend PIDs (process groups), **not**
  a global process grep.
- **ADR** promoted to an explicit deliverable.
- **0100 relationship** = no ordering dependency; only coupling is `--owner-pid 0`.
- Still open (non-blocking): `dev:logs` helper deferred; `kind: task` kept despite
  story-sized breadth.

## Code References

- `tasks/dev.py:7-11` — private path constants (`_TMP_DIR`, `_CONFIG_PATH`,
  `_SERVER_INFO_PATH`, `_SERVER_BIN`, `_WRITE_CONFIG`).
- `tasks/dev.py:14-39` — `server()`: config write (`--owner-pid 0`) + foreground binary run.
- `tasks/dev.py:42-54` — `frontend()`: Vite with `VISUALISER_INFO_PATH`, foreground.
- `tasks/build.py:109-119` — `build:server:dev` (debug binary, `dev-frontend` feature).
- `tasks/deps.py:21-24` — `deps:install:node` (`npm ci`).
- `tasks/__init__.py:33` — `dev` auto-exposed via `Collection.from_module`.
- `tasks/shared/paths.py:3-21` — path constants + `binary_path`/`debug_archive_path`.
- `tasks/shared/files.py:4-11` — `atomic_write_text` (available, unused by dev).
- `mise.toml:38-46` — `dev:server` / `dev:frontend` definitions and `depends`.
- `frontend/vite.config.ts:26-49` — `resolveApiPort()` precedence.
- `frontend/vite.config.ts:51,55-62` — config-eval-time resolution + `/api` proxy target.
- `server/src/server.rs:150-162,289-312` — `ServerInfo` struct and writer.
- `server/src/main.rs:39` — `info_path = cfg.tmp_path.join("server-info.json")`.
- `server/src/lifecycle.rs:41-48` — `owner_pid > 0` gate + idle-timeout watch.
- `scripts/launch-server.sh:26-42` — reuse short-circuit; `:57-72` flock/mkdir lock;
  `:200-207` 5 s readiness poll.
- `scripts/launcher-helpers.sh:45-74` — `start_time_of` (LANG=C on macOS); `:84-91`
  `start_time_matches`; `:135-198` `stop_server_stop` (2 s grace → SIGKILL).
- `pyproject.toml:10-23` — dependency groups (no `circus`).

## Architecture Insights

- **Server-owned lifecycle files**: the Rust server — not the launcher — writes
  `server-info.json` / `server.pid` (atomically, 0600), so a launcher race can't leave a
  stale PID for a server that never started. The supervisor's job is to serialise launches,
  detect/reuse, and poll for the handshake files. A circus arbiter should preserve this:
  start the server watcher, then **poll for `server-info.json`** rather than trusting the
  watcher's "running" state as readiness.
- **`(pid, start_time)` identity** is the defence against PID reuse, and the macOS `LANG=C`
  forcing is **mandatory** for start-time parity (already burned-in lesson — see auto-memory
  `project_playwright_launcher_locale.md`). Any start-time comparison the dev task does on
  macOS must force `LANG=C`.
- **`--owner-pid 0` under the harness**: the owner-watcher anchors a daemon's life to a PID;
  under Claude Code's ephemeral per-Bash-tool shells, `$$` dies seconds later and tears the
  daemon down. The dev path must keep `--owner-pid 0` so the server isn't killed by owner
  death; the circus arbiter itself must likewise be detached and not owner-anchored.
- **mise `depends` are barriers**: prerequisites complete before `run`. Concurrent
  long-running processes can't be expressed as `depends` — the orchestration belongs in
  Python (`tasks/dev.py`).
- **Two tmp dirs**: dev uses `.accelerator/tmp/dev-server/`; production uses
  `.accelerator/tmp/visualiser/`. The new dev task targets `dev-server/` (and the work item
  proposes per-process logs under `.accelerator/tmp/dev/`).
- **circus mechanism fit**: `FileStream` gives the per-process `server.log` / `frontend.log`
  (Requirement 5); the long-lived arbiter + `CircusClient` make reuse-detection and
  `dev:status` natural; the arbiter owns each watcher's process group, giving group-wide
  SIGTERM→SIGKILL teardown so Vite's node/esbuild children don't orphan.

## Historical Context

- `meta/plans/2026-04-18-meta-visualiser-phase-2-server-bootstrap.md` — foundational design
  for the launcher lifecycle 0101 mirrors: server-owned lifecycle files; `(pid, start_time)`
  identity (Rust ↔ shell byte-identical, with a Rust test); `owner_pid > 0` gate; SIGTERM →
  2 s grace → SIGKILL with synthesised `forced-sigkill`; non-blocking flock; 5 s
  `server-info.json` readiness poll (a deliberately-kept bound from the superpowers
  precedent).
- `meta/notes/2026-05-19-playwright-daemon-owner-pid-ephemeral-shell.md` — empirical
  justification for `--owner-pid 0` under the harness; documents the owner-watcher and the
  reuse-by-PID-liveness adoption mechanism. Caution: the Playwright daemon uses a **bare PID
  probe** (no start-time check) — the visualiser launcher's `(pid, start_time)` check is the
  safer model to mirror.
- `meta/reviews/work/0101-unified-dev-task-for-visualiser-review-1.md` — the binding spec
  (decisions + numeric contract above).
- `meta/work/0100-configurable-visualiser-auto-shutdown.md` (+ its 2026-06-06 plan/research/
  review) — adjacent auto-shutdown work; **no ordering dependency** with 0101, only the
  shared `--owner-pid 0` mechanism.
- `meta/decisions/ADR-0019-ephemeral-file-separation-via-paths-tmp.md` — `.accelerator/tmp/`
  convention the dev logs/state live under.
- `meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md` — original
  visualiser implementation context.

## Related Research

- `meta/research/codebase/2026-06-06-0100-configurable-visualiser-auto-shutdown.md` —
  sibling research on the auto-shutdown lifecycle that 0101's `--owner-pid 0` interacts with.
- `meta/research/codebase/2026-05-18-0072-playwright-daemon-cjs-import-bug.md` and
  `meta/research/codebase/2026-05-19-inventory-design-and-browser-agent-fixes.md` — daemon
  lifecycle / launcher precedents.

## Open Questions

1. **`circus` integration shape** — How exactly does a `CircusClient`-driven arbiter get
   launched detached from `invoke dev.dev` and survive the invoking shell? (The work item
   says "launches the arbiter and detaches"; the precise detach mechanism — `circusd` as a
   subprocess vs embedded arbiter in a forked daemon — is an implementation choice not yet
   pinned. No web research was requested; the `circus 0.19.0` capability claims in the work
   item are unverified against live docs.)
2. **Start-time identity for circus PIDs** — Will the dev task replicate the
   `(pid, start_time)` + `LANG=C` identity check for the arbiter/watcher PIDs, or rely on
   circus's own endpoint reachability for the "healthy/stale" determination? Requirement 9
   asks for stale-endpoint/recycled-PID handling.
3. **Frontend URL reporting** — `dev:status` needs the Vite dev URL, but Vite records none.
   Assume `http://localhost:5173`, or set `server.port` explicitly in `vite.config.ts` so the
   URL is deterministic?
4. **Path-constant promotion** — Promote dev tmp/info/pid paths from `dev.py`-private to
   `shared/paths.py` (so stop/status/restart share them), or keep them module-private?
5. **`dev:logs` helper** — in scope or deferred (open per the work item and review)?
6. **Reuse of existing `dev:server`/`dev:frontend`** — the assumption is they remain and the
   unified `dev` composes the same underlying launch logic; the circus watchers will likely
   reimplement (not shell out to) those foreground tasks, since `pty=True` foreground runs
   can't be backgrounded under an arbiter.
