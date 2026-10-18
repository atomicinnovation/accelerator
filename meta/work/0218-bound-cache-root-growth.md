---
type: "work-item"
id: "0218"
title: "Bound a relocated cache root's growth"
date: "2026-08-17T20:36:50+00:00"
author: "Toby Clemson"
producer: "implement-plan"
status: "in-progress"
kind: "story"
priority: "low"
parent: "work-item:0136"
derived_from: ["plan:2026-08-11-0189-warm-dispatch-latency-measurement"]
blocked_by: ["work-item:0334", "adr:ADR-0072", "work-item:0339"]
relates_to: ["work-item:0189", "work-item:0164", "work-item:0223", "work-item:0196", "work-item:0214", "work-item:0222", "adr:ADR-0063", "adr:ADR-0064"]
tags: ["cli", "launcher", "performance", "bootstrap", "cache"]
last_updated: "2026-10-21T19:17:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-747"
---

# 0218: Bound a relocated cache root's growth

**Kind**: Story
**Status**: In Progress
**Priority**: Low
**Author**: Toby Clemson

## Summary

Bound the growth of a cache root relocated with `ACCELERATOR_CACHE_DIR`. Plugin
versions that share such a root each leave their launcher, sub-binaries,
`.minisig` sidecars, staged verifier copies and vendored runtime trees there,
with claims, locks, sidecars and crash residue, and little of it is ever
reclaimed. This item reclaims the shapes listed under Requirements: Root prune,
each under the condition its rule states; the plugin-written files it
deliberately keeps are recorded there and under Assumptions. The default
per-version root is already bounded by Claude Code's orphan sweep (0223,
ADR-0063), so this item's version claims and root prune stay out of the default
root, while the default root's existing tree claims and tree prune continue
unchanged; the same holds for a development checkout, whose root is
`${ACCELERATOR_PLUGIN_ROOT}/bin`. An `ACCELERATOR_CACHE_DIR` set from a checkout
is in scope. Two safety changes apply in every root — the `ENOENT` re-resolve,
and `accelerator cache prune` (the verb) no longer creating tree locks it never
needed — and the existing `store` replacement invariant is preserved and newly
tested there.

## Context

This story serves end users who relocate the cache with
`ACCELERATOR_CACHE_DIR`, usually keeping the same root across plugin
upgrades, and whose disk use there currently grows without bound.

0189 measured `cache::find` at **0.036 ms** against a root of **21 entries and
47.8 MB** (`meta/measurements/warm-dispatch-3.json`). That root was a dev
checkout's `bin/`, which is out of scope here, but it shows how a root shared
across versions accumulates: the scan cost is negligible, and the cost that
matters is disk use.

0223 then established that the default root, `${ACCELERATOR_PLUGIN_ROOT}/bin`,
sits inside the plugin version directory: only one version writes there, and
Claude Code removes the directory 14 days after it is orphaned. ADR-0063 stands
for that root. The remaining unbounded case a user can reach is an
`ACCELERATOR_CACHE_DIR` override shared across plugin versions, where every
upgrade adds a full set of single-file entries and none is reclaimed.

Several plugin versions run against a shared root at once — a pre-upgrade
session keeps its version until `/reload-plugins` — so "keep the newest N
versions" can evict a version a live session still dispatches. The vendored
runtime trees solved the same problem with retention claims; this item extends
that model to the single-file entries as version claims, and also closes the
tree cache's own reclaim gaps in a relocated root.

The tree cache's own reclaim is incomplete in a shared root. Unclaimed trees are
reclaimed only by the `accelerator cache prune` verb, and only for the currently
pinned artifact names on the host platform, so a retired artifact or another
platform's tree is never removed. A tree claim is removed only when its whole
digest is reclaimed, per-artifact lock files are never removed, and sidecars or
leases orphaned by a crash before the generation rename are never reaped.

## Requirements

### Scope

- An override root is any cache root set by `ACCELERATOR_CACHE_DIR` that is
  not `${ACCELERATOR_PLUGIN_ROOT}/bin`, compared after resolving both paths
  canonically. Sharing across versions is why it needs pruning, not the
  condition for it: a root one version uses is pruned the same way.
- Everything this item adds applies only to an override root: version claims,
  single-file prune, the widened tree prune, the residue sweep and cold-path
  root prune. In `${ACCELERATOR_PLUGIN_ROOT}/bin` behaviour is unchanged:
  tree claims, the post-materialise orphan reap and the verb's existing tree
  prune continue as today. Three things apply in every root, because they
  only make existing paths safer: the new `ENOENT` re-resolve, the existing
  `store` replacement invariant, which this item preserves and now tests,
  and the verb no longer creating tree locks for artifacts it never
  materialised.

### Retention claims

- Every dispatch in an override root refreshes a retention claim for its
  plugin version before `find`: throttled as the tree claims are,
  best-effort, and never failing a dispatch on a read-only root.
- A version claim is the file `<root>/claims/<version>.<launcher-id>`, with
  `<launcher-id>` derived as for tree claims.

### Single-file prune

- A single-file entry is any `<name>-<64hex>` with a matching `.minisig`, or
  a launcher entry `accelerator-launcher-<version>-<platform>` with a known
  platform, together with its `.minisig`. Prune never splits an entry name
  into name and version: an entry is protected when it ends in
  `-<v>-<64hex>` (or, for a launcher entry, carries version `<v>`) for some
  version `<v>` with a fresh version claim. An entry no fresh claim protects
  is unclaimed.
