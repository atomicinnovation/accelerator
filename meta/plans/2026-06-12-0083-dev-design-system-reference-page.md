---
type: plan
id: "2026-06-12-0083-dev-design-system-reference-page"
title: "DevDesignSystem Reference Page Implementation Plan"
date: "2026-06-12T23:10:21+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0083"
parent: "work-item:0083"
derived_from: ["codebase-research:2026-06-12-0083-dev-design-system-reference-page"]
tags: [design, frontend, dev-tools, visualiser, design-system, visual-regression, theming, scroll-spy, keybind, icon]
revision: "e13cea2829821da417f7d7e91e9c2bb7ba8b3852"
repository: "accelerator"
last_updated: "2026-06-13T09:24:53+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

> **Revision note (2026-06-13).** Updated after plan review-1
> (`meta/reviews/plans/2026-06-12-0083-dev-design-system-reference-page-review-1.md`,
> verdict REVISE). Key changes: the `/dev` route model is retained but hardened
> (in-context router hooks instead of the singleton, `replace:true` alias
> normalisation, prior-path captured on every non-`/dev` location change, an
> explicit re-entrancy guard); the Code-blocks section now ports all 8 VR
> languages (was TS+Bash); accessibility, documentation, and AC-amendment gaps
> closed. `DevDesignSystem.tsx` stays a single file by decision; only the
> scroll-spy active-section computation is extracted as a pure, unit-testable
> helper. Pass-2 polish (after re-review): doc-type glyph sizes converge on the
> codebase's `16/24/32` + a net-new `48` (not the prototype's 22/28/36/48, so glyph
> baselines stay stable); the exit-target prior path uses **session-scoped** storage
> (not the localStorage-backed `safe-storage`); `enterDev()` navigates directly to
> `/dev` (the hash bridge handles only external alias URLs); the re-entrancy guard
> records only the bridge's own writes; and `DEV_CHORD_HINT` is platform-resolved.

# DevDesignSystem Reference Page Implementation Plan

## Overview

Implement a single consolidated `DevDesignSystem` reference page in the
visualiser frontend that reproduces all 24 design-system sections from the
prototype `view-dev.jsx` at content fidelity, in both light and dark themes.
The page is an **uncrumbed `/dev` TanStack route** activated through a
**hash bridge** (`#dev` / `#dev/<section>`), a `Cmd/Ctrl+Shift+L` keychord, or
a sidebar-foot triple-click. It carries a scroll-spy TOC that drives both the
active-section highlight and the URL hash, an in-page theme toggle, and the
dev-page chrome (DEV marquee, CONTENTS aside, exit-to-app, footer). The five
existing dev showcase routes are retired and their Playwright
visual-regression coverage migrated onto the corresponding sections. Along the
way we build the missing unified `Icon` primitive (33 names) and refactor the
~18 duplicated hand-written app icons onto it.

This is an epic-sized story kept as one work item because its strands are
indivisible (VR migration and per-section fidelity both need the sections to
exist first). The plan decomposes it into **11 independently
mergeable phases**, each leaving `main` green and shippable. Work is
test-driven throughout.

## Current State Analysis

The shipping app is TypeScript/TSX under
`skills/visualisation/visualise/frontend/`. Research
(`meta/research/codebase/2026-06-12-0083-dev-design-system-reference-page.md`)
established:

- **Routing is purely path-based** (`createRouter({ routeTree })`,
  `src/router.ts:218`); there is **zero** hash handling, no `hashchange`
  listener, and no `notFoundComponent`. The five showcase routes are
  registered as children of `rootRoute` (`src/router.ts:152-216`), so they
  already render inside `RootLayout`'s full provider tree.
- **The keybind pattern is hand-rolled** — the global `/` handler at
  `RootLayout.tsx:52-63` with `isPlainSlashKey` (`:20-28`) and
  `isEditableTarget` (`:30-39`) guards; tested in the `"global / keybind"`
  block at `RootLayout.test.tsx:73-196` (the `preventDefault` template is
  `:181-195`).
- **`<main className={styles.main}>` is the sole scroll container**
  (`RootLayout.tsx:94-96`, `RootLayout.module.css:24-39`); the window does not
  scroll. It is the only `<main>` element, selectable by tag.
- **No unified `Icon` / `ICON_NAMES` exists.** 18 of the prototype's 33 names
  exist as scattered, duplicated, geometry-drifting inline `<svg>`s (e.g.
  `chevron-right` has 5 copies; `search`/`kanban`/`check` differ in
  stroke-width); 15 are absent. The prototype `ui.jsx:8-53` holds all 33 path
  definitions to port.
- **Component shape mismatches** vs the prototype: `Brand` has no props (fixed
  28px + wordmark, hard-coded gradient `id="hexg"`); `Glyph` `size` union is
  `16|24|32`; `PipelineMini` takes `completeness={{present}}` (8 workflow
  stages, **no** `stages`/`present`/`compact` props); status/verdict/result are
  **three** components; live radii are a px-ladder (`--radius-0..-12`,
  `--radius-pill`) not `sm/md/lg`; there is no `--shadow-brand` (prototype
  "brand" = `--shadow-card`); `--ac-violet` is light-only; code blocks
  deliberately omit traffic-light dots; no `⌘K` kbd and no live tier-pill
  "default" state exist.
- **VR surface = 9 specs** (5 pixel + 4 resolved) under
  `tests/visual-regression/`. Baselines are foldered by **spec filename**
  (renaming orphans them unless the folder is `git mv`d) and committed per-OS
  (`darwin` + `linux`). Themes loop internally per-spec by writing
  `document.documentElement.dataset.theme`. `code-syntax-showcase.spec.ts`
  uses a **full-page** screenshot that must become a clipped locator.
- **Theme infra is reusable end-to-end**: `useThemeContext().toggleTheme()`
  writes `data-theme` on `<html>` (`use-theme.ts:38-40`); `<ThemeToggle/>`
  works anywhere under `RootLayout`'s providers.

## Desired End State

- Navigating to `#dev` / pressing `Cmd/Ctrl+Shift+L` / triple-clicking the new
  sidebar-foot element activates the `/dev` route rendering all 24 sections. The
  canonical URL is `/dev#<section>` (e.g. `/dev#colors`; `/dev` for overview);
  `#dev` and `#dev/<section>` are accepted **activation aliases** that the bridge
  normalises to that form.
- Each of the 24 sections reproduces the prototype's full variant set, ported
  to live tokens/components, with automated per-section variant-count /
  presence assertions binding to **live constants** where applicable.
- The page renders in light and dark; an in-page `<ThemeToggle/>` flips
  `data-theme` and the `--ac-*` values; per-section light + dark VR snapshots
  are the correctness oracle.
- A scroll-spy TOC drives the active highlight from actual scroll position and
  rewrites the hash to `#<active-section>`; landing on `/dev#<section>` (or the
  `#dev/<section>` alias) scrolls there; the highlight is never pinned.
- The five showcase routes are removed (former paths no longer resolve to a
  showcase); their VR specs are renamed to `dev-design-system-*`, repointed at
  the corresponding sections, and baselines regenerated.

> **Acceptance-criteria note.** The canonical `/dev#<section>` form (chosen for
> URL cleanliness over `/dev#dev/<section>`) departs from the work item's literal
> hash wording: AC3 ("`location.hash` starting with `#dev`"), AC5 ("hash updates
> to `#dev/<active-section>`"), and AC7 ("clearing the `#dev` hash"). These
> clauses are **amended as an owned step in Phase 11** to the route + bare-section
> form — activation is observable as path `/dev` + the DEV marquee/24-section TOC,
> the scroll-spy writes `#<section>`, and exit clears the section hash while
> restoring the prior path. `#dev`/`#dev/<section>` survive as activation aliases.
> Phase 11 also reconciles the work item's "returns 404" wording (see *What We're
> NOT Doing* — the app is a CSR SPA with no `notFoundComponent`, so a retired path
> resolves to the SPA not-found UI served over HTTP 200, not a true HTTP 404) and
> the stale count parentheticals in the oracle table (Stage dots **8** not 9,
> Doc-type glyphs **13** not 12), which the live-constant-bound assertions already
> use.
- A unified `Icon` primitive (33 names) exists and the app's duplicated inline
  icons are refactored onto it.
