---
type: "work-item"
id: "0387"
title: "Decide Whether an Untouched Unsynced Item's Watermark Advances"
date: "2026-11-16T11:25:19+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "low"
parent: "work-item:0372"
relates_to: ["work-item:0352", "work-item:0374"]
tags: ["sync", "baseline", "work-adapters"]
last_updated: "2026-11-16T11:25:19+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-978"
---
# 0387: Decide Whether an Untouched Unsynced Item's Watermark Advances

**Kind**: Task
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

A run advances the watermark of an unsynced item it did nothing for: a
draft, or any unsynced item under `--pull-only`, that still holds a recorded
baseline entry. 0352 stopped a failed create doing this on the grounds that
the item was not synced. Decide whether the same reasoning applies here, and
pin the answer with a test.

## Context

Raised as an observation on PR #188's chapter "A failed create stays
unsettled" and marked to be fixed. This epic owns it instead of the PR. The
behaviour predates 0352.

`definitively_reconciled` defers a `Noop` only for the two modified states.
An `(Unsynced, Noop, NotApplied)` row passes, so `finalise_baseline` moves
the item's watermark to the run start.

The effect is narrow. Advancing does nothing for an item with no entry or
with an unknown-recipe entry. Classification answers `Unsynced` before it
reads the baseline, so the watermark is consulted only once the item gains
an `external_id` while keeping the old entry. A local edit made before the
advancing run could then be skipped by the mtime pre-filter.

No test pins either answer.

## Requirements

- Decide whether an unsynced item the run leaves untouched is settled.
- If it is not, `definitively_reconciled` rejects its row, and its watermark
  stays where it was.
- A test pins the decided behaviour for a draft and for an item under
  `--pull-only`, each holding a recorded entry.
- The doc comment on `definitively_reconciled` states the rule.

## Acceptance Criteria

- [ ] The decision and its reason are recorded on this work item.
- [ ] Given an unsynced draft holding an entry with watermark 0, when an
      apply run finishes, then a test asserts the stored watermark is the
      decided value.
- [ ] Given an unsynced item holding such an entry under `--pull-only`,
      when an apply run finishes, then a test asserts the same.
- [ ] No stdout row or exit code changes.

## Open Questions

- Can an unsynced item gain an `external_id` and keep its old entry without
  its file's mtime moving past the advanced watermark? If not, the advance
  is harmless and the decision is only about stating the rule.

## Dependencies

- Blocked by: none.
- Blocks: none.

## Assumptions

- An unsynced item can hold a recorded entry. Which real sequence produces
  one is not known; the tests build it by hand with `seed_recorded_entry`.

## Technical Notes

- 0374 changes how `finalise_baseline` groups rows by id; the two touch the
  same function and are easier done together.

## Drafting Notes

- Kept as a decision with a test, not a prescribed change, because the
  harm is unproven.

## References

- Source: PR #188's review notes and its exploration record
  (`.accelerator/tmp/explore-pr-188-context.md`)
- Related: 0352, 0372
- PR reviews: `meta/reviews/prs/188-review-1.md`,
  `meta/reviews/prs/188-review-2.md`
