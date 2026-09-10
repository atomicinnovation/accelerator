---
type: "design-gap"
id: "2026-09-10-current-app-vs-claude-design-prototype"
title: "Design Gap Analysis: current-app → claude-design-prototype"
date: "2026-09-10T18:11:50+00:00"
author: "Toby Clemson"
producer: "analyse-design-gaps"
status: "draft"
current_inventory: "/Users/tobyclemson/Code/organisations/atomic/company/accelerator/meta/research/design-inventories/2026-09-10-184815-current-app/inventory.md"
target_inventory: "/Users/tobyclemson/Code/organisations/atomic/company/accelerator/meta/research/design-inventories/2026-09-10-174311-claude-design-prototype/inventory.md"
tags: ["design", "gap-analysis"]
last_updated: "2026-09-10T18:11:50+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Design Gap Analysis: current-app → claude-design-prototype

## Overview

This gap compares the shipped Accelerator Visualiser (current-app, sequence 3,
revision `50923ba2`, runtime crawl of the history-routed React SPA at
`http://127.0.0.1:64264`) against the claude-design-prototype (sequence 3,
revision `60e52207`, runtime crawl of the hash-routed static shell at
`http://localhost:8000/Accelerator%20Visualiser.html`). Both inventories were
captured on 2026-09-10 with the `runtime` crawler and the same executor family,
so token and DOM observations are directly comparable. The headline is
convergence: the semantic `--ac-*` colour layer is byte-for-byte identical in
both light and dark, so the residual drift is structural — token naming, chrome
layout, component shape, and a handful of net-new features — rather than colour.

Coverage is asymmetric, and that shapes what can be asserted. The current-app
crawl could not exercise interactions (the executor build omits `click`, `type`,
and `wait_for` — repo bug 0287), so its sort/filter option sets, kanban drag,
theme toggle, and the hidden design-system reference were never actuated, and no
empty, loading, or error state surfaced (the local server was always populated).
The prototype crawl reached both themes and the empty/fallback states but never
fired the SSE external-edit toast and does not bundle Fira Code. Where an
affordance exists in the current app but was not exercised, this analysis states
the target's specification and marks the current side unconfirmed rather than
claiming an absence. We need to treat each prose paragraph below as a candidate
driver for a discrete work item rather than collapsing related drifts into one
epic.

## Token Drift

The colour system has converged; the drift that remains is in how the scale
tokens are named, in the token-layer architecture, and in the per-doc-kind
accent values. Both stylesheets expose identical `--atomic-*` primitives,
identical `--sp-*` spacing (4–124px), identical shadow tokens, and an identical
`--ac-*` semantic layer across both themes. The work below is therefore refactor
and reconciliation, not a colour re-theme.

The size scale is named on incompatible conventions. The current app names sizes
numerically by pixel value (`--size-95` … `--size-680`, i.e. 9.5px through 68px,
with half-pixel steps at 9.5, 10.5, 11.5, 12.5, and 14.5), whereas the target
names them by role (`--size-hero` 68, `--size-h1` 48, `--size-h2` 36,
`--size-h3` 28, `--size-h4` 26, `--size-lg` 22, `--size-body` 20, `--size-md`
18, `--size-sm` 16, `--size-xs` 14, `--size-xxs` 12). We need to migrate the
size scale to the target's role-named tokens and decide the fate of the current
app's finer sub-14px steps, which the target's scale does not carry.

The radius scale drifts the same way. The current app uses a numeric ladder
(`--radius-0` … `--radius-12` mapping to 0, 1, 2, 3, 4, 6, 8, 12px) plus
`--radius-full` (50%) and `--radius-pill` (999px), while the target collapses
this to `--radius-sm` (4), `--radius-md` (8), `--radius-lg` (12), and
`--radius-pill` (999). We need to rename the radius tokens to the semantic set
and confirm whether `--radius-full` (50%) still has callers before dropping it.

The target ships a two-layer token architecture that the current inventory does
not record. In the prototype, `tokens.css` holds theme-independent primitives
plus light role-aliases (`--fg-1..3`, `--bg-1/2`, `--accent`, `--accent-2`,
`--stroke`) and `app.css` holds the semantic `--ac-*` layer, which is the only
layer that flips by theme. The system must adopt this split so that theming
touches only the `--ac-*` layer, and we need to introduce the light role-aliases
the current app is missing.

The per-doc-kind accent palette drifted in value and gained an entry. The
thirteen current kind accents differ from the target's fourteen — for example
work-items moves from `#af4b2f` to `rgb(188,66,36)` and design-inventories from
`#2e7e8a` to `rgb(36,176,188)` — and the target splits research into a Topic
research hue (`rgb(28,146,51)`) and a Codebase research hue (`rgb(188,107,36)`).
We need to re-derive the per-doc-kind glyph accents and their pale tints to the
target's values. The eight lifecycle stage hues sit within a few RGB units of
their current counterparts and can be treated as aligned.

