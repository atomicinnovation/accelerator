---
work_item_id: "0036"
title: "Sidebar Redesign"
date: "2026-05-06T14:04:04+00:00"
author: Toby Clemson
type: story
status: draft
priority: high
parent: ""
tags: [design, frontend, chrome, navigation]
---

# 0036: Sidebar Redesign

**Type**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Redesign the Sidebar to partition doc types by lifecycle phase under a single LIBRARY heading, decorate each item with a per-doc-type Glyph, count badge, and unseen-changes dot, and add a persistent search input plus an activity-feed block subscribed to SSE.

## Context

The current app's `Sidebar` partitions doc types into three flat groups (Documents / Views / Meta) and renders the navigation as plain text links. SSE / version status is rendered inside the sidebar footer; there is no search input and no activity-feed block.

The prototype's `Nav` partitions doc types by lifecycle phase (Define / Build / Ship / Remember) under a single LIBRARY heading. Each nav item is decorated with a per-doc-type `Glyph`, a count badge (e.g. Plans `· 18`), and an unseen-changes dot when that doc type has had file changes since the user's last visit. The sidebar also hosts a "Search meta/…" input with a `/` keyboard-shortcut hint chip and an `Activity` feed (`.ac-activity`) showing the five most recent file-change events as rows with per-doc-type Glyph, "type · action" line, relative timestamp + filename, and a `LIVE` badge in the heading.

Reference screenshots: `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`, `main-dark.png`, `library-view.png` (sidebar visible across all main views).

## Requirements

- Replace the three flat groups (Documents / Views / Meta) with a single LIBRARY heading whose entries are partitioned by lifecycle phase (Define / Build / Ship / Remember). The grouping reflects the research → plan → implement workflow rather than the storage-shape distinction between virtual and concrete doc types.
- Decorate each nav item with a per-doc-type Glyph (delivered by 0037), an item-count badge, and an unseen-changes dot.
- Implement an unseen-changes tracker: subscribe to SSE events, mark each doc type as "unseen" until its list view is opened, and feed the Nav so each item can display the dot.
- Add a persistent search input with a `/` keybind activator. Wire it to a forthcoming `/api/search` endpoint that returns matches across all twelve doc types and surfaces the results inline.
- Add an Activity feed block subscribed to the existing SSE `/api/events` stream, rendering rolling file-change events with per-doc-type Glyph, action verb (created / edited / moved-to-status), relative timestamp + filename line, and a `LIVE` badge in the heading.
- Remove sidebar-footer SSE/version status (relocated to Topbar by 0035).

## Acceptance Criteria

- [ ] Given the user opens the app, when the Sidebar renders, then doc-type entries are grouped under LIBRARY by lifecycle phase (Define / Build / Ship / Remember).
- [ ] Given a doc-type list view contains N documents, when the Sidebar renders, then the corresponding nav item displays a count badge of N.
- [ ] Given an SSE event indicates a doc type has changed since the user's last visit, when the Sidebar renders, then the corresponding nav item shows an unseen-changes dot until the user opens that list view.
- [ ] Given the user presses `/` anywhere in the app, when no other input is focused, then focus moves to the Sidebar search input.
- [ ] Given the user types into the search input, when the query reaches the configured threshold, then `/api/search` is called and matching results are rendered inline beneath the input.
- [ ] Given an SSE file-change event arrives, when the Activity feed receives it, then a new row is prepended showing the per-doc-type Glyph, action verb, relative timestamp + filename, and the heading shows a `LIVE` badge.

## Open Questions

- What is the exact specification of the `/api/search` endpoint (matching strategy, ranking, max results)?
- Should clicking an Activity feed row navigate to the affected document, or only highlight it in the nav?
- How is "unseen since last visit" persisted — `localStorage`, server-side per-user, or session-only?

## Dependencies

- Blocked by: 0033 (token system), 0035 (Topbar — sidebar footer status moves there), 0037 (Glyph component is consumed by Nav, Activity feed).
- Blocks: none.

## Assumptions

- A new `/api/search` backend endpoint will be required and is in scope for this work item (or will be coordinated as a parallel backend story).

## Technical Notes

- The existing SSE infrastructure (`/api/events`) already powers the doc-events context; both the unseen tracker and the Activity feed should subscribe to that same stream rather than opening a new connection.
- `useDocEventsContext` is the existing hook for SSE consumption.

## Drafting Notes

- Treated nav redesign, unseen-changes indicator, search, and activity feed as one cohesive story because they all live in the sidebar rectangle and share the SSE stream + Glyph dependency. If the team prefers narrower-scope stories, the natural split-points are: nav + unseen-dots (one), search (two), activity feed (three).
- The `/api/search` endpoint is described as "forthcoming" in the gap analysis; whether it is delivered as part of this story or as a separate backend story is a scope decision.
- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Screenshots: `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`, `main-dark.png`, `library-view.png`
- Related: 0033, 0035, 0037
