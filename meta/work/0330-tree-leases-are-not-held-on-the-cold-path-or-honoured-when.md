---
type: "work-item"
id: "0330"
title: "Tree leases are not held on the cold path or honoured when liveness is unknown"
date: "2026-10-11T15:00:35+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "ready"
kind: "bug"
priority: "low"
parent: "work-item:0136"
relates_to: ["work-item:0223", "work-item:0196", "work-item:0218"]
tags: ["runtime", "cache", "lease", "prune", "design"]
last_updated: "2026-10-13T09:00:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0330: Tree leases are not held on the cold path or honoured when liveness is unknown

**Kind**: Bug
**Status**: Ready
**Priority**: Low
**Author**: Toby Clemson

## Summary

Two defects let the cache remove a vendored-runtime tree that a live design
daemon (the Playwright daemon the design executor spawns) is using. A daemon
spawned after a cold `accelerator cache ensure` holds no lease on its trees, so
`accelerator cache prune` can reclaim them. And `reclaim_unclaimed` (in `prune`)
and `reap_superseded` (in `accelerator cache repair`) treat a lease whose
liveness is `Unknown` as free. Both
contradict the code's own documented contract. The fix moves the lease primitive
into a shared `tree-lease` crate so the design executor can hold leases. It
stops `reap_superseded` reclaiming an `Unknown` generation at all. For
`reclaim_unclaimed` it is a narrow mitigation, not a full fix: an `Unknown`
generation is kept only until its mtime age reaches `AGE_BACKSTOP` (1h). It
also verifies by test that the warm path passes its leases to the daemon, and
fixes the warm path if it does not.

## Context

Found during the 0223 spike. Every cache root is gated the same way: `prune`
takes the artifact's single-flight lock (the `flock` that serialises
materialisation and reclamation) and skips any generation whose shared `flock`
lease is held. That gate is only as good as the leases behind it.

Terms used throughout: an **artifact** is a named runtime component (`driver`,
`browser`); a **tree** is an artifact's materialised directory; a **lease
file** is the `.lease` path beside it; a **lease** is a shared `flock` held on
that file. **The daemon** always means the design daemon.

A **generation** is one materialised tree directory for an artifact's digest. A
`.ref` pointer names the generation currently serving a digest, and each
generation has a sibling lease file. A **superseded** generation is the one
a digest's pointer named before a fresh materialisation replaced it. An
**orphan** is a generation no pointer names. **The daemon's generations** are
those whose trees the daemon was started from.

`probe_liveness` reports a lease as `Held` (a process holds it), `Free` (no
process holds it) or `Unknown` (the filesystem cannot say).

A **claim** records that an installed launcher expects a digest. `prune`
reclaims a generation whose digest has no claim within `CLAIM_WINDOW` (14
days), or within `--older-than` when given.

A **design invocation** is one run of an `accelerator design` command that
needs the vendored runtime. It resolves its trees by one of two paths:

- **Warm path**: for each artifact whose tree is already materialised and
  passes verification against its attested digest, the
  launcher takes the tree's lease, clears `FD_CLOEXEC`, exports
  `ACCELERATOR_TREE_<NAME>`, and `exec`s into the `accelerator-design` executor
  (`design-cli/src/executor.rs`). The executor takes this path when
  `ACCELERATOR_TREE_DRIVER` is set and, unless a browser override is in effect,
  `ACCELERATOR_TREE_BROWSER` too. A browser override is a user-supplied
  browser path, which removes the need for the browser tree.
- **Cold path**: otherwise (for example an empty cache root, or a driver
  exported without its browser), the executor runs `accelerator cache ensure` as
  a subprocess, which materialises the trees and prints their paths and lease
  files. When an earlier `cache ensure` failed in a way the executor classifies
  as persistent, the executor records a sticky-failure marker. While that marker
  stands, the executor skips the cold path, reports a downgrade (the design
  command runs without the vendored runtime) and spawns no daemon, so no lease
  is needed.

