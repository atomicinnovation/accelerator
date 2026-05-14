---
date: 2026-05-12T10:40:16+01:00
researcher: Toby Clemson
git_commit: 6c0e429bfbbc6b4541906fe5cbdf1b04d5f4ef9c
branch: (jj working copy — no bookmark; ancestor bookmark `main` at 81461f75)
repository: accelerator
topic: "Glyph Component (work item 0037)"
tags: [research, codebase, visualiser, frontend, components, design-tokens, glyph, doc-types, theming]
status: complete
last_updated: 2026-05-12
last_updated_by: Toby Clemson
---

# Research: Glyph Component (work item 0037)

**Date**: 2026-05-12 10:40:16 BST
**Researcher**: Toby Clemson
**Git Commit**: 6c0e429bfbbc6b4541906fe5cbdf1b04d5f4ef9c
**Branch**: jj working copy (no bookmark); ancestor bookmark `main` at 81461f75
**Repository**: accelerator

## Research Question

For work item [0037 — Glyph Component](../work/0037-glyph-component.md), gather the codebase context an implementer needs to land a per-doc-type `Glyph` React component (12 doc types × sizes 16/24/32 px), introduce the `--ac-doc-<key>` token sub-namespace in light + dark, add a `/glyph-showcase` developer route, and provide both a Vitest smoke test and a Playwright visual-regression spec. Identify the integration points (types, tokens, theming, routing, tests), the component conventions to mirror, the historical decisions that constrain it, and the open gaps the implementer will hit.

## Summary

Every infrastructure piece the work item assumes already exists in the visualiser frontend, with **one exception**: there is no `skills/visualisation/visualise/frontend/README.md`. The work item's "link from a Developer routes section in the frontend README" requires creating the README from scratch (or relocating the dev-routes documentation).

The integration contract is tightly defined:

- **Types** — `DocTypeKey` is a closed 13-member union in `src/api/types.ts:4-8`; `templates` is the only `virtual: true` key. `GlyphDocTypeKey = Exclude<DocTypeKey, 'templates'>` is the 12-key constrained subset the work item specifies.
- **Tokens** — `--ac-*` tokens are defined in three CSS blocks (MIRROR-A: explicit `[data-theme="dark"]`; MIRROR-B: `@media (prefers-color-scheme: dark) :root:not([data-theme="light"])`; light `:root`) in `src/styles/global.css`, mirrored in `src/styles/tokens.ts` (`LIGHT_COLOR_TOKENS` / `DARK_COLOR_TOKENS`). MIRROR-A/MIRROR-B byte equivalence is enforced by a parity test in `global.test.ts`. There is no `--ac-doc-*` sub-namespace today — 0037 establishes the first one.
- **Theming** — `data-theme` on `document.documentElement` (driven by `src/api/use-theme.ts:30-32`) is the single CSS-cascade hinge. Glyph reads `var(--ac-doc-<key>)` and inherits the theme swap automatically; no theme detection inside Glyph.
- **Conventions** — Component layout is rigid: `src/components/<Name>/<Name>.tsx` + `.module.css` + `.test.tsx`, named exports only, no barrels, inline JSX SVGs. The Brand component (`src/components/Brand/Brand.tsx:25,31`) is the canonical example of writing `var(--ac-*)` directly into SVG `fill`/`stroke` attributes.
- **Routing** — TanStack Router with imperative `createRoute(...)` calls in `src/router.ts`; add `glyphShowcaseRoute` next to `kanbanRoute` (line 132) and append to the tree at line 150.
- **Tests** — Vitest config lives inside `vite.config.ts:63-70` (no standalone `vitest.config.ts`); Playwright is fully configured with a dedicated `visual-regression` project. The existing `tests/visual-regression/tokens.spec.ts` is the template — extend its `ROUTES` array with `['glyph-showcase', '/glyph-showcase']` rather than creating a parallel spec, unless a separate file better matches the work-item's named path `e2e/glyph.spec.ts` (the work item names `e2e/glyph.spec.ts`, but the project's visual-regression baselines are conventionally under `tests/visual-regression/__screenshots__/`, not `e2e/__snapshots__/`).

Two concrete deviations between the work item's wording and the codebase's reality the implementer should reconcile during planning:

