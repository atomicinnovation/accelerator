---
type: "work-item"
id: "0381"
title: "Unknown Recipe Becomes a Fact on the Entry"
date: "2026-11-16T11:17:16+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0372"
blocked_by: ["work-item:0379", "work-item:0373"]
relates_to: ["work-item:0352"]
tags: ["sync", "work-adapters", "unknown-recipe", "architecture"]
last_updated: "2026-11-16T11:17:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-972"
---
# 0381: Unknown Recipe Becomes a Fact on the Entry

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a maintainer of the sync engine, I want "this item's baseline entry uses
an unknown recipe" to be a fact carried by the item's entry, so that what
each kind of entry reports for it is a stated rule.

## Context

A run records which items have an unknown-recipe baseline entry in a set
held beside the plan. `noting_unsupported_recipe` consults it for rows from
the plan loop only. 0352 changed which items draw the upgrade warning purely
by changing which loop reports them: correct for a create that lands, and
unexamined for one that fails.

## Requirements

- The fact is known when the baseline is loaded and travels with the entry.
- Each kind states what it reports for such an item.
- An item the run leaves untouched reports `UnsupportedRecipe`, as today.
- A create that lands replaces the entry and draws no warning, as today.
- A create that fails follows the rule decided in 0373.
- The conflict status of each case follows from the same rule.

## Acceptance Criteria

- [ ] `PreparedRun` holds no `unsupported_recipe` set.
- [ ] Given a draft, or an unsynced item under `--pull-only`, with an
      unknown-recipe entry, when the run reports, then its row is
      `UnsupportedRecipe` and the upgrade warning is printed.
- [ ] Given a create over such an entry that is previewed or lands, when
      the run reports, then no row is `UnsupportedRecipe`.
- [ ] Given a create over such an entry that fails, when the run reports,
      then the row, the warning and the dossier status are as 0373 decided.
- [ ] The whole-stdout tests from 0377 pass unchanged.

## Open Questions

- Should the planner be told the fact, so it can plan such an item as left
  untouched, or should the adapter attach it afterwards? The ADR's entry
  model decides.

## Dependencies

- Blocked by: 0379, and 0373 for the failed-create rule.
- Blocks: 0383.

## Assumptions

- A linked item with an unknown-recipe entry keeps printing
  `noop\tindeterminate\tunsupported-recipe`.

## Technical Notes

- `ConflictStatus::of` already treats an `UnsupportedRecipe` outcome as
  `Unobserved`, which keeps the dossier.

## Drafting Notes

- Small, and separate from 0382 because it changes no output.

## References

- Source: PR #188 and its exploration record
  (`.accelerator/tmp/explore-pr-188-context.md`)
- Related: 0352, 0146
- PR reviews: `meta/reviews/prs/188-review-1.md`,
  `meta/reviews/prs/188-review-2.md`