Up to three kinds of process can hold the leases:

- **The executor client**: after spawning the daemon, the executor `exec`s,
  through `ExecClient`, into the client that sends the current command to the
  daemon. The process that took (on the cold path) or received (on the warm
  path) the leases therefore keeps holding them as the client until the
  command finishes.
- **The daemon**: inherits the leases at spawn.
  It is long-lived, detached with `setsid`, and reused by later invocations.
- **Daemon descendants**: such as the browser processes Playwright launches.

The `prune` exposure exists only on a shared `ACCELERATOR_CACHE_DIR` root. On
the default per-version root, the tree in use is always the launcher's
compiled-in digest, which `prune` never reclaims. On a shared root, a newer
launcher's `prune` can reclaim an older version's digest once its claim falls
outside the window.

The `repair` exposure exists on any root, but only where `flock` is
unavailable. `accelerator cache repair` (with `--force`, or when the current
generation fails verification) materialises a fresh generation and calls
`reap_superseded` on the old one. Where `flock` works, a live daemon's lease is
`Held` and the old generation survives.

## Requirements

Steps to reproduce the cold-path defect:

1. Point two launchers with different compiled-in digests (for example two
   plugin versions) at one empty `ACCELERATOR_CACHE_DIR`.
2. From the first, run a design invocation. With no trees materialised, the
   launcher exports no `ACCELERATOR_TREE_<NAME>`, so the executor takes the cold
   path and starts the daemon from trees materialised by `cache ensure`.
3. While that daemon is still running, run `accelerator cache prune
   --older-than 0` from the second.
4. Expected: the daemon's generations remain. Actual: they are removed while the
   daemon runs.

The `Unknown` defect cannot be reproduced on a local filesystem, because
`ENOLCK` needs NFS or a similar backend. Its substitute reproduction needs the
liveness-probe injection seam, which this item adds first. Until then the
actual outcomes below come from reading the code. The reproduction injects a
liveness probe that returns `Unknown`:

- Run `prune --older-than 0` over a generation outside the claim window and
  under `AGE_BACKSTOP` (1h) old. Expected: the generation remains. Actual: it is
  removed by `reclaim_unclaimed`.
- Run `accelerator cache repair --force` over an existing generation.
  Expected: the superseded generation remains. Actual: it is removed by
  `reap_superseded`.

Current behaviour:

1. **Cold path holds no lease.** `accelerator cache ensure` prints
   `name\t<tree>\t<lease>` lines with the contract "the caller holds the lease
   itself" (`cli/launcher/src/launch/cache.rs:108-115`). `design-adapters` parses
   `ResolvedTree.lease` (`cli/design-adapters/src/ensure.rs:102-118`), but nothing opens it, so the daemon
   spawned afterwards runs unprotected.
2. **`Unknown` counts as free.** `cli/launcher/src/launch/outbound/resolve/tree/lease.rs:8-10` says that where `flock` is
   unavailable (for example NFS returning `ENOLCK`), liveness is reported as
   unknown "so a spuriously successful probe cannot reclaim a live daemon's
   tree". `reclaim_unclaimed` (`cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:759-762`) and
   `reap_superseded` (`resolver.rs:691-700`) compare only against
   `Liveness::Held`, so `Unknown` is reclaimed at once.
3. **Warm-path inheritance is unproven.** This is a verification gap, not a
   defect. The launcher clears `FD_CLOEXEC` on the leases it takes
   (`lease.rs:172-179`), so the fd should reach the daemon through
   `accelerator-design` (`cli/design-adapters/src/process.rs:139-220`). No test
   asserts it. If the test shows the daemon does not inherit the lease, fixing
   the warm path is in scope only as far as clearing `FD_CLOEXEC` on the leases
   the launcher hands to `accelerator-design`. Anything larger moves to a
   follow-up item.

