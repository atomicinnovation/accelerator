---
type: "work-item"
id: "0340"
title: "Shared Cache Roots on Multi-User Systems"
date: "2026-10-21T18:42:16+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "epic"
priority: "low"
relates_to: ["work-item:0218", "adr:ADR-0072", "adr:ADR-0063", "adr:ADR-0064"]
tags: ["cli", "launcher", "cache", "multi-user", "security", "nfs"]
last_updated: "2026-10-21T18:42:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0340: Shared Cache Roots on Multi-User Systems

**Kind**: Epic
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

As a developer on a host shared with other developers, I want one
`ACCELERATOR_CACHE_DIR` that every authorised user can use and that stays
bounded, so that the team downloads and stores each release's sub-binaries
and runtime trees once rather than once per person. Today a shared root half
works: single-file sub-binaries are shared, runtime trees fail for every user
but the first, and the root is never pruned.

## Context

- Single-file sub-binaries are shared today: `store` writes `0600` then sets
  `0755` (`cache.rs:136-157`), and every use re-verifies against the shipped
  digest and signature.
- Runtime trees are not: `ensure_trees_dir` refuses a `trees/` that is not
  the effective user's and private (`tree/resolver.rs:949-965`,
  `is_privately_owned` at `layout.rs:349-361`), so a second user's
  materialise fails.
- 0218 prunes only a root writable by its owner alone, or through the
  owner's user-private group. The sticky bit stops renames but not other
  users planting names that the bootstrap's predictable writes follow
  (`bin/accelerator` `.accelerator-probe-$$` :184-190, `.staging.$$`
  :345-350, `.tmp-launcher-$$` :416-426; `cache_root.rs:131-137`), and
  macOS has no `protected_symlinks`. A shared root therefore works but is
  never pruned.
- No surveyed tool (ccache, git `--shared`, Nix, Homebrew, pip, uv, Bazel,
  Go, npm cacache, pnpm, conda) offers a daemonless, multi-user,
  tamper-resistant shared cache on NFS; the closest combine ccache's or
  git's group permissions with Go's or cacache's verify-against-trusted-digest
  model.

## Requirements

- **Trust model.** Anyone allowed to write the shared root may use it and
  may cause eviction: availability is shared. Integrity rests only on the
  digests and signatures that ship with the plugin; every read is verified
  against them, and no user trusts a file because another user wrote it.
- **Layout.** A group-shared root is a setgid directory, writable by a Unix
  group whose members are the authorised users. It may sit on local disk or
  NFS.
- Initial stories:
  1. Use a group-shared cache root safely.
  2. Prune a group-shared cache root.

## Acceptance Criteria

- [ ] Both child stories are done.
- [ ] `docs-site` documents the group-shared layout: creating the root
      (`2770`, group, setgid), the trust model, NFS mount requirements, and
      what remains unsupported.

## Open Questions

- Should a group-shared root ever be sticky (`3770`)? It would stop members
  deleting each other's entries but also stop pruning them; the stories
  assume not.

## Dependencies

- Blocked by: 0218 (through both children)

## Assumptions

- A shared root is owned by a dedicated Unix group; world-writable or
  sticky shared roots remain unsupported for pruning, as 0218 decided.
- A daemon or privileged owner (the Nix model) is out of scope.

## Drafting Notes

- 0218 stays under 0136: it bounds a single owner's relocated root and
  refuses multi-user roots; it is this epic's prerequisite, not its child.
- Priority low, no deadline, per the requester.

## References

- Source: `meta/plans/2026-10-19-0218-bound-cache-root-growth.md`, plan
  review `meta/reviews/plans/2026-10-19-0218-bound-cache-root-growth-review-1.md`
  (passes 3–4, decisions C and E)
- Related: 0218, 0334, 0339, 0222, 0223
- Research: ccache shared cache (https://ccache.dev/manual/latest.html),
  git `--shared` (https://git-scm.com/docs/git-init), Go module cache
  (https://go.dev/ref/mod), npm cacache (https://github.com/npm/cacache),
  Linux `fs.protected_*` (https://docs.kernel.org/admin-guide/sysctl/fs.html),
  `nfs(5)`, `flock(2)`
