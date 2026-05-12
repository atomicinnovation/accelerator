---
date: "2026-05-08T00:00:00+01:00"
type: plan
skill: create-plan
work-item: "meta/work/0035-topbar-component.md"
status: approved
---

# 0035 Topbar Component Implementation Plan

## Overview

Introduce a persistent `Topbar` chrome element above the existing
sidebar + main shell in the visualisation frontend. The Topbar surfaces
the brand wordmark, route-derived breadcrumbs (driven by per-route
TanStack Router `loader` data via a new `withCrumb()` helper), a
server-origin pill with a green connectivity pulse, a four-state SSE
indicator, and two empty `<div data-slot>` placeholders that 0034 will
populate with theme/font toggles. The work also removes the existing
`SidebarFooter` so chrome state consolidates in the Topbar.

The plan is organised around test-driven development: each phase
writes failing tests first, then the smallest production change that
makes them pass, then any incidental refactor. Sub-components are
split into sibling directories (`components/Brand/`,
`components/Breadcrumbs/`, `components/OriginPill/`,
`components/SseIndicator/`, `components/Topbar/`) so each TDD cycle
stays narrow and the codebase's one-component-per-directory convention
is preserved.

The Topbar introduces three first-of-kind conventions in the
frontend: a new `useOrigin()` hook (the only `window.location.*`
consumer, isolated behind a hook to match the codebase's existing
injection pattern), the first `@keyframes` declaration (a shared
`ac-pulse` in `global.css`, paired with a `prefers-reduced-motion`
opt-out), and the first usage of TanStack Router `useMatches()` +
`loaderData` for breadcrumbs (gated by a `withCrumb()` route helper
that makes the loader contract obvious at the call site). Each is
introduced in the phase that needs it, with tests asserting the
contract.

## Current State Analysis

The chrome shell at
`skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.tsx:17-26`
is a thin `<DocEventsContext.Provider>` wrapping a single flex-row
`div.shell` containing `<Sidebar>` + `<main>`. The corresponding
`RootLayout.module.css` has only two rules: `.shell` (flex row,
`min-height: 100vh`) and `.main` (`flex: 1; overflow: auto`). There
is no test file for `RootLayout`.

SSE / version status is currently rendered by
`components/SidebarFooter/SidebarFooter.tsx:1-28`, mounted at
`components/Sidebar/Sidebar.tsx:82` (imported at line 3). The footer
has its own `.module.css`, `.test.tsx`, and pins to the bottom of the
sidebar via `margin-top: auto`. It renders text-only states for
`reconnecting`, `open + justReconnected`, `connecting` (nothing),
`closed` (nothing), plus a `Visualiser v${version}` line driven by
`useServerInfo()`.

The router at `src/router.ts` defines eleven routes programmatically
(`createRootRoute` at L19; tree assembled L105-117). Zero of those
routes have a `loader`, and zero components in `src/` use
`useMatches`, `useMatch`, or `useLoaderData`. There is no breadcrumb
component anywhere.

`src/api/use-doc-events.ts:15-19` exports `DocEventsHandle`
(`{ setDragInProgress, connectionState, justReconnected }`) with
`ConnectionState = 'connecting' | 'open' | 'reconnecting' | 'closed'`
defined at `src/api/reconnecting-event-source.ts:6`. The
`DocEventsContext` has a non-throwing default at lines 165-169 so
out-of-Provider consumers receive `connectionState: 'connecting'`.
`useServerInfo()` (`src/api/use-server-info.ts:1-23`) returns
`{ name?, version? }` and has no origin field — the Topbar must
derive the origin from `window.location.host` (zero
`window.location.*` matches in `src/` today).

Design tokens have shipped via 0033: `src/styles/global.css:75-78`
defines `--ac-accent` / `--ac-accent-2` (light) and
`:154-155,184-185` (dark + media-query mirror); `--ac-ok` / `--ac-warn`
/ `--ac-err` are present; `--ac-font-display` (Sora),
`--ac-font-body` (Inter), `--ac-font-mono` (Fira Code) are bound;
`--radius-pill: 999px` and the `--sp-1`…`--sp-11` scale are bound.
`src/styles/tokens.ts` mirrors the typed surface and
`src/styles/global.test.ts` enforces token parity. There is no
`--ac-topbar-h` token.

Test infrastructure (`vite.config.ts:48-55`) uses Vitest + jsdom +
`src/test/setup.ts` (which stubs `EventSource`, `ResizeObserver`,
`scrollTo`, and `scrollIntoView` — but not `window.location`). The
codebase convention is to mock context hooks via `vi.mock(...)`
rather than wrap with `<Provider>`, query by role/text/data-attribute
(no CSS-modules class assertions, no `getComputedStyle`), and use
`fireEvent` rather than `userEvent`. Router tests build a bespoke
router with the real `routeTree` and `createMemoryHistory` (see
`src/router.test.tsx:1-152`).

## Desired End State

After this plan ships:

- `RootLayout` is a flex-column with a 56-px `Topbar` row above an
  inner row containing `Sidebar` + `main`. The
  `<DocEventsContext.Provider>` wraps the entire tree so the Topbar's
  SSE indicator reads live state. Layout invariant: `.root` uses
  `min-height: 100vh` and `.body` uses `flex: 1` inside the
  column — the shell fills the viewport when content is short and
  grows with the main content when content is tall.
- A new `--ac-topbar-h` token exists in `global.css` and `tokens.ts`
  under a new `LAYOUT_TOKENS` group (a domain-neutral home for future
  layout dimensions), with `global.test.ts` asserting parity.
- A new `withCrumb(crumb, options)` helper exported from `router.ts`
  wraps `createRoute` and attaches `loader: () => ({ crumb })` (or
  `loader: ({ params }) => ({ crumb: resolve(params) })` when the
  crumb is param-derived). Every contributing route uses the helper.
  Redirect-only routes (`indexRoute`, `libraryIndexRoute`) keep using
  raw `createRoute` and do not have a crumb.
- `Breadcrumbs` filters matches via `isMatch(m, 'loaderData.crumb')`
  and a length check, with no inline `as` cast. In `import.meta.env.DEV`,
  it `console.warn`s when a non-redirect, non-pending match is missing
  `crumb` so route authors get immediate feedback.
- New sibling component directories exist: `components/Topbar/`,
  `components/Brand/`, `components/Breadcrumbs/`, `components/OriginPill/`,
  `components/SseIndicator/`. Each contains its own
  `{Component}.{tsx,module.css,test.tsx}`. The Topbar imports the
  four sub-components directly from these sibling directories (each
  sub-component is independently exported).
- A new `src/api/use-origin.ts` exports `useOrigin()` returning the
  server origin host string. It is the codebase's only
  `window.location.*` consumer; tests mock it via `vi.mock` per
  the existing context-hook convention.
