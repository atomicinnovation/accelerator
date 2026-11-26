---
type: "work-item"
id: "PP-1004"
external_id: "PP-1004"
aliases: ["draft-wc7by3"]
title: "Activity Rows Carry Their Library Address"
date: "2026-11-24T21:22:40+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "medium"
blocked_by: ["work-item:PP-996"]
relates_to: ["work-item:PP-994", "work-item:PP-998"]
tags: []
last_updated: "2026-11-24T21:22:40+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# PP-1004: Activity Rows Carry Their Library Address

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The sidebar activity feed links each row by joining it to its doc type's full
listing, because an activity row carries only a path. The server should put
each row's published document id on `ActivityEvent` and the `/api/activity`
rows, so the feed links a row without fetching any listing.

## Context

PP-996 (PR #265) links every library document by its id. The activity feed's
`useActivityRowDocuments` finds each row's entry by subscribing to
`fetchDocs(type)` for every doc type among its 5 rows. Those listings carry
frontmatter and body previews. The sidebar is mounted on every page, so:

- every `doc-changed` event refetches that type's whole listing, from any page;
- a burst of agent writes refetches once per event, and an SSE reconnect
  refetches every listing;
- whether a row is linked depends on when `dispatchSseEvent` invalidates the
  listing, which leaves a stale-link window after a delete.

The server decides the published id: `publish_document_ids` in `indexer.rs`
publishes a frontmatter `id` only while it is unique within its type, and
otherwise uses a nested manifest's directory name. So the id an event carries
is the one published when the event was emitted, and an edit to one document
can change another document's published id.

## Requirements

- `ActivityEvent` and each `/api/activity` row carry the document's published
  id (`null` when it has none), taken when the event is emitted.
- The SSE `doc-changed` payload carries the same id, so live rows and history
  rows have one shape.
- The activity feed builds each row's link with `libraryLink` from the row
  alone, and no longer fetches doc listings.
- A deleted document's row stays unlinked.

**Out of scope**: rolling set-member changes up into set rows (PP-994); moving
the rest of the address rule (the stem fallback) to the server.

## Acceptance Criteria

- [ ] Given a research set with id `a`, when it is edited, then its
      `/api/activity` row and its live `doc-changed` event both carry `id: "a"`.
- [ ] Given two inventories sharing a frontmatter `id`, when one is edited,
      then its event carries its directory name as the id.
- [ ] Given the sidebar is mounted, when any `doc-changed` event arrives,
      then no `GET /api/docs` request is made on behalf of the activity feed.
- [ ] Given a row for a deleted document, when the feed renders, then the row
      is unlinked.
- [ ] Given a frontend talking to a server that predates the field, when
      activity rows arrive without `id`, then rows still render, linked by
      their filename stem.

## Open Questions

- Should a row carry a ready-made library address instead of the id, which
  would move the stem fallback to the server as the PR #265 review suggested?

## Dependencies

- Blocked by: PP-996 (PR #265), which introduces published ids.
- Related: PP-994, which also reshapes activity rows for sets.

## Assumptions

- A history row whose document's published id later changes keeps its
  emit-time id. Its link can go stale until the row ages out of the 5-row
  feed, and that is acceptable.

## Technical Notes

- `activity_feed.rs` `ActivityEvent::from_payload` projects from `SsePayload`,
  so the id belongs on `SsePayload::DocChanged` first.
- `watcher.rs` `payload_for_entry` already holds the `IndexEntry` and can read
  `entry.id`.
- `use-activity-row-documents.ts` would be deleted.

## Drafting Notes

- Took "task" over "story": the change is internal and has no new user-facing
  behaviour beyond faster, more reliable links.
- Kept the address rule's stem fallback in the frontend for now. That is the
  smaller step, and the open question asks whether to go further.

## References

- PR #265 review thread on `use-activity-row-documents.ts:21`
- Review: `meta/reviews/prs/265-review-1.md`
- Related: PP-994, PP-996, PP-998
