---
type: "work-item"
id: "0373"
title: "Sync Skill Reports Complete for an Item That Needs an Upgrade"
date: "2026-11-16T11:17:15+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "medium"
parent: "work-item:0372"
relates_to: ["work-item:0352"]
tags: ["sync", "sync-work-items", "skill", "unknown-recipe"]
last_updated: "2026-11-16T11:17:15+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0373: Sync Skill Reports Complete for an Item That Needs an Upgrade

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

`/sync-work-items` can say "sync complete" for an unsynced item whose
baseline entry a newer plugin release wrote. The row `work sync` prints for
it matches no rule in the skill, and the run exits `0`. Separately, a create
that fails over such an entry has no stated or tested behaviour.

## Context

Found by PR #188's second review (`R2-03`, `R2-05`, `R2-06`) and confirmed
by reading the code at its head. None of it is a regression from 0352.

An item the run leaves untouched because its baseline entry uses an unknown
digest recipe is reported with outcome `UnsupportedRecipe`, printed as
detail `unsupported-recipe` under the item's own state. The skill's dispatch
entry and its Membership rule both key on
`noop\tindeterminate\tunsupported-recipe`. An item with no `external_id`
prints `noop\tunsynced\tunsupported-recipe`, which matches neither.

That row is not a failure and does not await a human, so
`exit_code_for_report` returns `0`. The stderr `warning:` naming the item is
the only signal, and the Completion rule does not hold completion back for
a warning.

The refused-order rule (`noop\tunsynced` beside a `failed` row,
`order-refused`) never tests column 4, so it also silences this row.

## Requirements

Reproduction:

1. Give an unsynced item a baseline entry under a digest recipe newer than
   the binary knows.
2. Run `work sync --pull-only`, or run with `--no-promote` when the item is
   a draft.
3. Stdout carries `<id>\tnoop\tunsynced\tunsupported-recipe`, stderr carries
   the upgrade warning, and the exit code is `0`.
4. `/sync-work-items` has no rule for the row and can report the sync as
   complete.

Expected: the item is listed under `needs-upgrade` and completion is held
back.

Changes:

- The skill's dispatch entry and Membership rule for `needs-upgrade` key on
  the detail `unsupported-recipe` in any state.
- The refused-order rule applies only to a `noop\tunsynced` row whose detail
  is `-`.
- The skill structure test pins both.
- The engine states, and a test pins, what a create that fails over an
  unknown-recipe entry reports: whether the upgrade warning is printed and
  whether the item's conflict dossier may be retired.

## Acceptance Criteria

- [ ] Given a report carrying `<id>\tnoop\tunsynced\tunsupported-recipe`,
      when the skill summarises, then the item is under `needs-upgrade` and
      the skill does not say "sync complete".
- [ ] Given that row beside `<id>\tfailed\t-\torder-refused`, when the skill
      summarises, then the item is under both `needs-upgrade` and
      `needs-attention`.
- [ ] Given `<id>\tnoop\tunsynced\t-` beside the refusal, when the skill
      summarises, then the item is listed once, under `needs-attention`.
- [ ] `tests/unit/tasks/test_sync_work_items_skill_structure.py` fails when
      either rule reverts to its current wording.
- [ ] Given an unsynced item with an unknown-recipe entry whose create
      fails, when the run reports, then a test asserts the row, the
      presence or absence of the upgrade warning, and whether the item's
      dossier may be retired, as decided below.

## Open Questions

- What should a create that fails over an unknown-recipe entry report?
  Today it prints no upgrade warning and its dossier may be retired. The
  warning's remedy (upgrade) is wrong for this case, since a retry creates
  the item and replaces the entry; that argues for keeping the warning off
  and writing the rule down.
- Is retiring that item's dossier acceptable? The same dossier is retired
  for any unsynced item with no such entry.

## Dependencies

- Blocked by: none.
- Blocks: 0378, and 0381 takes the engine decision as its rule.

## Assumptions

- A real run reaches the row for a draft. Drafts are promoted, and so kept
  from the engine, only when ids are tracker-owned, the direction is not
  `--pull-only` and `--no-promote` is absent. This was read from
  `drafts_to_promote` and `plannable`, not observed in a run.

## Technical Notes

- `ConflictShape::of_unresolved` maps `(Unsynced, CreateFromLocal)` to
  `NotInConflict` whatever the outcome, which is why `settled_by` lets the
  dossier go.
- `noting_unsupported_recipe` relabels only `NotApplied` rows from the plan
  loop; create rows never pass through it.

## Drafting Notes

- The skill fix and the engine decision are one item because the decision
  is small and both concern the same row. Split them if the decision needs
  wider discussion.

## References

- Source: PR #188 and its exploration record
  (`.accelerator/tmp/explore-pr-188-context.md`)
- Related: 0352, 0146
- PR reviews: `meta/reviews/prs/188-review-1.md`,
  `meta/reviews/prs/188-review-2.md`
