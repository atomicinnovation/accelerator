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
relates_to: ["work-item:0218", "work-item:0296", "work-item:0300", "work-item:0302", "work-item:0334", "work-item:0339", "work-item:0343", "work-item:0344", "work-item:0345", "work-item:0346"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/172"
pr_number: 172
tags: []
revision: "f358ab2d229197a12930154c06649a5d7312156a"
repository: "accelerator"
last_updated: "2026-10-29T23:20:59+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0146] List every child of the sync epic

## Summary

Epic 0146's Stories section listed 17 of its 23 children. The six it left
out (0294, 0300, 0319, 0335, 0336 and 0338) are all still open, so the epic
under-reported the work left to do. This PR adds them, brings six work
items' statuses up to date, and records a Linear sync that linked five
work items to new issues. It changes only `meta/work/` and the Linear sync
state; there is no code change.

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

- **Linear links.** A sync created issues for 0334 and 0343–0346, and each
  of those work items gains its `external_id` (PP-930 to PP-934).
- **Sync state.** `.accelerator/state/integrations/linear/last-sync.json`
  records that sync:

  | Entries | Change |
  | --- | --- |
  | 0334, 0343–0346 | added |
  | 320 others | baseline re-recorded with split body, field and frontmatter digests (`digest_recipe: 1`) |
  | 17 conflicts | `local_hash` blanked; the sync skipped them |
  | 0121, 0281, 0284, 0349 | unchanged from `main` |

## Context

The gap turned up while working out 0146's dependency tree from every work
item's `parent`, `blocked_by` and Dependencies section. The `parent:` field
named 23 children, but the Stories section only listed 17.

The sync ran on top of this branch, before it was rebased onto `main`. The
rebase conflicted only in the sync state, because `main` had meanwhile
synced 0121, 0281, 0284 and 0349. No entry changed on both sides, so the
merge takes `main`'s entries for those four and the sync's entries for
everything else.

## Testing

- [x] `accelerator work list` parses all 347 work items, and
      `accelerator work list --parent 0146` lists all 23 children.
- [x] The merged sync state keeps every one of `main`'s 341 entries, with
      `main`'s four newer ones byte-identical; the only additions are the
      five new links.
- [ ] `mise run check` not run: the change touches only Markdown under
      `meta/work/` and the sync-state JSON, which no check lane covers.

## Notes for Reviewers

- The sync left these for the next `/sync-work-items` run, and
  `accelerator work list` shows each as 🔴 `conflict`:
  - All six work items this PR edits (0146, 0218, 0296, 0300, 0302 and
    0339). 0146's Stories change still needs pushing to PP-167.
  - Eleven older ones: 0222, 0223, 0225, 0303, 0304, 0307–0310, 0332 and
    0347.
  - `main`'s four: 0121, 0281, 0284 and 0349. Their entries and files
    match `main`, so they're conflicts there too.
- A blanked `local_hash` is the sync's own record of a skipped conflict,
  not damage from the rebase.
