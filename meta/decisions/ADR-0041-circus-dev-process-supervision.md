---
adr_id: ADR-0041
date: "2026-06-07T00:00:00+00:00"
author: Toby Clemson
status: accepted
tags: [adr, dev-tooling, process-supervision, circus, visualiser]
---

# ADR-0041: circus for dev process supervision

**Date**: 2026-06-07
**Status**: Accepted
**Author**: Toby Clemson

## Context

The unified `mise run dev` task (work item 0101) must start and supervise
**two** dev processes together — the visualiser API server and the Vite
frontend — as detached background processes, with:

- a **separate log file per process** (server output and frontend output must
  not be multiplexed into one stream);
- **reuse detection** — re-running `dev` while a healthy session is up must
  detect it and not start a duplicate;
- **group teardown** — stopping must reach Vite's child `node`/`esbuild`
  workers, not just the top-level processes, so nothing orphans;
- **macOS + Linux parity** — identical start/stop/restart/status behaviour on
  both developer machines and CI.

No process-supervision dependency exists today; the dev tasks run each process
in the foreground under a pty in its own terminal, with no PID tracking and no
programmatic startup ordering.

## Decision Drivers

- Native per-process log files (work item Requirement 5)
- A long-lived control surface that makes reuse detection and `dev:status`
  reporting natural (Requirements 1, 4)
- Group teardown reaching descendant workers (Requirement 9)
- A Python control client that fits the existing `invoke`-task model
- macOS + Linux support (Requirement 10)

## Considered Options

1. **`honcho`** — a Procfile-style foreground multiplexer. Rejected: it
   multiplexes all processes into a **single combined stream**, so per-process
   log files (Requirement 5) are not native; and it is a foreground runner, not
   a long-lived supervisor with a control surface for reuse/status.
2. **`supervisor`** — a mature daemon supervisor. Rejected: it is **Unix-only
   by design** (no documented macOS-parity contract for our use), and its
   control model is XML-RPC against a config-file-defined daemon, which fits the
   `invoke`-task control flow less cleanly than a Python client.
3. **Raw `subprocess` + `os.killpg`** — hand-rolled supervision. Rejected: it
   reimplements per-process log capture, reuse detection, identity gating, and
   group teardown from scratch — exactly the prior-art surface
   `launch-server.sh`/`launcher-helpers.sh` already shows is fiddly to get right
   cross-platform — with no reduction in dependency-management burden once the
   behaviour is correct.
4. **`circus`** — a ZeroMQ-based arbiter supervising one or more watchers, with
   native `FileStream` per-process stdout/stderr capture and a Python
   `CircusClient`. Selected.

## Decision

Use **`circus`** as the dev process-supervision substrate. The dev orchestration
launches a long-lived `circus` arbiter against a **generated INI** declaring two
watchers (`server`, `frontend`), and drives it via `CircusClient`.

Two facts, verified against the live circus docs and source, are load-bearing
and shape the decision:

- **Detach via plain `circusd` (no `--daemon`).** `circusd --daemon`
  double-forks, so the PID of the process we spawn is a short-lived intermediate
  that exits immediately after forking the real arbiter — useless as a reap
  target on the startup-failure path. Instead we launch plain `circusd --pidfile
  <file> <generated.ini>` via `subprocess.Popen(..., start_new_session=True)`:
  the child detaches into its own session (surviving the invoking shell) **and**
  the Popen PID *is* the real arbiter, so the handle is a reliable reap target.
  `--daemon` was therefore **rejected**.
- **`stop_children = true` + `graceful_timeout = 2` are mandatory, per watcher.**
  circus does **not** `killpg` the process group; it `os.setsid()`s each process
  but signals **by PID**, reaching descendants only when the watcher sets
  `stop_children = true` (it then walks the `psutil` child tree). Without it,
  Vite's `node`/`esbuild` workers orphan, defeating the group-teardown
  requirement. `graceful_timeout` defaults to 30 s, but the contract requires a
  2 s SIGTERM grace, so it is set explicitly.

