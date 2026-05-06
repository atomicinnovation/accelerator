---
work_item_id: "0035"
title: "Topbar Component"
date: "2026-05-06T14:04:04+00:00"
author: Toby Clemson
type: story
status: draft
priority: high
parent: ""
tags: [design, frontend, chrome]
---

# 0035: Topbar Component

**Type**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Introduce a persistent Topbar component as a new top-level chrome element owned by `RootLayout`, surfacing brand wordmark, route-derived breadcrumbs, server-origin pill with green pulse, SSE connectivity indicator, and theme/font-mode toggle slots.

## Context

The current app has no Topbar — `RootLayout` is a two-column shell of sidebar + main scroll area, and SSE / version status is rendered inside the sidebar footer. The prototype introduces a persistent `Topbar` component with brand wordmark, centred breadcrumb crumbs, server-origin pill (`127.0.0.1:NNNNN` plus a green pulse), `SSE` connectivity indicator, and a theme toggle.

The server-origin pill replaces today's text-only sidebar-footer status with a topbar pill that surfaces the live server origin from `useServerInfo` alongside a connectivity-pulse animation.

Reference screenshots: `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`, `main-dark.png` (topbar visible at the top of every page).

## Requirements

- Add a `Topbar` component to `RootLayout`, rendered above the existing sidebar + main column shell.
- Render brand wordmark on the left.
- Render breadcrumbs derived from the active TanStack route in the centre.
- Render the server-origin pill (`127.0.0.1:NNNNN`) on the right, sourced from `useServerInfo`, with a green pulse animation indicating live connectivity.
- Render an `SSE` connectivity indicator sourced from `useDocEventsContext`.
- Render slots for theme-toggle and font-mode-toggle controls (consumed by 0034).
- Remove the sidebar-footer SSE/version status (or relocate appropriately) so chrome state is consolidated in the topbar.

## Acceptance Criteria

- [ ] Given the app is loaded, when `RootLayout` renders, then a Topbar appears above the sidebar + main column shell.
- [ ] Given the user navigates to any TanStack route, when the route changes, then the Topbar breadcrumbs update to reflect the current route segments.
- [ ] Given `useServerInfo` returns a server origin, when the Topbar renders, then the origin pill displays `127.0.0.1:NNNNN` with a green pulse animation.
- [ ] Given `useDocEventsContext` reports SSE connectivity state, when state changes, then the Topbar `SSE` indicator reflects the current connection state.
- [ ] Given Topbar exposes theme/font toggle slots, when 0034 wires its hooks, then both toggles function from within the Topbar.

## Open Questions

- Should the breadcrumbs be derived purely from route segments, or should they include doc-type / document-title context (e.g. `Library › Decisions › 0024`)?
- What should the SSE indicator look like in disconnected vs reconnecting states?

## Dependencies

- Blocked by: 0033 (token system).
- Blocks: 0034 (theme/font toggles need the Topbar slot), 0036 (Sidebar redesign assumes the Topbar exists for chrome separation).

## Assumptions

- The Topbar replaces the sidebar-footer status; the existing footer block is removed once the Topbar is in place.

## Technical Notes

- The `useServerInfo` and `useDocEventsContext` hooks already exist and are currently consumed by the sidebar footer.
- TanStack Router exposes the active route, which is the canonical breadcrumb source.

## Drafting Notes

- The gap analysis describes the server-origin pill as a separate "Net-New Feature" item, but it's a feature *of* the Topbar. Treated as part of the Topbar story because the pill cannot exist without the Topbar component.
- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Screenshots: `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`, `main-dark.png`
- Related: 0033, 0034, 0036
