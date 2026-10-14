---
type: "codebase-research"
id: "2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness"
title: "Tree leases on the cold path and under Unknown liveness (work item 0334)"
date: "2026-10-12T02:54:33+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0334"
parent: "work-item:0334"
topic: "Tree leases on the cold path and under Unknown liveness (work item 0334)"
tags: ["research", "codebase", "launcher", "design-adapters", "design-cli", "tree-lease", "lease", "prune", "repair", "cache"]
revision: "7d85640a867ac7ac968b4ab359e832e8edb9d223"
repository: "accelerator"
last_updated: "2026-10-12T02:54:33+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Tree leases on the cold path and under Unknown liveness (work item 0334)

**Date**: 2026-10-12 02:54 UTC
**Author**: Toby Clemson
**Git Commit**: 7d85640a867ac7ac968b4ab359e832e8edb9d223
**Branch**: detached working copy (jj change `xqzvtuunrwvn`), not on trunk
**Repository**: accelerator

## Research Question

What does the codebase look like around work item 0334? The item covers three
things: extracting the lease primitive into a `tree-lease` crate, holding
leases on the executor's cold path, and treating `Liveness::Unknown` as
not-free in `reclaim_unclaimed` and `reap_superseded`. This research maps the
code, the tests and the registration surface those changes touch, and surfaces
anything the work item does not already account for.

## Summary

The work item's diagnosis is accurate on every count:

- **Cold path:** `materialise` returns no held lease, and nothing reads
  `ResolvedTree.lease`.
- **`Unknown` liveness:** `reclaim_unclaimed` and `reap_superseded` compare only
  against `Liveness::Held`.
- **Probe seam:** there is none for liveness. `probe_liveness` is called directly
  at three sites.
- **Contract:** the 0196 plan specified both the cold-path lease and the
  `Unknown`-to-backstop rule. It deferred the cold-path lease for verification
  reasons only, so neither defect was a design choice.

Three findings go beyond the work item:

1. ⚠️ **Fd 3 hazard.** `DaemonSpawner` `dup2`s the identity pipe onto a fixed
   fd 3 in the child (`IDENTITY_FD`, `cli/design-adapters/src/process.rs:79`,
   `:172`). If a lease occupies fd 3 in the parent, the daemon silently loses
   that lease. On the warm path the launcher's first lease is the first fd it
   keeps open, so it is likely to be fd 3. This is the most plausible way the
   "warm-path inheritance is unproven" test could fail. A cold-path lease opened
   before the identity pipe has the same exposure. The hypothesis comes from
   reading the code and has not been checked by running it.
2. **Template crate.** `process-probe` is only a partial precedent for
   `tree-lease`: it has no `pup.ron` rule and no probe pair.
   `remote-projection` is the better template for those two checklist items.
3. **Launcher-only types.** `SharedLease` is coupled to the launcher-only
   `TreeError` and `HeldLease`. Extraction needs its own error type in
   `tree-lease`. The `impl HeldLease for SharedLease` stays in the launcher,
   which the orphan rule permits because the trait is local.

## Detailed Findings

### Lease primitive (`lease.rs`)

- **Dependencies:** `rustix` only, using `fs::{flock, FlockOperation}` and
  `io::{fcntl_getfd, fcntl_setfd, FdFlags, Errno}`
  (`cli/launcher/src/launch/outbound/resolve/tree/lease.rs:25-26`). The
  workspace `rustix` has `["process", "fs"]` (`cli/Cargo.toml:128`).
- **`SharedLease { file: File }`** (`lease.rs:33-38`) has a test-only `file()`
  accessor (`:40-45`) and `impl HeldLease` (`:47`). The `HeldLease` marker trait
  and `TreeError::Lease` live in `cli/launcher/src/launch/core/tree.rs:279` and
  `:62-64`.
- **`hold_shared_lease`** (`lease.rs:56-72`) opens read/write/create with mode
  `0600`, takes a blocking `LockShared`, then calls `clear_cloexec`
  (`:172-179`). Errors map to `TreeError::Lease` via `lease()` (`:181-185`).
