---
type: "work-item"
id: "0339"
title: "Tree reap deletes another artifact's in-progress materialise"
date: "2026-10-15T23:11:47+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "done"
kind: "bug"
priority: "medium"
parent: "work-item:0136"
blocks: ["work-item:0218"]
relates_to: ["work-item:0334", "work-item:0214", "work-item:0222", "adr:ADR-0072"]
external_id: "PP-922"
tags: ["cli", "launcher", "cache", "tree", "concurrency"]
last_updated: "2026-10-16T00:08:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0339: Tree reap deletes another artifact's in-progress materialise

**Kind**: Bug
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

`reap_orphans` runs while holding only one artifact's tree lock, but its scan
removes every artifact's temp trees, `.ref` sidecars and non-resumable
partial archives under `trees/`. A reap for one artifact can therefore delete
another artifact's half-extracted tree or partial download while that
materialise is still running. Both callers are affected: the reap after a
materialise and the reap inside `prune`. The fix also spares other
artifacts' resumable archives, which today are lost. Found by code inspection
during the 0218 review; not yet observed.

## Context

In this item an **artifact** is the (name, platform) pair a tree lock is keyed
on: `trees/.tmp-<name>-<platform>.lock`; its **artifact name** is the name
alone. A materialise holds its artifact's tree lock from the start of the
archive download, through the rename of the generation into place, until its
`reap_orphans` call returns; `prune` takes the same lock and holds it across
the same reap. Two materialises of one
artifact serialise on that lock; two of different artifacts — the browser and
driver trees, for example — run concurrently. The reap's scan covers the whole
of `trees/`, not just its own artifact.

Terms used below:

- **Cache root**: the directory holding `trees/`; several launchers and
  artifacts can share one. A **root prune** reclaims unclaimed entries across
  a whole cache root (ADR-0072); it is 0218's work and reaches this reap only
  through `prune`.
- **`prune`**: the resolver's per-artifact prune, which takes one artifact's
  tree lock, reclaims its unclaimed generations and runs `reap_orphans`.
- **Tree lock**: the per-artifact single-flight lock file above, taken by
  `try_single_flight` or `take_single_flight`. It is distinct from a
  **lease**, the shared `flock` sidecar beside a finished generation that
  marks it in use; both live in `lease.rs`.
- **Generation**: one extracted tree directory, named
  `<name>-<platform>-<digest>-<layout>-<suffix>`; while being extracted it
  lives at `trees/.tmp-<generation>/` with a `.tmp-<generation>.ref` sidecar.
- **Pinned digest**: the archive digest the running launcher expects for an
  artifact, from its `expected_digests`. "Pinned" always means pinned for the
  archive's own artifact.
- **Keep set**: the digests a reap spares as resumable downloads. Today each
  caller passes only its own artifact's pinned digest, and an archive is
  spared when its name contains any keep-set digest. After this fix the keep
  set is replaced by the per-artifact resumable check below.
- **Resumable**: a partial archive whose digest is the one the running
  launcher pins for that archive's own artifact.
- **Orphan**: a temp tree, `.ref` sidecar or non-resumable partial archive
  that nothing will use again: either its artifact's tree lock is free, or it
  belongs to the reap's own artifact, whose caller has finished with it.
- **Retired artifact**: one with entries under `trees/` that the running
  launcher no longer pins. It gets no special handling and can meet any probe
  outcome below.

## Requirements

**Reproduction**: two artifacts X and Y materialise concurrently in one root.
Y finishes first and runs `reap_orphans` while X is still downloading to
`trees/.tmp-<X-name>-<platform>-<digest>.archive` or extracting to
`trees/.tmp-<X-generation>/`. The race reproduces without real concurrency:
create X's `.tmp-` entries, hold X's tree lock, then run a reap for Y.

**Actual**: Y's reap removes `.tmp-<X-generation>/` and
`.tmp-<X-generation>.ref` with no age or lock check, and X's partial
`.archive` because the keep set holds only Y's digest. X's materialise fails
or re-downloads.

**Expected outcome**: X's materialise completes unaffected by Y's reap; its
temp tree, `.ref` sidecar and partial archive survive until X's own reap.

**Expected rules**, for every reap whichever caller runs it. An artifact's
`.tmp-`
entries are its temp trees, their `.ref` sidecars and its partial archives;
lock files are never entries and no reap removes one.

