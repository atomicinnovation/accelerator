---
type: "pr-description"
id: "107"
title: "[0257] Add a --target flag to sync specific work items"
date: "2026-09-08T16:54:52+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0257"
parent: "work-item:0257"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/107"
pr_number: 107
tags: ["sync", "cli", "work-sync", "targeting"]
revision: "4184d1e0d0eae90031d83e80c9e2d5e62a0ef809"
repository: "accelerator"
last_updated: "2026-09-08T16:54:52+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0257] Add a --target flag to sync specific work items

## Summary

Adds a repeatable `accelerator work sync --target <id|external-id|path>` option
that reconciles only the named work items, reusing every per-item full-sync
behaviour unchanged and suppressing untracked-remote discovery. The engine owns
the target set through a single `ItemSelection` value; the skill only parses,
gates, and renders. With no `--target`, behaviour is identical to before.

The change is set-construction at the CLI boundary, not a change to the sync
state machine. Three couplings that resist a blanket narrowing are handled
explicitly: discovery (its "untracked = remote − local" definition inverts under
a narrowed set), the corpus-wide double-binding guard (must keep reading every
local `external_id`), and the change-detection watermark (moved from one global
timestamp to a per-item value so a narrowed run advances only what it
reconciled).

## Changes

- **`--target` flag and resolution** (`cli/work-cli/src/sync.rs`,
  `cli.rs`) — repeatable; resolves each token to a local item by local id, path,
  or `external_id` index. Local-id-wins on a dual-shape token, with the remote
  match reported as suppressed. Collect-all validation names every offender and
  aborts before any side effect. Validation runs ahead of the tracker-client
  build, so an abort is credential-independent.
- **Per-item change-detection watermark** (`cli/work-adapters/src/sync/
  baseline.rs`, `baseline_store.rs`, `fetch.rs`, `cli/work/src/sync/
  classify.rs`) — each baseline entry gains `local_synced_at`. The mtime
  pre-filter gates against the item's own watermark; a targeted run advances only
  the items it reconciled, and only items that reached a definitive reconciled
  outcome advance (closing a pre-existing hazard where an indeterminate item's
  watermark moved on a failed remote read).
- **Engine `ItemSelection`** (`cli/work-adapters/src/sync/run.rs`) — a
  payload-free `All` plus `Targeted(&[LocalItem])`, with `corpus` as the single
  source of truth. Reconciliation reads narrow to the selection; whole-corpus
  reads (double-binding guard, untracked discovery) keep reading `corpus`.
  `Targeted` gates discovery off and beats `PushOnly`.
- **Work-directory containment** (`cli/work-cli/src/resolve.rs`,
  `exit_codes.rs`, `main.rs`) — a path target resolving outside the work
  directory is rejected with a distinct `OutsideWorkDir` outcome and exit code 6.
- **Exit-code taxonomy** — new `RESOLVE_OUTSIDE_WORKDIR = 6`; target failures map
  onto the resolve band (no-match 3, out-of-dir 6, malformed/ambiguous 2), with
  `2 > 6 > 3` precedence when classes coexist. `sync --help` now names 3 and 6.
- **Skill surface** (`skills/work/sync-work-items/SKILL.md`, plus Exit-6 branches
  in `create-`/`update-`/`review-work-item`) — documents `--target`, the
  `skipped\ttargeted` discovery line, the suppressed-remote note, and the abort
  codes.

## Context

- Work item: `meta/work/0257-sync-specific-work-items.md`
- Plan: `meta/plans/2026-09-06-0257-sync-specific-work-items.md`
- Research: `meta/research/codebase/2026-09-06-0257-sync-specific-work-items.md`
- Validation: `meta/validations/2026-09-06-0257-sync-specific-work-items-validation.md`

## Testing

- [x] Full read-only check passes: `mise run check`
- [x] Workspace tests pass: `cargo test -p accelerator-work -p work-adapters -p work`
- [x] Targeting, discovery suppression, and per-item result parity with a full
      sync proven by fake-tracker lib tests (`sync_run.rs`, `sync_create.rs`)
- [x] Credential-independent aborts (exit 3/6/2) proven by subprocess tests
      (`cli_sync_targets.rs`)
- [x] Watermark regression (full → edit non-targeted → targeted → full still
      detects the edit) and read-failure watermark covered with deterministic
      epochs/mtime
- [ ] Manual run of `--target` against the configured Linear tracker (requires
      live credentials; see the validation report's manual checklist)

## Notes for Reviewers

- **Migration is read-time, no `migrate` step.** The baseline gains a per-item
  `local_synced_at`; an old baseline backfills each entry's watermark from the
  document-level `timestamp` on read, so gating is unchanged, and a new baseline
  read by an old binary degrades safely. The global timestamp is now vestigial
  for change detection — its only live role is that backfill.
- **Targeting a remote id requires a local counterpart.** A `--target` resolves
  only to an already-tracked local item; importing a brand-new remote-only issue
  remains the job of a full (untargeted) sync, whose discovery step targeting
  suppresses. This is deliberate, not a gap.
- **Focus areas:** the `resolve_targets` precedence/cascade table and the
  watermark advance gate (`definitively_reconciled`) carry the subtle logic.
