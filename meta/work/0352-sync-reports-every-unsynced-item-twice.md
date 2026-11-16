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
external_id: "PP-927"
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
beside a row that reads as if nothing was wrong, and `/sync-work-items` needs
an interim rule to ignore the `noop` row. The leftover `noop` entry also
advances a failed create's baseline watermark.

## Context

Found on 2026-10-24 during 0300's manual checks against the `TES` sandbox
team, on preview and apply runs, targeted and whole-corpus, and on the
released `1.24.0-pre.86` binary as well as the branch build:

```text
0004	failed	unsynced	terminal
0004	noop	unsynced	-
```

The decision table maps `Unsynced` to `Noop` in every direction
(`cli/work/src/sync/decide.rs:18-23`), so `plan()` adds a noop entry for the
item. `unsynced_creates` (`cli/work-adapters/src/sync/run.rs:1346-1368`) then
clones every unsynced entry into a `LocalCreate` and leaves the original in
`plan.actions`. `relinks` (`run.rs:1288-1310`) takes its entries out of the
plan because they are applied beside the creates; creates are not taken out.
Preview reports every `plan.actions` entry and then every create
(`run.rs:2360-2401`); apply does the same through `apply_planned_action` and
`apply_local_create`.

The leftover noop entry also passes `definitively_reconciled`
(`run.rs:368-384`), so the item's id joins `advance_ids` even when its create
failed.

`a_push_only_preview_plans_an_unsynced_item_without_reaching_the_tracker`
(`cli/work-cli/tests/cli_sync.rs:326-363`) asserts the two rows.

Terms used below: a *pending-push order* is an `adopt:<key>` or `create`
instruction recorded against an unsynced item, telling its create to link an
existing issue or make a new one. The *baseline watermark* (`local_synced_at`)
is the time up to which the next run treats a file not modified since as
unchanged, without comparing fingerprints.

## Requirements

Reproduction:

1. Create an unsynced, non-draft local work item (no `external_id`) in a
   repository with a tracker configured.
2. Run `accelerator work sync --target <id> --preview`.

Actual: it prints a `create-from-local` row and a `noop\tunsynced\t-` row for
the item.

Failed-create form: run `accelerator work sync --target <id>` against a
tracker that rejects the create. Actual: a `failed\tunsynced\tterminal` row
beside a `noop\tunsynced\t-` row.

Expected: one row per item the run plans a create for, carrying that create's
outcome, whether it succeeds or fails; an unsynced item the run plans no
create for keeps its single `noop` row.

Changes:

- `unsynced_creates` takes the entries it creates out of `plan.actions`, as
  `relinks` does, so each unsynced item is reported once, by its create.
- An unsynced item the run plans no create for (a draft, or any item under
  `--pull-only`) keeps its single `noop` row.
- For an unsynced item the run plans a create for, the baseline watermark
  advances only when that create succeeds and writes its baseline entry; a
  failed create leaves it where it was. For an unsynced item the run plans no
  create for, the watermark behaves as it does today.
- The push-only preview test asserts the single row.
- The `/sync-work-items` interim membership rule for the duplicate `noop` row
  and its structure test are removed.

## Acceptance Criteria

- [ ] Given an unsynced item, when a preview or apply run plans its create,
      then its only row is its create's row: `create-from-local` (or `adopt`
      for an adopt order) in a preview, that create's outcome in an apply, and
      no `noop` row.
- [ ] Given an unsynced item whose create fails, when the run reports, then its
      only row is the `failed` row.
- [ ] Given an unsynced draft, or an unsynced item under `--pull-only`,
      neither carrying a pending-push order, when the run reports, then its
      only row is `noop`.
- [ ] Given an unsynced, non-draft item carrying an adopt or create order, when
      a run other than `--pull-only` reports, then the item's only row is its
      `adopt` or `create-from-local` row, or that create's `failed` row.
- [ ] Given an unsynced item with an existing baseline entry whose create
      fails, when the run finalises its baseline, then that entry's
      `local_synced_at` watermark is unchanged.
- [ ] Given an unsynced item whose create succeeds, when the run finalises its
      baseline, then the item has a baseline entry whose `local_synced_at` is
      the run's watermark.
- [ ] `/sync-work-items` no longer carries the duplicate-`noop` membership
      rule, `test_a_create_s_duplicate_noop_row_joins_no_group` is removed,
      and the push-only preview test asserts the single row.

Each row-count criterion is checked against every report row for the id, not
the first one only.

