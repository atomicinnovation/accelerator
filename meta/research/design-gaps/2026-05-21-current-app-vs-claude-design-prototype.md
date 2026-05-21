---
date: "2026-05-21T03:53:43+01:00"
type: design-gap
current_inventory: "/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/build-system/meta/research/design-inventories/2026-05-21-004250-current-app/inventory.md"
target_inventory: "/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/build-system/meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/inventory.md"
author: "Toby Clemson"
status: draft
tags: [design, gap-analysis]
---

# Design Gap Analysis: current-app → claude-design-prototype

## Overview

This gap compares the current Accelerator Visualiser frontend (React 19 SPA
served by a local Rust HTTP server at `http://127.0.0.1:52339/`, commit
`07413d3a13`) against the claude-design-prototype (script-tag React + hash
router at `http://localhost:54844/Accelerator%20Visualiser.html`, captured
from `/Users/tobyclemson/Downloads/Accelerator/`). Both inventories were
captured on 2026-05-21 with hybrid (code-static + Playwright) methodology
and both treat the document detail page as the primary focus area. Coverage
is broadly symmetric across detail routes and the surrounding shell;
limitations carried over from the source inventories include the
malformed-frontmatter banner and wiki-link pending state (current app —
documented from source, not visually verified), the design-inventory
screenshots-gallery (prototype — present in source but not rendering at
runtime), and the prototype's lack of any 404/error screen. This gap focuses
on differences rather than parity, so areas of clean alignment (colour
tokens, font families, base spacing scale, base radii) are noted briefly in
sequencing rather than enumerated as drift. Readers extracting work items
from this artifact should note that we need to treat every prose paragraph
below as a candidate driver for a discrete work item rather than collapsing
related drifts into a single epic.

## Token Drift

The two inventories share a core `--ac-*` semantic token layer with
identical light-theme values for surface, foreground, stroke, accent, and
status tokens, and they share the same Sora / Inter / Fira Code typographic
stack at the same brand sizes. The drift sits in two places: a missing
brand layer in the current codebase, and divergent runtime consumption of
the size scale and per-doc-type hue palette.

We need to add the `--atomic-*` brand-layer palette declared in the
prototype's `assets/tokens.css:34-105` — roughly thirty named tokens such
as `--atomic-night`, `--atomic-ink`, `--atomic-indigo`, `--atomic-marigold`,
`--atomic-aquamarine`, `--atomic-cream-can`, plus the aliases
(`--atomic-violet`, `--atomic-teal`) and overlays
(`--atomic-overlay-ink`, `--atomic-stroke-light`, `--atomic-shadow-soft`).
The current `src/styles/global.css` declares only the semantic `--ac-*`
layer with no brand-named source-of-truth, so type-tinted iconography and
illustrative artwork have nowhere to pull from. We need a layered
architecture where the brand palette feeds the `--ac-*` semantic layer
rather than baking hex literals into the semantic layer directly.

The system must expose per-doc-type hues to the detail surface. The
prototype defines a thirteen-entry HSL map in `src/ui.jsx:264-278`
(`work` hue 12, `decisions` 355, `research` 28, `plans` 220, `notes` 50,
`design-inventories` 185, `design-gaps` 95, and so on) consumed by
`TypeGlyph`, `StageTile`, `BigGlyph`, and the empty-page tint. The current
app already declares `--ac-doc-{key}` and `--ac-doc-bg-{key}` tokens at
`src/styles/global.css:98-124` but explicitly does not consume them on the
detail page — they are restricted to the sidebar and library hub. We need
to surface these tokens on the detail page so the eyebrow, aside related
rows, and any future hero illustration carry consistent per-type colour.