## Component Drift

Every list, board, and chrome component changed shape even though the tokens
they consume converged. These are structural changes — variant counts, markup,
and composition — not restyles, so each is a discrete rebuild.

The chrome splits into a top bar and a slimmer sidebar. The current app has no
top bar: brand lockup, host/SSE indicator, theme and mono toggles, and the
search box all live in the 256px left rail, with a breadcrumb rendered
per-screen. The target introduces a 48px top bar (`.ac-topbar`) carrying the
brand, the breadcrumb trail, the SSE indicator, and the theme/font toggles,
leaving the sidebar for navigation, the activity feed, and the version footer.
We need a top-bar chrome region and to relocate the breadcrumb and the toggles
into it.

The status badge becomes a tinted chip. The current badge is a text-only span
with five variants (`draft` muted, `ready` and `in-progress` accent indigo,
`done` green, `abandoned` red) and no pill background, whereas the target uses
`.ac-chip` pills in three tinted variants (neutral for Draft/Todo, green for
Accepted/Done, indigo for Proposed/In progress). We need to reconcile the status
vocabulary and re-implement the badge as a background-tinted chip.

The target adds a work-item kind badge the current app has no equivalent for.
Its `.ac-kindbadge` renders the work-item kind — STORY (indigo), BUG (red), TASK
(neutral), SPIKE (amber), EPIC (violet) — on kanban cards and collection rows.
We need a KindBadge component surfacing work-item kind, since the current app
conveys kind only through the per-doc-kind accent, not a discrete badge.

The collection list changes markup and drops its count line. The current app
renders the collection as a CSS-grid list (`role=row`, not an HTML table) with a
document-count line above it; the target renders a semantic `<table>`
(`.ac-libtable`) of clickable rows with no count line. We need to decide whether
to move to the semantic table and drop the count line, keeping the shared column
set (ID/Date, Title, Status, Slug, Modified) intact.

The sort and filter controls gain specified internals the current side could not
confirm. The target defines a five-option sort listbox (Recently modified,
Oldest first, Title A→Z, Title Z→A, ID ascending) and a multi-facet filter
popover (a STATUS checkbox group with facet counts plus a CLUSTER SLUG group
with its own search input over a scrollable list); the current inventory records
only unexercised sort and filter menu-buttons. Users need the filter popover's
facet counts and cluster-slug search, and we need to verify whether the current
app already provides them before building.

The kanban board and card are reshaped. The current board uses five status
columns (Draft, Ready, In progress, Done, Other) and a card that shows a type
glyph plus the text "Lifecycle pipeline, N of 7 stages complete"; the target
uses three neutral columns (Todo, In progress, Done, where status is conveyed by
column position) and a card carrying a kind badge, an "N linked" count, and a
seven-dot hexchain progress row. We need to collapse the column set and rebuild
the card around the kind badge and hexchain (see Removed Features for the
status-granularity question this raises).

The lifecycle stage chain is redrawn as hexagons. The current app renders the
chain as `ac-stagechain` bars of roughly 125px stages; the target renders
`.ac-hexchain` — a seven-stage hexagon chain with a `reached/total` count and a
class `on` on reached stages. The system must render the lifecycle progress as
the hexagon chain with reached/total counts in place of the bar chain.

The document viewer gains typed sections and richer relation affordances. The
target adds typed body sections (for a Decision: Context, Decision,
Consequences, Sketch with highlighted code, and Links), an `(inferred)` tag on
related-artifact cards, a collapsed cluster expander, and a "Copy link" action
where the current header offers "Copy path". We need to add typed body rendering
and the related-artifact and cluster affordances to the detail reader.

## Screen Drift

The routing model and the collection identity scheme diverge, and the target
designs two collection states the current crawl never surfaced. These are
screen-level changes to navigation and the state matrix rather than component
restyles.

The routing model differs at every route. The current app uses history-based
path routing served by the Rust server (`/library`, `/library/<kind>/<slug>`,
`/kanban`, `/lifecycle`), whereas the target is hash-routed from a single static
shell (`#/library`, `#/library/<slug>/<doc-id>`, `#/kanban`, `#/lifecycle`) and
its document route is keyed by `<slug>/<doc-id>` rather than `<kind>/<slug>`.
Hash routing is most likely an artefact of the prototype's single-file static
delivery rather than a design intent, so we need to confirm with stakeholders
whether the app should adopt hash routing before treating this as a target to
implement.

The collection slug and label scheme is inconsistent between the two. The target
labels a collection "Work items" but routes it at `#/library/work` (the current
app uses `/library/work-items`), and it splits the DISCOVER research area into a
`topic-research` collection and a `research` collection labelled "Codebase
research" where the current app carries a single `research` kind. We need to
reconcile the collection slugs and decide whether to adopt the label/slug split
for work items and the two-way research split.