## Open Questions

- Resolved: adding a failed create's id to `advance_ids` is harmless in the
  usual case and visible in one narrow case; see Technical Notes.

## Dependencies

- Blocked by: none. 0300, which introduced the code this changes, is done.
- Blocks: none.
- Related: 0351 — its preview tests can assert one row per item once this
  lands; whichever lands second updates the other's row assertions.

## Assumptions

- No script parses the `noop\tunsynced` row. `/sync-work-items` treats it as
  belonging to no group when the same id also has a `failed`,
  `create-from-local` or `adopt` row. Neither the user guide (`docs-site/`)
  nor `CHANGELOG.md` documents the duplicate row.

## Technical Notes

**Size**: S — one partition in `unsynced_creates` (`cli/work-adapters/src/sync/run.rs`), one expected line dropped from `cli_sync.rs`, one skill-table row and its Python test removed, and four or five regression tests; nothing downstream reads the removed entries.

- Fix shape: `unsynced_creates` (`cli/work-adapters/src/sync/run.rs:1346`)
  takes `&mut SyncPlan` and partitions its entries out, as `relinks`
  (`run.rs:1288`) does. An entry leaves the plan only when all four hold:
  its state is `Unsynced`; its id is not a draft; a matching local item
  exists; the run is not `--pull-only`. `relinks` partitions under
  `--pull-only` too; `unsynced_creates` must not, or that run loses its
  `noop` rows.
- Nothing after planning reads the leftover entries. `refused_orders`
  (`run.rs:1373`) reads only `plan.unclassifiable` and the creates;
  `validate_pushes` (`run.rs:1000`) never validates a `Noop`; the pull and
  push bounds count no `Noop`; dossiers treat `(Unsynced, Noop)` and
  `(Unsynced, CreateFromLocal)` alike; the create row is also stated
  `Unsynced`, so the printed state does not change. Because the entry leaves
  the plan after planning, the decide golden
  (`cli/work/tests/fixtures/work-item-sync-decide.golden:32-34`) and the
  `(Unsynced, Noop)` arm in `cli/work-adapters/src/sync/dossiers.rs:90` are
  untouched.
- Watermark: `finalise_baseline` (`run.rs:334`) advances every id with a row
  passing `definitively_reconciled` (`run.rs:368`). The leftover
  `(Unsynced, Noop, NotApplied)` row passes, so a failed create's id advances
  today. A successful create advances through its own `Applied` row, and
  `record` (`cli/work-adapters/src/sync/apply.rs:1672`) has already written
  its baseline entry.
- Open question answered: the advance is harmless in the usual case —
  `advance_watermark` (`cli/work-adapters/src/sync/baseline.rs:483`) does
  nothing for an id with no baseline entry, and a failed create writes none.
  It is visible only when an unsynced item still carries an old entry (its
  `external_id` removed by hand) and its create links the file then fails at
  `ReadLocal` or `record` (`apply.rs:1602-1618`): with the link write in the
  same second as the run start, the advanced watermark hides that edit from
  the next run's local-change check.
- Pending-push orders ride on the unsynced `Noop` entry
  (`PlannedAction::ordered`, `cli/work/src/sync/plan.rs:90`), so the same
  partition gives adopt and create orders one row. An order that reaches no
  create (on a draft, or any order under `--pull-only`) keeps its `noop` row
  and also gets an `<id>\tfailed\t-\torder-refused` line from
  `refused_orders`; that predates this bug and is out of scope.
- Tests: only
  `a_push_only_preview_plans_an_unsynced_item_without_reaching_the_tracker`
  (`cli/work-cli/tests/cli_sync.rs:327`) asserts the duplicate row. `row()`
  and `outcome_of` in `cli/work-adapters/tests/support/repo.rs` return an
  id's first row — the `noop` row for an unsynced item today — so no current
  caller breaks, but neither can prove an id has exactly one row; the new
  tests need every row for the id. A rejected create
  (`a_rejected_create_leaves_no_marker_and_reports_its_cause`,
  `cli/work-adapters/tests/sync_create.rs:2350`) is the existing seam for the
  failed-create cases. The interim rule is at
  `skills/work/sync-work-items/SKILL.md:466`, its test at
  `tests/unit/tasks/test_sync_work_items_skill_structure.py:450`.

## References

- Found during: `meta/plans/2026-10-11-0300-sync-pulls-of-tracked-work-items-fail-silently.md`
  (Phase 7 manual verification)
