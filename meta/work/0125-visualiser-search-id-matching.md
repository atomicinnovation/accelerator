---
type: work-item
id: "0125"
title: "Visualiser Search Does Not Match Work Item IDs"
date: "2026-06-22T15:30:57+00:00"
author: Phil Helm
producer: create-work-item
status: draft
kind: bug
priority: medium
relates_to: ["work-item:0054"]
tags: [visualiser, search]
last_updated: "2026-06-22T15:42:00+00:00"
last_updated_by: Phil Helm
schema_version: 1
---

# 0125: Visualiser Search Does Not Match Work Item IDs

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Phil Helm

## Summary

Typing a work item's number into the visualiser sidebar search box returns
"No matches", even when that item exists. The search backend ranks entries
only on title, slug, and body preview — it never consults the item's ID — so
no ID query can ever match.

## Context

Work item 0054 ("Sidebar Search Input and API Search Endpoint") built the
`GET /api/search` endpoint and its four-bucket ranking. 0054 explicitly
decided (its design notes, line 121) that "search work items by ID" would be
served by the exact-slug bucket — "typing `0054` puts work item 0054 at the
top" — without a dedicated ID-aware code path.

That decision does not hold in practice: a work item's slug is the filename
tail *after* the ID (`0054-sidebar-search.md` → slug `sidebar-search`), so the
bare ID is absent from every field the ranker inspects. The ID lives in a
separate `work_item_id` field on the index entry (derived from frontmatter
`id:`), which `classify()` never reads. This is a gap against 0054's
stated-but-non-functional behaviour, not a new capability.

The empty-results panel compounds the confusion: it advises users to "Try a
slug, a fragment of a title, or a doc id" — but a doc id is exactly what the
backend cannot match.

## Requirements

Reproduction:
1. Open the visualiser with at least one work item present (e.g. `id: 0054`).
2. Focus the sidebar search box and type the work item's number (`0054`).

Expected: the matching work item appears in the results.
Actual: the panel shows "No matches".

The fix must:
- Match the query against the entry's `work_item_id` (local `id`), in addition
  to the existing title / slug / body-preview fields.
- Apply the same exact / prefix / interior tiering already used for the slug,
  so an exact ID ranks at the top, a prefix matches, and an interior substring
  matches (typing `42` finds `0042`).
- Not change the existing ranking behaviour for title / slug / body matches.

## Acceptance Criteria

- [ ] Given a work item `id: "0042"`, when I search `0042`, then it appears in
      the results (ranked in the top/exact bucket).
- [ ] Given a work item `id: "0042"`, when I search `42`, then it appears
      (interior substring match against the zero-padded id).
- [ ] Given a query that matches no id, title, slug, or body, then the panel
      still shows "No matches" (no false positives).
- [ ] All existing `/api/search` ranking and exclusion tests continue to pass
      (no regression to bucket ordering or template exclusion).

## Open Questions

- None outstanding — scope confirmed (local work item `id` only).
  `external_id`/tracker-key matching, tags, type-scoping, and a results view are
  deferred to a future, currently-untracked advanced-search work item.

## Dependencies

- Relates to: 0054 (origin of the search endpoint and the non-functional
  design decision this fixes).

## Assumptions

- "Work item number" means the `id` frontmatter field (surfaced by the indexer
  as `work_item_id`), matched as a string with substring tolerance rather than
  numeric equality.

## Technical Notes

- Root cause is in `classify()` at
  [`skills/visualisation/visualise/server/src/api/search.rs`](../../skills/visualisation/visualise/server/src/api/search.rs)
  — it inspects `title`, `slug`, and `body_preview` only.
- `IndexEntry.work_item_id` is already populated on the entry from the
  frontmatter `id:` key (see `indexer.rs`), so the fix is additive to the
  existing bucketing — no struct or indexer change needed.
- Two implementation options for the plan to weigh: (a) extend `classify()` to
  consult `work_item_id`; (b) change work-item slug derivation to include the
  bare ID so the exact-slug bucket works as 0054 intended. Option (a) is
  preferred — explicit and does not alter slugs used elsewhere.
- `external_id` was considered and explicitly dropped from this work item: it is
  not currently an indexed field (it lives only in the frontmatter blob), so
  matching it needs new plumbing that belongs with the broader advanced-search
  effort, not this bug fix.

## Drafting Notes

- Classified as a bug (not a feature) because it is a gap against 0054's
  documented intent.
- Confirmed with the requester: match scope is `work_item_id` only.
  `external_id`/tracker-key matching was dropped because it is not an indexed
  field and would require new plumbing; tags and broader "advanced search"
  (type-scoping, dedicated results view, sort, faceting) are out of scope. All
  are deferred to a future, currently-untracked advanced-search work item.

## References

- Related: 0054 (`meta/work/0054-sidebar-search.md`)
- Advanced-search enhancement (type-scoping / results view / sort / tag search)
  is currently untracked — a separate work item is being created to capture it.