`reap_orphans` (`cli/launcher/src/launch/outbound/resolve/tree/reap.rs:152-166`) is already correct. It
reclaims a generation that is not `Held` only once the generation directory's
mtime is at least `AGE_BACKSTOP` (1h) old. This item adopts only the `Unknown`
half of that rule: a `Free` generation outside the claim window is still
reclaimed by `reclaim_unclaimed` without an age check.

Expected behaviour:

- The lease primitive (`hold_shared_lease` and `SharedLease`) moves from the
  launcher into a new leaf library crate, `tree-lease`, which both `launcher`
  and `design-adapters` depend on. The crate is registered per the plain
  library-crate checklist in `tasks/README.md`.
- On the cold path, the executor takes a lease on every lease file named by
  `ResolvedTree.lease` before it spawns the daemon, with `FD_CLOEXEC` cleared so
  the daemon inherits it, matching the warm path.
- `reclaim_unclaimed` reclaims a generation whose lease probe returns `Unknown`
  only once the generation directory's mtime is at least `AGE_BACKSTOP` old, in
  addition to the existing claim-window check.
- `reap_superseded` never reclaims an `Unknown` generation. It leaves it to
  `reap_orphans`.
- Once the daemon's process group and the executor client have exited, the
  daemon's generations become reclaimable again.

## Acceptance Criteria

A lease is **observed held** when a non-blocking exclusive `flock` on its lease
file fails with `EWOULDBLOCK`. This is what `probe_liveness` reports as `Held`,
and it can be checked from any crate. Each daemon criterion starts with no
design daemon running for the cache root, so the invocation under test spawns
its own daemon.

- [x] The `tree-lease` crate exists and defines `hold_shared_lease` and
      `SharedLease`. `launcher` and `design-adapters` both depend on it, and
      `launcher` no longer defines them. Every item of the plain library-crate
      checklist in `tasks/README.md` is satisfied for it.
- [x] Given a cold design invocation, when the executor client has exited and
      the daemon is still running, then the lease of each of the daemon's
      generations is observed held. This holds both for an empty
      cache root and for a driver exported without its browser.
- [x] Given a warm design invocation, when the executor client has exited and
      the daemon is still running, then the lease of each of the daemon's
      generations is observed held.
- [x] Given two launchers with different compiled-in digests sharing one
      `ACCELERATOR_CACHE_DIR` root on a filesystem where `flock` reports
      liveness, when the first launcher has started a daemon and
      `accelerator cache prune --older-than 0` runs from the second, then none
      of the daemon's generations is removed. This holds separately for a daemon
      started by a warm invocation and for one started by a cold invocation.
- [x] Given the same shared root where the first launcher has materialised its
      trees by `cache ensure` but started no daemon, when the same `prune`
      runs, then those generations are removed.
- [x] Given the daemon's process group has been terminated and the executor
      client has exited, when the same `prune` runs, then the daemon's
      generations are removed. This holds separately for a daemon started by a
      warm invocation and for one started by a cold invocation.
- [x] Given a generation outside the claim window, `reclaim_unclaimed` behaves
      as follows:

      | Liveness  | mtime age < `AGE_BACKSTOP` | mtime age ≥ `AGE_BACKSTOP` |
      |-----------|----------------------------|----------------------------|
      | `Held`    | kept                       | kept                       |
      | `Free`    | reclaimed                  | reclaimed                  |
      | `Unknown` | kept                       | reclaimed                  |

- [x] Given the launcher's `repair` entry point is called in-process with
      `force`, an injected liveness probe returning `Unknown`, and an existing
      generation, then the superseded generation is kept. This proves the
      entry point passes the injected probe through to `reap_superseded`.
- [x] Given a superseded generation, `reap_superseded` keeps it when liveness is
      `Held` or `Unknown` and reclaims it when liveness is `Free`. A later
      `reap_orphans` reclaims the kept `Unknown` generation once its mtime age
      reaches `AGE_BACKSTOP`.