We need to align the typography-size consumption pattern. The brand size
scale (`--size-hero` … `--size-chip-md`) is defined in both stylesheets at
identical px values, but consumption diverges sharply: the current app
consumes the tokens (Page H1 uses `--size-h3`, chips use `--size-chip`,
markdown body inherits `--size-sm`), while the prototype defines them and
then hard-codes pixel values per component (body `14px`, page H1 `28px`,
eyebrow `11px`, chip `10.5px`, markdown body `14.5px`). The current app
also has its own off-scale outliers — `MarkdownRenderer` H1 at `1.75rem`,
`Page.module.css` eyebrow at `11px`, subtitle at `13px`, `RelatedArtifacts`
badge radius at `2px`, markdown `<pre>` radius at `6px`. We need a single
canonical scale-consumption rule (either everything pulls from `--size-*`
or the scale is dropped) and migration of every hard-coded outlier onto
that rule.

We need to add code-block local-palette tokens. The prototype declares a
self-contained dark code-block palette at `src/app.css:766-800` —
`--code-bg #0E1320`, `--code-fg #D7DCEC`, plus token-class colours
`--tk-com`, `--tk-str`, `--tk-num`, `--tk-kw`, `--tk-lit`, `--tk-typ`,
`--tk-fn`, `--tk-attr`, `--tk-var`, `--tk-key`, `--tk-tag`, `--tk-prop`,
`--tk-sel`, and diff tokens `--tk-dhdr`, `--tk-dhunk`, `--tk-dadd`,
`--tk-ddel`. The current app relies on hljs default class names with no
named token layer for syntax colours, which leaves no surface for theming
or per-language overrides. We need to introduce an equivalent token set so
code blocks render with the same theme-independent palette in both light
and dark themes.

We need to reconcile the shadow tokens. The prototype defines
`--ac-shadow-soft` and `--ac-shadow-lift` with light- and dark-theme
variants (`0 1px 2px rgba(10,17,27,0.04), 0 8px 28px rgba(10,17,27,0.06)`
and `0 2px 4px rgba(10,17,27,0.06), 0 20px 60px rgba(10,17,27,0.10)`); the
current app's `global.css:172-173,239-240` declares the same token names
per theme but the values were not captured in the inventory, indicating
either drift or undocumented parity. We need to audit the current values
and either align them with the prototype's elevation curve or document the
intentional divergence.

We need to widen the dark-theme accent. The prototype's dark theme remaps
`--ac-accent` to `#8A90E8` and `--ac-accent-2` to `#E86A6B` so accents
preserve contrast on the deep-night surface; the current app's dark mirror
is documented from source only and was not visually verified, so we need
to confirm that the dark accent in the current app actually shifts and, if
it does not, migrate it to the brighter prototype values.

## Component Drift

The component catalogues overlap on Chip, the page chrome, the related-
artifacts aside, the markdown renderer, and the template/lifecycle/kanban
views, but every overlapping component carries non-trivial structural,
behavioural, or stylistic drift. The biggest cluster sits in the detail
page itself, where the prototype's `DocPage` is a single component
spanning all twelve doc kinds while the current app uses TanStack-Router
detail routes against the shared `LibraryDocView`.

We need to add a frontmatter table to the detail body. The prototype
renders a `.ac-fm` CSS-grid (`auto 1fr`, Fira Code 11.5px,
`--ac-bg-sunken` background, `--ac-stroke` border, padding `12px 14px`)
showing every frontmatter key/value pair directly above the markdown body,
with `WORK*` values auto-linkified. The current app's `FrontmatterChips`
renders the same information as a chip strip in the subtitle slot, which
collapses everything to pill width and forces awkward truncation on long
values (the captured screenshots show a 40-character git commit hash on
research, a full file path on design-gaps, and a six-lens comma list on
plan-reviews — all behave as the widest chip in the strip). We need a
parallel rendering: keep the chip strip for status / verdict / date /
author signal and add the full frontmatter table for completeness.

We need to harmonise the aside section vocabulary. The current
`RelatedArtifacts` produces three groups (declared `Targets`, declared
`Referenced by`, inferred `Same lifecycle`) with a legend and a 2px solid
indigo / 2px dashed faint visual differentiation; the prototype produces
four sections in fixed order (`Related artifacts` always, `Declared links`
when `fm.target` exists, `File` always, `Cluster` when a matching cluster
exists) with a flatter `(declared)` / `(inferred)` text tag and no inline
legend. The aside eyebrow typography also drifts: the current app uses
Inter 12 / weight 600 / uppercase for aside H3s, while the prototype uses
Fira Code 10.5 / uppercase. We need a single canonical aside structure —
either the current declared/referenced/inferred trichotomy migrated into
the prototype's section ordering, or the prototype's flatter model adopted
in the current app — and a single eyebrow typography rule applied
consistently across the aside, lifecycle stage rail (currently Inter 12
weight 400 — a third style), and page eyebrow.