- The frontend README's developer-routes docs list `DevDesignSystem` and drop
  all five showcases.

### Key Discoveries

- The prototype's own model is a `hashchange`-driven `devMode` flag
  (`Accelerator Visualiser.html:102,146-197`); the chosen live design **bridges
  the `#dev` activation aliases to a real `/dev` route** (`router.navigate`),
  normalising them to the canonical `/dev#<section>` form and storing the prior
  path for exit.
- The scroll-spy tuning ports verbatim — `rootMargin: "-80px 0px -55% 0px"`,
  `threshold: [0, 0.25, 0.5]` (`view-dev.jsx:103-122`) — but the hash write
  becomes the bare `#<id>` form (the prototype wrote `#dev/<id>`), and the
  active-section pick must be recomputed from **all** observed positions (not
  single-dispatch highest-ratio) to avoid the prototype's pinned-to-Colours
  defect.
- The dev-page chrome is ~550 lines of `ds-*` CSS at `app.css:1392-1943`,
  re-authorable as a CSS module over live `--ac-*` tokens. The marquee animates
  the inner block `-50%` (content must be duplicated for a seamless loop);
  `.ds-spec*` classes referenced in the brief are **not** defined in the
  prototype CSS (ignore them).
- `history.replaceState` does **not** fire `hashchange`, so the scroll-spy's
  hash rewrites will not re-trigger the activation bridge.
- The showcases already render **live components**, so faithfully reproduced
  cells (same testids, sizes, clip targets) keep most baselines stable. The
  Doc-type-glyphs section converges on the **codebase's existing `16/24/32` sizes**
  (so the migrated glyph pixel baselines are unchanged) **plus a net-new `48` cell**;
  only that new `48` cell and the code-syntax clip genuinely change/add pixels.

## What We're NOT Doing

- **Not** switching the app to hash-history; the `/dev` route uses normal
  browser history with a hash bridge.
- **Not** adding a global `notFoundComponent`. The app is a pure client-side SPA
  with no `notFoundComponent` configured, so a retired path resolves to TanStack's
  **default SPA not-found UI served over HTTP 200** — not a true HTTP 404. "Returns
  404" is therefore satisfied as "no longer resolves to a showcase (no redirect)",
  asserted by a router test; the work item AC9 wording is amended to match in
  Phase 11. A generic HTTP client requesting a former path still gets 200 +
  `index.html` (acceptable — nothing in-app or external links to these dev-only
  paths).
- **Not** authoring a 25th "root-cause-analyses" section — RCA is a prototype-only
  doc *type* that appears inside the glyph/stage/hue sections; the live app has
  13 doc types (`DOC_TYPE_KEYS`).
- **Not** adding a `compact` mode to `PipelineMini` (no live equivalent — the
  Stage-dots section omits it, documented as a deviation).
- **Not** refactoring the non-mapping custom icons (Toaster info, copy/clipboard,
  font-mode "T", the framed-library wrapper) or the doc-type `Glyph` system onto
  the new `Icon` primitive — only the ~18 icons that map to the 33 names.
- **Not** chasing byte-identical VR baselines — coverage is preserved; baselines
  are regenerated (they drift darwin/linux regardless).

## Implementation Approach

Build bottom-up: shared primitives first (`Icon`, `AtomicMark`, `Glyph` union),
then activation + chrome, then scroll-spy + theme, then the 24 sections in four
content batches, then VR migration, then route retirement + docs. Each phase
is independently mergeable; the dependency order is
1→2 (icon migration after primitive), 3, 4→5 (chrome before scroll-spy),
6–9 (content, after chrome), 10 (VR, after the sections it asserts), 11
(retirement, after VR repoints off the old routes). Phases 2, 3, 6–9 can land
in any order within their tier. TDD: write the unit/e2e assertion, then the
implementation, for every behavioural change.

A single shared module owns the cross-cutting constants:

```typescript
// src/components/DevDesignSystem/dev-constants.ts
// DEV_ALIAS_RE matches ONLY the activation-alias forms (#dev, #dev/<section>),
// never the canonical bare #<section> hash — so the bridge ignores scroll-spy
// hash writes. INVARIANT: no DEV_SECTIONS id may begin with "dev" at a word
// boundary, or its canonical #dev… hash would be mis-classified as an alias
// (asserted by a unit test — see Phase 4 success criteria).
export const DEV_ALIAS_RE = /^#\/?dev(\b|\/|$)/;       // activation alias: #dev, #dev/<section>
export const aliasSection = (h: string) => h.replace(/^#\/?dev\/?/, ""); // "" | "<section>"
// NB: if the cross-browser matrix (Manual Testing step 5) forces the fallback,
// the ONLY change is DEV_CHORD + DEV_CHORD_HINT here — re-run the matrix on any edit.
export const DEV_CHORD = { code: "KeyL", shift: true, meta: true, ctrl: true } as const;
// Platform-resolved so non-mac maintainers see a pressable chord; marquee, footer,
// the DEV console hint, AND the README all bind to this one value (never a
// hardcoded ⌘⇧D). The chord itself binds meta||ctrl, so the hint must follow suit.
export const DEV_CHORD_HINT = isMac() ? "⌘⇧L" : "Ctrl+⇧+L";
export const DEV_SECTIONS = [
  { id: "overview", label: "Overview" },
  { id: "colors", label: "Colours" },
  // … all 24, ids = prototype slugs (colors, radii, glyphs, bigglyphs, mark,
  //   badges, stagedots, tierpills, form, nav, table, code, empty, toast)
] as const;
```

The chord matches on `event.code === "KeyL"` (physical key, layout-independent)
rather than `event.key` (which is the layout/Shift-sensitive `"l"`/`"L"`), so the
chord is reachable on non-US keyboard layouts.

---

## Phase 1: `Icon` primitive + `ICON_NAMES` registry

### Overview

Create a unified stroke-icon primitive porting all 33 prototype paths. This is
the foundation for the Icons section (Phase 7) and the app-wide icon refactor
(Phase 2). Fully independent — nothing else need exist.

### Changes Required

#### 1. Icon component

**File**: `src/components/Icon/Icon.tsx` (new), `Icon.module.css` (new),
`Icon.constants.ts` (new — matches the component-adjacent `Glyph.constants.ts` /
`BigGlyph.constants.ts` naming convention)
**Changes**: Port the prototype `Icon` (`ui.jsx:8-53`) — a name→path-data map
and a component rendering `<svg viewBox="0 0 24 24" fill="none"
stroke="currentColor" strokeWidth={2} strokeLinecap="round"
strokeLinejoin="round" aria-hidden={true} width={size} height={size}>`
(explicit `aria-hidden={true}`, matching every sibling inline SVG and `Glyph`'s
spread — not the bare-shorthand form, which a `aria-hidden="true"` grep gate would
miss). Export `ICON_NAMES` (the 33-name tuple) and `IconName` type. Carry a
**consumer-contract docstring** mirroring `Glyph`'s (don't override `fill`; tint
via CSS `color`; don't double-wrap the `<svg>`; `ariaLabel` semantics; stroke
icons are intentionally free-scaling, see below) and a **DEV-only `console.warn`
listing valid `ICON_NAMES`** when an unknown `name` is passed (TypeScript blocks
it at compile time, but a dynamic string caller otherwise gets a silent blank).

```typescript
export const ICON_NAMES = [
  "search","library","kanban","lifecycle","activity","clock","link",
  "chevron-right","chevron-down","chevron-left","doc","edit","close","check",
  "dot","plus","minus","git-pr","git-branch","filter","sort","sparkle","hex",
  "shield","moon","sun","settings","terminal","arrow-right","flag","folder",
  "layers","alert",
] as const;
export type IconName = (typeof ICON_NAMES)[number];

export interface IconProps {
  name: IconName;
  size?: number;          // default 16; intentionally an open number (stroke
                          // icons are freely scalable), unlike Glyph's curated
                          // size union — documented in the docstring so the
                          // divergence reads as deliberate, not an oversight
  ariaLabel?: string;     // presence flips aria-hidden → role="img"
  className?: string;
}
```

