---
work_item_id: "0039"
title: "Toaster and External-Edit Notifications"
date: "2026-05-06T14:04:04+00:00"
author: Toby Clemson
type: story
status: draft
priority: medium
parent: ""
tags: [design, frontend, components, notifications]
---

# 0039: Toaster and External-Edit Notifications

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Implement a Toaster ephemeral notification component mounted at the root layout, plus an SSE-driven external-edit notification that surfaces when another agent has modified the document the user is currently viewing.

## Context

The current app has no toast / notification component. The prototype includes a `Toaster` (`.ac-toaster`) ephemeral notification dialog with icon, heading, message, and close button.

The first-class consumer is the external-edit toast — demonstrated on initial load of the prototype with the message "External edit detected · A reviewer agent updated `WORK-0007` while you were looking at it. Query invalidated." This subscribes to the existing SSE event stream, correlates incoming change events against the active document route, and surfaces a Toaster notification with the affected ID and a "Query invalidated" message when a match is found.

Reference screenshot: `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png` (toast visible on initial load).

## Requirements

- Implement a `Toaster` ephemeral notification component with icon, heading, message, and close button. Mount it at the root layout.
- Toaster supports being triggered programmatically (e.g. via a `useToast()` hook or a context-backed dispatcher).
- Implement the external-edit subscriber: on every SSE document-change event, compare the affected document ID against the active document route; if they match and the change originated from a non-user actor (e.g. a reviewer agent), trigger a Toaster with the format "External edit detected · {actor} updated `{ID}` while you were looking at it. Query invalidated."
- Invalidate the affected query in the React Query cache so the user sees the latest state without manual reload.

## Acceptance Criteria

- [ ] Given a Toaster is triggered, when it appears, then an icon, heading, message, and close button are rendered.
- [ ] Given the user clicks the Toaster close button, when the click fires, then the Toaster disappears.
- [ ] Toaster auto-dismisses after a configured timeout (TBD — confirm with stakeholders).
- [ ] Given the user is viewing document `WORK-0007` and an SSE event reports that an external agent has updated `WORK-0007`, when the event arrives, then a Toaster appears with the message "External edit detected · {actor} updated `WORK-0007` while you were looking at it. Query invalidated." and the React Query cache for that document is invalidated.
- [ ] Given the user is viewing document `WORK-0007` and an SSE event reports a change to a different document, when the event arrives, then no Toaster appears.

## Open Questions

- What is the auto-dismiss timeout for non-error toasts?
- How does the SSE event payload identify the actor (reviewer agent vs. user)? Is there an explicit field, or is it inferred from absence of a session ID?
- Should the Toaster surface other event types (deletion, creation) in future, or is external-edit the only consumer for now?

## Dependencies

- Blocked by: 0033 (token system).
- Blocks: none.

## Assumptions

- The existing SSE infrastructure (`useDocEventsContext`) already exposes the data needed to identify the affected document and the actor type.
- React Query is the active data-fetching layer (consistent with current visualiser implementation).

## Technical Notes

- The toaster pattern is conventionally implemented as a portal mounted at the root.
- Correlation between SSE events and the active route can be done via TanStack Router's location state.

## Drafting Notes

- Treated Toaster + external-edit as one story because the gap analysis identifies external-edit as the only described consumer of Toaster; splitting them buys nothing.
- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Screenshots: `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`
- Related: 0033
