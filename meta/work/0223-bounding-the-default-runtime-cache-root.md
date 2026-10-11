---
type: "work-item"
id: "0223"
title: "Bounding the Default Runtime Cache Root"
date: "2026-08-20T00:00:00+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "done"
kind: "spike"
priority: "low"
parent: "work-item:0136"
relates_to: ["work-item:0196", "work-item:0218", "work-item:0330"]
tags: ["distribution", "runtime", "cache", "prune", "design"]
last_updated: "2026-10-11T12:52:42+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Converted to a spike and closed: ADR-0063 stands — no default-root reclamation, no cross-version adoption, no relocation to the plugin data directory."
schema_version: 1
external_id: "PP-753"
---

# 0223: Bounding the Default Runtime Cache Root

**Kind**: Spike
**Status**: Done
**Priority**: Low
**Author**: Toby Clemson

## Summary

Decide whether `accelerator cache prune` should reclaim the default cache root by
default, and whether a plugin upgrade should adopt the previous version's runtime
trees rather than re-materialising them. ADR-0063 delegates eviction there to
Claude Code's ~14-day orphan sweep, which leaves a user tracking prereleases
accumulating ~294MB per upgrade for up to a fortnight.

**Outcome**: neither. ADR-0063 stands unchanged. The orphan sweep is the right
evictor for the default root, and the only benefit of sharing trees across
versions — not re-fetching an unchanged artifact between prereleases — does not
justify the infrastructure it needs.

## Context

ADR-0063 chose to couple an artifact's lifetime to the plugin version's, so each
plugin version materialises its own copy and an upgrade re-fetches — the accepted
price of not building bespoke eviction. A frequently-upgrading prerelease user is
the accumulation case.

Digest keying (ADR-0064) makes cross-version adoption possible without a redesign:
an unchanged driver or browser is one tree per root, so an upgrade *could* adopt
the previous version's tree rather than re-fetching an identical one.

## Requirements

Research questions, all answered in Technical Notes:

- Does the default root need any eviction beyond Claude Code's orphan sweep?
- Should an upgrade adopt the previous plugin version's unchanged trees, and if
  so by what mechanism?
- Is default-root reclamation safe against a live daemon holding a lease?
- Does a per-plugin persistent directory change ADR-0063's analysis?

Time-box: conducted within a single work-item refinement session.

## Acceptance Criteria

- [x] Each research question has a recorded answer with evidence.
- [x] A decision on ADR-0063 is recorded: it stands, so no superseding ADR is
      written.
- [x] Defects found during the spike are captured in a follow-up work item
      (0330).

## Open Questions

None remaining. The lease question is answered in Technical Notes.

## Dependencies

- Relates to: 0196 (the vendored-runtime work), 0218 (bounding the launcher's
  single-file sub-binary cache in the same `bin/` directory), and ADR-0063.
- Blocks: nothing — no implementation follows from the outcome.
- Follow-up: 0330 (the lease defects found during this spike).

## Assumptions

- Anthropic will fix the known orphan-sweep defects, so the sweep's documented
  14-day bound holds in the long run. If the sweep stays broken, revisit.

## Technical Notes

### Claude Code's sweep already bounds the default root

The default root is `${ACCELERATOR_PLUGIN_ROOT}/bin` (`cache_root.rs:58-74`),
inside the version directory. On update or uninstall, Claude Code writes
`.orphaned_at` into the previous version directory and removes it in a background
sweep 14 days later, so sessions that loaded it keep running. Disk use is bounded
at ~294MB × versions in a 14-day window, on developer machines only; stable users
upgrade rarely. The sweep runs only while at least one plugin is installed, so
uninstalling a user's last plugin of any kind strands orphans until another is
installed — rare, and already accepted by ADR-0063.

Known sweep defects: anthropics/claude-code#82145 (open — leases from dead PIDs
block deletion) and #61855 (inconsistent orphan marking, closed not planned).

### Moving trees to `${CLAUDE_PLUGIN_DATA}` was considered and rejected

Claude Code provides `${CLAUDE_PLUGIN_DATA}` (`~/.claude/plugins/data/<id>/`),
persistent across upgrades and deleted on uninstall. That removes ADR-0063's
"shared store survives uninstall" objection, and with claims recording plugin
version, "current plus previous N" retention would work in a shared root.
Rejected because:

- The only benefit is skipping a re-fetch when consecutive prereleases pin the
  same digest — a developer-only convenience.
- `CLAUDE_PLUGIN_DATA` is not exported to Bash-tool commands, where
  `accelerator design` runs. Deriving it from the plugin root works for
  marketplace installs but not for a symlinked dev checkout, so locating it
  needs new infrastructure (e.g. the `SessionStart` hook recording it).

### Versioned trees with an "active" symlink do not fit

Proposed during the spike and rejected:

- Several plugin versions run concurrently (a pre-upgrade session keeps its
  version until `/reload-plugins`; daemons are reused across versions), so a
  single "active" pointer would be contended.
- Version-keyed trees duplicate unchanged digests; the digest-keyed `.ref`
  pointers already act as per-artifact active pointers.
- `layout.rs:268-286` deliberately refuses symlinked generations.
- Within a per-version root, a version label adds nothing — only one version
  writes there.

### Lease safety on reclamation

`prune` applies the same gate on every root — the exclusive single-flight lock,
held-lease skip, and claim window — with no default/relocated distinction. In a
per-version root, the tree in use is always the launcher's compiled-in digest,
which `prune` never reclaims. Two gaps matter only on a shared
`ACCELERATOR_CACHE_DIR` root and are carried to follow-up work item 0330:

- The cold path takes no lease: `cache ensure` prints the lease path, but no
  consumer opens it (`design-adapters/src/ensure.rs:102-118`).
- `reclaim_unclaimed` treats `Liveness::Unknown` as reclaimable, contradicting
  `lease.rs:8-10`.

### Correction

This item previously stated that `cache prune` reports the footprint across
sibling plugin-version roots. No code does; `prune` prints only
`reclaimed {n} entries` (`launch/cache.rs:153`). The claim originated in the 0196
design plan and was never implemented. With ADR-0063 standing, it is not needed.

## Drafting Notes

- Converted from `task` to `spike` because the remaining work was a decision, and
  the decision is to change nothing.
- Treated the orphan-sweep defects as Anthropic's to fix rather than as a reason
  to build our own eviction.
- Kept the rejected `${CLAUDE_PLUGIN_DATA}` and versioned-symlink designs in
  Technical Notes so they are not reopened without new facts.

## References

- Surfaced by:
  `meta/plans/2026-08-11-0196-design-vendored-runtime-distribution.md` (Removal
  sweep, follow-up work items)
- `meta/decisions/ADR-0063-plugin-version-scoped-artifact-cache.md`
- `meta/decisions/ADR-0064-producer-signed-tree-attestation-over-a-compiled-in-digest.md`
- Claude Code plugin loading reference:
  https://code.claude.com/docs/en/plugins/loading
- Claude Code plugins reference (`CLAUDE_PLUGIN_DATA`):
  https://code.claude.com/docs/en/plugins-reference
- anthropics/claude-code#82145, #61855
- Related: 0196; 0218; 0330; ADR-0063; ADR-0064