We need a Cluster section in the aside. The prototype's aside includes a
dedicated `Cluster` block (when a matching cluster exists) that navigates
to `#/lifecycle/<slug>` with cluster title and `<n> artifacts · <updated>`
metadata; the current app surfaces same-lifecycle relations as links
inside the `Same lifecycle` group but offers no dedicated "go to cluster"
affordance. Users need a one-click path from any document into its
lifecycle pipeline view.

Users need detail-page header actions. The prototype's `DocPage` ships
two right-aligned topbar buttons — `Open in editor` (`Icon name="edit"`)
and `Copy link` (`Icon name="link"`) — on every detail page. They are
decorative in the prototype but represent a deliberate affordance slot
that the current app's `Page` chrome already supports via the `actions?`
prop (`PageProps`, `Page.tsx:4-11`) but does not populate for
`LibraryDocView`. We need to wire concrete actions into that slot — at
minimum a working "Copy link" producing the canonical document URL, and
an "Open in editor" deep-link when an editor protocol is configured.

We need a StatusBadge component. The prototype's `StatusBadge` maps both
`status` and `verdict` frontmatter keys to a coloured chip tone
(`Accepted` → green, `Draft` → amber, `pass` → green, and so on); the
current app's `FrontmatterChips` only colours the `status` key via
`statusToChipVariant` and renders the `verdict` key as neutral (the
validation detail page renders the `pass` verdict as a neutral chip — no
semantic colour). We need a single component that maps both keys
identically, so review and validation pages signal their outcome at a
glance.

We need to upgrade the markdown renderer. The current app uses
`react-markdown` + `remark-gfm` + `remarkWikiLinks` + `rehype-highlight`,
which gives it real GFM (task lists, tables), real syntax highlighting
via hljs, and working wiki-link routing via TanStack-Router `Link`. The
prototype uses a hand-rolled `renderMarkdown` helper in `src/ui.jsx:122`
supporting only h1/h2/h3, p, ol/ul, table, fenced code (via
`HighlightedCode`/`tokenize`), inline `code`, `**bold**`, `*em*`, and
`[[wiki-links]]` (rendered as decorative `<a href="#">`). The current
app's renderer is strictly more capable on parser depth and link routing.
We need to keep the current renderer for parser correctness and adopt
only the prototype's syntax-highlight palette (the `--tk-*` tokens) so
fenced code blocks render with the prototype's visual treatment without
losing GFM, wiki-link routing, or hljs language support.

We need a HexChain pipeline visualisation. The prototype's `HexChain` /
`StageTile` components render a horizontal chain of tinted tiles with
connectors representing the eight-stage lifecycle (Work item → Research →
Plan → Plan review → Validation → PR description → PR review → Decision);
the current app's lifecycle cluster realises the stages as vertical rows
with a `padding-left: 22px` rail rather than a sibling rail element, and
has no HexChain on the lifecycle index. The system must expose a HexChain
on each cluster card in the lifecycle index and on the cluster detail
header so stage presence is visible at a glance without scrolling the
vertical timeline.

We need a BigGlyph hero illustration set. The prototype ships
per-doc-type hero illustrations (`src/big-glyphs.jsx`) with a 7-tone
palette derived from a single hue (`bigPalette(hue)`) and a default
fallback, used in `LibraryIndexEmpty` and the landing card hero; the
current app has no equivalent illustration system. Users need a
recognisable visual anchor on each doc-type empty state so the screen
does not collapse to a bare line of text.

We need an AtomicMark brand glyph. The prototype renders an inline-SVG
gradient hex brand mark (`AtomicMark`, `src/ui.jsx:56`) at 28px in the
topbar; the current app's `Brand` component renders a plain glyph in
`--ac-accent-2` red. We need to harmonise the brand expression — either
adopt the gradient hex or update the prototype to match the current red
mark — so the visualiser presents one identity across both surfaces.

