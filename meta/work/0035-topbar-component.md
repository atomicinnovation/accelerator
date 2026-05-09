---
work_item_id: "0035"
title: "Topbar Component"
date: "2026-05-06T14:04:04+00:00"
author: Toby Clemson
type: story
status: done
priority: high
parent: ""
tags: [design, frontend, chrome]
---

# 0035: Topbar Component

**Type**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

Introduce a persistent Topbar component as a new top-level chrome element owned
by `RootLayout`, surfacing brand wordmark, route-derived breadcrumbs,
server-origin pill with green pulse, SSE connectivity indicator, and
theme/font-mode toggle slots.

## Context

The current app has no Topbar — `RootLayout` is a two-column shell of sidebar +
main scroll area, and SSE / version status is rendered inside the sidebar
footer. The prototype introduces a persistent `Topbar` component with brand
wordmark, centred breadcrumb crumbs, server-origin pill (`127.0.0.1:NNNNN`
plus a green pulse), `SSE` connectivity indicator, and a theme toggle.

The server-origin pill replaces today's text-only sidebar-footer status with a
topbar pill that surfaces the live server origin from `window.location.host` alongside
a connectivity-pulse animation.

Reference screenshots: `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`,
`main-dark.png` (topbar visible at the top of every page).

## Requirements

- Add a `Topbar` component to `RootLayout`, rendered above the existing sidebar
  + main column shell.
- Render the brand mark on the left: an SVG hex logo mark alongside "Accelerator"
  as the primary name and "VISUALISER" as a sub-label. The hex mark uses
  `--ac-accent` and `--ac-accent-2` gradient tokens.
- Render breadcrumbs in the centre layout slot, left-aligned within that slot
  with no gap between the breadcrumb element and the brand mark container (zero
  left margin), derived from the active TanStack
  Router route matches, pulling document titles from loader data where available.
  The root/landing route displays "Library" as its crumb label.
- Add a `loader` function to each TanStack route that should contribute a
  breadcrumb, returning `{ crumb: string, ...otherData }`; the root route's
  loader returns `{ crumb: 'Library' }`.
- Render the server-origin pill (`127.0.0.1:NNNNN`) on the right, derived from
  `window.location.host`, with a green pulse animation indicating live
  connectivity.
- Render an SSE connectivity indicator sourced from `useDocEventsContext` in
  four visual states: `open` (green), `reconnecting` (amber / pulsing),
  `connecting` (grey/muted, no animation), `closed` (red).
- Render structural placeholder slots for theme-toggle and font-mode-toggle
  controls; wiring of the context hooks is 0034's responsibility — this story
  delivers the slot positions only.
- Remove the sidebar-footer SSE/version status block so chrome state is
  consolidated in the Topbar.

## Acceptance Criteria

- [ ] Given the app is loaded, when `RootLayout` renders, then a Topbar appears
  above the sidebar + main column shell, rendering the brand mark, breadcrumb
  area, server-origin pill, and SSE indicator.
- [ ] Given the Topbar renders, then the brand area shows the SVG hex mark
  alongside the "Accelerator" name and "VISUALISER" sub-label, with the mark
  gradient consuming `--ac-accent` / `--ac-accent-2` tokens.
- [ ] Given the Topbar renders, then the breadcrumb area occupies the centre
  layout slot with no gap between it and the brand mark container (zero left
  margin on the breadcrumb element), rather than centred within the slot.
- [ ] Given each TanStack route defines a `loader` returning `{ crumb: string }`
  and the user navigates to that route, when the route changes, then the Topbar
  breadcrumbs update to reflect the active route's `loaderData.crumb` value; on
  the root route, the crumb reads "Library".
- [ ] Given the app is running at a local address, when the Topbar renders,
  then the origin pill displays the value of `window.location.host` (e.g.
  `127.0.0.1:5173`) with a repeating CSS animation visible as a cycling opacity
  or scale change (green).
- [ ] Given `useDocEventsContext` reports a `connectionState` value, when state
  changes, then the SSE indicator renders: green when `open`, amber with a repeating CSS animation when
  `reconnecting`, grey/muted with no animation when `connecting`, and red when `closed`.
- [ ] Given the Topbar renders, then a `data-slot="theme-toggle"` element and
  a `data-slot="font-mode-toggle"` element are present in the Topbar's right
  rail; each slot renders with zero visible content and occupies no visible
  space until populated by 0034.
- [ ] Given the Topbar is in place, then the sidebar-footer SSE/version status
  block is removed from the sidebar.

## Dependencies

- Blocked by: 0033 (token system).
- Blocks: 0034 (theme/font toggles need the Topbar slot), 0036 (Sidebar
  redesign assumes the Topbar exists for chrome separation).