`psutil` is declared as a direct dependency (not relied on transitively via
circus), because the PID-identity check imports it directly.

### Wheel availability

For the pinned versions under `requires-python >=3.14`: `circus` is a pure-Python
wheel (`py3-none-any`); `pyzmq`, `psutil`, and `cffi` ship CPython-3.14 wheels
for macOS arm64, Linux x86_64, **and** Linux aarch64 (`manylinux2014_aarch64`);
`tornado` ships `cp39-abi3` wheels (stable-ABI, forward-compatible with 3.14)
for the same three targets plus musllinux. No native dependency falls back to a
source build on any of the three targets, so no C-toolchain/libzmq prerequisite
needs documenting. (Windows is out of scope regardless.)

## Consequences

### Positive

- Native per-process log files via `FileStream`, satisfying Requirement 5 with
  no multiplexing.
- The long-lived arbiter + `CircusClient` makes reuse detection, `dev:status`,
  and stop/restart natural Python control flow inside the `invoke` tasks.
- `stop_children` group teardown reaches Vite's descendant workers, so
  `dev:stop` leaves no orphans.
- `psutil` (transitive to circus, declared direct) supplies cross-platform
  `Process.create_time()` for the recycled-PID identity gate, sidestepping the
  macOS `LANG=C` start-time pitfall the shell launcher had to handle.

### Negative

- A new runtime dependency tree (`circus` → `pyzmq`/`tornado` + `psutil` →
  `cffi`) is added to the `build` group.
- The dev orchestration couples directly to circus's API surface — the INI
  dialect, `CircusClient` verbs, and `circus.exc.CallError` semantics — with
  **no abstraction boundary at the wire level**. Replacing circus later (with
  `supervisor`, `honcho`, or raw `subprocess`+`os.killpg`) is a rewrite of the
  supervision layer, not an adapter swap. This is acceptable lock-in for a
  dev-only tool; recording it makes the cost a conscious choice. To limit the
  blast radius, the orchestration depends on a small local `Supervisor`
  protocol (`status`/`start`/`quit`, raising a local `SupervisorUnreachable`)
  implemented by a thin circus adapter, and **all circus-specific code** (the
  INI rendering, the adapter, and the self-detaching `circusd` launcher) is
  confined to a single `tasks/shared/dev/circus.py` module — kept separate from
  the dev-state, lifecycle-orchestration, and reusable
  process/locking/polling helpers — so a future replacement is a rewrite of one
  module plus the `Supervisor` adapter, not a change rippling through every
  helper.
- `stop_children` and `graceful_timeout` **must be set explicitly** on every
  watcher, because circus signals by PID, not by process group, and its default
  grace is 30 s — an easy footgun for anyone adding a watcher later.

### Neutral

- circus is **not Windows-compatible**; macOS + Linux are the only supported dev
  targets, which matches the project's developer/CI platforms.
- "Manage" here means start / stop / restart / status only. Watchers are
  configured `respawn = false`, so circus does **not** resurrect a crashed
  process — crash auto-restart and file-watch supervision are out of scope (Vite
  already provides HMR).

## References

- `meta/work/0101-unified-dev-task-for-visualiser.md` — source work item
- `meta/plans/2026-06-06-0101-unified-dev-task-for-visualiser.md` —
  implementation plan
- `meta/decisions/ADR-0030-adr-template.md` — template this ADR follows
- `meta/decisions/ADR-0029-sequential-adr-identifiers.md` — identifier scheme
- `meta/decisions/ADR-0031-skill-level-adr-immutability.md` — status lifecycle
- `meta/decisions/ADR-0019-ephemeral-file-separation-via-paths-tmp.md` —
  `.accelerator/tmp/` convention the dev-state/logs live under
- Prior art: `skills/visualisation/visualise/scripts/launch-server.sh`,
  `skills/visualisation/visualise/scripts/launcher-helpers.sh`
- circus docs/source: `circus.readthedocs.io`, `circus-tent/circus`
