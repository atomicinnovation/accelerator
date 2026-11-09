---
type: "adr"
id: "ADR-0074"
title: "Prune Only a Relocated Root No Other User Can Write To"
date: "2026-11-09T13:15:48+00:00"
author: "Toby Clemson"
producer: "create-adr"
status: "proposed"
parent: "work-item:0218"
relates_to: ["adr:ADR-0072"]
tags: ["architecture", "cache", "launcher", "retention", "security"]
last_updated: "2026-11-09T13:15:48+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# ADR-0074: Prune Only a Relocated Root No Other User Can Write To

**Date**: 2026-11-09
**Status**: Proposed
**Author**: Toby Clemson

## Context

ADR-0072 lets the launcher remove entries, trees and residue from a relocated
cache root named by `ACCELERATOR_CACHE_DIR`. It records that a root shared by
several users "is not protected": foreign-owned claims are ignored, so one
user's prune can evict another's live version.

Removal is the more serious exposure. A prune resolves names under the root
and unlinks or recursively removes them. If another user can create, rename or
remove names in the root, or in any directory above it, they can swap a
directory the prune is about to descend into for a symlink or a directory of
their choosing between the check and the removal. Every path the prune joins
then becomes a path that user chose.

Shared locations are common in practice: `/var/cache` set up for a team, CI
workspaces and container bind mounts owned by another uid, and home
directories on network or removable volumes that do not enforce ownership.
Accounts are increasingly resolved through directory services (SSSD, LDAP,
Active Directory, `systemd-homed`), which the shipped static Linux launchers
reach only through `getent`.

## Decision Drivers

- No removal may follow a path another user could have redirected.
- Anything the launcher cannot examine must be treated as unsafe, never as
  safe.
- A refused root must still work as a cache, and the refusal must name the
  directory at fault and its fix.
- The default root, the plugin's own `bin/`, keeps the behaviour ADR-0072
  fixed for it.
- The check must stay off the warm path.

## Considered Options

1. **Prune any relocated root (ADR-0072 as written)** — Ignore foreign-owned
   claims and accept that a shared root is unprotected.
2. **Honour other users' claims** — Treat foreign-owned claims as protecting,
   so a shared root is pruned only of what no user claims.
3. **Prune a per-user subdirectory automatically** — Relocate each user's
   entries under `<root>/<uid>/` and prune only that.
4. **Prune only a root no other user can write to** — Walk the root and every
   ancestor, as sshd's `StrictModes` does, and refuse to prune unless no other
   user can create, rename or remove names anywhere on that path.

## Decision

We will prune a relocated root only when a walk from the root to `/` shows
that no other user can create, rename or remove names on that path. Every
check fails closed.

- The root must be owned by the effective uid. An ancestor may also be owned
  by root or by a uid listed in `ACCELERATOR_CACHE_TRUSTED_OWNERS`; the
  overflow uids 65534 and 99 are never trusted, even when listed.
- No directory may be world-writable, except an ancestor with the sticky bit.
  The sticky bit stops renames, not another user creating a predictable name
  first, so it never excuses the root.
- A directory may be group-writable only through the user's own primary
  group, and only when that group is private: named for the user and listing
  no one else. The group is resolved through the host's account database, and
  through `getent` from a fixed absolute path, with a cleared environment, one
  overall deadline and a bounded answer, when that database is not reachable
  directly. An unresolved group refuses.
- An access ACL that may let another principal write refuses the directory. On
  macOS any allow entry beyond read-only permissions counts; a volume without
  ACL support has none. On Linux a POSIX ACL is judged through its mask, the
  group bits; an NFSv4 or rich ACL refuses.
- A directory on a volume that ignores ownership refuses.
- A directory whose metadata, ACL or volume cannot be examined refuses.

A prune builds a `SafePruneRoot` from this walk, and every removal in a
relocated root takes one, so no removal can run without it. The bookkeeping
directories inside a root (`claims/`, `trees/` and `trees/claims/`) are
judged by the same verdict the walk gives a root. Version claims are written
only in a root that passes the walk, and only when one is due, so the walk
never runs on a warm start with a young claim. An override that cannot yet be
told apart from the plugin's `bin/` is neither claimed nor pruned.

A refused root still works as a cache. A cold path logs the refusal once per
process, and `cache prune` exits non-zero, naming the directory at fault and
its remedy: `chmod g-w`, `chmod 700`, a listed owner, or a per-user directory
such as `ACCELERATOR_CACHE_DIR=<shared>/$USER`.

Option 1 leaves removals redirectable. Option 2 does not help, because the
danger is the path a removal follows, not which entries it chooses. Option 3
still prunes under a shared parent that another user can rename.

## Consequences

### Positive

- No removal in a relocated root follows a path another user could have
  redirected.
- The failure modes are explicit: each refusal names its cause and its fix.
- The trust rule exists once, shared by the walk and the bookkeeping
  directories.

### Negative

- Shared roots, sticky or not, and roots under directories owned by an
  unlisted uid are never pruned and grow as every root did before ADR-0072.
- CI and container users must set `ACCELERATOR_CACHE_TRUSTED_OWNERS` for
  pruning to work under a bind mount or workspace owned by another uid.
- Hosts whose `getent` lives outside `/usr/bin` and `/bin`, such as NixOS,
  cannot resolve a directory-service group and refuse a group-writable root.
- Another account whose primary group is the user's private group, without
  being listed in it, is not detected: finding one would mean enumerating
  every account, which a directory service makes unbounded.
- A refusal is spurious where an ACL grants a harmless permission or names
  the directory's own owner, because telling those apart would need resolving
  the principal.
- A due claim write walks every ancestor, costing a few `lstat`, ACL and
  `statfs` calls about once every 1.4 days.

### Neutral

- The default root keeps ADR-0072's behaviour: its tree prune is gated on its
  `trees/` directory only.
- A root made safe later starts its claim grace window, recorded as
  `claims/.since`, with its first claim.

## References

- `meta/decisions/ADR-0072-claim-gated-automatic-prune-for-relocated-cache-roots.md`
  — The prune whose multi-user exposure this closes
- `meta/work/0218-bound-cache-root-growth.md` — The work item that built it
- `meta/plans/2026-10-19-0218-bound-cache-root-growth.md` — The plan
