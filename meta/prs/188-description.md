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
revision: "c4539f049c479a819aa013d2d94b795668b25dbd"
repository: "accelerator"
last_updated: "2026-11-11T09:12:09+00:00"
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
  watermark, and an unknown-recipe baseline entry. Regression guards cover a
  successful create's watermark, an unsynced draft, `--pull-only`, and orders
  refused on both.
- **`cli/work-cli/tests/cli_sync.rs`**: the push-only preview's expected
  stdout drops the `noop` line.
- **`skills/work/sync-work-items/SKILL.md`**: the interim Membership rule for
  a create's duplicate `noop` row goes. A `noop\tunsynced` row beside an
  `order-refused` line (an order on a draft, or any order under
  `--pull-only`) is now the only pairing left, and it gets the rule instead,
  so that item is still summarised once.
- **`CHANGELOG.md`**: a `Fixed` entry.

## Context

- Work item: `meta/work/0352-sync-reports-every-unsynced-item-twice.md`
- Plan: `meta/plans/2026-11-11-0352-sync-reports-every-unsynced-item-twice.md`
- Research:
  `meta/research/codebase/2026-11-10-0352-sync-reports-every-unsynced-item-twice.md`
- Related: 0351, whose preview tests can assert one row per item once this
  lands.

## Testing

- [x] Every new behaviour test, the extended
  `adopting_a_missing_issue_is_refused` and the CLI preview test were seen
  failing before the partition (`0001 reported as [Noop, CreateFromLocal]`,
  or the watermark at `Some(1700000000)` instead of `Some(0)`). The
  regression guards passed before and after.
- [x] `sync_create` passes 118/118 and `cli_sync` 19/19.
- [x] The whole `cli/` workspace passes (7382 passed, 2 skipped), and
  workspace rustfmt and clippy (`-D warnings`) are clean.
- [x] The `cli:check` Python guards and every `build-system:check` task
  pass, including the skill structure tests (74 passed, the new one seen
  failing first).
- [ ] The bare `mise run` was not run locally: the mise-pinned Python 3.14.4
  is SIGKILLed on launch on this machine, so every invoke-backed `mise` task
  fails before it starts. The checks above were run by hand, through a scratch
  venv on Homebrew Python 3.14.7, and CI covers the rest.
- [ ] Manual checks against the `TES` sandbox (preview, an `Attempted`
  pending-push marker, `--pull-only`) are still to do.

## Notes for Reviewers

- **Behaviour change for scripts**: an item sync creates or adopts no longer
  has a `noop\tunsynced` row. A script that listed unsynced items by that row
  should key on the `unsynced` state column, which every create row also
  carries. The CHANGELOG says so.
- **Dropped warning**: an unsynced item carrying a baseline entry under an
  unknown digest recipe no longer gets the "left untouched; upgrade the
  plugin" warning or an `Unobserved` dossier status when the run creates it.
  Neither was true: the create runs regardless and replaces the entry. Drafts
  and `--pull-only` items keep the warning.
- **Unreachable guard**: the `Action::Noop` clause in `creatable_item` cannot
  be reached by a test, since the planner gives every unsynced item `Noop` and
  `relinks` has already taken the `Relink` entries out. It keeps the
  predicate stating the rule rather than relying on call order.
- **Not changed**: the decision table, the dossier mapping, and the
  `order-refused` line an order on a draft or under `--pull-only` prints
  beside its `noop` row.
