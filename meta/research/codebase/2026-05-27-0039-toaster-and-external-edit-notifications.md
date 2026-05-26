---
date: 2026-05-27T01:25:11+01:00
author: Toby Clemson
git_commit: 0d17a89f376651d2d020d4b326d23fd9b2fc5cc2
branch: (no bookmark / detached)
repository: accelerator
topic: "Toaster and External-Edit Notifications (work item 0039)"
tags: [research, codebase, toaster, notifications, sse, self-cause, react-query, routing, design-tokens]
status: complete
last_updated: 2026-05-27
last_updated_by: Toby Clemson
---

# Research: Toaster and External-Edit Notifications (work item 0039)

**Date**: 2026-05-27T01:25:11+01:00
**Author**: Toby Clemson
**Git Commit**: 0d17a89f376651d2d020d4b326d23fd9b2fc5cc2
**Branch**: (no bookmark / detached)
**Repository**: accelerator

## Research Question

How should work item 0039 — a `Toaster` ephemeral notification component plus an SSE-driven external-edit subscriber — be implemented against the existing visualiser frontend? What infrastructure exists (SSE events, self-cause registry, React Query invalidation, root layout providers, route→relPath resolution), what conventions must new code follow (component structure, context/hook pattern, design tokens, tests), and what historical decisions constrain the design?

All paths below are relative to `skills/visualisation/visualise/frontend/`. The `workspaces/` checkouts are jj workspaces, not source — they are ignored here.

## Summary

Everything 0039 needs already exists; the story is almost entirely a composition/wiring exercise plus one greenfield UI component.

- **SSE plumbing is complete.** A single `EventSource` at `/api/events` fans every event out to multi-consumer `subscribe()` listeners *before* a self-cause drop, then runs the single-consumer `onEvent` and React Query invalidation *after* the drop. `dispatchSseEvent` already invalidates `docContent(event.path)`, so the open document refreshes with no extra work — the toast just rides on top.
- **The "external edit" test is a one-liner**: a `doc-changed` event whose `etag` is not in `defaultSelfCauseRegistry`. Because `subscribe()` fires before the drop, the new subscriber must consult `useSelfCauseRegistry().has(event.etag)` itself.
- **Correlation is `event.path === entry.relPath`.** The active doc's `relPath` is resolved in `LibraryDocView` from route params (`type` + `fileSlug`) via the docs-list query. There is no stable doc ID yet; relPath is the agreed (and explicitly deferred-to-be-replaced) identifier.
- **Mount point is `RootLayout`**, which already owns the SSE handle and a stack of context providers (`Theme`, `FontMode`, `DocEvents`, `UnseenDocTypes`). The Toast provider slots into that stack; the visual portal renders near the layout root (or to `document.body`).
- **A `Toaster` is greenfield** — no modal/dialog/overlay/portal/toast exists. Model it on `SseIndicator` (component trio + `data-*`-driven status + `?raw` CSS token test), `use-theme.ts` / `use-unseen-doc-types.ts` (context + owning-hook + consumer-hook, **no-op default handle, never throw**), and `use-deferred-fetching-hint.ts` (setTimeout auto-dismiss + fake-timer test). There is **no generic notification icon** in `Glyph` — inline a small SVG like `SseIndicator` does.
- **Copy is actor-free and action-specific**: heading "External edit detected", message "`{relPath}` was {verb} while you were looking at it." with the normative map `created`→"created", `edited`→"updated", `deleted`→"deleted". The prototype's actor-laden `WORK-0007` copy is deliberately not buildable (no actor identity, no doc ID).

## Detailed Findings

### SSE event types (`src/api/types.ts`)

- Discriminated union `SseEvent = SseDocChangedEvent | SseDocInvalidEvent | SseTemplateChangedEvent` at `types.ts:156-159`, discriminated on `type`.
- `ActionKind = 'created' | 'edited' | 'deleted'` at `types.ts:121`.
- `SseDocChangedEvent` at `types.ts:123-130`: `{ type: 'doc-changed'; action: ActionKind; docType: DocTypeKey; path: string; etag?: string; timestamp: string }`. **`etag` is optional** — when absent, the event can never be classed as self-caused.
- `SseDocInvalidEvent` (`types.ts:143-147`): no `etag`, no `timestamp`. `SseTemplateChangedEvent` (`types.ts:149-154`): irrelevant to this story.
- All fields camelCase to match the server's serde rename (`types.ts:1-2`).