- The Topbar's flex layout slots are: `[brand] [breadcrumbs]
  [spacer] [origin-pill] [sse-indicator] [theme-toggle slot]
  [font-mode-toggle slot]`. The breadcrumb element sits flush
  against the brand container (its CSS module declares
  `margin-left: 0`; correctness is asserted via a raw-CSS string
  check, not a runtime attribute marker).
- The origin pill renders `useOrigin()` in `--ac-font-mono` alongside
  a green pulse dot. The SSE indicator renders four colours mapped
  to `ConnectionState` (open=ok, reconnecting=warn, connecting=
  fg-faint, closed=err); the `reconnecting` state animates via
  `data-animated="true"` (which React omits from the DOM in other
  states). Both pulse animations reference a shared `@keyframes
  ac-pulse` declared in `global.css`. A `@media
  (prefers-reduced-motion: reduce)` rule disables both animations.
- Two `<div data-slot="theme-toggle">` and `<div
  data-slot="font-mode-toggle">` elements sit in their fixed
  Topbar positions. The CSS rule `.slot:empty { width: 0; height:
  0; overflow: hidden; }` collapses them while empty; once 0034
  inserts children the rule self-disengages. No
  `data-empty` attribute is needed.
- `SidebarFooter` (component, module CSS, test file, directory) is
  deleted. `Sidebar.tsx` no longer imports or renders it. The
  `Sidebar` test file is updated so it no longer asserts on footer
  text. `useServerInfo()` and `DocEventsHandle.justReconnected` are
  retained as deferred capabilities (see "What We're NOT Doing").

### Verification

- `cd skills/visualisation/visualise/frontend && npm test` is green.
- `cd skills/visualisation/visualise/frontend && npm run typecheck`
  is green.
- `cd skills/visualisation/visualise/frontend && npm run lint` is
  green.
- Running `npm run dev` and visiting `/library`, `/library/decisions`,
  `/library/decisions/<slug>`, `/lifecycle`, `/lifecycle/<slug>`,
  `/kanban` shows the Topbar with the correct breadcrumb at the top
  of every page; the origin pill reads `127.0.0.1:5173` (or whichever
  port Vite chose) and pulses green; the SSE indicator is green
  while the dev server is up.
- `git grep "SidebarFooter"` in
  `skills/visualisation/visualise/frontend/src/` returns zero hits.

### Key Discoveries

- TanStack Router 1.168.23 (`frontend/package.json:24`,
  `node_modules/@tanstack/react-router/package.json:3`) re-exports
  `isMatch` from `@tanstack/router-core`
  (`node_modules/@tanstack/react-router/dist/esm/index.d.ts:1`); the
  `useMatches() → filter(m => isMatch(m, 'loaderData.crumb'))`
  pattern is supported.
- `DocEventsContext`'s non-throwing default
  (`use-doc-events.ts:165-169`) means simple Topbar tests do not need
  to wrap with the Provider; tests that vary `connectionState` mock
  `useDocEventsContext` per the `SidebarFooter.test.tsx:5-11,16-29`
  pattern.
- `src/test/router-helpers.tsx` does not support loader data — its
  internal `buildTestRouter` registers an index route that renders
  the supplied `ui`. Loader-aware breadcrumb tests reuse the
  production `routeTree` import (mirroring
  `src/router.test.tsx:1-14`) plus `createMemoryHistory` so the
  loader contract is exercised against the real route definitions
  rather than a parallel fixture.
- `vite.config.ts:48-55` enables `css: true`, so CSS-module class
  lookups via the imported `styles` object work in tests. The
  codebase convention is to assert via roles, text content, or
  intrinsic `data-*` state attributes — never against mangled class
  names. Where structural CSS *is* the contract under test
  (margins, animations, media queries), this plan asserts via
  raw-string checks on `.module.css?raw` (mirroring
  `src/styles/global.test.ts`'s pattern) rather than introducing
  test-only `data-*` markers in production markup.
- `libraryDocRoute`'s parent is `libraryTypeRoute` (`router.ts:74`),
  not `libraryRoute`, so the breadcrumb chain at
  `/library/decisions/foo` is `Library / decisions / foo`
  automatically — no manual chain plumbing needed.
- `parseParams` on `libraryTypeRoute` (`router.ts:65-70`) narrows
  `params.type` to `DocTypeKey` before the loader runs, so a literal
  `params.type` crumb is always a known doc-type key.
- Beforehand `beforeLoad: () => { throw redirect(...) }` routes never
  expose `loaderData`; verified by inspection of `router.test.tsx`
  redirect cases. Redirect routes (`indexRoute` L21-25,
  `libraryIndexRoute` L35-41) intentionally have no loader.

## What We're NOT Doing

- Theme/font-mode context wiring or toggle button content — owned by
  0034. This story delivers structural empty `data-slot` placeholders
  only.
- Loader-fetched breadcrumb titles. Deep routes use literal slug /
  param values (`params.type`, `params.fileSlug`, etc.) as crumbs.
  Replacing these with human-readable titles fetched in the loader
  is **deferred to 0036** (sidebar redesign — the natural place to
  also revisit chrome readability), tracked against the work item AC
  line "pulling document titles from loader data where available".
- Sidebar redesign (lifecycle grouping, glyphs, count badges) — owned
  by 0036.
- A real `/library` overview component — owned by 0041. Until then,
  the `libraryRoute` keeps its current `LibraryLayout` (`<Outlet/>`-
  only) and the `Library` crumb appears whenever any `/library/*`
  URL is active.
- Visual regression baselines for the Topbar. The token-system plan
  (0033) introduced the Playwright visual harness; the Topbar's
  presence will likely shift those baselines. We will refresh the
  affected baselines as a final commit but will not author new
  Topbar-specific visual tests.
- Topbar-specific server-origin source. We add a new `useOrigin()`
  hook that reads `window.location.host`; we do not extend
  `useServerInfo` with a new field.
- Removal of `useServerInfo()` or `DocEventsHandle.justReconnected`.
  After Phase 8, both have no production consumer (the SSE indicator
  reads only `connectionState`; the version label is dropped). They
  are retained because:
  (a) `useServerInfo` is plausibly the basis of a future About
  panel, and removing the hook plus its tests now would mean
  recreating both later;
  (b) `justReconnected` is plausibly the basis of toast notifications
  for transient reconnect feedback (the SidebarFooter's "Reconnected
  — refreshing" UX intentionally moves to that future surface, not
  the persistent SSE indicator).
  The dead-code state is intentional and explicit.

## Implementation Approach

Eight phases, each test-first. Each phase is independently mergeable
and ships an honest interim state (no phase leaves the visible app
in a broken transitional layout). Each phase ends with an explicit
running of the local `npm test`, `npm run typecheck`, and `npm run
lint` gates from `skills/visualisation/visualise/frontend/`.

The natural ordering is bottom-up:

1. **Foundation tokens** — add `--ac-topbar-h` to `global.css` and
   `tokens.ts` under a new `LAYOUT_TOKENS` group, plus parity test.
   The shared `ac-pulse` keyframe is also declared in `global.css`
   here. No structural changes; nothing visually changes.
2. **Route loaders via `withCrumb()`** — introduce the
   `withCrumb()` helper in `router.ts` and convert every contributing
   route to use it. Provides the `loaderData.crumb` contract the
   Breadcrumbs component will consume.
3. **Brand** — pure presentation, no hooks; own sibling directory.
4. **`useOrigin()` hook** — small dependency for `OriginPill`; first
   `window.location.*` consumer, isolated behind a hook tested via
   `vi.mock`.
5. **Breadcrumbs** — needs Phase 2's `withCrumb`-driven loaders;
   reuses the production `routeTree` in tests; renders WAI-ARIA
   `<ol>`/`<li>` with `aria-current="page"`; logs a dev-only warn
   when a non-redirect match is missing `crumb`.
6. **OriginPill** — consumes `useOrigin()` from Phase 4; references
   the `ac-pulse` keyframe declared in Phase 1; ships its own
   `prefers-reduced-motion` override.
7. **SseIndicator** — needs `DocEventsContext` mock; uses `data-state`
   for colour mapping (asserted via raw-CSS string check) and
   `data-animated` for the amber pulse; ships its own
   `prefers-reduced-motion` override.
8. **Topbar composition + RootLayout integration + SidebarFooter
   removal** — composes the four sub-components, places the two
   empty `data-slot` divs (collapsed via `:empty` CSS), restructures
   `RootLayout` from a flex-row into a flex-column with the Topbar
   above the body row, and deletes the obsolete `SidebarFooter`.
   The shell restructure and Topbar mount land together so no
   interim commit ships a column shell with no header.

All file paths below are relative to the repo root.

---

## Phase 1: Foundation — `--ac-topbar-h` token + shared `ac-pulse` keyframe

### Overview

Add the `--ac-topbar-h: 56px` token under a new `LAYOUT_TOKENS` group,
wire it through the typed mirror and parity test, and declare the
shared `@keyframes ac-pulse` in `global.css` (consumed by both
`OriginPill` and `SseIndicator` in later phases). No structural or
visible changes — purely additive token + global CSS scaffolding.

### Changes Required

#### 1. Token parity test (test first)

**File**: `skills/visualisation/visualise/frontend/src/styles/global.test.ts`
**Changes**: Add the new `LAYOUT_TOKENS` group to whichever parity
loop iterates token groups, asserting that `--ac-topbar-h` is declared
in `global.css?raw` and equals `tokens.ts`'s `LAYOUT_TOKENS['--ac-topbar-h']`
value (`'56px'`). Also add a separate `it(...)` block asserting that
`global.css?raw` contains an `@keyframes ac-pulse` declaration with
the canonical body (`0%, 100% { opacity: 1; } 50% { opacity: 0.4; }`)
— a raw-string check so future refactors that delete or alter the
keyframe fail loudly.

#### 2. Token declaration

**File**: `skills/visualisation/visualise/frontend/src/styles/global.css`
**Changes**: Add `--ac-topbar-h: 56px;` inside `:root`. Also append
the shared keyframe at the end of the file (outside `:root`):

```css
:root {
  /* …existing tokens… */
  --ac-topbar-h: 56px;
}

