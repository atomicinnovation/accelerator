---
type: "work-item"
id: "0376"
title: "Planner Plans Local Creates"
date: "2026-11-16T11:17:15+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0372"
relates_to: ["work-item:0352"]
tags: ["sync", "planner", "work", "work-adapters"]
last_updated: "2026-11-16T11:17:15+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0376: Planner Plans Local Creates

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a maintainer of the sync engine, I want the planner to plan a
create-from-local for each unsynced item the run will create, so that the
plan states the real work and the rule for what is created lives in one
unit-tested place.

## Context

The planner gives every unsynced item `Noop`. The adapter then decides which
of those to create, in `creatable_item`, with the `--pull-only` part of the
rule held by an early return above it (`R2-07`). Moving that return below
the take would silently drop every unsynced item's row under `--pull-only`;
only three integration tests would notice.

`plan()` already has what the rule needs: the direction, the pending-push
orders and each item's id. It already overrides the decision table's answer
for an ordered unsynced item and for a relink.

## Requirements

- `plan()` gives an unsynced item `CreateFromLocal` when it is not a draft
  and the direction is not `PullOnly`, carrying any pending-push order.
- A draft, and any unsynced item under `--pull-only`, stays `Noop`.
- The adapter selects creates by action, as it selects relinks. It holds no
  rule of its own for which unsynced items are created.
- Stdout, exit codes and stored state are unchanged.
- Decide what entry an item with an unconfirmed link and a pending-push
  order gets under `--pull-only`.

## Acceptance Criteria

- [ ] Given an unsynced, non-draft item under a bidirectional or push-only
      direction, when planned, then its action is `CreateFromLocal`.
- [ ] Given that item with an adopt or create order, when planned, then the
      entry carries the order.
- [ ] Given an unsynced draft, or any unsynced item under `--pull-only`,
      when planned, then its action is `Noop`.
- [ ] `creatable_item` and the `--pull-only` early return in
      `unsynced_creates` no longer exist.
- [ ] Given an adopt order, when the push bound is computed, then the adopt
      does not count as a new remote issue, as today.
- [ ] Every existing `sync_create` and `cli_sync` test passes with its
      expected rows unchanged.
- [ ] Given an item with an unconfirmed link and an order under
      `--pull-only`, when the run reports, then the item has the row decided
      below and its order is refused.

## Open Questions

- Under `--pull-only`, `plan()` replaces a relink-ordered item's ordinary
  action with `Relink`, and `relinks` then takes the entry out and drops it.
  The item appears to get no row and no pull in that run, only the
  `order-refused` line. Is that intended? This was read from the code; no
  test for it was found and it was not run.

## Dependencies

- Blocked by: 0352 (PR #188) merging.
- Blocks: 0379.

## Assumptions

- The decision table and its golden file stay as they are: `decide` sees
  only the classification, not the id, so it cannot know a draft.

## Technical Notes

- The rule goes in `plan()` beside the `ordered` and `relink` arms.
- The `Action` doc comment says the create actions are never planned; it
  changes with this.
- The dossier mapping already maps `(Unsynced, CreateFromLocal)` to
  `NotInConflict`, because create rows carry that shape today.
- The lookup of the matching local item stays in the adapter.

## Drafting Notes

- This makes PR #188's `R2-07` and the review page's three observations on
  the create rule unnecessary, which is why they were not asked for in that
  PR.

## References

- Source: PR #188 and its exploration record
  (`.accelerator/tmp/explore-pr-188-context.md`)
- Related: 0352, 0146
- PR reviews: `meta/reviews/prs/188-review-1.md`,
  `meta/reviews/prs/188-review-2.md`
