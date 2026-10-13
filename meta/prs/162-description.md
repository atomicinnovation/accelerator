---
type: "pr-description"
id: "162"
title: "[0334] Hold tree leases on the cold path and honour unknown liveness"
date: "2026-10-13T16:32:45+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "work-item:0334"
parent: "work-item:0334"
relates_to: ["work-item:0223", "work-item:0196", "work-item:0218"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/162"
pr_number: 162
tags: ["runtime", "cache", "lease", "prune", "design", "tree-lease"]
revision: "9fd99a5562ee72b4d29c53835cd8b74a90523e54"
repository: "accelerator"
last_updated: "2026-10-13T16:39:10+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0334] Hold tree leases on the cold path and honour unknown liveness

## Summary

A live design daemon's vendored-runtime trees could be reclaimed under it in
three ways:

- **Cold path:** no lease was ever held.
- **Warm path:** `DaemonSpawner` `dup2`s its identity pipe onto fd 3, which
  silently closed the driver lease the daemon inherited.
- **`Liveness::Unknown`:** `prune` and `repair` treated it as free.

This PR moves the shared lease into a `tree-lease` crate that keeps every
lease at fd 10 or above. The executor now leases each tree `cache ensure`
returns, and the reclaimers defer an `Unknown` generation to the age
backstop instead of removing it.

## Changes

- **`tree-lease` crate** (`cli/tree-lease`, technical library):
  - It owns `hold_shared_lease`, `SharedLease`, `LeaseError` and
    `observed_held`.
  - `hold_shared_lease` opens the sidecar, raises it with
    `fcntl_dupfd_cloexec` to `LOWEST_LEASE_DESCRIPTOR` (10), and only then
    takes `LOCK_SH`. The low descriptor is closed before the lock, which
    holds where NFS emulates `flock` with POSIX locks. It clears
    `FD_CLOEXEC` so the lease survives `exec`.
  - `observed_held` is the one held/free observation that every suite
    uses. An absent file reads as not held, and errors other than
    `EWOULDBLOCK` surface.
  - The crate is registered in the workspace, the crate-role register,
    `pup.ron` (`tree_lease_holds_leases_only`, with its probe pair) and
    `_EXEMPT_MEMBERS`.
  - `design-adapters` asserts `IDENTITY_FD < LOWEST_LEASE_DESCRIPTOR` at
    compile time, beside the descriptor layout it owns.
- **Launcher** keeps only `impl HeldLease for tree_lease::SharedLease` and
  `From<LeaseError> for TreeError`. The resolver's lease calls change only
  their path.
- **Cold-path leases** (`design-adapters/src/ensure.rs`,
  `design-cli/src/executor.rs`):
  - `TreeLeases::hold` checks that each lease path is the tree's own
    `.lease` sibling, takes the lease, then re-checks that the tree still
    exists.
  - A tree that has gone is reported as gone. Its lease file is removed
    only if nobody else holds it.
  - A tree that cannot be leased is skipped with one `warning:` line, so
    the daemon still runs where `flock` is unavailable.
  - 🔒 A lease path outside the tree layout is never created or removed,
    because it arrives on a subprocess's stdout.
  - `ensure_and_pin` runs `cache ensure` at most twice. A tree still gone
    after the second attempt downgrades with `artifact-unavailable` and
    writes no sticky marker. A failed second ensure is classified like a
    first.
  - `run` binds the leases in `PinnedResolution`, so they live through the
    daemon spawn and the client `exec`.
  - A new launcher test pins the contract the executor relies on: `cache
    ensure` prints each lease as its tree's `.lease` sibling.
- **Liveness seam** (`launcher/.../tree/lease.rs`): `TreeResolver` carries a
  `LivenessProbe`, which is `FlockProbe` in production. `reap_orphans`,
  `reap_superseded` and `reclaim_unclaimed` consult it rather than calling
  `probe_liveness` directly.
- **Honouring `Unknown`** (`launcher/.../tree/reap.rs`): three pure `const
  fn` rules sit side by side.

  | Reclaimer | `Held` | `Free` | `Unknown` |
  |---|---|---|---|
  | `reclaim_unclaimed` | keep | reclaim | reclaim once aged |
  | `reap_superseded` | keep | reclaim | defer, restarting the backstop |
  | `reap_orphans` | keep | reclaim once aged | reclaim once aged |

  - The backstop counts from the generation's mtime, which is set at
    extraction. A deferred superseded generation would otherwise get no
    grace. `restart_backstop` therefore sets its mtime from the injected
    clock (`utimensat`, best-effort), so a live daemon on an old tree gets
    a full `AGE_BACKSTOP` (1h).
  - `reap_orphans` now removes a reclaimed generation's `.sealed`,
    `.sealed.sig` and `.lease` sidecars, so a deferred generation is
    cleaned up completely once it ages.
- **Test fixtures:**
  - `design-cli-fixture-warm-handover` reproduces the launcher's warm
    handover in a process whose only open descriptors are 0-2.
  - `launcher-fixture-lease-holder` stands in for a detached daemon.
  - Both close every inherited descriptor (found by listing `/dev/fd`)
    before taking their own leases. Lease descriptors are deliberately
    inheritable, and tests run as threads of one process, so a lease could
    otherwise leak between parallel tests.
- **Process.** The repository's `implement-plan` instructions now ask for
  small, focused commits.

## Context

- Work item: `meta/work/0334-tree-leases-are-not-held-on-the-cold-path-or-honoured-when.md`.
  It was captured as 0330 during the 0223 spike, and renumbered because trunk
  had already assigned 0330.
- Plan: `meta/plans/2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness.md`,
  with its review under `meta/reviews/plans/`
- Codebase research: `meta/research/codebase/2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness.md`
- Validation (result `pass`): `meta/validations/2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness-validation.md`
- Origin: the 0223 spike (`meta/work/0223-bounding-the-default-runtime-cache-root.md`),
  closed here with ADR-0063 standing
- Decisions: ADR-0061, ADR-0064, ADR-0069

### Acceptance-criteria map

| Criterion | Proven by |
|---|---|
| `tree-lease` exists; both depend; launcher no longer defines | Phase 1 checks, `crate-dependencies`, `pup` probe pair |
| Cold invocation: leases held after client exit | `a_cold_daemon_holds_every_lease_after_the_client_exits`, `a_driver_exported_without_its_browser_takes_the_cold_path_and_holds_both` |
| Warm invocation: leases held after client exit | `a_warm_daemon_holds_every_lease_after_the_client_exits` |
| Cross-launcher `prune` spares a live daemon's generations | The warm/cold tests above, plus `a_second_launcher_spares_generations_a_live_daemon_holds` |
| Ensured but no daemon: removed | `a_second_launcher_reclaims_generations_no_daemon_holds` |
| Daemon group terminated: removed | The warm/cold tests' post-`killpg` assertions, plus `a_second_launcher_reclaims_once_the_daemon_group_has_exited` |
| `reclaim_unclaimed` table | Five table tests, plus `an_injected_held_probe_spares_a_stale_sibling_in_prune` |
| `repair` entry point passes the probe through | `a_superseded_unknown_generation_is_kept` |
| `reap_superseded` and later `reap_orphans` | `a_kept_unknown_superseded_generation_is_reaped_once_aged`, `an_old_unknown_superseded_generation_is_kept_for_a_full_backstop` and the superseded table |
| `prune` entry point passes the probe through | `an_unclaimed_unknown_generation_is_kept_while_young` |

## Testing

- [x] `mise run` exits 0 end to end on `86bbe994`, with all 5,357 CLI tests
      passing. The branch was then rebased onto #161, which changes no code
      on this path (a `pre.79` → `pre.80` version bump and work items), and
      `frontmatter_goldens` passes again on the rebased tip.
- [x] Warm-path red run: before the descriptor floor,
      `a_warm_daemon_holds_every_lease_after_the_client_exits` failed on the
      driver lease alone, which confirms the fd 3 clobber. It passes after.
- [x] Six cold-path `executor_runtime` tests drive the real executor,
      `DaemonSpawner` and `ExecClient` against a stub `node` and a fake
      launcher. They cover:
  - an empty cache root, and a driver exported without its browser;
  - an unleasable tree: a single warning, no retry;
  - a tree reclaimed before its lease, and one still gone after a second
    ensure;
  - a failed second ensure.
- [x] Eight `tree_leases` tests, in a binary that spawns no children, cover:
  - held, unleasable and gone trees, and a cache that has gone;
  - a lease another holder keeps;
  - an unstattable tree (skipped as root);
  - both halves of the outside-the-layout rule.
- [x] `reap.rs` unit tests cover every cell of the three rules, the backstop
      boundary at `AGE_BACKSTOP - 1` and `AGE_BACKSTOP`, a backstop restart
      from the injected clock, and sidecar removal.
- [x] Cross-launcher tests use two in-process resolvers expecting different
      digests and a detached lease-holder process. Passing no lease paths
      to the holder turns the spares test red.
- [x] Manual, macOS arm64 with Playwright 1.55.1: after a `navigate`,
      `lsof` showed both leases held only by the daemon, on fds 10 and 11,
      and none by Chromium's four processes. No follow-up for daemon
      descendants is needed.
- [x] Manual, cross-version `prune` on a scratch `ACCELERATOR_CACHE_DIR`,
      with two real launchers whose driver digests differ. The installed
      `v1.24.0-pre.78` launcher predates the Node bump and pins driver
      `553e…`; this branch pins `5f4b…`.
  - A cold `design executor ping` through a dev launcher and
    `accelerator-design` built from this branch materialised both pre.80
    trees from the signed release and started the daemon. `lsof` showed
    both leases held only by the daemon's `node`, on fds 10 and 11.
  - pre.78's `cache prune --older-than 0` on the same root reclaimed 0
    entries while the daemon ran.
  - After `kill -TERM -- -<pgid>`, both leases read free, and the same
    prune reclaimed the branch's driver generation and its pointer.
    pre.78 kept the browser generation, whose digest it shares.

## Notes for Reviewers

- ⚠️ **Scope beyond the work item.** The warm-path fix was bounded to
  clearing `FD_CLOEXEC`. The descriptor floor fixes warm and cold alike, and
  the change was approved during planning and is recorded on 0334.
- **Accepted race.** Every reclaimer probes the lease, releases the probe,
  then removes the generation. A lease taken in that window still sees the
  directory. The cold path's re-check after leasing narrows the gap left by
  `cache ensure`'s hit path, which does not refresh the claim. It does not
  close the race, and the launcher's own `acquire` shares the same window.
- **Backstop limits.** The restart only applies to generations `repair`
  defers, and `utimensat` may be rejected on network filesystems.
  `reclaim_unclaimed`'s `Unknown` gate still counts from extraction, so the
  claim window is what protects a pointed-at generation there.
- **Places worth a close look:** `TreeLeases::hold` (`ensure.rs`),
  `ensure_and_pin` (`executor.rs`), and the descriptor-closing code in both
  fixtures (`Dir::new`, never `Dir::read_from`, so the listing's own
  descriptor is never closed twice).
- **Migration.** None. A daemon started by a launcher without this fix holds
  no cold-path lease, and stays exposed to a fixed launcher's `prune` on a
  shared root, as the work item accepts.
