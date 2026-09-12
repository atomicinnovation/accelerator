---
type: "design-inventory"
id: "2026-09-10-184815-current-app"
title: "Design Inventory: current-app"
date: "2026-09-10T17:48:15+00:00"
author: "Toby Clemson"
producer: "inventory-design"
status: "draft"
relates_to: ["design-inventory:2026-05-21-004250-current-app", "design-inventory:2026-09-10-174311-claude-design-prototype"]
source: "current-app"
source_kind: "running-app"
source_location: "http://127.0.0.1:64264"
crawler: "runtime"
sequence: 3
screenshots_incomplete: true
tags: ["design", "inventory", "current-app"]
revision: "50923ba247fb5545c8dc22f76d4822110c1a4b91"
repository: "accelerator"
last_updated: "2026-09-10T17:48:15+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Design Inventory: current-app

## Overview

**Scope**: the Accelerator visualiser — a React single-page app with
history-based routing served by a local Rust HTTP server at
`http://127.0.0.1:64264`, no auth. Six distinct screen templates were
enumerated and captured: the library hub, a per-kind collection table
(representative of all 13 kinds), the templates gallery, the long-form document
detail reader, the Kanban board, and the lifecycle overview. The persistent
left rail is shared by every screen.

**Crawler methodology**: `runtime` only. A browser-locator enumerated routes and
DOM-level component presence; a single browser-analyser then visited each
template once, read the `:root` CSS custom properties via computed style, and
captured a full-page screenshot. Tokens are therefore observed at runtime, not
read from source — this differs from the prior `hybrid` current-app inventory
(sequence 2), which used the frontend source as token ground truth.

**Known gaps** (see Crawl Notes for detail):
- **Interactive states not exercised.** The executor build omits the `click`,
  `type`, and `wait_for` commands (repo bug 0287), so no sort/filter menu,
  Kanban drag, theme toggle, or detail-action outcome could be triggered. Every
  screen was observed in its `success` state only.
- **Dark-theme and design-system-reference screenshots missing.** Both need a
  click. The dark token set was recovered statically from the stylesheet's
  `[data-theme="dark"]` rule and is reported in full; five `--ds-*` / `--toast-*`
  / `--ac-empty-page-hue` custom properties resolve empty on `:root` because they
  populate only inside the un-reachable design-system view. `screenshots_incomplete`
  is `true` for this reason — not a byte-budget exhaustion.
- **Loading, empty, error, and partial states unobserved.** The local server
  resolves instantly and every collection is populated (minimum `pr-reviews` = 1);
  no route surfaced a loading, empty, or error shell.

## Design System

### Tokens

#### Colours — brand palette (`--atomic-*`, light)

| Token | Value | Category |
|-------|-------|----------|
| `--atomic-indigo` | `rgb(89,95,200)` | brand |
| `--atomic-indigo-2` | `rgb(50,48,98)` | brand |
| `--atomic-indigo-tint` | `rgb(193,197,255)` | brand |
| `--atomic-ink` | `rgb(32,34,49)` | neutral |
| `--atomic-ink-2` | `rgb(44,46,65)` | neutral |
| `--atomic-night` | `rgb(14,15,25)` | neutral |
| `--atomic-night-2` | `rgb(10,17,27)` | neutral |
| `--atomic-night-3` | `rgb(23,25,37)` | neutral |
| `--atomic-night-4` | `rgb(29,33,49)` | neutral |
| `--atomic-slate` | `rgb(95,99,120)` | neutral |
| `--atomic-slate-2` | `rgb(74,84,95)` | neutral |
| `--atomic-smoke` | `rgb(199,201,216)` | neutral |
| `--atomic-mist` | `rgb(217,217,217)` | neutral |
| `--atomic-ash` / `geyser` | `#D3DBE0` | neutral |
| `--atomic-bone` | `rgb(251,252,254)` | neutral |
| `--atomic-white` | `rgb(255,255,255)` | neutral |
| `--atomic-river-bed` | `#4A545F` | neutral |
| `--atomic-red` | `rgb(203,70,71)` | status |
| `--atomic-red-2` | `rgb(223,87,88)` | status |
| `--atomic-red-3` | `rgb(226,78,83)` | status |
| `--atomic-aquamarine` | `#73E4E2` | accent |
| `--atomic-cream-can` | `#F5C25F` | accent |
| `--atomic-marigold` | `#F9DE6F` | accent |
| `--atomic-malibu` / `sky` | `#72CBF5` | accent |
| `--atomic-medium-purple` / `violet` | `#965DD9` | accent |
| `--atomic-pastel-green` | `#6BE58B` | accent |
| `--atomic-steel-blue` | `#4295A5` | accent |
| `--atomic-teal` / `tradewind` | `#52B0AA` | accent |
| `--atomic-link-water` | `#DDECF4` | accent |
| `--atomic-overlay-ink` | `rgba(23,25,37,.56)` | overlay |
| `--atomic-stroke-light` | `rgba(255,255,255,.35)` | overlay |
| `--atomic-shadow-soft` | `rgba(0,0,0,.08)` | overlay |

