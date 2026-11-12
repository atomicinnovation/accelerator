---
type: "pr-description"
id: "188"
title: "[0352] Report each unsynced item once, by its create"
date: "2026-11-11T09:12:09+00:00"
author: "Phil Helm"
producer: "describe-pr"
status: "complete"
work_item_id: "0352"
parent: "work-item:0352"
relates_to: ["work-item:0351"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/188"
pr_number: 188
tags: ["sync", "report", "work-adapters", "baseline", "sync-work-items"]
revision: "f9887ec718bb95a61801426c9173de864e8ca9a0"
repository: "accelerator"
last_updated: "2026-11-12T10:31:48+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

# [0352] Report each unsynced item once, by its create

## Summary

`accelerator work sync` reported every unsynced item it planned a create for
twice: once by its create (`create-from-local`, `adopt` or `failed`) and once
as a leftover `noop\tunsynced` row. A failed create sat beside a row that read
as if nothing was wrong, and because the leftover row counted as settled, a
failed create also advanced the item's baseline watermark. Each such item now
reports once, by its create, and a failed create leaves its watermark where it
was. Unsynced drafts and items under `--pull-only` keep their single `noop`
row.

The planner gives every unsynced item a `Noop` entry. Creates used to be
copied from those entries, leaving the `Noop` behind; now they are moved out,
as relinks already were:

```diff
 prepare_run
   plan = knowledge.plan(…)            # every unsynced item planned as Noop
   relinks(&mut plan)                  # moves relink entries out
+  unsynced_creates(&mut plan)         # moves create entries out
   …
-  unsynced_creates(&plan)             # copied create entries, left the Noop
 report
   for planned in plan.actions         # one row per entry left in the plan
   for create in creates_from_local    # one row per create
```

## Evidence

The push-only preview in `cli/work-cli/tests/cli_sync.rs` compares the whole
of stdout.

- **Before:**
  ```text
  0001	create-from-local	unsynced	-
  0001	noop	unsynced	-
  #	discovery	skipped	push-only
  ```
  **After:**
  ```text
  0001	create-from-local	unsynced	-
  #	discovery	skipped	push-only
  ```

The engine tests in `cli/work-adapters/tests/sync_create.rs` check every row
for an id, not the first:

- **Before:** nine tests fail against `main`.
  ```text
  a_failed_create_is_the_items_only_row
    only_row(report, "0001")
    ✗ 0001 reported as [Noop, CreateFromLocal]
  a_failed_create_leaves_the_items_watermark_where_it_was
    seed entry with local_synced_at 0, run against an unreachable tracker
    ✗ stored watermark Some(1700000000), expected Some(0)
  ```
  **After:** `sync_create` passes 121/121, and the regression guards for
  drafts and `--pull-only` pass both before and after.

## Merge Danger

**Door:** two-way

No stored state changes shape: the baseline file, pending-push markers and
dossiers are read and written as before. Reverting restores the old output.

**Blast Radius:** narrow

- `work sync` stdout drops one row per item it creates or adopts. A script
  that listed unsynced items by their `noop` row stops seeing those items.
  The duplicate only shipped in 1.24.0 pre-releases.
- `/sync-work-items` reads that output. It ships with the binary, and its
  Membership table changes here to match.
- A failed create no longer advances its baseline watermark. That only shows
  for an item still carrying an old baseline entry.

## Changes

- **`cli/work-adapters/src/sync/run.rs`**: `unsynced_creates` now takes the
  entries it turns into creates out of `plan.actions`, as `relinks` already
  did, instead of cloning them and leaving the originals behind. It returns
  before partitioning under `--pull-only`, so those runs keep their `noop`
  rows. The eligibility rule moves into `creatable_item`: an unsynced,
  non-draft entry the plan leaves as `Noop`, with a matching local item. The
  call sits directly after `relinks` in `prepare_run`, so the two steps that
  take entries out of the plan are side by side.
- **`cli/work-adapters/tests/sync_create.rs`**: `only_row` asserts an id has
  exactly one report row, rather than finding the first. Eight behaviour tests
  cover preview and apply, plain create, create order and adopt order, a
  tracker failure, an adopt refused at apply time, the failed-create
  watermark, and an unknown-recipe baseline entry in preview and apply.
  Regression guards cover a successful create's watermark, an unsynced draft,
  `--pull-only`, orders refused on both, and the unknown-recipe warning that
  drafts and `--pull-only` items keep. Two unit tests in `run.rs` pin that an
  unsynced `Noop` is created from local and an unsynced relink is not.
- **`cli/work-cli/tests/cli_sync.rs`**: the push-only preview's expected
  stdout drops the `noop` line.
- **`skills/work/sync-work-items/SKILL.md`**: the interim Membership rule for
  a create's duplicate `noop` row goes. A `noop\tunsynced` row beside a
  `failed` row, `order-refused` (an order on a draft, or any order under
  `--pull-only`) is now the only pairing left, and it gets the rule instead,
  so that item is still summarised once.
- **`CHANGELOG.md`**: a `Fixed` entry.

## Context

- Work item: `meta/work/0352-sync-reports-every-unsynced-item-twice.md`
- Plan: `meta/plans/2026-11-11-0352-sync-reports-every-unsynced-item-twice.md`
- Research:
  `meta/research/codebase/2026-11-10-0352-sync-reports-every-unsynced-item-twice.md`
- Reviews: `meta/reviews/prs/188-review-1.md`,
  `meta/reviews/prs/188-review-2.md`
- Related: 0351, whose preview tests can assert one row per item once this
  lands.

## Testing

- [x] Every new behaviour test, the extended
  `adopting_a_missing_issue_is_refused` and the CLI preview test were seen
  failing before the partition. The regression guards passed before and
  after.
- [x] The guards added after review were each seen failing against the
  breakage they protect from: the relink unit test with the `Action::Noop`
  clause removed, and the preview unknown-recipe test with create rows routed
  through the unknown-recipe relabelling.
- [x] `sync_create` passes 121/121 and `cli_sync` 19/19.
- [x] The whole `cli/` workspace passes locally, and workspace rustfmt and
  clippy (`-D warnings`) are clean.
- [x] The `cli:check` Python guards and every `build-system:check` task
  pass, including the skill structure tests (74 passed, the new one seen
  failing first).
- [x] All 15 required CI checks pass on `f9887ec71`.
- [ ] The bare `mise run` was not run locally: the mise-pinned Python 3.14.4
  is SIGKILLed on launch on this machine, so every invoke-backed `mise` task
  fails before it starts. The checks above were run by hand, through a scratch
  venv on Homebrew Python 3.14.7, and CI covers the rest.
- [ ] Manual checks against the `TES` sandbox (preview, an `Attempted`
  pending-push marker, `--pull-only`) are still to do.

## Notes for Reviewers

- **Behaviour change for scripts**: an item sync creates or adopts no longer
  has a `noop\tunsynced` row. A script that listed unsynced items by that row
  should read the item's `create-from-local`, `adopt` or `failed` row
  instead. The CHANGELOG says so.
- **Dropped warning**: an unsynced item carrying a baseline entry under an
  unknown digest recipe no longer gets the "left untouched; upgrade the
  plugin" warning or an `Unobserved` dossier status when the run creates it.
  Neither was true: the create replaces the entry. Drafts and `--pull-only`
  items keep the warning.
- **Order-independent guard**: the `Action::Noop` clause in `creatable_item`
  cannot be reached through `run_sync`, since the planner gives every
  unsynced item `Noop` and `relinks` has already taken the `Relink` entries
  out. It keeps the predicate stating the rule rather than relying on call
  order, and a unit test calls it directly.
- **Not changed**: the decision table, the dossier mapping, and the
  `order-refused` line an order on a draft or under `--pull-only` prints
  beside its `noop` row.