We need a Toaster notification surface. The prototype defines a
`Toaster` component (`Accelerator Visualiser.html:40`) rendering a
fixed bottom-right toast stack, triggered by `pushToast` from the kanban
drag handler and the simulated cache-invalidation flow; the current app
has no toast surface documented. Users need transient confirmation of
state-changing actions (kanban moves, copy-link, external-edit
detection) without a modal or page reload.

We need a Topbar topology adjustment. The prototype's topbar carries
workspace label `Accelerator · VISUALISER`, breadcrumbs, server status
text (`127.0.0.1:52914`), `SSE` indicator (plain Fira Code 11px, no
chip styling), and a theme-toggle icon button — and notably does *not*
expose a font-mode toggle even though `[data-font="mono"]` styling
exists in the stylesheet. The current app's topbar carries `Brand`,
breadcrumbs, `OriginPill`, `SseIndicator` (both rendered as actual
pills, not plain text), `ThemeToggle`, and `FontModeToggle`. We need to
decide whether the origin and SSE indicators should be pills or plain
text (the prototype's "state indicators are not pills" note flags the
prior pill styling as overstating chrome) and whether `FontModeToggle`
stays in the topbar or moves to a tweaks panel.

We need a Tweaks panel. The prototype includes a `TweaksPanel`
(`Accelerator Visualiser.html:59`) — a fixed bottom-left iframe-host
edit-mode panel exposing theme, font mode, and other live tweaks —
intended for the design-tweaking workflow. The current app has no such
panel. Users need a single place to flip theme, font mode, and any
other live design variables without the topbar carrying every toggle.

We need a dev design-system reference page. The prototype's
`DevDesignSystem` (`src/view-dev.jsx`) is a hidden 24-section reference
page covering every primitive (Overview, Colours, Type, Spacing, Radii
& shadows, Icons, Doc-type glyphs, Empty-state glyphs, Atomic mark,
Chips, Status badges, Stage dots, Tier pills, Buttons, Inputs & form,
Sidebar nav, Cards, Tables, Markdown, Code blocks, Frontmatter,
Empty & banners, Toasts, Topbar) activated by `#dev` hash,
Cmd/Ctrl+Shift+D keybind, or a sidebar foot triple-click; the current
app exposes `/glyph-showcase` and `/chip-showcase` as separate
uncrumbed dev routes covering only two primitives. Implement
DevDesignSystem in the current app as a single consolidated reference
page that supersedes the two showcase routes. (The prototype's
scroll-spy defect — TOC active highlight pinned to `02 Colours` — must
not be copied; the new implementation should drive active highlight
from the actual scroll source.)

We need an omnibar search. The prototype's `SearchBox` (`src/search.jsx`)
provides a debounced sidebar omnibar with `/` global keybind, inline
results panel, match counter, ranking by title / slug / id prefix → mtime,
and Enter-to-navigate behaviour. The current app's sidebar `Search row`
is documented only as a label — no debounce, ranking, keybind, or
results-panel detail surfaced. Users need a fast, keyboard-driven
omnibar across the whole `LIBRARY_INDEX` corpus.

Users need per-type counts in the sidebar. The prototype's sidebar shows
per-type document counts inline next to each LIBRARY group entry
(`Work items 14`, `Decisions 9`, `Templates 5`); the current sidebar
lists types under DEFINE/DISCOVER/BUILD/SHIP/REMEMBER groupings with no
count. We need to surface these counts so users know whether a list is
worth opening before clicking through.

## Screen Drift

Every screen that exists on both sides has visual or structural drift; the
chip-strip rationalisation, the lifecycle layout, and the kanban column
model are the three load-bearing differences.

