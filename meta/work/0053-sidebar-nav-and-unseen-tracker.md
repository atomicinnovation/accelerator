---
work_item_id: "0053"
title: "Sidebar Nav with Per-Type Change Indicators"
date: "2026-05-11T12:11:50+00:00"
author: Toby Clemson
type: story
status: ready
priority: high
parent: "0036"
tags: [ design, frontend, chrome, navigation ]
---

# 0053: Sidebar Nav with Per-Type Change Indicators

**Type**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

As a user navigating the visualisation app, I want the Sidebar nav to group doc
types by lifecycle phase with per-item count badges, Glyphs, and an
unseen-changes dot, so I can see at a glance where new activity has happened
since I last visited.

Replace the three flat Sidebar groups (Documents / Views / Meta) with a single
LIBRARY heading partitioned into five lifecycle phases (Define / Discover /
Build / Ship / Remember). Decorate each nav item with a per-doc-type Glyph (a
typed icon component delivered by 0037), an item-count badge driven by a new
`count` field on `GET /api/types`, and an unseen-changes dot driven by a
per-browser tracker. Also slot the empty search-input container and `/` keybind
hint chip so 0054 can drop in.

## Context

Child of 0036 — Sidebar Redesign. 0036 is the bundled epic; this story owns the
foundational nav restructure plus the unseen-changes tracker. The search input
behaviour and the activity feed land in sibling stories (0054, 0055).

The current `Sidebar` (`frontend/src/components/Sidebar/Sidebar.tsx`) partitions
doc types into three flat groups via `mainTypes` / `VIEW_TYPES` / `metaTypes`
and renders plain text links. The redesign moves the twelve concrete doc types
(every `DocTypeKey` variant except Templates) under a single LIBRARY heading
grouped by phase.

## Requirements

This story owns the frontend nav restructure plus one server-side change:
extending `GET /api/types` so each `DocType` payload carries a `count: number`
field. No other backend work is in scope here.

- Replace the three flat groups (Documents / Views / Meta) with a single LIBRARY
  heading whose entries are partitioned by lifecycle phase (Define / Discover /
  Build / Ship / Remember). Phase membership is fixed by the phase-to-doc-type
  table in 0036's Technical Notes.
- Decorate each nav item with a per-doc-type Glyph (a typed icon component
  delivered by 0037) and an item-count badge whose authoritative source is the
  `count` field on the corresponding `DocType` payload from `GET /api/types`.
  The badge is hidden when `count` is `0` or absent.
- Extend `describe_types` (`server/src/docs.rs:101-117`) and the `DocType` JSON
  shape (`server/src/docs.rs:90-99`) so each entry carries `count: usize` equal
  to the indexed entry count for that doc type.
- Implement an unseen-changes tracker: subscribe to SSE `doc-changed` events,
  mark a doc type as "unseen" when an event for that type arrives whose client
  receipt time is later than the per-doc-type T stored in `localStorage` under
  key `ac-seen-doc-types` (shape: `Record<DocTypeKey, ISO-8601 string>`), where
  T is defined as the wall-clock time at which `LibraryTypeView` last mounted
  for that doc type. On first SSE event for a doc type with no stored T, seed T
  to the current time and do not show the dot. On `LibraryTypeView` mount, bump
  T to the current time and clear any dot. Persistence is per-browser.
- Slot a (non-functional) search input container and a `/` keybind hint chip
  into the Sidebar so 0054 can wire behaviour into them without re-laying-out
  the chrome.
- Glyph rendering inside the nav items is consumed by this story but the
  component itself is delivered by 0037.

## Acceptance Criteria

- [ ] Given the user opens the app, when the Sidebar renders, then doc-type
  entries are grouped under LIBRARY by lifecycle phase (Define / Discover /
  Build / Ship / Remember), each doc type appears under the phase specified in
  0036's phase-to-doc-type table, items within each phase appear in the display
  order specified in that table, and `Templates` does not appear in LIBRARY.
- [ ] Given the corresponding `DocType` entry returned by `GET /api/types`
  carries `count: N`, when the Sidebar renders the nav item for that doc type,
  then it displays a count badge whose text equals N as an integer with no
  thousands-separator if N > 0, and renders no badge if N is `0` or if the
  `count` field is absent from the response.
