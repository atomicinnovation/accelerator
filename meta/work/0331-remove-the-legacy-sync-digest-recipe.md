---
type: "work-item"
id: "0331"
title: "Remove the Legacy Sync Digest Recipe"
date: "2026-10-10T21:48:33+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "low"
parent: "work-item:0146"
derived_from: ["plan:2026-10-10-0296-sync-round-trip-defects"]
relates_to: ["work-item:0296", "work-item:0290"]
tags: ["sync", "baseline", "digest"]
last_updated: "2026-10-10T21:48:33+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0331: Remove the Legacy Sync Digest Recipe

**Kind**: Task
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

0296 introduces digest recipe `1` and keeps classifying unmarked
`last-sync.json` entries with the legacy recipe (combined frontmatter and
body, per-line whitespace only). Once no unmarked entry remains in use,
remove the legacy recipe, its entry shape and the code that dispatches on it.

## Context

Every sync run rewrites each entry it classifies under the current recipe,
so one full sync migrates every entry except those it does not classify:
`remote-absent`, `indeterminate` and unresolved conflicts. Pending-push
markers written before 0296 also carry a legacy `local_hash`. The legacy
path is dead weight once those are gone.

## Requirements

- Remove the legacy `Entry` variant, `digest::local`'s legacy recipe and the
  legacy branches of classification and the pending-push marker reader.
- An entry with no recipe marker, after removal, reads as having no local
  hash and goes through 0296's recovery path rather than failing the run.
- Keep the `work-item-sync-baseline` oracle cases only where they pin
  behaviour that still exists.

## Acceptance Criteria

- [ ] Given a `last-sync.json` entry with no `digest_recipe`, when
      `work sync` runs, then the item is classified through the recovery
      path (equivalent bodies are baselined, others conflict) and the run
      does not fail.
- [ ] Given a pending-push marker with a legacy `local_hash`, when the
      promotion resumes, then it completes or reports a recoverable error.
- [ ] Given the codebase, when searched for the legacy recipe's entry
      variant and digest function, then neither exists.
- [ ] Given this repo's `last-sync.json` before merge, when inspected, then
      no entry lacks `digest_recipe`.

## Open Questions

- What evidence is enough to say other users' corpora have migrated: a
  release-count window, or a warning shipped first?

## Dependencies

- Blocked by: 0296.
- Related: 0290, whose recipe bump defines how superseded markers classify.

## Assumptions

- Users run a full sync at least once between 0296's release and this
  item's release.

## Technical Notes

- Legacy dispatch points after 0296: `cli/work-adapters/src/sync/baseline.rs`
  (`parse_entry`), `cli/work/src/sync/recipe.rs`, `cli/work/src/sync/classify.rs`,
  `cli/work-adapters/src/sync/pending_push.rs`.

## Drafting Notes

- 0296's review and PR #156 noted that no item existed for this removal.

## References

- Source: `meta/plans/2026-10-10-0296-sync-round-trip-defects.md`
- Related: 0296, 0290