- **`probe_liveness`** (`lease.rs:91-110`) probes as follows:

  | Condition | Result |
  |---|---|
  | `NotFound` on open | `Free` |
  | Any other open error | `Unknown` |
  | `LOCK_EX\|LOCK_NB` succeeds | `Free` |
  | `LOCK_EX\|LOCK_NB` fails with `EWOULDBLOCK` | `Held` |
  | `LOCK_EX\|LOCK_NB` fails with any other errno | `Unknown` |

  A lease file that cannot be opened (`chmod 000`, non-root) therefore already
  yields `Unknown`. That is a seam-free way to exercise the probe in one or two
  tests, but it does not replace the injection seam the work item requires.
- **Single-flight:** `take_single_flight` and `try_single_flight`
  (`lease.rs:125-157`) stay in the launcher, per the work item.
- **Non-unix stubs:** `lease.rs:261-302`. The `probe_liveness` stub returns
  `Unknown`.
- **Call sites:** the launcher crate only.
  - `hold_shared_lease`: `resolver.rs:318`, `:527`.
  - `probe_liveness`: `reap.rs:162`, `resolver.rs:693`, `:759`.
  - `Liveness` and `ProcessLiveness` in `cli/design*` are an unrelated pid
    probe.
- **cargo-pup context:** `cli/.pup/accelerator_context.json:469-473` records
  `SharedLease` as the sole `HeldLease` implementor, so this file changes when
  the type moves.

### Reclamation paths (`resolver.rs`, `reap.rs`)

Three paths can reclaim a generation:

| Path | Entry | Gate today | `Unknown` handling |
|---|---|---|---|
| `reap_orphans` | `materialise`, `prune` | unreferenced, not `Held`, mtime ≥ `AGE_BACKSTOP` | ✅ age backstop (`reap.rs:152-166`) |
| `reclaim_unclaimed` | `prune` | not own digest, no claim in window, not `Held` | ❌ reclaimed at once (`resolver.rs:759-762`) |
| `reap_superseded` | `repair` | old ≠ new path, not `Held` | ❌ reclaimed at once (`resolver.rs:691-700`) |

- **`TreeResolver<'a>`** (`resolver.rs:100-120`) is a struct with all-`pub`
  fields and no constructor.
  - Production literals: `cli/launcher/src/main.rs:330-342` (`run_cache`) and
    `:422-434` (warm `acquire_consumed_trees`).
  - Test literals: `Harness::resolver_with` (`cli/launcher/tests/tree_resolution.rs:266-292`)
    plus ad-hoc ones at `:1263`, `:1306`, `:1607`.
  - Adding a liveness field means touching every literal. That argues for a
    `&dyn` port shaped like `clock: &dyn Clock` and `steps: &dyn StepObserver`.
- **Existing seams:** `clock`, `steps`, `expected_digests`
  (`ExpectedDigests::Fixed`, `resolver.rs:79-97`) and `fetcher`.
- **`reap_superseded`** is an associated function with no `self`, so the
  injected probe must be passed in as a parameter or the function made a
  method. It runs after `materialise_fresh` publishes the new pointer
  (`resolver.rs:638-687`), so the superseded generation is already an orphan for
  `reap_orphans`. This confirms the work item's Technical Note.
- **`reap_orphans`** takes `clock` as a parameter (`reap.rs:41`) and calls
  `probe_liveness` directly (`:162`). An injected probe must reach it as well,
  or the "later `reap_orphans` reclaims the kept `Unknown` generation" criterion
  will run against the real probe. That still works, because a real lease file
  probes as `Free` and `Free` also falls through to the age check.
- **Constants:**
  - `AGE_BACKSTOP` (1h) is private to `reap.rs:28`. `reclaim_unclaimed` needs
    it, so it must become at least `pub(super)`.
  - `CLAIM_WINDOW` (14 days) is `pub` at `claims.rs:20`.
- **Clock semantics:** age is the clock's `now_seconds()` minus the real file
  mtime (`reap.rs:172-180`). A clock before the mtime reads as "not old", so
  `StoppedClock` (0) never ages anything. To place a generation either side of
  the backstop, set the clock to real now, or to real now plus 2×`AGE_BACKSTOP`,
  as `reap.rs` tests do with `AtClock`, `now()` and `well_aged()`
  (`reap.rs:225-239`). `reclaim_unclaimed`'s claim-window check reads the same
  clock, so the test needs `older_than = 0` or stale claims to make the
  generation eligible regardless of the clock.

