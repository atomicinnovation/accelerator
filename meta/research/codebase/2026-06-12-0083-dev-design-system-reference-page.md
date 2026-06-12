---
type: codebase-research
id: "2026-06-12-0083-dev-design-system-reference-page"
title: "Research: DevDesignSystem Reference Page (0083)"
date: "2026-06-12T22:17:22+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0083"
parent: "work-item:0083"
topic: "DevDesignSystem Reference Page (0083)"
tags: [research, codebase, visualiser, design-system, dev-tools, router, visual-regression, theming, scroll-spy, keybind]
revision: "442b08ff4f2d54a55395f5b53b76d7f378c317c1"
repository: "accelerator"
last_updated: "2026-06-12T22:17:22+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: DevDesignSystem Reference Page (0083)

**Date**: 2026-06-12 22:17 UTC
**Author**: Toby Clemson
**Git Commit**: 442b08ff4f2d54a55395f5b53b76d7f378c317c1
**Branch**: detached HEAD (jj change `pxvmrwymllmk`)
**Repository**: accelerator

## Research Question

Research the codebase to support implementing work item
[[0083-dev-design-system-reference-page]] — a single consolidated
`DevDesignSystem` reference page (24 sections, light + dark, activated by
`#dev`/`#dev/<section>` hash, a modifier keychord, or a sidebar-foot
triple-click) that reproduces every prototype design-system primitive at
content fidelity and retires the five existing dev showcase routes while
migrating their visual-regression coverage.

The goal: establish the **current live-app state** against the prototype
content spec (`view-dev.jsx`), confirm which primitives already exist,
and surface the gaps, net-new plumbing, and risks a plan must address.

All file/line references are at revision `442b08ff`. The shipping app is
TypeScript/TSX under `skills/visualisation/visualise/frontend/`.

## Summary

The work item is well-specified and most primitives exist live, but the
research surfaced **several divergences from the work item's stated
assumptions** that should be resolved during planning. In priority order:

1. **No unified `Icon` component / `ICON_NAMES` registry exists in the
   live app.** This is the single largest content-fidelity gap. The
   prototype renders a 33-icon `Icon` primitive (Icons section, Overview
   stroke-icon count, and `<Icon name="…">` usages inside Buttons /
   Sidebar nav / form). The live app has only scattered, hand-written
   per-route icon components (no name registry). Reaching fidelity on
   the **Icons** section (and several `<Icon>`-using sub-sections)
   requires building a new `Icon` + `ICON_NAMES` primitive — an upstream
   dependency the work item's Assumptions anticipate but do not yet name.

2. **`TYPE_META` and `STAGES` do not exist under those names**, and the
   live doc-type / stage sets differ in size from the prototype. Live
   `DOC_TYPE_KEYS` = **13** doc types (no `root-cause-analyses`); the
   prototype `TYPE_META` = **14** (RCA was just added, commit
   `6bd7276e8`). Live `PipelineMini` renders **8** workflow stages
   (`WORKFLOW_PIPELINE_STEPS`), not the prototype's **9** `STAGES`. The
   work item's instruction to bind these counts to *live* source-set
   lengths is exactly right; the oracle's frozen parentheticals
   (`12`/`13`/`9`) are advisory and now stale-low.

3. **The "24 sections" count is correct — there is no missing section.**
   (A sub-agent miscounted `DEV_SECTIONS` as 23 via an inclusive-range
   error; `view-dev.jsx:16-39` is 24 entries, ending in Topbar, matching
   the work item.) "Root cause analyses" is a doc *type* that appears
   *inside* the glyph/stage/hue sections, **not** a 24th section. Do not
   author a phantom RCA section.

