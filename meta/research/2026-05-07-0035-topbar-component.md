---
date: 2026-05-07T23:33:16+01:00
researcher: Toby Clemson
git_commit: e5f283acf73ac13bc9806e9d8ab6055aba929dca
branch: main
repository: accelerator
topic: "Implementation of work item 0035 — Topbar Component"
tags: [research, codebase, topbar, root-layout, sse, breadcrumbs, tanstack-router, design-tokens]
status: complete
last_updated: 2026-05-07
last_updated_by: Toby Clemson
---

# Research: Implementation of work item 0035 — Topbar Component

**Date**: 2026-05-07T23:33:16+01:00
**Researcher**: Toby Clemson
**Git Commit**: e5f283acf73ac13bc9806e9d8ab6055aba929dca (jj change `mrklkwwumpywomzqvlmooqlwkwkrtvyr`)
**Branch**: main
**Repository**: accelerator (workspace: `workspaces/visualisation-system`)

## Research Question

What is required, in concrete terms, to implement work item
[`meta/work/0035-topbar-component.md`](../work/0035-topbar-component.md) — a
persistent Topbar above the existing sidebar+main shell that surfaces brand,
breadcrumbs, server-origin pill, SSE state, and theme/font-mode toggle slots —
given the current state of the visualisation frontend?

## Summary

The Topbar is greenfield: there is no existing Topbar, no breadcrumb component,
no `@keyframes` anywhere in the frontend tree, no `data-slot` convention, and no
`loader` on any TanStack Router route. The work touches six concrete areas:

1. **Restructure `RootLayout`** from a flex-row shell into a flex-column with
   `<Topbar>` above an inner flex-row (Sidebar + main). The
   `DocEventsContext.Provider` already wraps the whole shell, so the Topbar can
   read `connectionState` simply by being inside it.
2. **Add a `Topbar` component** at `components/Topbar/` (co-located CSS module +
   tests), following the established convention.
3. **Add a `loader` to every contributing TanStack route** so a breadcrumb
   component can read `match.loaderData.crumb`. Today zero routes have loaders
   and zero components use `useMatches`/`useLoaderData`.
4. **Source the server origin from `window.location.host`** (not
   `useServerInfo` — it returns only `{ name?, version? }`). This is the first
   `window.location.*` call in the frontend.
5. **Author the first CSS keyframes** in the codebase for the green pulse
   (origin pill, `connectionState === 'open'`) and amber pulse
   (`reconnecting`); `connecting` is grey/static and `closed` is red/static.
6. **Delete `components/SidebarFooter/`** (component + module + test) and its
   import/render-site at `Sidebar.tsx:3` and `Sidebar.tsx:82`.

Pre-shipped foundation: 0033 has landed all `--ac-*` design tokens including
the dark-theme override block (inert until 0034 wires `data-theme`). 0034 is
blocked by 0035 and will populate the two `data-slot` placeholders we add.