- [ ] Given the user opened doc-type X's list view at time T (stored in
  `localStorage[ac-seen-doc-types][X]` as an ISO-8601 string), when a
  `doc-changed` event for X is received whose client receipt time is greater
  than T, then the Sidebar nav item for X displays an unseen-changes dot.
- [ ] Given no entry for X exists in `localStorage[ac-seen-doc-types]`, when a
  `doc-changed` event for X is received, then T for X is seeded to the current
  time and no unseen-changes dot is shown for X.
- [ ] Given `LibraryTypeView` for X is mounted (from first render through
  unmount, regardless of whether the AC7 mount-effect bump has yet completed),
  when a `doc-changed` event for X is received, then T for X is bumped to the
  current time and no unseen-changes dot is shown for X.
- [ ] Given the user just cleared the dot for X at time T_new, when a later
  `doc-changed` event for X is received whose client receipt time is greater
  than T_new, then the Sidebar nav item for X displays an unseen-changes dot
  again.
- [ ] Given the user opens doc-type X's list view, when `LibraryTypeView` for X
  has finished its mount effects, then T for X in
  `localStorage[ac-seen-doc-types]` is set to the current time and the Sidebar
  nav item for X displays no unseen-changes dot.
- [ ] Given the Sidebar renders, then a search input container is present in the
  Sidebar and a `/` keybind hint chip is rendered as the search input's
  end-adornment within the same row (behaviour is not in scope for this story —
  0054 wires it).
- [ ] Given the Sidebar renders, then every LIBRARY nav item displays a Glyph
  from the 0037 component receiving the item's `DocTypeKey` as its `type` prop.

## Open Questions

- None at story scope. Search ranking, activity-feed pagination, and
  activity-row click target remain open at the parent (0036) level and are
  inherited by 0054 / 0055.

## Dependencies

- Blocked by: 0033 (token system), 0037 (Glyph component).
- Blocks: 0054 (Sidebar search wires into the input slot this story creates),
  0055 (Sidebar activity feed renders inside the Sidebar this story
  restructures).

## Assumptions

- Unseen-changes state is persisted per-browser via `localStorage`; no
  server-side per-user persistence is required. This follows the existing
  `use-theme` / `use-font-mode` precedent.
- The SSE `doc-changed` event payload as it exists today (no `action`
  discriminator, no server `timestamp`) is sufficient for the unseen tracker —
  the tracker uses client receipt time, not a server timestamp, when comparing
  against T. The `action` discriminator and server `timestamp` field are added
  by 0055 for the Activity feed; neither is consumed here. 0055 may later
  promote the comparison to a server timestamp without breaking the AC, since "
  client receipt time greater than T" generalises.
- **Clock source**: both "client receipt time" and "current time" denote
  `Date.now()` (UTC milliseconds since epoch), evaluated synchronously — the
  former inside the SSE `onmessage` handler for the event being processed, the
  latter at the call site that writes T. Stored ISO-8601 strings are produced
  via `new Date(Date.now()).toISOString()` and compared by re-parsing to
  milliseconds.
- **Comparison semantics**: the dot rule uses strict greater-than. If a
  `doc-changed` event's client receipt time equals the stored T (same
  `Date.now()` reading), no dot is shown — the event is treated as already seen.
- The search input container is delivered inert as a chrome-layout scaffold for
  0054. If 0054 slips, the container ships as visually-present but
  non-functional UI; this is the accepted cost of avoiding a second layout pass
  on the Sidebar.

## Technical Notes

- **Phase-to-doc-type mapping** — see 0036 Technical Notes for the canonical
  table. Implement as a frontend-only constant in
  `skills/visualisation/visualise/frontend/src/api/types.ts` alongside
  `LIFECYCLE_PIPELINE_STEPS` (lines 139-169).
- **Sidebar component**:
  `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx`
  is purely presentational — receives `docTypes: DocType[]` as a prop and
  renders TanStack Router `<Link>` items with active state computed via
  `useRouterState`. Today's three-section partition uses the server-supplied
  `virtual` flag (`Sidebar.tsx:20-21` — inline filters, not named module-level
  constants) plus a hard-coded `VIEW_TYPES` array (`Sidebar.tsx:5-8`). The fetch
  lives upstream in `RootLayout.tsx:17-20` via TanStack
  `useQuery({ queryKey: queryKeys.types(), queryFn: fetchTypes })`. Replace the
  three-partition rendering with a five-phase grouping; the `virtual` flag stops
  being relevant for nav membership (Templates is excluded by phase-table
  omission, not by `virtual: true` filter).