### Self-cause registry (`src/api/self-cause.ts`)

- Interface (`self-cause.ts:3-7`): `register(etag)`, `has(etag | undefined)`, `reset()`.
- `createSelfCauseRegistry` (`self-cause.ts:15-48`): `ttlMs` default `5_000`, `maxEntries` default `256`, injectable `now`. Backed by an insertion-ordered `Map<etag, ts>`; `pruneExpired()` runs on every `register`/`has`; FIFO eviction of the oldest entry over capacity.
- `has(etag)` (`self-cause.ts:39-43`): returns `false` immediately for `undefined`; **non-consuming** — a match does *not* delete the entry (the "consumed" wording in the dispatch docstring is inaccurate; entries clear only by TTL or eviction). Multiple events with the same etag inside 5s all read as self-caused.
- Singleton + context (`self-cause.ts:50-56`): `defaultSelfCauseRegistry`, `SelfCauseContext`, `useSelfCauseRegistry()`. **`RootLayout` does not provide `SelfCauseContext`**, so all consumers (and `useDocEvents`) share `defaultSelfCauseRegistry` — a subscriber's `has()` will agree with the dispatcher's drop decision. Live example consumer: `use-move-work-item.ts:3,17`.

### SSE event flow and dispatch (`src/api/use-doc-events.ts`)

The load-bearing `onmessage` order (`use-doc-events.ts:202-232`):

1. **`subscribe()` listeners fire first (lines 210-219)** — for *every* event, before any self-cause check; each wrapped in its own try/catch.
2. **Self-cause drop (line 220)**: `if (event.type === 'doc-changed' && registry.has(event.etag)) return`.
3. **`onEvent` (line 221)** — only reached after the drop, so it never sees self-caused `doc-changed` events.
4. **Dispatch (lines 222-228)**: deferred into `pendingRef` while dragging, else `dispatchSseEvent(event, queryClient)`.

This confirms the work item's claim exactly: `subscribe()` = all events (used by `ActivityFeed`); `onEvent` = post-drop, single-consumer (used by `useUnseenDocTypes`).

`dispatchSseEvent` (`use-doc-events.ts:89-126`) for `doc-changed`/`doc-invalid` invalidates a fixed key set including **`queryKeys.docContent(event.path)`** (line 106) — `event.path` equals the `relPath` used by `fetchDocContent`, so the open detail view's body refreshes automatically. Also invalidates `types()`, `docs(docType)`, `lifecycle()`, `lifecycleClusterPrefix()`, `relatedPrefix()` (`refetchType: 'all'`), and conditionally `kanban()` when `docType === 'work-items'`. The optional `registry` param is test-only; production drops self-caused events upstream at line 220.

Public API a subscriber uses:
- `DocEventsHandle` (`use-doc-events.ts:31-45`): `{ setDragInProgress, connectionState, justReconnected, subscribe(listener): () => void }`. `subscribe` (`:155-160`) is `useCallback([])`-stable and returns its own unsubscribe.
- `useDocEventsContext()` (`:256-258`) returns the shared handle. **Do not call a second `useDocEvents()`** — it would open a second EventSource.

**Recommended wiring**: a child component inside the providers calls `useDocEventsContext().subscribe(listener)` in a `useEffect` (returning the unsubscribe), and inside the listener filters with `useSelfCauseRegistry().has(event.etag)`. External edit = `event.type === 'doc-changed' && !registry.has(event.etag)`. (The `onEvent` slot is already taken by `unseen`; reusing it would require fanning out to two handlers.)

### Root layout provider stack & mount point (`src/components/RootLayout/RootLayout.tsx`)

Handles owned once in the body (`:16-23`): `useUnseenDocTypes()` (17), `useDocEvents({ onEvent: unseen.onEvent, onReconnect: unseen.onReconnect })` (18-21), `useTheme()` (22), `useFontMode()` (23).