1. **Playwright spec location.** Work item says `e2e/glyph.spec.ts` with baselines at `e2e/__snapshots__/`. Project convention puts visual-regression specs at `tests/visual-regression/<name>.spec.ts` with baselines at `tests/visual-regression/__screenshots__/<spec>.ts-snapshots/<id>-<browser>-<platform>.png`. The work item's path is inconsistent with the existing `tokens.spec.ts` precedent and the Playwright config (`playwright.config.ts:21-37` declares two projects with different `testDir`s).
2. **`maxDiffPixelRatio` value.** Work item specifies `0.005` (0.5 %). Existing `tokens.spec.ts:34` uses `0.05` (5 %). The looser threshold is probably better-aligned with cross-platform `darwin`/`linux` snapshot drift; tightening to 0.5 % will likely cause CI flakes unless baselines are captured on the CI platform.

## Detailed Findings

### Type contract (`src/api/types.ts`)

The `DocTypeKey` union has **13 keys** (not 12) — `decisions`, `work-items`, `plans`, `research`, `plan-reviews`, `pr-reviews`, `work-item-reviews`, `validations`, `notes`, `prs`, `design-gaps`, `design-inventories`, `templates` (`types.ts:4-8`). The `DOC_TYPE_KEYS` runtime array mirrors it (`types.ts:14-19`).

`DocType` (`types.ts:26-36`) carries a required `virtual: boolean` field. The inline comment confirms `templates` is the only virtual key today: "Templates and any future virtual/derived types set `virtual: true`; real document types set `virtual: false`. Sidebar partitions on this flag." So `GlyphDocTypeKey = Exclude<DocTypeKey, 'templates'>` matches the runtime semantic of "real doc types" — but the work item should not rely solely on the literal exclusion: if a new virtual key is added later, `GlyphDocTypeKey` will silently include it. A more robust definition would derive from a const filter, though this is harder to express purely at the type level.

`LIFECYCLE_PIPELINE_STEPS` (`types.ts:134-169`) maps 11 of the 13 doc-type keys to pipeline-step records; it omits `work-item-reviews` and `templates`. The work item already notes this — Glyph itself is unaffected, but lifecycle consumers in 0040 render 11 glyphs, not 12.

### Token contract (`src/styles/global.css` + `src/styles/tokens.ts`)

Three theme blocks in `global.css` must be kept in sync:

- Light `:root` block — `global.css:66-143`
- MIRROR-A: explicit `[data-theme="dark"]` — `global.css:145-173`. The block-level comment (lines 145-149) declares it the "canonical source of truth read by `readCssVar('<name>', 'dark')`" and requires any dark edit to be mirrored to MIRROR-B.
- MIRROR-B: `@media (prefers-color-scheme: dark) :root:not([data-theme="light"])` — `global.css:175-205`. Byte-equivalent duplicate of MIRROR-A.

Parity between MIRROR-A and MIRROR-B is enforced by a unit test in `src/styles/global.test.ts`. Any new `--ac-doc-<key>` token must be added to all three blocks plus the TypeScript mirrors (`tokens.ts`).

`tokens.ts` exports flat-record `as const` objects keyed by bare token name (e.g. `'ac-bg'`, not `'--ac-bg'`):

- `LIGHT_COLOR_TOKENS` — `tokens.ts:1-25` (24 entries)
- `DARK_COLOR_TOKENS` — `tokens.ts:27-47` (20 entries — `ac-ok`, `ac-warn`, `ac-err`, `ac-violet` are light-only)

Plus `TYPOGRAPHY_TOKENS`, `SPACING_TOKENS`, `RADIUS_TOKENS`, `LIGHT_SHADOW_TOKENS`, `DARK_SHADOW_TOKENS`, `LAYOUT_TOKENS`, `MONO_FONT_TOKENS`. The 0033 plan locked this seven-frozen-export shape (`meta/plans/2026-05-06-0033-design-token-system.md:84-87`).

There is **no `--ac-doc-*` sub-namespace** in either `global.css` or `tokens.ts` today. 0037 establishes the first per-doc-type token sub-namespace in the codebase.

A `migration.test.ts` invariant (per 0033 plan lines 107-114) requires that "every `var(--NAME)` reference resolves to a key in the union of the declared token exports". Any new `--ac-doc-<key>` token must therefore be declared in `LIGHT_COLOR_TOKENS` and `DARK_COLOR_TOKENS` (or in a new dedicated export) — otherwise migration tests will fail.

