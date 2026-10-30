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
revision: "be0c0493c43f181e7291b66ae0e84a74b78ae134"
repository: "accelerator"
last_updated: "2026-10-30T02:35:58+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0146] List every child of the sync epic

## Summary

Epic 0146's Stories section listed 17 of its 23 children. The six it left
out (0294, 0300, 0319, 0335, 0336 and 0338) are all still open, so the epic
under-reported the work left to do. This PR adds them and brings six work
items' statuses up to date. It also records two Linear syncs, which leave
every work item in sync with Linear.

It also fixes the two CI failures `main` has had since PR #171 merged: a
`cargo-deny` advisory against `pulldown-cmark` 0.13.4, and a re-key test
that fails on Linux only.

## Changes

### Work items and sync state

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

### CI fixes

- **Dirty paths in path order.** `InProcessProbe::working_copy_state` now
  sorts the dirty paths both backends return. git's status walk yielded
  untracked files in the filesystem's directory order, so
  `a_user_file_named_like_rekeys_machinery_is_dirt` passed on APFS and
  failed on ext4. The probe's contract now says the paths come in path
  order, and new tests in `vcs-adapters/tests/dirty_paths.rs` pin it for
  git and jj.
- **`pulldown-cmark` 0.13.5.** RUSTSEC-2026-0414 reports exponential event
  growth in 0.13.4's wikilink parsing. We never enable wikilinks, but
  `deny.toml` takes every vulnerability through an upgrade, never an
  ignore. The third-party notices are regenerated.
- **Recipe guard.** The guard in `golden_digests.rs` used to demand a
  digest-recipe bump for any new parser version. It now also accepts a
  recipe re-verified under a new version: `recipe.json` records
  `(1, "0.13.5")` after `(1, "0.13.4")`. Recipe numbers still can't skip
  or go back, and every golden digest still has to hold.

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

`main`'s "Check supply chain" and "Run unit tests (ubuntu-latest)" jobs have
failed since the PR #171 merge (run 40970873387), as have the 0218, 0300 and
0306 branches. The fixes ride on this PR at the author's request rather than
in a PR of their own.

Bumping the digest recipe for 0.13.5 would have needed a designed recipe
1 → 2 re-baseline that doesn't exist yet. Without one, any item whose parse
moved would show changed digests on both sides at once and become a
conflict.

## Testing

- [x] `mise run check` passes.
- [x] `mise run test:unit:cli` passes: 6,608 tests, 2 skipped.
- [x] `mise run deny:check` reports advisories, bans, licences and sources
      ok, and `mise run notices:check` passes.
- [x] `git_lists_dirty_paths_in_path_order` failed before the sort, listing
      eight files in APFS hash order; it and the jj counterpart pass after.
- [x] Every golden digest pair matches under 0.13.5; before the recipe
      history recorded it, only the version guard failed.
- [x] `accelerator work list` reports all 347 work items as synced, and a
      `accelerator work sync --preview` afterwards plans nothing.
- [x] `accelerator work list --parent 0146` lists all 23 children.
- [x] After the rebase merge, the sync state kept every one of `main`'s 341
      entries, with `main`'s four newer ones byte-identical.
- [ ] Linux CI: the ordering fix is verified on APFS locally, but not yet
      on ext4.

## Notes for Reviewers

- The recipe-guard change relaxes the pin policy in `cli/Cargo.toml`. A
  parser release now needs a recipe bump only if a golden digest moves, not
  always. The risk this accepts: a document shape the golden pairs don't
  cover could parse differently under 0.13.5. Its digests would then move
  on both sides at once, and it would show as a spurious conflict at the
  next sync.
- Linear already reflects this branch. The second sync overwrote 21 Linear
  issues with their local versions, among them PP-167 (0146), which now
  carries the full Stories list. If this PR doesn't merge, Linear is ahead
  of `main` for those issues.
- The two Linear issues edited after their last sync were PP-885 (0332) and
  PP-923 (0347). Both held earlier drafts of their work items, so nothing in
  them was newer than the local file.