#### Colours — semantic surface and text (`--ac-*`, light)

| Token | Value | Category |
|-------|-------|----------|
| `--ac-bg` | `rgb(251,252,254)` | surface |
| `--ac-bg-card` / `-raised` / `-chrome` | `#fff` | surface |
| `--ac-bg-sidebar` | `#f7f8fb` | surface |
| `--ac-bg-sunken` | `#f4f6fa` | surface |
| `--ac-bg-hover` | `rgba(32,34,49,.04)` | surface |
| `--ac-bg-active` | `rgba(89,95,200,.09)` | surface |
| `--ac-fg` | `#14161f` | text |
| `--ac-fg-strong` | `rgb(10,17,27)` | text |
| `--ac-fg-muted` | `rgb(95,99,120)` | text |
| `--ac-fg-faint` | `#8b90a3` | text |
| `--ac-accent` | `rgb(89,95,200)` | accent |
| `--ac-accent-2` | `rgb(203,70,71)` | accent |
| `--ac-accent-tint` | `rgba(89,95,200,.12)` | accent |
| `--ac-accent-faint` | `rgba(89,95,200,.06)` | accent |
| `--ac-ok` | `#2e8b57` | status |
| `--ac-warn` | `#d98f2e` | status |
| `--ac-err` | `rgb(203,70,71)` | status |
| `--ac-violet` | `#7b5cd9` | accent |
| `--ac-stroke` | `rgba(32,34,49,.1)` | border |
| `--ac-stroke-soft` | `rgba(32,34,49,.06)` | border |
| `--ac-stroke-strong` | `rgba(32,34,49,.18)` | border |

#### Colours — per-doc-kind accent and tint (light)

Each library kind has a foreground accent and a pale background tint. In dark
theme every foreground collapses to `--atomic-white` and every tint to `#1d2030`.

| Kind | Accent (light) | Tint (light) |
|------|----------------|--------------|
| `work-items` | `#af4b2f` | `#fbe9e2` |
| `work-item-reviews` | `#ad3458` | `#f9e3ec` |
| `design-inventories` | `#2e7e8a` | `#dceaec` |
| `design-gaps` | `#5c9132` | `#e7f1d8` |
| `research` | `#b26f35` | `#f7ece0` |
| `plans` | `#3256b6` | `#e3ecf6` |
| `plan-reviews` | `#5127b5` | `#ebe3f5` |
| `validations` | `#2e8b57` | `#def0e7` |
| `pr-descriptions` | `#4588b8` | `#e2eff7` |
| `pr-reviews` | `#7f2cb6` | `#efe2f6` |
| `root-cause-analyses` | `#ab2c96` | `#f6e2f1` |
| `decisions` | `#ad3437` | `#fbe5e6` |
| `notes` | `#8e7b22` | `#f5f0d6` |

#### Colours — lifecycle stage (`--ac-stage-*`)

| Stage token | Light | Dark |
|-------------|-------|------|
| `work-items` | `#c52828` | `#e26060` |
| `research` | `#c56327` | `#e29560` |
| `plans` | `#2762c5` | `#6094e2` |
| `plan-reviews` | `#662cc5` | `#9560e2` |
| `validations` | `#208a52` | `#60e2a3` |
| `pr-descriptions` | `#2796c5` | `#60c2e2` |
| `pr-reviews` | `#952cc5` | `#c060e2` |
| `decisions` | `#c5273f` | `#e26077` |

#### Colours — code surface and syntax (`--code-*`, `--tk-*`)