@keyframes ac-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}
```

#### 3. Typed mirror

**File**: `skills/visualisation/visualise/frontend/src/styles/tokens.ts`
**Changes**: Add a new `LAYOUT_TOKENS` constant (a domain-neutral group
for layout dimensions; future entries: sidebar width, content max-
width, etc.) containing the `ac-topbar-h` entry. Keys are unprefixed
kebab-case to match the existing convention (`SPACING_TOKENS = {
'sp-1': '4px', ... }`, `RADIUS_TOKENS = { 'radius-pill': '999px' }`)
— the parity loop prepends `--` at the consumption site via
`readCssVar`. Update `global.test.ts`'s parity loops to iterate the
new constant alongside the existing groups.

```ts
export const LAYOUT_TOKENS = {
  'ac-topbar-h': '56px',
} as const
```

### Success Criteria

#### Automated Verification

- [ ] `npm test -- src/styles/global.test.ts` passes from
      `skills/visualisation/visualise/frontend/`.
- [ ] `npm run typecheck` passes.
- [ ] `npm run lint` passes.

#### Manual Verification

- [ ] `npm run dev` from the frontend directory; the app is
      visually unchanged (only token + keyframe scaffolding shipped).

---

## Phase 2: Route loaders via a `withCrumb()` helper

### Overview

Introduce a `withCrumb()` helper exported from `router.ts` that wraps
`createRoute` and attaches a `loader` returning `{ crumb }`. Convert
every contributing route to use the helper. The helper makes the
loader contract obvious at the call site (so future route authors
who add a route by copying an existing one inherit the crumb
contract) and centralises the loader-data shape so the breadcrumb
consumer never has to cast.

Redirect-only routes (`indexRoute`, `libraryIndexRoute`) keep using
raw `createRoute` and have no crumb.

### Changes Required

#### 1. Loader assertions (test first)

**File**: `skills/visualisation/visualise/frontend/src/router.test.tsx`
**Changes**: Add new `describe('loader crumbs', …)` block. For each
contributing route, drive the existing router to the matching path
using the same `waitForPath(router, '...')` helper that
`router.test.tsx:24-36` already uses for redirect-chain settling
(`router.load()` does not single-shot multi-hop redirects — see the
file's existing comment). Once the path settles, read
`router.state.matches` and assert the appropriate match has
`loaderData.crumb` set to the expected literal.

Cases to cover:

- `/library/templates` → templates index match carries `crumb:
  'Templates'`; chain also has `crumb: 'Library'`.
- `/library/templates/adr` → template detail match carries `crumb:
  'adr'` (the `params.name` value).
- `/library/decisions` → libraryType match carries `crumb:
  'decisions'`; parent `libraryRoute` carries `crumb: 'Library'`.
- `/library/decisions/some-slug` → libraryDoc match carries `crumb:
  'some-slug'`; chain also has `Library` and `decisions`.
- `/lifecycle` → lifecycleRoute match carries `crumb: 'Lifecycle'`.
- `/lifecycle/some-cluster` → lifecycleCluster match carries `crumb:
  'some-cluster'`.
- `/kanban` → kanbanRoute match carries `crumb: 'Kanban'`.

The `/` and `/library` cases are intentionally omitted from the
loader-crumbs block: both trigger redirect chains
(`indexRoute → /library → /library/decisions`,
`libraryIndexRoute → /library/decisions`) whose final settled state
is the `/library/decisions` case already covered above. The redirect
hops themselves carry no `loaderData` and are uninteresting for the
crumb contract.

#### 1b. `resolveCrumb()` unit test (test first)

**File**: `skills/visualisation/visualise/frontend/src/router-with-crumb.test.ts` (new)
**Changes**: Add a small unit suite for the pure `resolveCrumb()`
function (the helper at the heart of `withCrumb`). Testing
`resolveCrumb` directly avoids constructing a TanStack
`LoaderFnContext` — which would require ten required fields — and
isolates the helper's logic from the router. Three cases:

- `resolveCrumb('Static', {})` returns `{ crumb: 'Static' }`
  (params object is ignored for the static case).
- `resolveCrumb(({ params }) => params.x, { x: 'foo' })` returns
  `{ crumb: 'foo' }`.
- `resolveCrumb(({ params }) => params.x.toUpperCase(),
  { x: 'foo' })` returns `{ crumb: 'FOO' }` — confirms the resolver
  is invoked, not just the parameter spread.

The `withCrumb` integration with TanStack (loader attachment, route
construction) is implicitly covered by Phase 2 §1's loader-crumbs
block in `router.test.tsx`, which navigates the production router
to each contributing path and asserts `loaderData.crumb`. The two
together — pure-function unit + production-router integration —
exercise the helper without needing to mock a `LoaderFnContext`.

(File name `router-with-crumb.test.ts` rather than
`router-helpers.test.ts` to avoid semantic collision with the
existing `src/test/router-helpers.tsx`, which is unrelated test
infrastructure.)

#### 2. Add `withCrumb()` helper + `resolveCrumb()` pure function

**File**: `skills/visualisation/visualise/frontend/src/router.ts`
**Changes**: Export two related functions:

- `resolveCrumb(crumbOrResolver, params)` — a pure function that
  collapses static and resolver-based crumb sources into a single
  `{ crumb: string }` return shape. Unit-testable in isolation
  (Phase 2 §1b) without constructing a TanStack `LoaderFnContext`.
- `withCrumb(crumbOrResolver, options)` — wraps `createRoute` and
  attaches a loader that calls `resolveCrumb`. The type contract
  is intentionally runtime-only: the helper returns
  `ReturnType<typeof createRoute>`, and consumers narrow via the
  `hasCrumb` type guard in `Breadcrumbs.tsx` (Phase 5).

```ts
import { createRoute } from '@tanstack/react-router'

export type CrumbLoaderData = { crumb: string }

type CrumbResolver = (args: {
  params: Record<string, string>
}) => string

export function resolveCrumb(
  crumbOrResolver: string | CrumbResolver,
  params: Record<string, string>,
): CrumbLoaderData {
  return {
    crumb:
      typeof crumbOrResolver === 'string'
        ? crumbOrResolver
        : crumbOrResolver({ params }),
  }
}

export function withCrumb(
  crumbOrResolver: string | CrumbResolver,
  options: Omit<Parameters<typeof createRoute>[0], 'loader'>,
) {
  return createRoute({
    ...options,
    loader: ({ params }) =>
      resolveCrumb(crumbOrResolver, params as Record<string, string>),
  })
}
```

Notes on the design:

- **No `declare module` augmentation.** The plan does not add a
  TanStack module augmentation. `useMatches()[i].loaderData` will
  remain typed via TanStack's existing inference (likely `unknown`
  or a per-match union depending on the call site); the
  `Breadcrumbs.tsx` consumer narrows via `hasCrumb`, which is the
  single site that asserts the `{ crumb: string }` contract. This
  keeps the helper decoupled from TanStack's internal type
  machinery and avoids version-drift risk.
- **`Omit<..., 'loader'>` on the options parameter.** A future
  contributor who passes their own `loader` to `withCrumb`'s
  options will get a compile error (rather than having their
  loader silently overwritten by the spread). If a future story
  needs both a crumb and a data-fetching loader on the same route,
  it should add a sibling `withCrumbAndLoader` helper that
  composes the two — out of scope for 0035.
- **`Record<string, string>` cast on `params`.** TanStack's
  `LoaderFnContext` types `params` more narrowly per route, but
  `resolveCrumb`'s contract takes the broadest pre-`parseParams`
  shape; the cast inside `withCrumb` is local and bounded.

#### 3. Convert every contributing route

**File**: `skills/visualisation/visualise/frontend/src/router.ts`
**Changes**: Replace each `createRoute({ ..., loader: ... })` with
`withCrumb(crumb, { ... })`:

```ts
const libraryRoute = withCrumb('Library', {
  getParentRoute: () => rootRoute,
  path: '/library',
  component: LibraryLayout,
})

const libraryTemplatesIndexRoute = withCrumb('Templates', {
  getParentRoute: () => libraryRoute,
  path: '/templates',
  component: LibraryTemplatesIndex,
})

const libraryTemplateDetailRoute = withCrumb(
  ({ params }) => params.name,
  {
    getParentRoute: () => libraryRoute,
    path: '/templates/$name',
    component: LibraryTemplatesView,
  },
)

const libraryTypeRoute = withCrumb(({ params }) => params.type, {
  // existing parseParams stays
  getParentRoute: () => libraryRoute,
  path: '/$type',
  component: LibraryTypeView,
})

const libraryDocRoute = withCrumb(({ params }) => params.fileSlug, {
  getParentRoute: () => libraryTypeRoute,
  path: '/$fileSlug',
  component: LibraryDocView,
})

const lifecycleRoute = withCrumb('Lifecycle', {
  getParentRoute: () => rootRoute,
  path: '/lifecycle',
  component: LifecycleLayout,
})

const lifecycleClusterRoute = withCrumb(({ params }) => params.slug, {
  getParentRoute: () => lifecycleRoute,
  path: '/$slug',
  component: LifecycleClusterView,
})