- Related (informational, no ordering constraint): 0039 (Toaster also mounts at
  RootLayout — sibling concern), 0041 (Library page would contribute a crumb
  via the loader work introduced here).

## Assumptions

- The Topbar replaces the sidebar-footer status; the existing footer block is
  removed once the Topbar is in place.
- Toggle slots are structural placeholder positions only; the Topbar story does
  not own theme/font context wiring. If this interpretation is wrong, the scope
  of this story expands to include the React context setup from 0034.

## Technical Notes

- **RootLayout** (`RootLayout.tsx:18–25`): shell is a flex-row `div.shell`
  containing `<Sidebar>` + `<main>`. The `DocEventsContext.Provider` wraps the
  entire shell (line 18), so the `<Topbar>` must be inserted *inside* the
  Provider as a sibling above `div.shell`. `RootLayout.module.css` currently
  defines only `.shell` (flex-row) and `.main` (flex:1); a new outer wrapper
  (e.g. `div.root`) with `flex-direction: column` is needed.
- **Server origin**: `useServerInfo()` (`api/use-server-info.ts:4–23`) returns
  `{ name?, version? }` only — no origin or URL field. Use
  `window.location.host` to derive the server origin pill value (e.g.
  `127.0.0.1:5173`).
- **SSE connectivity**: `useDocEventsContext()` (`api/use-doc-events.ts:173–175`)
  exposes `connectionState: 'connecting' | 'open' | 'reconnecting' | 'closed'`
  and `justReconnected: boolean`. The Topbar indicator must handle all four
  states: `open` → green, `reconnecting` → amber/pulsing, `connecting` →
  grey/muted with no animation, `closed` → red.
- **SidebarFooter removal**: the block to remove is
  `SidebarFooter/SidebarFooter.tsx:1–28` (standalone component) rendered at
  `Sidebar.tsx:82`. Remove the `<SidebarFooter />` call and its import; delete
  the `SidebarFooter/` directory.
- **Breadcrumbs — no loaders exist**: `router.ts` has zero `loader` functions
  across all routes. Adding breadcrumb support requires adding `loader` to each
  route that should contribute a crumb. The idiomatic TanStack Router approach:
  each route's `loader` returns `{ crumb: titleString, ...otherData }`; the
  breadcrumb component filters with `isMatch(match, 'loaderData.crumb')`. Guard
  with `match.status === 'pending'` to hide stale crumbs during transitions.
  This is prerequisite work — the breadcrumb component cannot function until
  loaders are added.
- **Token names**: accent tokens are defined at `styles/global.css:75–78` as
  `--ac-accent` (indigo-purple) and `--ac-accent-2` (red). The brand mark SVG
  must use `var(--ac-accent)` and `var(--ac-accent-2)`, not `var(--accent)` /
  `var(--accent-2)`. Updated SVG:
  ```html
  <svg width="24" height="24" viewBox="0 0 40 40" class="ac-mark">
    <defs>
      <linearGradient id="hexg" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0%" stop-color="var(--ac-accent)" stop-opacity="1"/>
        <stop offset="100%" stop-color="var(--ac-accent-2)" stop-opacity="1"/>
      </linearGradient>
    </defs>
    <path d="M20 2 36 11v18L20 38 4 29V11z"
          fill="none" stroke="url(#hexg)" stroke-width="2"/>
    <circle cx="20" cy="20" r="3" fill="var(--ac-accent)"/>
    <circle cx="20" cy="20" r="7.5" fill="none"
            stroke="var(--ac-accent-2)" stroke-width="1" stroke-opacity="0.5"/>
  </svg>
  ```

## Drafting Notes

- The gap analysis describes the server-origin pill as a separate "Net-New
  Feature" item, but it's a feature *of* the Topbar. Treated as part of the
  Topbar story because the pill cannot exist without the Topbar component.
- Brand mark resolved as SVG hex mark + "Accelerator" + "VISUALISER" sub-label,
  extracted from prototype HTML. SVG markup embedded in Technical Notes as the
  prototype URL is ephemeral.
- Breadcrumbs resolved as loader-data titles where available; root route shows
  "Library" (the default landing page label). Breadcrumbs occupy the centre
  layout slot but are left-aligned within it, flush against the brand mark.
- SSE indicator resolved as four states: `open` (green), `reconnecting`
  (amber/pulsing), `connecting` (grey/muted, no animation), `closed` (red).
- Toggle slots resolved as structural placeholders only; context wiring belongs
  to 0034. If wrong, scope of this story grows.

## References

- Source: `meta/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Screenshots: `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`,
  `main-dark.png`
- Related: 0033, 0034, 0036, 0039, 0041