The code surface is dark in both themes. `--tk-*` is a full Prism-style
highlight set; representative entries below.

| Token | Value | Category |
|-------|-------|----------|
| `--code-bg` | `#0e1320` | code surface |
| `--code-bg-head` | `#161b2c` | code surface |
| `--code-fg` | `#d7dcec` | code surface |
| `--code-fg-faint` | `#6f7796` | code surface |
| `--code-stroke` | `rgba(255,255,255,.07)` | code surface |
| `--tk-kw` (keyword) | `#c1c5ff` | syntax |
| `--tk-str` (string) | `#6be58b` | syntax |
| `--tk-num` (number) | `#f9de6f` | syntax |
| `--tk-fn` (function) | `#ffc1a8` | syntax |
| `--tk-typ` (type) | `#73e4e2` | syntax |
| `--tk-com` (comment) | `#6f7796` | syntax |
| `--tk-tag` | `#df5758` | syntax |
| `--tk` diff-add / diff-del | `#6be58b` / `#e56b7e` | syntax |

#### Colours — dark theme overrides (`[data-theme="dark"]`)

Dark theme is driven by a `[data-theme="dark"]` attribute, not
`prefers-color-scheme`. Only overridden tokens are listed; unlisted tokens
inherit their light value.

| Token | Value (dark) | Category |
|-------|--------------|----------|
| `--ac-bg` | `rgb(10,17,27)` (`--atomic-night-2`) | surface |
| `--ac-bg-raised` / `-chrome` | `rgb(14,15,25)` (`--atomic-night`) | surface |
| `--ac-bg-sunken` | `#070b12` | surface |
| `--ac-bg-sidebar` | `#0b121c` | surface |
| `--ac-bg-card` | `#131524` | surface |
| `--ac-bg-hover` | `rgba(255,255,255,.04)` | surface |
| `--ac-bg-active` | `rgba(89,95,200,.22)` | surface |
| `--ac-fg` | `#e7e9f2` | text |
| `--ac-fg-strong` | `#fff` | text |
| `--ac-fg-muted` | `#a0a5b8` | text |
| `--ac-fg-faint` | `#6c7088` | text |
| `--ac-accent` | `#8a90e8` | accent |
| `--ac-accent-2` | `#e86a6b` | accent |
| `--ac-accent-tint` | `rgba(138,144,232,.18)` | accent |
| `--ac-accent-faint` | `rgba(138,144,232,.08)` | accent |
| `--ac-ok` / `-warn` / `-err` | `#79d9a6` / `#e4b76e` / `#e86a6b` | status |
| `--ac-stroke` | `rgba(255,255,255,.08)` | border |
| `--ac-stroke-soft` | `rgba(255,255,255,.04)` | border |
| `--ac-stroke-strong` | `rgba(255,255,255,.16)` | border |

#### Typography

| Token | Value | Category |
|-------|-------|----------|
| body font | `"Inter", system-ui, sans-serif` | family |
| display font | `"Sora", system-ui, sans-serif` | family |
| mono font | `"Fira Code", ui-monospace, monospace` | family |
| `--size-95` … `--size-680` | 9.5, 10, 10.5, 11, 11.5, 12, 12.5, 13, 14, 14.5, 16, 18, 20, 22, 26, 28, 36, 48, 68 px | size scale |
| line-heights | tight 1.05 · snug 1.2 · normal 1.5 · loose 1.6 · prose 1.65 | line-height |
| `--tracking-caps` | `.12em` | tracking |

#### Spacing, radii, shadows

| Token | Value | Category |
|-------|-------|----------|
| `--sp-1` … `--sp-11` | 4, 8, 12, 16, 24, 32, 40, 48, 64, 80, 124 px | spacing |
| `--radius-0` … `--radius-12` | 0, 1, 2, 3, 4, 6, 8, 12 px | radius |
| `--radius-full` / `--radius-pill` | 50% / 999px | radius |
| `--ac-shadow-soft` | `0 1px 2px rgba(10,17,27,.04), 0 8px 28px rgba(10,17,27,.06)` | shadow |
| `--ac-shadow-lift` | `0 2px 4px rgba(10,17,27,.06), 0 20px 60px rgba(10,17,27,.1)` | shadow |
| `--shadow-crisp` | `0 1px 2px rgba(10,17,27,.06), 0 4px 12px rgba(10,17,27,.04)` | shadow |
| `--shadow-card` | `6px 12px 85px 0 rgba(0,0,0,.08)` | shadow |
| `--shadow-card-lg` | `12px 24px 120px 0 rgba(0,0,0,.12)` | shadow |

