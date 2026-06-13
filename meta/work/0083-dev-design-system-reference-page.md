---
id: "0083"
title: "DevDesignSystem Reference Page"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
kind: story
status: ready
priority: low
tags: [design, frontend, dev-tools, documentation]
type: work-item
schema_version: 1
last_updated: "2026-06-13T07:24:32+00:00"
last_updated_by: Toby Clemson
blocked_by: ["work-item:0033", "work-item:0035", "work-item:0037", "work-item:0038", "work-item:0039", "work-item:0040", "work-item:0041", "work-item:0076", "work-item:0086"]
source: "design-gap:2026-05-21-current-app-vs-claude-design-prototype"
---

# 0083: DevDesignSystem Reference Page

**Kind**: Story
**Status**: Ready
**Priority**: Low
**Author**: Toby Clemson

## Summary

Implement a single consolidated `DevDesignSystem` reference page in the
current app that reproduces every design-system primitive across 24
sections — at full content fidelity to the prototype — in both light and
dark themes, activated by a `#dev` / `#dev/<section>` URL fragment, a
modifier keychord that is not a reserved browser shortcut, or a
sidebar-foot triple-click, and
retire all five existing dev showcase routes in favour of the
consolidated reference, migrating their visual-regression coverage rather
than deleting it.

## Context

Developers and designers maintaining the visualiser frontend need a
single, authoritative reference for every design-system primitive — to
check a token's value, confirm a component's variant set, or eyeball
light/dark rendering — rather than the five scattered single-primitive
showcase routes that exist today.

The prototype's `DevDesignSystem` (`src/view-dev.jsx`) is a hidden
24-section reference page covering every primitive (Overview, Colours,
Type, Spacing, Radii & shadows, Icons, Doc-type glyphs, Empty-state
glyphs, Atomic mark, Chips, Status badges, Stage dots, Tier pills,
Buttons, Inputs & form, Sidebar nav, Cards, Tables, Markdown, Code
blocks, Frontmatter, Empty & banners, Toasts, Topbar) activated by
`#dev` hash, a keychord, or a sidebar-foot triple-click.

The prototype `view-dev.jsx` (and its supporting sources — `tokens.css`,
`src/app.css`, `src/ui.jsx`, `src/data.jsx`, `src/highlight.jsx`,
`src/big-glyphs.jsx`) is the **authoritative content specification** for
this page: each section's variant set was verified against it (see the
section content inventory in Technical Notes), and the implementation
must port that content to the live app's tokens and components without
dropping any token group, variant, or primitive.

The current app exposes **five** uncrumbed dev showcase routes —
`/glyph-showcase`, `/big-glyph-showcase`, `/chip-showcase`,
`/code-syntax-showcase`, and `/kanban-card-showcase` — each covering a
single primitive and serving as a Playwright visual-regression fixture
surface (`src/router.ts:152-216`). They are reached by direct URL only
(no in-app links). `src/router.ts:161-162` already designates *this*
work item as the home for consolidating them and migrating their VR
specs + baselines.

The prototype's scroll-spy defect (TOC active highlight pinned to
`02 Colours`) must not be copied; the new implementation should drive the
active highlight from the actual scroll position.

## Requirements

- Implement a single `DevDesignSystem` route covering 24 sections, one
  per primitive listed in Context.
- **Content fidelity**: each of the 24 sections reproduces the full set
  of variants shown in the prototype `view-dev.jsx` (the authoritative
  content specification — see the inventory in Technical Notes), ported
  to the live app's tokens and components. No token group, variant, or
  primitive shown in the prototype may be dropped.