4. **There is zero URL-hash handling in the app today** — routing is
   purely TanStack path-based browser history. `#dev`/`#dev/<section>`
   activation must be built from scratch *outside* the router (a
   `hashchange` listener + an overlay/conditional render), not as a
   TanStack route. This also affects "exit to app restoring the prior
   route" (an overlay keeps the underlying route mounted, making restore
   trivial) and the keychord (which should toggle `location.hash`, not
   `router.navigate` as the work item's Technical Notes suggest).

5. **Several token/primitive shapes differ** and need a port decision:
   live `Brand` (= "AtomicMark") has **no props** (fixed 28px + wordmark)
   so the Atomic-mark size ramp can't render as-is; live `Glyph` size is
   locked to the union `16|24|32` so the prototype's 22/28/36/48 sizes
   need the union widened; live status/verdict badges are **three**
   components (`StatusBadge` + `VerdictBadge` + `ResultBadge`); radii are
   a px-ladder (`--radius-0..-12`, ADR-0039) not `sm/md/lg`; there is no
   `--shadow-brand`; `--ac-violet` has no dark value.

6. **The VR migration surface is ~9 spec files, not 5** — each pixel
   showcase has a companion resolved-colour/style spec, and
   `code-syntax-showcase.spec.ts` uses a **full-page** screenshot that
   must become a clipped locator. Specs key off route-specific
   `data-testid`s and descendant selectors that the consolidated page
   must reproduce verbatim; baselines are filename-keyed and per-OS
   (`darwin`/`linux`), so spec renames orphan baselines.

The activation plumbing (keychord guards, theme toggle, scroll-spy
values, sidebar-foot host) is all cleanly portable, with strong existing
patterns to copy. Detailed findings and a fidelity matrix follow.

## Detailed Findings

### 1. Activation architecture — router, hash, keychord, triple-click

**Router is purely path-based; no hash handling exists.**
`createRouter({ routeTree })` is called with no `history` option
(`src/router.ts:218`), so TanStack defaults to browser/path history;
mounted via `<RouterProvider router={router}/>` in `src/main.tsx:6,18`.
An exhaustive search for `location.hash`, `hashchange`, `#dev`,
`createHashHistory` returned **nothing** router-related. Memory history
is used only in tests (`src/test/router-fixtures.ts`,
`src/test/router-helpers.tsx`). The exported singleton is
`export const router` (`src/router.ts:218`); the only production
`router.navigate(...)` call site is `src/components/Breadcrumbs/Breadcrumbs.tsx:39`
(there are **no** `useNavigate` usages — everything else is `<Link>`).

**Implication:** `#dev`/`#dev/<section>` must be a parallel activation
channel built outside the router. Two viable mounting strategies (the
RootLayout agent confirmed both fit the current shell):
- **Overlay (recommended):** a `useHashDevMode()` hook listening to
  `window.hashchange`, rendering `<DevDesignSystem/>` as a sibling of
  `<div className={styles.root}>` inside the providers (the same slot as
  `<Toaster/>` at `RootLayout.tsx:104-105`). "Exit to app" just flips
  the hash off; the underlying route never unmounted, so the prior route
  is restored for free. This mirrors the prototype, which used the hash
  as the single source of truth.
- **Route:** register an uncrumbed `/dev` route alongside the showcase
  routes (`src/router.ts:211-215`) and bridge the hash to it. More
  machinery; "restore prior route" needs explicit history bookkeeping.

**404 for retired routes is nuanced.** There is **no**
`notFoundComponent`/`defaultNotFoundComponent` configured anywhere
(`createRouter` gets only `{ routeTree }`). The only redirect-to-safety
is type-level: an unknown `/$type` throws `redirect({ to: "/library" })`
in `parseParams` (`src/router.ts:107-117`, tested `src/router.test.tsx:80-85`).
Removing the five showcase routes means their paths fall to TanStack's
**built-in default** not-found behaviour (an SPA "404", not an HTTP 404).
The plan should decide whether the AC's "returns 404" is satisfied by
the default not-found UI or warrants adding a `notFoundComponent`.

**Keychord pattern (clean to copy, but the navigation mechanism differs
from the work item's note).** The global `/` keybind lives in
`src/components/RootLayout/RootLayout.tsx`:
- `useEffect` handler at `:52-63` (`document.addEventListener("keydown", …)`,
  matching cleanup, empty deps).
- Guard `isPlainSlashKey` (`:20-28`) and `isEditableTarget` (`:30-39`,
  with a JSDOM `contenteditable` fallback).
- **Correction to the work item's Technical Notes:** the `/` handler
  does **not** navigate — it focuses the sidebar search input via a ref
  (`searchInputRef`, `RootLayout.tsx:50,92` → `Sidebar.tsx:35`). The work
  item says the chord would navigate "via the exported router
  `router.navigate(...)`"; under the recommended hash-overlay model the
  chord should instead **toggle `location.hash`** (exactly as the
  prototype does — `Accelerator Visualiser.html:176-189`). Cross-platform
  modifier detection precedent: `Breadcrumbs.tsx:37`
  (`e.metaKey || e.ctrlKey`). When Shift is held, `event.key` for L is
  uppercase `"L"` — match both cases.
- **Test home:** the `"global / keybind"` describe block at
  `RootLayout.test.tsx:73-196` (vitest + @testing-library + user-event).
  The `preventDefault` template is `:181-195` (construct a raw
  `KeyboardEvent({bubbles, cancelable})`, `vi.spyOn(event, "preventDefault")`,
  dispatch on `document`, assert the spy). Modifier negative cases use
  `user.keyboard("{Meta>}/{/Meta}")` etc. (`:130-168`). This is the exact
  template for the new chord's AC test.
- **Prototype chord = `⌘⇧D`** (`Accelerator Visualiser.html:178`,
  `e.preventDefault()`), which the work item forbids porting (Chromium
  reserves it for "bookmark all tabs"). The marquee hint
  `view-dev.jsx:160` (`⌘⇧D toggles`) and footer hint `view-dev.jsx:859-861`
  (`press ⌘⇧D to leave`) must be retargeted to the chosen chord via a
  single shared constant (suggested `Cmd/Ctrl+Shift+L`).

**Sidebar-foot triple-click host is net-new.** The live `Sidebar`
(`src/components/Sidebar/Sidebar.tsx`) has **no** foot/footer/version
element (grep-confirmed; CSS ends at `Sidebar.module.css:486` with no
`.foot` rule). Insertion point: as the last child of the `<nav>`, between
the META block (ends `Sidebar.tsx:171`) and `</nav>` (`:172`); pin with
`margin-top:auto`. The prototype model is `.ac-sidebar__foot` markup at
`app-shell.jsx:111-117` with the triple-click counter at `app-shell.jsx:7-20`
(3 clicks within a 600 ms window → activate).

### 2. The 24-section content map & live-component fidelity matrix

`DEV_SECTIONS` (`view-dev.jsx:16-39`) is **24** sections, an exact match
to the work item, ending in Topbar. Slugs are stable lowercase ids used
in `#dev/<id>` and differ from labels in many cases (the canonical
example: slug `colors` vs label "Colours"; also `radii`, `glyphs`,
`bigglyphs`, `mark`, `badges`, `stagedots`, `tierpills`, `form`, `nav`,
`table`, `code`, `empty`, `toast`).

Fidelity status of each primitive against the live app (the de-risking
core of this research):

| # | Section | Live component(s) | Status / gap |
|---|---------|-------------------|--------------|
| 01 | Overview | composed | OK — but the stroke-icon count card needs a live `Icon`/`ICON_NAMES` (see #06); doc-type count = live `DOC_TYPE_KEYS.length` (**13**), font families = 3 |
| 02 | Colours | tokens | OK — renders 8 surfaces + 4 fg + 8 accent/status + 3 strokes + 19 named `--atomic-*` + doc-type hues; live `--atomic-*` file has **37** entries (19 named + aliases/overlays) so assert the 19 by name; doc-type hues = `DOC_TYPE_HUE` (13) |
| 03 | Type | CSS | OK — 7 ramps across Sora/Inter/Fira Code |
| 04 | Spacing | tokens | OK — `--sp-1..--sp-11` (11) exist (`global.css:196-206`) |
| 05 | Radii & shadows | tokens | **Naming gap** — live radii are a px-ladder `--radius-0..-12` + `--radius-pill` + `--radius-full` (ADR-0039), not `sm/md/lg/pill`; shadows are `--ac-shadow-soft`/`--ac-shadow-lift` (+ invariant `--shadow-card/-card-lg/-crisp`), **no `--shadow-brand`** (prototype "brand" = `--shadow-card`). Re-express the section against live names |
| 06 | Icons | **none** | **BLOCKING GAP** — no unified `Icon` / `ICON_NAMES` (33) exists; only ad-hoc per-route icons. Needs a new primitive to reach fidelity |
| 07 | Doc-type glyphs | `Glyph` (`components/Glyph/Glyph.tsx`) | OK with caveat — `Glyph` covers 13 `DocTypeKey`s; **size locked to union `16|24|32`** so prototype sizes 22/28/36/48 need the union widened. Live count = **13** (not the oracle's "12") |
| 08 | Empty-state glyphs | `BigGlyph` (`components/BigGlyph/BigGlyph.tsx`) | OK — freely scalable `size`; covers 13 `DocTypeKey`s. Live count = **13** (not the oracle's "13" being prototype's 14); sizes 48/64/80/96/128 fine |
| 09 | Atomic mark | `Brand` (`components/Brand/Brand.tsx`) | **GAP** — `Brand` has **no props**, fixed 28px, always bundles the "Accelerator/VISUALISER" wordmark. Cannot render the 20/24/32/48/72 size ramp or a mark-only variant without extracting a parameterised mark SVG |
| 10 | Chips | `Chip` (`components/Chip/Chip.tsx`) | OK — all 6 tones (neutral/indigo/green/amber/red/violet) × sm/md = 12. Caveat: `--ac-violet` is light-only (no dark token) |
| 11 | Status badges | `StatusBadge` + `VerdictBadge` (+ `ResultBadge`) | OK but split — 8 statuses via `StatusBadge`+`FrontmatterChip`, 4 verdicts via `VerdictBadge`. The combined vocabulary is covered across 3 components, not 1 |
| 12 | Stage dots | `PipelineMini` (`components/PipelineMini/PipelineMini.tsx`) | **Count divergence** — renders `WORKFLOW_PIPELINE_STEPS` = **8** stages, not the prototype's 9 `STAGES`. Bind the assertion to the live constant (8), not the oracle's "9" |
| 13 | Tier pills | (CSS `.ac-tier-pill` in prototype) | Verify a live tier-pill primitive exists; prototype renders present/active/absent/default |
| 14 | Buttons | Topbar buttons / `FilterPill` / `ds-link` | OK — but several use `<Icon name=…>` (depends on #06) |
| 15 | Inputs & form | live search input + `FilterPill` option rows | OK — live search kbd is `/` not `⌘K`; checkbox/count primitive is `FilterPill.tsx:138-167` (the prototype's `⌘K` + `ac-filter__opt` are reproduced verbatim on the dev page) |
| 16 | Sidebar nav | `Sidebar` JSX patterns | OK — group label/sub-label/default/active/faded exist; note live "unseen dot" is **static** (`.dot`), the prototype's `ac-pulse` is **animated** — reproduce the pulse for fidelity |
| 17 | Cards | lifecycle / `WorkItemCardPresentation` / `RelatedCluster` / empty | OK — kanban card = `WorkItemCardPresentation` (same component the kanban showcase uses) |
| 18 | Tables | library table | OK — library table with a selected row |
| 19 | Markdown | `MarkdownRenderer` (`components/MarkdownRenderer/`) | OK — react-markdown + remark-gfm; **wiki-links only render when a `resolveWikiLink` resolver prop is passed** — the section must supply one |
| 20 | Code blocks | `MarkdownRenderer` (rehype-highlight, story 0076) | OK — TS confirmed; **Bash** works via highlight.js default common set but has no current fixture/assertion. Live chrome header has a lang label but **deliberately omits** the prototype's traffic-light dots |
| 21 | Frontmatter | `FrontmatterTable` (`components/FrontmatterTable/`) | OK — `<dl>` key/value rows with resolver-driven links |
| 22 | Empty & banners | live empty/banner | OK — inline empty + warn banner |
| 23 | Toasts | `Toaster`/`ToastContext`/`ExternalEditToast` (story 0039) | OK — ok/warn/err, dismissible |
| 24 | Topbar | `Topbar` (`components/Topbar/`) | OK — brand+sub-label, breadcrumbs, OriginPill, SseIndicator, ThemeToggle (5 parts) |

**Shared constants reconciliation (prototype name → live reality):**
- `ICON_NAMES` (33) → **does not exist** live.
- `TYPE_META` (14 incl. RCA) → decomposed into `DOC_TYPE_HUE`
  (`styles/tokens.ts:11-25`), `DOC_TYPE_TOKEN_KEY`/`DOC_TYPE_COLOR_VAR`
  (`components/Glyph/Glyph.constants.ts:12-44`), `DOC_TYPE_LABELS`
  (`api/types.ts:68-82`), and `DOC_TYPE_KEYS` (**13**, `api/types.ts:23-37`).
  Live key spellings differ (`work-items` vs prototype `work`,
  `work-item-reviews` vs `work-reviews`; **no `root-cause-analyses`**).
- `STAGES` (9) → `LIFECYCLE_PIPELINE_STEPS` (**11** total =
  8 `WORKFLOW_PIPELINE_STEPS` + 3 `LONG_TAIL_PIPELINE_STEPS`,
  `api/types.ts:284-368`); `PipelineMini` renders the 8 workflow stages.

### 3. Token & theming system

Single source: `src/styles/global.css`, mirrored as drift-tested TS maps
in `src/styles/tokens.ts`.
- **Light `--ac-*`** in `:root` (`global.css:76-334`): surfaces
  (`:80-87`, 8), foreground (`:88-91`, 4), strokes (`:92-94`, 3), accent/
  status (`:95-102`, 8 incl. light-only `--ac-violet`).
- **`--atomic-*` brand palette** (`global.css:247-283`): **37** entries
  (`BRAND_COLOR_TOKENS`, `tokens.ts:262-300`), incl. 4 `var()` aliases and
  3 overlay/shadow tokens; declared theme-invariant (ADR-0026/0035).
- **Dark values:** explicit `[data-theme="dark"]` block (`global.css:341-406`,
  "MIRROR-A", canonical) + a `@media (prefers-color-scheme: dark)` mirror
  (`:412-473`, "MIRROR-B"). E.g. `--ac-bg` is `--atomic-bone` (#fbfcfe)
  light vs `--atomic-night-2` (#0a111b) dark.
- **`data-theme` is set on `document.documentElement` (`<html>`)** in two
  places: the React effect `use-theme.ts:38-40` and the pre-mount boot
  script `boot-theme.ts:21-28` (FOUC guard). This is exactly what the VR
  specs write (`document.documentElement.dataset.theme = "dark"`).
- **Theme toggle:** `ThemeToggle` (`components/ThemeToggle/ThemeToggle.tsx`)
  mounted in `Topbar.tsx:19-21`, backed by React Context + `useState` +
  `localStorage` (`api/use-theme.ts`, `useTheme` factory `:32-58`,
  `THEME_STORAGE_KEY="ac-theme"`). The owning hook runs once in
  `RootLayout.tsx:47` and provides `<ThemeContext.Provider>` (`:80`). **The
  dev page's in-page theme toggle can reuse this end-to-end** — render
  `<ThemeToggle/>` (or call `useThemeContext().toggleTheme()`) anywhere
  under `RootLayout`'s `<Outlet/>` subtree; no new state needed. The
  `FontModeToggle` is a ready second template.
- **Doc-type hues:** numeric `DOC_TYPE_HUE` (`tokens.ts:11-25`) + the
  `--ac-doc-*` / `--ac-doc-bg-*` glyph tokens (`global.css:107-133`,
  dark `:366-391`) + `--ac-stage-*` accents (`:143-150`; long-tail stages
  have none).
- **Spacing/radii/shadows:** `--sp-1..--sp-11` (`:196-206`); radii px-ladder
  `--radius-0..-12` + `--radius-pill`/`--radius-full` (`:214-223`); shadows
  `--shadow-card/-card-lg/-crisp` + `--ac-shadow-soft`/`--ac-shadow-lift`
  (`:226-230`, dark `:403-404`).

### 4. Scroll-spy + hash sync (net-new)

The prototype `IntersectionObserver` (`view-dev.jsx:103-122`) is exactly
portable in its values:
- `root` = the scroll container **passed explicitly** (`document.querySelector(".ac-main")`).
- `rootMargin: "-80px 0px -55% 0px"`, `threshold: [0, 0.25, 0.5]`.
- Writes the active id back via `history.replaceState(null, "", "#dev/<id>")`
  and drives the TOC `is-active` class via `setSection(id)`.
- Deep-link landing reads `location.hash.split("/")[1]` (`:95-98`) and
  `jump(id)` scrolls `.ac-main` to `el.offsetTop - 60` (`:124-131`).

**Live scroll root** = `<main className={styles.main}>` rendered by
`RootLayout.tsx:94-96`; `.main` is the sole scroller
(`overflow-y: scroll; flex:1; min-height:0` — `RootLayout.module.css:24-39`).
The window/document does **not** scroll. Because `.main` carries only a
**CSS-module** class (no `id`/`data-*`), the observer cannot select it by
a stable global selector — **a ref or `data-*` hook must be added** to
that `<main>`.

**The pinned-to-Colours defect:** the prototype callback inspects only
the entries from one observer dispatch and picks the highest
`intersectionRatio`; with the tall `-55%` bottom margin and coarse
thresholds, the tallest section (Colours) wins on load and adjacent short
sections never produce a higher-ratio entry, so updates stall. The fix
(per AC): recompute the active section from **all** observed sections'
positions on every intersection (or from actual scroll position), and
never pin to one section.

### 5. Visual-regression migration surface

Config: `playwright.config.ts` — two Chromium projects,
`visual-regression` (`testDir ./tests/visual-regression`,
`snapshotDir ./tests/visual-regression/__screenshots__`) running before
the `chromium` e2e project; `workers: 1`. **Themes are not projects** —
each spec loops `["light","dark"]` internally and sets dark via
`document.documentElement.dataset.theme = "dark"`. Baselines use
Playwright's default naming: `<spec>.spec.ts-snapshots/<name>-visual-regression-<os>.png`
with **both `darwin` and `linux`** committed (linux baselines drift behind
darwin — regen via the "Update visual regression baselines" workflow).

The migration is **~9 spec files, not 5** (each pixel spec has a
resolved companion):

| Showcase → section | Pixel spec | Resolved spec | Key selectors to reproduce |
|---|---|---|---|
| glyph → Doc-type glyphs | `glyph-showcase.spec.ts` | `glyph-resolved-fill.spec.ts` | `[data-testid="glyph-cell-<type>-<size>"]` |
| big-glyph → Empty-state glyphs | `big-glyph-showcase.spec.ts` | (dark commit-gate only) | `[data-testid="big-glyph-cell-<type>"]` |
| chip → Chips | `chip-showcase.spec.ts` | `chip-resolved-colours.spec.ts` | `[data-testid="chip-cell-<variant>-<size>"]`, `[data-variant]` |
| code-syntax → Code blocks | `code-syntax-showcase.spec.ts` (**full-page**) | `code-block-resolved-colours.spec.ts` | `[data-testid="code-syntax-cell-<lang>"]`, `.hljs-*`, `[data-language]` |
| kanban-card → Cards | `kanban-card-showcase.spec.ts` | `kanban-card-resolved-styles.spec.ts` | `[data-testid="kanban-card-cell-<state>"] .ac-kcard` |

Migration constraints:
- The consolidated page sections must reproduce the exact `data-testid`
  cells **and** descendant selectors (`.ac-kcard`, `.hljs-*`,
  `[data-language]`, `[data-variant]`) the specs assert against.
- `code-syntax-showcase.spec.ts:16` snapshots `expect(page)` (whole
  viewport) — must become a clipped section locator when moved off a
  dedicated route.
- Baselines are keyed by **spec filename folder**; renaming a spec orphans
  every PNG unless the folder is renamed and screenshot `name` args kept
  stable. Resolved specs have **no** PNG baselines (computed-value only).
- Shared helpers: `tests/visual-regression/helpers.ts` (`applyTheme`) and
  `tests/visual-regression/lib/expected-colours.ts` (`setTheme`,
  `hexToRgb`, `parseRgb`, `resolveToken`, `EXPECTED_COLOR`). All write
  `data-theme` on `<html>`, so a page that respects
  `document.documentElement.dataset.theme` satisfies all of them.
- A separate `playwright.glyph.config.ts` exists (per the README) for
  regenerating glyph baselines.

### 6. README & developer-routes docs

`skills/visualisation/visualise/frontend/README.md`:
- "Developer Routes" section (`:44-48`) lists **only** `/glyph-showcase`
  (the other four were never documented) and says "all 12 doc-type
  Glyphs" — **already stale** (live `DOC_TYPE_KEYS` = 13). This section
  must be rewritten to list `DevDesignSystem` and drop the showcases.
- A second block, "Glyph showcase specs" (`:67-77`), describes
  regenerating glyph baselines via `playwright.glyph.config.ts` — also
  needs attention when consolidating.

## Code References

- `skills/visualisation/visualise/frontend/src/router.ts:152-216` — five
  showcase routes; consolidation/migration warning at `:158-162`;
  `routeTree` `:199-216`; exported `router` `:218`; no `notFoundComponent`.
- `skills/visualisation/visualise/frontend/src/main.tsx:6,18` — production
  router mount (browser/path history).
- `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.tsx:20-39,52-63` —
  keybind guards + `/` handler (focuses search, does not navigate);
  `:47,80` theme provider; `:94-96` scroll-root `<main>` + `<Outlet/>`;
  `:104-105` Toaster overlay slot.
- `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.module.css:24-39` —
  `.main` is the sole scroller.
- `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.test.tsx:73-196` —
  `"global / keybind"` test block; `preventDefault` template `:181-195`.
- `skills/visualisation/visualise/frontend/src/components/Breadcrumbs/Breadcrumbs.tsx:37,39` —
  cross-platform modifier guard + only `router.navigate` call site.
- `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx:30-172` —
  sidebar structure; no foot element; insert foot host at `:171→172`.
- `skills/visualisation/visualise/frontend/src/components/FilterPill/FilterPill.tsx:138-167` —
  checkbox/count primitive for the "Inputs & form" section.
- `skills/visualisation/visualise/frontend/src/components/Glyph/Glyph.tsx:39-55` —
  `Glyph` props; size union `16|24|32` at `:41`.
- `skills/visualisation/visualise/frontend/src/components/BigGlyph/BigGlyph.tsx:48-59` —
  `BigGlyph` props (freely scalable `size`).
- `skills/visualisation/visualise/frontend/src/components/Brand/Brand.tsx` —
  `Brand` (= AtomicMark), no props, fixed 28px + wordmark.
- `skills/visualisation/visualise/frontend/src/components/Chip/Chip.tsx:4-21` —
  6 tones × sm/md.
- `skills/visualisation/visualise/frontend/src/components/StatusBadge/StatusBadge.tsx`,
  `…/VerdictBadge/VerdictBadge.tsx`, `…/ResultBadge/ResultBadge.tsx` —
  status/verdict/result split; mappings in `api/status-variant.ts`,
  `api/verdict-variant.ts`.
- `skills/visualisation/visualise/frontend/src/components/PipelineMini/PipelineMini.tsx:5-18` —
  renders 8 `WORKFLOW_PIPELINE_STEPS`.
- `skills/visualisation/visualise/frontend/src/api/types.ts:23-37,284-368` —
  `DOC_TYPE_KEYS` (13), `LIFECYCLE_PIPELINE_STEPS` (11)/`WORKFLOW…`(8)/`LONG_TAIL…`(3).
- `skills/visualisation/visualise/frontend/src/styles/tokens.ts:11-25,262-300` —
  `DOC_TYPE_HUE`, `BRAND_COLOR_TOKENS` (37).
- `skills/visualisation/visualise/frontend/src/styles/global.css:76-334,341-473` —
  `--ac-*` light/dark, `--atomic-*`, spacing/radii/shadows.
- `skills/visualisation/visualise/frontend/src/api/use-theme.ts:32-58` &
  `src/api/boot-theme.ts:21-28` — `data-theme` on `<html>`, theme context.
- `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.tsx:131-235` —
  markdown + rehype-highlight + code chrome header (no dots);
  `wiki-link-plugin.ts` (resolver-gated).
- `skills/visualisation/visualise/frontend/src/components/FrontmatterTable/FrontmatterTable.tsx:103-121` —
  frontmatter `<dl>` rows with links.
- `skills/visualisation/visualise/frontend/src/routes/{glyph-showcase,big-glyph-showcase,chip-showcase,code-syntax-showcase,kanban-card-showcase}/*.tsx` —
  the five showcase page components (content to port).
- `skills/visualisation/visualise/frontend/playwright.config.ts` &
  `tests/visual-regression/*.spec.ts` + `__screenshots__/` — VR surface.
- `skills/visualisation/visualise/frontend/README.md:44-48,67-77` —
  developer-routes docs to update.

## Architecture Insights

- **Hash activation is orthogonal to TanStack routing.** The cleanest
  design treats `#dev` as an app-level overlay driven by a `hashchange`
  hook, keeping the underlying route mounted so "exit to app" is a
  no-navigation hash flip. This sidesteps both the missing hash-history
  support and the "restore prior route" requirement, and matches the
  prototype's own model.
- **The dev page lives inside the existing provider tree.** Rendering it
  under (or as a sibling of) `RootLayout`'s shell means the theme
  context, toast context, and `<main>` scroll root are all already in
  scope — the in-page theme toggle and scroll-spy reuse existing
  infrastructure rather than re-implementing it.
- **"Port, don't import" amplifies naming drift.** The biggest planning
  risk isn't missing components — it's the *shape* mismatches (Icon
  registry absent; Brand un-parameterised; Glyph size union; badge split;
  radii ladder; stage count). Each is a small decision, but collectively
  they mean "content fidelity" is a set of deliberate mappings, not a
  mechanical copy. The work item's choice to assert **variant presence +
  live-constant-bound counts** (not pixel/integer equality) is the
  correct hedge against exactly this drift.
- **The Icon gap is a genuine upstream dependency.** Per the work item's
  Assumptions ("any primitive the prototype shows that does not yet exist
  surfaces a dependency"), the absent `Icon`/`ICON_NAMES` should be
  raised as a blocker/sub-task before the Icons section (and the
  `<Icon>`-using Buttons/Sidebar-nav/form sub-sections) can reach
  fidelity. Decide scope: build a full 33-icon registry, or a curated
  subset matching what the live app actually uses.
- **VR is the correctness oracle, and it's filename- and OS-coupled.**
  Preserving `data-testid` contracts and screenshot `name` args (and
  regenerating darwin+linux baselines) is the safest migration path;
  converting the one full-page code-syntax snapshot to a clipped locator
  is the only structural spec change forced by consolidation.

## Historical Context

- `meta/work/0083-dev-design-system-reference-page.md` — the work item
  itself (status `ready`); all distinctive topics (scroll-spy, keychord,
  `#dev`, VR migration) are specified in-line. No separate plan/decision
  exists yet.
- `meta/reviews/work/0083-dev-design-system-reference-page-review-1.md` —
  review-1 (the 2026-06-12 refinement/review that resolved the open
  questions: keychord, triple-click host, single-story sizing, kind/priority).
- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md` —
  the source design-gap analysis that spawned 0083.
- `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/`
  — the authoritative prototype: `prototype-full/src/view-dev.jsx` (+
  `ui.jsx`, `data.jsx`, `highlight.jsx`, `big-glyphs.jsx`, `app.css`),
  `prototype-full/assets/tokens.css`, the hash/keychord activation in
  `prototype-full/Accelerator Visualiser.html`, the triple-click handler
  in `prototype-full/src/app-shell.jsx`, and reference screenshots
  (`screenshots/dev-design-system.png`, `dev-overview-fullpage.png`).
- Blocked-by work items (all `done`): 0033 (tokens), 0035 (topbar), 0037
  (glyph — established the `/glyph-showcase` pattern), 0038 (chip), 0039
  (toaster), 0040 (pipeline), 0041 (library wrapper), 0076 (code-block
  highlight — `/code-syntax-showcase`), 0086 (kanban drag —
  `/kanban-card-showcase`).
- Commit `6bd7276e8` "Add root-cause-analyses doc type to the design
  prototype" — explains the upward drift of the prototype's
  `TYPE_META`/`BIG_GLYPHS`/`STAGES` counts (and why the oracle
  parentheticals read low). RCA is **prototype-only**; the live app still
  has 13 doc types.

## Related Research

- `meta/research/codebase/2026-05-21-0076-code-block-syntax-highlight-palette.md`
  — code-block highlighting internals (Code blocks section).
- `meta/research/codebase/2026-06-09-0082-big-glyph-hero-illustrations.md`
  — BigGlyph / hero-showcase context (Empty-state glyphs section + VR
  migration precedent).
- `meta/research/codebase/2026-06-01-0054-sidebar-search.md` — has a
  "Coordinates with 0083 (DevDesignSystem keybind)" section relevant to
  the chord + sidebar-foot.

## Open Questions

1. **Mounting model:** overlay-via-`hashchange` (recommended) vs a
   TanStack `/dev` route? Drives how `#dev`, deep-links, and "exit to app
   / restore prior route" are implemented.
2. **Icon primitive scope:** build a full `Icon`/`ICON_NAMES` (33) to
   match the prototype, or a curated subset of the icons the live app
   actually renders? This is the gating dependency for the Icons section
   and `<Icon>`-using sub-sections — file as a blocker?
3. **Atomic-mark size ramp:** extract a parameterised, wordmark-less mark
   SVG from `Brand`, or render the section with `Brand`'s fixed form and
   adjust the section's claimed variants?
4. **Glyph size union:** widen `Glyph`'s `16|24|32` union to admit the
   prototype's 22/28/36/48, or render the Doc-type-glyphs section at the
   live-supported sizes only?
5. **Stage-dots count:** assert against `WORKFLOW_PIPELINE_STEPS.length`
   (8, what `PipelineMini` renders) vs the oracle's "9" — confirm the
   plan uses the live constant.
6. **"Returns 404":** is TanStack's default not-found UI sufficient for
   the retired paths, or should a `notFoundComponent` be added?
7. **Chord choice + verification:** confirm `Cmd/Ctrl+Shift+L` (or
   alternative) is interceptable across Chrome/Edge/Firefox/Safari; bind
   via a single shared constant feeding the handler, marquee, and footer.
8. **VR spec/folder strategy:** rename specs/baseline folders to
   `dev-design-system-*` (and regen darwin+linux), or keep existing spec
   filenames and only repoint the routes/selectors to preserve baseline
   keys? The latter minimises baseline churn.