- Single-file prune removes an unclaimed entry and its `.minisig` only when
  both the entry and its `.minisig` are older than `AGE_BACKSTOP`. A
  version's entries written before its first claim — the bootstrap's
  launcher entry, or a `store` ahead of the first resolve — are therefore
  kept through that window.
- A claim is fresh when it was refreshed within `CLAIM_WINDOW`, or within
  `--older-than <seconds>` when the verb is given it.
- Single-file prune removes a stale version-claim file independently of the
  version's other claims; when it is that version's last claim, only after
  every unclaimed entry it protected has been removed.
- Single-file prune removes every staged verifier shim copy except the
  running plugin's own for each platform (a copy whose digest equals that of
  `${plugin_root}/bin/accelerator-verify-<platform>`), and shim staging
  residue older than `AGE_BACKSTOP`. When a platform's source cannot be
  hashed, every copy for that platform is kept.
- Single-file prune removes the plugin's own top-level `.tmp-` staging
  entries — `.tmp-<name>-<version>-<64hex>-<pid>-<seq>[.minisig]` and
  `.tmp-launcher-<pid>[.minisig]` — older than `AGE_BACKSTOP`, and never
  younger ones. Everything under `trees/`, including `trees/.tmp-*`, is
  governed by tree prune alone.

### Tree prune

- In an override root, tree prune visits every artifact and platform with
  any pointer, generation, sidecar or tree lock under `trees/`, holding the
  tree lock for that artifact and platform while it handles it.
- Tree prune reclaims a tree — its pointer and generation — of any artifact
  name and any platform when all of these hold:
  - no tree claim on its digest is fresh;
  - its digest is not the one the running launcher pins for that artifact;
  - its lease probe reports free, or unknown with the generation older than
    `AGE_BACKSTOP`;
  - its generation is of this launcher's layout version.
  A generation of another layout version is kept with the pointer naming it.
  Whether a tree is superseded plays no part.
- A generation no pointer names is removed as the orphan reap does today:
  when its lease probe reports free or unknown, and it is older than
  `AGE_BACKSTOP`.
- Tree prune removes individual tree-claim files not refreshed within
  `CLAIM_WINDOW` (or `--older-than`).
- Tree prune removes `.sealed`, `.sealed.sig` and `.lease` sidecars whose
  generation is absent, when the lease probe reports free or unknown and they
  are older than `AGE_BACKSTOP`.
- Tree prune removes a tree lock `trees/.tmp-<name>-<platform>.lock`, while
  holding it, once no pointer or generation for that artifact and platform
  remains and it is older than `AGE_BACKSTOP`.
- In every root, the verb no longer creates a tree lock for an artifact that
  has no pointer or generation in the root.

### Residue sweep

- The residue sweep removes `.accelerator-probe-*` files older than
  `AGE_BACKSTOP`.
- The residue sweep removes an `.accelerator-lock-<platform>/` older than
  `AGE_BACKSTOP` whose `pid` file is empty or names a dead process.

### Root prune

- Root prune is single-file prune, tree prune and the residue sweep. It
  removes only these shapes, each under its rule above: single-file entries
  and their `.minisig` sidecars; the plugin's own top-level `.tmp-` shapes;
  staged shim copies and their `.staging.<pid>` residue; stale
  `claims/<version>.<launcher-id>` files; `.accelerator-probe-*` files;
  stale bootstrap lock directories; and under `trees/` pointers,
  generations, tree claims, sidecars, leases and tree locks. Every other name
  is kept, including `.accelerator-unverified.log`,
  `.accelerator-prune.lock`, a bootstrap lock directory whose `pid` is live,
  and the `claims/` and `trees/` directories.
- Every cold path in an override root — a successful sub-binary `store`
  (before `exec`) or a tree materialise — runs a root prune, trying each lock
  without blocking and skipping the work under any lock that is held.
- In an override root, `accelerator cache prune` runs a root prune, waiting
  for each lock without a time limit, as today's tree prune does.
- Single-file prune and the residue sweep run under the root lock, a new
  single-flight lock.
- Tree prune handles each artifact and platform under the existing tree lock
  for that artifact and platform, taking each in turn; a materialise reuses
  the tree lock it already holds for its own artifact and platform.
- A cold path skipping a held lock skips only the work under it. The `store`
  write and rename take no lock; only the root prune that follows a
  successful `store` does.
- The root lock is the file `<root>/.accelerator-prune.lock`.
- In an override root, when the root lock or a tree lock cannot be taken for
  a reason other than contention, such as `ENOLCK` on a network filesystem,
  a cold path skips the work under it and proceeds, and the verb reports the
  failure and exits non-zero. In `${ACCELERATOR_PLUGIN_ROOT}/bin` the verb's
  lock handling is unchanged.

### Recovery and invariants

- A dispatch whose `exec` fails with `ENOENT` re-resolves once through the cold
  path; a second failure surfaces as `ResolutionError::Exec`.
- Before any other launcher change lands, a filesystem-call recorder for
  tests is added, in its own commit if it needs a launcher source change.
  Today's default-root warm-dispatch filesystem-call sequence is then
  captured as a fixed expected list, in a commit that changes no launcher
  source and passes there; later commits do not modify the list, and the
  warm-path criteria assert against it.