We need to rebalance the detail-page chip strip. The current app's chip
strip in the subtitle slot includes every frontmatter key for some doc
kinds (work-item: id, type, status, priority, tags, title, author, date)
and is empty for others (notes render with no chips, leaving H1 sitting
directly over the divider). On design-inventory pages the chips duplicate
author and timestamp values (`last_updated` mirrors `date`,
`last_updated_by` mirrors `author`), and on design-gap and research pages
chips carry full file paths or 40-character git hashes that dominate the
strip. The prototype's chip row is hard-limited to status + verdict +
date + author (max four chips). The system must enforce a maximum chip
set in the strip; everything else lives in the frontmatter table proposed
in §Component Drift.

We need to humanise every detail-page H1. The current app surfaces a
humanised `entry.title` on plan, research, decision, design-inventory,
design-gap, and work-item detail pages, but renders the raw slug as H1
on work-item-review, plan-review, and validation pages (the
`0042-templates-view-redesign-review-1` form). The prototype consistently
uses a humanised H1 across all twelve kinds. Users need a single rule for
how H1 is derived, applied uniformly.

We need to redesign the lifecycle cluster layout. The current app renders
the eight-stage pipeline as vertical rows with a `padding-left: 22px`
rail on each row, an Inter 12 / weight 400 uppercase stage label (versus
weight 600 elsewhere — a typography inconsistency), and a "no <stage>
yet" placeholder per missing stage. The prototype renders the cluster
with a sticky pipeline header containing a `HexChain` (horizontal tinted
tiles showing presence at a glance) above the vertical stage timeline.
Users need both representations simultaneously — quick visual scan plus
detailed scrollable rows — without the eyebrow weight inconsistency.

We need to redesign the lifecycle index. The current app's `/lifecycle`
index sorts by Recent / Oldest / Completeness but does not visualise
stage presence; the prototype's `#/lifecycle` shows each cluster as a
card with an embedded `HexChain` so completeness is visible without
opening the cluster. We need the index to embed the stage chain on every
cluster card.

We need to reconcile the kanban column model. The current app's
`KanbanBoard` exposes four columns (Draft / Ready / In progress / Done)
with no drag-and-drop documented; the prototype's `KanbanBoard` exposes
three columns (Todo / In progress / Done) with HTML5 drag-and-drop,
status write-back, and toast confirmations. Users need a single column
schema (we need to decide whether Draft and Ready are distinct lanes or
collapse into Todo) and drag-and-drop status mutation backed by
disk write-back and toast feedback.

We need to redesign the library landing. The current `/library` hub
groups doc types under per-phase H2 headings (`DEFINE` / `DISCOVER` /
`BUILD` / `SHIP` / `REMEMBER`) and surfaces an inline `(no documents
yet)` placeholder for empty types; the prototype's `LibraryLanding`
renders a per-type card grid with a hero `TypeGlyph` per card and a
dashed/striped `LibraryLandingEmptyCard` variant for empty types. Users
need a more illustrative landing that signals each doc kind's identity
at a glance rather than a tabular grouping.

We need a full-page empty state per type. The current
`/library/{type}` list view renders a sortable table; when a type has
zero rows the inventory does not document a dedicated empty state
(the data-flow degrades silently). The prototype's `LibraryIndexEmpty`
renders a full-page empty state with `BigGlyph` hero and `TYPE_COPY`
purpose copy explaining what the type is for. Users need this
empty-state treatment so a freshly-initialised repo does not present
twelve identical empty tables.

We need a 404 / error screen with affordances. The current app's
`error-not-found` renders the `Page` shell with H1 `"Document not
found"` and a single `<p>Document not found.</p>` line — no back-link,
retry, or related-suggestions affordance — while the prototype has no
404 screen at all (unknown hashes were not exercised). Users need a
404 surface that offers at minimum a link back to the library landing
and the parent type's list, and ideally search suggestions for nearby
slugs.

We need to harmonise the markdown body width. The current app caps
the markdown body at a hard-coded `max-width: 720px` in
`MarkdownRenderer.module.css:2` (off the `--ac-content-max-width-narrow`
600px scale); the prototype caps at `max-width: 72ch` on `.ac-md` with a
14.5px / line-height 1.65 body. The two caps produce different visual
reading widths in practice. We need to settle on one cap (px or `ch`)
and one body-text size and apply it across both surfaces.

