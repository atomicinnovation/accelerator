---
work_item_id: "0041"
title: "Library Page Wrapper, Overview Hub, and List Views"
date: "2026-05-06T14:04:04+00:00"
author: Toby Clemson
type: story
status: done
priority: medium
parent: ""
tags: [design, frontend, library]
---

# 0041: Library Page Wrapper, Overview Hub, and List Views

**Type**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a user of the meta directory visualiser, I want a structured library experience — an overview hub at `/library`, consistent page chrome on every library screen, a pill-driven sort + filter UI, and clear empty states — so that I can navigate between doc types, scan recent activity, and refine list views without relearning controls per screen.

Introduce a generic `Page` wrapper (eyebrow, H1, subtitle/count, right-aligned actions, standard max-width and spacing). Apply it across the library: replace the `/library` redirect with a phase-grouped overview hub of doc-type cards; restructure each doc-type list view with the same wrapper, a new column layout, a sort-pill menu, and a doc-type-aware filter-pill menu; render a dedicated empty state when a doc type has no documents.

## Context

The current app's `LibraryTypeView` renders a sortable table where sort is driven by clicking column headers; there is no filter UI, no Page wrapper, and `/library` redirects straight to `/library/decisions`. The prototype supersedes all three concerns.

**Updated design (canonical)**: the `-updated-light` / `-updated-dark` / `-wide` overview screenshots show five lifecycle phases (DEFINE / DISCOVER / BUILD / SHIP / REMEMBER) with twelve doc-type cards in the current snapshot. The earlier `library-view.png` shows a superseded four-phase composition kept only for historical reference.

**Doc-type set evolves over time**. The phase groupings and doc-type list must therefore be **server-driven** rather than hard-coded in the frontend — the server returns the phase structure, each doc type's identity (id, label, Glyph, route), count, and "latest" preview, and the frontend renders whatever it receives.

Reference screenshots:
- Overview hub: `library-view-updated-light.png`, `library-view-updated-dark.png`, `library-view-wide.png` (canonical); `library-view.png` (superseded composition).
- List view + page chrome: `library-list-view.png`.
- Sort menu: `library-list-sort-menu.png`.
- Filter menus: `library-list-filter-menu-1.png` (ADR-style, Status + Cluster slug), `library-list-filter-menu-2.png` (Work items, Status + Project + Cluster slug with option search).
- Empty state: `library-list-no-documents.png`.

## Requirements

### Generic Page wrapper

- Implement a `Page` component used across the library (and available to other screens such as lifecycle / kanban): provides an eyebrow row (small uppercase label with optional Glyph), `<h1>`, a single subtitle slot (a text slot whose content varies by caller — the overview hub passes descriptive prose, the list view passes `{N} documents`), right-aligned actions slot, and a content slot.
- Owns standard page-level concerns: max content width (via a new `--ac-content-max-width` design token introduced as part of this story — see Coupled migrations), horizontal padding (`--sp-6`), vertical spacing/margins (`--sp-5` between header and content), and the separator (horizontal rule) between header and content.
- Actions slot is generic — callers (overview, list view) decide which controls to render.
- Replaces the bare `<h1>` currently in `LibraryTypeView` and the `PageSubtitle` component (which is deleted in this story; its sole consumer `KanbanBoard` migrates to `Page` here — see Coupled migrations).
- `Page` owns horizontal padding so that pages are visually self-contained; `RootLayout.main`'s existing `padding: var(--sp-5) var(--sp-6)` is removed in this story, and every route that today relied on that padding is migrated to `Page` (see Coupled migrations).

### Library overview hub at `/library`

- Replace the `/library` → `/library/decisions` redirect with a new overview screen using the `Page` wrapper.
- Eyebrow: `LIBRARY` (with library Glyph). H1: `All artifacts in meta/` with `meta/` rendered as inline mono/code. Subtitle: `Browse every doc type produced by the research → plan → implement workflow. Click a type to drill in, or jump into a view.`
- Group doc-type cards by lifecycle phase. Phase set and grouping are **driven by a server-provided structure**, not hard-coded; the frontend renders whatever phases and members the server returns. Current snapshot: DEFINE / DISCOVER / BUILD / SHIP / REMEMBER.
- Each card layout (exact rendering normative to `library-view-updated-light.png` / `-updated-dark.png`): doc-type Glyph (coloured per type) + doc-type label on a single row at the top-left; doc count at the top-right; a `latest · {title}` preview line for the most recently modified document of that type below.
- Cards are drillable: clicking navigates to that doc type's list view.
- Card grid is responsive: 1 column at narrow widths, 2 columns at medium, 3 columns at wide (per `library-view-wide.png`).
- Light and dark themes both supported; rendering matches `library-view-updated-light.png` (light) and `library-view-updated-dark.png` (dark) for surface backgrounds, card surfaces, eyebrow/H1/subtitle text colours, and Glyph colours.
- Zero-document doc types still render a card; the `latest · {title}` line is replaced with a muted `no documents yet` line.