### Layout Primitives

| Primitive | Value | Notes |
|-----------|-------|-------|
| left rail | fixed 256px | `nav._sidebar`, bg `--ac-bg-sidebar`, 1px right border |
| content offset | 256px from left | main content column |
| `--ac-content-max-width` | 1200px | inner `section._page`, padding `0 40px 48px` |
| `--ac-content-max-width-narrow` | 600px | narrow forms |
| `--ac-content-max-width-prose` | 72ch | prose reading width |
| `--ac-topbar-h` | 48px | topbar height token |
| hub grid | 3 equal cols (~306px), 12px gap | `_hubGrid` per phase section |
| kanban board | 240px columns, 16px gap | 4-up at 1280px viewport with slight horizontal scroll |
| lifecycle row | ~650px content + ~228px pipeline | 2-column grid, 8px/24px gap |

No explicit media breakpoints were detected — only the 1280×720 viewport was
observed.

## Component Catalogue

### Sidebar (left rail)

- **Variants / props**: fixed 256px; brand lockup (Accelerator / VISUALISER),
  host + `SSE` live-connection indicator, Dark-theme toggle, Mono-font toggle,
  search box, phase-grouped nav, `v1.24.0-pre.65` version button at the foot.
- **Used on screens**: all.
- **Source**: `nav._sidebar` (runtime selector).

### Breadcrumb

- **Variants / props**: kind label path (e.g. `Library / work-items`), 13px,
  bottom border `--ac-stroke-soft`.
- **Used on screens**: collection, templates-gallery, doc-detail.

### Page heading

- **Variants / props**: eyebrow (caps, tracked) + h1 (Sora, 28px/600, tracking
  −0.28px, `--ac-fg-strong`) + lead paragraph.
- **Used on screens**: all top-level screens.

### Collection table

- **Variants / props**: CSS-grid list (`role=row`, not an HTML `<table>`);
  columns ID/DATE · TITLE · STATUS · SLUG · MODIFIED; a document-count line; a
  sort menu-button ("Recently modified") and a filter menu-button.
- **Used on screens**: all 13 collection screens.

### Status badge

- **Variants / props**: text-only span, 10.5px (`--size-105`), weight 500,
  letter-spacing 0.21px, no pill background. Five variants: `draft` muted
  (`--ac-fg-muted`), `ready` and `in-progress` accent indigo (`--ac-accent`),
  `done` green (`--ac-ok`), `abandoned` red (`--ac-err`).
- **Used on screens**: collection, doc-detail (as a mono pill), kanban, lifecycle.

### Kind card (hub)

- **Variants / props**: white bg, radius 6px, padding 14px 16px, ~307×87px,
  Inter 16px; one per kind, grouped by lifecycle phase.
- **Used on screens**: library-hub.

### Template row

- **Variants / props**: grid row (filename 224px / tier ~680px / trailing arrow
  38px), 48px tall, flat list (no card). Tier indicators `default` / `user` /
  `config` — active `default` is indigo (`--ac-accent`), inactive faint grey
  (`--ac-fg-faint`).
- **Used on screens**: templates-gallery.

### Document detail header

- **Variants / props**: h1 title, mono status pill (Fira Code 10.5px/500,
  coloured per status), date, author, "Open in editor" link, "Copy path" button.
- **Used on screens**: doc-detail.

### Metadata aside

- **Variants / props**: 280px panel, padding-left 24px, 13px Inter muted; three
  labelled blocks — RELATED ARTIFACTS, FILE (path, `sha256-…` etag, size),
  CLUSTER (sibling links + full frontmatter key:value rows).
- **Used on screens**: doc-detail.

### Kanban card

- **Variants / props**: ~214px draggable card (`roledescription: sortable`, with
  keyboard drag instructions); shows a type glyph, work-item number, "Lifecycle
  pipeline, N of 7 stages complete", title, slug, relative timestamp.
- **Used on screens**: kanban-board.

### Lifecycle row and stage chain