const kanbanRoute = withCrumb('Kanban', {
  getParentRoute: () => rootRoute,
  path: '/kanban',
  component: KanbanBoard,
})
```

The redirect routes (`indexRoute`, `libraryIndexRoute`,
`lifecycleIndexRoute`) keep using `createRoute` — they intentionally
don't contribute a crumb.

### Success Criteria

#### Automated Verification

- [ ] `npm test -- src/router.test.tsx` passes including the new
      `loader crumbs` block.
- [ ] `npm run typecheck` passes (TanStack Router infers
      `loaderData.crumb` as `string`).
- [ ] `npm run lint` passes.

#### Manual Verification

- [ ] `npm run dev`; existing routing still functions end-to-end —
      no behavioural changes expected since no consumer reads
      `loaderData.crumb` until Phase 5.

---

## Phase 3: `Brand` component (own sibling directory)

### Overview

Pure-presentation component rendering the SVG hex mark plus the
"Accelerator" wordmark and "VISUALISER" sub-label. No hooks. The SVG
is inlined inside `Brand.tsx`; the gradient consumes
`var(--ac-accent)` and `var(--ac-accent-2)`. The visible text serves
as the accessible name; the SVG is marked `aria-hidden="true"` so
screen readers don't double-announce.

### Changes Required

#### 1. Test first

**File**: `skills/visualisation/visualise/frontend/src/components/Brand/Brand.test.tsx` (new)
**Changes**: Render `<Brand />`; assert:

- An `<svg>` element is present, has `aria-hidden="true"`, and
  contains a `<linearGradient>` whose stops reference
  `var(--ac-accent)` and `var(--ac-accent-2)` (assert via
  `getAttribute('stop-color')`).
- Text content contains "Accelerator" and "VISUALISER" (separate
  `getByText` calls — these are the accessible name).

#### 2. Brand component

**File**: `skills/visualisation/visualise/frontend/src/components/Brand/Brand.tsx` (new)
**Changes**: Implement the component. SVG markup taken from the work
item Technical Notes (`meta/work/0035-topbar-component.md:148-162`),
with `var(--ac-accent)` / `var(--ac-accent-2)` references intact and
`aria-hidden="true"` on the `<svg>` root. "Accelerator" rendered with
`--ac-font-display`; "VISUALISER" with `--ac-font-body` +
`letter-spacing: var(--tracking-caps)` + uppercase text-transform. The
visible text constitutes the accessible name; the wrapper is a plain
`<div className={styles.brand}>` (no `role`, no `aria-label`).

#### 3. CSS

**File**: `skills/visualisation/visualise/frontend/src/components/Brand/Brand.module.css` (new)
**Changes**: Create the file. Add a `.brand` class for layout (flex
row, gap, mark + text alignment), `.brandName` (Sora display font),
`.brandSub` (uppercase, tracked, smaller).

### Success Criteria

#### Automated Verification

- [ ] `npm test -- src/components/Brand/Brand.test.tsx` passes.
- [ ] `npm run typecheck` passes.
- [ ] `npm run lint` passes.

#### Manual Verification

- [ ] None at this phase (component not mounted yet).

---

## Phase 4: `useOrigin()` hook

### Overview

Add the codebase's first (and only) `window.location.*` consumer
isolated behind a small hook in `src/api/`. The hook reads
`window.location.host` once at mount, returning a stable string for
the session. This matches the codebase's existing injection pattern
(other ambient I/O like `EventSource` is also accessed via factories/
hooks) and lets `OriginPill` tests mock the hook with `vi.mock`
rather than monkey-patching `window.location` (which is unreliable
in jsdom).

### Changes Required

#### 1. Test first

**File**: `skills/visualisation/visualise/frontend/src/api/use-origin.test.ts` (new)
**Changes**: Render the hook via `renderHook` from
`@testing-library/react`. Three assertions:

- **Initial value**: assert the hook returns `window.location.host`
  (assert via equality to `window.location.host` rather than a
  hard-coded literal so the test is environment-agnostic).
- **Read-once contract** (meaningful regression guard): in
  `beforeEach`, save the original descriptor via
  `Object.getOwnPropertyDescriptor(window.location, 'host')`,
  install a getter spy via `Object.defineProperty(window.location,
  'host', { configurable: true, get: vi.fn().mockReturnValue(
  'initial.example') })`; mount the hook; call `rerender()` twice;
  assert the getter `vi.fn()` was called exactly once
  (`expect(spy).toHaveBeenCalledTimes(1)`). In `afterEach`, restore
  the saved original descriptor.
- **Stability under host mutation**: with the hook still mounted,
  call `(spy as Mock).mockReturnValue('changed.example')` on the
  existing spy — do NOT redefine the property descriptor (which
  would invalidate the saved original). Call `rerender()`; assert
  the hook still returns `'initial.example'` (proving the read is
  cached at mount, not re-read on every render).

Note: this test stubs `window.location.host` (a single property),
not `window.location` itself. The implementer should confirm
`host` is configurable in the project's installed jsdom version
*before authoring the test* by running a one-liner in the dev
console (`Object.getOwnPropertyDescriptor(window.location,
'host').configurable`) — modern jsdom versions install `host` as a
configurable accessor, but historical jsdom installed it as
non-configurable. If the project's installed jsdom returns
`configurable: false`, fall back to refactoring `useOrigin` to
accept an optional injected reader (`useOrigin(read = () =>
window.location.host)`) and mock that reader argument in the test
instead. The fallback also matches the codebase's existing
factory-injection pattern (`EventSourceFactory` in
`use-doc-events.ts`).

#### 2. Hook implementation

**File**: `skills/visualisation/visualise/frontend/src/api/use-origin.ts` (new)
**Changes**: Read once via `useState`'s lazy initialiser. SPA-only
codebase, so no SSR guard:

```ts
import { useState } from 'react'