Match the live authoring convention exemplified by
`SortPill.tsx:110-126` / `FilterPill.tsx:178-194` (24×24 viewBox, currentColor,
2px rounded). Several prototype paths are byte-identical to live ones
(`chevron-down`, `filter`, `sort`), confirming the convention.

**Framed eyebrow icons**: `IconProps` has **no** `framed` option (unlike `Glyph`)
— framing stays a composition concern. The two eyebrow icons that render inside an
`IconFrame` tinted square (`LifecycleEyebrowIcon`, `KanbanEyebrowIcon`) are
migrated in Phase 2 by rendering `<Icon name="lifecycle"/>` / `<Icon
name="kanban"/>` **inside the retained `IconFrame` wrapper** (not bare), so the
established framed treatment is preserved while still consolidating the inner
glyph onto the primitive.

### Success Criteria

#### Automated Verification

- [x] Type-check + lint pass: `mise run frontend:check`
- [x] Unit tests pass: `mise run test:unit:frontend`
- [x] Test asserts `ICON_NAMES.length === 33` and every name renders an `<svg>`
- [x] Test asserts `size` prop sets `width`/`height`; default is 16
- [x] Test asserts `stroke="currentColor"` on the root `<svg>` (tints via CSS `color`)
- [x] Test asserts the decorative default emits explicit `aria-hidden="true"`, and
  `ariaLabel` flips it to `role="img"`+`aria-label`
- [x] Test asserts an off-registry `name` emits the DEV `console.warn` (and renders
  nothing/blank rather than throwing)

#### Manual Verification

- [ ] Rendering all 33 icons in a scratch story shows no missing/broken glyphs
  and consistent 2px stroke weight (covered by the Icons section in Phase 7)

---

## Phase 2: Refactor duplicated app icons onto `Icon`

### Overview

Replace the ~18 duplicated hand-written inline SVGs across the app with
`<Icon name=…>`, picking canonical geometry from the new primitive, and delete
the dead per-route icon modules. Independently mergeable; the riskiest phase
for VR drift (normalising geometry/stroke-width changes rendered pixels).

### Changes Required

#### 1. Migrate mapping consumers

**Files** (replace inline `<svg>` with `<Icon name=…>`):
- `components/Sidebar/Sidebar.tsx` — `SearchIcon`→`search`, `KanbanIcon`→`kanban`,
  `LifecycleIcon`→`lifecycle`, `CloseIcon`→`close`
- `components/FilterPill/FilterPill.tsx` — `SearchIcon`→`search`,
  `FilterIcon`→`filter`
- `components/SortPill/SortPill.tsx` — `SortIcon`→`sort`,
  `ChevronDownIcon`→`chevron-down`, `CheckIcon`→`check`
- `components/SseIndicator/SseIndicator.tsx` — `activity`
- `components/ThemeToggle/ThemeToggle.tsx` — `moon`/`sun`
- `components/Toaster/Toaster.tsx` — ok→`check`, error→`alert`, dismiss→`close`
  (leave the default **info** icon custom — no prototype name)
- `components/MarkdownRenderer/MarkdownRenderer.tsx` — task-list `CheckIcon`→`check`
- `components/Breadcrumbs/Breadcrumbs.tsx`,
  `RelatedArtifacts/RelatedArtifacts.tsx`,
  `RelatedCluster/RelatedCluster.tsx`,
  `SearchResultsPanel/…`, `routes/library/LibraryTemplatesIndex.tsx` —
  the 5 `chevron-right` copies → `chevron-right`
- `routes/lifecycle/icons.tsx` — `ClockIcon`→`clock`,
  `ChevronRightIcon`→`chevron-right`; `LifecycleEyebrowIcon` → `<IconFrame><Icon
  name="lifecycle"/></IconFrame>` (keep the frame wrapper, migrate the inner glyph)
- `routes/kanban/icons.tsx` — `ActivityIcon`→`activity`, `LinkIcon`→`link`;
  `KanbanEyebrowIcon` → `<IconFrame><Icon name="kanban"/></IconFrame>` (keep the
  frame wrapper, migrate the inner glyph)
- `components/DetailHeaderActions/OpenInEditorButton.tsx` — `editGlyph`→`edit`
- `routes/library/LibraryOverviewHub.tsx` — `LibraryIcon`→`library` (keep the
  framing wrapper)

**Contract preservation**: the migrated icons stay decorative (`aria-hidden`,
label carried by the parent button/toast text) — none gains an `ariaLabel`. Verify
the existing test contracts survive the swap: `ThemeToggle`'s `data-icon="moon"`/
`"sun"` lives on the parent `TopbarIconButton` (not the SVG), so the swap is safe;
the `Toaster` ok/error/dismiss icons must keep their role/dismiss semantics. Add a
Phase 2 note confirming these contracts are unchanged so the a11y tests keep
passing.

**Leave custom** (no prototype-name mapping): `Brand.tsx` hex mark (→ Phase 3
`AtomicMark`), Toaster info, `CopyPathButton` copy, `FontModeToggle` font icon,
doc-type `Glyph` system.

#### 2. Delete dead modules

Remove `routes/lifecycle/icons.tsx` and `routes/kanban/icons.tsx` once their
exports are unused (or trim to only the still-custom ones).

#### 3. Regenerate affected VR baselines

Any pixel spec whose rendered icon geometry/stroke-width changes
(e.g. `library-doc-view.spec.ts`, `task-list-visual.spec.ts`) needs baselines
regenerated. Resolved-colour specs (`aside-row`, `detail-eyebrow`,
`eyebrow-unification`) assert computed colour only and are unaffected.

### Success Criteria

#### Automated Verification

- [ ] Type-check + lint pass: `mise run frontend:check`
- [ ] Unit tests pass: `mise run test:unit:frontend`
- [ ] No remaining ad-hoc `chevron-right`/`search`/`sort` inline `<svg>` in the
  migrated files (grep gate in review)
- [ ] Targeted geometry unit assertions on a few canonical migrated icons (e.g.
  `chevron-right` path data + `strokeWidth=2`) so the refactor's correctness does
  not rest solely on a human reading regenerated pixel diffs (baseline regen blesses
  whatever rendered — it cannot catch a wrong stroke-width or a swapped icon)
- [ ] E2E + VR pass with regenerated baselines: `mise run test:e2e:visualiser`

#### Manual Verification

- [ ] Sidebar, topbar, sort/filter pills, toasts, breadcrumbs, related rows
  render visually unchanged (or intentionally crisper) in light + dark
- [ ] Regenerated VR diffs reviewed — only expected icon-geometry changes

---

## Phase 3: `AtomicMark` extraction + `Glyph` size union

### Overview

Two small, independent shared-component changes the dev page's Atomic-mark and
Doc-type-glyph sections depend on. No dev page yet.

### Changes Required

#### 1. Parameterised AtomicMark

**File**: `components/AtomicMark/AtomicMark.tsx` (new); refactor
`components/Brand/Brand.tsx` to consume it.
**Changes**: Extract the hex mark SVG (`Brand.tsx:6-35`) into an `AtomicMark`
taking `size?: number` (default 28) and rendering a **unique gradient id per
instance** (e.g. `useId()`-suffixed) so multiple sizes on one page don't
collide. `Brand` renders `<AtomicMark size={28}/>` + the wordmark — its output is
**pixel-identical** (NOT byte-identical: the per-instance `useId()` changes both
the `<linearGradient id>` and its `url(#…)` reference, and `useId()` ids are
non-deterministic across renders). Fills stay `var(--ac-accent*)` so the on-night
variant flips with `data-theme` for free. The app is pure CSR (no SSR/hydration),
so `useId()` carries no server/client id-mismatch risk.

#### 2. Widen Glyph size union

**File**: `components/Glyph/Glyph.tsx:41`
**Changes**: Widen `size: 16 | 24 | 32` → `16 | 24 | 32 | 48` — **converge on the
codebase's existing sizes and add only the one new size (`48`)** the Doc-type-glyphs
section needs for a four-size ramp, rather than switching to the prototype's
`22/28/36/48` set. This keeps every existing glyph consumer and its VR baselines at
`16/24/32` unchanged; only `48` is net-new. The docstring (`:71-72`) blesses
widening; nothing else is size-gated. Update the `/glyph-showcase` `SIZES` tuple
only if still referenced (it is removed in Phase 11).

