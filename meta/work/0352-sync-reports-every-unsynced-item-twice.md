---
type: "work-item"
id: "0352"
title: "Sync Reports Every Unsynced Item Twice"
date: "2026-10-24T11:40:00+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "medium"
parent: "work-item:0146"
relates_to: ["work-item:0300", "work-item:0296"]
tags: ["sync", "report", "work-adapters"]
last_updated: "2026-10-24T11:40:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0352: Sync Reports Every Unsynced Item Twice

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

`accelerator work sync` reports each unsynced local item twice: once as its
create (`create-from-local`, `adopt`, or `failed … terminal` when the create
fails) and once as `<id>\tnoop\tunsynced\t-`. A failed create therefore sits
beside a row that reads as if nothing was wrong, and `/sync-work-items` has no
rule for the `noop` row.

## Context

Found on 2026-10-24 during 0300's manual checks against the `TES` sandbox
team, on preview and apply runs, targeted and whole-corpus, and on the
released `1.24.0-pre.86` binary as well as the branch build:

```text
0004	failed	unsynced	terminal
0004	noop	unsynced	-
```

The decision table maps `Unsynced` to `Noop` in every direction
(`cli/work/src/sync/decide.rs:15-23`), so `plan()` adds a noop entry for the
item. `unsynced_creates` (`cli/work-adapters/src/sync/run.rs:1324-1346`) then
clones every unsynced entry into a `LocalCreate` and leaves the original in
`plan.actions`. `relinks` (`run.rs:1263-1288`) takes its entries out of the
plan because they are applied beside the creates; creates are not taken out.
Preview reports every `plan.actions` entry and then every create
(`run.rs:2326-2342`); apply does the same through `apply_planned_action` and
`apply_local_create`.

The leftover noop entry also passes `definitively_reconciled`
(`run.rs:368-380`), so the item's id joins `advance_ids` even when its create
failed.

`a_push_only_preview_plans_an_unsynced_item_without_reaching_the_tracker`
(`cli/work-cli/tests/cli_sync.rs:326-363`) asserts the two rows.

## Requirements

Reproduction:

1. Create an unsynced local work item (no `external_id`).
2. `accelerator work sync --target <id> --preview` prints
   `create-from-local` and `noop` rows for it.

Expected: one row per item, carrying its create's outcome.

Changes:

- `unsynced_creates` takes the entries it creates out of `plan.actions`, as
  `relinks` does, so each unsynced item is reported once, by its create.
- An unsynced item a run does not create (a draft, or any item under
  `--pull-only`) keeps its single `noop` row.
- The baseline watermark advances for an unsynced item only when its create
  settles.
- The push-only preview test asserts the single row.

## Acceptance Criteria

- [ ] Given an unsynced item, when a preview or apply run plans its create,
      then the report has exactly one row for it.
- [ ] Given an unsynced item whose create fails, when the run reports, then its
      only row is the `failed` row.
- [ ] Given an unsynced item under `--pull-only`, when the run reports, then
      its only row is `noop`.
- [ ] Given an adopt or create order, when the run reports, then the item has
      exactly one row.

## Open Questions

- Does adding a failed create's id to `advance_ids` have a visible effect
  today, for example hiding a later edit from the mtime pre-filter, or is it
  harmless because the item has no baseline entry?

## Dependencies

- Blocked by: none.
- Blocks: none.

## Assumptions

- No script parses the `noop\tunsynced` row; `/sync-work-items` has no entry
  for it.

## Technical Notes

- Dossier building expects an unsynced item to plan as noop
  (`cli/work-adapters/src/sync/dossiers.rs:90`), as does the decide golden
  (`cli/work/tests/fixtures/work-item-sync-decide.golden:32-34`); taking the
  entry out of the plan after planning should leave both untouched.

## References

- Found during: `meta/plans/2026-10-11-0300-sync-pulls-of-tracked-work-items-fail-silently.md`
  (Phase 7 manual verification)
