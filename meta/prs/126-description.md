---
type: "pr-description"
id: "126"
title: "Mark work item 0258 (help shows subcommands) done"
date: "2026-09-20T19:29:50+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
relates_to: ["work-item:0146", "work-item:0258", "work-item:0290", "work-item:0291"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/126"
pr_number: 126
tags: ["work-item", "epic-0146", "sync", "backlog"]
revision: "744f7fd981e4ff3ed28e429f51ebf45434253410"
repository: "accelerator"
last_updated: "2026-09-20T19:29:50+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Mark work item 0258 (help shows subcommands) done

## Summary

Documentation-only meta-directory bookkeeping in three commits: close work item
0258 (help should show subcommands), extract two new stories — 0290 and 0291 —
from epic 0146 to cover its outstanding field-mapping and parent-child
acceptance criteria, and add one item to the ideas backlog. No code changes.

## Changes

- Mark 0258 done: flip `status` from `ready` to `done` in both the frontmatter
  and the body header of `meta/work/0258-help-show-subcommands.md`.
- Extract 0290 (*Bidirectional Field Mapping for Work Item Sync*): a new draft
  story under epic 0146 for synchronising status, kind, and priority
  bidirectionally across Jira and Linear via per-tracker mapping tables, with
  last-writer-wins conflict resolution and skip-and-warn on unmappable values.
- Extract 0291 (*Parent-Child Relationship Synchronisation for Work Item
  Sync*): a new draft story under epic 0146 for the single parent edge across
  sync, with topological parents-first push ordering and cycle detection.
- Add `[Task] Add internal CLI wrapper skills` to the third ideas backlog note.

## Context

0290 and 0291 decompose the remaining field-mapping and parent-child
acceptance criteria of epic 0146
(`meta/work/0146-work-item-sync-enhancements.md`) into independently trackable
stories; both are siblings of 0228 (done) and 0229 (ready). 0258 is closed as
done under parent 0276.

## Testing

- [x] Metadata-only change — no code paths touched; verified against the diff.
- [x] The two new work items carry `schema_version: 1` frontmatter consistent
      with the `extract-work-items` template.
- [ ] Aggregate `mise run check` not run — it targets the four code toolchains,
      not meta markdown, so it exercises none of this change.

## Notes for Reviewers

- The branch bundles three unrelated meta changes. The PR title names the 0258
  closure, but the bulk of the diff (275 of 282 added lines) is the 0290/0291
  extraction; the `Extract 0290 and 0291` commit was already the branch's base
  above `main`, so it rides along here.
- Both new stories flag the same limitation in their open questions:
  `last_updated` is item-level rather than per-field, so under last-writer-wins
  a stale unrelated field on the more-recently-touched side can win. Worth a
  view before either is planned.
- 0291 leaves the exact Jira parent mechanism (parent field vs epic link) open;
  0290 leaves the priority round-trip and opt-out granularity open.