- **Variants / props**: `_cardLink_` row — title, slug, status badge, timestamp,
  "N artifacts", pipeline summary, and an `ac-stagechain` visual of seven ~125px
  stages coloured by `--ac-stage-*`.
- **Used on screens**: lifecycle-overview.

## Screen Inventory

### library-hub — `/library`

- **Purpose**: landing hub listing every artifact kind grouped by lifecycle phase.
- **Components used**: Sidebar, Page heading, Kind card (six phase sections).
- **States observed**: success.
- **Key interactions**: kind-card and nav links (not exercised — no click command).
- **Screenshot**: `screenshots/library-hub.png`

### collection — `/library/work-items`

- **Purpose**: sortable, filterable table of one kind's documents (representative
  of all 13 kinds).
- **Components used**: Sidebar, Breadcrumb, Page heading, Collection table,
  Status badge, sort menu-button, filter menu-button.
- **States observed**: success. All five status-badge variants present.
- **Key interactions**: sort and filter menus (not exercised — no click command).
- **Screenshot**: `screenshots/collection.png`

### templates-gallery — `/library/templates`

- **Purpose**: browse the starting template for every doc kind and see which tier
  is active.
- **Components used**: Sidebar, Breadcrumb, Page heading, Template row (13 rows).
- **States observed**: success. Tier indicators: `default` active, `user` /
  `config` inactive.
- **Screenshot**: `screenshots/templates-gallery.png`

### doc-detail — `/library/design-inventories/015231-claude-design-prototype`

- **Purpose**: long-form reader for one document with a metadata aside.
- **Components used**: Sidebar, Breadcrumb, Document detail header (status pill
  "Superseded", red mono), Metadata aside (RELATED ARTIFACTS / FILE / CLUSTER),
  rendered-markdown body (Sora headings).
- **States observed**: success (a "Superseded" instance — a non-default status
  example). RELATED ARTIFACTS showed its empty sub-state inline.
- **Key interactions**: "Open in editor", "Copy path" (not exercised).
- **Screenshot**: `screenshots/doc-detail.png`

### kanban-board — `/kanban`

- **Purpose**: all 288 work items grouped by status; drag to move (persists to
  disk).
- **Components used**: Sidebar, Page heading, live indicator, status columns
  (Draft 113, Ready, In progress, Done, Other), Kanban card.
- **States observed**: success. The defining drag interaction was not
  exercisable (no click/drag command).
- **Screenshot**: `screenshots/kanban-board.png`

### lifecycle-overview — `/lifecycle`

- **Purpose**: every work unit and how far it has progressed through the seven-
  stage pipeline.
- **Components used**: Sidebar, Page heading, "Updated" / "Completeness" sort
  toggles, Lifecycle row with stage chain (~494 links).
- **States observed**: success. "Completeness" sort toggle not exercisable
  (active "Updated" state observed statically only).
- **Screenshot**: `screenshots/lifecycle-overview.png`

## Feature Catalogue

### artifact-library-browsing

- **Capability**: browse every `meta/` artifact, one sortable/filterable table
  per kind, reached from a phase-grouped hub.
- **Surfaces on**: library-hub, collection.
- **Depends on**: the `meta/` corpus on disk; the Rust server's listing endpoints.

### document-reading

- **Capability**: read one document's rendered markdown alongside its
  frontmatter, file metadata, cluster, and cross-links.
- **Surfaces on**: doc-detail.
- **Depends on**: per-document read + frontmatter-parse endpoints.

### kanban-status-board

- **Capability**: view work items by status and drag a card to change its status,
  writing back to the file on disk.
- **Surfaces on**: kanban-board.
- **Depends on**: work-item corpus; a status-write endpoint.

### lifecycle-overview

- **Capability**: track each work unit across a seven-stage pipeline (Work item →
  Research → Plan → Plan review → Validation → PR descriptions → PR review).
- **Surfaces on**: lifecycle-overview.
- **Depends on**: cluster/lifecycle aggregation of the corpus.

### live-activity-feed

- **Capability**: a server-sent-events stream surfaces recently created or edited
  documents; the `SSE` rail indicator signals the live connection.
- **Surfaces on**: all screens (rail indicator); ACTIVITY (LIVE) nav panel.
- **Depends on**: an SSE endpoint on the Rust server.

### theme-and-font-toggles

