---
type: "work-item"
id: "0342"
title: "Prune a Group-Shared Cache Root"
date: "2026-10-21T18:42:51+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "low"
parent: "work-item:0340"
blocked_by: ["work-item:0218", "work-item:0341"]
tags: ["cli", "launcher", "cache", "prune", "multi-user", "nfs"]
last_updated: "2026-10-21T18:42:51+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0342: Prune a Group-Shared Cache Root

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

As a developer sharing a cache root with my team, I want versions and trees
nobody in the group still uses to be reclaimed, while anything a member is
using survives, so that the shared root stays bounded as 0218 bounds a
private one.

## Context

0218 prunes only an owner-only root and counts only claims owned by the
effective uid. Once 0341 hardens every writer, a group-shared root no longer
has the planted-name exposure that made 0218 refuse it.

## Requirements

- Claims written by any group member count when they are group-owned
  regular files in a group-shared `claims/` or `trees/claims/`; one member's
  claim protects a version or tree for all.
- A group-shared root passes the prune safety rule; every other 0218 safety
  check (canonical path, ancestors, trusted directories, pointer
  containment) still applies.
- A reader holds a shared lock on a version's lock file while using it;
  prune takes the exclusive lock (`Try` on cold paths), re-checks the claim,
  renames the generation to `trees/trash/<random>`, and removes the trash
  best-effort on a later prune, tolerating NFS `.nfs*` silly-renamed files
  and `ENOTEMPTY`.
- Age comparisons allow for clock skew between NFS server and client;
  thresholds stay in hours or days.

## Acceptance Criteria

- [ ] Given versions A and B in a group-shared root, B claimed by user X and
      A by nobody, when user Y's cold path prunes, then A's entries go and
      B's stay.
- [ ] Given a version X is using under a shared lock, when Y prunes, then
      that version and its trees survive until X releases the lock.
- [ ] Given a pruned tree with a file still open on an NFS client, when
      prune runs, then the generation leaves the index at once and its trash
      is cleaned on a later prune without errors.
- [ ] Given a claim whose mtime is ahead of the client clock by less than
      the skew margin, when prune runs, then the claim counts as fresh.
- [ ] Given a root that is not group-shared, when anything runs, then
      behaviour is exactly 0218's.

## Open Questions

- What skew margin should the age comparisons use?

## Dependencies

- Blocked by: 0218, 0341

## Assumptions

- Group members are trusted for availability: one member's prune may evict
  content another member has not claimed.

## Drafting Notes

- Rename-to-trash replaces 0218's direct `remove_tree` only in group-shared
  roots, where an NFS reader on another client would otherwise see `ESTALE`
  mid-use.

## References

- 0218 plan and review; uv cache locking
  (https://docs.astral.sh/uv/concepts/cache/); Nix GC roots
  (https://nix.dev/manual/nix/stable/package-management/garbage-collector-roots)