- A reap handles its own artifact's entries as today, without probing its own
  tree lock, which its caller already holds: temp trees and `.ref` sidecars
  are removed, and a partial archive is removed unless it is resumable.
- For another artifact's entries, the reap tries that artifact's tree lock
  without blocking and without creating the lock file:
  - lock held by someone else: the entries are kept and the reap moves on to
    other artifacts;
  - lock taken: the reap holds it until it has removed that artifact's
    entries, then releases it;
  - no lock file: the entries are removed, with no lock held;
  - probe error: the entries are kept and the reap moves on to other
    artifacts.
- Two exceptions override the removal rules above:
  - a partial archive is kept when its digest is the one the running
    launcher pins for the archive's own artifact, whichever artifact the
    reap is for; a digest pinned only for some other artifact does not
    protect it;
  - an entry whose name parses to no known platform is kept.

## Acceptance Criteria

Every reap for Y runs with Y's tree lock held by its caller, as in
production, and under a bounded timeout, so a blocking probe fails rather than
hangs. Fixture entries are named with the production generation and archive
name builders. In every criterion, every `trees/.tmp-*.lock` file present
before the reap is still present after it.

In the criteria below, artifacts A and Z are bystanders whose names sort
before and after X's respectively; each has an orphan `.tmp-<generation>/`
and a lock file that exists and is held by no one.

- [ ] Given X's tree lock is held, `.tmp-<X-generation>/`,
      `.tmp-<X-generation>.ref` and a non-resumable
      `.tmp-<X-name>-<platform>-<digest>.archive` exist, and A's and Z's
      orphans exist, when a reap for Y runs, then it returns `Ok` while X's
      lock is still held, all three of X's entries survive, and A's and Z's
      orphans are removed.
- [ ] Given the same X entries with X's tree lock free, and a removal
      observer injected into the reap, when a reap for Y runs, then all three
      are removed; at every removal of an X entry, a non-blocking exclusive
      lock attempt on X's lock file from a separate open file description
      reports it held; and X's tree lock is free once the reap returns.
- [ ] Given a partial archive whose digest the running launcher pins for X,
      with X's tree lock free, when (a) a materialise of Y completes and (b)
      `prune` runs for Y, then the archive survives in both cases.
- [ ] Given X's tree lock is free and a partial archive named for X whose
      digest the running launcher pins only for Y, when a reap for Y runs,
      then the archive is removed.
- [ ] Given `.tmp-` entries for a retired artifact with no lock file, when a
      reap for Y runs, then the entries are removed and no lock file is
      created for that artifact.
- [ ] Given the non-creating probe returns an error for X's lock path (for
      example a directory in place of the lock file, which the probe opens
      for writing), X's `.tmp-` entries, and A's and Z's orphans, when a reap
      for Y runs, then X's entries survive, A's and Z's orphans are removed
      and the reap returns `Ok`.
- [ ] Given Y has a temp tree, its `.ref` sidecar, a non-resumable partial
      archive and a resumable partial archive, when a reap for Y runs, then
      the temp tree, sidecar and non-resumable archive are removed and the
      resumable archive survives.
- [ ] Given `.tmp-` entries for Y's artifact name on another platform whose
      tree lock is held, when a reap for Y runs, then they survive.
- [ ] Given `.tmp-` entries for Y's artifact name on another platform whose
      tree lock is free, when a reap for Y runs, then they are removed.
- [ ] Given Y's artifact name is `<Y-name>` and an artifact named
      `<Y-name>-extra` on the same platform holds its tree lock, when a reap
      for Y runs, then `<Y-name>-extra`'s `.tmp-` entries survive.
- [ ] Given two `.tmp-` entries whose names parse to no known platform — one
      containing no platform string, and one whose platform string is not
      preceded by an artifact name — when a reap for Y runs, then both
      survive.
- [ ] Given X's tree lock is held and X's `.tmp-` entries exist, when (a) a
      materialise of Y completes and (b) `prune` runs for Y, then X's entries
      survive in both cases.
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Open Questions

None.

## Dependencies

- **Blocks** 0218, whose cold-path root prune runs this reap far more often.
  0218 inherits the contract this fix relies on: a held tree lock means a
  materialise or `prune` of that artifact is in progress. When 0218 is
  planned, its root prune and orphan-lock removal must reuse this item's
  non-creating probe and never remove a tree lock file that is held. 0218
  already lists 0339 in `blocked_by`.
