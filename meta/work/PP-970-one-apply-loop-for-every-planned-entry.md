---
type: "work-item"
id: "PP-970"
title: "One Apply Loop for Every Planned Entry"
date: "2026-11-16T11:17:16+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:PP-963"
blocked_by: ["work-item:PP-966", "work-item:PP-967", "work-item:PP-968"]
relates_to: ["work-item:PP-927", "work-item:PP-926"]
tags: ["sync", "work-adapters", "architecture"]
last_updated: "2026-11-16T11:17:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-970"
aliases: ["0379"]
---
# PP-970: One Apply Loop for Every Planned Entry

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a maintainer of the sync engine, I want every planned entry previewed and
applied by one loop that dispatches to a strategy for its kind, so that
creates and relinks stay in the plan and no step has to take entries out of
it.

## Context

Creates and relinks are applied by `apply_local_create`, which needs the
request, the author port, the carrier lookup and the local item, and returns
an adoption beside its row. Every other action goes through
`apply_planned_action`, which has none of those and marks the create and
relink arms `unreachable!`. That difference in signature is why the adapter
takes those entries out of the plan and applies them from a second list.

Preview has the same split: `validate_pushes` reports the plan, and a
separate loop reports the creates.

## Requirements

- One run context gives every strategy the ports and run facts it needs.
- Each strategy returns one result shape: the row, plus any adoption and
  any request to blank the item's local hash.
- Each kind has a preview strategy and an apply strategy.
- Creates and relinks are no longer taken out of the plan.
- Row order on stdout is preserved by an explicit ordering.
- The rule that a create's row is not relabelled for an unknown recipe is
  stated, not a by-product of which loop reported it.

## Acceptance Criteria

- [ ] `relinks` and `unsynced_creates` no longer remove entries from the
      plan, and `PreparedRun` holds no separate list of creates.
- [ ] No arm of the apply dispatch is `unreachable!`.
- [ ] The whole-stdout tests from 0377 pass unchanged.
- [ ] Given a previewed create over an unknown-recipe entry, when the run
      reports, then its row is not `UnsupportedRecipe`.
- [ ] Each strategy has unit tests that run it against a fake run context.
- [ ] A relink under `--pull-only` behaves as 0376 decided.

## Open Questions

- Is the current apply order (imports, plan entries, creates, relinks)
  required for correctness, or only for the order of rows? No stated reason
  for it was found.

## Dependencies

- Blocked by: 0375, 0376, 0377.
- Blocks: 0380, 0381, 0382, 0383.

## Assumptions

- The dispatch stays an exhaustive match over the existing `Action` enum
  here; per-kind payload types come in 0383.

## Technical Notes

- `ItemApplier` already holds most of the bodies (`create_from_local`,
  `relink`, `create_from_remote`, the send and pull paths).
- The push bound must count a planned create that does not adopt, since
  `push_count` ignores `CreateFromLocal` today.
- 0351's precondition check for a previewed create belongs in the create's
  preview strategy.
- The run context should hold ports and run facts only, not accumulators.

## Drafting Notes

- "Strategy" means one implementation per kind chosen by one match, not
  trait objects stored in the plan.

## References

- Source: PR #188 and its exploration record
  (`.accelerator/tmp/explore-pr-188-context.md`)
- Related: 0352, 0146
- PR reviews: `meta/reviews/prs/188-review-1.md`,
  `meta/reviews/prs/188-review-2.md`
