---
type: note
id: "2026-05-26-toast-correlation-should-use-document-id"
title: "Future: switch external-edit correlation/display from relPath to a document ID"
date: "2026-05-26T00:00:00+00:00"
author: Toby Clemson
producer: create-note
status: captured
topic: "Future: switch external-edit correlation/display from relPath to a document ID"
tags: []
revision: "5fabe73c39b5"
repository: "ticket-management"
last_updated: "2026-05-26T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
relates_to: ["work-item:0039"]
---

# Future: switch external-edit correlation/display from relPath to a document ID

## Context

Work item `0039` (Toaster and External-Edit Notifications) correlates an
incoming SSE `doc-changed` event against the document the user is currently
viewing, and shows an identifier for the affected document in the toast copy.

Today there is **no stable per-document ID** in the system:

- SSE events identify the affected document by `path` (relative file path) —
  `SseEvent` in `frontend/src/api/types.ts` has no ID field.
- The active document is identified by TanStack Router params (`type` +
  `fileSlug`), resolved to a `relPath` via the docs list
  (`frontend/src/routes/library/LibraryDocView.tsx`).
- React Query keys are keyed on `relPath` (`docContent(relPath)` in
  `frontend/src/api/query-keys.ts`).

So `0039` correlates by `event.path === active relPath` and displays the
relative path in the toast. The prototype mockup showed a work-item ID
(`WORK-0007`), which is closer to the intended UX but isn't buildable yet.

Note: `IndexEntry` (`types.ts`) already carries a `workItemId` field, but
that is per-index-entry and only meaningful for work items — not a general
per-document ID across all doc types.

## Why this is deferred

A document moved/renamed on disk changes its `relPath`, which breaks both
correlation and any displayed identifier. A stable ID would survive renames
and give cleaner, type-appropriate toast copy. But the ID doesn't exist yet,
and adding one is a broader change (doc types, index, SSE payload, query
keys) that shouldn't be coupled into the toast story.

## Path forward (future work)

When doc types gain a stable per-document ID:

1. Plumb the ID through the SSE `doc-changed` payload (or a path→ID lookup
   the client can perform reliably).
2. Switch `0039`'s correlation from `event.path` matching to ID matching.
3. Switch the toast's displayed identifier from `relPath` to the ID
   (restoring the prototype's `WORK-0007`-style copy where applicable).

Track this as its own work item; it depends on the document-ID feature
landing first.

## References

- Work item: `meta/work/0039-toaster-and-external-edit-notifications.md`
  (see Drafting Notes — "Future work: switch the toast ... to a real
  document ID")
- `frontend/src/api/types.ts` (`SseEvent`, `IndexEntry.workItemId`)
- `frontend/src/api/query-keys.ts` (`docContent(relPath)`)
- `frontend/src/routes/library/LibraryDocView.tsx` (route params → relPath)
