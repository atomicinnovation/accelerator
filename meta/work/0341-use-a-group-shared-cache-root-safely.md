---
type: "work-item"
id: "0341"
title: "Use a Group-Shared Cache Root Safely"
date: "2026-10-21T18:42:37+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "low"
parent: "work-item:0340"
blocked_by: ["work-item:0218"]
relates_to: ["adr:ADR-0064"]
tags: ["cli", "launcher", "bootstrap", "cache", "multi-user", "security", "nfs"]
last_updated: "2026-10-21T18:42:37+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0341: Use a Group-Shared Cache Root Safely

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

As a developer sharing a cache root with my team, I want runtime trees and
sub-binaries that any group member materialised to work for me, without any
member being able to make me run something the plugin did not ship, so that
the team shares one copy of each artifact.

## Context

Trees are refused in a shared root today (`tree/resolver.rs:949-965`). A
tree hit verifies the signed attestation and the compiled-in digest
(ADR-0064, `resolver.rs:157-201`) but not each member file: the signed
`.files` table is walked only by `verify` (`resolver.rs:856-905`). That is
sound while only the user can write their tree; in a shared root another
member could replace one file after sealing and the hit would still pass.
The bootstrap's and launcher's predictable writers follow planted names.

## Requirements

- **Shared layout recognised.** A root that is setgid, group-writable
  (`2770`) and whose group the user belongs to is *group-shared*.
  `ensure_trees_dir` and every trust check accept a group-shared `trees/`,
  `claims/` and lock directory where they demand the effective uid today.
- **Explicit permissions.** The launcher and bootstrap set modes rather than
  relying on umask: directories `2770`, files `0660`, executables `0770`,
  lock files `0660` opened read-write; a sealed tree is read-only to the
  group after its rename.
- **Hardened writers.** Every predictable writer becomes unpredictable or
  exclusive, on the same filesystem: the bootstrap probe, shim staging and
  launcher staging (`mktemp "$dir/.x.XXXXXXXX"`, bash 3.2-safe, `chmod`
  before rename), and the launcher probe and `store`/materialise temp files
  (`O_CREAT|O_EXCL|O_NOFOLLOW` or random names).
- **Integrity from shipped digests only.**
  - Sub-binaries and launcher entries are verified on every use, as today.
  - A tree's member files are fully re-hashed against the signed `.files`
    table the first time each user uses that generation. The result is a
    per-user verification record under `${CLAUDE_PLUGIN_DATA}`, written
    `0600` with `O_NOFOLLOW` in a trusted `0700` directory, keyed by
    generation name and the generation directory's `(dev, ino)`, holding
    each member's `(dev, ino, size, mtime, ctime)`.
  - Later hits `lstat` each member and compare with the record; any
    difference re-runs the full re-hash, and a failed re-hash refuses and
    re-materialises the tree.
  - When `${CLAUDE_PLUGIN_DATA}` is unavailable, every hit of a shared tree
    runs the full re-hash and the user is warned once per process; a shared
    tree is never used unverified.
- **NFS.** Locks are whole-file `flock` on group-writable lock files; a lost
  or refused lock (`ENOLCK`) is a cache miss, never success. Mounts with
  `nolock` or `local_lock` give no cross-host exclusion and are documented
  as unsupported for concurrent use across hosts.
- **Measured cost.** First-use re-hash and per-hit stat cost for the largest
  tree are measured on local disk and NFS and recorded.

## Acceptance Criteria

- [ ] Given a `2770` setgid root owned by group `dev`, when user A
      materialises a tree and user B in `dev` dispatches its consumer, then
      B uses A's tree without re-downloading it.
- [ ] Given a shared tree one of whose member files another member replaced
      after sealing, when user B next uses it, then the stat check or full
      re-hash detects it, the tree is refused and re-materialised, and the
      replaced file is never executed.
- [ ] Given a per-user record for a generation, when B uses it again with no
      member changed, then no member is re-hashed and each member is
      `lstat`ed once.
- [ ] Given `${CLAUDE_PLUGIN_DATA}` unset, when B uses a shared tree twice,
      then both uses run the full re-hash and B sees one warning.
- [ ] Given a planted symlink or pre-created file at any name a writer would
      use, when the bootstrap or launcher writes, then it fails or picks a
      fresh name and the symlink's target is unchanged, for each writer, on
      macOS and Linux.
- [ ] Given a user whose umask is `077`, when they populate a shared root,
      then every entry they create is usable by other group members.
- [ ] Given a shared root on NFS where `flock` returns `ENOLCK`, when two
      users dispatch concurrently, then both succeed and neither corrupts
      the other's entries.
- [ ] Given a root that is not group-shared, when anything runs, then
      behaviour is exactly 0218's.

## Open Questions

- Which minimum Claude Code version sets `${CLAUDE_PLUGIN_DATA}` in the
  dispatch environment, and does the plugin's v2.1.144 floor guarantee it?
- Is the per-hit `lstat` walk acceptable for the largest tree on NFS, or
  does that case need a coarser check?

## Dependencies

- Blocked by: 0218
- Blocks: the pruning story under 0340

## Assumptions

- Any group member may delete or evict any entry; only integrity is
  protected.
- A full re-hash once per user per generation, with a stat check afterwards,
  is an acceptable cost.

## Drafting Notes

- Writer hardening sits here, not in the pruning story: those writers are
  unsafe in any shared root today, even without pruning.
- The per-user record trusts ctime, which no user can set; a write that
  leaves ctime unchanged is not possible without privilege.
- The record lives under `${CLAUDE_PLUGIN_DATA}` at the requester's choice,
  over `${XDG_CACHE_HOME}`; the unset case fails closed.

## References

- ADR-0064 (tree attestation), ADR-0063, ADR-0072
- Related: 0218, 0334, 0186, 0191