- `store` keeps replacing a corrupt entry — one `find` locates but
  re-verification rejects — by renaming verified bytes over it; neither
  `store` nor the `ENOENT` re-resolve removes an entry before replacing it.

### Documentation

- The `ACCELERATOR_CACHE_DIR` entries in
  `docs-site/src/content/docs/design.md` and `internals.md` state that a
  relocated root is pruned automatically on cold paths, that a version idle
  for `CLAIM_WINDOW` (14 days) loses its entries and re-fetches, what
  `accelerator cache prune` now covers, and that a root is pruned only when
  it is writable by its owner alone, or through the owner's private group,
  and no directory above it is writable by others without the sticky bit,
  so a shared host gives each user `<shared>/$USER`.
- A post-change `mise run measure:warm-dispatch` run is recorded under
  `meta/measurements/`, whatever its gating result, as a record rather than
  as evidence of this change's cost.

## Acceptance Criteria

- [ ] Given an override root holding entries for versions A and B, where only B
      has a claim refreshed within `CLAIM_WINDOW`, when single-file prune runs,
      then A's sub-binaries, launcher entry and `.minisig` sidecars are
      removed and B's are kept.
- [ ] Given versions A and B both claimed within `CLAIM_WINDOW`, when
      single-file prune runs, then no entry is removed.
- [ ] Given a cold dispatch in an override root that stores a new version's
      sub-binary alongside an unclaimed version's entries, when the store
      succeeds, then those entries are already absent when a test hook at
      `exec` runs.
- [ ] Given an override root whose root lock is held, when a cold dispatch's
      store succeeds, then that dispatch skips single-file prune and the
      residue sweep and proceeds to `exec` without waiting.
- [ ] Given a dispatch in an override root whose version has no claim, or a
      claim older than `CLAIM_WINDOW/10`, then
      `<root>/claims/<version>.<launcher-id>` is written before `find` runs —
      shown by a test recording the dispatch's filesystem calls.
- [ ] Given a dispatch in an override root whose version claim is younger than
      `CLAIM_WINDOW/10`, then the claim is not rewritten.
- [ ] Given a version whose only claims are a symlink and a file owned by
      another user (through the uid seam), when single-file prune runs, then
      that version's entries are removed.
- [ ] Given the cache root is `${ACCELERATOR_PLUGIN_ROOT}/bin` — directly, or
      via `ACCELERATOR_CACHE_DIR` naming it through a symlink or trailing
      slash — seeded with an unclaimed older version's paired entries, an old
      `.tmp-` entry, a staged shim copy, an old probe file, an orphan tree
      sidecar, an unclaimed tree for a retired artifact and for another
      platform, a tree claim older than `CLAIM_WINDOW` on a live digest, an
      old lock file with no pointer, and a stale lock directory with a dead
      `pid`, when a cold dispatch, a tree materialise and
      `accelerator cache prune` run, then no version claim is written and
      every seeded entry survives.
- [ ] Given the cache root is `${ACCELERATOR_PLUGIN_ROOT}/bin` holding an
      unreferenced generation older than `AGE_BACKSTOP` and an unheld one
      younger, when a tree materialise runs, then the orphan reap removes the
      older and keeps the younger, as today.
- [ ] Given the cache root is `${ACCELERATOR_PLUGIN_ROOT}/bin`, then the
      `ENOENT` re-resolve, corrupt-entry replacement and no-new-tree-lock
      criteria each hold there as they do in an override root.
- [ ] Given the cache root is `${ACCELERATOR_PLUGIN_ROOT}/bin` holding an
      unclaimed superseded tree for a currently pinned artifact on the host
      platform, when `accelerator cache prune` runs, then that tree is
      removed, as today.
- [ ] Given a dispatch whose entry a test hook removes between `find` and
      `exec`, when `exec` fails with `ENOENT`, then the dispatch re-resolves
      once through the cold path and succeeds — shown by a named test.
- [ ] Given the re-resolved `exec` also fails, then the second error surfaces as
      `ResolutionError::Exec`; given a first `exec` failure other than
      `ENOENT`, then no re-resolve occurs.
- [ ] Given an override root containing names the plugin does not produce —
      foreign `.tmp-*` files older than `AGE_BACKSTOP`, `*-*-<64hex>` names
      with no matching `.minisig`, a foreign dotfile, and under `trees/` a
      foreign file, a foreign directory and a `.ref` naming no generation
      shape — when a root prune runs from the cold path and from the verb,
      then they are kept.
- [ ] Given an override root holding an unclaimed version's entries alongside an
      `.accelerator-lock-<platform>/` whose `pid` is live, a probe file younger
      than `AGE_BACKSTOP`, `.accelerator-unverified.log`, `trees/`,
      `accelerator-launcher-<version>-<unknown-platform>` and a `.minisig` with
      no matching entry, when a root prune runs from the cold path and from the
      verb, then only the unclaimed version's entries are removed.
- [ ] Given the plugin's own `.tmp-` staging entries
      (`.tmp-<name>-<version>-<64hex>-<pid>-<seq>[.minisig]`,
      `.tmp-launcher-<pid>[.minisig]`) younger than `AGE_BACKSTOP`, when
      single-file prune runs, then they are kept; given ones older than
      `AGE_BACKSTOP`, they are removed.
