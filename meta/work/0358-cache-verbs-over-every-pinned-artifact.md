---
type: "work-item"
id: "0358"
title: "Cache Verbs over Every Pinned Artifact"
date: "2026-10-27T18:01:29+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0355"
blocks: ["work-item:0222"]
blocked_by: ["work-item:0356"]
tags: ["runtime", "cache", "cli"]
last_updated: "2026-10-27T18:01:29+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-950"
---
# 0358: Cache Verbs over Every Pinned Artifact

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As an operator warming a cache, I want `accelerator cache ensure` (and `verify`
and `repair`) to work over every pinned artifact — sub-binaries as well as
trees — and to warm everything when no artifact is named, so that one command
prepares a cache root for a session.

## Context

`cache ensure` accepts only tree artifact names, requires at least one, and
prints `<name>\t<tree path>\t<lease path>` for the design executor, which holds
the lease. `verify` and `repair` already treat an omitted name as "every
artifact", but only over trees. Once sub-binaries are pinned, the launcher
knows the full pinned set for its platform, so the verbs can cover it
uniformly. The output contract is unreleased, so it can change.

## Requirements

- `cache ensure [<artifact>...]` accepts any pinned artifact name for the host
  platform; with no names it ensures every pinned artifact.
- `cache verify` and `cache repair` cover sub-binaries by re-hashing against
  the pin.
- `cache ensure` prints one line per artifact: name, outcome (`present`,
  `fetched`, or a later source-specific outcome), and resolved path; failures
  keep the structured `ensure-failed` envelope and the existing `EnsureCause`
  tokens.
- The design executor and any other consumer of `cache ensure` output move to
  the new contract in the same change, including how it obtains the tree lease.
- An unknown name is refused before any filesystem access, as today.

## Acceptance Criteria

- [ ] Given an empty cache root, when `cache ensure` runs with no names, then
      every pinned sub-binary and tree for the host platform is present and
      `cache verify` passes.
- [ ] Given a populated cache, when `cache ensure` runs again, then every line
      reports `present` and no network call is made.
- [ ] Given `cache ensure research browser`, then only those two artifacts are
      ensured.
- [ ] Given a sub-binary whose cached bytes no longer match the pin, then
      `cache verify` reports it and `cache repair` replaces it.
- [ ] Given the design executor, then it consumes the new output contract and
      its existing lease and downgrade behaviour is unchanged.

## Open Questions

- Does the executor keep receiving a lease path from `cache ensure`, or does it
  acquire the lease itself once the output carries per-artifact outcomes?

## Dependencies

- Blocked by: pinning sub-binary digests into the launcher.
- Blocks: 0222 (seed the runtime cache from a staged release directory).

## Technical Notes

- `cli/launcher/src/launch/cache.rs` (`targets`, `ensure`, `ensure_envelope`)
  and `inbound/cli.rs` (`CacheAction::Ensure`, `required = true`).

## Drafting Notes

- Kept the verb as `cache ensure` rather than adding `cache warm`, and broke
  its output contract, per the author: the contract has not been released.

## References

- `cli/launcher/src/launch/cache.rs`, `cli/launcher/src/launch/core/tree.rs`
  (`EnsureCause`).