- **Theme coverage**: the page renders in both the light and dark themes
  the app supports without layout breakage and with theme-appropriate
  token values (the per-section light/dark VR baselines are the
  correctness oracle); surfaces and the Colours section respond to
  `data-theme`, and the page offers an in-page theme toggle (per the
  prototype's Overview) that switches the rendered theme.
- Activation triggers (any one activates the route):
  - the `/dev` route, reached canonically at `/dev#<section>` to deep-link
    to a specific section, where `<section>` is the prototype `DEV_SECTIONS`
    id — a stable lowercase slug (e.g. `colors`), **not** the display label,
    so the Colours section is reached at `/dev#colors` even though its
    heading reads "Colours". `#dev` and `#dev/<section>` are accepted
    **activation aliases** — a `hashchange` bridge normalises them to the
    canonical `/dev#<section>` form. All net-new — the router uses
    path-based history today, with no hash handling.
  - a **modifier-based keyboard chord** that is *not* a reserved browser
    shortcut. `Cmd/Ctrl+Shift+D` is explicitly excluded — Chromium
    browsers reserve it for "bookmark all tabs" and ignore
    `preventDefault()`, so it cannot be intercepted. Suggested default:
    `Cmd/Ctrl+Shift+L`; the final chord must be verified interceptable
    across browsers (see Acceptance Criteria);
  - a triple-click on a **new minimal sidebar-foot element** (e.g. a
    version/build label). The sidebar has no foot element or version
    label today, so this affordance is net-new.
- **Scroll-spy + hash sync**: a scroll-spy TOC drives the active-section
  highlight from the actual scroll position (net-new — no
  `IntersectionObserver` or scroll-spy exists today) and updates the URL
  hash to `#<active-section>` as sections enter view; landing on
  `/dev#<section>` scrolls to that section. Do not copy the prototype's
  pinned-to-`02 Colours` defect.
- **Page chrome**: reproduce the dev-page affordances — a sticky
  table-of-contents aside with a "CONTENTS" eyebrow, two-digit numbered
  jumplinks (01–24), and an "exit to app" control; a "DEV" marquee
  identifying the page; and a footer. Any keybind hint shown in the
  marquee or footer must reflect the chosen activation chord, **not**
  `⌘⇧D`.
- Retire all five dev showcase routes (`/glyph-showcase`,
  `/big-glyph-showcase`, `/chip-showcase`, `/code-syntax-showcase`,
  `/kanban-card-showcase`): remove them from the router so their paths
  return 404 (no redirects — nothing in-app links to them), and
  **migrate each route's visual-regression spec + baselines** onto the
  corresponding `DevDesignSystem` section rather than deleting coverage.
- Update the visualiser frontend README's developer-routes section to
  list `DevDesignSystem` and remove all five retired showcases.

## Acceptance Criteria

- [ ] Given the live design system (tokens, components), when
  `DevDesignSystem` renders, then all 24 sections are present and every
  primitive currently shown by the five retired showcases appears in its
  section, checked against the showcase→section mapping in Technical
  Notes (so "every primitive" resolves to a finite written checklist, not
  an external-source reconstruction).
- [ ] Each of the 24 sections renders the full variant set sourced from
  the live tokens/components; an automated test asserts the rendered count
  per section against the per-section variant-count oracle table in
  Technical Notes (e.g. Chips 12, Status badges 12, Cards 4). Counts that
  bind to a live constant (Icons `= ICON_NAMES.length`, Stage dots
  `= STAGES.length`, Doc-type glyphs/hues `= TYPE_META` doc-type count)
  assert against that constant's length rather than a frozen integer;
  compositional sections assert presence of each named variant. Porting to
  live tokens means pixel values differ from the prototype, so the check
  is on variant presence and count, not pixel equality.
- [ ] Given the app is loaded, when the user navigates to `#dev` (or
  `#dev/<section>`), presses the activation chord, or triple-clicks the
  sidebar-foot element, then `DevDesignSystem` activates on the `/dev`
  route — observable as the path being `/dev` with the "DEV" marquee and the
  24-section TOC aside rendered (the `#dev` / `#dev/<section>` forms are
  activation aliases the bridge normalises to the canonical `/dev#<section>`
  form) — and each trigger works independently.
- [ ] Given a `/dev#<section>` URL (e.g. `/dev#colors`; the `#dev/<section>`
  alias is also accepted and normalised), when the page loads, then it
  scrolls so that section's heading sits within the scroll-spy observer's
  active region (per the chosen `rootMargin`; see Technical Notes) and that
  section's TOC entry carries the active state.
- [ ] Given `DevDesignSystem` is open, when the user scrolls a lower
  section into the scroll-spy observer's active region (per the chosen
  `rootMargin`; see Technical Notes — the same region the `/dev#<section>`
  deep-link criterion uses), then the TOC active highlight updates to that
  section **and** the URL hash updates to `#<that-section>` — asserted
  by an automated test; the highlight is never pinned to a single
  section.
- [ ] The page is captured under a light and a dark visual-regression
  snapshot per section (giving "renders under both themes" a concrete
  oracle), and switching theme changes the resolved values of the `--ac-*`
  surface tokens — asserted by reading computed styles for at least the
  Colours swatches, which must reflect the active theme's token values.
- [ ] An in-page theme toggle is present and, when activated, switches the
  rendered theme (light↔dark) — asserted by an automated test that toggles
  it and observes `data-theme` (and the resulting `--ac-*` surface token
  values) change.