Option for organising the new tokens:
- Add the 12 new entries directly into the existing `LIGHT_COLOR_TOKENS` / `DARK_COLOR_TOKENS` (consistent with the flat-record convention).
- Or introduce dedicated `LIGHT_DOC_COLOR_TOKENS` / `DARK_DOC_COLOR_TOKENS` exports for stronger encapsulation and a separate parity describe. The work item is silent; either matches existing conventions.

### Theme switching contract (`src/api/use-theme.ts`)

`useTheme` writes `data-theme={theme}` on `document.documentElement` via `useEffect` (`use-theme.ts:30-32`). This is the only DOM side-effect that drives the CSS-cascade hinge. Setting the attribute to `'dark'` activates MIRROR-A; flipping to `'light'` short-circuits the OS-preference fallback in MIRROR-B.

Owning/consumer split (`use-theme.ts:59-78`): `useTheme` is the OWNING hook — call exactly once at RootLayout. Leaf components (Glyph included) must use `useThemeContext()`. **But Glyph does not need to call either**: referencing `var(--ac-doc-<key>)` in an SVG attribute is sufficient; the browser resolves the variable against the live `data-theme` cascade. The work item's "no React state change has occurred" acceptance criterion (line 69) is the explicit assertion of this property.

Initial-theme resolution (`use-theme.ts:18-24`) checks `data-theme` attribute → `localStorage` → OS preference. Combined with `boot-theme.ts` (a pre-paint theme bootstrapper, named in the locator output), this means by the time Glyph renders, `data-theme` is already set.

### Component conventions (`src/components/`)

14 existing components, all following the same layout:

- `<Name>/<Name>.tsx` + `<Name>.module.css` + `<Name>.test.tsx`
- Named function exports only — `export function ComponentName()`
- No `index.ts` / barrel files (zero matches under `src/components/`)
- CSS Modules — `import styles from './X.module.css'`
- Inline JSX SVGs — no imported `.svg` files, no `<use href>`
- Props: local `interface Props { ... }` for one-use components (e.g. `PipelineDots.tsx:5-9`); exported `interface ComponentNameProps` with JSDoc for reusable atoms (e.g. `TopbarIconButton.tsx:4-15`).