- **Capability**: switch between light and dark theme (`[data-theme]`) and toggle
  a mono display font.
- **Surfaces on**: Sidebar (all screens).
- **Depends on**: client-side attribute toggle; dark token set in the stylesheet.

### templates-gallery

- **Capability**: browse the starting template for each doc kind and see which
  configuration tier (default / user / config) is active.
- **Surfaces on**: templates-gallery.

### design-system-reference

- **Capability**: a hidden design-system reference revealed by triple-clicking the
  version button. Not reachable in this crawl (no click command); inferred from
  the five `--ds-*` / `--toast-*` / `--ac-empty-page-hue` custom properties that
  resolve empty on `:root`.
- **Surfaces on**: version button in the Sidebar.

## Information Architecture

Primary navigation is grouped into lifecycle phases in the left rail. Collections
and details relate as `/library/<kind>` → `/library/<kind>/<slug>`; details
cross-link to sibling artifacts in the same cluster.

| Route | Screen | Notes |
|-------|--------|-------|
| `/` | — | redirects to `/library` |
| `/library` | library-hub | phase-grouped kind links |
| `/library/work-items` | collection | 288 documents |
| `/library/work-item-reviews` | collection | 134 |
| `/library/design-inventories` | collection | 5 |
| `/library/design-gaps` | collection | 3 |
| `/library/research` | collection | 173 |
| `/library/plans` | collection | 194 |
| `/library/plan-reviews` | collection | 176 |
| `/library/validations` | collection | 74 |
| `/library/pr-descriptions` | collection | 82 |
| `/library/pr-reviews` | collection | 1 |
| `/library/root-cause-analyses` | collection | 5 |
| `/library/decisions` | collection | 68 |
| `/library/notes` | collection | 19 |
| `/library/templates` | templates-gallery | 13 template rows |
| `/library/<kind>/<slug>` | doc-detail | dynamic per-document reader |
| `/kanban` | kanban-board | work items by status |
| `/lifecycle` | lifecycle-overview | work units across seven stages |

Left-rail nav groups: **LIBRARY** (hub) · **DEFINE** (work items, work-item
reviews) · **DISCOVER** (design inventories, design gaps, research) · **BUILD**
(plans, plan reviews, validations) · **SHIP** (PR descriptions, PR reviews) ·
**OPERATE** (root cause analyses) · **REMEMBER** (decisions, notes) · **VIEWS**
(Kanban, Lifecycle) · **ACTIVITY (LIVE)** · **META** (templates).

## Crawl Notes

- **Executor interaction commands absent (bug 0287).** The daemon's allowlist was
  `ping, navigate, snapshot, screenshot, evaluate, links, daemon-stop` — no
  `click`, `type`, or `wait_for`. Every screen was captured in its `success`
  state; no menu, drag, toggle, or detail action could be triggered, and the
  hidden design-system reference (triple-click) was unreachable.
- **`screenshots_incomplete: true`.** Two planned screenshots — `library-hub-dark.png`
  and `design-system-reference.png` — were not produced because both require a
  click. This is a tooling gap, not a byte-budget exhaustion; the six captured
  screenshots total ~0.83 MB against the 50 MB budget. Dark-theme tokens were
  recovered statically from the `[data-theme="dark"]` stylesheet rule and are
  reported in full.
- **Screenshot output root not set by the skill (bug 0286).** The analyser worked
  around the unset `ACCELERATOR_INVENTORY_OUTPUT_ROOT` by exporting the target
  directory inline before capturing.
- **No auth walls, 404 shells, or navigation-refusal policy refusals** were
  encountered; all routes rendered populated content.
- **URL scrubbing**: no query strings were present on any observed route; none
  were written into this document.
- **Crawl bounds**: well within limits — 8 locator navigations plus 6 analyser
  route visits (one extra collection-index hop to resolve a slug), 0 policy
  refusals, 0 unreachable routes, under the 50-route cap and 5-minute wall-clock.

## References

- Source: `http://127.0.0.1:64264`
- Related: `meta/research/design-inventories/2026-05-21-004250-current-app/inventory.md`
  (prior current-app inventory, hybrid, sequence 2 — superseded by this one)
- Related: `meta/research/design-inventories/2026-09-10-174311-claude-design-prototype/inventory.md`
  (claude-design-prototype target, sequence 3 — gap-analysis counterpart)