### Cold path: why no lease survives

```text
executor ensure_runtime ──spawn──▶ accelerator cache ensure
                                     │ materialise()
                                     │   locate hit / reuse hit: no lease
                                     │   materialise_fresh: _lease dropped at return
                                     ▼
                         stdout "name\t<tree>\t<lease>"  (process exits)
executor ◀──parse_resolved── ResolvedTree{path, lease}  (lease never read)
   │
   ├─ DaemonSpawner::spawn  (fork+exec, setsid)   ← daemon: no lease
   └─ ExecClient::run       (execvp)              ← client: no lease
```

- **Inside `cache ensure`:** `materialise` returns a bare `SealedTree` on its
  locate hits (`resolver.rs:584-601`). `materialise_locked`'s reuse hit takes no
  lease (`:194-198`). `materialise_fresh` holds `_lease` only until it returns
  (`:318`, `:337`).
- **Output contract:** `cli/launcher/src/launch/cache.rs:105-116` prints
  `name\tpath\tlease_path`, with the comment "The caller holds the lease itself".
- **Parsing:** `ensure` runs the launcher via `Command::output()`
  (`cli/design-adapters/src/ensure.rs:83-98`). `parse_resolved` (`:102-118`)
  builds `ResolvedTree { artifact, path, lease }` (`:21-26`).
- **Unused lease:** `ensure_runtime` uses only `.path`
  (`cli/design-cli/src/executor.rs:290-313`). `ResolvedTree.lease` has no reader
  in any crate.
- **Where the holder must live:** `Ensured` (`executor.rs:96-99`, created at
  `:238`) is a local of `resolve_runtime`, which is dropped before `act_on`. The
  leases must be returned out of `resolve_runtime` or owned by `run`
  (`executor.rs:107-175`), so they stay alive through `act_on` → `launch`
  (`:346-471`). After a successful `exec`, no destructor runs.
- **Mixed trigger:** with a driver exported but no browser
  (`executor.rs:268-278`), control falls through to the cold path with
  `trees = ["driver", "browser"]`. Both trees are re-ensured, so the driver is
  leased twice, once inherited and once taken by the executor. The work item
  already expects this.
- **Reused daemon:** in `Launcher::launch`, when an existing daemon is reused
  (`cli/design/src/executor/launch.rs:90-92`, `:103-106`), only the client
  inherits the new leases. The work item handles this by requiring that no
  daemon is running at the start of each criterion.

### Warm path and fd inheritance

- **Launcher side:** `run` binds `_tree_leases = export_consumed_trees(...)`
  (`cli/launcher/src/main.rs:444`) and holds them across `dispatch`'s exec. The
  exec is `UnixExec`, `Command::exec` with no fd closing
  (`cli/launcher/src/launch/outbound/exec.rs:14-22`). `acquire` does `locate` →
  `hold_shared_lease` → `(dev, ino)` re-check → retry once
  (`resolver.rs:509-550`).
- **`DaemonSpawner::spawn`** (`cli/design-adapters/src/process.rs:139-221`):
  - It sets `env_clear` plus an allowlist, so `ACCELERATOR_TREE_*` never reaches
    the daemon's environment. That does not matter for leases.
  - Its `pre_exec` calls `setsid`, `umask(0o077)` and `dup2(read_fd, 3)`.
  - Nothing closes inherited fds: the workspace has no `close_range` or
    `closefrom`. Every non-`CLOEXEC` fd reaches the daemon except whatever sat
    on fd 3.
- ⚠️ **Fd 3 clobber.** `dup2(read_fd, IDENTITY_FD)` with `IDENTITY_FD = 3`
  closes whatever the child inherited on fd 3. The sequence that puts a lease
  there:
  1. The launcher's `acquire` opens and closes transient fds.
  2. It then keeps the first lease, which takes the lowest free descriptor.
     Stdio occupies 0-2 and `kernel::logging` keeps no file open, so that is
     probably fd 3.
  3. The `exec` into `accelerator-design` preserves fd 3.

  The client keeps the lease, because `ExecClient` execs without `pre_exec`. The
  daemon loses the lease for whichever artifact was acquired first, probably
  `driver`. Two fixes are possible: make `tree-lease` move its fd above a floor
  (for example `F_DUPFD` with minimum 10), or have the spawner choose its
  identity fd so it avoids inherited ones. Either way, the warm-path proof test
  is likely to go red first. The work item allows a fix only for clearing
  `FD_CLOEXEC`, so a fix for this would fall outside that bound.
