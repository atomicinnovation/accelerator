---
type: "work-item"
id: "0330"
title: "Tree leases are not held on the cold path or honoured when liveness is unknown"
date: "2026-10-11T15:00:35+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "low"
parent: "work-item:0136"
relates_to: ["work-item:0223", "work-item:0196"]
tags: ["runtime", "cache", "lease", "prune", "design"]
last_updated: "2026-10-11T15:00:35+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0330: Tree leases are not held on the cold path or honoured when liveness is unknown

**Kind**: Bug
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

Two defects let `accelerator cache prune` reclaim a vendored-runtime tree that a
live Playwright daemon is using. A daemon spawned after a cold `accelerator cache
ensure` holds no lease on its trees, and the reclaim and repair paths treat a
lease whose liveness is `Unknown` as free. Both contradict the code's own
documented contract.

## Context

Found during the 0223 spike. Every cache root is gated the same way: `prune`
takes the single-flight lock and skips any generation whose shared `flock` lease
is held. That gate is only as good as the leases behind it.

The defects matter only on a shared `ACCELERATOR_CACHE_DIR` root. On the default
per-version root, the tree in use is always the launcher's compiled-in digest,
which `prune` never reclaims. On a shared root, a newer launcher's `prune` can
reclaim an older version's digest once its claim is older than the 14-day window,
or immediately under `--older-than`.

## Requirements

Current behaviour:

1. **Cold path holds no lease.** `accelerator cache ensure` prints
   `name\t<tree>\t<lease>` lines with the contract "the caller holds the lease
   itself" (`launch/cache.rs:108-115`). `design-adapters` parses
   `ResolvedTree.lease` (`ensure.rs:102-118`), but nothing opens it, so the daemon
   spawned afterwards runs unprotected.
2. **`Unknown` counts as free.** `lease.rs:8-10` says that where `flock` is
   unavailable (for example NFS returning `ENOLCK`), liveness is reported as
   unknown "so a spuriously successful probe cannot reclaim a live daemon's
   tree". `reclaim_unclaimed` (`resolver.rs:759-762`) and `reap_superseded`
   (`resolver.rs:691-700`) compare only against `Liveness::Held`, so `Unknown` is
   reclaimed at once. `reap_orphans` (`reap.rs:162`) is correct, because its
   1h `AGE_BACKSTOP` follows the probe.
3. **Warm-path inheritance is unproven.** The launcher clears `FD_CLOEXEC` on the
   leases it takes (`lease.rs:172-179`), so the fd should reach the daemon
   through `accelerator-design` (`design-adapters/src/process.rs:139-220`). No
   test asserts it.

Expected behaviour:

- The design executor takes a shared lease on every `ResolvedTree.lease` returned
  by a cold `cache ensure` before it spawns the daemon, with `FD_CLOEXEC`
  cleared so the daemon inherits it, matching the warm path.
- `reclaim_unclaimed` and `reap_superseded` treat `Unknown` as not known to be
  free: they reclaim the generation only once its age reaches `AGE_BACKSTOP`.
- Once a daemon exits, its generations become reclaimable again.

## Acceptance Criteria

- [ ] Given trees materialised by a cold `accelerator cache ensure`, when the
      Playwright daemon is running, then `probe_liveness` reports `Held` for each
      consumed generation's lease.
- [ ] Given a warm design invocation, when the Playwright daemon is running, then
      `probe_liveness` reports `Held` for each consumed generation's lease.
- [ ] Given a running daemon from either path, when `accelerator cache prune
      --older-than 0` runs from a launcher with a different compiled-in digest,
      then none of the daemon's generations is removed.
- [ ] Given that daemon has exited, when the same `prune` runs, then its
      generations are reclaimed.
- [ ] Given a generation whose lease probe returns `Unknown` and whose age is
      under `AGE_BACKSTOP`, when `reclaim_unclaimed` or `reap_superseded` runs,
      then the generation survives; once its age reaches `AGE_BACKSTOP`, it is
      reclaimed.
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Open Questions

None blocking.

## Dependencies

- Blocked by: none.
- Blocks: none.
- Relates to: 0223 (the spike that found these defects) and 0196 (the
  vendored-runtime work that introduced the lease model).

## Assumptions

- The age check is the right fallback for `Unknown`, as `lease.rs:8-10`
  prescribes, rather than never reclaiming. On a filesystem without `flock`, a
  daemon older than 1h with a stale claim can still lose its tree; accepted
  because the alternative leaves such trees unreclaimable.

## Technical Notes

- `hold_shared_lease` lives in the launcher crate
  (`cli/launcher/src/launch/outbound/resolve/tree/lease.rs`), which
  `design-cli` cannot depend on. The lease primitive needs a home both crates can
  share.
- Rust's `std` opens files with `O_CLOEXEC`, so the executor must clear it
  explicitly, as the launcher does, for the daemon to inherit the lease.
- The executor must hold the leases across the client `exec` as well as the
  daemon spawn, as the warm path does.
- Testing the `Unknown` criterion needs a seam for injecting a liveness probe,
  because `ENOLCK` cannot be reproduced on a local filesystem.

## Drafting Notes

- The warm-path proof is in scope because the cross-launcher `prune` criterion is
  meaningless unless both paths are verified.
- Priority is low because only shared `ACCELERATOR_CACHE_DIR` roots are exposed.
- Out of scope: version-aware daemon reuse (design or visualiser), footprint
  reporting across sibling roots, and moving the tree root to
  `${CLAUDE_PLUGIN_DATA}` (rejected in 0223).

## References

- Source: `meta/work/0223-bounding-the-default-runtime-cache-root.md`
  (Technical Notes, "Lease safety on reclamation")
- `meta/decisions/ADR-0064-producer-signed-tree-attestation-over-a-compiled-in-digest.md`
  (shared `flock` lease and age backstop carried forward from ADR-0061)
- Related: 0196; 0223
