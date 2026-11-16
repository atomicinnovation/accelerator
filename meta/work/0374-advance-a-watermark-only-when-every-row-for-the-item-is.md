---
type: "work-item"
id: "0374"
title: "Advance a Watermark Only When Every Row for the Item Is Settled"
date: "2026-11-16T11:17:15+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0372"
relates_to: ["work-item:0352"]
tags: ["sync", "baseline", "work-adapters", "tests"]
last_updated: "2026-11-16T11:17:15+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0374: Advance a Watermark Only When Every Row for the Item Is Settled

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

`finalise_baseline` advances an item's watermark when any one of its report
rows is settled. That is safe only while each item has exactly one row,
which production code does not check. Make the rule hold however many rows
an item has, and make the older test helpers able to see a second row.

## Context

Before 0352 a created item had two rows, and the leftover `noop` row moved
the watermark of an item whose create had failed. 0352 fixed it by removing
the row and left the any-row rule in place. A later change that reports an
item twice would bring the fault back without touching the finaliser.

In `cli/work-adapters/tests/sync_create.rs`, only the tests added by 0352
read a row through `only_row`, which asserts one row per id. The older
helpers (`local_create_row`, `create_from_local_outcome`,
`create_from_local_failure`, `relink_row`) return the first row with a
wanted action and cannot notice a second (`R2-09`).

## Requirements

- `finalise_baseline` advances an id only when none of its rows fails
  `definitively_reconciled`.
- The older row helpers assert that the id has exactly one row.
- No output or stored state changes for a run that reports each item once.

## Acceptance Criteria

- [ ] Given an id with one settled row and one failed row, when the
      baseline is finalised, then its watermark is unchanged.
- [ ] Given an id with a single settled row, when the baseline is
      finalised, then its watermark is the run start, as today.
- [ ] `a_failed_create_leaves_the_items_watermark_where_it_was` still passes
      when `unsynced_creates` is made to clone its entries again, showing
      the finaliser no longer depends on one row per id.
- [ ] Each of `local_create_row`, `create_from_local_outcome`,
      `create_from_local_failure` and `relink_row` fails its test when the
      id has a second row.

## Open Questions

- Is there any item that legitimately has two engine rows today? None was
  found, but the search was by reading, not by a test.

## Dependencies

- Blocked by: none.
- Blocks: none. 0382 makes the rule structural and may simplify this code.

## Assumptions

- Identity rows, promotion rows and refused orders are held outside
  `RunReport::reported`, so they are not rows in this sense.

## Technical Notes

- The third criterion is a mutation check to run while implementing, not a
  permanent test.

## Drafting Notes

- Kept although the epic later makes it redundant: it is small, and the
  structural fix is several items away.

## References

- Source: PR #188 and its exploration record
  (`.accelerator/tmp/explore-pr-188-context.md`)
- Related: 0352, 0146
- PR reviews: `meta/reviews/prs/188-review-1.md`,
  `meta/reviews/prs/188-review-2.md`
