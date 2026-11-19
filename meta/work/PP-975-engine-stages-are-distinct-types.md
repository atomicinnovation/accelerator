---
type: "work-item"
id: "PP-975"
title: "Engine Stages Are Distinct Types"
date: "2026-11-16T11:17:16+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "low"
parent: "work-item:PP-963"
blocked_by: ["work-item:PP-974"]
relates_to: ["work-item:PP-927"]
tags: ["sync", "work-adapters", "work", "architecture"]
last_updated: "2026-11-16T11:17:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-975"
aliases: ["0384"]
---
# PP-975: Engine Stages Are Distinct Types

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

As a maintainer of the sync engine, I want each stage of the engine's run to
produce its own type, so that a plan cannot be applied before it has been
through discovery and the bounds check, and a name never means two things.

## Context

`prepare_run` builds the plan, changes it in place, gathers the other lists
and checks the bounds in one function. Its result keeps the planner's name
`plan` for what is by then a different thing, and `validate_pushes` still
documents that "the report never shrinks below the plan".

## Requirements

- Planned, discovered and bounded are distinct types; only a bounded plan
  can be previewed or applied.
- Preview and apply take the same bounded plan, so both report the same
  work.
- Pull and push counts are computed from the plan alone, including creates
  and excluding adopts.
- The parallel lists in `PreparedRun` are gone.
- Stdout is unchanged.

## Acceptance Criteria

- [ ] Applying or previewing a plan that has not been bounded does not
      compile.
- [ ] `PreparedRun` is replaced by the bounded plan and the run-level facts
      (discovery status and deferral, read failure, baseline degradation,
      leftover attempt markers).
- [ ] Given a run over its `--max-pulls` or `--max-pushes`, then it is
      refused before anything is applied, with the same counts as today.
- [ ] Doc comments that describe the plan describe the stage they are on.
- [ ] The whole-stdout tests from 0377 pass unchanged.

## Open Questions

- Do the stage types live in `work`, with the adapter supplying the facts
  each transition needs, or in `work-adapters`?

## Dependencies

- Blocked by: 0383.
- Blocks: 0385.

## Assumptions

- The pure transitions (add imports, bound) can be functions over data in
  the `work` crate.

## Technical Notes

- The bounds refusal reports new local files and new remote issues
  separately; both must still be derivable.

## Drafting Notes

- Scoped to the engine. The identity pass's stages are 0385.

## References

- Source: PR #188 and its exploration record
  (`.accelerator/tmp/explore-pr-188-context.md`)
- Related: 0352, 0146
- PR reviews: `meta/reviews/prs/188-review-1.md`,
  `meta/reviews/prs/188-review-2.md`