Provider nesting (`:40-60`), outer→inner: `ThemeContext` → `FontModeContext` → `DocEventsContext value={docEvents}` → `UnseenDocTypesContext value={unseen}` → `<div styles.root>` → `<Topbar/>` + `<div styles.body>` (`<Sidebar/>` + `<main><Outlet/></main>` at line 53).

- **Toast provider**: add a new owning hook call next to the others, and nest `<ToastContext.Provider>` inside `UnseenDocTypesContext.Provider` (after line 43) so both `<Outlet/>` views and chrome can enqueue.
- **Toast portal**: render inside `styles.root` as a sibling of `styles.body` (after line 56's close), or portal to `document.body`. Prototype positions it bottom-right.

### Route → relPath resolution (`src/routes/library/LibraryDocView.tsx`)

- Params (`:37-42`): `useParams({ strict: false }) as { type?; fileSlug? }`, props override params, `type` narrowed via `isDocTypeKey`.
- Docs-list query (`:44-48`): `useQuery({ queryKey: queryKeys.docs(type), queryFn: () => fetchDocs(type!), enabled: type !== undefined })`. There is **no `use-docs` wrapper hook** — the list is queried inline. `fetchDocs` → `GET /api/docs?type=…` → `IndexEntry[]` (`fetch.ts:66-71`).
- Entry match (`:50-52`): `entries.find(e => e.slug === fileSlug || fileSlugFromRelPath(e.relPath) === fileSlug)`; `fileSlugFromRelPath` from `../../api/path-utils` (last segment, strips `.md`).
- **The viewed relPath is `entry?.relPath`** (`:57`), `undefined` until the docs list loads and matches. This is the value to compare against `event.path`.
- `useDocPageData(entry?.relPath)` (`use-doc-page-data.ts`) composes `useDocContent` (key `docContent(relPath)` → `['doc-content', relPath]`, `query-keys.ts:47`) + `useRelated` (`['related', relPath]`).

### Existing subscriber to model on (`src/api/use-unseen-doc-types.ts`)

Owning hook called once at `RootLayout`; exposes `onEvent`/`onReconnect` callbacks on its handle (does not attach itself). `onEvent` (`:74-97`) filters `if (event.type !== 'doc-changed') return` then `if (!isDocTypeKey(event.docType)) return`. Context plumbing at `:118-131` with a **no-op default handle** (not a throw). The derived consumer `useMarkDocTypeSeen` (`:138-143`) shows the `useEffect` attach/cleanup shape.

### Component / context / token / test conventions

- **Component trio** — model on `SseIndicator/` (`SseIndicator.{tsx,module.css,test.tsx}`): **named export** (house convention), CSS module imported as `styles`, visual state via `data-*` attributes (use `data-status="ok|warn|err"`), decorative SVG `aria-hidden="true"`. The test has a `describe('CSS source assertions')` block importing the CSS via `?raw` and regex-asserting `var(--ac-*)` bindings (`SseIndicator.test.tsx:77-104`) — replicate for `--ac-ok`/`--ac-warn`/`--ac-err`.
- **Context/provider/hook** — contexts live in `src/api/`, not `components/`. Template: `use-theme.ts` (handle interface → owning hook held in `useState`, called once at RootLayout → `createContext` seeded with **no-op default handle** → consumer `useXContext()`). For collection-state + dispatcher (closer to a toast list), `use-unseen-doc-types.ts:63-131` is the better template (`useState` set + `useCallback` immutable mutators). **This codebase never throws "must be used within provider"** — match the no-op default.
- **Icons** — `Glyph` is strictly per-doc-type (12 keys, `Glyph.constants.ts:19-25`); there is **no generic success/info/warn glyph**. Inline a small purpose-built `<svg viewBox="0 0 24 24" stroke="currentColor" aria-hidden="true">` per the `SseIndicator` precedent (icon drawing example: `Glyph/icons/ValidationsIcon.tsx`).
- **Tokens** — status colours `--ac-ok` (#2E8B57), `--ac-warn` (#D98F2E), `--ac-err` in `src/styles/global.css` (light `:root` ~76-101, dark `[data-theme="dark"]` ~333-335, plus `prefers-color-scheme` mirror). For an overlaid card: `--ac-bg-card` and the elevation token `--ac-shadow-lift`. `SseIndicator.module.css:1-23` is the canonical status-colour + `prefers-reduced-motion` example. Per-component token enforcement is the `?raw`+regex test, not the `src/styles/testing/` utilities (those are for the global catalogue).
- **Auto-dismiss timer** — no existing toast/tooltip, but `use-deferred-fetching-hint.ts:16-31` is the canonical `setTimeout`+`useEffect`+`clearTimeout` transient pattern; the `justReconnected` flag in `use-doc-events.ts:176-197` shows the "reset timer on re-trigger" edge case and the `ReturnType<typeof setTimeout>` id typing.
- **Fake-timer test** — `use-deferred-fetching-hint.test.tsx` is a direct template: `beforeEach(vi.useFakeTimers)` / `afterEach(vi.useRealTimers)`, `act(() => vi.advanceTimersByTime(...))`. For AC: assert visible at 4000ms, gone by 5500ms.
- **Route-aware render harness** — `src/test/router-helpers.tsx` `renderWithRouterAt(ui, atUrl)` (memory history + RouterProvider); SSE+router+query prior art in `LibraryDocView.dispatch.test.tsx` and `use-move-work-item.test.tsx`.

## Code References

- `src/api/types.ts:121,123-130,156-159` - `ActionKind`, `SseDocChangedEvent`, `SseEvent` union
- `src/api/self-cause.ts:3-7,15-48,50-56` - registry interface, factory (TTL/eviction, non-consuming `has`), singleton + context
- `src/api/use-doc-events.ts:31-45` - `DocEventsHandle` (`subscribe`)
- `src/api/use-doc-events.ts:89-126` - `dispatchSseEvent` invalidation set (`docContent(event.path)` at :106)
- `src/api/use-doc-events.ts:202-232` - `onmessage` order: subscribe → self-cause drop → onEvent → dispatch
- `src/api/use-doc-events.ts:155-160,245,254-258` - `subscribe`, `useDocEvents`, `DocEventsContext`, `useDocEventsContext`
- `src/components/RootLayout/RootLayout.tsx:16-23,40-60` - owned handles & provider nesting (Outlet at :53)
- `src/routes/library/LibraryDocView.tsx:37-42,44-48,50-52,57` - params → docs query → entry match → `entry.relPath` → `useDocPageData`
- `src/api/use-unseen-doc-types.ts:63-131,138-143` - subscriber + context + consumer-hook pattern
- `src/api/use-doc-content.ts`, `use-doc-page-data.ts`, `use-related.ts` - content/related queries & keys
- `src/api/query-keys.ts:46-47,54` - `docs`, `docContent`, `related`
- `src/api/path-utils.ts:6-8` - `fileSlugFromRelPath`
- `src/api/fetch.ts:66-71,73` - `fetchDocs`, `fetchDocContent`
- `src/components/SseIndicator/SseIndicator.{tsx,module.css,test.tsx}` - component trio + `data-*` state + `?raw` token test
- `src/api/use-theme.ts:8-12,26-78` - context/owning-hook/consumer-hook template (no-op default)
- `src/api/use-deferred-fetching-hint.ts:16-31` + `.test.tsx` - setTimeout auto-dismiss + fake-timer test
- `src/components/Glyph/Glyph.tsx`, `Glyph.constants.ts:19-25`, `Glyph/icons/ValidationsIcon.tsx` - per-doc-type only; inline-SVG conventions
- `src/styles/global.css` (status tokens), `src/test/router-helpers.tsx:32` (`renderWithRouterAt`)

## Architecture Insights

- **Self-cause is the only "actor" signal available.** The SSE payload has no actor and no doc ID. "External" is definitionally "etag not in my registry" — covers reviewer agents, background processes, other users, and the same user in another tab. The toast copy must therefore be actor-generic.
- **The invalidation already happens; the toast is purely additive.** Because `dispatchSseEvent` invalidates `docContent(event.path)` regardless, the "content refreshes without reload" AC is satisfied by existing code — the subscriber must not duplicate or fight that invalidation, just notify.
- **`subscribe()` vs `onEvent` is a deliberate two-tier contract.** `subscribe()` (pre-drop, multi-consumer) is correct for the external-edit toast precisely because it also lets the subscriber *choose* to self-filter; `onEvent` is a taken single slot. This means the toast subscriber owns its self-cause check explicitly — a feature, not a workaround.
- **No-op-default contexts, never throw.** Four existing contexts seed a no-op handle so out-of-provider consumers degrade silently. Toast must follow suit for consistency (and so tests can render leaf components without the provider).
- **relPath is a knowingly fragile correlation key.** A rename/move mid-view breaks both correlation and the displayed identifier. Accepted for 0039; superseded later by a stable doc ID (see note below).

## Historical Context

- `meta/notes/2026-05-26-toast-correlation-should-use-document-id.md` — Deferral note. No stable per-doc ID exists; SSE identifies by `path`, active doc resolved to `relPath`, query keys are relPath-based. 0039 correlates by `event.path === relPath` and displays the path. Known limitation: rename/move mid-view breaks it. Future work (separate, unnumbered item): plumb a doc ID through the SSE payload, switch correlation and displayed identifier to it (restoring `WORK-0007`-style copy).
- `meta/reviews/work/0039-...-review-1.md` — Final **APPROVE** (pass 3). Two Major issues resolved: (1) normative action→verb mapping + per-action AC strings; (2) an AC binding the `useToast()` dispatcher to a render outcome. Baked-in decisions: 5s auto-dismiss, generic actor-free copy, all-action scope, relPath correlation, etag-based self-cause definition, Toaster bundled with ExternalEditToast, mounted at RootLayout. Residual non-blocking items for planning: stable test hook for the icon (test id / role), concrete before/after fixture values for the content-refresh and dispatch ACs, the manual-dismiss-before-auto-dismiss timer race, verifying positive correlation with a real resolved relPath vs real event.path (not two equated literals).
- `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/inventory.md` (`superseded`) — `.ac-toaster`: icon + heading + message + close button, overlaid bottom-right, no animation/sub-classes captured. Prototype demo copy "External edit detected · A reviewer agent updated `WORK-0007` … Query invalidated." is design intent only and explicitly not buildable (no actor, no doc ID, and "Query invalidated" dropped as impl detail).
- `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md` (`draft`) — Identifies the missing Toaster and names `ExternalEditToast` as its first consumer. Sequences net-new features after Topbar/Sidebar chrome; 0039 **deliberately overrides** this by mounting at RootLayout (no chrome dependency).
- `meta/plans/2026-04-22-meta-visualiser-phase-4-sse-hub-and-notify-watcher.md` — The plan that built the SSE hub/notify-watcher this story rides on.
- 0033 design-token system (work item, plan, research, ADR-0026 + ADR-0035) — the token layer `Toaster.module.css` must consume; 0033 is a sibling blocker, not a parent.

## Related Research

- `meta/research/codebase/2026-05-06-0033-design-token-system.md` — token system codebase research
- `meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md` — original SSE/RootLayout implementation context
- `meta/specs/2026-04-17-meta-visualisation-design.md` — original design spec (SSE/EventSource/RootLayout)

## Open Questions

- **Icon test hook.** No glyph mandated; pick one inline SVG and give it a stable hook (`data-testid` or `role`) so the "icon renders" AC is checkable. (Review residual.)
- **Manual-dismiss vs auto-dismiss race.** No AC covers clicking close before the 5s timer; decide at plan time whether the timer is cleared on manual dismiss (it should be, per the `justReconnected` reset pattern).
- **Multiple concurrent toasts.** Story describes a single external-edit toast; `useToast()` is generic. Decide whether the provider holds a `Toast[]` stack or a single current toast. `use-unseen-doc-types` immutable-set pattern supports a stack cleanly.
- **Self-cause timing.** `has()` is non-consuming with a 5s TTL matching the auto-dismiss; confirm no scenario where a legitimately-external second edit with a reused etag within 5s is wrongly suppressed (unlikely — etags are content-derived per write).
- **Positive-correlation test fidelity.** Per the review, assert with a real resolved `relPath` and a real `event.path`, not two equated string literals.