### List view chrome and table

- Apply the `Page` wrapper to each doc-type list view. Eyebrow shows the doc-type Glyph + uppercase doc-type name; H1 shows the doc-type label; subtitle shows `{N} documents`.
- Actions slot renders a sort pill and a filter pill, right-aligned.
- Table columns (per `library-list-view.png`): `ID / DATE`, `TITLE`, `STATUS`, `SLUG`, `MODIFIED`. ID rendered as a monospace pill in the form `{DOC_TYPE_PREFIX}-{4-digit-zero-padded-id}` (e.g. `PROJ-0001`); status as a coloured chip using the canonical mapping from `statusToChipVariant` (`api/status-variant.ts`); slug as monospace text; modified as a relative timestamp produced by `formatMtime` (`<n>s ago` / `<n>m ago` / `<n>h ago` / `<n>d ago` / `<n>w ago`, falling back to a locale date for values older than one week).
- Column headers are **not** sort entry points — sort is driven exclusively from the sort pill. The existing column-header click-sort behaviour is removed.

### Sort pill and menu

- Pill displays the active sort option (e.g. `↕ Recently modified ⌄`) and opens a menu on click. The `↕` and `⌄` glyph affordances are illustrative; final iconography defers to the Glyph system delivered by 0037.
- Menu has a `SORT BY` header and five options (current snapshot): `Recently modified`, `Oldest first`, `Title (A → Z)`, `Title (Z → A)`, `ID (ascending)`.
- Selected option is visually highlighted and shows a checkmark; default selection is `Recently modified`.
- "Modified" semantics: last modification time of the document file, sourced from the indexer's `mtime_ms` field. This is the same value rendered in the table's `MODIFIED` column and feeds the overview hub's "latest" preview.
- When two documents share the same mtime, secondary sort is by document ID ascending so ordering is deterministic. "Document ID" means `IndexEntry.workItemId` when present (work-items, plans, ADR-style types that carry one), else `IndexEntry.relPath` as a deterministic fallback — both ascending. The fallback ensures doc types without a `workItemId` (e.g. `notes`, `research`) still order deterministically on ties.

### Filter pill and menu

- Pill labelled `▽ Filter`; opens a menu on click.
- Menu structure: `FILTER` header, followed by per-facet sections. Each facet section shows the facet name (uppercase) and a list of options as checkboxes with an option-count to the right.
- Status options render as inline status chips matching the table's status-chip styling.
- **Available facets are doc-type-aware**, driven by the server's doc-type metadata. Examples from the screenshots:
  - ADR-style doc types (Architecture Decision Record and its kin — `decisions`, `principles`, and other rationale-record doc types that share frontmatter shape): `STATUS`, `CLUSTER SLUG`.
  - Work items: `STATUS`, `PROJECT`, `CLUSTER SLUG`.
- When a facet has more than 8 options, the section renders an inline option-search input (e.g. `Filter slugs…`) above a vertically scrollable checkbox list (max-height equal to 8 option rows; `overflow-y: auto`).
- Multiple options within a facet combine as OR; multiple facets combine as AND (standard faceted-search semantics).
- Option counts use standard faceted-search scoping: each option's count reflects the document set filtered by all **other** facets' current selections, ignoring this facet's own selections. This ensures toggling an option within a facet never collapses that facet's counts to zero; counts visibly update as you change other facets.

### Empty / zero states

Notation: `{type}` in path strings below is the doc-type slug (e.g. `notes`, `decisions`); `{type-plural}` is the human-readable plural form rendered in lowercase (e.g. `notes`, `decisions`, `pr descriptions`).

- When a doc type has zero documents, the list view still renders the `Page` wrapper with the same eyebrow, H1, and subtitle (`0 documents`).
- Sort and filter pills are hidden when the doc type is empty (no documents to sort or filter).
- Body shows a single empty-state card: doc-type Glyph (large), the doc-type path heading (e.g. `meta/notes/`), a `no {type-plural} yet.` headline, a doc-type-specific description sentence (hard-coded in the frontend per doc type), and an indexer-aware footer (`New files added to meta/{type}/ are picked up live — this view will populate as soon as the indexer sees them.`).
- On the overview hub, a doc type with zero documents still renders a card; the `latest · {title}` line is replaced with a muted `no documents yet` line.
- **Filter-applied empty state** (list has documents but the active filter excludes all of them): render a smaller panel inside the table area showing the headline `no results match your filter`, a sentence explaining that current filters return no rows, and a `Clear filters` button that resets all facet selections. The page chrome (eyebrow, H1, subtitle, sort pill, filter pill) remains visible — this is a distinct state from doc-type-empty.