- [ ] Given staged shim copies `accelerator-verify-<platform>-<sha>` in an
      override root, when single-file prune runs, then the running plugin's
      own copy for each platform is kept and every other copy is removed;
      given an unreadable shim source, every copy for that platform is kept;
      given
      `accelerator-verify-*.staging.<pid>` residue, it is removed only when
      older than `AGE_BACKSTOP`.
- [ ] Given an override root holding a version claim and a tree-claim file
      each refreshed 2 days ago, when
      `accelerator cache prune --older-than 86400` runs, then that version's
      entries, its version claim and the tree-claim file are removed; without
      the flag, all are kept.
- [ ] Given `accelerator cache prune` in an override root holding an
      unclaimed version's single-file entries and an unclaimed tree, then
      both are removed.
- [ ] Given the root lock and a tree lock are held, when
      `accelerator cache prune` runs, then it waits for each and prunes once
      each is released.
- [ ] Given an override root whose root lock is held, holding an unclaimed
      version's single-file entries, an old probe file and an unclaimed tree
      for a second artifact whose tree lock is free, when a tree materialise
      runs, then it does not wait, the single-file entries and probe file
      survive, and
      the second artifact's unclaimed tree is removed.
- [ ] Given an override root holding unclaimed trees for artifacts X and Y,
      with X's tree lock held, when a cold dispatch's store succeeds, then it
      proceeds to `exec` without waiting, X's unclaimed tree survives, and
      Y's is removed.
- [ ] Given an override root holding unclaimed, unpinned trees whose lease
      probe reports held, unknown and younger than `AGE_BACKSTOP`, unknown and
      older, and free, plus an unclaimed tree with a free lease whose digest
      the running launcher pins (lease results through the lease-liveness
      seam), when tree prune runs, then only the unknown-and-older and free
      unpinned trees are removed.
- [ ] Given an override root holding an unreferenced generation older than
      `AGE_BACKSTOP` and an unheld one younger, and orphan sidecars and a tree
      lock for a retired artifact with no pointer or generation, when tree
      prune runs, then the older generation, the old sidecars and the old
      tree lock are removed and the younger generation is kept.
- [ ] Given unclaimed trees for a retired artifact and for another platform,
      and a claimed tree for another platform, when tree prune runs, then the
      unclaimed two are removed and the claimed one is kept.
- [ ] Given claim files older and younger than `CLAIM_WINDOW` on a live
      digest, when tree prune runs, then only the older ones are removed.
- [ ] Given sidecars with no generation, older and younger than
      `AGE_BACKSTOP`, and one whose lease is held, when tree prune runs, then
      only the old, unheld ones are removed.
- [ ] Given a lock file for an artifact and platform with no remaining pointer
      or generation, older than `AGE_BACKSTOP`, when tree prune runs, then it
      is removed; one younger than `AGE_BACKSTOP` is kept; a lock for a
      current tree is kept; and the verb creates no lock for an artifact it
      never materialised.
- [ ] Given an override root holding an unclaimed, unpinned, free-lease pointer
      and generation of another layout version older than `AGE_BACKSTOP`,
      beside an otherwise identical tree of this launcher's layout version,
      when tree prune runs, then the other-layout pointer and generation are
      kept and the same-layout tree is removed.
- [ ] Given probe files older and younger than `AGE_BACKSTOP`, when the residue
      sweep runs, then only the older ones are removed.
- [ ] Given lock directories older than `AGE_BACKSTOP` with an empty `pid` and
      with a dead `pid`, one with a live `pid`, and one younger than
      `AGE_BACKSTOP` with a dead `pid` (through the `pid`-liveness seam), when
      the residue sweep runs, then only the first two are removed.
- [ ] Given a tree materialise in an override root holding an unclaimed
      superseded tree for the same artifact and platform, an unclaimed
      version's single-file entries older than `AGE_BACKSTOP` and an old probe
      file, then the materialise has run a root prune that removed all three
      and kept the new tree.
- [ ] Given a single-file prune paused, through a test hook, after listing
      entries and before reading claims, when a dispatch for a version with no
      prior claim writes its claim and stores its entry, then that entry
      survives; and given the prune paused before listing, the same holds.
- [ ] Given one root prune paused inside the root lock through a test hook,
      when a second root prune starts on the same override root, then a
      cold-path second prune returns without touching seeded prunable
      entries, and a verb second prune blocks until the first is released.
- [ ] Given an override root after dispatches and tree materialises from
      versions A, B and C, each pinning distinct digests for distinct
      artifacts, including for each a store interrupted after its temporary
      write (seeded as `.tmp-` files), with every file A and B created
      back-dated past `CLAIM_WINDOW` (so neither has a fresh version claim or
      tree claim) and all residue back-dated past `AGE_BACKSTOP`, when C,
      through the `bin/accelerator` bootstrap, dispatches a sub-binary not yet
      stored, fetched from a local fixture, and exits, then the root holds
      only: C's single-file entries and launcher entry with their `.minisig`
      sidecars; C's version claim; the trees whose digests C pins, with their
      pointers, tree claims, leases, `.sealed` and `.sealed.sig` sidecars and
      tree locks; the running plugin's staged shim copy; the root lock;
      `.accelerator-unverified.log` if present; and the `claims/` and `trees/`
      directories.
- [ ] Given version V with one stale and one fresh version claim, and
      version W whose only claim is stale, with W's entries older than
      `AGE_BACKSTOP`, when single-file prune runs, then V's stale claim is
      removed and its fresh claim and entries kept, and W's entries are
      removed before W's claim — shown by a test recording the prune's
      filesystem calls.