- **`ExecClient`** (`process.rs:304-327`) keeps the full environment and calls
  `command.exec()`. Non-`CLOEXEC` fds survive into Node.
- **Executor lock:** `cli/design-adapters/src/lock.rs` relies on std's
  `O_CLOEXEC` so `launcher.lock` does not leak into the daemon (`lock.rs:6-9`).
  The 0196 plan requires the lease and this lock to stay separate disciplines
  (plan `:1727-1730`).

### `tree-lease` crate extraction

**Role:** `technical-library` with no context, per ADR-0069 roles (`:72-82`).
Both new edges pass `tasks/lint/crate_dependencies.py`:

- **`launcher` → `tree-lease`:** `_launcher_rules` (`:361-366`) rejects only
  composition roots and non-platform contexts.
- **`design-adapters` → `tree-lease`:** `_adapter_rules` (`:352-358`) passes,
  because `_upstream` returns `[]` for a target with no context.

The constraint runs inward: a technical library may depend only on `kernel` and
other technical libraries (`:378-383`). That is why `TreeError` and `HeldLease`
cannot come along with the moved code.

**Checklist** (`tasks/README.md:789-853`), with the concrete edit for
`tree-lease`:

| Item | Edit | Template |
|---|---|---|
| Workspace membership | `cli/Cargo.toml` members, sync lock via `cargo metadata` | `process-probe` (`cli/Cargo.toml:46`) |
| Inherited fields + `[lints]` | `cli/tree-lease/Cargo.toml` | `cli/process-probe/Cargo.toml` |
| Role declaration | `[package.metadata.accelerator] role = "technical-library"`; row in `tests/unit/tasks/test_crate_dependencies.py:405-448` | `process-probe` row `:430` |
| `pup.ron` rule | `denied`-only rule over `^tree_lease($\|::)`, plus add `tree_lease` to the regex at `cli/pup.ron:443` | `remote_projection_extracts_json_only` (`cli/pup.ron:203-221`) |
| Probe pair | violation + compliant control in `tests/integration/pup/test_import_rule.py` | `:916-993` (`_write_shared_crate_probe`) |
| Public API classification | `_EXEMPT_MEMBERS` with a reason | `process-probe` entry `tasks/public_api.py:84-88` |

Other changes outside the checklist:

- **Run `mise run deny:check`.** No `deny.toml` edit is expected.
- **Optionally** add `tree-lease` to the ADR-0069 technical-library row
  (`:79`).
- **New `rustix` edge:** `design-adapters` uses `libc` directly (`lock.rs:58`,
  `process.rs:263`), so depending on `tree-lease` brings `rustix` into its
  graph. It is the same version, so `multiple-versions = "deny"`
  (`cli/deny.toml:117`) still holds.
- **Launcher feature-graph guard:** `tests/integration/deny/test_launcher_feature_graph.py`
  runs against the launcher's new edge. `rustix` adds nothing that guard bans.
- **Core import rule:** `launch_core_imports_only_permitted`
  (`cli/pup.ron:25-39`) means only outbound launcher code may name `tree_lease`.
  The core port remains `HeldLease`.
- **Moved tests:** the `lease.rs` unit tests for held/free, shared concurrency
  and `CLOEXEC`-cleared (`:187-258`, `cloexec_set` helper `:198-202`) move with
  the code. `tempfile` comes along as a dev-dependency.

### Test infrastructure available

