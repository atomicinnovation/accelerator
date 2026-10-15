---
type: "work-item"
id: "0339"
title: "Tree reap deletes another artifact's in-progress materialise"
date: "2026-10-15T23:11:47+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "medium"
parent: "work-item:0136"
blocks: ["work-item:0218"]
relates_to: ["work-item:0334", "work-item:0214"]
external_id: "PP-922"
tags: ["cli", "launcher", "cache", "tree", "concurrency"]
last_updated: "2026-10-15T23:11:47+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0339: Tree reap deletes another artifact's in-progress materialise

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

`reap_orphans` runs after a materialise while holding only that artifact's
tree lock, but its scan removes every artifact's `.tmp-` entries under
`trees/`. A reap for one artifact can therefore delete another artifact's
half-extracted tree or partial download while that materialise is still
running. Found by code inspection during the 0218 review; not yet observed.

## Context

A materialise holds its artifact's tree lock
(`trees/.tmp-<name>-<platform>.lock`) from the start of the archive download
until the generation is renamed into place, then runs `reap_orphans`. Two
materialises of the same artifact serialise on that lock; two of different
artifacts — the browser and driver trees, for example — run concurrently.
The reap's scan covers the whole of `trees/`, not just its own artifact.

## Requirements

**Reproduction**: two artifacts X and Y materialise concurrently in one root.
Y finishes first and runs `reap_orphans` while X is still downloading to
`trees/.tmp-<X>-<platform>-<digest>.archive` or extracting to
`trees/.tmp-<X-generation>/`.

**Actual**: Y's reap removes X's `.tmp-<generation>/` and
`.tmp-<generation>.ref` with no age or lock check, and X's partial `.archive`
because `keep` holds only Y's digest. X's materialise fails or re-downloads.

**Expected**:

- A reap handles its own artifact's `.tmp-` entries as today.
- A reap removes another artifact's `.tmp-` entries only when it can take
  that artifact's tree lock without blocking; if the lock is held, it keeps
  them and continues.
- Orphan `.tmp-` entries of other artifacts, including retired ones, are
  still removed once their tree lock is free.
- A partial archive for a digest the running launcher pins for its artifact
  is kept as a resumable download, whichever artifact it belongs to.

## Acceptance Criteria

- [ ] Given X's tree lock is held and `trees/.tmp-<X-generation>/`,
      `.tmp-<X-generation>.ref` and `.tmp-<X>-<platform>-<digest>.archive`
      exist, when a reap for Y runs, then all three survive.
- [ ] Given the same entries with X's tree lock free, when a reap for Y runs,
      then the temp tree and `.ref` are removed, and the archive is removed
      unless the running launcher pins its digest.
- [ ] Given a partial archive for a digest the running launcher pins for X,
      with X's lock free, when a reap for Y runs, then it is kept.
- [ ] Given a reap for Y that finds X's tree lock held, then it neither waits
      nor fails, and completes.
- [ ] Given `.tmp-` entries for a retired artifact whose tree lock is free,
      when a reap runs, then they are removed.
- [ ] Given Y's own orphan `.tmp-` entries and a resumable Y archive, when a
      reap for Y runs, then the orphans are removed and the archive kept, as
      today.
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Open Questions

None.

## Dependencies

- **Blocks** 0218, whose cold-path root prune runs this reap far more often.
- **Relates to** 0334, which fixed lease handling on the generation path of
  the same reaper, and 0214, which settled the lease model.
- **Parent**: epic 0136.

## Assumptions

- An artifact and platform can be read from a `.tmp-` entry's name using the
  known platform set as the separator, as in `<name>-<platform>-…`; an entry
  whose name parses to no platform is treated as foreign and kept.

## Technical Notes

- The race: `reap.rs:62-87` removes every `TempTree` and `TempArchive` entry
  `classify` returns (`reap.rs:105-120`); `keep` holds one artifact's digest
  (`resolver.rs:604-605`, `:746-749`).
- The tree lock covers the whole materialise: `download::stream_archive`
  (`resolver.rs:251`) runs inside `materialise_locked` (`:186`), called under
  the lock taken at `:593`. A lock that can be taken therefore means no
  materialise of that artifact is in progress.
- The non-blocking probe is `try_single_flight` (`tree/lease.rs:84-95`).

## Drafting Notes

- Chose a non-blocking tree-lock probe over an `AGE_BACKSTOP` age guard: a
  large download on a slow link can exceed an hour, and the lock states
  exactly what is in progress.
- Priority medium: not observed, and it needs two artifacts materialising at
  once, but when it fires it corrupts a fetch.
- Made it block 0218 rather than fold into it, so the fix lands in every root
  ahead of 0218's override-root work.

## References

- `cli/launcher/src/launch/outbound/resolve/tree/reap.rs` — the reaper
- `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs` — materialise
  and the lock scope
- `meta/reviews/work/0218-bound-cache-root-growth-review-1.md` — where it was
  found
- Related: 0136, 0214, 0218, 0334
