---
work_item_id: "0039"
title: "Toaster and External-Edit Notifications"
date: "2026-05-06T14:04:04+00:00"
author: Toby Clemson
kind: story
status: done
priority: medium
parent: ""
tags: [design, frontend, components, notifications]
---

# 0039: Toaster and External-Edit Notifications

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Implement a Toaster ephemeral notification component mounted at the root layout, plus a Server-Sent Events (SSE) driven external-edit notification that surfaces when the document the user is currently viewing is changed by a write that did not originate from their own browser session (i.e. an "external" change — see Context for the precise definition).

## Context

The current app has no toast / notification component. The prototype includes a `Toaster` (`.ac-toaster`) ephemeral notification dialog with icon, heading, message, and close button.

The first-class consumer is the external-edit toast — demonstrated on initial load of the prototype with the message "External edit detected · A reviewer agent updated `WORK-0007` while you were looking at it. Query invalidated."

The buildable behaviour differs from that prototype copy in two ways, both grounded in the current SSE infrastructure (`useDocEvents` / `SseEvent` in `frontend/src/api/`):

- **No actor identity.** The `SseEvent` payload carries no field identifying who made a change. The only available distinction is the existing self-cause registry (`frontend/src/api/self-cause.ts`), which matches event `etag`s against the client's own recent writes. An **external change** is therefore defined as any `doc-changed` event whose `etag` is *not* in the registry — i.e. a write that did not originate from this browser session. This covers a reviewer agent, a background process, another user, or even the same user in a different tab; the system can only tell "not me" and cannot attribute the change to a specific actor. The toast therefore uses generic wording with no actor.
- **No document ID.** SSE events identify the affected document by `path` (relative file path). The active document is identified by TanStack Router params (`type` + `fileSlug`) resolved to a `relPath` via the docs list. Correlation is therefore `event.path === active relPath`, and the toast shows the relative path until a stable per-document ID exists (see Drafting Notes).

Reference screenshot: `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png` (toast visible on initial load).

## Requirements

- Implement a `Toaster` ephemeral notification component with icon, heading, message, and close button. Mount it at the root layout (`RootLayout`). The external-edit toast renders a single consistent notification glyph of the implementer's choosing (no specific icon is mandated; the same glyph is used for every external-edit toast).
- Toaster supports being triggered programmatically via a `useToast()` hook (or equivalent context-backed dispatcher) that accepts a heading and message, and auto-dismisses after 5 seconds, while also being manually dismissible via the close button.
- Implement the external-edit subscriber: subscribe to SSE `doc-changed` events; for any action (`created`, `edited`, `deleted`), compare the event's `path` against the active document route's resolved `relPath`. If they match and the event is not self-caused (not present in the self-cause registry), trigger a Toaster.
- The toast message is actor-generic but action-specific (generic with respect to who made the change, specific with respect to the action verb), naming the affected relative path and no actor — heading "External edit detected", message "`{relPath}` was {verb} while you were looking at it." The action-to-verb mapping is normative: `created`→"created", `edited`→"updated", `deleted`→"deleted". Do not include implementation detail such as "Query invalidated" in the copy.
- React Query cache invalidation for the affected document already occurs in `dispatchSseEvent`; the external-edit feature must ride on top of that, so when the toast fires the displayed document content updates to the new version without a page reload.

## Acceptance Criteria