The 0035 work item itself has been through a pass-2 review with three open
items the author may want to tighten before implementation begins (see
[Review status](#review-status-of-the-work-item) below).

## Detailed Findings

### Current chrome shell — what we are wrapping

`components/RootLayout/RootLayout.tsx:17-26` is a thin shell:

```tsx
<DocEventsContext.Provider value={docEvents}>      // line 18
  <div className={styles.shell}>                   // line 19
    <Sidebar docTypes={docTypes} />                // line 20
    <main className={styles.main}>                 // line 21
      <Outlet />                                   // line 22
    </main>
  </div>
</DocEventsContext.Provider>
```

`components/RootLayout/RootLayout.module.css` has only:

```css
.shell { display: flex; min-height: 100vh; font-family: var(--ac-font-body); }
.main  { flex: 1;     overflow: auto;     padding: var(--sp-5) var(--sp-6); }
```

To insert a Topbar that occupies a row above the sidebar+main row, the
simplest restructure is: wrap the existing shell content in an outer flex
column. Either keep `.shell` as the inner row and add an outer `.root`
(flex-direction: column), or change `.shell` to column and nest a `.body` row.
Either keeps `DocEventsContext.Provider` outside everything and the Topbar
sees its value.

Notable: `RootLayout` has no test file — this is an opportunity to add one
when restructuring.

### `Sidebar` and the SidebarFooter that must be removed

- `components/Sidebar/Sidebar.tsx:3` imports `SidebarFooter`.
- `components/Sidebar/Sidebar.tsx:82` renders `<SidebarFooter />` as the last
  child of `<nav className={styles.sidebar}>`.
- `components/Sidebar/Sidebar.module.css:1-10` sets `.sidebar` to `width:
  220px; min-height: 100vh; display: flex; flex-direction: column; gap:
  var(--sp-5)`. The footer pins to the bottom via `margin-top: auto` on the
  footer itself.
- The whole `components/SidebarFooter/` directory (3 files: `.tsx`,
  `.module.css`, `.test.tsx`) is deleted by 0035.

Today `SidebarFooter.tsx:13-27` renders only labels — there are no
animations or pulses anywhere in its CSS. Visible states (per
`Sidebar.test.tsx`-style check on `SidebarFooter.test.tsx:31-69`):

- `reconnecting` → text "Reconnecting…" coloured `var(--ac-warn)`
- `open` + `justReconnected` → "Reconnected — refreshing" coloured
  `var(--ac-warn)`
- `connecting` and `closed` → render nothing
- `version` → `Visualiser v${serverInfo.version}` when present

The Topbar replaces all four of these visual states with a single
four-state SSE indicator + the version absorbed into the origin pill area
(per work-item AC1 / AC5 / AC6).

### Data sources the Topbar consumes

#### `useDocEventsContext` (`api/use-doc-events.ts`)

Returns `DocEventsHandle` (lines 15-19):

```ts
interface DocEventsHandle {
  setDragInProgress(v: boolean): void
  connectionState: ConnectionState
  justReconnected: boolean
}
```

`ConnectionState = 'connecting' | 'open' | 'reconnecting' | 'closed'` is
defined at `api/reconnecting-event-source.ts:6` and set at:

- `connecting` — initial state, lines 29 and 38
- `open` — `src.onopen`, line 66
- `reconnecting` — `scheduleReconnect()`, line 86
- `closed` — `close()`, line 110 (called by hook cleanup at
  `use-doc-events.ts:154`)

`justReconnected` (line 95, 128-132): goes `true` for exactly **3,000 ms**
after a successful reconnect (a `setTimeout` flips it back). Useful if the
Topbar wants a brief "just reconnected" flash overlay, but the work item
spec doesn't require this and the existing 3-second flash is what
`SidebarFooter` already used.

The context has a non-null default at `use-doc-events.ts:165-169`
(`connectionState: 'connecting'`, no-op handle), so consumers outside the
Provider get a benign default — the Topbar won't throw in tests rendered
without the Provider, which simplifies test fixtures.

#### `useServerInfo` (`api/use-server-info.ts`)

Returns the raw `UseQueryResult<{ name?, version? }>` from React Query.
Fetches `/api/info` once per session (`staleTime: Infinity`). **No origin
field** — the work item correctly insists on `window.location.host`.

`window.location.*` precedent: zero matches in the entire frontend `src/`.
The Topbar will be the first consumer.

### Routing — TanStack Router v1.168.23

Confirmed version via `frontend/package.json:24` (`^1`) and resolved
`1.168.23` via `node_modules/@tanstack/react-router/package.json:3`.

`router.ts` defines the route tree programmatically (no `__root` file; no
file-based routing). `createRootRoute({ component: RootLayout })` at line
19; `rootRoute.addChildren([...])` at lines 105-117; `createRouter({
routeTree })` at line 119.

Every route and its current loader status:

| Route file:line | Path | Component | Has loader? | Crumb candidate |
|---|---|---|---|---|
| `router.ts:21-25` | `/` | redirect → `/library` | no | — |
| `router.ts:27-31` | `/library` | `LibraryLayout` (just `<Outlet/>`) | no | `Library` |
| `router.ts:35-41` | `/library/` index | redirect → `/library/decisions` | no | — |
| `router.ts:47-51` | `/library/templates` | `LibraryTemplatesIndex` | no | `Templates` |
| `router.ts:53-57` | `/library/templates/$name` | `LibraryTemplatesView` | no | template `$name` |
| `router.ts:62-72` | `/library/$type` | `LibraryTypeView` | no | doc-type label |
| `router.ts:74-78` | `/library/$type/$fileSlug` | `LibraryDocView` | no | doc title |
| `router.ts:80-84` | `/lifecycle` | `LifecycleLayout` (just `<Outlet/>`) | no | `Lifecycle` |
| `router.ts:86-90` | `/lifecycle/` index | `LifecycleIndex` | no | — |
| `router.ts:92-96` | `/lifecycle/$slug` | `LifecycleClusterView` | no | cluster title |
| `router.ts:98-102` | `/kanban` | `KanbanBoard` | no | `Kanban` |

Existing route-state access patterns (from grepping `src/`):

- `useRouterState({ select: s => s.location })` — `Sidebar.tsx:16`
- `useParams({ strict: false })` — used in three view components
- `useParams({ from: lifecycleClusterRoute.id })` — `LifecycleClusterView.tsx:138`
- `useMatches`, `useMatch`, `useLoaderData` — **zero usages**
- `crumb`, `breadcrumb`, `Breadcrumb` (case-insensitive) — **zero matches**

This confirms the work item's claim: breadcrumb plumbing is entirely net-new.
The idiomatic TanStack v1 pattern is exactly what the work item describes:

```ts
// per route
loader: () => ({ crumb: 'Library' })
// in the breadcrumb component
const matches = useMatches()
const crumbs = matches.filter(m => isMatch(m, 'loaderData.crumb'))
```

Loaders that compute titles from per-route data (e.g. doc title for
`/library/$type/$fileSlug`) currently rely on React Query — `LibraryDocView`
fetches in-component via the doc-page-data hook. Two options:

- **Static crumbs only** — loaders return literal strings, deep crumbs render
  the slug/$name/$type as fallback labels. Lowest risk, ships with 0035.
- **Loader-fetched titles** — loaders call the same doc-fetch and return
  `{ crumb: title }`. More invasive (loader race vs. existing in-component
  fetch), but the breadcrumb says the doc title, not the slug.

The work item allows either; AC4 is satisfied as long as a loader returns
`{ crumb: string }` and the Topbar reads it.

#### Special case: the Library root crumb and the `/library` redirect

`/library` today is a redirect to `/library/decisions` (`router.ts:35-41`),
so attaching a `loader` returning `{ crumb: 'Library' }` to a redirect
route is awkward. Work item 0041 (Library page wrapper) replaces this
redirect with a real `/library` overview component — until then, the
"Library" crumb is most cleanly produced by attaching the loader to the
`libraryRoute` itself (`router.ts:27-31`, the `LibraryLayout` parent),
which renders `<Outlet/>` and is on every `/library/*` URL.

The 0035 review (pass 2) flagged this: 0041 will consume 0035's
`{ crumb: string }` loader-data shape — the coupling is stronger than
"informational" but doesn't block 0035.

### Design tokens — what to use

Confirmed in `styles/global.css` and mirrored typed in `styles/tokens.ts`
(parity asserted by `styles/global.test.ts`).

**Brand-mark gradient** (`--ac-accent` / `--ac-accent-2`):

- Light `:root` — line 75 (`#595FC8`), line 76 (`#CB4647`)
- Dark `[data-theme="dark"]` — line 154-155 (`#8A90E8`, `#E86A6B`)
- Dark `@media (prefers-color-scheme: dark)` mirror — line 184-185

There are **no `--accent` / `--accent-2` tokens without the `ac-` prefix** —
the work item's warning is correct. Always `var(--ac-accent)`.

**SSE state colours** (already present):
- `--ac-ok` — green for `open`
- `--ac-warn` — amber for `reconnecting`
- `--ac-err` — red for `closed`
- `--ac-fg-muted` or `--ac-fg-faint` — grey for `connecting`

**Typography**:
- `--ac-font-display` ("Sora", system-ui, sans-serif) — for "Accelerator"
- `--ac-font-body` ("Inter", system-ui, sans-serif) — for crumbs and pill
- `--ac-font-mono` ("Fira Code", ui-monospace) — for `127.0.0.1:NNNNN` pill text
- `--tracking-caps: 0.12em` — for the "VISUALISER" sub-label

**Shapes & spacing**:
- `--radius-pill: 999px` — origin pill border-radius
- `--sp-1`…`--sp-11` — spacing scale
- `--ac-stroke`, `--ac-stroke-soft` — borders
- `--ac-bg-chrome` — chrome surface background
- `--ac-shadow-soft` — subtle elevation if Topbar uses bottom shadow

### Animations and `data-slot` placeholders — both net-new

Greps across the entire frontend (`src/**/*.{css,ts,tsx}`):

- `@keyframes` — **zero matches** (no animations exist anywhere)
- `animation:` property — zero matches
- `data-slot` — zero matches

The Topbar will introduce both conventions. Keyframes for the green/amber
pulse must be defined either inside `Topbar.module.css` (CSS-modules scope
keyframes per file via class-name mangling) or in `styles/global.css` if
they should be reusable. The pass-2 review flagged the animation language
as "unverifiable" — the recommended remediation is "repeating CSS animation
visible as cycling opacity or scale change", which the work item's
acceptance criteria already adopt as of the latest revision.

For tests, asserting on the presence of an `animation` style or a CSS class
that maps to a keyframe is the practical bar — visual rendering of a CSS
animation isn't checked in unit tests.

### Review status of the work item

`meta/reviews/work/0035-topbar-component-review-1.md` — pass-2 verdict is
**REVISE**. Five pass-1 majors resolved; two new majors and three minors
remain:

1. 🟡 Animation acceptance criteria need measurable form (cycling
   opacity/scale change) — partially adopted in the latest AC text.
2. 🟡 "Ready for 0034 to populate" phrasing in AC7 is untestable. The
   work-item AC actually requires "zero visible content and occupies no
   visible space until populated by 0034" — measurable via computed styles.
3. 🔵 Requirements vs AC3 wording: "flush against the brand mark" vs "zero
   left margin" — Requirements still has the older phrasing at lines
   199/208 of the review's quoted analysis.
4. 🔵 0041 ↔ 0035 loader-shape coupling stronger than informational.
5. 🔵 Brand-mark gradient AC checks SVG source (implementation), not
   rendered output.

None of these block implementation; they are tightening opportunities.

### Division of responsibility with 0033 / 0034 / 0036 / 0041

| Work item | Status | What it owns vs 0035 |
|---|---|---|
| 0033 (token system) | shipped | All `--ac-*` tokens incl. dark override block (inert until 0034 wires `data-theme`). Three-family typography. Eleven-step spacing. `--radius-pill`. Five elevation tokens. |
| 0034 (theme/font toggles) | blocked by 0035 | `ThemeContext` / `FontModeContext`, inline boot script, `localStorage` persistence (`ac-theme`, `ac-font-mode`), `prefers-color-scheme` fallback, `[data-font="mono"]` CSS block, populating 0035's two `data-slot` placeholders with functional buttons. |
| 0035 (this work) | ready, pass-2 review pending | Topbar component, breadcrumb component + loaders, origin pill + green pulse, four-state SSE indicator + amber pulse, `data-slot="theme-toggle"` and `data-slot="font-mode-toggle"` empty placeholders, **deletion of `SidebarFooter/`**. |
| 0036 (sidebar redesign) | blocked by 0035 | Lifecycle-phase grouping, doc-type glyphs, count badges, unseen dots, search input, Activity feed. **Confirms 0035 owns SidebarFooter removal.** |
| 0041 (library landing) | depends on 0035's loader contract | Replaces `/library` redirect with a real overview hub; will attach a `{ crumb: 'Library' }` loader against 0035's API. |

## Code References

### Files to edit

- `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.tsx:17-26` — restructure shell to flex-column, mount `<Topbar>`
- `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.module.css:1-2` — add column wrapper class, keep row class for inner body
- `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx:3` — remove `SidebarFooter` import
- `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx:82` — remove `<SidebarFooter />` render site
- `skills/visualisation/visualise/frontend/src/router.ts:21-102` — add `loader: () => ({ crumb: '...' })` to each contributing route

### Files to create

- `skills/visualisation/visualise/frontend/src/components/Topbar/Topbar.tsx` — main component
- `skills/visualisation/visualise/frontend/src/components/Topbar/Topbar.module.css` — layout grid, brand, crumbs, pill, indicator, slots, **first `@keyframes` in the codebase** (green-pulse, amber-pulse)
- `skills/visualisation/visualise/frontend/src/components/Topbar/Topbar.test.tsx` — unit tests (mock `useDocEventsContext`, simulate route matches, assert pill / indicator / slot DOM)
- Optional: split into sub-files (`Brand.tsx`, `Breadcrumbs.tsx`, `OriginPill.tsx`, `SseIndicator.tsx`) — common in this codebase since `Sidebar`, `RelatedArtifacts`, etc. are flat single files; keeping `Topbar` as one file with internal helpers fits convention.

### Files to delete

- `skills/visualisation/visualise/frontend/src/components/SidebarFooter/SidebarFooter.tsx`
- `skills/visualisation/visualise/frontend/src/components/SidebarFooter/SidebarFooter.module.css`
- `skills/visualisation/visualise/frontend/src/components/SidebarFooter/SidebarFooter.test.tsx`
- (the directory itself)

### Reference files (read-only, for patterns)

- `skills/visualisation/visualise/frontend/src/api/use-doc-events.ts:15-19,165-169,173-175` — context shape and default
- `skills/visualisation/visualise/frontend/src/api/reconnecting-event-source.ts:6,29,38,66,86,110` — connection-state state machine
- `skills/visualisation/visualise/frontend/src/api/use-server-info.ts:1-23` — useServerInfo (NOT the origin source)
- `skills/visualisation/visualise/frontend/src/styles/global.css:75-78,85-87,154-155,184-185` — token bindings the Topbar reads
- `skills/visualisation/visualise/frontend/src/styles/tokens.ts:1-115` — typed mirror
- `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx:16` — example of `useRouterState` usage
- `skills/visualisation/visualise/frontend/src/test/router-helpers.tsx` — test helpers for rendering chrome under a router

## Architecture Insights

- **Component convention**: `src/components/<Name>/<Name>.{tsx,module.css,test.tsx}` — co-located CSS modules and tests. `Topbar` should follow this exactly, including a co-located test (which `RootLayout` notably lacks).
- **CSS-modules + global tokens**: layout/structural classes are scoped via
  `*.module.css` while design tokens come from `styles/global.css`. The
  Topbar should not introduce new global CSS — its keyframes belong inside
  the module file unless they need to be reused.
- **No theme provider yet**: `data-theme` is read by CSS but nothing
  toggles it (0034 owns that). The Topbar's brand gradient and SSE
  colours must therefore work in both light (default) and dark (declared
  but unreachable from the UI today) modes, but until 0034 ships, only
  light mode is observable in the running app.
- **DocEventsContext default**: the context has a non-throwing default so
  Topbar tests can mount the component without a Provider in trivial
  cases. For SSE-state assertions the test should still wrap with a
  custom Provider.
- **Code-defined route tree**: adding loaders is a one-line change per
  route in `router.ts` — no file-based loader files. Keep loader bodies
  trivially synchronous (return literals) where possible to avoid loader
  races during navigation.
- **First `window.location.*` consumer**: rendering during SSR or in a
  test environment without `jsdom`'s `location` set could be brittle.
  Practical mitigation: read `window.location.host` lazily inside an
  effect or a `useState` initialiser; fall back to an empty string if
  absent. (`vitest` + `jsdom` will provide `window.location.host`
  out of the box, but be deliberate.)
- **First keyframes**: defining them inside the CSS module keeps scope
  contained, but CSS-modules name-mangles `@keyframes` declarations only
  when referenced as `animation-name: keyframeName` from the same file —
  good. Use `@keyframes pulse-green` and `@keyframes pulse-amber`
  locally; reference them from `.originPulse` / `.sseReconnecting`
  rules.

## Historical Context

- `meta/work/0033-design-token-system.md` — token foundation; 0033 has
  shipped per 0034 Context.
- `meta/work/0034-theme-and-font-mode-toggles.md` — populates 0035's
  slots; blocked by 0035.
- `meta/work/0036-sidebar-redesign.md` — confirms 0035 owns SidebarFooter
  removal.
- `meta/work/0041-library-page-wrapper-and-overview-hub.md` — replaces
  `/library` redirect with a real route, will consume 0035's
  `{ crumb: string }` loader contract.
- `meta/reviews/work/0035-topbar-component-review-1.md` — pass-2 review,
  verdict REVISE; three open tightening items.
- `meta/design-inventories/2026-05-06-140608-claude-design-prototype/inventory.md` — defines BEM names (`.ac-topbar__brand`, `.ac-topbar__crumbs`, `.ac-topbar__sep`, `.ac-topbar__status`, `.ac-topbar__btn`, `.ac-topbar__spacer`) and the prose layout: brand | centred crumbs | spacer | status pill | SSE | theme button. **The inventory specifies no Topbar height, padding, grid columns, separator glyph, animation duration, or amber-pulse variant — these are unspecified design details.**
- `meta/design-inventories/2026-05-06-135214-current-app/inventory.md` — baseline; sidebar width 220 px; reconnected-flash duration 3,000 ms.
- `meta/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md` — the gap entry that originated this work item.
- `meta/decisions/ADR-0026-css-design-token-application-conventions.md` — token-application conventions that the Topbar must follow.

## Related Research

- `meta/research/2026-05-06-0033-design-token-system.md` — research backing
  the token foundation.
- `meta/research/2026-05-02-design-convergence-workflow.md` — the workflow
  that produced 0033-0042.
- `meta/research/2026-04-17-meta-visualiser-implementation-context.md` —
  earlier implementation context (SSE, connectivity).

## Open Questions

1. **Brand-mark structure**: do we keep the SVG inline in `Topbar.tsx` (per
   the example in 0035 Technical Notes) or extract into
   `assets/brand-mark.svg` and import? The codebase has no precedent for
   inline-SVG components yet; a small inline SVG keeps the brand mark
   self-contained and avoids a build-time SVG import config decision.
2. **Loader content for deep routes**: do loaders for `$type` / `$fileSlug`
   / `$slug` / `$name` return human-friendly titles (requires fetching) or
   literal slug fallbacks? The work item permits either; safer scope-wise
   is literal slugs in 0035 with a follow-up to enrich titles when 0036
   or 0041 lands.
3. **Topbar height token**: no token exists for chrome height. Define
   locally in `Topbar.module.css`, or introduce `--ac-topbar-h` in
   `global.css`? The latter is more invasive but useful when the main
   pane needs to compute `calc(100vh - var(--ac-topbar-h))`. The current
   `.shell { min-height: 100vh }` approach probably needs revisiting in
   the column layout.
4. **Origin-pill text font**: spec is silent. `--ac-font-mono` (Fira Code)
   matches the address-like content visually but the prototype CSS isn't
   committed to the repo. Decision to make during implementation.
5. **Toggle-slot rendering**: the AC says "zero visible content and
   occupies no visible space until populated by 0034" — clearest
   approach is `<div data-slot="theme-toggle" />` with no styling and
   `display: contents`-style behaviour, OR an empty fragment. A plain
   empty `<div>` of `width: 0` is testable via computed style; a
   `display: contents` version is invisible to layout but still queryable
   by `data-slot`. Suggest the former because it is straightforwardly
   testable.
6. **Pulse animation timing**: no spec; pick a reasonable default (e.g.
   2 s ease-in-out alternating opacity 1 → 0.4) and document the choice
   in the component file.
