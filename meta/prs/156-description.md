---
type: "pr-description"
id: "156"
title: "[0296] Review sync round-trip defects and settle body equivalence"
date: "2026-10-10T20:54:03+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0296"
parent: "work-item:0296"
relates_to: ["work-item:0146", "work-item:0230", "work-item:0290", "work-item:0291", "work-item:0320", "work-item:0321"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/156"
pr_number: 156
tags: ["sync", "work-item-review", "linear", "jira"]
revision: "67bf68772c25b2000d4481d18e1c40de47452caa"
repository: "accelerator"
last_updated: "2026-10-10T21:01:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0296] Review sync round-trip defects and settle body equivalence

## Summary

Work item 0296 went through five review passes, and this PR applies their
findings so that it is ready for planning. 0296 covers why
`accelerator work sync` is unsafe to run on this corpus. Pulls write the
tracker's projected title line into local files. Linear's cosmetic changes
to the markdown also register as edits and as conflicts: a preview run on
2026-10-10 planned 7 pulls and 25 conflicts, most of them noise. The review
closes with a COMMENT verdict, and its decisions are carried into four
related work items. Two new items, 0320 and 0321, cover the parts 0296
deliberately leaves out.

## Changes

- **0296 renamed and reworked as a tracker-neutral story.** The file is now
  `…-and-the-tracker.md`, and the kind changes from `bug` to `story`. It now
  defines when two bodies count as **equivalent**: they parse to the same
  CommonMark events after a fixed set of normalisations, using a pinned
  parser. A table lists Linear's 13 observed changes to markdown, each with
  a real example: 10 are cosmetic and 3 change meaning. It also specifies:
  - splitting the local digest into frontmatter and body hashes, with a
    `digest_recipe` marker on each baseline entry;
  - migrating old entries, which have no marker and use a **legacy recipe**,
    so that one full sync converts every entry it classifies;
  - a recovery table for items with a missing or empty baseline;
  - that `--preview` is read-only, and the new dossier `title:` form
    `<local> → <remote>`.

  There are 38 Given/When/Then criteria.
- **New review file** at
  `meta/reviews/work/0296-…-review-1.md`, containing all five passes and the
  verdict change from REVISE to COMMENT.
- **0320 (new):** Linear rewrites that change meaning on push. It is aligned
  with 0296's table: 13 kinds in total, with dotted tokens such as
  `crates.io` included. It is blocked by 0296, and gains a criterion for
  repairing 0161, 0203, 0276 and 0293 once it lands.
- **0321 (new):** an ADR task to decide which side is the authority for each
  thing sync touches. 0296 goes ahead without waiting for it. Its Context
  quotes 0296's revised rule.
- **0290:** the title joins the synced fields, with Linear `title` and Jira
  `summary` mapped directly. The baseline stores each field's last-synced
  value. It is blocked by 0296.
- **0291 and 0230:** 0291's pull-path changes wait for 0296's title-line
  strip. 0230, which is in progress, coordinates with 0296: whichever lands
  second adapts.
- **0146:** the Stories list adds 0296, 0320 and 0321 with their landing
  order.
- **0230 plan and research:** the citations now use 0296's new filename.

## Context

- Work item: `meta/work/0296-sync-round-trip-defects-between-local-work-items-and-the-tracker.md`
- Review: `meta/reviews/work/0296-sync-round-trip-defects-between-local-work-items-and-the-tracker-review-1.md`
- Parent epic: `meta/work/0146-work-item-sync-enhancements.md`

The rewrite table and the list of conflicted items come from a scan of the
2026-10-10 conflict dossiers in
`.accelerator/state/integrations/linear/conflicts/`. That directory is not
committed and is overwritten by the next sync preview. The fixtures will be
re-captured with `accelerator linear show` before implementation starts.

## Testing

- [x] `accelerator corpus frontmatter validate` passes for all 10 changed
      files that remain after the rename.
- [x] No file under `meta/` still links to 0296's old filename.
- [ ] `mise run` not run: the PR changes only `meta/` markdown.

## Notes for Reviewers

- **Decisions to check.** Each of these settles behaviour that later code
  depends on:
  - A missing or empty local hash counts as *unknown*, not *changed*.
  - An update is sent only when the local side changed *and* the local
    title differs from the remote one. This means a remote title edit is
    never overwritten unless a local change is pushed.
  - Jira coverage is preventive, since no Jira noise has been observed.
- **Accepted scope.** The scope lens repeatedly argued that 0296 should be
  split into an epic. The author kept it whole and changed the kind to
  `story` instead. A split is still an option before planning.
- **Open before work starts.** These are listed in 0296's Open Questions:
  - choosing between `pulldown-cmark` and `comrak`;
  - where `adf` lives: `remote-projection` depending on `jira-client`, or
    `adf` moving down a crate;
  - capturing an `issueCreate` read-back.
- **Known follow-up.** No work item yet removes the legacy digest recipe.
  Its trigger is reachable, because one full sync migrates every entry it
  classifies.
- **Title on PP-880.** 0296's local title changed. Title sync is outside
  this item's scope, so the Linear issue's title needs a manual rename.