### Server-driven library structure endpoint

The server-side contract is in-scope for this story. Promote `describe_types` or add a sibling handler under `server/src/api/` to expose the shape below.

- The endpoint returns `{ phases: [{ id, label, doc_types: [{ id, label, glyph_id, route, count, latest: { title, slug, modified_at } | null, filter_facets: [{ id, label, kind, options: [{ id, label, count }] }], empty_description: string | null }] }] }`.
- Phase order in the response is the canonical display order.
- `latest` is `null` for doc types with zero documents; otherwise it reflects the maximum `mtime_ms` across that doc type's indexed entries.
- `filter_facets` is doc-type-specific; for the current snapshot ADR-style types expose `[status, cluster_slug]` and work items expose `[status, project, cluster_slug]`. The frontend renders whatever the server returns.
- **Facet data sources** (server-side derivation, not new frontmatter fields):
  - `status` — read from `IndexEntry.frontmatter["status"]` (raw value passed through; option labels canonicalised client-side via `statusToChipVariant.normalise` when rendering as chips, but the server returns the underlying string so option counts remain accurate).
  - `cluster_slug` — derived from `IndexEntry.slug` (the kebab tail computed by `slug::derive`; `server/src/clusters.rs` already buckets entries by this string). No new frontmatter field is introduced.
  - `project` — derived per work-item from the prefix of `IndexEntry.workItemId` (e.g. `"PROJ-0042"` → `"PROJ"`); falls back to `config.work_item.default_project_code` when the prefix is absent. No new frontmatter field is introduced.
- `empty_description` may be `null`, in which case the frontend uses its own per-doc-type fallback copy.
- The existing client-side `PHASE_DOC_TYPES` constant (`frontend/src/api/types.ts:228-254`) is retired in this story; both the library views and `Sidebar.tsx` consume the new server shape.

### Coupled migrations

- **Sidebar.tsx** is migrated to the new server-driven phase structure in this story (`Sidebar.tsx:38-78` currently consumes the client-side `PHASE_DOC_TYPES`).
- **`PageSubtitle` is deleted outright** in this story. Its sole consumer (`KanbanBoard.tsx`, three call sites at lines 143, 152, 183-185) migrates to `Page` here, so the thin-shim deferral previously contemplated is no longer needed.
- **All five `<main>`-padded routes migrate to `Page` in this story** to accommodate the new layout split (Page owns horizontal padding; RootLayout drops its `padding: var(--sp-5) var(--sp-6)` rule): `KanbanBoard`, `LifecycleIndex`, `LibraryDocView`, `LibraryTemplatesView`, `LibraryTemplatesIndex`, plus the new `LibraryOverviewHub` and the refactored `LibraryTypeView`. Each route's existing `max-width` literal is replaced with the new `--ac-content-max-width` token (or the route opts out via a `Page` prop where its current width differs from the canonical 1100px — to be settled per route during implementation).
- **`--ac-content-max-width` design token** is introduced in this story. Added to `frontend/src/styles/tokens.ts` (LAYOUT_TOKENS group, theme-invariant) and all three light/dark mirror blocks in `frontend/src/styles/global.css`. `frontend/src/styles/migration.test.ts`'s per-route hardcoded-max-width allowlist is updated as routes migrate to consume the token.
- **`RootLayout.main`** drops its `padding: var(--sp-5) var(--sp-6)` rule. After this story, `Page` is the canonical owner of horizontal padding within the main scroll area; routes that don't wrap in `Page` will not have horizontal padding.
- **`PR descriptions` rename**. The human-readable label (rendered in UI, page titles, eyebrow text) becomes the literal string `PR descriptions` (with a space) everywhere it appears: server `DocTypeKey::Prs.label()`, frontend `DOC_TYPE_LABELS['…']`, pipeline-step labels, and any test fixtures. The wire-format identifier becomes `pr-descriptions` (kebab-case, matching the existing kebab convention used by every other doc-type wire token): the Rust enum variant renames `DocTypeKey::Prs` → `DocTypeKey::PrDescriptions` with `#[serde(rename = "pr-descriptions")]`, and the frontend `DocTypeKey` union member renames `'prs'` → `'pr-descriptions'`. **The on-disk directory stays at `meta/prs/`** to minimise scope — `config_path_key()` for the variant continues returning `Some("prs")`, so `cfg.doc_paths["prs"]` still resolves the directory. This deliberate asymmetry (wire token `pr-descriptions`, config key `prs`, dir name `prs`) is acceptable; a future tidy-up can rename the directory if desired.
- **Router test updates**: `router.test.tsx` (three test cases at lines 40-44, 46-49, 71-77) is updated to reflect the removal of the `/library` → `/library/decisions` redirect — terminal expectation changes from `/library/decisions` to `/library`. `router-with-crumb.test.ts` only tests the pure `resolveCrumb` helper and is unaffected.