- **Decisions**: ADR-0072 already has the root prune take single-flight locks
  without blocking; this fix applies the same rule to the tree reap, so no ADR
  changes.
- **Relates to** 0334, which fixed lease handling on the generation path of
  the same reaper and in `prune`; this fix builds on that version of
  `reap.rs` and `prune`.
- **Relates to** 0222, whose offline populate route
  (`accelerator cache ensure --from`) is a second way to materialise a tree.
  It is protected by this fix only if it holds the artifact's tree lock from
  download through to its reap, as `acquire` does. 0222 owns that
  obligation; this item only records it.
- **Relates to** 0214, which settled the lease model.
- **Parent**: epic 0136.

## Assumptions

- An artifact name and platform can be read from a `.tmp-` entry's name
  using the known platform set as the separator, as in
  `<name>-<platform>-…`: the first `-<platform>-` occurrence after a non-empty
  artifact name decides the split. Artifact names never contain a known
  platform string. An entry that parses to no platform may belong to a newer
  launcher with a platform this one does not know, so it is kept; the residue
  is accepted.
- Protection holds only when every launcher reaping a root carries the fix. A
  root shared with an older launcher stays exposed until that launcher is
  replaced; the window is accepted.
- The protection relies on `flock` giving real mutual exclusion on the cache
  root's filesystem, as ADR-0072 already assumes for root prunes.
- With no lock file there is no lock to hold across removal, and the probe
  must not create one. A materialise of that artifact starting during the
  removal could lose a non-resumable partial archive it was about to resume;
  that window is accepted.

## Technical Notes

- The race: `reap.rs:62-87` removes every `TempTree` and `TempArchive` entry
  `classify` returns (`reap.rs:105-120`); `keep` holds one artifact's digest
  (`resolver.rs:604-605`, `:746-749`).
- The two callers: the post-materialise reap in `acquire`
  (`resolver.rs:603-610`) and `prune` (`resolver.rs:730-754`).
- The tree lock covers the whole materialise: `download::stream_archive`
  (`resolver.rs:251`) runs inside `materialise_locked` (`:186`), called under
  the lock taken at `:593`. A lock that can be taken therefore means no
  materialise or `prune` of that artifact is in progress.
- The non-blocking probe is `try_single_flight` (`tree/lease.rs:84-95`), but
  it opens the lock file with `create(true)` (`lease.rs:101`); probing a
  retired artifact through it would leave a lock file `classify` never
  removes.
- Every pinned digest is already reachable: the resolver's `expected_digests`
  answers `digest_for` for any artifact name and platform
  (`resolver.rs:130-131`).
- Holding each other artifact's lock across its removals means the reap
  groups entries by artifact and takes each lock once, so all of one
  artifact's entries share one outcome.

## Drafting Notes

- Chose a non-blocking tree-lock probe over an `AGE_BACKSTOP` age guard: a
  large download on a slow link can exceed an hour, and the lock states
  exactly what is in progress.
- The reap holds another artifact's lock across removal rather than probing
  and releasing it, so a materialise starting between probe and removal
  cannot lose its entries.
- Keeping another artifact's pinned partial archive while its lock is free
  goes beyond the race: deleting it corrupts nothing but loses a resumable
  download. It is bundled deliberately because it changes the same `keep`
  set and needs no new plumbing.
- Priority medium: not observed, and it needs two artifacts materialising at
  once, but when it fires it corrupts a fetch.
- Made it block 0218 rather than fold into it, so the fix lands in every root
  ahead of 0218's override-root work.

## References

- `cli/launcher/src/launch/outbound/resolve/tree/reap.rs` — the reaper
- `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs` — materialise,
  `prune` and the lock scope
- `cli/launcher/src/launch/outbound/resolve/tree/lease.rs` — the tree lock
- `meta/reviews/work/0218-bound-cache-root-growth-review-1.md` — where it was
  found
- `meta/decisions/ADR-0072-claim-gated-automatic-prune-for-relocated-cache-roots.md`
  — the non-blocking lock rule for root prunes
- `meta/reviews/work/0339-tree-reap-deletes-another-artifact-s-in-progress-materialise-review-1.md`
- Related: 0136, 0214, 0218, 0222, 0334
