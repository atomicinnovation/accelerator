---
type: "work-item"
id: "0382"
title: "Unclassifiable Items and Refused Orders Become Entries"
date: "2026-11-16T11:17:16+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0372"
blocked_by: ["work-item:0378", "work-item:0380"]
relates_to: ["work-item:0352"]
tags: ["sync", "work-adapters", "stdout-contract", "architecture"]
last_updated: "2026-11-16T11:17:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-973"
---
# 0382: Unclassifiable Items and Refused Orders Become Entries

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a maintainer of the sync engine, I want every local item a run considers
to have exactly one entry, including items it could not classify and items
whose order it refused, so that one row per item holds by construction.

## Context

An item whose classification fails is left out of the plan and held in
`unclassifiable`. An order that reaches no create is held in
`refused_orders`. Both are printed as extra `failed` lines after the rows.

A refused order on a draft, or under `--pull-only`, is the one case left
where an item prints two lines: `noop\tunsynced\t-` and
`failed\t-\torder-refused`. The skill carries a Membership rule to stop the
item being summarised twice.

## Requirements

- An unclassifiable item is an entry of a terminal kind carrying its reason.
- A refused order is the outcome of the entry it names, where the item has
  an entry.
- An order naming an id outside the corpus, or outside a targeted run's
  selection, stays a failure of the run, since no entry exists for it.
- The item prints one row for a refused order. The skill's Membership rule
  for the pairing goes, and the CHANGELOG announces the change to stdout.
- Exit codes are unchanged.

## Acceptance Criteria

- [ ] Given any run, then each local item it reconciles has exactly one
      entry, and a test asserts it over the whole report.
- [ ] Given an order on a draft or under `--pull-only`, when the run
      reports, then stdout has one row for the item, carrying the refusal.
- [ ] Given an order naming no item, or an item outside the selection, when
      the run reports, then its `order-refused` line is as today.
- [ ] Given an unclassifiable item, when the run reports, then its line and
      the exit code are as today.
- [ ] The skill's Membership table has no rule for a `noop\tunsynced` row
      beside a refusal, and the check from 0378 passes.
- [ ] The CHANGELOG entry sits under the heading the ADR's stdout stance
      calls for.

## Open Questions

- What is the folded row's shape: `failed\tunsynced\torder-refused`, or the
  item's planned action with the refusal in the detail column?
- Does folding belong under `Breaking` in the CHANGELOG? The pairing has
  shipped in stable releases, unlike 0352's duplicate row.

## Dependencies

- Blocked by: 0378, 0380.
- Blocks: 0383.

## Assumptions

- The ADR from 0375 decides that a refusal folds into its item's row. If it
  decides otherwise, the stdout change and the skill change drop out and
  only the one-entry rule remains.

## Technical Notes

- With one entry per item, the by-id rule from 0374 can be simplified or
  removed.
- `why_no_create` needs the corpus and the selection, so the reason for a
  refusal is still worked out in the adapter.

## Drafting Notes

- This is the item at which one row per item becomes structural for the
  engine, and the earliest point the epic could pause.

## References

- Source: PR #188 and its exploration record
  (`.accelerator/tmp/explore-pr-188-context.md`)
- Related: 0352, 0146
- PR reviews: `meta/reviews/prs/188-review-1.md`,
  `meta/reviews/prs/188-review-2.md`