## Acceptance Criteria

### Overview hub

- [ ] Given the user navigates to `/library`, when the page renders, then the new overview hub appears (the redirect to `/library/decisions` is removed).
- [ ] The overview hub renders the eyebrow `LIBRARY`, H1 `All artifacts in meta/` (with `meta/` styled as inline mono), and the subtitle copy specified above.
- [ ] Given the server response declares phases `[{id: "define", doc_types: [t1]}, {id: "discover", doc_types: [t2, t3]}]`, when the hub renders, then two phase groupings appear in the order `define`, `discover`, with one card in the first and two cards in the second. Changing the server response between two test runs produces correspondingly different rendered output (no hard-coded phase list).
- [ ] Each doc-type card shows the doc-type Glyph (coloured per type) + label on a single row at the top-left, the doc count at the top-right, and a `latest · {title}` preview line below.
- [ ] Clicking a doc-type card navigates to that doc type's list view (`/library/{doc-type-route}`).
- [ ] The card grid renders 1 column below 640px viewport width, 2 columns between 640px and 1024px, and 3 columns at 1024px and above. (Exact breakpoints subject to design-token alignment with 0033 — confirm during implementation.)
- [ ] In both light and dark themes, the overview hub renders using only `--ac-*` tokens from 0033 (plus the new `--ac-content-max-width` introduced by this story) — no hard-coded colour or shadow values appear in the hub's CSS. Specifically: card surfaces use `var(--ac-bg-card)`; the page background is the body-level `var(--ac-bg)` (the canonical app-background token from 0033 — no separate `--ac-bg-app` exists); eyebrow and subtitle text use `var(--ac-fg-muted)`; H1 uses `var(--ac-fg-strong)`; card borders and the header `<hr>` use `var(--ac-stroke)`; doc-type Glyph colours are provided by the `Glyph` component's per-doc-type colour mapping (0037).
- [ ] Given a doc type with zero documents, when the overview hub renders, then a card for that type still appears with the muted text `no documents yet` in place of the `latest · {title}` preview line.

### Page wrapper

- [ ] The `Page` wrapper provides eyebrow row, H1, subtitle/count, right-aligned actions slot, and content slot, separated by a horizontal rule.
- [ ] The `Page` wrapper applies max content width = `var(--ac-content-max-width)` (introduced by this story in `tokens.ts` and `global.css`), horizontal padding = `var(--sp-6)`, and vertical spacing between header and content = `var(--sp-5)`. Every consuming screen renders with identical computed values for these three CSS properties.
- [ ] The `Page` wrapper is consumed by the overview hub, each doc-type list view, and all five pre-existing `<main>`-padded routes (`KanbanBoard`, `LifecycleIndex`, `LibraryDocView`, `LibraryTemplatesView`, `LibraryTemplatesIndex`).
- [ ] `RootLayout.main`'s `padding: var(--sp-5) var(--sp-6)` rule is removed. Visual output is unchanged for every existing route because each migrates to `Page` in this story.
- [ ] `PageSubtitle` (`frontend/src/components/PageSubtitle/PageSubtitle.tsx`) and its CSS module are deleted. No consumer remains after `KanbanBoard` migrates to `Page`.

### List view + sort