- [ ] The TOC aside lists all 24 sections with two-digit numbers and
  provides an "exit to app" control that returns to the app (leaving the
  `/dev` route, clearing the section hash, and restoring the prior app
  route / sidebar). The "DEV" marquee and footer are present, and their
  keybind hint equals the chord the activation handler binds (a single
  shared constant), not just any non-`⌘⇧D` string.
- [ ] The activation chord is a modifier-based combination and is not
  `Cmd/Ctrl+Shift+D`. Verification splits in two: (a) an automated test
  asserts the chosen chord's keydown handler fires and `preventDefault()`
  is honoured; and (b) a recorded manual matrix names the exact chord and
  the browser + version + OS tested (Chrome, Edge, Firefox, Safari),
  confirming the chord delivers the keydown and suppresses the browser
  default. (Scope: browser built-in shortcuts; extension/user-custom
  bindings are out of scope. The "not reserved" property holds only for
  the tested browser versions.)
- [ ] Given all five showcase routes are removed from `src/router.ts`,
  when any former showcase path is requested, then it returns 404 (no
  redirect).
- [ ] The visual-regression spec + baselines that covered each retired
  showcase are migrated to assert the corresponding `DevDesignSystem`
  section(s), audited row-by-row against the showcase→section mapping in
  Technical Notes (glyph-showcase → Doc-type glyphs; big-glyph-showcase →
  Empty-state glyphs; chip-showcase → Chips; code-syntax-showcase → Code
  blocks; kanban-card-showcase → Cards), so each retired spec's assertions
  are reproduced and no primitive loses VR coverage.
- [ ] The frontend README's developer-routes section lists
  `DevDesignSystem` and no longer lists any of the five retired
  showcases.

## Open Questions

None outstanding. The prior open questions were resolved during
refinement (2026-06-12) and review (2026-06-12):

- **Keychord conflict** — `Cmd/Ctrl+Shift+D` collides with the browser
  "bookmark all tabs" action (reserved in Chromium; not interceptable),
  so the trigger is now a verified non-reserved modifier chord. See
  Requirements and Technical Notes.
- **Triple-click host** — the sidebar has no existing version label to
  reuse, so the triple-click affordance attaches to a new minimal
  sidebar-foot element.
- **Sizing — single story vs epic** — though the expanded scope is
  epic-sized, the work is kept as one story because its strands are
  indivisible (VR migration and per-section fidelity both require the
  sections to exist first). Indivisibility rationale recorded in Drafting
  Notes. Resolved on review (2026-06-12).
- **`kind` / `priority`** — `kind` stays `story` (not `task`) and
  `priority` stays `low`, both as deliberate calls on review
  (2026-06-12); the developer-tooling framing was considered and set
  aside in favour of treating the page as a story for the visualiser's
  maintainers.

## Dependencies

- Blocked by (all `done` — completed prerequisites recorded as a
  historical dependency graph, not active blockers): 0033
  (design-token-system), 0035 (topbar-component), 0037 (glyph-component),
  0038 (generic-chip-component), 0039
  (toaster-and-external-edit-notifications), 0040
  (pipeline-visualisation-overhaul), 0041
  (library-page-wrapper-and-overview-hub), 0076
  (code-block-syntax-highlight-palette — delivered `/code-syntax-showcase`),
  and 0086 (kanban-drag-and-drop — delivered `/kanban-card-showcase`).
  Each delivered a primitive, token set, or showcase that
  `DevDesignSystem` reproduces or consolidates; because all are complete,
  0083 is unblocked and ready to plan. (0076 and 0086 were confirmed from
  the `src/router.ts` showcase route comments during review.)
- Content fidelity is contingent on every primitive the prototype shows
  already existing in the live app. Any prototype primitive found missing
  during implementation surfaces a new upstream blocker to track before
  that section can reach fidelity (see Assumptions).
- The chosen activation chord is coupled to browser-engine
  reserved-shortcut behaviour across all four target browsers (Chrome,
  Edge, Firefox, Safari); a reservation discovered in any of them is an
  external constraint to resolve, not an unexpected blocker (see
  Acceptance Criteria).
- This work mutates two existing shared artefacts (captured in
  Requirements/Acceptance Criteria, surfaced here for record-completeness):
  the five Playwright VR specs + baselines, repointed onto `DevDesignSystem`
  sections rather than deleted (baselines must be regenerated — they drift
  between darwin/linux); and the visualiser frontend README's
  developer-routes section.