- **`GET /api/types` extension**: extend the `DocType` JSON shape in
  `skills/visualisation/visualise/server/src/docs.rs:90-99` to carry
  `count: usize` (camelCased to `count` on the wire under the container-level
  `#[serde(rename_all = "camelCase")]`). The current `describe_types`
  (`docs.rs:101-117`) returns only static schema and does not see the index —
  wire the per-type count in at the handler boundary (`server/src/api/types.rs`)
  by joining `describe_types` output with an entries-per-type lookup against the
  index (`server/src/indexer.rs`). Add the matching `count: number` field to the
  frontend `DocType` interface in `api/types.ts:26-36`.
- **Unseen tracker persistence pattern**: follow `use-theme.ts` (factory +
  owning hook + Context + consumer hook, see `use-theme.ts:26-78`) — owning hook
  in `RootLayout`, Context + Provider wrapping children, consumer hook in
  `Sidebar` (read) and `LibraryTypeView` (write). Mirrors `use-font-mode.ts` for
  the same shape. New storage key `'ac-seen-doc-types'` added to
  `skills/visualisation/visualise/frontend/src/api/storage-keys.ts`; values
  written via `safeSetItem` from `api/safe-storage.ts` (no need to thread
  through `BOOT_SCRIPT_SOURCE` since unseen state has no pre-paint render path).
- **SSE delivery mechanism — design implication**: `use-doc-events.ts` does not
  expose an event-subscription API to consumers — `dispatchSseEvent`
  (`use-doc-events.ts:48-78`) maps each `SseEvent` to
  `queryClient.invalidateQueries` calls and consumers re-fetch. The
  unseen-tracker therefore cannot "subscribe to `doc-changed` events" through
  the existing handle. Two viable paths: (a) extend `use-doc-events.ts` so the
  owning hook also pushes events to a per-doc-type seen-tracker callback
  alongside cache invalidation; (b) add a second `EventSource` consumer
  dedicated to the tracker. Path (a) is preferred — it reuses the existing
  reconnecting `EventSource` plumbing and the `SelfCauseRegistry`. The factory
  shape (`makeUseDocEvents`, `use-doc-events.ts:86-160`) already supports test
  injection.
- **Clear-on-visit hook point**: `routes/library/LibraryTypeView.tsx:46-54`
  narrows `params.type` (or `propType`) to `DocTypeKey | undefined` via
  `isDocTypeKey`. Insert `useMarkSeen(type)` after narrowing (line 54), called
  unconditionally with the same `enabled`-style gating idiom as the adjacent
  `useQuery` (`LibraryTypeView.tsx:59-63`) — the hook is a no-op when
  `type === undefined`.
- **Vestigial test mocks**: `Sidebar.test.tsx:7-21` mocks `useServerInfo`,
  `useDocEventsContext`, `useOrigin` — none are called by the current
  `Sidebar.tsx`. Repurpose the `useDocEventsContext` / `useDocEvents` mock for
  the new unseen-tracker integration; drop the other two.
- **Server/frontend rollout ordering for `count`**: the `count` field on
  `GET /api/types` is a frontend dependency for the badge. Ship the server
  change first, or rely on AC2's "absent `count` → no badge" path to allow
  either order. No feature-flag is needed.
- **Shared frontend primitives potentially consumed by siblings**: this story
  introduces `useMarkSeen(type)`, a `PHASE_DOC_TYPES` constant in
  `api/types.ts`, and the `ac-seen-doc-types` storage key. 0054 and 0055 do not
  currently need any of these; if a sibling later needs them, surface the
  dependency in that story rather than pre-exporting here.

## Drafting Notes

- Extracted from 0036 (Sidebar Redesign) as part of decomposing that bundled
  story into three deliverable units. See 0036 for the full design rationale,
  reference screenshots, and parent-level Technical Notes.

## References

- Parent: `meta/work/0036-sidebar-redesign.md`
- Source:
  `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Screenshots:
  `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`,
  `main-dark.png`, `library-view.png`
- Related: 0033, 0037, 0054, 0055