### Success Criteria

#### Automated Verification

- [ ] Type-check + lint pass: `mise run frontend:check`
- [ ] Unit tests pass: `mise run test:unit:frontend`
- [ ] `AtomicMark` test: distinct gradient ids when two are rendered; `size`
  sets width/height
- [ ] `Brand` regression test asserts pixel/structural equivalence (no markup
  snapshot of the literal gradient id — assert structure or normalise the
  non-deterministic `useId()` id)
- [ ] `Glyph` specimen test renders at the added `48` size without type error
- [ ] Existing glyph VR specs still pass (no rendered change at 16/24/32 — only `48`
  is net-new)

#### Manual Verification

- [ ] Atomic mark renders at 20/24/32/48/72 and on a dark backdrop without
  gradient-id collision artefacts

---

## Phase 4: `/dev` route, activation bridge, chrome shell

### Overview

Register the uncrumbed `/dev` route rendering a `DevDesignSystem` **shell**
(marquee + 24-entry TOC aside + footer + exit-to-app; sections are empty stubs).
Build the hash→route activation bridge, the `Cmd/Ctrl+Shift+L` keychord, and the
sidebar-foot triple-click. At phase end, all three triggers activate a working
(contentless) dev page and exit restores the prior route.

### Changes Required

#### 1. Route registration

**File**: `src/router.ts`
**Changes**: Add an uncrumbed `devRoute = createRoute({ getParentRoute: () =>
rootRoute, path: "/dev", component: DevDesignSystem })`; add to `routeTree`
(alongside, then ahead of, the soon-removed showcase routes).

#### 2. Activation bridge hook

**File**: `src/components/DevDesignSystem/use-dev-activation.ts` (new); mount in
`RootLayout`. **Use the in-context `useRouter()`/`useNavigate()` hooks — NOT the
exported `router` singleton.** `RootLayout` is `rootRoute`'s component, so
importing the singleton here would form a `router.ts → RootLayout →
use-dev-activation → router.ts` cycle (TDZ-`undefined` risk); the codebase already
avoids this (Breadcrumbs uses `useRouter()`). The hook exposes the activation
surface — `enterDev()`, `exitDev()`, `isDevActive` — that the chord, Escape, and
triple-click all call, so the "am I in dev?" predicate lives in one place.

```typescript
const navigate = useNavigate();
const router = useRouter();

// Capture the prior path on EVERY non-/dev location change — not only on alias
// entry — so exit-restore is correct for direct /dev entry too (a router
// subscription on router.state.location). Persisted in SESSION-scoped storage
// (per-tab; a guarded safeSessionGet/Set wrapper, see below); validate it still
// resolves before restoring; fall back to "/library".

function enterDev() {                              // chord + triple-click
  navigate({ to: "/dev" });                        // navigate directly — no hash round-trip
}

function sync() {                                  // mount + "hashchange": EXTERNAL alias URLs only
  if (!DEV_ALIAS_RE.test(location.hash)) return;   // canonical bare /dev#<section> — no action
  const section = aliasSection(location.hash);     // "" for #dev, "colors" for #dev/colors
  // replace:true → the alias URL never enters history, so Back does not bounce
  // back through the bridge (a pushState here would trap the browser Back button).
  navigate({ to: "/dev", hash: section || undefined, replace: true });
}

function exitDev() {
  const prior = resolvablePrior() ?? "/library";   // validate stored path still resolves
  navigate({ to: prior, hash: undefined });
}
```

So `/library#dev/colors` → `/dev#colors` (prior `/library` already captured by the
location subscription); a direct `/dev#colors` is left untouched; a stray
`/dev#dev/colors` self-heals to `/dev#colors`; pressing Back after an alias entry
does **not** re-trigger activation (the alias URL was replaced, not pushed).
`enterDev()` (chord, triple-click) navigates **directly** to `/dev` rather than
round-tripping through `location.hash`, so the `hashchange` bridge handles only
genuinely external alias URLs (typed/pasted/bookmarked `#dev` / `#dev/<section>`).

The prior path is persisted in **session-scoped** storage — per-tab is the correct
lifetime for an exit target, so a fresh window's cold-load deep-link never restores
another tab's or another session's route. Use a small guarded `safeSessionGet/Set`
wrapper mirroring `safe-storage`'s exception-swallowing (the existing `safe-storage`
helpers are `localStorage`-backed, so a `sessionStorage` variant is added), with the
key in `storage-keys.ts`; the in-memory value is authoritative while mounted, the
store only seeds it on cold load.

`navigate` (history `pushState`/`replaceState`) and the scroll-spy's
`history.replaceState` do **not** fire `hashchange`, so none re-enters `sync()`. A
**re-entrancy guard** additionally protects the invariant: the bridge records the
hash values **it itself writes** (the `sync()` / scroll-spy `replaceState` writes) in
a "last programmatic hash" ref and bails if a `hashchange` reports that same value,
so a future switch to `location.hash = …` cannot reintroduce the activation loop.
Because `enterDev()` no longer writes the hash, there is no risk of the guard
suppressing activation. A unit test asserts both that a programmatic re-write is
ignored AND that an external alias `hashchange` still activates.

#### 3. Keychord

**File**: `RootLayout.tsx` (second `useEffect`, after the `/` handler)
**Changes**: Add a `keydown` handler matching `(e.metaKey || e.ctrlKey) &&
e.shiftKey && e.code === "KeyL"`, reusing `isEditableTarget`; `e.preventDefault()`;
toggle via the activation hook — if `isDevActive` → `exitDev()`, else `enterDev()`
(navigates directly to `/dev`). Matching on `e.code` (physical key position) is
layout-independent, sidestepping the `e.key` Shift-uppercase / non-US-layout
sensitivity — note `e.code` is a **net-new pattern** here (every other handler keys
off `e.key`), so the chord tests must build events with `code: "KeyL"` explicitly
(the `RootLayout.test.tsx:181-195` template sets `key` only, leaving `code === ""`),
and assert the handler ignores a `key:"l"`-with-no-matching-`code` event. **Escape
while on `/dev` → `exitDev()`, but guarded by `isEditableTarget`** (as the `/`
handler is) so that pressing Escape inside a focused demo input (the Inputs & form
search composite) clears/blurs the field rather than ejecting the whole page.
Modifier precedent: `Breadcrumbs.tsx:37`.

#### 4. Sidebar-foot triple-click host

**File**: `components/Sidebar/Sidebar.tsx` (insert at `:171→172`, last `<nav>`
child, `margin-top:auto`), `Sidebar.module.css`
**Changes**: New minimal `.foot` element — a **visible** version/build label (not a
blank target), so it doubles as the discoverability affordance — with a
triple-click counter — 3 clicks within a rolling 600 ms window → `enterDev()`. Port
the counter from `app-shell.jsx:7-20`; `userSelect:none`,
`title="triple-click for the design-system reference"`. The triple-click gesture
remains the least-discoverable of the three triggers (hinted only by the `title`); a
subtle hover/cursor affordance on the foot label is a cheap nice-to-have, but this is
an accepted dev-only tradeoff noted in the deviations aside.

**Discoverability**: because the page has no production nav entry, all three
triggers are otherwise hidden knowledge. Beyond the visible foot label and the
README documentation (Phase 11), the activation hook logs a **one-line DEV-build
console hint** on first mount naming the three triggers and the chord, so a
maintainer who opens devtools can find the page. (Full in-app surfacing is out of
scope — this is a dev-only tool.)

#### 5. Chrome shell

**File**: `components/DevDesignSystem/DevDesignSystem.tsx` (new),
`DevDesignSystem.module.css` (new — **selectively** re-author `ds-*` from
`app.css:1392-1943`, dropping selectors with no live consumer such as the
undefined `.ds-spec*` family rather than porting the legacy sheet wholesale)
**Changes**: Marquee (DEV tag, title, route, **`DEV_CHORD_HINT`** kbd —
duplicated inner content for the `-50%` loop), the sticky CONTENTS aside with
24 two-digit jumplinks + exit-to-app control, and the footer (keybind hint =
`DEV_CHORD_HINT`). The **active TOC jumplink carries `aria-current="location"`**
(not just a visual `is-active` class), following the codebase's navigation
convention (`Breadcrumbs.tsx:55` / `LibraryTemplatesIndex.tsx:108` use
`aria-current="page"`) so the active section is exposed to assistive tech, not
conveyed by colour alone. The token is intentionally `"location"` rather than the
codebase's `"page"` — the scroll-spy marks the current *position within a single
page*, not the current page within a nav set; add a one-line code comment noting
this so it isn't "normalised" back to `"page"`.
Each jumplink's `title` includes the section slug (e.g. "Stage dots — #stagedots")
so a maintainer can discover the deep-link slug from the page. Sections rendered as
empty `<DSSection>` stubs. The marquee and footer hints bind to the shared
`DEV_CHORD_HINT`, never `⌘⇧D`. Sticky offsets parameterised to the live topbar
height (`--ac-topbar-h`).

