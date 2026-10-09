---
type: "work-item"
id: "0315"
title: "Unknown Applied Migration IDs Silence the Lag Advisory"
date: "2026-10-09T09:34:53+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "medium"
parent: "work-item:0312"
relates_to: ["work-item:0298", "work-item:0265"]
tags: ["migration-engine", "discoverability", "hooks"]
last_updated: "2026-10-09T09:34:53+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0315: Unknown Applied Migration IDs Silence the Lag Advisory

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The SessionStart lag advisory compares the highest applied migration ID
against the highest bundled one. An unknown ID, such as one left by a newer
plugin before a downgrade, is counted, so it can suppress the advisory while
bundled migrations are still pending.

## Context

`highest_applied` takes the lexicographic maximum of every ledger line,
including unknown IDs (`cli/migrate-cli/src/discoverability.rs:29-37`).
ADR-0023 and the runner both preserve unknown IDs on purpose, so this case
is expected to arise.

## Requirements

Reproduction (verified at revision `e76a8a9b`):

1. In a directory with `meta/`, write
   `.accelerator/state/migrations-applied` containing
   `0001-rename-tickets-to-work` and `9999-from-a-newer-plugin`.
2. Run `accelerator migrate --discoverability-hook --format=hook --fail-safe`.

Expected: the advisory reports that the repo is behind, because bundled
migrations 0002–0010 are unapplied.
Actual: no output.

Fix: compute lag over IDs the registry knows only.

## Acceptance Criteria

- [ ] Given an applied ledger holding a known ID below the highest bundled
      ID plus an unknown ID sorting above it, when the SessionStart hook
      runs, then the lag advisory is emitted.
- [ ] Given an applied ledger holding every bundled ID plus an unknown ID,
      when the hook runs, then no advisory is emitted.
- [ ] Given an applied ledger holding only unknown IDs, when the hook runs,
      then the advisory is emitted.

## Open Questions

None.

## Dependencies

- Blocked by: none.
- Blocks: none. 0298 may move the hook's entry point, but not this rule.

## Assumptions

- "Behind" means some bundled migration is unapplied by highest-known-ID
  comparison, not a full set difference. This keeps ADR-0023's lag rule.

## Technical Notes

- Unknown IDs are already warned about at run time (`ledger::warnings`).
  The advisory does not need to mention them.

## Drafting Notes

- A full set difference (any bundled ID unapplied) was considered, but it
  changes the recorded rule. A skipped or no-op-pending migration would then
  keep the advisory on permanently.

## References

- `cli/migrate-cli/src/discoverability.rs`
- Related: 0298, 0265
