---
type: adr
id: "ADR-0063"
title: "Plugin-Version-Scoped Artifact Cache"
date: "2026-08-17T12:21:50+00:00"
author: Toby Clemson
producer: create-adr
status: accepted
relates_to: ["adr:ADR-0046", "adr:ADR-0054", "adr:ADR-0061", "adr:ADR-0062",
  "work-item:0196", "work-item:0210"]
tags: [architecture, distribution, cache, launcher, plugin, design]
last_updated: "2026-08-17T12:21:50+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# ADR-0063: Plugin-Version-Scoped Artifact Cache

**Date**: 2026-08-17
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0061 addresses tree artifacts by content digest, platform and generation, and
notes that this creates a reclamation obligation it does not discharge. Two trees
per platform are roughly 294MB, so where they live and who deletes them is a
decision in its own right.

The default cache root is `${ACCELERATOR_PLUGIN_ROOT}/bin`
(`cache_root.rs:45-46`), inside the versioned plugin directory. There is
deliberately no XDG fallback: the skills' `allowed-tools` grants name that path —
`Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator design *)` — so a sub-binary resolved
elsewhere would not match the grant. That constraint binds the **launcher and its
sub-binaries**, which Claude Code's Bash tool invokes directly. It does not bind
trees: a tree's Node binary is spawned by `accelerator-design`, itself already
granted, so trees could live anywhere.

Claude Code documents the plugin version directory as ephemeral and gives it an
evictor. An update or uninstall marks the previous version orphaned, and a
background sweep removes it roughly **14 days** later — a window that is not
configurable, and that exists so sessions which already loaded the old version
keep working. Two cases fall outside the sweep: a symlinked development checkout,
which is never marked orphaned, and a relocated root under
`ACCELERATOR_CACHE_DIR`, which the sweep never sees.

Observed on one developer machine at a fast pre-release cadence: eight versions
retained across twelve days, the oldest exactly at the fourteen-day boundary, with
194MB of single-file sub-binaries across their `bin/` directories. The sweep is
working as documented; the retention is the grace window, not an absence of
cleanup.

## Decision Drivers

- **Every supported cache root needs a named evictor**, so that "who deletes this"
  is never unanswered.
- **An artifact's lifetime should be coupled to something that ends.** A cache
  whose lifetime is nothing in particular is a cache nobody reclaims.
- **A tree must not be mutated or adopted by a launcher version other than the one
  that wrote it**, unless something makes that safe.
- **Download, disk and hashing cost are explicitly lower-priority** than lifecycle
  clarity here. Stable releases are infrequent, and the pre-release path that pays
  most is a development path.
- **Prefer delegating to a mechanism that already works** over building an
  equivalent.

## Considered Options

1. **Per-version root, eviction delegated** — keep trees in
   `${ACCELERATOR_PLUGIN_ROOT}/bin`; Claude Code's orphan sweep reclaims them;
   `accelerator cache prune` covers the roots the sweep cannot reach.
2. **Shared root, size-cap LRU** — one version-independent store, capped, evicting
   least-recently-used trees and skipping leased ones, with the lease file's mtime
   as the last-use signal.
3. **Shared root, liveness-checked ref-counting** — each plugin version records a
   ref naming its own plugin root; refs whose root has vanished are dropped, and a
   zero-ref tree is reaped.
4. **Shared root, keep-current-pin-plus-previous** — retain the tree for the
   current pinned digest and the one before it, reap the rest.
5. **Split root** — sub-binaries stay under the plugin root to satisfy the
   `allowed-tools` grant; trees alone move to a shared store.

## Decision