- [ ] Given the user navigates to any doc-type list view, when the page renders, then the page header shows the doc-type Glyph + name as eyebrow, the doc-type label as H1, and `{N} documents` as subtitle.
- [ ] The list view renders the columns `ID / DATE`, `TITLE`, `STATUS`, `SLUG`, `MODIFIED`. ID is rendered as a monospace pill in the form `{DOC_TYPE_PREFIX}-{4-digit-zero-padded-id}` (e.g. `PROJ-0001`); status as a coloured chip using `statusToChipVariant` (`api/status-variant.ts`); slug as monospace; modified as a relative timestamp produced by `formatMtime` (`<n>s ago` / `<n>m ago` / `<n>h ago` / `<n>d ago` / `<n>w ago`, falling back to a locale date for values older than one week).
- [ ] Clicking a column header does **not** change the sort order (column-header click-sort is removed).
- [ ] Given the sort pill is clicked, when the menu opens, then the options `Recently modified`, `Oldest first`, `Title (A → Z)`, `Title (Z → A)`, `ID (ascending)` are shown under a `SORT BY` header, with `Recently modified` selected by default and the selected option visually highlighted with a checkmark.
- [ ] Selecting a sort option re-orders the list accordingly and updates the pill label.
- [ ] When two documents share the same `mtime_ms`, the rendered order between them is deterministic — secondary sort is by `IndexEntry.workItemId` ascending when both entries have one, else by `IndexEntry.relPath` ascending. The chosen comparator is total (no nulls reach the sort step).

### Filter

- [ ] Given the filter pill is clicked, when the menu opens, then the menu shows a `FILTER` header followed by per-facet sections appropriate to the current doc type.
- [ ] Given the server response declares `filter_facets: [{id: "status", ...}, {id: "cluster_slug", ...}]` for the current doc type, when the filter menu opens, then exactly those two facet sections appear in that order under the `FILTER` header. Given a different doc type whose response declares three facets, the menu reflects that three-facet set.
- [ ] Status options within a filter facet render as inline coloured chips matching the table styling (using `statusToChipVariant` as the canonical mapping).
- [ ] When a facet has more than 8 options, the facet section renders an inline option-search input (placeholder `Filter {facet-noun}…`) above a vertically scrollable checkbox list with max-height equal to 8 option rows and `overflow-y: auto`.
- [ ] Multiple selected options within a single facet combine with OR; selections across multiple facets combine with AND. Worked example: given 10 documents — 6 with `status=active`, 4 with `status=draft`, 7 with `project=alpha` — when `status=active` and `project=alpha` are both selected, then the list shows only documents satisfying `(status=active) AND (project=alpha)`.
- [ ] Each filter option shows a count to its right. Counts reflect the document set after filtering by all **other** facets' current selections, ignoring the facet's own selections (standard faceted-search scoping). Toggling an option within a facet must not collapse that facet's counts to zero; toggling an option in a different facet visibly updates this facet's option counts.
- [ ] Given the active filter combination returns zero rows but the doc type has at least one document, when the list view renders, then the page chrome (eyebrow, H1, subtitle, sort pill, filter pill) remains visible and the table area shows a panel with the headline `no results match your filter`, an explanatory sentence, and a `Clear filters` button that resets all facet selections.
- [ ] Given the filter-applied empty state is showing, when the user clicks the `Clear filters` button, then all facet checkboxes return to unchecked, the empty-state panel is removed, the filter pill returns to its unselected label (`▽ Filter`), and the full unfiltered list re-renders.

### Empty / zero states (doc-type-empty)

- [ ] Given a doc type has zero documents, when the user navigates to its list view, then the `Page` header still renders (eyebrow, H1, `0 documents` subtitle) and the sort and filter pills are hidden.
- [ ] The empty-state card shows the doc-type Glyph, the doc-type path heading (`meta/{type}/` where `{type}` is the doc-type slug, e.g. `notes`), a `no {type-plural} yet.` headline, a doc-type-specific description sentence (hard-coded in the frontend per doc type), and the footer string `New files added to meta/{type}/ are picked up live — this view will populate as soon as the indexer sees them.`

### Server-driven library structure endpoint

- [ ] The server exposes an endpoint returning the structure shape described in Requirements (`{ phases: [{ id, label, doc_types: [...] }] }`), populated by extending `describe_types` or via a new handler under `server/src/api/`.
- [ ] Each doc type in the response includes its `count` and (where the doc type is non-empty) a `latest` field populated by the maximum `mtime_ms` across that doc type's indexed entries; `latest` is `null` when `count == 0`.
- [ ] Each doc type in the response includes its `filter_facets` array; for the current snapshot ADR-style doc types expose `[status, cluster_slug]` and work-items expose `[status, project, cluster_slug]`. Facet options are server-derived: `status` from `frontmatter["status"]`, `cluster_slug` from `IndexEntry.slug`, `project` from the prefix of `IndexEntry.workItemId` (or `config.work_item.default_project_code` fallback). No new `IndexEntry` field or frontmatter convention is introduced.
- [ ] The client-side `PHASE_DOC_TYPES` constant in `frontend/src/api/types.ts` is removed. Both the new library overview hub and `Sidebar.tsx` consume the server response.
- [ ] `DocTypeKey::Prs` is renamed to `DocTypeKey::PrDescriptions` with `#[serde(rename = "pr-descriptions")]`; the frontend `DocTypeKey` union member renames `'prs'` to `'pr-descriptions'`; UI labels become `PR descriptions`. The on-disk directory `meta/prs/` is unchanged and `config_path_key()` for the variant continues returning `Some("prs")`.
- [ ] `router.test.tsx` is updated to reflect the removal of the `/library` → `/library/decisions` redirect: three test cases (`'redirects / to /library/decisions (via /library)'`, `'redirects bare /library to /library/decisions'`, `'redirects /library/bogus to /library/decisions when the type is unknown'`) change their terminal-path expectation from `/library/decisions` to `/library`. `router-with-crumb.test.ts` is unaffected.

