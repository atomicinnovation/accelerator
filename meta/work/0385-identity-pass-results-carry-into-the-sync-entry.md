---
type: "work-item"
id: "0385"
title: "Identity Pass Results Carry Into the Sync Entry"
date: "2026-11-16T11:17:16+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "low"
parent: "work-item:0372"
blocked_by: ["work-item:0384"]
relates_to: ["work-item:0352"]
tags: ["sync", "identity", "promotion", "architecture"]
last_updated: "2026-11-16T11:17:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-976"
---
# 0385: Identity Pass Results Carry Into the Sync Entry

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

As a maintainer of the sync engine, I want the identity pass to be a typed
stage whose result for each item travels with that item into the engine's
plan, so that an item whose identity changed is reported once without the
renderer merging two sources.

## Context

`run_settled` runs the identity pass before the engine: it resumes
interrupted retirements, reads the remote once, decides key changes and
promotions, bounds them, applies them, rediscovers the corpus and hands the
engine what is left. Its rows (`IdentityRow`, `PromotionRow`) are attached
to the report beside the engine's.

`render_report` then drops an engine row whose id matches an identity row's
settled id, unless it failed, and the identity line borrows that row's
state. `plannable` withholds from the engine every item with an unapplied
key change, every draft the run set out to promote and every item just
promoted, so those items have no entry at all.

## Requirements

- The identity pass is a stage with its own closed set of entries: key
  change, promotion, resumed retirement.
- A subject's key survives its id being retired onto a tracker key.
- A settled subject's sync entry carries its identity result, and the item
  prints one row.
- An item the engine is told to leave alone has an entry saying so.
- "Issue not found" is a fact on the item's sync entry.
- A stage that stops reports the changes that had already landed, as today.
- Stdout is unchanged unless the ADR states otherwise.

## Acceptance Criteria

- [ ] `render_report` suppresses no row by comparing ids across sources.
- [ ] Given a key change that lands and an engine action on the same item,
      when the run reports, then the item has one row, as today.
- [ ] Given a draft promoted in the run, when the run reports, then its
      promotion row is as today and the item has an entry.
- [ ] Given a key change that is refused or fails, when the run reports,
      then the item is reported as awaiting a human and the engine leaves it
      alone.
- [ ] Given a run the engine refuses after identity changes landed, then
      the landed changes and the `identity-applied-before-refusal` note are
      printed as today.
- [ ] Given a resumed retirement for an item outside a targeted run, then
      it is reported as today.

## Open Questions

- Should this split into: the subject key; the identity stage's types; and
  the merge into the sync entry?
- Do `IdentityRow` and `PromotionRow` become one family, given each has its
  own exit-code rule?
- The pass spends budget before the engine plans, so the bounds are checked
  twice. Is that kept as two stages, each with its own check?

## Dependencies

- Blocked by: 0384.
- Blocks: none.

## Assumptions

- The engine cannot plan until identity changes have landed, because they
  rename and rewrite the items it plans over.

## Technical Notes

- `SettledView` already carries some of the pass's knowledge into the
  engine (keys followed and keys promoted this run).
- `IdentityPlan` is already an honest plan: decided in full, bounded, then
  applied in order.

## Drafting Notes

- The least understood item in the epic. It was traced by reading
  `settled_run.rs`, `identity_settlement.rs` and the renderer, not by
  running anything, and is expected to change once the ADR is written.

## References

- Source: PR #188 and its exploration record
  (`.accelerator/tmp/explore-pr-188-context.md`)
- Related: 0352, 0146
- PR reviews: `meta/reviews/prs/188-review-1.md`,
  `meta/reviews/prs/188-review-2.md`