- **Launcher integration:** `cli/launcher/tests/tree_resolution.rs`.
  - Harnesses: `happy_harness`, `Harness::resolver_with`, `run_cache`.
  - Clocks: `StoppedClock`, `FixedClock`, `AdvancingClock`.
  - `plant_sibling_generation` (`:853-869`) plants a generation for a fabricated
    digest with a `.ref` but no lease file.
  - Lease-relevant tests:
    - `prune_spares_a_generation_whose_lease_is_still_held` (`:1054-1085`)
    - `a_repair_spares_a_superseded_generation_whose_lease_is_held`
      (`:1482-1512`)
    - `two_launchers_with_the_same_digest_share_one_generation_offline`
      (`:1241-1289`): a second resolver literal. This is the nearest thing to
      the two-digest launcher fixture.
  - All lease holding is in-process. No test checks across processes or for
    `Unknown`.
- **Reap and claims units:** `reap.rs:211-347` and `claims.rs:148-257` use the
  `AtClock` pattern. They have no lease or `Unknown` cases. `resolver.rs` has no
  in-file tests.
- **Design adapters:**
  - `cli/design-adapters/tests/spawn_properties.rs` drives the real
    `DaemonSpawner` with a `/bin/sh` stub. The stub reads fd 3 via `cat <&3`, a
    ready pattern for asserting fd inheritance. It reads `pgid` via `ps`. Its
    60s `await_content` bound applies because the `setsid` child cannot be
    waited on.
  - `ensure.rs:222-231` has `fake_launcher`, a shell-script `accelerator` that
    prints canned `cache ensure` output.
- **Design CLI:** `cli/design-cli/tests/executor_preflight.rs` runs the real
  `accelerator-design executor` binary with a controlled `PATH`, `HOME` and
  `ACCELERATOR_PLUGIN_ROOT`.
  - No test drives the cold path through spawn and exec with a fake launcher via
    `ACCELERATOR_LAUNCHER_PATH`.
  - No test covers `ExecClient`. The file's header (`:3-7`) defers post-exec
    properties to an opt-in lane with a real runtime. This matters because the
    work item's end-to-end criteria need the real `DaemonSpawner` and
    `ExecClient` with a stub daemon.
- **Domain fakes:** `cli/design/src/executor/launch.rs:199-768` has fakes for
  every executor port, in case the lease becomes a domain port. The work item
  does not require that.

## Code References

- `cli/launcher/src/launch/outbound/resolve/tree/lease.rs:33-72` – `SharedLease`, `hold_shared_lease`
- `cli/launcher/src/launch/outbound/resolve/tree/lease.rs:75-110` – `Liveness`, `probe_liveness`
- `cli/launcher/src/launch/outbound/resolve/tree/lease.rs:172-179` – `clear_cloexec`
- `cli/launcher/src/launch/core/tree.rs:62-64,279,363-368` – `TreeError::Lease`, `HeldLease`, `Clock`
- `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:100-120` – `TreeResolver` fields
- `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:318,337` – cold materialise lease dropped at return
- `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:509-550` – warm `acquire`
- `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:638-700` – `repair`, `reap_superseded`
- `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:713-774` – `prune`, `reclaim_unclaimed`
- `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:28,152-180` – `AGE_BACKSTOP`, backstop rule
- `cli/launcher/src/launch/cache.rs:105-116` – `cache ensure` output contract
- `cli/launcher/src/main.rs:380-444` – warm export and `_tree_leases`
- `cli/design-adapters/src/ensure.rs:21-26,83-118` – `ResolvedTree`, `ensure`, `parse_resolved`
- `cli/design-adapters/src/process.rs:79,139-221` – `IDENTITY_FD`, `DaemonSpawner::spawn`
- `cli/design-adapters/src/process.rs:304-327` – `ExecClient`
- `cli/design-cli/src/executor.rs:223-327` – `resolve_runtime`, `ensure_runtime`
- `cli/design-cli/src/executor.rs:346-471` – `act_on`, `launch`
- `cli/design/src/executor/launch.rs:85-136` – spawn and reuse sequence
- `tasks/README.md:789-853` – library-crate checklist
- `tasks/lint/crate_dependencies.py:352-383` – role rules
- `cli/pup.ron:203-221,443` – template rule, catch-all regex
- `tests/integration/pup/test_import_rule.py:916-993` – template probe pair
- `tasks/public_api.py:84-88` – `process-probe` exemption

## Architecture Insights

- **Kernel as oracle.** A `flock` is per open file description, so liveness
  needs no pid bookkeeping and a crashed holder releases without cleanup
  (ADR-0061 `:135-144`). ADR-0061 rejected a pid-and-start-time gate (`:173-176`).
  That is why the fix is to hold the lease, not to record who uses a tree.
