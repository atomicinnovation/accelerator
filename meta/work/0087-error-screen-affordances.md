---
id: "0087"
title: "404 / Error Screen with Affordances"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
kind: story
status: done
priority: low
tags: [design, frontend, error-states]
type: work-item
schema_version: 1
last_updated: "2026-06-12T16:46:31+00:00"
last_updated_by: Toby Clemson
blocked_by: ["work-item:0041", "work-item:0082", "work-item:0074"]
source: "design-gap:2026-05-21-current-app-vs-claude-design-prototype"
relates_to: ["work-item:0054"]
external_id: PP-109
---

# 0087: 404 / Error Screen with Affordances

**Kind**: Story
**Status**: Ready
**Priority**: Low
**Author**: Toby Clemson

## Summary

As a reader who follows a stale or mistyped link, I want the not-found
screen to help me recover — a way back to the library and the parent
type's list, plus suggestions for documents I might have meant — so that
a dead URL becomes a navigable dead-end rather than a wall. Replace the
bare inline `Document not found` *rendering* with a proper 404 surface
(which keeps `Document not found` as its heading for the genuine
missing-document case), and give truly-unmatched routes a not-found
surface they currently lack.

## Context

Today the app has **no dedicated 404 surface**. "Document not found" is
rendered **inline inside `LibraryDocView.tsx`** (the `/library/$type/$fileSlug`
detail view), not as a separate route or component, and there is **no
router-level `notFoundComponent`** in `router.ts` — so a truly-unmatched
URL (e.g. `/garbage`) falls back to the framework default rather than a
designed surface.

The inline message fires on **three different branches**, only one of
which is genuinely a 404:

1. **Unknown slug under a valid type** — `entries` loaded but no entry
   matches `fileSlug`. This is the true "document not found" case.
2. **Doc-list fetch errored** — a network/server failure.
3. **Doc-content fetch errored** — a network/server failure.

Branches 2 and 3 are not 404s, yet they show identical "Document not
found" copy. An unknown doc **type** never reaches this code at all:
`router.ts` redirects unknown types to `/library` in `parseParams`, so
type-inference for the back-to-type affordance only matters when the
type segment is valid but the slug is not.

The Claude design prototype has **no 404 screen** (unknown hashes were
silently coerced to the last-visited / Library route, and unknown doc
ids fell through to a real document), so it offers no direct design to
copy. It does, however, own the building blocks this surface should
reuse for visual and tonal consistency: the `.ac-page` / `.ac-pagehead`
shell, the `.ac-empty-page` hero-illustration layout (with a
`DEFAULT_BIG` fallback glyph for unknown types), the `.ac-search__empty`
"No matches" microcopy pattern, the `.ac-search__row` result-row markup
for suggestion links, `.ac-topbar__btn` for CTA links, and a calm,
no-apology, sentence-case copy voice that quotes the failed query in a
monospace span.

## Requirements

Terminology: the **not-found surface** is the shared component; it renders
in two not-found contexts (the unknown-slug **404 surface** and the
router-level catch-all), and a sibling **error state** handles fetch
failures. "The surface" below means the shared not-found surface unless a
specific context is named.

- Introduce a reusable not-found surface, rendered through the `Page`
  shell (work item 0041), used in two places:
  - The true-404 branch of `LibraryDocView.tsx` (unknown slug under a
    valid type).
  - A new router-level `notFoundComponent` in `router.ts`, so
    truly-unmatched URLs land on the same surface instead of the
    framework default.
- The not-found surface must contain:
  - A `Back to library` link to `/library` (always present).
  - A `Back to {type} list` link to `/library/{type}` — rendered **only**
    when the URL's first `/library/` segment passes `isDocTypeKey`
    (i.e. the type is known but the slug is not). Omitted on the
    router-level catch-all where no valid type is present.
  - A `Did you mean…` block listing **up to five** nearby-slug
    suggestions as links, derived from the known slugs (see below).
    Omitted entirely when there are no suggestions (not rendered empty).
- Nearby-slug suggestions:
  - Source the candidate slugs **client-side** by aggregating
    `IndexEntry.slug` across all `DOC_TYPE_KEYS` (there is no single
    global slug endpoint), following the precedent in
    `use-wiki-link-resolver.ts` / `wiki-links.ts`.
  - Generate suggestions only when the missing slug is **at least two
    characters** (after any normalisation), matching the existing search
    minimum (`use-search.ts` gates on `length >= 2`). Below that, render
    no suggestions — this is the case that exercises the empty-block
    omission below.
  - Rank using the same bucketing the server's `/api/search` `classify()`
    applies, restricted to slug matching: a **prefix match** (candidate
    slug `starts_with` the missing slug) ranks above an **interior
    substring match** (candidate slug `contains` the missing slug), both
    case-insensitive. An exact-slug match cannot occur on a 404 surface —
    an exact slug match is a *found* document, not a missing one.
    Levenshtein / fuzzy matching is **out of scope** for this story.
  - The match-quality bucket is the **primary** sort key. Within a bucket,
    break ties by **mtime, most-recent first** (then a stable key such as
    `rel_path`), mirroring `classify()`'s
    `sort_by_cached_key((Reverse(mtime_ms), rel_path))`. mtime never
    promotes a lower-quality bucket above a higher-quality one.
  - Each suggestion is a link to that document's detail route.