- Blocks: none.

## Assumptions

- All 24 sections can render from existing components and tokens; no
  new design content is required for the showcase itself.
- The content of all five retired showcases is fully represented within
  the 24 sections, so migrating their VR coverage does not require new
  design content.
- The live app already exposes the components and tokens the prototype
  uses (Icon/TypeGlyph/BigGlyph/AtomicMark/Chip/StatusBadge/PipelineMini,
  markdown + code-highlight renderers, the `--ac-*`/`--atomic-*` token
  sets, `TYPE_META`, `STAGES`). Any primitive the prototype shows that
  does not yet exist in the live app surfaces a dependency to resolve
  before this section can reach fidelity.

## Technical Notes

- The shipping app is **TypeScript/TSX** under
  `skills/visualisation/visualise/frontend/`. `src/view-dev.jsx` is the
  reference prototype
  (`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/src/view-dev.jsx`),
  not app source — port its patterns, don't import them.
- **Authoritative content spec**: `prototype-full/src/view-dev.jsx`
  (`DEV_SECTIONS` at lines 15-40 is the canonical section list, verified
  an exact match to this story's 24 sections). Supporting prototype
  sources that define the primitives to surface: `tokens.css` (brand
  `--atomic-*` palette), `src/app.css` (`--ac-*` tokens, `ds-*` layout
  classes), `src/ui.jsx` (Icon, TypeGlyph, BigGlyph, AtomicMark, Chip,
  StatusBadge, PipelineMini, `renderMarkdown`), `src/data.jsx`
  (`window.TYPE_META`, `window.STAGES`), `src/highlight.jsx` (code
  syntax highlighting), `src/big-glyphs.jsx` (empty-state hero glyphs).
- **Scroll-spy porting**: the prototype uses an `IntersectionObserver`
  whose `root` is the **scroll container** (`.ac-main`, not the
  viewport — pass it explicitly or the callback never fires),
  `rootMargin: "-80px 0px -55% 0px"`, `threshold: [0, 0.25, 0.5]`, and
  writes the active id back to the URL via
  `history.replaceState(null, "", "#dev/<id>")` (`view-dev.jsx:103-122`).
  In the current app the scroll container differs; bind the observer to
  the actual scroll root.
- **Section content inventory** (from `view-dev.jsx`; reproduce each at
  fidelity):
  - Overview: intro prose + 4 overview cards (font families = 3
    Sora/Inter/Fira Code; stroke-icon count = `ICON_NAMES.length` (33);
    doc-type glyph count = the live doc-type set length; themes = 2 with a
    live light/dark flip).
  - Colours: Surfaces (8), Foreground (4), Accent & status (8), Strokes
    (3), brand palette `--atomic-*` (19), doc-type hues (one per doc-type
    in `TYPE_META` — assert against the live `TYPE_META` doc-type count,
    not a frozen integer); swatches show the computed colour value.
  - Type: 7 ramps across Sora / Inter / Fira Code (hero → eyebrow).
  - Spacing: `--sp-1`..`--sp-11` (11 steps).
  - Radii & shadows: 4 radii (sm/md/lg/pill) + 3 shadows
    (soft/lift/brand).
  - Icons: full `ICON_NAMES` set (33 stroke icons = `ICON_NAMES.length`) +
    a size ramp (12–32px).
  - Doc-type glyphs: one `TypeGlyph` per doc-type (the component the
    prototype/router comments also call "Glyph" — same component), sizes
    22/28/36/48. Count = the `TypeGlyph` source set length (the retired
    `/glyph-showcase` rendered 12).
  - Empty-state glyphs: one `BigGlyph` per type, sizes 48/64/80/96/128.
    Count = the `BigGlyph` source set length (the retired
    `/big-glyph-showcase` rendered 13). The doc-type-glyph and
    empty-state-hero sets are sized independently in the live app, so each
    section's count binds to its own component's source set rather than a
    shared figure.
  - Atomic mark: sizes 20/24/32/48/72 + an on-night variant.
  - Chips: 6 tones (neutral/indigo/green/amber/red/violet) × sm/md.
  - Status badges: 8 statuses + 4 verdicts.
  - Stage dots: `PipelineMini` over `STAGES` (9 stages = `STAGES.length`)
    — all/partial/none + compact.
  - Tier pills: present/active/absent/default.
  - Buttons: topbar buttons (icon-only, filter, sort, sort-active),
    filter badge, inline link + `ds-link`.
  - Inputs & form: sidebar search input (with `⌘K` kbd), checkboxes
    (checked/unchecked, with counts).
  - Sidebar nav: group label, sub-label, default/active/with-pulse/faded
    items, with counts.
  - Cards: lifecycle card, kanban card, related-item row, empty-state
    lifecycle card (4 variants).
  - Tables: library table with a selected row.
  - Markdown: headings, bold/italic/inline code, wiki-link, ordered +
    unordered lists, table.
  - Code blocks: TypeScript + Bash fenced blocks, syntax-highlighted with
    a chrome header.
  - Frontmatter: key/value rows with links.
  - Empty & banners: inline empty state + warn banner.
  - Toasts: ok / warn / err, dismissible, with reset.
  - Topbar: brand + sub-label, breadcrumbs, origin/status pill, SSE
    (server-sent events) indicator, theme button.
- **Per-section variant-count oracle** (for the primitive-coverage and
  per-section-fidelity criteria): the count a test asserts per section.
  Counts marked `= <const>` bind to the live source constant's length
  (robust to types being added later), with the current value in
  parentheses; fixed design sets are frozen integers; compositional
  sections assert presence of each named variant rather than a single
  total.

  | #  | Section | Count to assert |
  |----|---------|-----------------|
  | 01 | Overview | 4 cards |
  | 02 | Colours | 8 surfaces + 4 foreground + 8 accent/status + 3 strokes + 19 `--atomic-*` + doc-type hues (`= TYPE_META` doc-type count) |
  | 03 | Type | 7 ramps × 3 families |
  | 04 | Spacing | 11 (`--sp-1`..`--sp-11`) |
  | 05 | Radii & shadows | 4 radii + 3 shadows |
  | 06 | Icons | `= ICON_NAMES.length` (33) + size ramp |
  | 07 | Doc-type glyphs | `= TypeGlyph` source set (12) × 4 sizes |
  | 08 | Empty-state glyphs | `= BigGlyph` source set (13) × 5 sizes |
  | 09 | Atomic mark | 5 sizes + 1 on-night |
  | 10 | Chips | 12 (6 tones × 2 sizes) |
  | 11 | Status badges | 12 (8 statuses + 4 verdicts) |
  | 12 | Stage dots | `= STAGES.length` (9); 3 fill states + compact |
  | 13 | Tier pills | 4 (present/active/absent/default) |
  | 14 | Buttons | 7 (icon-only, filter, sort, sort-active, filter badge, inline link, `ds-link`) |
  | 15 | Inputs & form | 3 (search input, checkbox checked, checkbox unchecked) |
  | 16 | Sidebar nav | 6 (group label, sub-label, default/active/with-pulse/faded items) |
  | 17 | Cards | 4 (lifecycle, kanban, related-row, empty) |
  | 18 | Tables | 1 (library table with a selected row) |
  | 19 | Markdown | 8 element kinds (headings, bold, italic, inline code, wiki-link, ordered list, unordered list, table) |
  | 20 | Code blocks | 2 (TypeScript, Bash) |
  | 21 | Frontmatter | presence of key/value rows with links |
  | 22 | Empty & banners | 2 (inline empty, warn banner) |
  | 23 | Toasts | 3 (ok / warn / err) |
  | 24 | Topbar | 5 parts (brand+sub-label, breadcrumbs, origin/status pill, SSE indicator, theme button) |
- No keybind registry exists; bindings are hand-rolled
  `document.addEventListener("keydown", …)`. Closest pattern: the global
  `/` search binding at `src/components/RootLayout/RootLayout.tsx:52-63`,
  with the `isPlainSlashKey` (`:20-28`) and `isEditableTarget` (`:30-39`)
  guards. The new chord handler fits as a second `useEffect` there,
  navigating via the exported `router` (`src/router.ts:218`)
  `router.navigate(...)`. Existing test home: the `"global / keybind"`
  describe block at `src/components/RootLayout/RootLayout.test.tsx:73`.
- The sidebar (`src/components/Sidebar/Sidebar.tsx`) has no foot/footer/
  version element today; the triple-click host is a new minimal element
  added to the sidebar nav.
- The five showcase routes are defined in `src/router.ts:152-216`
  (TanStack Router, imperative `createRoute`), reached by direct URL
  only; each is a Playwright VR fixture (specs named in the route
  comments). `src/router.ts:161-162` designates 0083 as the consolidation
  home and warns to migrate the VR spec + baselines, not just delete.
- **Showcase → section mapping** (the oracle for AC #1 and the
  VR-migration AC, confirmed against the `src/router.ts` route comments):
  - `/glyph-showcase` (12 doc-type Glyphs × 3 sizes) → **Doc-type glyphs**
  - `/big-glyph-showcase` (13 doc-type heroes; spec
    `big-glyph-showcase.spec.ts`) → **Empty-state glyphs**
  - `/chip-showcase` (6 variants × 2 sizes) → **Chips**
  - `/code-syntax-showcase` (spec `code-block-resolved-colours.spec.ts`)
    → **Code blocks**
  - `/kanban-card-showcase` (specs `kanban-card-showcase.spec.ts`,
    `kanban-card-resolved-styles.spec.ts`) → **Cards**
  - Note: `/glyph-showcase` exercises the doc-type `Glyph`, not the
    stroke `Icon` set, so the Icons section is net VR coverage rather than
    a migration target.
- 0037 established the `/glyph-showcase` pattern; this story generalises
  it and retires all five showcases.

## Drafting Notes

- Verified the 24-section list against the prototype `view-dev.jsx`
  (`DEV_SECTIONS`, lines 15-40) on 2026-06-12 — an exact match, nothing
  missing or extra at the section level.
- Expanded scope to capture full content fidelity (the per-section
  variant inventory), `#dev/<section>` deep-linking with scroll-driven
  hash sync, light/dark theme coverage, and the dev-page chrome (TOC
  aside + "exit to app", "DEV" marquee, footer) after checking the
  prototype against the prior draft, which under-specified these. The
  marquee and footer are page chrome rather than design-system
  primitives but were kept for faithful reproduction.
- Keychord: per author direction, the trigger must be a modifier-based
  chord (inert during text entry); `Cmd/Ctrl+Shift+D` was dropped due to
  the confirmed Chromium "bookmark all tabs" reservation. The exact chord
  is deferred to implementation behind a cross-browser verification AC,
  with `Cmd/Ctrl+Shift+L` suggested. The prototype's marquee/footer hint
  text (`⌘⇧D`) must be updated to the chosen chord when ported.
- Scope was expanded from the two originally-named showcases to all five,
  with VR spec/baseline migration, on the strength of
  `src/router.ts:161-162` naming this work item as the consolidation
  home. Combined with the fidelity, theme, deep-link, and chrome scope
  above, this is materially larger than the original draft implied. On
  review (2026-06-12) it was kept as a single story rather than split or
  promoted to an epic: the strands are indivisible in practice — the
  VR-spec/baseline migration can only assert against `DevDesignSystem`
  sections once those sections exist, and per-section content fidelity
  needs the primitives present before any section closes, so a partial
  split would leave the consolidation half-done with no working
  consolidation home. `kind` stays `story` and `priority` stays `low` as
  deliberate calls (see Open Questions).
- Retired paths 404 rather than redirect, since nothing in-app links to
  the showcases.
- `kind` confirmed as `story` on review (2026-06-12) rather than `task`:
  although it is developer tooling with no direct end-user benefit, it is
  treated as a story serving the visualiser's maintainers. Recorded in
  Open Questions as a settled decision.
- Mounting + hash form (refined 2026-06-13 during planning): `DevDesignSystem`
  is an uncrumbed `/dev` TanStack route reached through a `hashchange` bridge.
  The canonical URL is `/dev#<section>` (e.g. `/dev#colors`); `#dev` and
  `#dev/<section>` are kept as activation aliases the bridge normalises to that
  form. AC3/AC4/AC5, the "exit" clause (AC7), and the Activation /
  Scroll-spy requirements were re-worded from the earlier hash-only
  (`#dev/<section>`) phrasing to this route-plus-bare-section-hash model —
  chosen for URL cleanliness over the redundant `/dev#dev/<section>`. The
  prototype's own `history.replaceState(…, "#dev/<id>")` (Technical Notes)
  remains the pattern to port from, now adapted to the bare `#<id>` write.
  Plan: `meta/plans/2026-06-12-0083-dev-design-system-reference-page.md`.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Authoritative content spec (prototype dev page):
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/src/view-dev.jsx`
- Supporting prototype sources (same `prototype-full/` directory):
  `assets/tokens.css`, `src/app.css`, `src/ui.jsx`, `src/data.jsx`,
  `src/highlight.jsx`, `src/big-glyphs.jsx`
- Related: 0033, 0035, 0037, 0038, 0039, 0040, 0041