export function useOrigin(): string {
  const [host] = useState(() => window.location.host)
  return host
}
```

### Success Criteria

#### Automated Verification

- [ ] `npm test -- src/api/use-origin.test.ts` passes.
- [ ] `npm run typecheck` passes.
- [ ] `npm run lint` passes.

#### Manual Verification

- [ ] None at this phase (hook not consumed yet).

---

## Phase 5: `Breadcrumbs` component (own sibling directory)

### Overview

Render the breadcrumb trail from `useMatches()` filtered by
`isMatch(m, 'loaderData.crumb')` plus a non-empty string check
(empty-string crumbs are excluded). Markup follows the W3C WAI-ARIA
Breadcrumb pattern: `<nav aria-label="Breadcrumb">` wrapping an
`<ol>` of `<li>` items. Ancestor crumbs are navigable via plain
`<a href={m.pathname}>` plus an `onClick` that calls
`router.navigate({ to: m.pathname })` (TanStack's imperative
`navigate` accepts a runtime `string`, while `<Link to=...>` is a
constrained literal type that does not accept arbitrary string
pathnames). The trailing crumb is a `<span aria-current="page">`.

Using `<a href>` preserves middle-click and ⌘/Ctrl-click open-in-
new-tab semantics for free; the `onClick` handler intercepts plain
left-click (when no modifier keys are held) to delegate to the
router for client-side navigation.

In `import.meta.env.DEV`, the component `console.warn`s when a
non-root match with `status === 'success'` is missing `crumb` so
route authors get immediate feedback when they forget to use
`withCrumb()`. The root match is filtered out by importing
`rootRouteId` from `@tanstack/router-core` and comparing
`m.routeId === rootRouteId`. Redirect-status matches are excluded
by the `m.status === 'success'` check (redirects resolve with
`status: 'redirected'`).

Tests reuse the production `routeTree` import via a shared
`setupRouterFixtures()` helper so loader-contract drift between
production and tests is impossible and the fetch-stub set is the
single source of truth (see Phase 5 §0 below).

### Changes Required

#### 0. Shared router fixtures helper (test prerequisite)

**File**: `skills/visualisation/visualise/frontend/src/test/router-fixtures.ts` (new)
**Changes**: Consolidate the router test fixtures into a reusable
helper so `router.test.tsx` and `Breadcrumbs.test.tsx` share a
single fixture surface and the fetch-stub set is enumerated
explicitly (no implicit "etc." gaps). The helper exports three
things:

- `setupRouterFixtures()` — installs `vi.spyOn` stubs on **only the
  unconditionally-needed fetches** with sensible default fixture
  data. This matches the existing discipline in
  `src/router.test.tsx:54-60` where only `fetchTypes`,
  `fetchTemplates`, and `fetchTemplateDetail` are stubbed in
  `beforeEach`. Per-test fetches (`fetchDocs`,
  `fetchLifecycleClusters`, `fetchLifecycleCluster`, kanban
  config) remain explicit per-test setup so each test's intent
  is locally visible.
- `buildRouter({ url })` — constructs a router with the production
  `routeTree` and `createMemoryHistory({ initialEntries: [url] })`,
  ready for `await waitForPath(router, url)`.
- `waitForPath(router, path)` — extracted from the existing
  `router.test.tsx:24-36` helper if not already exported there.

Refactor `src/router.test.tsx` to import the same helper so the
fixture set has one definition. Importantly, the refactor
**preserves the existing per-test stub discipline** — moving only
the unconditional `beforeEach` set into `setupRouterFixtures()`,
not hoisting per-test stubs. This keeps each test's intent locally
readable and prevents the global helper from masking missing
stubs.

**Per-URL fetch dependencies for the Breadcrumbs URL matrix
(Phase 5 §1).** Each Breadcrumbs test case must explicitly stub the
fetches its target route's component invokes; `setupRouterFixtures()`
covers only the unconditional set. Enumeration:

| URL                                 | Component(s) mounted              | Required per-test stubs                                              |
|-------------------------------------|-----------------------------------|----------------------------------------------------------------------|
| `/library/templates`                | `LibraryTemplatesIndex`           | (none — covered by `setupRouterFixtures` `fetchTemplates`)           |
| `/library/templates/adr`            | `LibraryTemplatesView`            | (none — covered by `setupRouterFixtures` `fetchTemplateDetail`)      |
| `/library/decisions`                | `LibraryTypeView`                 | `fetchDocs('decisions')` → empty array fixture                       |
| `/library/decisions/foo-slug`       | `LibraryDocView` (+ `LibraryTypeView` outlet) | `fetchDocs('decisions')` returning a doc whose slug is `'foo-slug'` so the route resolves; `fetchDocContent` if `LibraryDocView` invokes it on mount |
| `/lifecycle/cluster-x`              | `LifecycleClusterView`            | `fetchLifecycleCluster('cluster-x')` → minimal fixture cluster        |
| `/kanban`                           | `KanbanBoard`                     | kanban config fetch (per `KanbanBoard.test.tsx:41-67` pattern); `fetchDocs` for each tracked type if invoked |

(The implementer should grep each component's mount path before
writing the test to confirm the fetch list is complete; the table
above is the planning-time best estimate based on
`router.test.tsx`'s existing stub patterns.)

#### 1. Test first

**File**: `skills/visualisation/visualise/frontend/src/components/Breadcrumbs/Breadcrumbs.test.tsx` (new)
**Changes**: Use `setupRouterFixtures()` from §0. Build a router
via `buildRouter({ url })` per case, await `waitForPath(router,
url)`, then render `<Breadcrumbs />` inside the router context
using a small wrapper component that places `<Breadcrumbs />` above
the `<Outlet />`. Cover:

Use `screen.getByRole('link', { name: '<text>' })` to query
ancestor crumbs (asserts the `<a>` role, scoping the match to
non-current items since the trailing crumb is a `<span>` not an
`<a>`); read `.getAttribute('href')` for the URL assertion.
`screen.getByText(/<text>/)` is fine for the `aria-current="page"`
crumb since only one element renders that text in the trailing
position.

- At `/library/templates`: two `<li>` items ("Library", "Templates"),
  with `aria-current="page"` on "Templates"; "Library" rendered as
  `<a>` with `href="/library"`.
- At `/library/templates/adr`: three items ("Library", "Templates",
  "adr"), `aria-current="page"` on "adr"; "Library" link
  `href="/library"`, "Templates" link `href="/library/templates"`.
- At `/library/decisions`: two items ("Library", "decisions"),
  `aria-current` on "decisions"; "Library" link `href="/library"`.
- At `/library/decisions/foo-slug`: three items ("Library",
  "decisions", "foo-slug"), `aria-current` on "foo-slug";
  "Library" link `href="/library"`, "decisions" link
  `href="/library/decisions"`.
- At `/lifecycle/cluster-x`: two items ("Lifecycle", "cluster-x"),
  `aria-current` on "cluster-x"; "Lifecycle" link `href="/lifecycle"`.
- At `/kanban`: one item ("Kanban"), `aria-current="page"`. (Single-
  item trail — no ancestor links; `screen.queryAllByRole('link')`
  inside the breadcrumbs nav returns an empty array.)
- The root element is `<nav aria-label="Breadcrumb">` wrapping an
  `<ol>`.

Click-handler unit (test second):

- For one ancestor crumb, simulate a plain left-click via
  `fireEvent.click(linkElement)`; assert the router's `navigate`
  spy is called with `{ to: '/library' }` (or the expected target).
- Simulate a ⌘-click via `fireEvent.click(linkElement, { metaKey:
  true })`; assert `navigate` is NOT called (the native href
  behaviour takes over for new-tab navigation).

Pending-state and empty-list units (test second):

- Add a small unit suite that mocks `useMatches` directly (rather
  than driving the full router) to assert: (a) a synthetic match
  with `status: 'pending'` is excluded; (b) when the filtered list
  is empty, the component returns `null`.

Dev-warn unit:

- Mock `useMatches`. Assert each status branch of the production
  predicate (`m.status === 'success' && m.routeId !== rootRouteId
  && !isMatch(m, 'loaderData.crumb')`):
  - (a) `routeId: '/some/path'`, `status: 'success'`, `loaderData:
    {}` → triggers one `console.warn`.
  - (b) `routeId: rootRouteId`, `status: 'success'`, `loaderData:
    {}` → does NOT warn (root is exempt).
  - (c) `routeId: '/library/'`, `status: 'redirected'`, `loaderData:
    undefined` → does NOT warn (redirect-status excluded).
  - (d) `routeId: '/library/$type'`, `status: 'pending'`,
    `loaderData: undefined` → does NOT warn (pending excluded).
  - (e) `routeId: '/library/$type'`, `status: 'error'`,
    `loaderData: undefined` → does NOT warn (error excluded — a
    failed loader is a separate concern from a missing
    `withCrumb()`).
  - (f) `routeId: '/library'`, `status: 'success'`, `loaderData:
    { crumb: 'Library' }` → does NOT warn (loader contract
    fulfilled).

  Spy on `console.warn` per case via `vi.spyOn(console, 'warn')`.

Together, cases (a)–(f) exercise every status value and the
rootRouteId exemption, locking the predicate into the suite so a
future refactor that drops `m.status === 'success'` (and silently
starts warning on every pending navigation) fails loudly.

CSS structural assertion:

- Read `Breadcrumbs.module.css?raw` and assert it matches the regex
  `/\.breadcrumbs\s*\{[^}]*margin-left:\s*0/` (selector-bound, so a
  refactor that moves the rule to another selector fails the
  check).

#### 2. Breadcrumbs component

**File**: `skills/visualisation/visualise/frontend/src/components/Breadcrumbs/Breadcrumbs.tsx` (new)
**Changes**:

```tsx
import {
  isMatch,
  useMatches,
  useRouter,
} from '@tanstack/react-router'
import { rootRouteId } from '@tanstack/router-core'
import type { MouseEvent } from 'react'
import styles from './Breadcrumbs.module.css'

type Match = ReturnType<typeof useMatches>[number]
type CrumbMatch = Omit<Match, 'loaderData'> & {
  loaderData: { crumb: string }
}

function hasCrumb(m: Match): m is CrumbMatch {
  return (
    isMatch(m, 'loaderData.crumb') &&
    typeof m.loaderData?.crumb === 'string' &&
    m.loaderData.crumb.length > 0
  )
}

export function Breadcrumbs() {
  const matches = useMatches()
  const router = useRouter()

  if (import.meta.env.DEV) {
    for (const m of matches) {
      if (
        m.status === 'success' &&
        m.routeId !== rootRouteId &&
        !isMatch(m, 'loaderData.crumb')
      ) {
        console.warn(
          `[Breadcrumbs] Route ${m.routeId} has no loaderData.crumb. ` +
            `Did you forget to use withCrumb()?`,
        )
      }
    }
  }

  const crumbs = matches.filter(hasCrumb)
  if (crumbs.length === 0) return null

  const handleClick = (pathname: string) => (e: MouseEvent) => {
    // Allow modifier-click (cmd/ctrl/middle/shift) to open in a new
    // tab via the native href; intercept plain left-click for SPA
    // navigation.
    if (e.metaKey || e.ctrlKey || e.shiftKey || e.button !== 0) return
    e.preventDefault()
    router.navigate({ to: pathname as never })
  }

  return (
    <nav className={styles.breadcrumbs} aria-label="Breadcrumb">
      <ol className={styles.list}>
        {crumbs.map((m, i) => {
          const isLast = i === crumbs.length - 1
          return (
            <li key={m.id} className={styles.crumb}>
              {i > 0 && (
                <span className={styles.sep} aria-hidden="true">
                  /
                </span>
              )}
              {isLast ? (
                <span aria-current="page">{m.loaderData.crumb}</span>
              ) : (
                <a
                  href={m.pathname}
                  onClick={handleClick(m.pathname)}
                  className={styles.link}
                >
                  {m.loaderData.crumb}
                </a>
              )}
            </li>
          )
        })}
      </ol>
    </nav>
  )
}
```

Notes on the design:

- **`<a href>` + `router.navigate` rather than `<Link>`.** TanStack
  Router's `Link` component types `to` as a constrained literal
  path (`ConstrainLiteral<...>`), not `string`. `Match.pathname` is
  a runtime string and cannot satisfy that constraint without
  casting through generics. Using a plain `<a href={m.pathname}>`
  keeps middle-click and ⌘/Ctrl-click open-in-new-tab working
  natively, and the `onClick` interceptor delegates plain
  left-click to `router.navigate({ to })`. The single `as never`
  cast on the imperative `navigate` call is localised — TanStack's
  imperative API accepts strings at runtime even though the type
  is constrained.
- **`Omit<Match, 'loaderData'> & { ... }`.** Defining `CrumbMatch`
  via `Omit` rather than a bare intersection avoids
  `loaderData?: undefined` branches of the Match union collapsing
  to `never`; the result is stable even if a future route adds a
  loader returning a non-crumb shape.
- **Dev-warn condition.** `m.status === 'success' && m.routeId !==
  rootRouteId && !isMatch(m, 'loaderData.crumb')`. Excludes the
  root match (`rootRouteId === '__root__'`) and any non-success
  match (pending, error, notFound, redirected). Redirect routes
  resolve with `status: 'redirected'` and so never reach the warn
  — verified by the test cases in §1 (which drive the production
  redirect routes through the real `routeTree`).

#### 3. CSS

**File**: `skills/visualisation/visualise/frontend/src/components/Breadcrumbs/Breadcrumbs.module.css` (new)
**Changes**:

```css
.breadcrumbs {
  margin-left: 0;
}

.list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  font-family: var(--ac-font-body);
}

.crumb {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-2);
}

.link {
  color: var(--ac-fg);
  text-decoration: underline;
  text-decoration-color: var(--ac-fg-faint);
  text-underline-offset: 2px;
}