- **Fetch-error states are distinct from 404.** The doc-list and
  doc-content fetch-error branches must render an error state that does
  **not** include the `Did you mean…` suggestions block (a network/server
  failure is not a missing document). The error state:
  - Keeps the always-present `Back to library` link, and the
    `Back to {type} list` link when the type segment is valid — the same
    affordance rules as the 404 surface.
  - Carries a heading **distinct from the 404 H1** — not the string
    `Document not found` — that names a load/fetch failure (e.g.
    `Couldn't load this document`), with body copy referring to the
    failure rather than a missing document. No retry control is in scope
    for this story.
- **Headings.** For the unknown-slug 404 case, keep the existing H1
  `Document not found`. For the router-level catch-all (a truly-unmatched
  URL with no document slug), use `Page not found` — there is no missing
  *document* to name. Both retain the `Page` wrapper chrome.
- Match the prototype's copy voice: sentence case, terminal period, the
  missing slug quoted in a monospace span **when one is present** (the
  unknown-slug case; the catch-all has no slug to quote), concrete
  recovery hints, no apologies.

## Acceptance Criteria

- [ ] Given a URL for a non-existent document slug under a **valid** type,
  when the view renders, then the request routes to the new 404 surface
  (not the old inline `Document not found` text) with the `Document not
  found` H1; the individual affordances are asserted in the criteria below.
- [ ] Given any not-found state, when it renders, then a `Back to library`
  link to `/library` is present and navigates there.
- [ ] Given a not-found URL whose first `/library/` segment is a known
  doc type, when the surface renders, then a `Back to {type} list` link
  to `/library/{type}` is present; given an unknown/absent type segment
  (router-level catch-all), then that link is **not** rendered.
- [ ] Given at least one nearby-slug suggestion, when the surface renders,
  then the `Did you mean…` block lists up to five suggestions as links,
  ordered by match-quality bucket (prefix before interior substring) with
  mtime (most-recent first) as the within-bucket tie-breaker. **Worked
  example:** for missing slug `error-screen` and candidate slugs
  `error-screen-v2` (mtime T₂, newer), `error-screens` (mtime T₁, older),
  `legacy-error-screen` (interior match), and `error-handling` (no match),
  the block renders, in order: `error-screen-v2`, `error-screens`,
  `legacy-error-screen` — and omits `error-handling`. (The two prefix
  matches outrank the interior match regardless of mtime; mtime only
  orders the two prefix matches relative to each other.) Where two
  candidates share the same bucket **and** the same mtime, they are
  ordered by the stable `rel_path` key (ascending), so the ordering is
  fully deterministic.
- [ ] Given **more than five** matching candidate slugs, when the surface
  renders, then the `Did you mean…` block lists exactly the top five in
  ranked order and omits the sixth and beyond.
- [ ] Given a **mixed-case** missing slug (e.g. `Error-Screen`) and a
  lowercase candidate slug it matches case-insensitively (e.g.
  `error-screen-v2`), when the surface renders, then that candidate is
  still listed (matching is case-insensitive on both prefix and interior
  branches).
- [ ] Given no nearby-slug suggestions — because no candidate slug matches
  or because the missing slug is shorter than two characters — when the
  surface renders, then the `Did you mean…` block is omitted entirely (not
  rendered as an empty block).
- [ ] Given a truly-unmatched URL (no matching route), when the app
  renders, then the router's `notFoundComponent` shows the not-found
  surface with the H1 `Page not found` (back-to-library present;
  back-to-type omitted).
- [ ] Given a **doc-list or doc-content fetch error** (not a missing
  document) — i.e. the doc-list query fails (e.g. a 5xx or network
  failure) or the doc-content query rejects — when the view renders, then
  an error state is shown **without** the `Did you mean…` suggestions
  block; its heading differs from the 404 H1 (it is **not** the string
  `Document not found`) and names a load/fetch failure, and the `Back to
  library` link is present.
- [ ] Given the unknown-slug 404 surface renders, then the failed slug
  appears inside a monospace element, and the body copy is sentence-case
  with a terminal period. (The "no apologies" voice rule in Requirements is
  a design-review note, not a mechanical check.)

## Open Questions

- (Resolved) Substring vs Levenshtein — resolved to substring +
  prefix-boost for this story; Levenshtein deferred as future work.
- (Resolved) mtime in ranking — resolved to mtime as tie-breaker only.

## Dependencies

- Blocked by: 0041 (Page wrapper).
- Blocked by (satisfied — both `done`): 0082 (BigGlyph Hero Illustration
  Set) and 0074 (Per-Doc-Type Hues on Detail Page). The hero glyph and
  per-doc-type tint this surface reuses are the real shipped components
  from those stories, not prototype-only mockups; they were net-new when
  the source design-gap was written but have since landed. They are
  retained in `blocked_by` as real dependency edges; because both are
  `done`, neither is an *open* blocker — only 0041 remains outstanding.