We will keep tree artifacts in the **per-plugin-version cache root** and
**delegate their eviction to Claude Code's orphan sweep**, with `accelerator cache
prune` as the evictor for the two roots that sweep does not reach.

- **A tree's lifetime is its plugin version's.** Each version materialises its own
  copy, and an upgrade re-fetches. Nothing is shared between versions, so nothing
  outlives the plugin except in the two cases named below.
- **We add no eviction mechanism for the default root.** The sweep is the evictor,
  and a re-derivable content-addressed cache is precisely what belongs in a
  directory documented as ephemeral. This is delegation, not an assumption: the
  behaviour is documented, and the grace window's stated purpose is to protect
  sessions that already loaded the version.
- **`accelerator cache prune` owns the two roots outside the sweep** — a relocated
  `ACCELERATOR_CACHE_DIR` and a symlinked development checkout. It is a
  user-invoked verb, not a scheduled collector.
- **Cross-version tree adoption cannot arise by default**, because no two plugin
  versions share a root. ADR-0061's layout version therefore protects the
  escape-hatch cases rather than the common one, and remains required for them.

Option 2 was rejected because it requires building and maintaining a collector —
cap policy, eviction order, a last-use signal — to buy efficiency this decision
deprioritises, and because a shared store survives plugin uninstall with nothing
left running to reclaim it.

Option 3 was rejected for the same orphan-on-uninstall reason plus per-version
bookkeeping: a ref cannot remove itself once its own version directory is gone, so
correctness depends on another version running later.

Option 4 was rejected as actively wrong here. "Current pin" is a compiled-in
constant per plugin version, so with several versions coexisting each would reap
the others' trees and all of them would refetch.

Option 5 was rejected because the `allowed-tools` constraint does not force it —
trees are not Bash-invoked — so the split buys cross-version sharing while
incurring every shared-store cost, and adds a second root to validate and own.

## Consequences

### Positive

- Every root has exactly one named evictor, and which one is recorded rather than
  assumed.
- Removing a plugin version reclaims its artifacts, so the disk cost of an
  abandoned version is bounded by the grace window rather than by our diligence.
- No garbage collector to design, test or get wrong; `prune` stays a convenience
  verb rather than becoming correctness-critical.
- One cache root, so one ownership check, one path validation, and no split-root
  branching.
- External reclamation will not pull a tree from a session that already loaded it,
  within the grace window — protecting exactly that is the window's stated purpose.

### Negative

- Each version materialises its own copy, so an upgrade re-fetches ~294MB even
  when the pinned artifact is unchanged, and every orphaned version holds its copy
  for the whole grace window — seven orphans in the observed twelve-day window
  would be roughly 2GB transient. Accepted as the price of lifecycle coupling.
- The sweep does not run once the last plugin is uninstalled, so a final uninstall
  strands the cache until a plugin is installed again. It is the one case where
  removing the plugin does not reclaim the artifact.
- A symlinked development checkout is never marked orphaned, so it receives no
  sweep and grows until `prune` is run. The workflow least likely to run `prune`
  is the one that most needs it.
- The retention policy and the grace window are another product's, not
  configurable by us, and can change without a signal on our side.
- `prune` cannot bound total growth on the default root, since it only ever sees
  one version's root.
- A session alive beyond the grace window on an already-orphaned version outlives
  the protection the window gives, and would have its tree swept from under it. A
  new invocation re-materialises, so the damage is bounded to whatever is in
  flight, but the guarantee is time-limited rather than absolute.

### Neutral

- `ACCELERATOR_CACHE_DIR` remains supported, and is where content addressing's
  cross-version reuse actually pays — at the cost of owning eviction there.
- Whether `prune` reports the abandoned legacy Playwright cache is an independent
  question.
- Nothing here constrains single-file sub-binaries, whose placement is already
  fixed by the `allowed-tools` grant.

## References

- ADR-0061 (signed content-addressed tree generations) — creates the reclamation
  obligation this ADR assigns, and owns the layout version
- ADR-0046 (zero-setup static binary distribution), ADR-0054 (git-style modular
  CLI), ADR-0062 (browser automation's platform boundary)
- Claude Code plugin caching, the orphan sweep and its grace window, and the
  instruction to treat the plugin root as ephemeral:
  https://code.claude.com/docs/en/plugins-reference#plugin-caching-and-file-resolution
- `cli/launcher/src/launch/outbound/resolve/cache_root.rs` — cache-root selection
  and the `allowed-tools` reason there is no XDG fallback
- `meta/work/0210-settle-the-vendored-runtime-tree-artifact-mechanisms.md`