The target designs empty and fallback collection states that the current app
never rendered in the crawl. It specifies a real empty page
(`collection-notes-empty`: a human-readable type label, a bespoke type blurb, a
mono path eyebrow, and the live-indexer hint) and a distinct unknown-slug
fallback (`collection-unknown-fallback`: a raw-slug h1 and a generic "Documents
of this type live here." blurb). The system must render designed empty and
unknown-slug states; the current app's behaviour here is unconfirmed because its
server returned populated collections throughout the crawl.

## Net-New Features

These capabilities are present in the target and absent from the current app's
feature catalogue. Each introduces a surface or channel the current app does not
have, so each is additive rather than a reshape.

The target promotes search from a sidebar box to a global overlay. The current
app exposes only an unexercised search box in the left rail and does not
catalogue search as a feature, whereas the target ships `.ac-search__panel` — a
relevance-ranked results listbox, a match-count meta bar, keyboard control
(`↵`/`esc`), and defined loading and empty states. We need to implement Search
as a global overlay with ranked results and keyboard control that surfaces on
every route.

The target adds an external-edit toast that the current app lacks. The current
app has the SSE stream, the rail connection indicator, and an activity feed, but
no toast component; the target mounts a `.ac-toaster` (fixed bottom-right) whose
SSE-driven toast announces an external edit ("A reviewer agent updated … Query
invalidated.") with a dismiss control. We need a Toaster wired to the SSE stream
to surface external edits, noting the toast never fired in either crawl and is
recorded from the stylesheet.

The target introduces Topic research as a distinct corpus kind. Where the
current app has a single `research` kind, the target carries both Topic research
(accent `rgb(28,146,51)`) and Codebase research within the DISCOVER group. We
need a Topic research collection kind with its own accent, route, nav entry, and
count, alongside the existing research kind.

## Removed Features

These capabilities are present in the current app and absent from the target's
inventory. Each may be an intentional scope cut in the prototype or a gap in the
prototype crawl, so each is flagged for confirmation before any removal.

The templates gallery and its tier indicators are not inventoried in the target.
The current app has a full templates-gallery screen (`/library/templates`) with
a Template-row component and default/user/config tier indicators showing which
configuration tier is active; the target keeps a "Templates" nav entry (count 6)
but records no gallery screen, template row, or tier indicators. We need to
confirm with stakeholders whether the templates gallery and its tier indicators
are intentionally dropped or merely uncrawled in the prototype before removing
them.

The hidden design-system reference is not inventoried in the target. The current
app has a design-system reference revealed by triple-clicking the version button
(inferred, and unreachable in this crawl because the executor lacks a click
command); the target does not catalogue it, though its `ds-marquee` keyframe and
a dedicated `tweaks` z-index layer hint that a comparable surface exists. We need
to confirm whether the hidden design-system reference survives in the target
before treating its absence as a removal.

The kanban status granularity is reduced. The current board distinguishes five
columns (Draft, Ready, In progress, Done, Other), while the target's three-column
board (Todo, In progress, Done) drops the Draft, Ready, and Other lanes. We need
to confirm that collapsing the status vocabulary to three lanes is intended and
not a loss of the Draft, Ready, and Other distinctions currently shown.

## Suggested Sequencing

The dependency order runs tokens → chrome → components → deferred routing, with
the removed-feature questions gating any deletion. We need to sequence the token
refactor first — the role-named size and radius scales, the two-layer split with
light aliases, and the re-derived per-doc-kind accents — because every reshaped
component inherits those values, and doing it first prevents re-touching each
component twice.

The chrome split into a top bar should follow, since relocating the breadcrumb,
toggles, and search reframes every screen and is a prerequisite for the search
overlay and the reshaped sidebar. Component rebuilds (the tinted chip, the kind
badge, the collection table, the sort/filter internals, the kanban board, the
lifecycle hexchain, and the typed document viewer) can then proceed in parallel
against the settled tokens and chrome. The routing-model change should stay
parked until the hash-versus-history question is answered, and the three
removed-feature items should be resolved with stakeholders before any code is
deleted.

## References

Anyone re-running this analysis after either inventory is re-captured will need
to re-resolve the source-ids and re-read the inventories; we need to keep both
inventory references current here as the source of truth.

- Current inventory: `/Users/tobyclemson/Code/organisations/atomic/company/accelerator/meta/research/design-inventories/2026-09-10-184815-current-app/inventory.md`
- Target inventory: `/Users/tobyclemson/Code/organisations/atomic/company/accelerator/meta/research/design-inventories/2026-09-10-174311-claude-design-prototype/inventory.md`
- Prior gap (superseded on re-capture): `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Resolver notes: both source-ids resolved cleanly to the highest-sequence non-superseded inventory (`sequence: 3` in each); sequences 1 and 2 per source-id are `superseded`. No multi-match or corrupt-frontmatter warnings were emitted.