- Behavioural coupling (not blocking): the suggestion ranking must stay
  consistent with the server's `classify()` bucketing (see Technical
  Notes) — if that convention changes, this surface should follow.
  Suggestion generation also assumes per-type `IndexEntry` data is warm in
  the TanStack Query cache at render time (see Assumptions); if it is not,
  it may trigger up to one `fetchDocs(type)` per `DOC_TYPE_KEYS` entry.
- Related: 0054 (sidebar search infrastructure — `use-search.ts`,
  `SearchResultsPanel.tsx`, `NoResultsPanel.tsx` — provides a no-results
  affordance precedent and substring-search machinery, but the
  suggestion engine here is client-side over cached index entries and
  does not depend on it).
- Blocks: none.

## Assumptions

- Slugs are reachable **client-side per type** via `fetchDocs(type)`
  (`IndexEntry.slug`), but there is **no single global slug endpoint** —
  suggestion generation must aggregate across `DOC_TYPE_KEYS`
  client-side. (Corrects the earlier assumption of a single exposed
  slug list.)
- The index entries needed for suggestions are already (or can be
  cheaply) cached client-side via the existing TanStack Query fetches,
  so generating suggestions does not require a new endpoint.

## Technical Notes

- **Integration points:**
  - `frontend/src/routes/library/LibraryDocView.tsx` (lines ~95–115) —
    the inline not-found / fetch-error branches to be split and replaced
    with the new surface.
  - `frontend/src/router.ts` — add a `notFoundComponent` on
    `createRouter` for truly-unmatched URLs.
- **Page shell:** `frontend/src/components/Page/Page.tsx` (the 0041
  dependency).
- **Slug source / suggestions:** aggregate `IndexEntry.slug` across
  `DOC_TYPE_KEYS` via `fetchDocs(type)` (`frontend/src/api/fetch.ts`,
  `frontend/src/api/types.ts`); precedent for a client-side slug/id
  index in `frontend/src/api/use-wiki-link-resolver.ts` and
  `frontend/src/api/wiki-links.ts` (note: those do **exact** resolution
  only — fuzzy ranking is net-new here).
- **Type inference:** validate the first `/library/` path segment with
  `isDocTypeKey` / `DOC_TYPE_KEYS` (`frontend/src/api/types.ts`).
- **Navigation targets:** back-to-library → `/library`
  (`LibraryOverviewHub`); back-to-type → `/library/$type`
  (`LibraryTypeView`).
- **Matcher to echo:** `classify()` in `server/src/api/search.rs` is the
  authoritative ranking convention — discrete buckets (exact-slug → prefix
  → interior substring → body), then
  `sort_by_cached_key((Reverse(mtime_ms), rel_path))` within each bucket.
  The suggestion engine reuses the slug-relevant buckets (prefix,
  interior) and the same mtime-descending tie-break. The prototype's
  `rankCorpus` is an illustrative equivalent, not a second source of
  truth. No Levenshtein/fuzzy utility exists anywhere in the codebase.
- **Hero glyph / type tint:** consume the shipped components, not the
  prototype mockups — the BigGlyph hero illustration (0082, with its
  default fallback for an unknown/absent type) and the per-doc-type hue
  tokens surfaced on the detail page (0074). The catch-all (no type)
  falls back to the default glyph and carries no per-type tint.
- **Copy / visual conventions (prototype inventory for layout/voice):**
  follow the `.ac-empty-page` hero+illustration layout, the
  `.ac-search__empty` microcopy pattern, the `.ac-search__row` markup for
  suggestion links, and `.ac-topbar__btn` for the back-link CTAs;
  sentence-case, no-apology voice with the missing slug in a monospace
  span.

## Drafting Notes

- Corrected the original Context/Technical Notes: there is no discrete
  `error-not-found` route/component — "Document not found" is rendered
  inline in `LibraryDocView.tsx`, and there is no router
  `notFoundComponent`. If "error-not-found" was a logical name from the
  source design-gap doc, the work here is to *create* the surface, not
  modify an existing component.
- Scoped the router-level `notFoundComponent` into the story (per the
  author) so truly-unmatched URLs are covered, not only unknown slugs.
- Baked in the fetch-error / 404 split (per the author): treating the
  three current branches identically is a latent correctness issue, so
  the two fetch-error branches now get a distinct error state without
  suggestions.
- Resolved both prior open questions toward the lower-complexity option
  (substring matching; mtime as tie-breaker), consistent with the
  codebase having no fuzzy-matching utility and this being a low-priority
  screen.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Prototype inventory: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full`
  (notable: `src/search.jsx`, `src/view-empty.jsx`, `src/app-shell.jsx`,
  `src/big-glyphs.jsx`, `src/app.css`)
- Current code: `frontend/src/routes/library/LibraryDocView.tsx`,
  `frontend/src/router.ts`, `frontend/src/components/Page/Page.tsx`,
  `frontend/src/api/types.ts`, `frontend/src/api/fetch.ts`,
  `frontend/src/api/wiki-links.ts`, `server/src/api/search.rs`
- Related: 0041, 0054