We need to fix the templates preview body. The current app's template
preview pane uses Fira Code 12 with `white-space: normal`, which
collapses internal whitespace despite the monospace face — an obvious
styling oversight in a code-preview surface. The prototype's templates
list reuses `HighlightedCode` and preserves whitespace via per-line
wrappers. The system must set `white-space: pre` (or `pre-wrap`) on the
preview body so template content renders verbatim.

We need to redesign templates IA. The current app exposes both
`/library/templates` (index) and `/library/templates/{name}` (per-template
detail with tier picker, preview pane, and three-tier resolution
panels — plugin-default, user-override, config-override); the prototype
exposes only `#/library/templates` (single list with `config user
default` tier pills inline). Users need the per-template detail page
preserved from the current app (it carries content the prototype list
cannot), and the list page needs the tier-pill compactness from the
prototype so the list reads at a glance before the user clicks through.

## Net-New Features

These features exist in the prototype but are absent (or absent in
practice) from the current app. Each is scoped to the screens it
surfaces on and the components it depends on.

Implement Omnibar to provide a debounced, keyboard-driven search across
the full `LIBRARY_INDEX` corpus with `/` global keybind, an inline
results panel, match counter, ranking by title / slug / id prefix →
mtime, and Enter-to-navigate behaviour. Surfaces on the sidebar; depends
on `useSearch` / `buildCorpus` / `rankCorpus` and a `useDebouncedValue`
hook. Users need a fast keyboard-driven entry point that does not
require knowing which library group a document lives under.

Implement HexChain to render a horizontal chain of tinted stage tiles
with connectors representing the eight-stage lifecycle pipeline.
Surfaces on the lifecycle index (one chain per cluster card) and the
lifecycle cluster detail header (sticky pipeline header); depends on
`StageTile`, `window.STAGES`, and the per-doc-type hue palette.

Implement BigGlyph to provide per-doc-type hero illustrations with a
seven-tone palette derived from a single hue. Surfaces on
`LibraryIndexEmpty`, the landing card hero, and any future hero slot;
depends on `bigPalette(hue)` and the doc-type hue map.

Implement Toaster to render a fixed bottom-right toast stack triggered
by `pushToast` from kanban drag confirmations, copy-link confirmations,
and external-edit-detected alerts. Surfaces on every route; depends on
a top-level toast queue in the app shell.

Implement TweaksPanel to expose a fixed bottom-left iframe-host
edit-mode panel for live design tweaks — theme, font mode, and any
future tunable token. Surfaces on every route when activated; depends
on app-level state for each tweakable variable.

Implement DevDesignSystem to consolidate component and token showcases
into a single hidden reference page (24 sections covering every
primitive — colours, type, spacing, radii, icons, glyphs, chips,
buttons, inputs, tables, markdown, code blocks, frontmatter, empty
states, toasts, topbar). Activated by `#dev` hash, Cmd/Ctrl+Shift+D
keybind, or a sidebar-foot triple-click; supersedes the existing
`/glyph-showcase` and `/chip-showcase` routes.

Implement KanbanDragAndDrop so kanban cards can be dragged between
columns, status mutation is written back to disk, and a toast confirms
the move. Surfaces on `/kanban`; depends on HTML5 drag-and-drop, a
status-mutation API on work items, and `Toaster`.

Implement LibraryLandingEmptyCard to render a dashed/striped per-type
empty card variant on the library landing for any doc type with zero
rows. Surfaces on `/library`; depends on the doc-type hue palette and
`TypeGlyph`.

Implement LibraryIndexEmpty to render a full-page empty state with a
`BigGlyph` hero and `TYPE_COPY` purpose copy for any per-type list with
zero rows. Surfaces on `/library/{type}` when empty; depends on
`BigGlyph`, `TYPE_COPY`, and the type metadata.

Implement TypeColourCoding so every doc type carries a stable hue
consumed by `TypeGlyph` (eyebrow 16, aside row 22, landing 34),
`StageTile`, `BigGlyph`, and the empty-page tint. Surfaces on every
route featuring documents; depends on the `--ac-doc-*` tokens being
made available to the detail surface.

