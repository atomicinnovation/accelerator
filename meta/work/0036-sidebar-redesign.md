---
work_item_id: "0036"
title: "Sidebar Redesign"
date: "2026-05-06T14:04:04+00:00"
author: Toby Clemson
type: epic
status: draft
priority: high
parent: ""
tags: [ design, frontend, chrome, navigation ]
---

# 0036: Sidebar Redesign

**Type**: Epic
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Redesign the Sidebar to partition the twelve LIBRARY doc types by lifecycle
phase under a single LIBRARY heading, decorate each item with a per-doc-type
Glyph, item-count badge, and unseen-changes dot, add a persistent search input
wired to a new `/api/search` endpoint, and add an Activity feed block subscribed
to SSE with a new `action` discriminator and seeded from a new `/api/activity`
history endpoint.

## Context

The current `Sidebar` partitions doc types into three flat groups (Documents /
Views / Meta) and renders the navigation as plain text links. There is no search
input, no activity-feed block, and no per-doc-type change indicator.
(SSE/version status has already been relocated to the Topbar — `SseIndicator` is
mounted in `Topbar.tsx:17` and `Sidebar.test.tsx:52-56` asserts the version
label is absent — so no further footer-removal work is required.)

The prototype's `Nav` partitions doc types by lifecycle phase under a single
LIBRARY heading, decorates each item with a Glyph + count badge + unseen-changes
dot, hosts a search input with `/` keybind activator, and renders an Activity
feed of recent file-change events with a LIVE badge. This epic delivers that
redesign across three child stories.

The LIBRARY nav covers the twelve concrete doc types — every `DocTypeKey`
variant except `Templates`, which is the only `is_virtual: true` type
(`server/src/docs.rs:6-20`) and remains accessible via its existing
`/library/templates` route.

Reference screenshots:
`meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`,
`main-dark.png`, `library-view.png` (sidebar visible across all main views).

## Goals

This epic owns the end-to-end Sidebar redesign and every server-side endpoint
required to satisfy it. Backend changes in scope across the children: extending
`SsePayload` with an `action` discriminator, a new `GET /api/activity?limit=N`
history endpoint backed by an in-memory ring buffer, a new
`GET /api/search?q=<query>` endpoint, and extending `GET /api/types` with a
per-type `count` field.

### Child work items

- 0053 — Sidebar Nav with Per-Type Change Indicators — nav restructure into five
  lifecycle phases, count badges, Glyph slots, unseen-changes dot tracker,
  `GET /api/types` `count` extension. Lands the foundational Sidebar layout that
  0054 and 0055 build into.
- 0054 — Sidebar Search Input and API Search Endpoint — new
  `GET /api/search?q=<query>` server route + Sidebar search input behaviour (`/`
  keybind with suppression, debounce, inline results). Wires into the search
  input slot delivered by 0053.
- 0055 — Sidebar Activity Feed and SSE Action Discriminator —
  `SsePayload::DocChanged` extended with `action` + `timestamp`; new
  `GET /api/activity?limit=N` route backed by an in-memory ring buffer;
  ActivityFeed component with LIVE badge, relative-timestamp ticker,
  initial-history fetch.

0053 is the foundational story; 0054 and 0055 each depend on it and can ship in
parallel after 0053 merges.

## Acceptance Criteria

The epic is accepted when all three child stories are accepted. Capability-level
acceptance criteria live in 0053, 0054, and 0055.

## Cross-cutting Decisions

The decisions below span all three child stories and are recorded here as the
canonical reference.

### Phase-to-doc-type mapping

Referenced by 0053's AC1; order within each phase is the intended display order
in the Sidebar.

| Phase    | Purpose                                                                 | Doc types (in display order)            |
|----------|-------------------------------------------------------------------------|-----------------------------------------|
| Define   | Backlog management — pre-anything to do with designs or the codebase.   | WorkItems, WorkItemReviews              |
| Discover | Gathering context for the work — the system as-is and the gap to close. | DesignInventories, DesignGaps, Research |
| Build    | Going from work item to implementation — plan and validate.             | Plans, PlanReviews, Validations         |
| Ship     | Getting an implementation deployed.                                     | Prs, PrReviews                          |
| Remember | Context that exists outside the delivery pipeline.                      | Decisions, Notes                        |

Twelve doc types total; Templates is excluded (accessed separately via
`/library/templates`). Implement as a frontend-only constant alongside
`LIFECYCLE_PIPELINE_STEPS` in `frontend/src/api/types.ts`.

### Action-verb canonical set

The Activity feed and the `SsePayload::DocChanged.action` discriminator share a
single canonical verb set: `created` | `edited` | `moved` | `deleted`. The
watcher (`server/src/watcher.rs:139-150`) populates the discriminator from FS
event kind; deletions correspond to `etag: None` post-rescan.

### Persistence strategy for the unseen-changes tracker

Per-browser via `localStorage` under key `ac-seen-doc-types` with shape
`Record<DocTypeKey, ISO-8601 string>`. No server-side per-user persistence.
Follows the existing `use-theme` / `use-font-mode` precedent.

## Open Questions

These remain open at the epic level; the responsible child story is named for
each.

- (0054) What ranking/matching strategy should `/api/search` use across doc
  types (substring, fuzzy, full-text rank, recency boost), and does any one
  type — e.g. work items by ID — need an exact-match shortcut?
- (0055) Should clicking an Activity feed row navigate to the affected document,
  or only highlight it in the nav?
- (0055) Should the Activity feed support scroll/expand to history beyond the
  initial five events, or remain a rolling-five view?

## Dependencies

- Blocked by: 0033 (token system), 0037 (Glyph component consumed by all three
  children). 0035 is not a blocker — its sidebar-footer-status migration is
  already complete (`SseIndicator` mounted in `Topbar.tsx:17`; version-label
  absence asserted by `Sidebar.test.tsx:52-56`).
- Blocks: none currently known. The new `GET /api/search`,
  `GET /api/activity?limit=N`, `SsePayload.action` discriminator, and
  `GET /api/types` `count` field introduced by the children are reusable
  primitives; add downstream consumers to the relevant child story as they
  emerge.

## Assumptions

- The new backend endpoints (`/api/search`, `/api/activity`) and the
  `SsePayload` / `GET /api/types` extensions are delivered within this epic's
  child stories, not coordinated as parallel backend work.
- The phase-to-doc-type mapping above is the author's proposed allocation;
  confirm before promoting child stories from `draft` to `ready`.

## Drafting Notes

- Originally drafted as a single bundled story; refined and reviewed via
  `/refine-work-item` then `/review-work-item`. The review found scope to be the
  dominant concern (4 major dependency findings, 2 major scope findings; full
  record in the review artefact). Decomposed into 0053 / 0054 / 0055 along the
  natural seams (nav + unseen / search / activity feed) and promoted from
  `type: story` to `type: epic`.
- Detailed Requirements, capability-level Acceptance Criteria, and
  child-specific Technical Notes have moved to the child stories; this epic body
  retains only cross-cutting decisions, the epic-level dependency graph, and
  unresolved questions that span more than one child.

## References

- Source:
  `meta/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Screenshots:
  `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`,
  `main-dark.png`, `library-view.png`
- Children: 0053, 0054, 0055
- Reviews: `meta/reviews/work/0036-sidebar-redesign-review-1.md`
- Related: 0033, 0037