## Open Questions

- Exact breakpoint pixel values (640px / 1024px proposed) for the overview hub card grid are subject to confirmation against the design-token spacing scale delivered by 0033 during implementation.
- Server endpoint path (`/api/library/structure` vs extending `/api/types`) is an implementation detail to be settled at code-review time; the observable behaviour is what's specified above.
- Canonical value for the new `--ac-content-max-width` token. Current per-route literals are 1100px (`LibraryDocView`), 900px (`LibraryTypeView`, `LibraryTemplatesView`, `LifecycleIndex`), 800px (`LifecycleClusterView`), 720px (`MarkdownRenderer`), 600px (`LibraryTemplatesIndex`). Likely answer: 1100px is the canonical content max-width; routes that today use narrower widths either stay at their literal (passed to `Page` as a prop override) or align to 1100. To be settled per route during implementation against the prototype screenshots.

## Dependencies

- Blocked by: 0033 (token system), 0037 (Glyph), 0038 (Chip — used in subtitle/count and as status/filter chips).
- Blocks: 0042 (Templates view also uses the Page wrapper).
- Internal ordering within this story: the `Page` wrapper, the new `--ac-content-max-width` token, and the popover/floating-menu primitive (all net-new in this story) must land before the overview hub, list-view chrome, sort pill, filter pill, empty-state work, and the migrations of `KanbanBoard` / `LifecycleIndex` / `LibraryDocView` / `LibraryTemplatesView` / `LibraryTemplatesIndex` begin, since each of those consumes one or more of these primitives.

## Assumptions

- The Page wrapper is a **generic** component, not library-specific. Reused beyond the library (e.g. lifecycle index, kanban) without modification.
- The doc-type set and phase groupings are **server-driven** — the frontend renders whatever structure the server returns rather than encoding it locally.
- "Most recently modified" uses filesystem mtime sourced from the indexer's `mtime_ms` field; no Git/JJ-time alternative in this story.
- Per-doc-type empty-state description sentences are hard-coded in the frontend per doc type. The server's `empty_description` field is reserved for future use but ignored by the frontend in this story; no authoring UI for empty-state copy is introduced.
- Filter combination semantics are standard faceted-search: OR within a facet, AND across facets; option counts use standard faceted-search scoping (post-other-facet-filter, pre-own-facet-filter).
- The popover/floating-menu primitive used by sort and filter pills is **hand-rolled** (not introduced via `@floating-ui/react` or similar). It matches the codebase's existing hand-rolled small-primitives house style (Sidebar, Topbar, SortButton, PipelineDots) and ships with a co-located `useDismiss` hook handling click-outside (document `mousedown`) and Escape-to-close. Keyboard navigation (arrow keys, Enter, focus management on open/close) is specified in this story as a requirement of the primitive. Positioning is via `getBoundingClientRect` + absolute placement relative to the triggering pill.
- The `Page` wrapper owns horizontal padding; `RootLayout.main`'s `padding: var(--sp-5) var(--sp-6)` rule is removed in this story so every existing `<main>`-padded route is migrated to `Page` to preserve visual output.

## Technical Notes