- **Fd inheritance as transport.** The lease is a non-`CLOEXEC` fd carried
  across `exec` and `fork`. Anything that rearranges descriptors in a child is
  part of the lease contract: `dup2` onto fixed numbers, a future `close_range`,
  or Playwright launching browsers.
- **Hexagonal seams on `TreeResolver`.** Injection uses `&dyn` trait fields
  (`Clock`, `StepObserver`) and test enums (`ExpectedDigests::Fixed`). A
  liveness probe port fits this pattern. `reap_orphans` and `reap_superseded`
  are free and associated functions that take collaborators as parameters, so
  the probe must be threaded through them as well.
- **Crate-role lattice.** Technical libraries sit beneath every other role, so
  a primitive two composition roots share must drop its dependence on either
  root's types. This shapes `tree-lease`'s error type.

## Historical Context

- `meta/plans/2026-08-11-0196-design-vendored-runtime-distribution.md`:
  - `:1700-1710` specifies the cold-path lease: `accelerator-design` opens and
    `LOCK_SH`es the printed lease path before spawning.
  - `:1712-1719` and `:1734-1738` specify the rule that an `Unknown` result
    falls through to the age backstop.
  - Phase 3 deviation 1 (`:494-500`) deferred the cold-path lease because it was
    "only end-to-end verifiable with a real daemon and a concurrent
    `cache prune`".
  - Unticked criteria `:2244-2249` are effectively inherited by 0334.
- `meta/decisions/ADR-0061-signed-content-addressed-tree-generations.md` explains
  why leases exist: Chromium lazily opens `.pak`, locale and ICU files
  (`:61-66`). ⚠️ `:145-146` limits the age backstop to lease-less generations,
  whereas the 0196 plan, the implementation and 0334 apply it to `Unknown` too.
  The 0334 review settled in pass 3 that no ADR amendment is needed.
- `meta/decisions/ADR-0064-producer-signed-tree-attestation-over-a-compiled-in-digest.md:91-94`
  carries the lease and backstop forward unchanged.
- `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`
  covers roles (`:72-82`) and rules 4-8 (`:106-128`). These place `tree-lease`
  as a technical library.
- `meta/work/0223-bounding-the-default-runtime-cache-root.md:132-143` is the
  spike note that handed both defects over.
- `meta/reviews/work/0334-tree-leases-are-not-held-on-the-cold-path-or-honoured-when-review-1.md`
  ends in APPROVE after pass 5. Pass 5 widened the `repair` exposure to every
  root and recast the `reclaim_unclaimed` change as a narrow mitigation.
- `meta/plans/2026-08-11-0196-design-cli-migration.md:218-225,1055-1068` records
  the `process-probe` extraction and why it got no `pup` surface.

## Related Research

- `meta/research/codebase/2026-08-11-0196-design-cli-implementation-surface.md`
- `meta/research/codebase/2026-08-10-0196-accelerator-design-inventory-gap-tooling-cli.md`
- `meta/research/codebase/2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee.md`

## Open Questions

- ❓ **Fd 3 clobber.** Does the warm daemon lose the lease on fd 3? A spawner
  test with a stub that probes its own fds, or the work item's warm-path
  criterion, will answer it. If it does, decide whether the fix extends the work
  item's warm-path scope bound or becomes a follow-up. The cold-path lease
  should avoid fd 3 regardless.
- **Seam shape.** The liveness seam could be a `&dyn` port on `TreeResolver`
  threaded into `reap_orphans` and `reap_superseded`, or a function pointer.
  This research recommends the port, for consistency with `Clock`.
- **Two-digest fixture.** Can the end-to-end cross-launcher criteria use the
  real `accelerator cache prune` binary with `ExpectedDigests::Fixed`? That enum
  is only reachable in-process, so these tests would need either in-process
  `prune` against a lease held by a real spawned stub daemon, or a test-only
  pin override in the binary.
- **Descendant fd retention.** The work item's open question, whether
  Playwright's browser processes keep inherited fds, remains unanswered. No
  code in the workspace closes fds in descendants.