- [x] Given the launcher's `prune` entry point is called in-process with an
      injected liveness probe returning `Unknown`, `older_than` of zero, and a
      generation outside the claim window whose mtime age is under
      `AGE_BACKSTOP`, then the generation is kept. This proves the entry point
      passes the injected probe through to `reclaim_unclaimed`.
- [x] `mise run` (bare default task) exits 0 end-to-end.

## Open Questions

- Non-blocking: do daemon descendants, such as Playwright's browser processes,
  keep the inherited lease fd? If they do, an orphaned browser would pin the
  daemon's generations after the daemon and client exit. The plan checks this
  early and records the answer here. If descendants keep the fd, it creates
  the follow-up item and adds it to Blocks and `relates_to`. Any fix goes in
  that follow-up, not this one. The acceptance
  criteria terminate the daemon's whole process group, so they do not depend
  on the answer.

  Answered: no. On macOS (arm64, Playwright 1.55.1), after a `navigate`
  launched Chromium, `lsof` showed both leases held only by the daemon, on
  fds 10 and 11, and none by Chromium's four processes. Chromium runs in its
  own process group under the daemon, and stopping the daemon left no
  Chromium and no lease held. No follow-up item is needed.

## Dependencies

- Blocked by: none.
- Blocks: none. If the Open Question confirms that descendants keep the lease
  fd, the follow-up that closes it depends on this item.
- Ordering: extract `tree-lease` before the cold-path fix. Add the
  liveness-probe injection seam before the `Unknown` reclaim changes. The two
  orderings are independent. The cross-launcher criteria come last: they need
  both per-path lease criteria passing and the two-digest launcher fixture in
  place.
- External couplings: how Playwright's browser processes treat inherited fds
  (see Open Questions), and filesystems whose `flock` fails (for example NFS
  returning `ENOLCK`). `Unknown` comes only from the latter, so verification
  relies on the injected probe, not a real backend.
- Rollout: protection on a shared root is complete only once every launcher
  sharing it includes this fix (see Assumptions).
- Relates to: 0136 (parent epic); 0223 (the spike that found these defects);
  0196 (the vendored-runtime work that introduced the lease model); 0218
  (bounds the launcher's sub-binary cache in `bin/`, which is independent of the
  tree reclaim paths changed here).
- Decisions: the change stays within ADR-0064's shared `flock` lease and age
  backstop, so no ADR amendment is needed.

## Assumptions

- The age check is the right fallback for `Unknown` in `reclaim_unclaimed`,
  matching `reap_orphans`, rather than never reclaiming. Its protection is
  narrow: a generation outside the claim window was materialised long before it
  became eligible, so under the default window its mtime age is almost always
  past `AGE_BACKSTOP`. The backstop protects in practice only against
  `--older-than` values under 1h. On a filesystem without `flock`, a live
  daemon's tree whose claim is stale can still be reclaimed. This is accepted
  because the alternative leaves such trees unreclaimable.
- On the cold path, `cache ensure` exits before the executor takes its leases.
  `cache ensure` does not always refresh the claim: `materialise` returns
  early on a located generation without writing one. The gap is instead
  narrowed by the executor re-checking each tree after leasing it, and by one
  further `cache ensure` when a tree has gone. Every reclaimer probes the
  lease, releases the probe and only then removes the generation, so a lease
  taken inside that window still sees the directory. The launcher's `acquire`
  shares that window. It is accepted.
- Daemons started by launchers released before this fix hold no lease on the
  cold path, so a fixed launcher's `prune` can still reclaim their trees on a
  shared root. This residual exposure is accepted.

## Technical Notes

- `hold_shared_lease` lives in the launcher crate
  (`cli/launcher/src/launch/outbound/resolve/tree/lease.rs`), which
  `design-adapters` cannot depend on. `tree-lease` follows the precedent of
  `process-probe`, the crate that reads a process's start time: a primitive
  extracted so the Playwright side can share it without
  inheriting an application crate's dependency graph. `probe_liveness` and the
  single-flight lock stay in the launcher.