**Size**: L/XL — Touches frontend and server. Net-new components (`Page` wrapper, overview hub, sort pill+menu, filter pill+menu, empty-state card, popover primitive with `useDismiss` hook, ID formatter) plus a substantive `LibraryTypeView.tsx` refactor (column-header click-sort removed at `:50-51,71-74,143-166`; columns restructured at `:106-132`; empty state at `:135` replaced) and **five `<main>`-padded routes migrated to `Page`** (`KanbanBoard`, `LifecycleIndex`, `LibraryDocView`, `LibraryTemplatesView`, `LibraryTemplatesIndex`), with `RootLayout.main` losing its padding rule. Server-side work extends `describe_types` (`server/src/docs.rs:115-132`) or adds a new `/api/library/structure` handler, derives per-doc-type `count` (already in `Indexer.counts_by_type` at `:326-332`), `latest` (net-new `latest_by_type` style query against `Indexer.entries`), and per-facet option-counts at request time; retires client-side `PHASE_DOC_TYPES` (`frontend/src/api/types.ts:228-254`) with a parallel `Sidebar.tsx:38-78` migration. The `prs` → `pr-descriptions` wire-token rename touches `docs.rs:16,34,52,70,150`, `slug.rs:27,142`, `clusters.rs:76,123`, all three light/dark token mirror blocks in `global.css:107,206,252`, `tokens.ts:38,79`, `Glyph.tsx:55`, and the `PrsIcon.tsx` filename. New `--ac-content-max-width` token added to `tokens.ts` (LAYOUT_TOKENS) and `global.css` (all three mirror blocks) with `migration.test.ts` allowlist updates. Router change at `router.ts:60,71-77` with test updates in `router.test.tsx:40-44,46-49,71-77`. Borderline XL given the five-route migration scope.

- `Page` is a generic wrapper. Consider whether the actions slot accepts arbitrary children or a structured props API (e.g. `actions=[{kind: 'sort', ...}, {kind: 'filter', ...}]`); the former is simpler, the latter helps enforce consistent pill styling.
- The server contract for the overview hub plausibly looks like: `GET /api/library/structure` → `{ phases: [{id, label, doc_types: [{id, label, glyph, route, count, latest: {title, modified_at}}]}] }`. Sketch only — actual shape is part of the server-side work.
- The filter facet metadata likely sits on the doc-type record: e.g. `{ id, label, ..., filter_facets: [{id: 'status', label: 'STATUS', kind: 'enum-chip', options: [...]}, {id: 'cluster_slug', label: 'CLUSTER SLUG', kind: 'enum-with-search', options: [...]}] }`.
- Removing column-header click-sort affects existing behaviour in `LibraryTypeView` — ensure no callers rely on URL state set by that path.
- Current `LibraryTypeView.tsx:50-51,71-74,143-166` holds the sort state (`sortKey`/`sortDir`), the `toggleSort` callback, and the `SortHeader` component; the refresh deletes all three in favour of the sort pill and rebuilds the table without `<button>` headers. **Status cell at `:122-126` does NOT currently use `Chip` or `statusToChipVariant`** — it renders a bare `<span className={styles.badge}>` where `styles.badge` is not defined in `LibraryTypeView.module.css` (silent no-op). Wiring `Chip` + `statusToChipVariant` here is new code, not reuse. Empty state at `:135` (`<p>No documents found.</p>` — work item previously said line 142, actual line is 135) is replaced by the new empty-state card. There is no current `ID` column; `IndexEntry.workItemId: string | null` is unused — the `{DOC_TYPE_PREFIX}-{0000}` formatter is net-new code with no precedent in the codebase. There is no URL state for sort (no `validateSearch` on `libraryTypeRoute`), so removing column-header click-sort breaks no URL contract.
- The `/library` redirect lives at `router.ts:71-77` (`libraryIndexRoute` with `beforeLoad: () => redirect({ to: '/library/$type', params: { type: 'decisions' } })`). Replace with a `component:` referencing the new hub. The invalid-doc-type fallback at `router.ts:101-104` already targets `/library` and becomes consistent with the hub automatically. **Note**: `indexRoute` at `router.ts:60` also redirects `/` → `/library`, which chains through the removed `libraryIndexRoute`; the test at `router.test.tsx:40-44` that asserts `/` → `/library/decisions` must update its terminal expectation. **Only `router.test.tsx` is affected** — `router-with-crumb.test.ts` tests only the pure `resolveCrumb` helper and has no redirect dependency.
- Doc-type metadata currently lives on **both** sides: server `describe_types` in `server/src/docs.rs:115-132` returns the flat `Vec<DocType>` served by `server/src/api/types.rs:14-21` at `GET /api/types`; the frontend mirrors it via `DOC_TYPE_KEYS`/`DOC_TYPE_LABELS` and hard-codes the phase grouping in `frontend/src/api/types.ts:228-254` (`PHASE_DOC_TYPES`). Promoting the phase structure server-side means extending `describe_types` (or adding a new `/api/library/structure` handler under `server/src/api/`) and retiring the client-side `PHASE_DOC_TYPES` constant — `Sidebar.tsx:38-78` is the other consumer that needs to migrate to the server shape.
- `mtime_ms` is already populated per `IndexEntry` in `server/src/indexer.rs` (line 30) and rendered via `formatMtime` (`frontend/src/api/format.ts:19-27`). The "latest per doc-type" preview for the overview hub is net-new derivation — likely computed in the same `describe_types` pass by scanning per-type entries for the max mtime.
- `PageSubtitle` (`components/PageSubtitle/PageSubtitle.tsx`) is replaced by `Page` (new at `components/Page/`), which subsumes its `<h1>` + subtitle role and adds the eyebrow row, actions slot, and standard chrome. `PageSubtitle` is **deleted outright** in this story: its sole consumer `KanbanBoard.tsx` (three call sites at `:143`, `:152`, `:183-185`) migrates to `Page` here, so no shim is needed.
- No menu/popover/checkbox/click-outside/Escape-handling primitive exists anywhere in `frontend/src/` — every one of the floating-menu building blocks is net-new. Closest existing precedent is `LifecycleIndex.tsx:72-87` (file is 143 lines total; the work item previously cited `:75-142` which was off), which renders inline `<input type="search">` and `SortButton` toggles inline in a toolbar — no floating menu. `package.json` has no floating-ui / Radix / Headless UI dependency; consistent with the hand-rolled house style.
- All three blocker work items are merged: 0033 (`styles/tokens.ts`, `styles/global.css`), 0037 (`components/Glyph/Glyph.tsx` plus 12 icons in `Glyph/icons/`), 0038 (`components/Chip/Chip.tsx` with variants `neutral|indigo|green|amber|red|violet`).