For Glyph, **exported `interface GlyphProps`** is the right choice — it is a reusable atom consumed by ≥8 downstream surfaces (per the work item's Out of Scope list).

**SVG colour application — two precedents:**

1. **`currentColor` + CSS class** — `SseIndicator.tsx:33` uses `stroke="currentColor"`; the colour is set via CSS `color:` rule. Works when one colour applies to the whole SVG.
2. **CSS variable as SVG attribute** — `Brand.tsx:15-31` writes `fill="var(--ac-accent-2)"`, `stroke="var(--ac-accent)"`, `stopColor="var(--ac-accent-2)"` directly as attribute literals. React passes the string through; the browser resolves at paint time.

For Glyph, **option 2 is the work item's prescribed pattern** (Requirements bullet "Source all Glyph fill colours from the `--ac-doc-<key>` tokens"). The `Brand` component is the most direct prior art; its test (`Brand.test.tsx:11-19`) shows the exact assertion shape — query the SVG element, read the `fill` attribute via `getAttribute('fill')`, and assert against the literal string `'var(--ac-doc-<key>)'`. This is the assertion pattern to mirror, with one caveat: the work item's AC #4 (`0037-glyph-component.md:69`) requires `getComputedStyle(glyphSvg).fill` to resolve to a hex. JSDOM does not compute CSS-variable substitutions in `fill` attributes the same way browsers do — the implementer should verify experimentally whether `getComputedStyle(svg).fill` in JSDOM returns the resolved hex or the literal `var(...)` string. If JSDOM returns the literal, the Vitest test can fall back to `getAttribute('fill') === 'var(--ac-doc-<key>)'` and the resolution-to-hex assertion moves to the Playwright visual-regression check.

**Accessibility precedents:**

- Decorative SVGs use `aria-hidden="true"` on the `<svg>` element — see `Brand.tsx:10`, `SseIndicator.tsx:28`, `ThemeToggle.tsx:16`, `FontModeToggle.tsx:13`. No `role="img"` appears anywhere in the components directory today. Glyph's optional `accessibleLabel` path would be the first introduction of `role="img"` — fine, but novel.
- Container-level `aria-label` when state-bearing — `OriginPill.tsx:7` ("Server origin"), `PipelineDots.tsx:11` ("Lifecycle pipeline").
- Data attributes (`data-state`, `data-icon`, `data-stage`, `data-present`) used both for CSS selectors and test hooks — `TopbarIconButton.tsx:22`, `PipelineDots.tsx:17-18`. Glyph could expose `data-doc-type` for the same purpose.

**Media-query guards:**

- `@media (prefers-reduced-motion: reduce)` — `OriginPill.module.css:19-23`, `SseIndicator.module.css:19-23`.
- `@media (forced-colors: active)` — `TopbarIconButton.module.css:29-33`.

Glyph has no animation (per work item), so reduced-motion is not required. Forced-colors is a defensible addition since Glyph is a colour-conveys-meaning component — Windows High Contrast users would otherwise see a uniform fill across all 12 doc types. Not in the work item's Requirements, but worth flagging.

**Component test patterns:**

- `import { describe, it, expect } from 'vitest'` + `@testing-library/react`'s `render` / `screen`.
- `import css from './X.module.css?raw'` then regex-assert the CSS source (`OriginPill.test.tsx:3,27`, `TopbarIconButton.test.tsx:51`) — useful for CSS-level behaviour JSDOM can't compute.
- For inline-SVG components, query the DOM and assert the literal `var(--ac-...)` attribute value (`Brand.test.tsx:11-19`).

### Routing setup (`src/router.ts`)

TanStack Router, imperative `createRoute` style. The existing top-level routes are `/`, `/library`, `/library/templates`, `/library/templates/$name`, `/library/$type`, `/library/$type/$fileSlug`, `/lifecycle`, `/lifecycle/$slug`, `/kanban`. The tree is assembled at `src/router.ts:139-151`. Add `glyphShowcaseRoute` after `kanbanRoute` (line 132 area) and append to the array at line 150. No JSX route definitions, no `App.tsx`.

`withCrumb` wraps routes that should appear in the Breadcrumbs trail; for a developer-only showcase, omit `withCrumb` (use plain `createRoute`) or include it for consistency — either is defensible.

### Test infrastructure

**Vitest** (`vite.config.ts:63-70`): jsdom + globals + CSS enabled + setup file at `src/test/setup.ts`. Setup stubs `EventSource`, `ResizeObserver`, `matchMedia`, `scrollTo`, `scrollIntoView` (`src/test/setup.ts:31-50`). `e2e/` and `tests/visual-regression/` are excluded from Vitest collection. Test files use the convention `*.test.tsx` co-located with sources.

**Playwright** (`playwright.config.ts`): two projects.

- `visual-regression` — `testDir: './tests/visual-regression'`, `snapshotDir: './tests/visual-regression/__screenshots__'`, chromium-only, runs first.
- `chromium` — `testDir: './e2e'`, depends on `visual-regression`.

Snapshot file naming: `<spec>.ts-snapshots/<screenshot-id>-<browser>-<platform>.png`, with both `-darwin.png` and `-linux.png` checked in. Existing `tests/visual-regression/tokens.spec.ts` is the template — 6 routes × 2 themes = 12 screenshots, with `maxDiffPixelRatio: 0.05`, `animations: 'disabled'`, and a relative-time-mask helper. Theme switch is `document.documentElement.dataset.theme = 'dark'` followed by a `requestAnimationFrame` yield.

**Reconciliations the implementer must make with the work item:**

- The work item specifies `e2e/glyph.spec.ts` with baselines at `e2e/__snapshots__/`. That path doesn't match the existing project layout. Recommended: extend `tests/visual-regression/tokens.spec.ts` with a new `ROUTES` entry, or create `tests/visual-regression/glyph-showcase.spec.ts` mirroring the same shape.
- The work item specifies `maxDiffPixelRatio: 0.005`. Existing precedent is `0.05`. The implementer should match the precedent unless they're prepared to baseline on CI's `linux` runner directly.

### Package & dependencies (`package.json`)

- `@tanstack/react-router ^1`, `react ^19`, `react-dom ^19`
- Test: `@playwright/test ^1.59.1`, `vitest ^3`, `@testing-library/{react,jest-dom,user-event}`, `jsdom ^26`, `@vitejs/plugin-react ^4`, `vite ^6`.
- Scripts: `dev` (`vite`), `test` (`vitest run`), `test:watch` (`vitest`), `test:e2e` (`playwright test`), `test:e2e:ui`.

No `react-router-dom` — TanStack is the only router. No standalone `vitest.config.ts` — config is inside `vite.config.ts`.

### Canonical screenshots (verified present)

- `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/library-view-updated-light.png` ✓
- `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/library-view-updated-dark.png` ✓

These are the source of truth for icon shape and per-doc-type fill colour. The work item names `magick identify -format "%[hex:p{x,y}]" file.png` for sampling — `magick` (ImageMagick) is the expected tool; no internal helper exists.

Additional consumer-context screenshots in the same directory (`main-{light,dark}.png`, `kanban-view.png`, `lifecycle-cluster-detail.png`, `templates-view.png`) are useful for visualising downstream Glyph integration but not required for 0037 itself.

## Code References

- `skills/visualisation/visualise/frontend/src/api/types.ts:4-8` — `DocTypeKey` union (13 keys)
- `skills/visualisation/visualise/frontend/src/api/types.ts:14-19` — `DOC_TYPE_KEYS` runtime array
- `skills/visualisation/visualise/frontend/src/api/types.ts:26-36` — `DocType` interface (with `virtual: boolean`)
- `skills/visualisation/visualise/frontend/src/api/types.ts:134-169` — `LIFECYCLE_PIPELINE_STEPS` (11 of 13 doc types)
- `skills/visualisation/visualise/frontend/src/styles/global.css:66-143` — light `:root` theme block
- `skills/visualisation/visualise/frontend/src/styles/global.css:145-173` — MIRROR-A `[data-theme="dark"]`
- `skills/visualisation/visualise/frontend/src/styles/global.css:175-205` — MIRROR-B `@media` mirror
- `skills/visualisation/visualise/frontend/src/styles/tokens.ts:1-47` — `LIGHT_COLOR_TOKENS` / `DARK_COLOR_TOKENS`
- `skills/visualisation/visualise/frontend/src/styles/global.test.ts` — MIRROR-A/B parity test
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts` — token-existence invariants
- `skills/visualisation/visualise/frontend/src/api/use-theme.ts:30-32` — `data-theme` attribute write
- `skills/visualisation/visualise/frontend/src/api/use-theme.ts:59-78` — owning/consumer hook split
- `skills/visualisation/visualise/frontend/src/api/boot-theme.ts` — pre-paint theme bootstrap
- `skills/visualisation/visualise/frontend/src/components/Brand/Brand.tsx:15-31` — `var(--ac-*)` in SVG attributes
- `skills/visualisation/visualise/frontend/src/components/Brand/Brand.test.tsx:11-19` — assertion pattern for var-in-attribute
- `skills/visualisation/visualise/frontend/src/components/OriginPill/OriginPill.tsx:7-8` — container `aria-label` + decorative inner
- `skills/visualisation/visualise/frontend/src/components/OriginPill/OriginPill.test.tsx:3,25-35` — `?raw` CSS-source test pattern
- `skills/visualisation/visualise/frontend/src/components/TopbarIconButton/TopbarIconButton.tsx:4-15` — exported `Props` interface pattern
- `skills/visualisation/visualise/frontend/src/components/PipelineDots/PipelineDots.tsx:5-9` — local `Props` pattern + `data-stage`/`data-present`
- `skills/visualisation/visualise/frontend/src/components/PipelineDots/PipelineDots.test.tsx:13-28` — example component test to mirror
- `skills/visualisation/visualise/frontend/src/components/SseIndicator/SseIndicator.tsx:33` — `currentColor` precedent
- `skills/visualisation/visualise/frontend/src/router.ts:132-151` — route tree assembly (insertion point)
- `skills/visualisation/visualise/frontend/src/router.ts:155-159` — TanStack type registry
- `skills/visualisation/visualise/frontend/vite.config.ts:63-70` — Vitest config block
- `skills/visualisation/visualise/frontend/src/test/setup.ts:31-50` — Vitest setup + DOM stubs
- `skills/visualisation/visualise/frontend/playwright.config.ts:21-37` — visual-regression + chromium project config
- `skills/visualisation/visualise/frontend/tests/visual-regression/tokens.spec.ts:9-39` — visual-regression spec template
- `skills/visualisation/visualise/frontend/tests/visual-regression/__screenshots__/` — baseline snapshot directory (darwin + linux per route)
- `skills/visualisation/visualise/frontend/package.json:9-46` — scripts + dependencies
- `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/library-view-updated-light.png` — canonical light reference
- `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/library-view-updated-dark.png` — canonical dark reference

## Architecture Insights

- **Token sub-namespaces are new territory.** 0037 introduces the first `--ac-<domain>-<key>` family. Whether to grow `LIGHT_COLOR_TOKENS` or split into a dedicated `LIGHT_DOC_COLOR_TOKENS` export is an organisational choice; both fit the 0033 plan's "seven frozen exports" mould (which allows new exports so long as the parity-test pattern is added).
- **The CSS cascade carries all theme switching.** Glyph holds no theme state, calls no hooks, has no `useEffect`. This is structurally cleaner than the prototype's `ThemeProvider`-pattern would suggest and is enforced by AC #4's "no React state change has occurred" assertion.
- **The closed-set principle from 0033** ("No new tokens beyond the inventory's named set", `meta/plans/2026-05-06-0033-design-token-system.md:173-176`) is being deliberately relaxed by 0037 — the prototype inventory **is** the source the 12 new tokens come from. The implementer should call this out explicitly in the implementation commit.
- **MIRROR-A/MIRROR-B parity is enforced by test, not by lint.** Adding new tokens in only one block fails CI. The implementer must update both blocks in lockstep — and the `tokens.ts` mirror as well so `migration.test.ts` passes.
- **`color-mix()` is *not* the path here.** ADR-0026 reserves `color-mix()` for tint/hover/border surfaces composed against `--ac-bg`, not for solid foreground fills like glyph icons. 0037 adds named colour tokens directly, not a tint family.
- **Per-platform Playwright baselines.** Both `-darwin.png` and `-linux.png` baselines are checked in for every visual-regression screenshot. The implementer must capture both — running `npx playwright test --update-snapshots` on macOS produces darwin baselines; Linux baselines come from CI or a Linux container.
- **GlyphDocTypeKey via `Exclude`.** The work item's `Exclude<DocTypeKey, 'templates'>` is correct today but silently degrades if a second virtual key is added. Consider documenting this in a TSDoc comment so future contributors know the invariant.
- **`getComputedStyle(svg).fill` in JSDOM.** AC #4 (`0037-glyph-component.md:69`) and AC #8 (`0037-glyph-component.md:74`) both assert `getComputedStyle(glyphSvg).fill` resolves to a non-empty colour string. JSDOM's CSS engine does not match browser behaviour for `var()` substitution in SVG presentation attributes — the implementer should test this empirically before committing to that assertion; a fallback to `getAttribute('fill')` + a separate Playwright-asserted resolution check may be needed.

## Historical Context

### ADR-0026 — CSS design-token application conventions
`meta/decisions/ADR-0026-css-design-token-application-conventions.md`

- Governs **post-0033** token *application*, not authoring of new categories. The ADR is silent on per-doc-type colour tokens.
- "Replace hardcoded tint backgrounds and borders with `color-mix()` computed against the current surface token" — locked percentages (8/18/30 %) for semantic colours only. Per-doc-type tints would need explicit recording in the ADR if introduced.
- Irreducible categories table allows fixed component icon pixels (`14px`, `5px`) — confirms 16/24/32 px Glyph sizes are appropriate as literal numbers, not tokens.

### 0033 Design Token System
`meta/plans/2026-05-06-0033-design-token-system.md`, `meta/research/codebase/2026-05-06-0033-design-token-system.md`

- Established the MIRROR-A/MIRROR-B byte-equivalent duplication pattern and the parity-test approach 0037 must extend.
- Locked the seven `tokens.ts` exports (`LIGHT_COLOR_TOKENS`, `DARK_COLOR_TOKENS`, etc.) — adding new exports for doc-type colours is allowed; modifying the export list requires a parity-test update.
- Migration-test invariants enforce "every `var(--NAME)` reference resolves to a key in the union of the declared token exports" — any new `--ac-doc-<key>` must be declared in a frozen TS export.
- Research explicitly noted palette extension as a permitted option ("extend the palette with `--ac-err-tint`…"; "the small palette extension"). 0037 is exercising that latent affordance.
- WCAG AA contrast considerations were applied to the existing palette. Per-doc-type colours used as icon foregrounds against `--ac-bg` / `--ac-bg-card` should pass equivalent AA — this isn't a 0037 Acceptance Criterion but is implicitly required.

### Design gap analysis
`meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`

- Original gap entry described **9** colour-coded doc types ("red Decision, orange Research, blue Plan, purple Plan review, green Validation, teal PR, mauve PR review, red-pin Note, dark-red Work item"). The updated library-view screenshots and 0037 expanded scope to **12** non-virtual types.
- "Per-document detail screen redesign and the screen-level drift items… should come last, since they depend on Glyph, Chip, and the Page wrapper being available" — confirms Glyph as a load-bearing dependency for 0036, 0040, 0041, 0042, 0043, 0053, 0054, 0055.

### Theme + font-mode toggles (0034) and Topbar (0035)
`meta/research/codebase/2026-05-08-0034-theme-and-font-mode-toggles.md`, `meta/research/codebase/2026-05-07-0035-topbar-component.md`

- Sibling component-pattern blueprints. The `TopbarIconButton` exported-Props convention with JSDoc came out of 0035 and is the closest match for Glyph's external-facing atom shape.
- 0034 confirmed the theme-attribute → CSS-cascade pattern that Glyph relies on for zero-state theme switching.

### Visualiser implementation context
`meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md`

- General visualiser frontend architecture (TanStack Router, Vite, Vitest, the components-folder convention).

## Related Research

- `meta/research/codebase/2026-05-06-0033-design-token-system.md` — Design token system foundation
- `meta/research/codebase/2026-05-07-0035-topbar-component.md` — Sibling component-pattern blueprint
- `meta/research/codebase/2026-05-08-0034-theme-and-font-mode-toggles.md` — Theme-attribute cascade
- `meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md` — Visualiser frontend architecture
- `meta/research/codebase/2026-05-02-design-convergence-workflow.md` — How design-gap and design-inventory docs feed work items

## Open Questions

1. **Frontend README does not exist.** The work item names "a 'Developer routes' section in `skills/visualisation/visualise/frontend/README.md`" — but no README file exists. The implementer should either create the README from scratch (with at minimum Overview / Dev / Test / Build / Developer routes sections) or refine the work item to relocate the dev-routes documentation. Recommend creating the README.

2. **Playwright spec location and snapshot path.** Work item specifies `e2e/glyph.spec.ts` with baselines at `e2e/__snapshots__/`. Project convention places visual-regression specs under `tests/visual-regression/` with baselines at `tests/visual-regression/__screenshots__/<spec>.ts-snapshots/`. The `e2e/` directory is for functional tests, not visual-regression. Recommend reconciling the work item to either extend `tests/visual-regression/tokens.spec.ts` or create `tests/visual-regression/glyph-showcase.spec.ts`.

3. **`maxDiffPixelRatio` discrepancy.** Work item specifies `0.005`; existing precedent uses `0.05`. The tighter threshold is liable to cause cross-platform snapshot flakes. Recommend the work item adopt `0.05` to match precedent.

4. **`getComputedStyle(svg).fill` in JSDOM.** AC #4 and AC #8 assume browser-equivalent CSS-variable resolution under JSDOM. This may not hold; the Vitest assertion may need to fall back to `getAttribute('fill') === 'var(--ac-doc-<key>)'` with the resolved-hex assertion deferred to Playwright. Worth verifying experimentally during implementation.

5. **Icon-asset packaging — left open by the work item itself.** Per-component SVGs vs sprite vs inline-SVG map keyed by `DocTypeKey`. Codebase precedent (Brand, ThemeToggle, FontModeToggle, SseIndicator) is inline JSX SVG. For 12 doc types × 3 sizes, an inline-SVG map keyed by `GlyphDocTypeKey` is the natural match — single `viewBox`-based shape per doc type, sized via `width`/`height` on the rendered `<svg>`. No sprite-sheet precedent exists in the codebase.

6. **Forced-colors fallback.** Not in the Acceptance Criteria. Windows High Contrast users will see all 12 glyphs in a single colour (since `var()` resolves to a system colour under `forced-colors`). The implementer may want to add `@media (forced-colors: active)` rules to differentiate by stroke or shape; following `TopbarIconButton.module.css:29-33` precedent. Worth flagging but not blocking.

7. **`GlyphDocTypeKey` robustness.** `Exclude<DocTypeKey, 'templates'>` is correct today but silently expands if a new virtual key is added. A `virtual: false`-derived const would be safer but harder to express in types. Probably tolerable with a TSDoc comment.