- Rust's `std` opens files with `O_CLOEXEC`, so the executor must clear it
  explicitly, as the launcher does, for the daemon to inherit the lease.
- The executor must hold the leases across the `ExecClient` `exec` as well as
  the daemon spawn, as the warm path does.
- When the driver is exported warm but the browser is not, the cold path
  re-ensures both trees. Leasing every returned tree covers the driver twice,
  which is harmless for a shared lock.
- `reap_superseded` has no clock. Leaving `Unknown` to `reap_orphans` avoids
  adding one. `reap_superseded` runs after the fresh generation's pointer is
  published, so by then no pointer names the superseded generation and it is
  already an orphan.
- Testing the `Unknown` criteria needs a seam for injecting a liveness probe,
  because `ENOLCK` cannot be reproduced on a local filesystem.
- The reclaim paths already read an injected `Clock`, so tests control mtime
  age either side of `AGE_BACKSTOP` by advancing the clock, not by waiting.
- `ExpectedDigests::Fixed` already lets a test pin the digest a resolver
  expects. The **two-digest launcher fixture** builds on it: two resolvers over
  one cache root, each expecting a different digest, standing in for two
  launcher versions.
- The end-to-end criteria need the daemon spawned through the executor's real
  `DaemonSpawner` and `ExecClient` path. A stub daemon program is acceptable,
  and it keeps the criteria clear of the descendant question.

## Drafting Notes

- The two defects ship together because each closes one way a live tree on a
  shared root can be reclaimed: the cold-path fix where `flock` reports
  liveness, and the `Unknown` fix, within its narrow backstop, where it does
  not. Neither the size nor
  the priority justifies a second item. The plan can still land them as
  separate phases.
- The warm-path proof is in scope because the cross-launcher `prune` criterion is
  meaningless unless both paths are verified.
- Priority is low because `prune` exposes only shared `ACCELERATOR_CACHE_DIR`
  roots, and `repair` exposes only filesystems without `flock` and runs only when
  the user asks for it.
- Out of scope: version-aware daemon reuse (design or visualiser), footprint
  reporting across sibling roots, moving the tree root to
  `${CLAUDE_PLUGIN_DATA}` (rejected in 0223), and closing the lease fd in
  daemon descendants (a follow-up item if the Open Question confirms it), and
  releasing leases when the daemon fails to spawn after they are taken. The
  executor reports a downgrade or an error and exits on that branch, which
  releases them with the process.

- The warm-path fix went beyond clearing `FD_CLOEXEC`, with the user's
  approval during planning. `DaemonSpawner` `dup2`s its identity pipe onto
  fd 3, which closed the first lease the daemon inherited. `tree-lease` now
  raises every lease to `LOWEST_LEASE_DESCRIPTOR` (10) or above, which fixes
  the warm and cold paths alike.
- The cross-launcher criteria are proven by two suites together. The
  executor's runtime suite proves a warm or cold daemon holds every lease and
  releases them when its group is killed. The launcher's two-digest fixture,
  with a stand-in daemon holding a real lease, proves a second launcher's
  `prune` spares a held lease and reclaims a free one.
- Backstop limits: a superseded generation that `repair` keeps as `Unknown`
  has its mtime reset, best-effort, so it gets a full `AGE_BACKSTOP` from
  the deferral. `reclaim_unclaimed`'s `Unknown` gate still counts from
  extraction, as the table above specifies, so the claim window is what
  protects a pointed-at generation there.

## References

- Source: `meta/work/0223-bounding-the-default-runtime-cache-root.md`
  (Technical Notes, "Lease safety on reclamation")
- `meta/decisions/ADR-0064-producer-signed-tree-attestation-over-a-compiled-in-digest.md`
  (shared `flock` lease and age backstop carried forward from ADR-0061)
- Related: 0136; 0196; 0218; 0223
