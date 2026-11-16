---
type: "work-item"
id: "0375"
title: "Record the Sync Pipeline Architecture as an ADR"
date: "2026-11-16T11:17:15+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0372"
relates_to: ["work-item:0352"]
tags: ["sync", "architecture", "adr"]
last_updated: "2026-11-16T11:17:15+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0375: Record the Sync Pipeline Architecture as an ADR

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Record, as an ADR, the target architecture for a sync run: typed stages over
one entry per subject, closed entry kinds in the pure crate, and adapter-side
strategies for preview and apply. The later items in this epic each rely on
one of its decisions.

## Context

The architecture was worked out while reviewing PR #188. The reason on
record for leaving the create decision in the adapter (only the adapter
knows direction, drafts and orders) is not true of the code: `plan()`
receives the direction and the pending-push orders, and `DraftId` lives in
the `work` crate. No ADR captures either the current design or the target.

## Requirements

The ADR decides and records:

- The stage list and what each stage adds: resume, identity plan,
  rediscover, sync plan, discover imports, bound, then preview or apply.
  Creates and relinks belong to the initial sync plan; discovery is the only
  stage that adds entries.
- How a subject is keyed, including across a draft id being retired onto a
  tracker key.
- Closed entry kinds with their own payloads in `work`, and one strategy per
  kind in `work-adapters`, sharing one run context.
- The stance on stdout: row order and shapes are a published contract, and
  whether a refused order folds into its item's row.
- How a stopped stage reports the changes that already landed.
- How the change to the `work` crate's public API is made and checked.
- The alternatives rejected: trait objects for entries, one mutable plan,
  and filtering the report instead of fixing ownership.

## Acceptance Criteria

- [ ] An ADR exists under `meta/decisions/` covering each decision above,
      with the alternatives considered and their costs.
- [ ] It has been reviewed with `/review-adr` and accepted.
- [ ] It corrects the recorded rationale for where the create decision
      lives.
- [ ] Each of 0379 to 0385 can name the ADR decision it implements.

## Open Questions

- One ADR, or one for the pipeline and one for the entry model? A single
  ADR is assumed.

## Dependencies

- Blocked by: none.
- Blocks: 0379, and through it 0380 to 0385.

## Assumptions

- An accepted ADR is immutable, so the decisions must be settled before it
  is accepted, not refined as the stories land.

## Technical Notes

- `settle_identities` is a precedent to cite: it decides an `IdentityPlan`
  in full, checks its ceilings, then applies it in order.

## Drafting Notes

- A dedicated item, not part of a story, because six later items depend on
  its decisions.

## References

- Source: PR #188 and its exploration record
  (`.accelerator/tmp/explore-pr-188-context.md`)
- Related: 0352, 0146
- PR reviews: `meta/reviews/prs/188-review-1.md`,
  `meta/reviews/prs/188-review-2.md`
