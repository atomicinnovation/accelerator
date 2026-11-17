---
type: "work-item"
id: "0377"
title: "Pin the Sync Stdout Row Contract Through the Binary"
date: "2026-11-16T11:17:16+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0372"
relates_to: ["work-item:0352", "work-item:0351"]
tags: ["sync", "work-cli", "tests", "stdout-contract"]
last_updated: "2026-11-16T11:17:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0377: Pin the Sync Stdout Row Contract Through the Binary

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add whole-stdout tests for the `work sync` outputs that the restructuring in
this epic could disturb. Row order and row shapes are read by scripts and by
`/sync-work-items`, and only one case is pinned through the binary today.

## Context

0352 was pinned at the CLI by one test: a push-only preview of one unsynced
item, compared as the whole of stdout. A contains-check had passed with the
duplicate row present. The other outputs are guarded only at the engine
boundary, where row order across sources and the renderer's own merging are
invisible.

Stdout is assembled from several sources in a fixed order: identity rows,
promotion rows, then engine rows (imports, plan rows, creates, relinks),
then the failures for unclassifiable items and refused orders.
`render_report` also drops an engine row whose id matches an identity row's
settled id.

## Requirements

Whole-stdout equality tests in `cli/work-cli/tests/cli_sync.rs` for:

- An apply run that creates one item and leaves another untouched.
- An apply run whose create fails.
- A preview and an apply with an adopt order.
- A refused order beside its item's `noop` row, on a draft and under
  `--pull-only`.
- A run with an import, a plan row and a create together, fixing their
  order.
- A run with an identity row beside the engine row for the same settled id.
- An unclassifiable item.
- An unsynced item with an unknown-recipe entry under `--pull-only`.

## Acceptance Criteria

- [ ] Each case above has a test that compares the whole of stdout and the
      exit code.
- [ ] Each test fails when a row is added, removed or reordered.
- [ ] No production code changes.

## Open Questions

- Should stderr be compared whole as well, for the cases that print a
  warning or an `error:` line?

## Dependencies

- Blocked by: 0352 (PR #188) merging.
- Blocks: 0378, 0379.

## Assumptions

- These are characterisation tests: they record current behaviour,
  including behaviour a later item changes on purpose.

## Technical Notes

- 0351 changes what a preview prints for a create whose pending-push marker
  would be refused. Whichever lands second updates the other's expectations.

## Drafting Notes

- Placed before the structural stories so that "stdout unchanged" in their
  criteria is checkable.

## References

- Source: PR #188 and its exploration record
  (`.accelerator/tmp/explore-pr-188-context.md`)
- Related: 0352, 0146
- PR reviews: `meta/reviews/prs/188-review-1.md`,
  `meta/reviews/prs/188-review-2.md`