.link:hover {
  color: var(--ac-accent);
  text-decoration-color: var(--ac-accent);
}

.link:focus-visible {
  outline: 2px solid var(--ac-accent);
  outline-offset: 2px;
  border-radius: 2px;
}

.sep {
  color: var(--ac-fg-faint);
}
```

The `.link` resting state uses `--ac-fg` (not `--ac-fg-faint`) plus
a subtle underline tinted with `--ac-fg-faint` so the affordance is
visible without shouting; hover lifts both colour and underline to
`--ac-accent`. `:focus-visible` provides a clear keyboard-focus
indicator on persistent chrome.

Add a CSS-source raw-string assertion in `Breadcrumbs.test.tsx`
that the file contains `:focus-visible` declared on `.link`
(mirrors the existing raw-CSS assertion patterns).

### Success Criteria

#### Automated Verification

- [ ] `npm test -- src/components/Breadcrumbs/Breadcrumbs.test.tsx`
      passes (URL cases including ancestor-link href assertions +
      click-handler unit + pending unit + empty unit + dev-warn
      unit covering all five status values + CSS-source assertions
      for `margin-left: 0` selector binding and `:focus-visible`
      declaration).
- [ ] `npm run typecheck` passes. The `hasCrumb` type guard is the
      single site that asserts the crumb contract; the consumer's
      `loaderData.crumb` access is narrowed by the guard so no
      inline `as` casts on `loaderData` are needed in
      `Breadcrumbs.tsx`.
- [ ] `npm run lint` passes.

#### Manual Verification

- [ ] None at this phase (component not mounted yet).

---

## Phase 6: `OriginPill` component (own sibling directory)

### Overview

Render `useOrigin()` (from Phase 4) inside a pill with a green pulse
dot. Consumes the shared `@keyframes ac-pulse` declared in
`global.css` (Phase 1). Ships with a `prefers-reduced-motion: reduce`
override that disables the pulse animation.

Tests mock `useOrigin` via `vi.mock` per the codebase's existing
context-hook pattern — no `window.location` monkey-patching.

### Changes Required

#### 1. Test first

**File**: `skills/visualisation/visualise/frontend/src/components/OriginPill/OriginPill.test.tsx` (new)
**Changes**:

- Mock `useOrigin` per the existing pattern:
  ```ts
  vi.mock('../../api/use-origin', () => ({
    useOrigin: vi.fn(),
  }))
  ```
- For each of two host strings (`'127.0.0.1:5173'`, `'localhost:3000'`):
  set `useOrigin.mockReturnValue(host)`, render `<OriginPill />`,
  assert text content includes the host.
- CSS-source assertion: read `OriginPill.module.css?raw` and assert
  it contains `animation: ac-pulse` on the `.pulseDot` rule, and a
  `@media (prefers-reduced-motion: reduce)` block disabling the
  animation. (Replaces the `data-pulse` runtime marker.)

#### 2. OriginPill component

**File**: `skills/visualisation/visualise/frontend/src/components/OriginPill/OriginPill.tsx` (new)
**Changes**:

```tsx
import { useOrigin } from '../../api/use-origin'
import styles from './OriginPill.module.css'

export function OriginPill() {
  const host = useOrigin()
  return (
    <div className={styles.originPill} aria-label="Server origin">
      <span className={styles.pulseDot} aria-hidden="true" />
      <span className={styles.originText}>{host}</span>
    </div>
  )
}
```

(No `role="status"` — the host is static and a live region would be
spurious. The `aria-label` gives the pill an accessible name when
focused or inspected.)

#### 3. CSS

**File**: `skills/visualisation/visualise/frontend/src/components/OriginPill/OriginPill.module.css` (new)
**Changes**:

```css
.originPill {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-2);
  padding: var(--sp-1) var(--sp-3);
  border-radius: var(--radius-pill);
  border: 1px solid var(--ac-stroke-soft);
  background: var(--ac-bg-chrome);
  font-family: var(--ac-font-mono);
  font-size: 0.875rem;
  color: var(--ac-fg);
}

.pulseDot {
  width: 8px;
  height: 8px;
  border-radius: 999px;
  background: var(--ac-ok);
  animation: ac-pulse 2s ease-in-out infinite alternate;
}

@media (prefers-reduced-motion: reduce) {
  .pulseDot {
    animation: none;
  }
}
```

(Token names like `--ac-fg`, `--ac-stroke-soft`, `--ac-bg-chrome` are
present per 0033; verify exact names and adjust if 0033 settled on
slightly different identifiers.)

### Success Criteria

#### Automated Verification

- [ ] `npm test -- src/components/OriginPill/OriginPill.test.tsx`
      passes.
- [ ] `npm run typecheck` passes.
- [ ] `npm run lint` passes.

#### Manual Verification

- [ ] None at this phase (component not mounted yet).

---

## Phase 7: `SseIndicator` component (own sibling directory)

### Overview

Render a small dot whose colour and animation depend on
`useDocEventsContext().connectionState`. Four states:
`open` (green static), `reconnecting` (amber pulsing), `connecting`
(grey static), `closed` (red static). The amber pulse references the
shared `@keyframes ac-pulse` declared in `global.css` (Phase 1).
Ships with a `prefers-reduced-motion: reduce` override that disables
the pulse animation. The `aria-label` exposes a human-readable state
when focused or inspected; no `role="status"` (a persistent live
region in chrome over-announces during transient reconnects).

### Changes Required

#### 1. Test first

**File**: `skills/visualisation/visualise/frontend/src/components/SseIndicator/SseIndicator.test.tsx` (new)
**Changes**: Mock `useDocEventsContext` per the
`SidebarFooter.test.tsx:5-11,16-29` pattern. Per state:

- `open`: render; assert root has `data-state="open"` and no
  `data-animated` attribute.
- `reconnecting`: render; assert root has `data-state="reconnecting"`
  AND `data-animated="true"`.
- `connecting`: render; assert `data-state="connecting"` and no
  `data-animated` attribute.
- `closed`: render; assert `data-state="closed"` and no
  `data-animated` attribute.
- The element has an `aria-label` reflecting the state in human
  terms (e.g. "SSE connection: open"); assert the label string per
  state.

CSS-source assertions (raw-string check on `SseIndicator.module.css?raw`):

- `[data-state='open']` selector binds `background: var(--ac-ok)`.
- `[data-state='reconnecting']` selector binds `background: var(--ac-warn)`.
- `[data-state='connecting']` selector binds `background: var(--ac-fg-faint)`.
- `[data-state='closed']` selector binds `background: var(--ac-err)`.
- `[data-animated='true']` selector binds `animation: ac-pulse ...`.
- A `@media (prefers-reduced-motion: reduce)` block disables the
  animation.

#### 2. SseIndicator component

**File**: `skills/visualisation/visualise/frontend/src/components/SseIndicator/SseIndicator.tsx` (new)
**Changes**:

```tsx
import { useDocEventsContext } from '../../api/use-doc-events'
import type { ConnectionState } from '../../api/reconnecting-event-source'
import styles from './SseIndicator.module.css'

const LABELS: Record<ConnectionState, string> = {
  open: 'SSE connection: open',
  reconnecting: 'SSE connection: reconnecting',
  connecting: 'SSE connection: connecting',
  closed: 'SSE connection: closed',
}

export function SseIndicator() {
  const { connectionState } = useDocEventsContext()
  const animated = connectionState === 'reconnecting'

  // `data-animated={animated ? 'true' : undefined}` — React omits
  // attributes whose value is `undefined`, so the attribute is
  // absent from the DOM in non-reconnecting states. Do not change
  // to `String(animated)` — that would render `"false"` and break
  // the [data-animated='true'] selector.
  return (
    <span
      className={styles.sse}
      aria-label={LABELS[connectionState]}
      data-state={connectionState}
      data-animated={animated ? 'true' : undefined}
    />
  )
}
```

`LABELS` is typed `Record<ConnectionState, string>` so the compiler
enforces exhaustive coverage if `ConnectionState` ever gains a
member.

#### 3. CSS

**File**: `skills/visualisation/visualise/frontend/src/components/SseIndicator/SseIndicator.module.css` (new)
**Changes**:

```css
.sse {
  width: 8px;
  height: 8px;
  border-radius: 999px;
  display: inline-block;
}
.sse[data-state='open']         { background: var(--ac-ok); }
.sse[data-state='reconnecting'] { background: var(--ac-warn); }
.sse[data-state='connecting']   { background: var(--ac-fg-faint); }
.sse[data-state='closed']       { background: var(--ac-err); }
.sse[data-animated='true']      {
  animation: ac-pulse 2s ease-in-out infinite alternate;
}

