---
type: "work-item"
id: "0380"
title: "Imports Join the Plan at Discovery"
date: "2026-11-16T11:17:16+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0372"
blocked_by: ["work-item:0379"]
relates_to: ["work-item:0352"]
tags: ["sync", "work-adapters", "discovery", "architecture"]
last_updated: "2026-11-16T11:17:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-971"
---
# 0380: Imports Join the Plan at Discovery

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a maintainer of the sync engine, I want the remote issues a run will
import to be entries added to the plan when discovery finds them, so that
all the work a run does is in one collection and bounds are computed from
it.

## Context

Imports come from `untracked_to_import`, which searches the tracker after
planning. They are held as a list of external ids beside the plan, applied
by their own loop, and reported through `create_report` with the external
id placed where a local item's id goes.

Discovery is the one stage of a run that needs I/O after planning and adds
work no local item stands for.

## Requirements

- An entry names its subject explicitly: a local item or a remote issue.
- Discovery is a stage that takes a plan and returns it with import entries
  added.
- Imports are previewed and applied by the loop from 0379.
- The pull bound is computed from the plan alone.
- Stdout is unchanged.

## Acceptance Criteria

- [ ] `PreparedRun` holds no separate list of imports.
- [ ] No report row carries an external id in a field meant for a local
      item's id.
- [ ] Given discovery is skipped, deferred or fails, when the run reports,
      then the discovery status and deferral lines are as today.
- [ ] Given imports that would exceed `--max-pulls`, when the run is
      bounded, then it is refused with the same counts as today.
- [ ] The whole-stdout tests from 0377 pass unchanged.

## Open Questions

- Does the subject key chosen here survive 0385, where an item's id changes
  under it? The ADR from 0375 should settle the key before this starts.

## Dependencies

- Blocked by: 0379.
- Blocks: 0382.

## Assumptions

- Discovery does not read the plan: it takes the request, the settled view
  and the outstanding pending pushes.

## Technical Notes

- The discovery status and its deferral are facts about the run, not about
  a subject, and stay outside the plan.

## Drafting Notes

- The subject type is introduced here, not in 0383, because imports are the
  first entries with no local item.

## References

- Source: PR #188 and its exploration record
  (`.accelerator/tmp/explore-pr-188-context.md`)
- Related: 0352, 0146
- PR reviews: `meta/reviews/prs/188-review-1.md`,
  `meta/reviews/prs/188-review-2.md`