Implement StatusBadge so both `status` and `verdict` frontmatter keys
map to coloured chip tones (verdicts `pass` / `fail`, statuses
`Accepted` / `Draft` / `Merged`). Surfaces on every detail page with a
verdict or status; depends on the existing `Chip` primitive and a
shared status-tone map.

Implement ExternalEditAlert to surface an inline "External edit
detected" notice in the aside region (or wherever the editing context
demands) with a dismiss button and cache-invalidation copy, fired when
the SSE indexer reports a disk change to the currently-rendered
document. Surfaces on detail pages and lifecycle cluster view; depends
on the SSE event stream and document-cache invalidation.

Implement FrontmatterTable to render every non-null frontmatter
key/value pair as a CSS-grid table above the markdown body, with
`WORK*` values auto-linkified. Surfaces on every detail page; depends
on the frontmatter parse output already produced by the detail-page
loader. We need this in addition to the existing chip strip, not as a
replacement, because the chip strip and the table communicate at
different fidelity levels.

Implement DetailHeaderActions to expose `Open in editor` and `Copy
link` buttons in the existing `Page.actions` slot on every detail
page, with `Copy link` writing the canonical document URL to the
clipboard and `Open in editor` invoking an editor deep-link when
configured. Surfaces on every detail page; depends on the existing
`Page.actions` prop slot.

Implement DocPageClusterAside as a dedicated `Cluster` block in the
detail-page aside that navigates to `/lifecycle/{slug}` with cluster
title and `<n> artifacts · <updated>` metadata. Surfaces on every
detail page where a matching lifecycle cluster exists; depends on the
lifecycle cluster lookup.

Implement SidebarTypeCounts so every per-type entry in the sidebar
LIBRARY tree displays the doc count for that type
(`Work items 14`, `Decisions 9`, `Templates 5`). Surfaces on the
sidebar; depends on the library index counts.

Implement AtomicMark to render an inline-SVG gradient hex brand mark
in the topbar, replacing the current `Brand` glyph. Surfaces on the
topbar; depends on the brand palette tokens being introduced in
§Token Drift.

Implement CodeBlockPalette by introducing the `--code-*` and `--tk-*`
token set so fenced code blocks and the templates preview share a
theme-independent palette across light and dark modes. Surfaces on
every detail page and the templates list; depends on the syntax
highlighter being able to consume CSS variables instead of fixed
class colours.

## Removed Features

The current app carries several capabilities that are not present in
the prototype. Some may be deliberate scope cuts; others may be
oversights. Each entry requests explicit stakeholder confirmation
before removal.

The current app's wiki-link routing rewrites `[[ADR-NNNN]]` and
`[[WORK-ITEM-NNNN]]` text-node matches in markdown to working
TanStack-Router `Link` elements (resolved), italic dim pending markers
(`.wiki-link-pending`), or dotted-underline unresolved markers
(`.unresolved-wiki-link`), backed by `useWikiLinkResolver` combining
the decisions cache, work-items cache, and `/api/work-item/config`
endpoint. The prototype renders `[[wiki-links]]` as decorative
`<a class="ac-md-wikilink" href="#">` with no navigation. We need to
confirm with stakeholders that working wiki-link routing is a feature
to retain — the prototype's decorative-only treatment is almost
certainly a scope cut for the prototype rather than an intentional
removal, and the current app's working implementation should remain.

The current app's `useDeferredFetchingHint(query, 250)` surfaces an
"Updating…" label on `RelatedArtifacts` only after a refetch has been
in flight for ≥ 250 ms, suppressing flash from unrelated `doc-changed`
invalidations; the prototype has no equivalent deferred-loading UX. We
need to confirm whether this UX should be preserved in the redesign or
replaced by a different loading-state pattern.

The current app's `RelatedArtifacts` produces a clear distinction
between *declared* relations (with `Targets` and `Referenced by`
sub-groups, 2px solid indigo left border, `.badgeDeclared`) and
*inferred* relations (with the `Same lifecycle` group, 2px dashed
faint left border, `.badgeInferred`), plus an inline `<dl>` legend
defining each kind. The prototype flattens this to a single `Related
artifacts` group plus an optional `Declared links` block, with no
`Referenced by` distinction and no legend. We need to confirm whether
the bidirectional declared / referenced split is meaningful enough to
keep or whether a single declared group is sufficient.

