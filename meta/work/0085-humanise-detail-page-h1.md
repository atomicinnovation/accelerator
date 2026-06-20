---
id: "0085"
title: "Humanise Detail-Page H1 Across All Doc Kinds"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
kind: story
status: done
priority: low
tags: [backend, detail-page, indexer]
type: work-item
schema_version: 1
last_updated: "2026-05-21T09:16:34+00:00"
last_updated_by: Toby Clemson
blocked_by: ["work-item:0065", "work-item:0066", "work-item:0070"]
source: "design-gap:2026-05-21-current-app-vs-claude-design-prototype"
relates_to: ["work-item:0078"]
external_id: PP-107
---

# 0085: Humanise Detail-Page H1 Across All Doc Kinds

**Kind**: Story
**Status**: Ready
**Priority**: Low
**Author**: Toby Clemson

## Summary

Replace the raw `filename_stem` fallback in the server-side title cascade
with a humanised slug, so no detail page ever renders an unhumanised slug
as the page's `<h1>` heading. Post unified-schema migration every doc kind carries
`frontmatter.title`, so this fallback becomes the defensive belt-and-braces
layer rather than the primary derivation path.

## Context

Today the title shown as H1 on a detail page is computed server-side by
`title_from` in `frontmatter.rs` and shipped to the frontend as
`entry.title`. The shared `LibraryDocView` component renders it verbatim
via the `Page` wrapper introduced by 0041 — there is no per-route
humanisation. The current cascade is: `frontmatter.title` → first H1 →
raw `filename_stem`. Three doc kinds (work-item-reviews, plan-reviews,
validations) currently lack both `frontmatter.title` and a first H1, so
they fall through to the unhumanised stem (e.g.
`0042-templates-view-redesign-review-1`). Only those three kinds break
today, but the humanised fallback and its test cover all thirteen kinds
in `DocTypeKey::all()`, so any future kind that ships without a title or
first H1 is protected without further change.

The unified frontmatter schema work (epic 0057) makes `title` a base
field on every artifact type. Once the producer (0065), inline-generator
(0066), and corpus-migration (0070) stories ship, every document —
including reviews and validations — will have `frontmatter.title`
populated and the primary cascade path will resolve. The job of this
work item is to ensure the residual fallback path is also humanised,
so that any document that slips through (hand-authored, legacy,
malformed) still renders a readable H1.

## Requirements

- Replace the raw `filename_stem` fallback in `title_from`
  (`skills/visualisation/visualise/server/src/frontmatter.rs`) with a
  humanised form of the slug.
- Introduce a `humanise_slug` helper next to the existing
  `humanise_status` helper in `api/library.rs`. Strip at most one
  leading prefix matching a numeric ID (e.g. `0042-`) or ISO date (e.g.
  `2026-05-21-`), then split the remainder on hyphens and title-case
  each segment. "Title-case" means: uppercase the first character of
  each segment and leave the remaining characters untouched; segments
  led by a digit (e.g. `0042`, `21`, a trailing `1`) are emitted
  verbatim. Hyphens inside a *residual* date or ID (one that was not the
  leading prefix) are treated as ordinary segment separators.
  Extraction to a dedicated `humanise.rs` module is deferred until a
  third humaniser appears.
- Apply the new cascade uniformly: the change is a single function edit
  in the indexer's title resolution; the shared `LibraryDocView` and
  `Page` consume `entry.title` verbatim and require no edits.
- Cover every doc kind in `DocTypeKey::all()` (thirteen variants:
  decisions, work-items, plans, research, plan-reviews, pr-reviews,
  work-item-reviews, validations, notes, pr-descriptions, design-gaps,
  design-inventories, templates).

## Acceptance Criteria

- [ ] A unit test iterates over every variant of `DocTypeKey::all()`
  (thirteen variants), feeds a fixed test stem (e.g.
  `"0042-test-fixture"`) with **no** `frontmatter.title` and **no**
  first H1 through `title_from`, and asserts the resolved title
  equals `humanise_slug(stem)` (and therefore differs from the raw
  stem). To guard against a shared bug in the test oracle (the
  `humanise_slug(stem)` call computing the expected value) and the
  production path, the test also asserts the resolved title against a
  concrete literal for the fixed stem — `"0042-test-fixture"` →
  `"Test Fixture"` — for at least one variant. This guarantees no
  detail-page route can render an unhumanised filename stem as the
  page's `<h1>` heading for any doc kind.
- [ ] `humanise_slug` is implemented as a discrete helper with unit
  tests covering, at minimum:
  - simple hyphen splits — `humanise_slug("design-token-system") == "Design Token System"`
  - leading numeric IDs stripped — `humanise_slug("0042-templates-view-redesign-review-1") == "Templates View Redesign Review 1"`
  - leading ISO dates stripped — `humanise_slug("2026-05-21-current-app-vs-claude-design-prototype") == "Current App Vs Claude Design Prototype"`
  - mixed prefixes — single-pass strip: only the leading matching prefix is removed, e.g. `humanise_slug("2026-05-21-0042-foo") == "0042 Foo"` and `humanise_slug("0042-2026-05-21-foo") == "2026 05 21 Foo"`. Once the leading prefix is stripped, hyphens inside any *residual* date or ID become ordinary separators — which is why `0042` survives as a single token in the first example while the residual date splits into `2026 05 21` in the second.
  - single-segment slugs — `humanise_slug("notes") == "Notes"`
  - degenerate inputs (the malformed-document case the fallback defends) — a prefix-only stem yields the stripped prefix's tokens, e.g. `humanise_slug("2026-05-21") == "2026 05 21"`; an empty stem yields an empty string (`humanise_slug("") == ""`)
