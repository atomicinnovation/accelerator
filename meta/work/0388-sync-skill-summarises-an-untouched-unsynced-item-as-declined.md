---
type: "work-item"
id: "0388"
title: "Sync Skill Summarises an Untouched Unsynced Item as Declined"
date: "2026-11-16T11:25:19+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "low"
parent: "work-item:0372"
blocks: ["work-item:0378"]
relates_to: ["work-item:0352", "work-item:0373"]
tags: ["sync", "sync-work-items", "skill"]
last_updated: "2026-11-16T11:25:19+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-979"
---
# 0388: Sync Skill Summarises an Untouched Unsynced Item as Declined

**Kind**: Bug
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

A lone `noop\tunsynced\t-` row has no rule in the `/sync-work-items`
Membership table. The model places it by the name of a Step 5 group, most
likely `unsynced (not pushed)`, which is annotated "(declined)" although
nobody declined anything.

## Context

Raised as an observation on PR #188's chapter "What the skill is told" and
marked to be fixed. This epic owns it instead of the PR. It predates 0352.

`work sync` prints `<id>\tnoop\tunsynced\t-` for an unsynced item it leaves
alone: a draft when promotion is off, or any unsynced item under
`--pull-only`. The Membership table calls itself the single source for Step
5's groups, yet four groups (`pushed`, `pulled`, `pushed-unsynced`,
`unsynced (not pushed)`) are fed by no rule.

## Requirements

Reproduction:

1. Run `/sync-work-items` with `--pull-only` over a corpus holding an
   unsynced item, or with `--no-promote` over one holding a draft.
2. The report carries `<id>\tnoop\tunsynced\t-`.
3. No Membership rule matches; the item is summarised under a group chosen
   by name, annotated as declined.

Expected: the item is summarised by a rule, in words that say why it was
not pushed.

Changes:

- The Membership table gains a rule for a lone `noop\tunsynced\t-` row,
  naming its group and that it does not block completion.
- Step 5's wording for that group distinguishes an item the user declined
  from one the run's direction or `--no-promote` left alone.
- The skill structure test pins the rule.

## Acceptance Criteria

- [ ] Given a `--pull-only` report carrying `<id>\tnoop\tunsynced\t-`, when
      the skill summarises, then the item is listed once and is not
      described as declined.
- [ ] Given a report where the user declined to push that item, when the
      skill summarises, then it is described as declined.
- [ ] Given that row beside a refusal for the same id, when the skill
      summarises, then the refused-order rule still applies and the item is
      listed once.
- [ ] `tests/unit/tasks/test_sync_work_items_skill_structure.py` fails when
      the new rule is removed.

## Open Questions

- Should the other three groups with no rule get rules here, or in 0378
  when the table is checked against the engine's row shapes?

## Dependencies

- Blocked by: none. 0373 edits the same table and test; do them in either
  order.
- Blocks: 0378.

## Assumptions

- The skill can tell a declined push from a run that left the item alone,
  from the flags it ran with.

## Technical Notes

- `test_a_refused_order_s_noop_row_joins_no_group` asserts the table holds
  exactly one `noop\tunsynced` rule. A second rule changes that test's
  shape, not only its expectations.

## Drafting Notes

- Filed as a bug, not folded into 0373, because 0373 is about an item that
  wrongly reads as complete and this is about wording for one that is
  correctly not blocking.

## References

- Source: PR #188's review notes and its exploration record
  (`.accelerator/tmp/explore-pr-188-context.md`)
- Related: 0352, 0372
- PR reviews: `meta/reviews/prs/188-review-1.md`,
  `meta/reviews/prs/188-review-2.md`
