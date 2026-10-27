---
type: "pr-description"
id: "172"
title: "[0146] List every child of the sync epic"
date: "2026-10-27T23:22:33+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0146"
parent: "work-item:0146"
relates_to: ["work-item:0218", "work-item:0296", "work-item:0300", "work-item:0302", "work-item:0339"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/172"
pr_number: 172
tags: []
revision: "0cd12d207a31f2c697c7d2117f8d58eb5376a999"
repository: "accelerator"
last_updated: "2026-10-27T23:22:33+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0146] List every child of the sync epic

## Summary

Epic 0146's Stories section listed 17 of its 23 children. The six it left
out (0294, 0300, 0319, 0335, 0336 and 0338) are all still open, so the epic
under-reported the work left to do. This PR adds them and brings six work
items' statuses up to date. It changes only `meta/work/`; there is no code
change.

## Changes

- **Stories list on 0146.** Adds a one-line entry for each missing child,
  with its blockers, in the style of the existing entries. 0294 sits next to
  0293 and 0335 next to 0321, since they're related; the other four go at the
  end. `last_updated` is bumped.
- **Stale cross-reference in 0296.** Its Dependencies section described 0230
  as "in progress"; 0230 is done.
- **Status updates.** Two earlier commits that were never pushed move
  statuses to match the work:

  | Item | From | To |
  | --- | --- | --- |
  | 0218 | ready | in-progress |
  | 0296 | in-progress | done |
  | 0300 | draft | in-progress |
  | 0302 | ready | done |
  | 0339 | ready | done |

## Context

The gap turned up while working out 0146's dependency tree from every work
item's `parent`, `blocked_by` and Dependencies section. The `parent:` field
named 23 children, but the Stories section only listed 17.

## Testing

- [x] `accelerator work list --parent 0146` parses every edited file and
      lists all 23 children.
- [ ] `mise run check` not run: the change touches only Markdown under
      `meta/work/`, which no check lane covers.

## Notes for Reviewers

- After this merges, the next `/sync-work-items` run has these to deal with:
  - 0146's description changes, so the PP-167 issue in Linear needs a push.
  - 0296, 0300 and 0302 show 🔴 `conflict` in `work list`, because of the
    status commits. Expect to resolve those three.
  - 0343, 0345 and 0346 have never been synced.
