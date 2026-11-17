---
type: "work-item"
id: "0372"
title: "Sync Run as a Typed Pipeline Over One Entry per Subject"
date: "2026-11-16T11:17:15+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "epic"
priority: "medium"
parent: "work-item:0146"
relates_to: ["work-item:0352"]
tags: ["sync", "architecture", "work-adapters", "work"]
last_updated: "2026-11-16T11:17:15+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0372: Sync Run as a Typed Pipeline Over One Entry per Subject

**Kind**: Epic
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a maintainer of the sync engine, I want a run modelled as typed stages
that transform one collection holding one entry per subject, so that one row
per item, watermark safety and the reporting rules hold by construction
instead of by convention.

A sync run today prepares its work as several parallel lists and rebuilds
the report by concatenating them. Every defect and review finding on 0352
sat at a seam between two of those lists.

## Context

0352 (PR #188) removed a duplicate `noop\tunsynced` row by moving each
create's entry out of the plan, as relinks already were. The fix is correct,
but it keeps "an item is in the plan or in the creates, never both" as a
rule held by two functions and their tests.

`prepare_run` splits a run's work across `plan.actions`, the imports found
by discovery, the local creates and relinks, the refused orders, the
unclassifiable items and the set of items with an unknown-recipe baseline
entry. The seams have each produced a fault or a finding:

- The duplicate row was one item held in two lists.
- The watermark fault was `finalise_baseline` trusting that the lists do not
  overlap.
- Whether an item draws the unknown-recipe warning depends on which list its
  row came from.
- A refused order prints beside its item's own row.
- `render_report` hides an engine row when an identity row has the same
  settled id: a merge of two sources done at print time.

The identity pass that runs before the engine already has the wanted shape:
it decides an `IdentityPlan` in full, bounds it, then applies it in order.

## Requirements

Goals:

- The plan states the work a run will do, including local creates, relinks
  and imports, and nothing is taken out of it before it is applied.
- One entry per subject (a local item or a remote issue), enriched stage by
  stage, so one row per item is structural.
- Each stage is its own type: an unbounded plan cannot be applied.
- Each kind of entry carries only the data that kind can have.
- Side effects are applied by one strategy per kind, in the adapter, behind
  one run context; the entries stay in the pure `work` crate.
- The stdout rows stay a published contract: any change to them is
  deliberate, tested through the binary and announced.

Initial stories, in order:

1. 0373: the sync skill reports complete for an item that needs an upgrade
   (worth doing whatever happens to the architecture).
2. 0374: advance a watermark only when every row for the item is settled
   (an interim safety net this epic later makes redundant).
3. 0375: record the architecture as an ADR.
4. 0376: the planner plans local creates.
5. 0377: pin the stdout row contract through the binary.
6. 0378: check the skill's Membership table against the engine's row shapes.
7. 0379: one apply loop for every planned entry.
8. 0380: imports join the plan at discovery.
9. 0381: unknown recipe becomes a fact on the entry.
10. 0382: unclassifiable items and refused orders become entries.
11. 0383: each kind of planned entry carries its own payload.
12. 0384: the engine's stages are distinct types.
13. 0385: identity pass results carry into the sync entry.
14. 0386: bring 0352's CHANGELOG entry and plan into line with what shipped.
15. 0387: decide whether an untouched unsynced item's watermark advances.
16. 0388: the sync skill summarises an untouched unsynced item as declined.
17. 0389: run 0352's outstanding manual sync checks.

This epic owns the fixes PR #188's review asked for, so the PR can merge
without them. Each finding marked to be fixed, and the item that owns it:

- `R2-01` (CHANGELOG advice and scope): 0386.
- `R2-03`, `R2-05` (a failed create over an unknown-recipe entry, and its
  missing test): 0373, made structural by 0381.
- `R2-06` (the unknown-recipe row matches no skill rule): 0373.
- `R2-07` (the `--pull-only` rule rests on an early return): 0376.
- `R2-09` (first-match test helpers beside `only_row`): 0374.
- The relabelling depends on which loop reported the row: 0373, then 0379
  and 0381.
- An untouched unsynced item's watermark still advances: 0387.
- Only one output is compared whole through the binary: 0377.
- Nothing ties the Membership table to the engine's rows: 0378.
- The entry sits under `Fixed` on an unverified claim: 0386.
- The plan is stale against what shipped: 0386.
- A lone `noop\tunsynced` row is summarised as declined: 0388.
- The manual checks are unticked: 0389.
- The direction rule is written down nowhere near the code; the planner
  still plans `Noop` for an item the adapter creates; the two takes use
  different idioms: 0376, which removes the code all three describe.

## Acceptance Criteria

- [ ] No function takes an entry out of the plan between planning and
      applying.
- [ ] Every local item a run considers has exactly one entry, and every
      report row is derived from exactly one entry.
- [ ] `render_report` suppresses no row on the grounds that another source
      reported the same item.
- [ ] The whole-stdout tests from 0377 pass, changed only where a child
      item states a deliberate change to the output.
- [ ] The ADR from 0375 is accepted and describes the code as built.
- [ ] Every finding PR #188's review marked to be fixed is closed by the
      item listed for it above.

## Open Questions

- Is the epic allowed to pause after 0382? By then one row per item is
  structural for the engine, and 0383 to 0385 are the larger, riskier half.
- Does 0385 need splitting once the ADR settles how a subject is keyed
  across a retirement?

## Dependencies

- Blocked by: 0352 (PR #188) merging, since every child builds on its head.
- Blocks: none.

## Assumptions

- Reaching this architecture is a goal, not only a recorded direction.
- `SyncPlan` and `PlannedAction` are public API of the `work` crate, so
  reshaping them is a deliberate, checked API change.

## Technical Notes

- Rejected in discussion: trait objects for the entries (the crate boundary
  forbids effects on core types, exhaustive matching does safety work in the
  dossier mapping, and there are few kinds but many operations over them);
  and one mutable plan that means different things over time.
- Creates and relinks need no late information: direction, drafts and orders
  are all known when `plan()` runs. Discovery is the only stage that adds
  entries.

## Drafting Notes

- 0373 and 0374 are placed under this epic at the author's request, although
  both stand on their own.
- The epic sits under 0146, the parent of 0352.
- Line numbers are deliberately absent: the code is cited by function name,
  as read at PR #188's head `581fee36`.

## References

- Source: PR #188 and its exploration record
  (`.accelerator/tmp/explore-pr-188-context.md`)
- Related: 0352, 0146
- PR reviews: `meta/reviews/prs/188-review-1.md`,
  `meta/reviews/prs/188-review-2.md`
