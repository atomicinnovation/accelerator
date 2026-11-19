---
type: "work-item"
id: "PP-980"
title: "Run PP-927's Outstanding Manual Sync Checks"
date: "2026-11-16T11:25:19+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:PP-963"
relates_to: ["work-item:PP-927", "work-item:PP-926", "work-item:PP-968"]
tags: ["sync", "sync-work-items", "manual-verification"]
last_updated: "2026-11-16T11:25:19+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-980"
aliases: ["0389"]
---
# PP-980: Run PP-927's Outstanding Manual Sync Checks

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

PP-927 merged with its manual checks unticked: the two `/sync-work-items`
checks in its plan, and the runs against the `TES` sandbox its PR
description lists as still to do. Run them and record the results.

## Context

Raised as an observation on PR #188's chapter "What the skill is told" and
marked to be fixed. This epic owns it instead of the PR.

The skill change in PP-927 is verified only by a scan of the skill's text.
That an item with a refused order is summarised once by a model following
the narrowed Membership rule has never been observed. The PR's own testing
notes list a preview, an `Attempted` pending-push marker and `--pull-only`
against the sandbox as outstanding.

## Requirements

Against the `TES` sandbox team, on a build containing PP-927:

- A preview and an apply of an unsynced item: one row each, by its create.
- An apply whose create fails: one `failed` row and no `noop` row.
- An unsynced item with an `Attempted` pending-push marker: one row.
- An unsynced item under `--pull-only`: its single `noop` row.
- `/sync-work-items` over a `--resolve` order on a draft: the draft is
  summarised once, under `needs-attention`.
- `/sync-work-items` over a run that creates an item: the item is
  summarised once.

## Acceptance Criteria

- [ ] Each check above is recorded with the command run, the stdout seen
      and the exit code.
- [ ] The two manual checks in PP-927's plan are ticked, or a defect is filed
      for each that fails.
- [ ] Any defect found is a work item under PP-963.

## Open Questions

- None.

## Dependencies

- Blocked by: PP-927 (PR #188) merging, and a pre-release build containing
  it.
- Blocks: none.

## Assumptions

- The `TES` sandbox team is still available for manual checks.

## Technical Notes

- PP-926 is the known defect for the `Attempted` marker under `--preview`;
  expect its symptom until it is fixed.
- The stdout captured here is useful source material for PP-968's tests.

## Drafting Notes

- A person has to run these; they cannot be automated without the sandbox
  credentials.

## References

- Source: PR #188's review notes and its exploration record
  (`.accelerator/tmp/explore-pr-188-context.md`)
- Related: PP-927, PP-963
- PR reviews: `meta/reviews/prs/188-review-1.md`,
  `meta/reviews/prs/188-review-2.md`
