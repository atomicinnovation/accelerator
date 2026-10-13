---
type: "pr-description"
id: "161"
title: "Capture 0332 and 0333 and reconcile work item links"
date: "2026-10-13T15:58:26+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
relates_to: ["work-item:0332", "work-item:0333", "work-item:0136"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/161"
pr_number: 161
tags: []
revision: "faaf7c6287c898a91764657c3405e3e1da296e90"
repository: "accelerator"
last_updated: "2026-10-13T15:58:26+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Capture 0332 and 0333 and reconcile work item links

## Summary

This PR adds two new work items: 0332 (push PR descriptions to GitHub
unwrapped) and 0333 (work item writers omit the required `tags` field). It
also brings five existing items into line with reality: two are marked done,
missing `blocked_by` links are recorded, and the 0136 epic is reconciled with
its actual child set. All changes are to `meta/` and sync state; no code or
skills change.

## Changes

- **New: 0332** (story, PP-885). `accelerator collaboration pr update-body`
  should unwrap paragraphs, list items and blockquotes before PATCHing, while
  the `meta/prs/` artefact stays wrapped per repository convention.
- **New: 0333** (bug, PP-886). `compose_frontmatter` in `cli/work/src/create.rs`
  omits `tags` when empty, but the corpus schema requires it. `work create`,
  batch create and the sync importer all write items that fail
  `accelerator corpus frontmatter validate`. Both new items needed `tags: []`
  added by hand for exactly this reason.
- **Done: 0230 and 0299.** Frontmatter and body status move to Done.
- **Dependencies:** 0281 gains `blocked_by` 0278 (the frontend registration
  that renders the `report` kind); 0304 gains `blocked_by` 0310.
- **Epic 0136 reconciled:**
  - fourteen direct children missing from the Children line are added;
  - 0286 joins the Decomposition list;
  - Decomposition titles now match each item's `title` verbatim;
  - the body status moves from Ready to In Progress, matching the
    frontmatter;
  - a new Dependencies line notes 0227, 0298, 0271 and 0273, which are
    blocked by its children but sit outside the epic.
- **Linear sync baseline** in `.accelerator/state/integrations/linear/last-sync.json`
  is refreshed to record 0332 and 0333.

## Context

0332 came out of `describe-pr` producing ragged 80-column bodies on GitHub.
0333 surfaced while 0332 was being written.

## Testing

- [x] `accelerator corpus frontmatter validate` passes for all seven changed
      work items.
- [x] `last-sync.json` parses and carries entries for 0332 and 0333.
- [ ] CI `check` lane. Not run locally; this PR touches no source.

## Notes for Reviewers

The 0136 diff is large but mechanical: most lines are title re-syncs. The
substantive changes are the Children line and the new note that records the
2026-10-10 reconciliation. In line with 0332's intent, this body was
unwrapped for GitHub; the `meta/prs/161-description.md` artefact stays
wrapped.
