---
type: "work-item"
id: "PP-974"
title: "Each Kind of Planned Entry Carries Its Own Payload"
date: "2026-11-16T11:17:16+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "low"
parent: "work-item:PP-963"
blocked_by: ["work-item:PP-970", "work-item:PP-971", "work-item:PP-972", "work-item:PP-973"]
relates_to: ["work-item:PP-927"]
tags: ["sync", "work", "public-api", "architecture"]
last_updated: "2026-11-16T11:17:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-974"
aliases: ["0383"]
---
# PP-974: Each Kind of Planned Entry Carries Its Own Payload

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

As a maintainer of the sync engine, I want each kind of planned entry to be
its own type holding only what that kind can have, so that combinations the
planner never produces cannot be built and no reader has to defend against
them.

## Context

`PlannedAction` is flat: an id, a state, an action and an optional
instruction. It permits an order on a pull, a resolution on a create and
many state and action pairs the planner never emits. The dossier mapping
answers `Unobserved` for a long list of them so that "a surprise never
retires a dossier", and the apply path has fallbacks for an entry whose
local item is missing.

## Requirements

- A closed set of entry kinds in the `work` crate, each with its own
  payload.
- An order exists only on a create or a relink; a resolution only on the
  actions a resolution produces; an import carries a tracker key.
- The strategies from 0379 are implemented per payload type.
- Readers of state and action (the dossier mapping, the settled-row rule,
  the awaiting-human rule, the renderer) match exhaustively on the kinds.
- The change to the `work` crate's public API is deliberate and recorded.
- Stdout is unchanged.

## Acceptance Criteria

- [ ] An entry with an order on a kind that cannot carry one does not
      compile.
- [ ] The dossier mapping has no catch-all arm for combinations that can no
      longer be built.
- [ ] The apply strategies have no fallback for a missing local item where
      the kind guarantees one.
- [ ] The public API check is updated in the same change, with the removed
      and added items listed in the PR description.
- [ ] The whole-stdout tests from 0377 pass unchanged, and the row-shape
      fixture from 0378 is unchanged.

## Open Questions

- Do the report rows keep `PlannedAction` as their public shape for
  consumers of `RunReport`, or change with the entries?

## Dependencies

- Blocked by: 0379, 0380, 0381, 0382.
- Blocks: 0384.

## Assumptions

- No consumer outside this repository depends on the `work` crate's API.

## Technical Notes

- Several readers are `const fn` and tests compare planned actions by
  equality and clone them; an enum of payload structs keeps both.
- Tests that build a planned action by hand will need a builder per kind.

## Drafting Notes

- Ordered after the single loop and single collection so that the API
  changes once, against the final set of kinds.

## References

- Source: PR #188 and its exploration record
  (`.accelerator/tmp/explore-pr-188-context.md`)
- Related: 0352, 0146
- PR reviews: `meta/reviews/prs/188-review-1.md`,
  `meta/reviews/prs/188-review-2.md`