- [ ] Given an override root holding entries for version W with no version
      claim, where W's launcher entry and one sub-binary are younger than
      `AGE_BACKSTOP` and another sub-binary is older, when single-file prune
      runs, then only the older sub-binary and its `.minisig` are removed.
- [ ] Given entries `x-1.24.0-pre.78-<64hex>` and `x-y-1.24.0-<64hex>` with
      matching `.minisig` files older than `AGE_BACKSTOP`, and fresh version
      claims only for `1.24.0-pre.78`, when single-file prune runs, then the
      first is kept and the second removed.
- [ ] Given a read-only override root whose version claim is missing or
      older than `CLAIM_WINDOW/10`, when a warm dispatch tries and fails to
      write its claim, then the dispatch succeeds.
- [ ] Given an entry that `find` locates but re-verification rejects, when the
      dispatch falls through to the cold path and stores a verified
      replacement, then the only operation on the entry path is a rename onto
      it — no unlink precedes it — and the entry then holds the verified
      bytes — shown by a test recording the store's filesystem calls.
- [ ] Given `ACCELERATOR_CACHE_DIR` is unset, when a warm dispatch runs, then
      it performs exactly the fixed pre-change filesystem-call list — shown by
      a test recording the dispatch's filesystem calls.
- [ ] Given `ACCELERATOR_CACHE_DIR` naming `${ACCELERATOR_PLUGIN_ROOT}/bin`,
      when a warm dispatch runs, then beyond the fixed pre-change list it
      performs only the canonical resolution of the two roots, with no claim
      read or write — shown by a test recording the dispatch's filesystem
      calls.
- [ ] Given the lock-failure seam makes the root lock fail with `ENOLCK`,
      when a cold dispatch's store succeeds in an override root, then it
      skips single-file prune and the residue sweep and proceeds to `exec`;
      and when `accelerator cache prune` runs, then it reports the failure
      and exits non-zero.
- [ ] Given the lock-failure seam makes artifact X's tree lock fail with
      `ENOLCK` in an override root holding unclaimed trees for X and Y, when
      a cold store and a tree materialise of Y each run, then both proceed,
      X's tree survives and Y's unclaimed tree is removed; and when
      `accelerator cache prune` runs, then it reports the failure and exits
      non-zero.
- [ ] Given `ACCELERATOR_CACHE_DIR` set from a development checkout to a
      path other than its `bin/`, when a dispatch runs, then it writes a
      version claim and its cold path runs a root prune.
- [ ] Given an override root holding `trees/.tmp-<generation>` older than
      `AGE_BACKSTOP` whose lease is held, when single-file prune runs, then it
      is untouched.
- [ ] Given an override root whose version claim is fresh, when a warm
      dispatch runs, then beyond the fixed pre-change list it performs only
      the canonical resolution of the two roots and one metadata read of its
      version claim — no write and no directory listing — shown by a test
      recording the dispatch's filesystem calls.
- [ ] The `ACCELERATOR_CACHE_DIR` entries in
      `docs-site/src/content/docs/design.md` and `internals.md` each state
      that a relocated root is pruned automatically on cold paths, that a
      version idle for 14 days loses its entries and re-fetches, what
      `accelerator cache prune` now removes, and that a root is pruned only
      when it is writable by its owner alone, or through the owner's private
      group, and no directory above it is writable by others without the
      sticky bit, so a shared host gives each user `<shared>/$USER`.
- [ ] A post-change `mise run measure:warm-dispatch` run's JSON is committed
      under `meta/measurements/`, whatever its gating result.
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Open Questions

None remaining.

## Dependencies