- [ ] `title_from` has fixture-driven cascade tests verifying:
  - (a) `frontmatter.title` present → returned verbatim
  - (b) `frontmatter.title` absent, first H1 present → first H1
    returned
  - (c) both absent → `humanise_slug(stem)` returned
- [ ] The change is server-side only: the PR diff touches no frontend
  files, and in particular no modifications appear to `LibraryDocView.tsx`
  or `Page.tsx`. (Verifiable against the actual diff by a reviewer or a
  CI path guard, rather than asserted as an unobservable "nothing
  changed".)
- [ ] The cascade order is documented inline in `title_from` with a
  one-line comment per layer; each comment names its source
  (`frontmatter.title`, `first H1`, `humanise_slug(stem)`) and its
  position in the cascade.

## Open Questions

- None remaining. The prefix-handling question (preserve vs strip
  leading numeric IDs and ISO dates) was resolved during review to
  strip — see Requirements and AC2 for the worked examples.

## Dependencies

- Blocked by: 0065 (templates emit unified schema), 0066 (review skills'
  inline generators emit unified schema), 0070 (corpus migration applied
  to existing `meta/` documents). Without these, reviews and validations
  would still hit the humanised-slug fallback as their primary path,
  which would force this work item to grow a kind-aware synthesis layer.
- Builds on: 0041 (Page wrapper standardised across detail routes);
  0060 (ADR establishing `title` as a base frontmatter field on every
  artifact type — the invariant this story's primary cascade path
  relies on, leaving the humanised slug as a defensive fallback).
- Related: 0078 (frontmatter table — shares parsed frontmatter source),
  0074 (per-doc-type hues — same header surface), 0080 (header actions —
  same header surface), 0084 (chip strip cap — same header surface),
  0057 (unified frontmatter epic — parent of the gating dependencies).
- Blocks: none.
- Ordering: can land independently of 0074/0084 — those touch the
  header markup and styling, while this story changes only the
  server-derived value of `entry.title`. No merge ordering required.

## Assumptions

- Once 0065, 0066, and 0070 ship, `frontmatter.title` is reliably
  populated on every doc kind in the indexed corpus. The humanised-slug
  fallback exists for edge cases (hand-authored or legacy documents
  that bypass templates/migrations) rather than as a primary derivation
  path.
- `IndexEntry.title` remains a plain `String` on the wire; no schema
  change to the API DTO is required.

## Technical Notes

- Fix surface: one function. `title_from` in
  `skills/visualisation/visualise/server/src/frontmatter.rs` (around
  line 278). Current cascade returns `filename_stem` unhumanised; the
  change swaps that final layer for `humanise_slug(filename_stem)`.
- Indexer integration: `indexer.rs` (lines 1018–1047) derives
  `title_fallback_stem` and calls `title_from`. No structural change
  expected — just pass the same stem through the new humaniser.
- Helper placement: `humanise_slug` lives next to `humanise_status(id)`
  in `api/library.rs:236`. Extraction to a `humanise.rs` module is
  deferred until a third humaniser appears.
- Tests: extend existing `title_cascade_*` tests in `frontmatter.rs`
  (around lines 421–440) with fixtures that have no `frontmatter.title`
  and no first H1.
- Frontend: no changes. `LibraryDocView.tsx:94` reads `entry.title`;
  `Page.tsx:31` renders it as `<h1>`.

## Drafting Notes

- Scope-collapsed during enrichment. Original draft assumed per-route
  frontend work and required synthesising review/validation titles
  from `target` + `review_number` since those documents had no
  `frontmatter.title`. Discovering that the title cascade is a single
  server-side function, and that 0065/0066/0070 will populate
  `frontmatter.title` everywhere, reduced the work item to a humanised
  fallback helper plus a one-line cascade-layer swap.
- Made this work item dependent on 0065, 0066, and 0070 rather than
  carrying a kind-aware synthesis layer. The alternative — landing
  this story first with a synthesis layer that later becomes dead code
  once `frontmatter.title` is populated — would have introduced tech
  debt to no user-visible benefit.
- Corrected the doc-kind count from twelve to thirteen
  (`DocTypeKey::all()` enumerates: decisions, work-items, plans,
  research, plan-reviews, pr-reviews, work-item-reviews, validations,
  notes, pr-descriptions, design-gaps, design-inventories, templates).
- Added the `backend` tag because the fix is in Rust server code, not
  the frontend as the original write-up implied.
- Renamed frontmatter `type:` → `kind:` and body `**Type**:` →
  `**Kind**:` to align with the current work-item template (the field
  was canonicalised across the project by 0063, which this file
  predates).
- Kept priority at `low` per the original. If you want this landed in
  the same wave as the in-progress sibling header work (0084, 0074),
  consider bumping to `medium`.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related:
  - 0041 — Page wrapper standardised across detail routes (builds-on, see Dependencies)
  - 0057 — Unified frontmatter epic; this story is a defensive
    follow-on rather than a child (see Dependencies)
  - 0060 — ADR for the unified base frontmatter schema; establishes
    `title` as a base field on every artifact type, which is the
    invariant this work item's primary cascade path relies on
  - 0063 — Frontmatter field canonicalisation (`type:` → `kind:`)
    that this file post-dates; see Drafting Notes
  - 0065, 0066, 0070 — Gating dependencies (see Dependencies)
  - 0074, 0078, 0080, 0084 — Related work touching the same header
    surface or parsed frontmatter source (see Dependencies)
