---
type: adr
id: "ADR-0058"
title: "Shell-Free CLI-to-Node Delegation"
date: "2026-08-10T16:19:07+00:00"
author: Toby Clemson
producer: create-adr
status: accepted
relates_to: ["adr:ADR-0045", "adr:ADR-0046", "adr:ADR-0048", "adr:ADR-0049",
  "adr:ADR-0053", "adr:ADR-0057", "work-item:0196"]
tags: [architecture, cli, shell, node, process-supervision, playwright, design]
last_updated: "2026-08-10T16:45:46+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# ADR-0058: Shell-Free CLI-to-Node Delegation

**Date**: 2026-08-10
**Status**: Accepted
**Author**: Toby Clemson

## Context

The `inventory-design` skill reaches Playwright through `run.sh`, a 203-line
bash launcher that supervises a long-lived Node daemon. It resolves the tmp
directory by shelling out to `accelerator config path tmp`, derives a cache
namespace from a `package-lock.json` hash, decides whether an existing daemon
is reusable (PID liveness plus a start-time comparison that guards against PID
recycling), takes a lock (`flock`, with a `mkdir` fallback), spawns
`run.js daemon` under `nohup`, waits for readiness, and finally `exec`s the
requested command. The daemon and the automation it drives are JavaScript
(`lib/daemon.js` and siblings).

That final `exec` starts a short-lived Node client, which reads the daemon's
`server-info.json` and POSTs a versioned JSON command to its HTTP endpoint
(`lib/client.js`, 52 lines). A Node process therefore starts on every browser
command, whether or not the daemon was reused.

Work-item:0196 moves this tooling into an `accelerator-design` sub-binary,
which forces a decision about the launcher: after the move, a Rust binary is
the caller, so the shell layer is either reproduced, retained, or relocated.

The shell path is not free. It depends on `jq`, `flock`, `sha256sum` and
`nohup` being present on the host, reads process start times from `/proc` on
Linux and `ps` on macOS, and sits on the bash 3.2 floor that ADR-0049 records.
That portability surface has already cost us: `start_time_of` carries an
explicit `LANG=C` fix because both `ps lstart` and `date -j` localise day and
month names, and without it every reuse check failed, the daemon respawned
between commands, and page state from a prior `navigate` was lost.

One further detail bears on the choice. `run.sh` shells out to the
`accelerator` binary to read a config path. Once the caller *is* that binary,
this becomes a process invoking itself re-entrantly to obtain data it already
holds.

A prior re-scope of work-item:0196 (2026-08-08) had disposed of `run.sh` as a
thin wrapper to retain. This ADR records the decision that replaced it.

## Decision Drivers

- **Shell's footprint should shrink, not persist** — ADR-0048 states the
  direction; every retained bash line carries the 3.2 floor with it.
- **Fewer host prerequisites** — the tools this launcher assumes are exactly
  the kind of ambient dependency ADR-0046 set out to remove.
- **No re-entrant self-invocation** — a process should not shell out to itself
  for state it already has.
- **Supervision in a language with types and tests** — a daemon that
  misbehaves is easier to diagnose when the logic supervising it can be type
  checked and unit tested, which bash cannot.
- **Keep the JavaScript that must be JavaScript** — `playwright-core` is a JS
  library; the automation has no choice about its runtime, but the launcher
  does.

## Considered Options

1. **Reproduce the launcher in Rust** — the CLI performs reuse detection,
   locking and spawning itself, then runs the vendored Node directly.
2. **Retain `run.sh` as a thin wrapper** invoked by the Rust binary, leaving
   supervision in bash.
3. **Fold supervision into `run.js`** — move reuse detection, locking and
   daemonisation into the JavaScript that already owns the daemon, leaving the
   CLI a plain exec of Node.
4. **Speak the daemon protocol from Rust** — reproduce the launcher *and* the
   client, retiring `lib/client.js` and `run.js`'s dispatch, so the CLI talks
   to the daemon over its HTTP/JSON protocol directly.

## Decision

We will **reproduce `run.sh`'s launcher behaviour in Rust**, so the delegation
chain is **CLI to Node with no shell in between**.

- What remains in JavaScript is the Playwright automation itself (`run.js` and
  `lib/*.js`), which must run in Node because `playwright-core` is a
  JavaScript library. The daemon stays where it is; only the launcher moves.
- The per-command client stays in Node too. It is reached by spawning `run.js`,
  exactly as the shell launcher does today, so the wire protocol keeps a single
  implementation.