## Drafting Notes

- Title broadened to "Library Page Wrapper, Overview Hub, and List Views" because list view chrome, sort menu, filter menu, empty states, and server endpoint were added to scope across enrichment + review passes — the original title under-described the work.
- This is a sizeable story. Considered splitting (Page wrapper / overview / list-view chrome / sort / filter / empty states / server endpoint) and confirmed during review to keep as one because: (a) every UI piece consumes the same `Page` wrapper and the same net-new popover primitive — splitting would create a long chain of cross-blocking PRs against unstable shared APIs; (b) the server-shape change touches one endpoint that all sub-features simultaneously depend on, so it cannot land in isolation without breaking the consumers; (c) the seams are clean (each Requirements subsection is independently testable) but not independent in time. Internal ordering (Page wrapper + popover primitive → consumers) is captured in Dependencies.
- The canonical design is the `-updated-*` and `library-list-*` screenshots; the original `library-view.png` four-phase composition is treated as superseded.
- All six original Open Questions were resolved through the review pass: server contract in-scope; filesystem mtime as canonical timestamp; filter-applied empty state as a separate panel with `Clear filters`; zero-doc overview card renders with muted preview; `PR descriptions` is the canonical doc-type label and identifier; empty-state description sentences are frontend-hardcoded for now.
- **Codebase-research pass (2026-05-15)** resolved a further set of decisions: introduce `--ac-content-max-width` token (was missing from 0033); `--ac-bg` is the canonical app-background token (no `--ac-bg-app` exists); `Page` owns horizontal padding and `RootLayout.main` drops its padding rule; all five `<main>`-padded routes migrate to `Page` in this story (so `PageSubtitle` is deleted outright rather than shimmed); hand-rolled popover with co-located `useDismiss` (no floating-ui library introduced); `cluster_slug` and `project` are server-derived (no new frontmatter); sort secondary key falls back to `relPath` where `workItemId` is absent; on-disk `meta/prs/` directory stays (only the wire token, variant name, and label rename). See `meta/research/codebase/2026-05-15-0041-library-page-wrapper-and-overview-hub.md` for the supporting analysis.
- The `KanbanBoard` → `Page` migration that was previously deferred is now in scope here, alongside `LifecycleIndex`, `LibraryDocView`, `LibraryTemplatesView`, and `LibraryTemplatesIndex`. The story expanded from L to L/XL as a result.

## References

- Source: `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Overview screenshots: `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/library-view-updated-light.png`, `library-view-updated-dark.png`, `library-view-wide.png`, `library-view.png` (superseded composition).
- List view screenshot: `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/library-list-view.png`.
- Sort menu screenshot: `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/library-list-sort-menu.png`.
- Filter menu screenshots: `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/library-list-filter-menu-1.png`, `library-list-filter-menu-2.png`.
- Empty state screenshot: `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/library-list-no-documents.png`.
- Related: 0033, 0037, 0038, 0042.
