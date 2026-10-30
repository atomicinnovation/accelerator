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
relates_to: ["work-item:0218", "work-item:0296", "work-item:0300", "work-item:0302", "work-item:0334", "work-item:0339", "work-item:0343", "work-item:0344", "work-item:0345", "work-item:0346", "work-item:0348"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/172"
pr_number: 172
tags: []
revision: "5c77a83a25a57886df321a0e6d1cd9ecfdc696af"
repository: "accelerator"
last_updated: "2026-10-30T01:17:39+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0146] List every child of the sync epic

## Summary

Epic 0146's Stories section listed 17 of its 23 children. The six it left
out (0294, 0300, 0319, 0335, 0336 and 0338) are all still open, so the epic
under-reported the work left to do. This PR adds them and brings six work
items' statuses up to date. It also records two Linear syncs, which leave
every work item in sync with Linear. It changes only `meta/work/` and the
Linear sync state; there is no code change.

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

- **Linear links.** The syncs created issues for six work items, and each
  gains its `external_id`: 0334 and 0343–0346 (PP-930 to PP-934), and 0348
  (PP-936).
- **Sync state.** `.accelerator/state/integrations/linear/last-sync.json`
  goes from 341 entries to 347, adding those six. Every entry now carries
  split body, field and frontmatter digests (`digest_recipe: 1`), and every
  baseline is re-recorded at the second sync.

## Context

The gap turned up while working out 0146's dependency tree from every work
item's `parent`, `blocked_by` and Dependencies section. The `parent:` field
named 23 children, but the Stories section only listed 17.

The first sync ran on top of this branch before it was rebased onto `main`.
It linked five work items, moved most entries to split digests, and skipped
21 items as conflicts. The rebase conflicted only in the sync state, because
`main` had meanwhile synced 0121, 0281, 0284 and 0349. No entry changed on
both sides, so the merge takes `main`'s entries for those four and the
sync's entries for everything else.

The second sync ran after the rebase. It pushed 0278, 0305 and 0306, linked
0348, and resolved all 21 conflicts as `local`. For 19 of them Linear hadn't
changed since the last sync, so the local file held the only edits; for 0332
and 0347, Linear held an older version than the local file.

## Testing

- [x] `accelerator work list` reports all 347 work items as synced, and a
      `accelerator work sync --preview` afterwards plans nothing.
- [x] `accelerator work list --parent 0146` lists all 23 children.
- [x] `accelerator corpus frontmatter validate` passes for 0348, the only
      work item the second sync wrote.
- [x] After the rebase merge, the sync state kept every one of `main`'s 341
      entries, with `main`'s four newer ones byte-identical.
- [ ] `mise run check` not run: the change touches only Markdown under
      `meta/work/` and the sync-state JSON, which no check lane covers.

## Notes for Reviewers

- Linear already reflects this branch. The second sync overwrote 21 Linear
  issues with their local versions, among them PP-167 (0146), which now
  carries the full Stories list. If this PR doesn't merge, Linear is ahead
  of `main` for those issues.
- The two Linear issues edited after their last sync were PP-885 (0332) and
  PP-923 (0347). Both held earlier drafts of their work items, so nothing in
  them was newer than the local file.
- A work item changed on `main` before this merges shows as locally
  modified at the next sync, as usual.
