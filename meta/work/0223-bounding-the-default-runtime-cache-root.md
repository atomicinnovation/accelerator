---
type: "work-item"
id: "0223"
title: "Bounding the Default Runtime Cache Root"
date: "2026-08-20T00:00:00+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "low"
relates_to: ["work-item:0196"]
tags: ["distribution", "runtime", "cache", "prune", "design"]
last_updated: "2026-08-20T00:00:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-753"
---

# 0223: Bounding the Default Runtime Cache Root

**Kind**: Task
**Status**: Draft
**Priority**: Low

## Summary

Decide whether `accelerator cache prune` should reclaim the default cache root by
default, and whether a plugin upgrade should adopt the previous version's runtime
trees rather than re-materialising them. ADR-0063 delegates eviction there to
Claude Code's ~14-day orphan sweep, which leaves a user tracking prereleases
accumulating ~294MB per platform per upgrade for up to a fortnight with no
ceiling.

## Context

ADR-0063 chose to couple an artifact's lifetime to the plugin version's, so each
plugin version materialises its own copy and an upgrade re-fetches — the accepted
price of not building bespoke eviction. `cache prune` already reports the
footprint across sibling plugin-version roots, so the growth is at least visible.
But visibility is not a ceiling, and a frequently-upgrading prerelease user is
exactly the accumulation case.

Digest keying (ADR-0064) makes cross-version adoption possible without a redesign:
an unchanged driver or browser is one tree per root, so an upgrade *could* adopt
the previous version's tree rather than re-fetching an identical one.

## Requirements

- Decide whether `cache prune` reclaims the default root by default, and if so,
  under what retention rule.
- Decide whether an upgrade adopts the previous plugin version's unchanged trees
  rather than re-materialising them.
- Record the decision (likely a superseding or amending ADR to ADR-0063) and
  implement whichever behaviours are chosen.

## Acceptance Criteria

- [ ] Given a decision, then ADR-0063's default-root reclamation and
      cross-version-adoption stance is recorded and, where changed, superseded.
- [ ] Given the chosen prune policy, when `cache prune` runs on an accumulated
      default root, then the footprint is bounded per the recorded rule.

## Open Questions

- Is a default-on reclamation of the default root safe against a live daemon
  holding a lease there, or does it need the same `flock` gate as the relocated
  root?

## Dependencies

- Relates to: 0196 (the vendored-runtime work) and ADR-0063 (the cache-root
  placement decision this would amend).

## References

- Surfaced by:
  `meta/plans/2026-08-11-0196-design-vendored-runtime-distribution.md` (Removal
  sweep, follow-up work items)
- Related: 0196; ADR-0063; ADR-0064
