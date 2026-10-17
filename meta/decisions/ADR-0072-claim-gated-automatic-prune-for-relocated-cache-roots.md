---
type: "adr"
id: "ADR-0072"
title: "Claim-Gated Automatic Prune for Relocated Cache Roots"
date: "2026-10-15T22:19:18+00:00"
author: "Toby Clemson"
producer: "create-adr"
status: "accepted"
parent: "work-item:0218"
relates_to: ["adr:ADR-0063", "adr:ADR-0064", "work-item:0223", "work-item:0334"]
tags: ["architecture", "cache", "launcher", "retention"]
last_updated: "2026-10-15T22:19:18+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# ADR-0072: Claim-Gated Automatic Prune for Relocated Cache Roots

**Date**: 2026-10-15
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0063 keeps artifacts in the per-version plugin root and delegates their
eviction to Claude Code's orphan sweep. For the two roots the sweep cannot
reach — a relocated `ACCELERATOR_CACHE_DIR` and a symlinked development
checkout — it makes `accelerator cache prune` the evictor, "a user-invoked
verb, not a scheduled collector", and counts it a benefit that prune "stays a
convenience verb rather than becoming correctness-critical".

A relocated root is shared across plugin upgrades. Each version adds its
launcher entry, sub-binaries, `.minisig` sidecars and staged verifier copies;
the vendored runtime trees add pointers, generations, claims, sidecars and
per-artifact lock files; crashes leave probe files and lock directories.
Nothing removes the single-file entries, and the verb's tree reclaim covers
only currently pinned artifacts on the host platform (0223, 0218). Nothing
runs the verb automatically, so in practice the root grows without bound.

Several plugin versions dispatch against one root at once: a session keeps
its version until `/reload-plugins`. Any eviction policy must therefore know
which versions are live, not merely which are newest. The tree cache already
answers that question with retention claims — per-digest files a launcher
refreshes while it uses a tree, ignored once older than `CLAIM_WINDOW` (14
days) — and with `flock` leases that mark a tree in use (ADR-0064).

ADR-0063 rejected per-version ref-counting (its Option 3) because a ref
cannot remove itself once its version is gone, so correctness depends on
another version running later. A claim differs: it expires by age, so a
departed version stops protecting its entries without any action of its own.

## Decision Drivers

- Bound a relocated root's disk use without the user running anything.
- Never evict what a live session on any version still uses.
- Reuse the tree cache's retention model rather than design a second one.
- Leave the default root and development checkouts exactly as ADR-0063
  decided, so the default warm path gains no cost.

## Considered Options

1. **Verb only (ADR-0063 as it stands)** — `accelerator cache prune`
   remains the only evictor for a relocated root, run by hand.
2. **Keep the newest N versions** — evict every version outside the N most
   recent.
3. **Evict during `store`** — each `store` removes other versions' entries
   as it writes its own.
4. **Size-cap LRU** — cap the root's size and evict least-recently-used
   entries, skipping leased trees (ADR-0063's Option 2).
5. **Claim-gated automatic prune on cold paths** — every dispatch in a
   relocated root refreshes a per-version claim; every cold path (a
   successful `store`, a tree materialise) runs a prune that removes only
   what no fresh claim or held lease protects.
6. **Claim-gated prune from the `SessionStart` hook** — the same claims and
   prune, run once per session by a hook alongside the ones that already
   invoke the launcher, instead of on cold paths.

## Decision

We will make pruning automatic and claim-gated for relocated roots
(Option 5). In a root set by `ACCELERATOR_CACHE_DIR` that is not
`${ACCELERATOR_PLUGIN_ROOT}/bin`:

- each dispatch refreshes a version claim, throttled as tree claims are;
- each cold path runs a root prune under single-flight locks, tried without
  blocking, removing only an allowlisted set of plugin-written shapes whose
  version or digest has no fresh claim, whose lease is not held, and which
  are older than `AGE_BACKSTOP` where crash residue or pre-claim writes are
  concerned;
- `accelerator cache prune` runs the same prune, waiting for locks.

This amends ADR-0063 for relocated roots only. ADR-0063 continues to govern
the default per-version root and development checkouts, where dispatches
neither claim nor prune and the verb behaves as today. Tree prune stays
within ADR-0064's lease and layout-version rules.

Option 1 leaves growth unbounded for the users who relocate the cache.
Option 2 evicts versions that live sessions still dispatch. Option 3 cannot
work: `store` knows only its own version, so it cannot tell which others are
live. Option 4 needs a cap policy, an eviction order and a last-use signal,
the collector ADR-0063 declined to build, and still has no notion of a live
version. Option 6 keeps prune off the dispatch path but adds its cost to
every session start, and a session left open for weeks never prunes; the cold
path instead prunes exactly when the root grows, on a path already paying for
a fetch.

## Consequences

### Positive

- A relocated root is bounded without user action, to the versions and trees
  used within `CLAIM_WINDOW` plus residue younger than `AGE_BACKSTOP`.
- One retention model covers trees and single-file entries.
- The default root's warm path is unchanged; only an override root pays for
  claim checks.

### Negative

- Prune becomes correctness-critical in relocated roots, which ADR-0063
  avoided: a bug in it can remove an entry a live dispatch needs. The
  bounded `ENOENT` re-resolve mitigates this but adds launcher code.
- A version idle for longer than `CLAIM_WINDOW` loses its entries and
  re-fetches on its next dispatch.
- Launchers that predate version claims have no claim; their entries are
  evicted on the first prune, and one caught between `find` and `exec` fails
  that dispatch.
- A root shared by several users is not protected: foreign-owned claims are
  ignored, so one user's prune can evict another's live version.
- Removing a superseded tree adds latency to the cold path that triggers it,
  about once per superseded tree.
- A final uninstall still leaves a relocated root unpruned.

### Neutral

- Development checkouts remain unbounded for single-file entries, as
  ADR-0063 already accepts.
- Correctness depends on `flock` on the relocated root's filesystem; where it
  fails for a reason other than contention, the cold path skips that work.

## References

- `meta/work/0218-bound-cache-root-growth.md` — the work item implementing
  this decision
- `meta/decisions/ADR-0063-plugin-version-scoped-artifact-cache.md` —
  amended for relocated roots
- `meta/decisions/ADR-0064-producer-signed-tree-attestation-over-a-compiled-in-digest.md`
  — lease and layout-version rules tree prune follows
- `meta/work/0223-bounding-the-default-runtime-cache-root.md` — settled the
  default root
- `meta/work/0334-tree-leases-are-not-held-on-the-cold-path-or-honoured-when.md`
  — lease fixes this decision relies on