The current app exposes per-template detail at
`/library/templates/{name}` with three explicit tier panels
(plugin-default, user-override, config-override) showing active vs
absent states via border colour + accent-faint ring + `Chip
variant=indigo` / opacity 0.55 + `Chip variant=neutral`, plus a content
preview pane with truncated content-hash display
(`sha256-xxxxx…` with full hash exposed via `title` and
`data-full-sha`); the prototype's templates view exposes only the
list with inline tier pills and a code-style preview, no per-template
detail route. We need to confirm whether the per-template detail page
should be preserved (it carries content the prototype list cannot) or
whether the inline tier-pill model from the prototype is enough.

The current app exposes a `FontModeToggle` in the topbar driving the
`[data-font="mono"]` attribute that collapses display + body fonts to
Fira Code; the prototype's stylesheet still supports the attribute but
the topbar does not expose the toggle. We need to confirm whether the
font-mode toggle should remain in the topbar, move into the proposed
`TweaksPanel`, or be removed entirely.

The current app exposes `/glyph-showcase` and `/chip-showcase` as
separate uncrumbed dev routes; the prototype consolidates everything
into `#dev` (`DevDesignSystem`). We need to confirm whether the
existing showcase routes should be removed in favour of the
consolidated `DevDesignSystem` once that implementation lands, rather
than co-existing.

The current app's first-class doc types include `pr-descriptions` and
`pr-reviews` (declared in `LIBRARY_INDEX`, given sidebar entries under
SHIP, but with 0 documents in the captured run); the prototype omits
`pr-reviews` from the navigated route table (`pr-descriptions` is
present, `pr-reviews` is not). We need to confirm with stakeholders
whether `pr-reviews` is a first-class doc type the redesign must
preserve or whether the prototype's omission reflects an intentional
collapse into `pr-descriptions`.

## Suggested Sequencing

We need to land the token and primitive layers first so every downstream
component can consume them, then rebuild the detail page (the primary focus
area of both inventories), then the surrounding shell. Concretely: introduce
the `--atomic-*` brand palette and the `--code-*` / `--tk-*` syntax-highlight
palette as a new layered stylesheet; resolve the size-scale consumption
question; surface the per-doc-type `--ac-doc-*` tokens to the detail page.
Then ship the cross-cutting primitives that almost every screen will
depend on — `TypeGlyph` consumed at multiple sizes, `BigGlyph`,
`StatusBadge`, `Toaster`, `AtomicMark`. With those in place, rework
the detail page: add `FrontmatterTable`, wire `DetailHeaderActions`
into `Page.actions`, add the `Cluster` aside block, harmonise the
chip-strip cap to four chips max, humanise every H1, settle on a
single markdown-body cap and eyebrow typography. Then move outward to
the shell: redesign the library landing onto the per-type-card model
with `LibraryLandingEmptyCard`, redesign the per-type list empty
state with `LibraryIndexEmpty`, redesign the lifecycle index and
cluster onto `HexChain`, rationalise the kanban column model and
land drag-and-drop with toasts, add the omnibar with `/` keybind,
add per-type sidebar counts, and consolidate the dev showcases into
`DevDesignSystem`. Removed-feature confirmations from §Removed
Features should run in parallel as decision items so that work on
the detail page does not strip wiki-link routing,
`useDeferredFetchingHint`, or the declared/inferred relation split
by accident.

## References

Anyone re-running this analysis after either inventory is re-captured will
need to re-resolve the source-ids and re-read the inventories; we need to
keep both inventory references current here as the source of truth.

- Current inventory: `/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/build-system/meta/research/design-inventories/2026-05-21-004250-current-app/inventory.md`
- Target inventory: `/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/build-system/meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/inventory.md`
- Prior gap (superseded): `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Resolver notes: both source-ids resolved cleanly to the highest-sequence non-superseded inventory directory (`sequence: 2` in both); no multi-match or corrupt-frontmatter warnings.