**Focus management**: on activation, move keyboard focus to the dev-page heading
(or the first TOC entry) rather than leaving it where the chord was pressed; on
exit, restore focus to a named stable app anchor so keyboard-only users do not lose
their place to `<body>`. Assert the restore target for **each** of the three exit
paths (exit-to-app control, Escape, chord-toggle), not just one.

### Success Criteria

#### Automated Verification

- [ ] Type-check + lint pass: `mise run frontend:check`
- [ ] Unit tests pass: `mise run test:unit:frontend`
- [ ] Chord test (RootLayout.test template `:181-195`): `Cmd/Ctrl+Shift+L`
  keydown fires the handler and `preventDefault()` is honoured; the four
  single-modifier negatives do not activate; the likely accidental-fire negatives
  also do not activate — `Cmd+L` (no Shift, a real browser shortcut),
  `Cmd+Shift+<other key>`; chord is inert when an editable target is focused
- [ ] Escape test: `Escape` on `/dev` calls `exitDev()`, but is inert when an
  editable target is focused (does not eject the page from a focused input)
- [ ] Triple-click test: 3 clicks <600 ms set `location.hash` to `#dev`; a
  slow third click does not
- [ ] Bridge test: setting `#dev` navigates to `/dev`; `#dev/colors` normalises
  to `/dev#colors` (via `replace:true`); a direct `/dev#colors` is left untouched;
  a stray `/dev#dev/colors` self-heals; `exitDev()` restores the stored prior path
  and clears the hash
- [ ] Back-navigation test: after an alias entry (`/library#dev/colors`), a
  `hashchange` back to the alias URL does **not** re-trigger forward activation
  (the alias was replaced, not pushed)
- [ ] Re-entrancy test: a programmatic hash write (`history.replaceState`) does
  **not** re-enter `sync()` (guards the no-loop invariant)
- [ ] Exit-restore test: a direct `/dev` entry (no alias) and a cold-load deep-link
  (prior seeded from `sessionStorage`) both restore correctly, falling back to
  `/library` only when no resolvable prior exists
- [ ] Invariant test: no `DEV_SECTIONS` id, expressed as a bare `#<id>` hash,
  matches `DEV_ALIAS_RE` (so no section can be mis-classified as an alias)
- [ ] Marquee/footer keybind hint text equals `DEV_CHORD_HINT` (asserted), not
  `⌘⇧D`
- [ ] TOC active jumplink carries `aria-current` and focus moves into the page on
  activation / restores to a stable anchor on exit (asserted)
- [ ] Router test: `/dev` resolves to `DevDesignSystem` and is uncrumbed

#### Manual Verification

- [ ] All three triggers land on `/dev` (DEV marquee + 24-section TOC rendered);
  `#dev`/`#dev/<section>` aliases normalise to `/dev#<section>`
- [ ] Exit-to-app, Escape, and the chord-toggle all return to the prior route
  with the sidebar restored
- [ ] DEV marquee scrolls seamlessly (no jump at the loop point)

---

## Phase 5: Scroll-spy, hash sync, in-page theme toggle

### Overview

Add the IntersectionObserver scroll-spy (active highlight + hash rewrite),
deep-link scrolling, and the in-page `<ThemeToggle/>`. Sections may still be
stubs (they have stable `id`s).

### Changes Required

#### 1. Scroll-spy + hash sync

**File**: `DevDesignSystem.tsx` (+ a co-located pure helper — see below; the page
stays a single component file by decision, only this computation is extracted so it
is unit-testable in isolation)
**Changes**: Bind an `IntersectionObserver` whose `root` is the `<main>` scroll
element, `rootMargin: "-80px 0px -55% 0px"`, `threshold: [0, 0.25, 0.5]`. Rather
than `closest("main")` (a tag-name traversal that silently resolves to the wrong
element if a nested `<main>` is ever introduced), bind to the scroll root exposed
by `RootLayout` via an explicit `data-scroll-root` attribute it sets on its
`<main>`.