- The port is subtractive in its dependencies. It removes this path's runtime
  dependence on `jq`, `flock`, `sha256sum`, `nohup` and the bash 3.2 floor, and
  replaces the `accelerator config` shell-out with an in-process call. Whether
  it is subtractive in latency is a question for measurement, not for this
  record — the bash process goes away, but callers reach the path through the
  CLI's own launcher and dispatch instead.
- The daemon-state contract is preserved, not redesigned. The Rust launcher
  reads and writes the same files `lib/state.js` owns — the pid file, the info
  JSON carrying `start_time`, and the stopped sentinel — including the
  `LANG=C` convention under which that start time is recorded.
- Behaviour parity is pinned by characterization tests against the existing
  shell launcher before the shell path is deleted.
- The `browser-executor` resolver skill and its two agent call sites retire
  with the shell launcher they existed to locate.

Option 2 was rejected because a wrapper keeps every host prerequisite and the
bash floor while adding a process hop — it preserves the cost the migration
exists to remove.

Option 3 was rejected because it puts lifecycle control inside the thing being
controlled: the process deciding whether to daemonise would be the daemon's own
runtime, and a bootstrap failure would surface in JavaScript's vocabulary
rather than the CLI's own error reporting. Rust owning the process tree it
spawns keeps that reporting in one place. The choice is about where supervision
belongs, not about process count — a Node client starts per command either
way.

Option 4 was rejected because it would give the daemon protocol a second
implementation. The versioned JSON contract and the on-disk state contract
would both then span Rust and JavaScript, and `PROTOCOL` changes would need
matching moves on both sides. Speaking it from Rust would remove the
per-command Node start, which is a real gain, but not one worth a duplicated
wire contract in the same change that ports the launcher. It remains available
later, once the launcher port has settled.

## Consequences

### Positive

- Four host tools and the bash 3.2 floor leave this path, removing a class of
  macOS-only failures of exactly the kind the `LANG=C` fix addressed.
- The re-entrant `accelerator config` shell-out becomes an in-process call.
- Every browser command drops a bash process and the `jq`, `sha256sum` and
  `ps`/`date` children it spawned. The Node client remains, so this shortens
  the process chain rather than emptying it.
- Daemon supervision gains Rust's type checking and test seams. It does not
  become single-language: the daemon still records its own state from
  `lib/state.js`, so lifecycle logic spans Rust and JavaScript exactly as it
  spanned bash and JavaScript before.
- The `browser-executor` resolver skill and its two agent call sites retire,
  removing an indirection whose only purpose was locating a shell script.

### Negative

- The 203 lines being replaced encode hard-won fixes — locale-forced
  start-time parsing, a one-second drift tolerance, the `mkdir` lock fallback,
  killing a daemon that times out mid-bootstrap. Each regresses silently if
  the port misses it, which makes characterization tests a precondition of the
  port rather than a follow-up.
- We take ownership of per-platform PID liveness and process start-time
  reading (`/proc` on Linux, `ps` on macOS) in our own code, rather than
  leaning on tools the operating system ships.
- The daemon-state contract now spans Rust and JavaScript, so a change to the
  pid file, info JSON or stopped sentinel must be made on both sides.

### Neutral

- This supersedes the 2026-08-08 thin-wrapper disposition recorded in
  work-item:0196's drafting notes, not any accepted ADR.
- The per-command Node start survives this change untouched. Removing it means
  reproducing the client as well, which is option 4's territory and a decision
  for another day.
- The launcher's lock is process-local: a plain `flock` or `mkdir` with no
  owner sentinel, sharing no on-disk contract with `scripts/atomic-common.sh`.
  The port therefore has no cross-implementation lock protocol to honour.
- Extracted from ADR-0057, where this decision had been recorded as a neutral
  consequence of an unrelated choice about platform scope.

## References

- ADR-0045 (skills vs CLI division of labour), ADR-0046 (zero-setup static
  binary distribution), ADR-0048 (four-toolchain split), ADR-0049 (bash 3.2
  compatibility floor), ADR-0053 (thin CLI over a hexagonal core), ADR-0057
  (browser automation as a glibc-only capability)
- `meta/work/0196-accelerator-design-inventory-gap-tooling-cli.md`
- `meta/research/codebase/2026-08-10-0196-accelerator-design-inventory-gap-tooling-cli.md`
- `skills/design/inventory-design/scripts/playwright/run.sh` — the launcher
  being reproduced