@media (prefers-reduced-motion: reduce) {
  .sse[data-animated='true'] {
    animation: none;
  }
}
```

### Success Criteria

#### Automated Verification

- [ ] `npm test -- src/components/SseIndicator/SseIndicator.test.tsx`
      passes (state matrix + CSS-source assertions).
- [ ] `npm run typecheck` passes.
- [ ] `npm run lint` passes.

#### Manual Verification

- [ ] None at this phase (component not mounted yet).

---

## Phase 8: `Topbar` composition + RootLayout integration + SidebarFooter removal

### Overview

This phase composes the four sub-components into a single `Topbar`,
restructures `RootLayout` from a flex-row into a flex-column with the
Topbar above the body row, and removes the obsolete `SidebarFooter`.
These changes ship together so no interim commit leaves the visible
app in a broken transitional layout (a column shell with no header
would either fill 100vh with the body or leave 56 px of empty space
at the bottom; both are wrong).

The Topbar is a flex row with seven slots:
`[brand] [breadcrumbs] [spacer] [origin pill] [sse] [theme-toggle
slot] [font-mode-toggle slot]`. The two trailing slots are rendered
as plain `<div data-slot="...">` elements. While empty, the CSS rule
`.slot:empty { width: 0; height: 0; overflow: hidden; }` collapses
them; once 0034 inserts children the rule self-disengages
automatically — no attribute toggling needed.

### Changes Required

#### 1. Topbar composition test (test first)

**File**: `skills/visualisation/visualise/frontend/src/components/Topbar/Topbar.test.tsx` (new)
**Changes**: Mock `useDocEventsContext` and `useOrigin`. Render
`<Topbar />` under `<MemoryRouter>` from `src/test/router-helpers.tsx`
with the URL set to `/library/decisions`; assert:

- Brand text "Accelerator" and "VISUALISER" present.
- A `<nav aria-label="Breadcrumb">` is present (delegated to
  `Breadcrumbs`).
- Origin pill text contains the mocked host string.
- An element with `data-state="open"` exists (SSE indicator).
- A `<div data-slot="theme-toggle">` exists and has no children.
- A `<div data-slot="font-mode-toggle">` exists and has no children.

Note: `MemoryRouter` from `src/test/router-helpers.tsx` does not
support loaders, so this test does not exercise the breadcrumb
*content* — only the `<nav aria-label="Breadcrumb">` *presence*.
Breadcrumb content is asserted in Phase 5's production-routeTree
test.

CSS-source assertion: read `Topbar.module.css?raw` and assert it
contains `.slot:empty { width: 0; height: 0;` (or the equivalent
collapsed form with whitespace tolerance). Replaces the
`data-empty="true"` runtime marker.

Negative assertion (cements the SidebarFooter behaviour drop): set
`useDocEventsContext` to return `{ connectionState: 'open',
justReconnected: true, setDragInProgress: vi.fn() }` — the exact
state in which the old `SidebarFooter` would have rendered the
"Reconnected — refreshing" text — and assert
`expect(screen.queryByText(/Reconnected — refreshing/)).toBeNull()`.
Asserting against the open-non-just-reconnected default would be
trivially true (the old footer never rendered the text in that
state); driving the assertion with `justReconnected: true` is what
proves the chrome no longer responds to the post-reconnect transient.

#### 2. Topbar component

**File**: `skills/visualisation/visualise/frontend/src/components/Topbar/Topbar.tsx` (new)
**Changes**:

```tsx
import { Brand } from '../Brand/Brand'
import { Breadcrumbs } from '../Breadcrumbs/Breadcrumbs'
import { OriginPill } from '../OriginPill/OriginPill'
import { SseIndicator } from '../SseIndicator/SseIndicator'
import styles from './Topbar.module.css'

export function Topbar() {
  return (
    <header className={styles.topbar}>
      <Brand />
      <Breadcrumbs />
      <div className={styles.spacer} />
      <OriginPill />
      <SseIndicator />
      <div className={styles.slot} data-slot="theme-toggle" />
      <div className={styles.slot} data-slot="font-mode-toggle" />
    </header>
  )
}
```

(`<header>` already maps to the implicit `banner` landmark when used
at document scope, so `role="banner"` is unnecessary.)

#### 3. Topbar CSS

**File**: `skills/visualisation/visualise/frontend/src/components/Topbar/Topbar.module.css` (new)
**Changes**:

```css
.topbar {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  height: var(--ac-topbar-h);
  padding: 0 var(--sp-5);
  border-bottom: 1px solid var(--ac-stroke);
  background: var(--ac-bg-chrome);
}

.spacer { flex: 1; }

.slot:empty {
  width: 0;
  height: 0;
  overflow: hidden;
}
```

#### 4. RootLayout test (test first)

**File**: `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.test.tsx` (new)
**Changes**: Create the file. Mock `useDocEvents`, `useServerInfo`,
and `useOrigin` per house style; render `RootLayout` under
`<MemoryRouter>`; assert:

- A `<main>` element exists and contains the `<Outlet/>` content.
- A `<nav>` (the sidebar) exists.
- A `<header>` (the Topbar) is present and is a sibling of the body
  row, rendered above the body row in DOM order.

CSS-source assertion: read `RootLayout.module.css?raw` and assert
the `.root` rule declares `flex-direction: column` and `min-height:
100vh`, and the `.body` rule declares `flex: 1` (the layout
invariant: `.root` uses `min-height: 100vh` and `.body` uses
`flex: 1` inside the column).

#### 5. Sidebar test (test first)

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.test.tsx`
**Changes**: Remove any assertions that reference `SidebarFooter`
content (version text, reconnecting text, etc.). Add a negative
assertion: `expect(screen.queryByText(/Visualiser v/)).toBeNull()` —
confirms the footer is gone.

#### 6. RootLayout shell restructure + Topbar mount

**File**: `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.tsx`
**Changes**: Restructure from `<DocEventsContext.Provider><div.shell>
…</div></DocEventsContext.Provider>` to a flex column with the Topbar
above the body row. The `<DocEventsContext.Provider>` continues to
wrap the entire tree so the Topbar's `SseIndicator` reads live state.

```tsx
import { Topbar } from '../Topbar/Topbar'
// …
<DocEventsContext.Provider value={docEvents}>
  <div className={styles.root}>
    <Topbar />
    <div className={styles.body}>
      <Sidebar docTypes={docTypes} />
      <main className={styles.main}>
        <Outlet />
      </main>
    </div>
  </div>
</DocEventsContext.Provider>
```

#### 7. RootLayout CSS

**File**: `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.module.css`
**Changes**: Restructure the rules. `.root` becomes the outer flex
column with `min-height: 100vh`; `.body` (the inner row, repurposed
from the existing `.shell` rule) uses `flex: 1` so the body fills
whatever the Topbar leaves. `.main` keeps its existing flex/overflow
declarations.

```css
.root {
  display: flex;
  flex-direction: column;
  min-height: 100vh;
  font-family: var(--ac-font-body);
}

.body {
  display: flex;
  flex: 1;
}

.main {
  flex: 1;
  overflow: auto;
  padding: var(--sp-5) var(--sp-6);
}
```

(The pre-existing `.shell` rule is renamed to `.body`. If you want
to preserve git-blame fidelity, instead keep `.shell` and add a new
`.root` outer wrapper — both are acceptable.)

#### 8. Remove SidebarFooter from Sidebar

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx`
**Changes**: Delete the import at line 3 and the `<SidebarFooter />`
render at line 82.

#### 9. Sidebar CSS cleanup

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.module.css`
**Changes**: Inspect for any rules that targeted the footer (the
`margin-top: auto` was on `.footer` inside
`SidebarFooter.module.css`, so likely no `Sidebar.module.css` rules
need changing — but verify and remove anything that becomes dead).

#### 10. Delete SidebarFooter directory

**Files** (delete):

- `skills/visualisation/visualise/frontend/src/components/SidebarFooter/SidebarFooter.tsx`
- `skills/visualisation/visualise/frontend/src/components/SidebarFooter/SidebarFooter.module.css`
- `skills/visualisation/visualise/frontend/src/components/SidebarFooter/SidebarFooter.test.tsx`
- The `components/SidebarFooter/` directory itself.

### Success Criteria

#### Automated Verification

- [x] `npm test` (full suite) passes from
      `skills/visualisation/visualise/frontend/`.
- [x] `npm run typecheck` passes.
- [x] `npm run lint` passes.
- [x] `git grep -n SidebarFooter -- 'skills/visualisation/visualise/frontend/src/'`
      returns zero hits.
- [x] `git grep -n "components/SidebarFooter" -- 'skills/visualisation/visualise/frontend/src/'`
      returns zero hits.

#### Manual Verification

- [ ] `npm run dev`; visit `/library` — Topbar is visible above the
      sidebar, with brand mark, "Library" crumb only, origin pill
      reading `127.0.0.1:5173` with green pulse, SSE indicator green.
- [ ] Visit `/library/decisions` — crumbs show "Library /
      decisions" with `aria-current="page"` on "decisions"; the
      "Library" crumb is a clickable link that navigates to
      `/library`.
- [ ] Visit `/library/decisions/<some doc>` — crumbs show "Library /
      decisions / <slug>"; ancestor crumbs ("Library", "decisions")
      are clickable links that navigate up the hierarchy.
- [ ] Visit `/lifecycle/<some cluster>` — crumbs show "Lifecycle /
      <slug>".
- [ ] Visit `/kanban` — only "Kanban" crumb.
- [ ] In light mode the brand-mark gradient is visible (indigo →
      red transition).
- [ ] Stop the dev server (or temporarily kill the SSE endpoint) —
      SSE indicator turns red after the close handler fires; restart
      — indicator transitions through `connecting` (grey) →
      `reconnecting` (amber, pulsing) → `open` (green).
- [ ] Sidebar no longer shows any version / reconnecting status
      text at the bottom.
- [ ] Theme-toggle and font-mode-toggle slots occupy zero visible
      space (verify via DevTools inspector — empty divs collapsed
      via `:empty` CSS).
- [ ] Toggle the OS-level "Reduce motion" setting on; reload — both
      pulse animations are static.
- [ ] Refresh the Playwright visual baselines if needed:
      `npm run test:visual -- --update-snapshots` then re-run
      `npm run test:visual` and confirm they pass with the new
      Topbar baseline.

---

## Testing Strategy

### Unit Tests

Per phase, listed above. Headlines:

- **Token parity** (`global.test.ts`) — adds `ac-topbar-h` (under
  the new `LAYOUT_TOKENS` group) and a raw-string check that the
  shared `ac-pulse` keyframe is declared.
- **Router loaders** (`router.test.tsx`) — asserts every
  `loaderData.crumb` value via `router.state.matches`, using the
  same `waitForPath` helper for redirect-chain settling.
- **`resolveCrumb()` pure function** (`router-with-crumb.test.ts`,
  new) — direct unit covering the static and resolver-based crumb
  shapes in isolation from any router or `LoaderFnContext`. The
  `withCrumb` integration with TanStack is covered transitively
  by `router.test.tsx`'s loader-crumbs block.
- **Shared router fixtures** (`src/test/router-fixtures.ts`, new) —
  consolidates `setupRouterFixtures()` (only the unconditional
  fetch stubs), `buildRouter({ url })`, and `waitForPath` so
  `router.test.tsx` and `Breadcrumbs.test.tsx` share one fixture
  surface. Per-test fetches (`fetchDocs`, `fetchLifecycleCluster`,
  kanban config) remain explicit per-test setup so test intent is
  locally visible.
- **`useOrigin` hook** (`use-origin.test.ts`) — read-once-per-mount
  contract verified by spying on `window.location.host`'s getter
  and asserting one call across multiple `rerender()`s, plus a
  stability-under-mutation case.
- **Component tests** — narrow, mock-only-what-you-need; each
  sub-component lives in its own sibling directory:
  - `Brand.test.tsx` — pure render, no hooks, no context.
  - `Breadcrumbs.test.tsx` — production `routeTree` via
    `setupRouterFixtures()` plus per-URL fetch stubs per the
    enumerated table in Phase 5 §0; URL matrix asserts both crumb
    text and ancestor-link `href` per case via
    `getByRole('link', { name })` (trailing crumb is `<span
    aria-current="page">`, ancestors are `<a href>`); click-handler
    unit covering plain-click → `router.navigate` and ⌘-click →
    native href; pending-state and empty-list units; dev-warn unit
    covering all five `m.status` values plus the rootRouteId
    exemption; raw-CSS source assertions for the flush-margin
    contract (selector-bound regex) and `:focus-visible` declaration.
  - `OriginPill.test.tsx` — mocks `useOrigin`; raw-CSS source
    assertion for `ac-pulse` reference and reduced-motion override.
  - `SseIndicator.test.tsx` — `useDocEventsContext` mocked per
    state; raw-CSS source assertions for the four `data-state`
    colour bindings, the `data-animated` animation binding, and
    the reduced-motion override.
  - `Topbar.test.tsx` — composition smoke test under
    `<MemoryRouter>`; asserts presence of all parts and the
    `:empty`-collapsed slot CSS rule via raw-string check; negative
    assertion driven with `justReconnected: true` to cement that
    the post-reconnect transient text is gone.
- **RootLayout test** (`RootLayout.test.tsx`, new) — full shell
  including the Topbar above the body row; raw-CSS source assertion
  for the column-direction and `flex: 1` body invariants.
- **Sidebar test** (`Sidebar.test.tsx`) — pruned of footer
  assertions; gains a negative assertion confirming the version
  label is gone.

### SidebarFooter behaviour: intentional coverage drops

The previous `SidebarFooter.test.tsx` covered six assertions:
version label render, missing version, `Reconnecting…` text,
`Reconnected — refreshing` text, the `justReconnected + open`
precedence rule, and `Visualiser v${version}`. After Phase 8 these
behaviours are intentionally not represented anywhere:

- `Reconnecting…` text → replaced by `SseIndicator`'s
  `data-state="reconnecting"` + amber pulse + `aria-label="SSE
  connection: reconnecting"` (asserted in `SseIndicator.test.tsx`).
- `Reconnected — refreshing` text → dropped from the chrome layer
  by design. `Topbar.test.tsx` adds a negative assertion confirming
  no such text appears. The `justReconnected` field is retained on
  `DocEventsHandle` for a possible future toast notification surface
  (see "What We're NOT Doing").
- Version label → dropped by design. `useServerInfo()` is retained
  for a possible future About panel.

These drops are deliberate; the Sidebar test's negative assertion
plus `Topbar.test.tsx`'s negative assertion together cement that
neither piece of text appears anywhere in the chrome.

### Integration Tests

The router test (`router.test.tsx`) already covers route
resolution end-to-end and will gain loader assertions. The
Breadcrumbs test reuses the same `routeTree` import, so loader and
render are exercised against a single source of truth. No new
integration test file is added.

### Manual Testing Steps

Listed under Phase 8 Manual Verification above. The full-route
walkthrough plus the SSE state cycle plus the reduced-motion
toggle is the acceptance evidence for AC1, AC4, AC5, AC6.

## Performance Considerations

- The Topbar mounts once per session via `RootLayout` — no
  per-route remount cost.
- `useMatches()` re-runs on every navigation but breadcrumb
  filtering is O(matches) — bounded at 4-5 in this app.
- One shared CSS keyframe animation (`ac-pulse` declared in
  `global.css`) is introduced and consumed by `OriginPill`'s green
  dot and `SseIndicator`'s amber state. It animates opacity only —
  no layout, no compositor pressure beyond that. A
  `prefers-reduced-motion: reduce` override disables it for users
  who opt out at the OS level.
- `useOrigin()` reads `window.location.host` exactly once at mount
  via `useState`'s lazy initialiser, so the host string identity is
  stable across the session and `OriginPill` re-renders are cheap.

## Migration Notes

### Component organisation

The four sub-components (`Brand`, `Breadcrumbs`, `OriginPill`,
`SseIndicator`) live in their own sibling directories under
`components/`, following the project's established
one-component-per-directory convention. Each is independently
exported and could in principle be reused elsewhere; the Topbar
imports them from their sibling paths. There is no nested
private-sub-component pattern in this story.

### 0034 slot contract

0034 will populate the two `data-slot` divs by inserting children
inside them. Their positions in the Topbar's flex layout are fixed
by this story; 0034 must NOT relocate them. The CSS rule
`.slot:empty { width: 0; height: 0; overflow: hidden }`
self-disengages as soon as 0034 inserts a child element, so 0034
does not need to clear any attribute or class — just render content.

### 0036 sidebar redesign

0036 assumes the Topbar exists for chrome separation; this story
owns the SidebarFooter removal. **0036 is also the natural home for
slug prettification in breadcrumbs** — the work item AC notes
"pulling document titles from loader data where available". This
story ships literal slug crumbs; 0036 should replace them with
loader-fetched titles (e.g. by calling `queryClient.ensureQueryData`
inside the loader and reading the front-matter title).

### 0041 library overview

0041 (Library page wrapper) will replace the `/library` redirect
with a real `LibraryOverview` component. This story already
attaches the `Library` crumb to `libraryRoute` via `withCrumb`, so
0041 inherits the crumb without further loader changes.

### Future pulse callers

The shared `ac-pulse` keyframe lives in `global.css` so any future
chrome element can reference it by name. Each caller is responsible
for shipping its own `prefers-reduced-motion: reduce` override.

## References

- Original work item: `meta/work/0035-topbar-component.md`
- Research: `meta/research/codebase/2026-05-07-0035-topbar-component.md`
- Related work items: `meta/work/0033-design-token-system.md`
  (shipped foundation), `meta/work/0034-theme-and-font-mode-toggles.md`
  (consumes our slots), `meta/work/0036-sidebar-redesign.md`,
  `meta/work/0041-library-page-wrapper-and-overview-hub.md`
- Pass-2 review of 0035:
  `meta/reviews/work/0035-topbar-component-review-1.md`
- Design inventories:
  - `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/`
  - `meta/research/design-inventories/2026-05-06-135214-current-app/`
- Design gap source:
  `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Anchoring code:
  - `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.tsx:17-26`
  - `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx:3,82`
  - `skills/visualisation/visualise/frontend/src/router.ts:19-119`
  - `skills/visualisation/visualise/frontend/src/api/use-doc-events.ts:15-19,165-175`
  - `skills/visualisation/visualise/frontend/src/styles/global.css:75-78,154-155,184-185`
  - `skills/visualisation/visualise/frontend/src/test/router-helpers.tsx`
  - `skills/visualisation/visualise/frontend/src/router.test.tsx:1-152`
  - `skills/visualisation/visualise/frontend/src/components/SidebarFooter/SidebarFooter.test.tsx:5-29`