**Active-section computation — a pure, total-order helper** (`pickActiveSection`,
co-located and unit-tested with fabricated rects). The observer persists each
section's latest rect across dispatches into a ref; on every dispatch the helper
recomputes from ALL persisted rects (never the single-dispatch highest-ratio,
which is the prototype's pinned-to-Colours bug). The rule is a defined total order
robust to gaps and tall sections: **pick the last section whose top edge is at or
above the active-band top; default to the first section when scrolled above all,
and the last when scrolled below all.** This is well-defined for every scroll
position — including a tall section spanning the whole band and the gap between
two short sections — so the highlight never clears, flickers, or pins.

On change, `setActive(id)` and write the canonical bare-section hash with
`history.replaceState`. **For the overview section, clear the hash** to
`history.replaceState(null, "", location.pathname)` (canonical `/dev`, matching
Desired End State) — do NOT write `#overview`. Deep-link landing reads
`location.hash.slice(1)` (or `aliasSection()` if an alias slipped through) and
scrolls the section into the active region; compute the target robustly via
`el.getBoundingClientRect().top - main.getBoundingClientRect().top + main.scrollTop
- offset` (independent of the `offsetParent` chain — the live `.main` declares no
`position`, so the ported `el.offsetTop` is unreliable; alternatively give `.main`
`position: relative`). TOC jumplinks (`href="#<id>"`) call the same `jump(id)`.

#### 2. In-page theme toggle

**File**: `DevDesignSystem.tsx` (chrome) + Overview card
**Changes**: Render `<ThemeToggle/>` in the dev chrome and wire the Overview
"flip to {theme}" card to `useThemeContext().toggleTheme()`. No new state.

### Success Criteria

#### Automated Verification

- [ ] Type-check + lint pass: `mise run frontend:check`
- [ ] Unit tests pass: `mise run test:unit:frontend`
- [ ] Unit test: `pickActiveSection` returns the correct id across fabricated
  rects — gap between two short sections, a tall section spanning the whole band,
  scrolled-above-all (first), scrolled-below-all (last) — proving the total order
  is defined everywhere
- [ ] E2E test: scrolling a lower section into the active region updates the TOC
  active item **and** `location.hash` to `#<that-section>`. Use **retrying
  assertions** (`await expect(page).toHaveURL(/#…/)`, `expect.poll` on the active
  TOC entry) with an explicit settle condition — never `waitForTimeout` — because
  IntersectionObserver dispatches are async/batched
- [ ] **Dedicated "never pinned" regression** E2E test: after scrolling past the
  (tall) Colours section into the next short section, the active TOC entry is that
  next section and the hash is `#<that-section>`, **not** Colours — a test that
  fails against the prototype's single-dispatch-highest-ratio behaviour
- [ ] E2E test: loading `/dev#colors` (and the `#dev/colors` alias) scrolls so the
  Colours heading sits in the observer's active region and its TOC entry is active
  (and carries `aria-current`)
- [ ] E2E test: with the overview section active (cold load on `/dev`, and after
  scrolling back to top), `location.hash` is empty and the URL is the bare `/dev`
  (the overview clears the hash — never `#overview`, never a stale `#colors`)
- [ ] E2E test: activating the in-page theme toggle flips
  `document.documentElement.dataset.theme` and the resolved `--ac-*` surface
  token values

#### Manual Verification

- [ ] Scrolling top→bottom advances the highlight smoothly through all 24
  entries; no section is skipped or pinned
- [ ] Deep-links to several sections land correctly on cold load

---

## Phase 6: Section content — tokens & type

### Overview

Port the five token/type sections (pure CSS + token reads). Fills the stubs.

### Changes Required

**File**: `DevDesignSystem.tsx` + module CSS
- **Overview**: intro prose + 4 cards — font families `3`, stroke icons
  `ICON_NAMES.length` (33), doc-type glyphs `DOC_TYPE_KEYS.length` (13), themes
  `2` with the live flip control. **Deviations home**: the Overview also renders a
  short "Deviations from the prototype" aside — the single authoritative, reader-
  facing home for the intentional divergences (no compact `PipelineMini`, no
  code-block traffic-light dots, the three-component status/verdict/result split,
  radii px-ladder vs `sm/md/lg`, no `--shadow-brand`, the canonical `/dev#<section>`
  URL form, the hand-authored-vs-live-component drift from Phase 9, and the
  doc-type-glyph sizes converging on `16/24/32`+`48`). The aside is the **union of
  all per-section inline deviation notes** — each affected section also carries a
  one-line inline note so a reader checking that primitive sees the divergence in
  context rather than mistaking it for a fidelity bug. (This is what "documented as
  a deviation" throughout this plan resolves to.)
- **Colours**: a `Swatch` that reads `getComputedStyle().backgroundColor` for
  the computed value. Groups: 8 surfaces, 4 foreground, 8 accent/status, 3
  strokes, the **19 named** `--atomic-*` tokens (assert by name — the live file
  has 37 incl. aliases), doc-type hues (one per `DOC_TYPE_HUE` entry = 13).
- **Type**: 7 ramps across Sora/Inter/Fira Code (hero→eyebrow).
- **Spacing**: `--sp-1..--sp-11` (11) bars.
- **Radii & shadows**: re-expressed against **live** names — radii ladder
  (`--radius-0..-12`, `--radius-pill`) + 3 shadows (`--ac-shadow-soft`,
  `--ac-shadow-lift`, `--shadow-card` labelled "brand"). No `--shadow-brand`.

### Success Criteria

#### Automated Verification

- [ ] `mise run frontend:check` and `mise run test:unit:frontend` pass
- [ ] Overview counts bind to `ICON_NAMES.length` / `DOC_TYPE_KEYS.length`
  (asserted against the constants, not frozen integers)
- [ ] Colours section asserts 8+4+8+3 named swatches, 19 named `--atomic-*`, and
  doc-type hues `= DOC_TYPE_HUE` count
- [ ] Colours theme-responsiveness (the AC's named oracle): read at least one
  surface swatch's computed `backgroundColor` in light and again in dark and assert
  they differ and match the expected light/dark token (reuse the
  `expected-colours.ts` helpers) — not left to a manual "plausible values" check
- [ ] Spacing asserts 11 steps; Radii asserts the live radii ladder + 3 shadows

#### Manual Verification

- [ ] Swatches show plausible computed values that change between light/dark for
  surface tokens
- [ ] No layout breakage in either theme

---

## Phase 7: Section content — glyphs, mark, icons

### Overview

Port the four glyph/mark/icon sections, rendering the Phase 1/3 primitives.
Emits the `data-testid` cells the migrated glyph VR specs will assert.

### Changes Required

**File**: `DevDesignSystem.tsx` + module CSS
- **Icons**: grid over `ICON_NAMES` (33) via `<Icon>` + a size ramp
  (12–32px). Net-new VR coverage (no migration target).
- **Doc-type glyphs**: `<Glyph docType size>` per `DOC_TYPE_KEYS` (13) at the
  codebase's existing sizes **16/24/32 plus the net-new 48** (a four-size ramp; not
  the prototype's 22/28/36/48), with `data-testid="glyph-cell-<type>-<size>"` cells
  reproducing the showcase contract. The `16/24/32` cells are exactly the ones the
  existing glyph pixel spec asserts (baselines unchanged); the `48` cell is net-new
  coverage.
- **Empty-state glyphs**: `<BigGlyph docType size>` per `DOC_TYPE_KEYS` (13) at
  48/64/80/96/128, with `data-testid="big-glyph-cell-<type>"` cells (per-cell
  `--bg-hue` inline).
- **Atomic mark**: `<AtomicMark size>` at 20/24/32/48/72 + an on-night cell.

### Success Criteria

#### Automated Verification

- [ ] `mise run frontend:check` and `mise run test:unit:frontend` pass
- [ ] Icons count asserts `= ICON_NAMES.length` (33)
- [ ] Doc-type glyphs count asserts `= DOC_TYPE_KEYS.length` (13) × 4 sizes
  (16/24/32/48); `glyph-cell-<type>-24` exists (for `glyph-resolved-fill`)
- [ ] Empty-state glyphs count asserts `= DOC_TYPE_KEYS.length` (13) × 5 sizes;
  `big-glyph-cell-<type>` cells present
- [ ] Atomic mark asserts 5 sizes + 1 on-night

#### Manual Verification

- [ ] All glyphs/icons render hue-/theme-correct in light + dark at every size

---

## Phase 8: Section content — interactive primitives

### Overview

Port chips, badges, stage dots, tier pills, buttons, form, sidebar nav.

### Changes Required

**File**: `DevDesignSystem.tsx` + module CSS
- **Chips**: `<Chip>` 6 tones × sm/md (12), with
  `data-testid="chip-cell-<variant>-<size>"` cells (`[data-variant]` preserved
  by `Chip`).
- **Status badges**: 8 statuses via `<StatusBadge>` + 4 verdicts. Per the live
  variant maps: `approve`→`<VerdictBadge>` (green), `request-changes`→
  `<VerdictBadge>` (red), `approve-with-changes`→`<StatusBadge>` (amber — Verdict
  maps it to neutral), `pass`→`<ResultBadge>` (green). Document the
  three-component split.
- **Stage dots**: `<PipelineMini completeness={{present}}>` over the 8
  `WORKFLOW_PIPELINE_STEPS` — all / partial / none. **Compact omitted**
  (no live prop) — documented deviation.
- **Tier pills**: hand-author 4 states (present/active/absent/default) reading
  live tokens (the live `.tierPill` has only 3 states and is template-coupled).
- **Buttons**: topbar/filter/sort/sort-active/filter-badge/inline-link/`ds-link`
  (7), using `<Icon>`.
- **Inputs & form**: hand-author the search-input-with-`⌘K` composite (no live
  `⌘K` — reproduced verbatim per the work item) + checkbox option rows with
  counts.
- **Sidebar nav**: group label, sub-label, default/active/with-pulse
  (animated `ac-pulse` keyframe)/faded items with counts.

### Success Criteria

#### Automated Verification

- [ ] `mise run frontend:check` and `mise run test:unit:frontend` pass
- [ ] Chips assert 12 (6×2) with `[data-variant]` cells; Status badges assert
  presence of all 12 named values via the correct component
- [ ] Stage dots count asserts `= WORKFLOW_PIPELINE_STEPS.length` (8) with
  all/partial/none present
- [ ] Tier pills assert 4 named states; Buttons assert 7; Form asserts 3
  (search + checked + unchecked); Sidebar nav asserts the 6 named variants

#### Manual Verification

- [ ] `approve-with-changes` renders amber and `pass` renders green
- [ ] The with-pulse nav item animates; checkboxes show counts

---

## Phase 9: Section content — composites & chrome

### Overview

Port cards, tables, markdown, code, frontmatter, empty/banners, toasts, topbar.

Several primitives are hand-authored here because the live component cannot render
standalone (breadcrumbs render null off a crumbed route; no live `ac-libtable`; the
`.tierPill` is template-coupled; the Toaster is a portal). This is an acknowledged
tradeoff: the hand-authored copy can drift from the runtime component as the latter
evolves, and the migrated VR baselines protect the copy, not the production
component. Where a live component can be made to render via a presentational prop
path (kanban via `WorkItemCardPresentation`, the mark via `AtomicMark`), prefer
that; the remaining hand-authored cases are noted in the in-page deviations aside.

### Changes Required

**File**: `DevDesignSystem.tsx` + module CSS
- **Cards**: lifecycle card, **kanban card via `WorkItemCardPresentation`** with
  `data-testid="kanban-card-cell-<state>"` + `.ac-kcard` (resting/dragging/
  overlay, the kanban-showcase contract), related-item row (`RelatedArtifacts`/
  `RelatedCluster`), empty-state lifecycle card (4 variants).
- **Tables**: library table with a selected row (hand-authored — no live
  `ac-libtable`).
- **Markdown**: `<MarkdownRenderer content resolveWikiLink>` (pass
  `useWikiLinkResolver()` so the wiki-link renders) — headings, bold/italic/
  inline code, wiki-link, ordered + unordered lists, table (8 element kinds).
- **Code blocks**: `<MarkdownRenderer>` fenced blocks with
  `data-testid="code-syntax-cell-<lang>"`, `data-language`, `.hljs-*`,
  `.codeblockHead` — the code-syntax-showcase contract. **Render all 8 languages the
  existing `code-block-resolved-colours.spec.ts` asserts** — `python`, `typescript`,
  `yaml`, `json`, `css`, `html`, `diff`, `markdown` — **plus the `diff`-override
  fixture** (the cells proving the `.language-diff` override is scoped, not global),
  so the migrated resolved-colour spec preserves story 0076's per-language +
  diff-scoping coverage rather than shrinking it. `bash` is added as net-new
  coverage. **No traffic-light dots** (live omits them) — documented deviation.
- **Frontmatter**: `<FrontmatterTable frontmatter resolveWikiLink bareIdPattern>`
  (resolver from `useWikiLinkResolver()`).
- **Empty & banners**: inline empty + warn banner (hand-authored, `<Icon
  name="alert">`).
- **Toasts**: ok/warn/err dismissible + reset (hand-authored static demos using
  `<Icon>`; not the live portal Toaster).
- **Topbar**: `Brand`, breadcrumbs (hand-authored — live `Breadcrumbs` renders
  null off a crumbed route), `OriginPill`, `SseIndicator`, `ThemeToggle`
  (5 parts).

### Success Criteria

#### Automated Verification

- [ ] `mise run frontend:check` and `mise run test:unit:frontend` pass
- [ ] Cards assert 4 variants incl. `kanban-card-cell-<state>` `.ac-kcard` cells
- [ ] Markdown asserts 8 element kinds incl. a resolved wiki-link
- [ ] Code blocks assert all 8 migrated `code-syntax-cell-<lang>` cells (python,
  typescript, yaml, json, css, html, diff, markdown) + the diff-override fixture +
  net-new bash, each with `data-language` + `.hljs-*` present
- [ ] Frontmatter asserts key/value rows with a link; Empty/banners assert 2;
  Toasts assert 3; Topbar asserts 5 parts

#### Manual Verification

- [ ] Wiki-link and frontmatter links resolve/render; code blocks highlight in
  both themes; kanban card matches the live kanban surface

---

## Phase 10: Visual-regression migration

### Overview

Rename the 5 pixel + 4 resolved showcase specs to `dev-design-system-*`,
`git mv` their snapshot folders, repoint navigation at `/dev#<section>`,
reproduce the exact testids/selectors, convert the code-syntax full-page
screenshot to a clipped locator, and regenerate baselines. Depends on Phases
7–9 (the asserted sections exist).

### Changes Required

#### 1. Rename + repoint specs

**Files** (rename, `git mv` snapshot folders to match):
- `glyph-showcase.spec.ts` → `dev-design-system-glyph.spec.ts` (+ folder)
- `glyph-resolved-fill.spec.ts` → `dev-design-system-glyph-resolved-fill.spec.ts`
- `big-glyph-showcase.spec.ts` → `dev-design-system-big-glyph.spec.ts` (+ folder)
- `chip-showcase.spec.ts` → `dev-design-system-chip.spec.ts` (+ folder)
- `chip-resolved-colours.spec.ts` → `dev-design-system-chip-resolved-colours.spec.ts`
- `code-syntax-showcase.spec.ts` → `dev-design-system-code-syntax.spec.ts` (+ folder)
- `code-block-resolved-colours.spec.ts` → `dev-design-system-code-block-resolved-colours.spec.ts`
- `kanban-card-showcase.spec.ts` → `dev-design-system-kanban-card.spec.ts` (+ folder)
- `kanban-card-resolved-styles.spec.ts` → `dev-design-system-kanban-card-resolved-styles.spec.ts`

Each spec: change `page.goto(...)` to the `/dev#<section>` deep-link (so the
section is scrolled into view), keep the same `data-testid`/descendant
selectors, keep the same screenshot `name` args. **The renamed
`dev-design-system-glyph.spec.ts` retains its existing `16/24/32` size loop** (so
its `git mv`d baselines stay valid with no size drift) and **adds a `48` cell as
net-new coverage** — Phase 7 renders glyphs at exactly `16/24/32/48`, so the spec's
existing cells all resolve. Convert `dev-design-system-code-syntax.spec.ts` from
`expect(page).toHaveScreenshot` to a clipped
`[data-testid="code-syntax-cell-<lang>"]`-anchored locator (its viewport was
1440×900). `glyph-resolved-fill` reads the `24px` cell (retained in Phase 7).

#### 2. playwright.glyph.config.ts

**File**: `playwright.glyph.config.ts`
**Changes**: Update `testMatch` to the renamed glyph specs and the
`webServer.url` health route from `/glyph-showcase` to `/dev`.

#### 3. Regenerate baselines

Regenerate darwin baselines locally and linux via the "Update visual regression
baselines" workflow (linux baselines drift behind darwin).

### Success Criteria

#### Automated Verification

- [ ] `mise run frontend:check` passes
- [ ] `mise run test:e2e:visualiser` passes with the renamed specs + regenerated
  baselines (all 9 specs assert `DevDesignSystem` sections)
- [ ] **Automated cell-presence gate**: a test enumerates the required
  `data-testid` cells per migrated section with **concrete, pinned dimensions** —
  `glyph-cell-<type>-{16,24,32,48}`, `big-glyph-cell-<type>`,
  `chip-cell-<variant>-{sm,md}`, `code-syntax-cell-<lang>` for all 8 + diff-override,
  `kanban-card-cell-<state>` — and fails if any is missing. Pinning the sizes (not a
  generic `<size>` placeholder) means a glyph size drift fails CI. Coverage
  preservation is thereby enforced by CI, not by the manual row-by-row audit alone
  (the resolved-colour companion specs assert hard-coded cell lists that can silently
  shrink on a rename)
- [ ] No orphaned snapshot folders (old `*-showcase.spec.ts-snapshots/` removed,
  new `dev-design-system-*-snapshots/` present)
- [ ] Code-syntax spec is a clipped locator (no full-page screenshot)

#### Manual Verification

- [ ] Row-by-row audit against the showcase→section mapping confirms each retired
  spec's assertions are reproduced (glyph→Doc-type glyphs; big-glyph→Empty-state
  glyphs; chip→Chips; code-syntax→Code blocks; kanban→Cards)
- [ ] linux + darwin baselines committed for every renamed pixel case

---

## Phase 11: Retire showcase routes + README/docs

### Overview

Remove the five showcase routes and components, assert the former paths no
longer resolve to a showcase, and update the README. Last phase (after VR is
repointed off the old routes).

### Changes Required

#### 1. Remove routes

**File**: `src/router.ts`
**Changes**: Delete `glyphShowcaseRoute`, `bigGlyphShowcaseRoute`,
`chipShowcaseRoute`, `codeSyntaxShowcaseRoute`, `kanbanCardShowcaseRoute` and
their imports + `routeTree` entries (`:49-54,152-216`). No redirects.

#### 2. Delete components

Remove `src/routes/{glyph-showcase,big-glyph-showcase,chip-showcase,
code-syntax-showcase,kanban-card-showcase}/`.

#### 3. README

**File**: `README.md`
**Changes**: Rewrite the "Developer Routes" section (`:44-48`) to list
`DevDesignSystem` and drop all five showcases. Fix the stale copy while rewriting:
"12 glyphs"→13; the "all 3 supported sizes (16/24/32 px)" glyph claim is now false
(the size union gains `48`) so correct it to `16/24/32/48` or drop the size
enumeration (the page itself is the live size reference); and replace the trailing
`0037-glyph-component.md` doc-link (which describes the retired showcase and would
dangle) with the `0083` work item, or remove it. Because the README is the only
persistent developer-facing home for this dev-only surface (no in-app nav links to
`/dev`), it must itemise — not merely gesture at —: all **three activation
triggers** (`#dev` hash, the chord, the sidebar-foot triple-click); the **chosen
chord** (`Cmd/Ctrl+Shift+L`, and the `Cmd/Ctrl+Shift+G` fallback if the
cross-browser matrix forced it); the **canonical `/dev#<section>` deep-link form**
and that `#dev`/`#dev/<section>` are accepted aliases the bridge normalises; and
that `<section>` is the **lowercase slug, not the display label** (e.g.
`/dev#stagedots` reaches "Stage dots"). Update the "Glyph showcase specs" block
(`:67-77`) for the renamed glyph specs/config (`dev-design-system-glyph*.spec.ts`,
the `/dev` health route), and verify the adjacent "Visual-Regression Baselines"
section's example `--update-snapshots` commands still name live spec filenames after
the Phase-10 renames. **If the chord matrix forced the `Cmd/Ctrl+Shift+G` fallback,
re-verify the README chord wording (and any in-app hint) against the final
`DEV_CHORD_HINT` value** so the documented chord can never drift from the bound one.

#### 4. Amend the work item acceptance criteria

**File**: `meta/work/0083-dev-design-system-reference-page.md`
**Changes**: Own the AC reconciliation the Desired End State note describes (left
to this phase so the canonical record matches the shipped behaviour). Amend AC3/
AC5/AC7 from the literal `#dev`-hash wording to the route + bare-section-hash form
(`/dev` + DEV marquee/TOC as the activation oracle; scroll-spy writes `#<section>`;
exit clears the section hash + restores the prior path; `#dev`/`#dev/<section>`
retained as aliases). Amend AC9's "returns 404" to "resolves to the SPA not-found
UI (no redirect)". Reconcile the stale oracle parentheticals (Stage dots `8` not 9,
Doc-type glyphs `13` not 12), which the live-constant-bound assertions already use.

### Success Criteria

#### Automated Verification

- [ ] `mise run frontend:check` and `mise run test:unit:frontend` pass
- [ ] Router test: each former showcase path (`/glyph-showcase`, etc.) no longer
  resolves to a showcase component (default not-found, no redirect)
- [ ] `mise run test:e2e:visualiser` passes (specs target `/dev`, not the removed
  routes)
- [ ] No dangling imports of the deleted route components (build clean)

#### Manual Verification

- [ ] README developer-routes section lists `DevDesignSystem` and none of the
  five showcases, and documents all three triggers, the chord (+ fallback), the
  canonical `/dev#<section>` form, and the slug-vs-label distinction
- [ ] Work item 0083 ACs amended (AC3/AC5/AC7 route+bare-section form; AC9
  "SPA not-found"; stale count parentheticals reconciled to 8/13)
- [ ] Visiting a former showcase path shows the not-found UI, not a stale page

---

## Testing Strategy

### Unit Tests (vitest + @testing-library)

- `Icon`: name coverage (33), size, explicit `aria-hidden`/aria flip, DEV
  unknown-name warn (Phase 1).
- `AtomicMark`: unique gradient ids, size; `Brand` pixel/structural equivalence
  (no literal-id snapshot) (Phase 3).
- `Glyph`: `48`-size specimen (the one added size; Phase 3).
- Keychord: `preventDefault` + modifier negatives (incl. `Cmd+L`, wrong-key) +
  editable-target inertness, using the `RootLayout.test.tsx:181-195` template;
  Escape exit guarded by `isEditableTarget` (Phase 4).
- Triple-click counter: 3-within-600 ms vs slow third click (Phase 4).
- Activation bridge: enter/exit/normalise/deep-link transitions, `replace:true`
  back-navigation (no re-trigger), re-entrancy guard (`replaceState` doesn't
  re-enter `sync()`), exit-restore for direct `/dev` + cold-load, and the
  no-section-id-matches-`DEV_ALIAS_RE` invariant (Phase 4).
- `pickActiveSection` pure helper: gap / tall-section / above-all / below-all
  rects (Phase 5).
- Per-section variant-count/presence oracles binding to live constants
  (`ICON_NAMES.length`, `DOC_TYPE_KEYS.length`, `WORKFLOW_PIPELINE_STEPS.length`,
  `DOC_TYPE_HUE` count) (Phases 6–9).
- VR cell-presence gate enumerating required `data-testid`s per migrated section
  (Phase 10).
- Router: `/dev` resolves + uncrumbed; former showcase paths unresolved (Phases
  4, 11).

### Integration / E2E (Playwright)

- Scroll-spy advances active + hash using retrying assertions, never pinned;
  a dedicated "leaves Colours for the next section" regression (Phase 5).
- `/dev#<section>` deep-link scrolls into the active region; active TOC entry
  carries `aria-current` (Phase 5).
- In-page theme toggle flips `data-theme` + `--ac-*` values; Colours surface
  swatch computed `backgroundColor` differs light↔dark (Phases 5–6).
- The 9 migrated VR specs (5 pixel light+dark per cell, 4 resolved-colour/style)
  assert the consolidated sections — incl. all 8 code-syntax languages + the
  diff-override fixture (Phase 10).

### Manual Testing Steps

1. Activate via `#dev`, `Cmd/Ctrl+Shift+L`, and sidebar-foot triple-click —
   each independently; confirm each lands on `/dev` (and `#dev` aliases
   normalise to `/dev#<section>`).
2. Deep-link `/dev#colors` (and the `#dev/colors` alias); scroll through all
   sections; confirm hash (`#<section>`) + highlight track and never pin to
   Colours.
3. Toggle theme in-page; eyeball every section in light + dark.
4. Exit via the control, Escape, and chord-toggle; confirm prior route restores.
5. **Cross-browser chord matrix** (the dedicated AC): in Chrome, Edge, Firefox,
   Safari (record exact version + OS), confirm `Cmd/Ctrl+Shift+L` delivers the
   keydown, `preventDefault()` suppresses any browser default, and the page
   toggles. Record the matrix in the work item / PR. If reserved anywhere,
   fall back to `Cmd/Ctrl+Shift+G` (the only change is `DEV_CHORD`/
   `DEV_CHORD_HINT`).
6. Visit each former showcase path; confirm not-found (no redirect).

## Performance Considerations

- The page is dev-only and not in the production nav; render cost is
  immaterial. The single `IntersectionObserver` over 24 sections is cheap;
  disconnect on unmount.
- The marquee animation uses `will-change: transform` (port as-is).
- Multiple `AtomicMark`/`Glyph` instances each carry an inline SVG; acceptable
  for a reference page.

## Migration Notes

- **VR baselines must be regenerated** for renamed pixel specs (Phase 10) and
  any icon-geometry-affected specs (Phase 2). Use the "Update visual regression
  baselines" workflow for linux; regenerate darwin locally. A `GITHUB_TOKEN`
  push won't re-trigger Main CI (per project conventions).
- **Spec folder renames orphan baselines** unless the `*-snapshots/` folder is
  `git mv`d alongside the spec and screenshot `name` args are kept stable.
- Run all `jj` VCS operations from within the active workspace, never the repo
  root.

## References

- Original work item: `meta/work/0083-dev-design-system-reference-page.md`
- Codebase research: `meta/research/codebase/2026-06-12-0083-dev-design-system-reference-page.md`
- Review-1: `meta/reviews/work/0083-dev-design-system-reference-page-review-1.md`
- Authoritative content spec: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/src/view-dev.jsx`
- Supporting prototype sources (same `prototype-full/`): `assets/tokens.css`,
  `src/app.css` (`ds-*` chrome `1392-1943`), `src/ui.jsx` (Icon paths `8-53`),
  `src/data.jsx`, `src/highlight.jsx`, `src/big-glyphs.jsx`,
  `Accelerator Visualiser.html` (hash/keychord `102,146-197`),
  `src/app-shell.jsx` (triple-click `7-20`)
- Live anchors: `src/router.ts:152-218`, `RootLayout.tsx:20-96`,
  `RootLayout.test.tsx:73-196`, `Glyph.tsx:39-55`, `Brand.tsx:6-39`,
  `PipelineMini.tsx:5-18` + `api/types.ts:23-37,284-368`,
  `playwright.config.ts`, `tests/visual-regression/*`