- [ ] Given a caller invokes the toast dispatcher (`useToast()` or equivalent) with a heading and message, when it is called, then a Toaster renders with that heading and message.
- [ ] Given a Toaster is triggered, when it appears, then an icon, heading, message, and close button are rendered.
- [ ] Given the user clicks the Toaster close button, when the click fires, then the Toaster disappears.
- [ ] Given a Toaster has been shown and not dismissed, when time elapses, then it remains visible at 4 seconds and is removed by 5.5 seconds (5-second timer, ±0.5s tolerance).
- [ ] Given the user is viewing the document at relative path `X` and a non-self-caused SSE `doc-changed` event reports a change to `X`, when the event arrives, then a Toaster appears with heading "External edit detected", a message naming `X`, and the existing React Query invalidation for `X` occurs. The message verb matches the action per the normative mapping, and each action must be verified independently: `created`→"`X` was created while you were looking at it.", `edited`→"`X` was updated while you were looking at it.", `deleted`→"`X` was deleted while you were looking at it."
- [ ] Given the toast fires for a non-self-caused change to `X`, when the React Query invalidation completes, then the rendered document body reflects the post-change content for `X` (assert a known changed value from the new version appears, and stale content is gone) without any full-page navigation or reload.
- [ ] Given the user is viewing the document at relative path `X` and an SSE event reports a change to a different path `Y`, when the event arrives, then no Toaster appears.
- [ ] Given the user is viewing the document at relative path `X` and the change to `X` was caused by this client's own write (its `etag` is in the self-cause registry), when the event arrives, then no Toaster appears.

## Open Questions

- None outstanding. (Auto-dismiss timeout, actor wording, and event-type scope were resolved during refinement — see Drafting Notes.)

## Dependencies

- Blocked by: 0033 (token system).
- Blocks: none.
- Followed by: a separate work item (pending creation) to switch the toast and its correlation from relative path to a stable per-document ID once doc types gain one (see Drafting Notes).
- Note: the referenced gap analysis sequences net-new notification features after the Topbar/Sidebar chrome work. This story instead mounts the Toaster as a `RootLayout` portal, so it does **not** depend on that chrome work and the gap-doc sequencing does not apply here.

## Assumptions

- "External" change is defined as any `doc-changed` event whose `etag` is not in the self-cause registry; there is no finer actor attribution available.
- React Query is the active data-fetching layer (confirmed in `frontend/src/api/query-client.ts`), and SSE is the authoritative invalidator (`staleTime: Infinity`).
- Active-document correlation depends on the docs-list query being loaded so the route's `type` + `fileSlug` can be resolved to `entry.relPath`, and on that `relPath` using the same path format as the SSE `event.path`. If the docs list is unloaded or stale, or the two path formats diverge, the `event.path === relPath` comparison fails silently and no toast fires.

## Technical Notes

- Toaster is conventionally a portal mounted at the root. `RootLayout` (`frontend/src/components/RootLayout/RootLayout.tsx`) is the mount point and already provides `DocEventsContext` and consumes `useDocEvents`.
- SSE event shape: `SseEvent` discriminated union in `frontend/src/api/types.ts` (`doc-changed` with `action`/`docType`/`path`/`etag`/`timestamp`, plus `doc-invalid` and `template-changed`).
- Subscription paths in `frontend/src/api/use-doc-events.ts`: `DocEventsHandle.subscribe()` fires for all events (including self-caused); the `onEvent` option fires only after the self-cause drop. The external-edit subscriber should consult the self-cause registry (`frontend/src/api/self-cause.ts`) to exclude the user's own writes.
- Active-document correlation: read `type` + `fileSlug` via `useParams({ strict: false })` (pattern in `frontend/src/routes/library/LibraryDocView.tsx`) and resolve to `entry.relPath` via the docs list; compare against `event.path`.
- Query keys / invalidation: `query-keys.ts` (`docContent(relPath)`, etc.); `dispatchSseEvent` in `use-doc-events.ts` already invalidates `docContent(event.path)` and related list keys.

## Drafting Notes

- Treated Toaster + external-edit as one story because external-edit is the only described consumer of Toaster; splitting them buys nothing.
- Message wording is generic with no actor, because the SSE payload carries no actor identity (only self-cause/etag matching is available). Confirmed with stakeholder.
- "Query invalidated" was dropped from the user-facing copy as an implementation detail, per stakeholder direction.
- The toast shows the relative file path as the document identifier because no stable per-document ID exists yet. **Future work:** switch the toast (and related correlation) to a real document ID once doc types gain one — add/track this as a separate work item.
- The toast fires on all action types (created / edited / deleted) for the active document, per stakeholder direction.
- Auto-dismiss timeout set to 5 seconds, per stakeholder direction.

## References

- Source: `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Screenshots: `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`
- Related: 0033