- **Blocked by** 0334 (done; merged in PR #162): it set the lease-liveness
  seam, `AGE_BACKSTOP` semantics and the tree prune this item extends. The
  uid seam, the `pid`-liveness seam, the lock-failure seam and the test
  hooks (prune pause, `find`/`exec` removal, `exec`) are new work in this
  item, as is the filesystem-call recorder for tests. Automatic tree prune on
  cold paths also relies on 0334's guarantee that the design executor holds
  a lease on a tree on both cold and warm paths; a regression there would
  let a live daemon's tree be reclaimed.
- **Blocked by** 0339: `reap_orphans` deletes another artifact's in-progress
  materialise, and this item's cold-path root prune would run that reap far
  more often. This item inherits 0339's contract: a held tree lock means a
  materialise or `prune` of that artifact is in progress. The root prune and
  orphan-lock removal must reuse 0339's non-creating lock probe and never
  remove a tree lock file that is held.
- **Orphan-lock removal** must unlink a lock file only while holding it. That
  is safe against concurrent materialises only for launchers from 0339
  onwards, which confirm the inode after locking.
- **Relates to** 0189, which measured the root; 0164, which established the
  fetch-verify-cache resolver; 0223, which settled the default root; 0214,
  which settled the lease design (now governed by ADR-0064); and 0196, whose
  implementation (plan `2026-08-11-0196-design-vendored-runtime-distribution`)
  added tree claims and the lease. 0222 plans a second materialise route
  (`accelerator cache ensure --from`); it must refresh tree claims and take
  part in, or explicitly skip, the cold-path root prune.
- **Parent**: epic 0136.
- **Measurement harness** (0189): `measure:warm-dispatch` measures only
  `${ACCELERATOR_PLUGIN_ROOT}/bin` and refuses `ACCELERATOR_CACHE_DIR`; its
  run-to-run spread (cell C5 of `warm-dispatch-N.json`, the ratio of gated
  warm dispatch to the shell baseline, at 1.318–1.342 across three
  same-version runs) exceeds this change's cost, so default-root cost
  is verified by the filesystem-call criterion and override-root cost by the
  claim-throttle and override-root filesystem-call criteria.
- **Rollout**: safe concurrent dispatch on an override root holds only once
  every launcher sharing it includes this change; a pre-claim launcher caught
  between `find` and `exec` by a prune fails that dispatch rather than
  re-resolving.
- **External**: root prune relies on `flock` on the override root's
  filesystem; where it fails for a reason other than contention, the cold
  path skips that work and the verb reports it.
- **External**: the default-root exclusion relies on Claude Code's orphan
  sweep bounding `${ACCELERATOR_PLUGIN_ROOT}/bin`. Known defects
  (anthropics/claude-code#82145, open; #61855, closed not planned) are
  recorded in 0223; if the sweep does not meet its documented 14-day bound,
  revisit the exclusion.
- **Ordering**: ADR-0072 (see Decisions) accepted first; then the
  filesystem-call recorder; then the fixed warm-path call list, in a commit that
  changes no launcher source; then the other test seams and hooks; then the
  `ENOENT` re-resolve; then version claims; then single-file prune, tree prune
  and the residue sweep behind the verb; and last, the cold-path triggers. Each
  step is verifiable on its own, and no automatic prune runs before the recovery
  and claims it relies on exist.
- **Decisions**: ADR-0072 amends ADR-0063 for relocated roots — automatic,
  claim-gated root prune on cold paths — and is accepted before implementation;
  ADR-0063 stands for the default root and development checkouts. Tree prune
  stays within ADR-0064: it probes leases as that ADR prescribes, removes a
  `.lease` only with its generation absent, and never reaps another layout
  version.

## Assumptions

- A version idle for longer than `CLAIM_WINDOW` (14 days) may lose its entries
  and re-fetch on its next dispatch. That is the trade-off ADR-0063 already
  accepts for the default root.
- Versions whose launchers predate version claims have no claim, so the first
  prune by a claim-aware launcher evicts their entries. A live pre-upgrade
  session re-fetches; this one-off cost is accepted.
- A prune that removes another version's staged shim copy between that
  bootstrap's staging check (`bin/accelerator:336`) and its use in
  `verify_launcher` (`:435`) fails that bootstrap with `fail_integrity`. The
  failure is appended to `.accelerator-unverified.log`, which the SessionStart
  hook reports until the user clears it. It arises only across releases that
  change the vendored shim. Accepted.
- A prune that removes an unclaimed version's launcher entry older than
  `AGE_BACKSTOP` between the bootstrap's verification and its `exec` fails
  that one dispatch; the next invocation re-fetches. Accepted.
- `.accelerator-unverified.log` grows by one line per integrity failure and is
  never pruned, because the SessionStart hook reads it to warn the user.
  Accepted.
- Removing a superseded tree adds latency to the cold path that runs it, about
  once per superseded tree. Accepted.
- A launcher from before 0339 does not confirm its lock's inode, so after a
  lock file is unlinked it can lock the orphaned inode and no longer exclude a
  launcher that holds the recreated file. While both hold a lock:
  - any reap can remove that launcher's in-progress residue, through the
    `Absent`, `Taken` or own-artifact arms;
  - a concurrent materialise of the same artifact can append to the same
    partial archive and corrupt it.

  Each consequence records a sticky marker for that artifact, lasting for the
  marker TTL in the session. Its cause, `UnexpectedDigest` or `Extraction`, is
  Refusal-class, so it also stops a `--fail-safe` crawl rather than degrading
  it. This item must either accept that by name, or remove lock files only
  once no launcher without the inode check shares the root.
- The dead-`pid` test assumes the root is not shared across hosts.
- Generations of another layout version, and their pointers, are left to a
  launcher that understands that layout; if none runs against the root
  again, they stay. Accepted: layout bumps are rare, and reaping a layout
  this launcher cannot read risks a newer launcher's live tree.
- A development checkout's `bin/` keeps accumulating single-file entries
  across versions, since neither dispatches nor the verb prune them there.
  Accepted: it affects only plugin developers, who can clear the directory
  by hand.
- While launchers from before this change share an override root, one of
  them caught between `find` and `exec` by a prune fails that dispatch.
  Accepted for the rollout window; no compatibility gating is in scope.
- A final uninstall leaves a relocated root unpruned, since no launcher runs to
  prune it. Accepted.
- An override root that any other user can create names in, sticky or not,
  or one under a directory others can write to without the sticky bit, is
  not pruned, automatically or by the verb. A root writable only through
  the user's private group counts as safe, and a symlinked root is judged by
  its canonical target. Another user could otherwise swap a subdirectory,
  or the root, for a symlink, or plant a name the bootstrap's predictable
  writes follow, and redirect removals or writes outside the root. Such a
  root still works as a cache but is no longer pruned at all, where today
  the verb prunes its pinned trees. A shared host gives each user their own
  root, such as `/var/cache/accelerator/$USER`. A pruned shared cache is
  out of scope: 0340 covers it. Accepted.

## Technical Notes

- Terms: the **verifier shim** is the `accelerator-verify-<platform>`
  minisign verifier shipped in `${ACCELERATOR_PLUGIN_ROOT}/bin`; the
  bootstrap stages a copy into the cache root as
  `accelerator-verify-<platform>-<sha>` to verify the launcher. A **claim** is
  an age-based retention marker a launcher refreshes for what it uses, and
  protects it for `CLAIM_WINDOW`. A **lease** is a shared `flock` on a
  generation's `.lease` file, held only while a process is using that tree
  (ADR-0064). A lease probe reports one of held, free or unknown; rules
  name these results rather than "unheld".
- Tree-cache terms (ADR-0064): a **generation** is one extracted, sealed copy
  of a tree, created under a `.tmp-` name and renamed into place; a
  **pointer** (`<name>-<platform>-<digest>.ref`) names the generation serving
  a digest; a tree is **superseded** when no current pin names its digest;
  the **layout version** in a generation's name records the extraction
  policy that produced it. To **materialise** is to fetch, verify and
  extract a tree. To **pin** a digest is for a launcher to have it compiled
  in as the digest it expects for an artifact. The **orphan reap** is the
  existing post-materialise removal of generations no pointer names. The
  **warm path** is a dispatch whose entry `find`
  already locates; the **cold path** fetches and stores it first. A
  **single-flight** lock lets one process do a piece of work while others
  skip or wait.
- Qualified terms: a **tree claim** (`trees/claims/<digest>.<launcher-id>`)
  protects a tree digest and a **version claim**
  (`claims/<version>.<launcher-id>`) protects a plugin version's single-file
  entries. A **launcher entry** is the cached
  `accelerator-launcher-<version>-<platform>` file; "launcher" alone means
  the running process. The **root lock** is this item's new single-flight
  lock, a **tree lock** is `trees/.tmp-<name>-<platform>.lock`, and a
  **bootstrap lock directory** is `.accelerator-lock-<platform>/`, which
  `bin/accelerator` holds while fetching the launcher, with a `pid` file
  naming the holder. The root lock's file is `.accelerator-prune.lock`. A
  **probe file** (`.accelerator-probe-<pid>[-<seq>]`)
  is written and removed to test that the root is writable; a **lease
  probe** is the exclusive-lock attempt that tests whether a lease is held;
  it reports **unknown** when it cannot tell — the lock call fails for a
  reason other than contention, or the platform ties `flock` locks to
  processes rather than open files.
  A **seam** is a point where a test can inject behaviour, such as a pause
  or a stand-in liveness check.
- Two checks go through test seams: claim ownership compares against an
  injected current uid, and the residue sweep's `pid` liveness test is an
  injected probe. Tests drive both through these seams rather than through
  real foreign-owned files or real dead processes.
- `AGE_BACKSTOP` is 1 hour and `CLAIM_WINDOW` 14 days. `AGE_BACKSTOP` is
  private to `tree/reap.rs` today; it is lifted to where both the tree
  reaper and single-file prune can share it, keeping its value.
- The tree cache's claims (`tree/claims.rs`) are the precedent: each launcher
  writes `<root>/trees/claims/<digest>.<launcher-id>`, skips the rewrite while
  the claim is younger than a tenth of `CLAIM_WINDOW`, and ignores symlinked or
  foreign-owned claims. `launcher_id` hashes the launcher's path, which already
  embeds the cache directory, version and platform. Single-file entries need
  the same model keyed by plugin version rather than digest.
- A version claim suffices because every versioned entry name embeds the
  version (`{name}-{version}-{sha256}`,
  `accelerator-launcher-{version}-{platform}`), so one claim covers a version's
  sub-binaries, launcher and signatures. Staged shim copies are the exception.
- The launcher entry is written by the `bin/accelerator` shell bootstrap, not by
  `store`, and the launcher's first resolve writes the version claim. Until
  then the entry is unclaimed; the `AGE_BACKSTOP` grace in single-file prune
  is what protects it, so the shell needs no change.
- The warm path takes no lock. Unlinking a binary that is already executing is
  safe on Unix; only the gap between `find` and `exec` is exposed, and it needs
  a version idle for longer than `CLAIM_WINDOW` to coincide with a prune. Cold-
  path recovery covers that gap at no warm-path cost, where a shared lock would
  add a syscall pair to every dispatch.
- The launcher does not re-resolve on `ENOENT` today: `run_external` makes a
  single `Command::exec` attempt (`core.rs:313-322`). The bounded re-resolve
  is new code.
- No root-wide lock exists today; the only `flock` single-flight locks are the
  per-artifact tree locks under `trees/`. Single-file prune takes the new
  root lock with a non-blocking try on the cold path, and `store` stays
  lock-free. Safety against a concurrent `store` rests on the dispatching
  version's claim being refreshed before `find`, and on prune listing a
  version's entries before reading its claims and removing only the entries
  it listed: any listed entry was stored after its claim was written, so the
  claim read sees it. An entry written before its version's first claim is
  covered by the `AGE_BACKSTOP` grace instead.
- Skipping `${ACCELERATOR_PLUGIN_ROOT}/bin` avoids redundant work in a root
  Claude Code's orphan sweep already bounds, keeps the claim check off the
  default warm path, and removes any risk to the shipped files there
  (`accelerator`, `accelerator-verify-<platform>`,
  `accelerator-verify.vendored.sha256`).
- `ENOENT` from `execve` can also mean a missing ELF interpreter; the single
  bounded re-resolve keeps that case from looping and reports the second
  failure rather than swallowing it.
- The authoritative list of shapes root prune may remove, and of what it
  keeps, is under Requirements: Root prune.
- Tree reclaim today loops only over `pins::artifact_names()` on the host
  platform (`tree/resolver.rs:801`); the automatic reap ignores `.lock` files
  and sidecar suffixes (`tree/reap.rs:117-135`); and claims are dropped only
  by `drop_all` with their digest (`tree/claims.rs:86-97`).
- Staged shim copies carry the shim's sha, not a version, so no claim covers
  them. Prune keeps the running plugin's own copy for each platform and
  removes the rest: the
  bootstrap restages a missing copy locally from `${plugin_root}/bin`
  (`bin/accelerator:336-352`). An age guard would not help, since an in-use
  copy's mtime is its first staging time.

## Drafting Notes

- Amended on 2026-10-21 during plan review: prune runs only in a root writable
  by its owner alone, or through the owner's private group, under ancestors
  no other user can rename entries in. The private-group exception exists
  because a umask of 002 under user-private groups makes every new root
  group-writable. Ancestors are checked because another user could
  otherwise swap the root itself. A sticky shared root is refused too:
  the sticky bit stops renames but not other users planting names that the
  bootstrap's predictable writes follow. A shared root still works as a
  cache; it is just not pruned, and a per-user root beneath it is the
  supported layout. An ancestor owned by another uid, as in a container's
  bind mount or a CI workspace, is refused unless the operator lists that
  uid in `ACCELERATOR_CACHE_TRUSTED_OWNERS`: relaxing ownership for root
  automatically would let an ordinary user steer root's deletions.

- Amended on 2026-10-20 during plan review: prune now keeps the running
  plugin's own shim copy. Removing it raced every live bootstrap into
  `fail_integrity`, whose `.accelerator-unverified.log` entry the SessionStart
  hook reports indefinitely, so a routine prune became a lasting trust-chain
  alarm. Removing it also freed no space, because the copy is restaged at
  once. The rule covers every platform, not only the host's, because a root
  shared across platforms would otherwise delete the other platform's live
  copy on every prune. The A/B/C end state gains the kept shim copy.

- Narrowed from "every cache root" to the shared `ACCELERATOR_CACHE_DIR` root
  after 0223 showed the default root is bounded by Claude Code's orphan sweep.
- Dev-checkout `bin/` roots are excluded: the item protects end users who
  relocate the cache. The 0189 measurement, taken on a dev checkout, is cited
  only to show how a shared root accumulates.
- Chose prune-after-store with claims over eviction during `store`: `store`
  knows only its own version, so it cannot tell which other versions are live.
- Dropped the original "size the policy from real data" requirement in favour of
  reusing `CLAIM_WINDOW` and `AGE_BACKSTOP`, so the tree and single-file caches
  share one retention policy.
- Stress-tested on 2026-10-15. The "roots are treated uniformly" requirement
  was replaced with override-only claims and prune; staged shim copies were
  brought into scope; the lock, claim ordering, cold-path prune behaviour,
  `ENOENT` recovery bound, tight allowlist and `--older-than` scope were made
  explicit; and the accepted risks were recorded under Assumptions.
- Considered and rejected for shim copies: an entry-mtime age guard (an in-use
  copy's mtime never refreshes) and recording the shim sha in the version
  claim (the bootstrap already restages a missing copy locally).
- Widened on 2026-10-15 from single-file entries to the whole override root:
  trees, their claims, locks and sidecars, probes and stale lock directories.
  The known-sub-binary-name rule was replaced by `.minisig` pairing so that
  retired sub-binaries are reclaimed as retired trees are.
- Considered and rejected for pre-claim versions: an entry-mtime fallback claim
  and a claims-directory grace period, in favour of a one-off eviction.

## References

- `cli/launcher/src/launch/outbound/resolve/cache.rs` — `find`, `store` and the
  `.tmp-` namespace
- `cli/launcher/src/launch/outbound/resolve/tree/claims.rs` — retention claims
  and `CLAIM_WINDOW`
- `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:28` — `AGE_BACKSTOP`
- `cli/launcher/src/launch/cache.rs` — the `accelerator cache prune` verb
- `cli/launcher/src/launch/core.rs:313-322` and
  `cli/launcher/src/launch/outbound/exec.rs` — the single-attempt `exec`
- `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:593-611` — the
  non-blocking `try_single_flight` precedent for post-store reaping
- `bin/accelerator:196-217` — the cache root derivation
- `bin/accelerator:328-352` — shim staging into the cache root
- `bin/accelerator:404-453` — the launcher entry, its verification and fetch
- `meta/measurements/warm-dispatch-3.json` — 21 entries, 47.8 MB, 0.036 ms
- `meta/decisions/ADR-0063-plugin-version-scoped-artifact-cache.md`
- `meta/decisions/ADR-0072-claim-gated-automatic-prune-for-relocated-cache-roots.md`
  — the amendment this item implements
- `meta/decisions/ADR-0064-producer-signed-tree-attestation-over-a-compiled-in-digest.md`
- `meta/plans/2026-08-11-0196-design-vendored-runtime-distribution.md` —
  where tree claims and the lease were built
- Related: 0136, 0164, 0189, 0196, 0214, 0222, 0223, 0334, 0340 (shared cache
  roots on multi-user systems: sharing trees and pruning a group-shared root)
