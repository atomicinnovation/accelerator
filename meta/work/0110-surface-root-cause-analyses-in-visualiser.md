---
type: work-item
id: "0110"
title: "Surface Root Cause Analyses in the Visualiser Under a New Operate Category"
date: "2026-06-13T09:31:54+00:00"
author: Toby Clemson
producer: create-work-item
status: done
kind: story
priority: high
relates_to: ["work-item:0041", "work-item:0074", "work-item:0054", "work-item:0057", "work-item:0082", "work-item:0093", "work-item:0096"]
tags: ["visualiser", "rca", "doc-types", "library"]
last_updated: "2026-06-13T09:31:54+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0110: Surface Root Cause Analyses in the Visualiser Under a New Operate Category

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

As a visualiser user, I want root cause analyses (RCAs) to be a first-class
document type — browsable under a new top-level "Operate" category, with their
own listing and detail pages — so that the issue-research artifacts produced by
`/research-issue` are discoverable and navigable alongside every other doc type.

The visualiser predates the RCA document type, so `issue-research` artifacts
(the RCA doc type — see Technical Notes for the naming triad) are currently
absent from the library, listing/detail navigation, related artifacts, and
search. This work surfaces the type end-to-end against the now-authoritative
design prototype.

## Context

The RCA (`issue-research`) doc type is produced by the `/research-issue` skill
and was added to the system after the visualiser was built. Its canonical type
discriminator and frontmatter shape are defined by the unified-artifact schema
(work item 0057); its producing skill and typed-linkage slots are mapped in
0093; the stem `rca` is used in registration maps.

The visualiser's library is **server-driven** (0041): the server returns the
category/phase structure (currently DEFINE / DISCOVER / BUILD / SHIP / REMEMBER)
plus each doc type's id, label, glyph, route, count, and "latest" preview, and
the frontend renders whatever it receives. Per-doc-type colour/glyph mapping
lives in the HSL map and related-artifacts render path (0074); hero
illustrations come from the BigGlyph set (0082); search flows through the
endpoint and sidebar UI (0054). Work item 0096 (functionally done, status not
yet updated) auto-discovers templates and added the `rca` glyph/`STEM_TO_GLYPH`
registration — this work builds on that rather than re-adding it.

The authoritative design for the RCA pages is the updated prototype at
`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full`,
which now includes the RCA doc type, the Operate category, and the listing and
detail page designs.

## Requirements

1. **New "Operate" category** — add a new top-level grouping to the library,
   a peer of the existing lifecycle groupings (DEFINE / DISCOVER / BUILD / SHIP
   / REMEMBER), emitted by the server as part of the phase/category structure,
   with the RCA doc type as its member. Per the prototype (`src/data.jsx`
   `LIBRARY_GROUPS`), Operate is positioned **between Ship and Remember** with
   key `operate` and label "Operate".
2. **RCA registered as a first-class doc type** — display labels "Root cause
   analyses" (plural) / "Root cause analysis" (singular), short label "RCA",
   route, glyph (`STEM_TO_GLYPH`), per-doc-type hue **310** (the HSL —
   hue/saturation/lightness — map from 0074), and a BigGlyph hero illustration.
   Reuse the registration 0096 already landed where it exists; add only what is
   missing. Consumed using the existing `issue-research` frontmatter schema
   (0057) as-is.
3. **RCA listing page** — a server-driven list view following the existing
   list-view pattern, rendering all `issue-research` documents, matching the
   prototype. Each row surfaces the document's `status` (e.g. `resolved`,
   `monitoring`) via the shared status-badge treatment, consistent with the
   prototype's listing rows.
4. **RCA detail page** — a detail view following the existing detail-page
   pattern with the RCA-specific hue (310), glyph, and BigGlyph hero applied,
   matching the prototype.
5. **Library overview hub updated** — the Operate category and a "Root cause
   analyses" card appear with a correct document count (= the number of
   `issue-research` artifacts in the repo) and a "latest" preview (the most
   recently modified RCA), with semantics identical to existing doc-type cards
   (0041).
6. **Cross-cutting integration** — RCAs appear wherever doc types are
   enumerated: related-artifacts rows (0074 render path) with the correct
   glyph/hue and status badge, and search results (0054 endpoint + sidebar UI)
   labelled and routed as RCAs.

## Acceptance Criteria

- [ ] Given the visualiser running against a repo containing `issue-research`
      artifacts, when I open the library page, then an "Operate" category appears
      as a top-level grouping (between Ship and Remember) containing a "Root cause
      analyses" card whose count equals the number of `issue-research` artifacts
      in the repo and whose latest preview shows the most recently modified RCA.
- [ ] Given a repo with **no** `issue-research` artifacts, when I open the
      library page, then the Operate/RCA card behaves identically to other
      zero-count doc-type cards (shown with count 0, per 0041 semantics).
- [ ] Given the Operate/RCA card on the library page, when I click it, then the
      RCA listing page renders all `issue-research` documents using the shared
      list-view layout, each row showing the document's `status` (e.g.
      `resolved`, `monitoring`) via the shared status-badge treatment.
- [ ] Given the RCA listing page, when I open a single RCA, then its detail page
      renders the document with the RCA-specific glyph, hue (310), and BigGlyph
      hero.
- [ ] Given an artifact whose frontmatter links to an RCA, when I view that
      artifact's detail page, then the RCA appears in related artifacts with the
      correct RCA glyph and hue (310) and routes to the RCA detail page.
- [ ] Given a search query matching an RCA's title or content, when I search via
      the sidebar, then matching RCAs appear in the results, labelled and routed
      as RCAs.
- [ ] Given the listing and detail pages, then each of the following discrete
      properties holds: the doc-type hue is HSL 310; the short label is "RCA" and
      the display labels are "Root cause analyses" (plural) / "Root cause
      analysis" (singular); the Operate card sits between Ship and Remember; the
      listing reuses the same list-view component as existing doc types, with a
      status column; and the detail page shows the RCA BigGlyph hero. (Broader
      visual parity is exercised by the E2E/visual-regression coverage named in
      the final criterion rather than by subjective comparison.)
- [ ] Given the new doc type, when `mise run check` and the visualiser test
      suites run, then they pass with new coverage that asserts: a server test
      that the Operate category and the RCA doc type are emitted in the
      phase/category structure; a frontend unit test that renders the RCA library
      card and listing; and an E2E test that navigates library → RCA listing →
      RCA detail.

## Open Questions

- _(Resolved)_ "Latest preview" and count behaviour on the library card follow
  existing doc-type card semantics (0041): count = number of `issue-research`
  artifacts, preview = most recently modified RCA. Now fixed in Requirement 5
  and the acceptance criteria.

## Dependencies

- Builds on: 0041 (server-driven library/list views), 0074 (per-doc-type
  hue/glyph map + related-artifacts render path), 0054 (search endpoint + UI),
  0057 (unified `issue-research` frontmatter schema), 0082 (BigGlyph set),
  0093 (RCA producing-skill + typed-linkage slot + `rca` stem mapping the
  related-artifacts integration and stem registration rely on),
  0096 (templates auto-discovery + `rca` glyph registration — functionally done).
- **Precondition**: 0096 is relied on in an un-closed state. Confirming its `rca`
  glyph/template registration is actually present is the first step of this work
  (per Requirement 2, "add only what is missing"); if it is absent or reverted,
  this work's scope expands to re-add it.
- Blocks: none known.
- No external dependencies.

## Assumptions

- The "Operate" category is server-emitted as part of the existing
  phase/category structure (0041), not a frontend-only constant — so this work
  touches both server and frontend.
- "Match the prototype" means visual and structural parity for the RCA pages
  specifically; it does not require reconciling unrelated drift between the
  current visualiser and the prototype.
- The prototype calls for an RCA BigGlyph hero, so creating it (if not already
  present in the BigGlyph set from 0082) is in scope for this work. The
  implementer should confirm presence first and create the asset if missing.
- Exercising the non-empty acceptance criteria (listing renders all RCAs, count
  and latest preview, detail page, related-artifacts row, search) depends on a
  test fixture containing a known set of `issue-research` artifacts plus at
  least one artifact whose frontmatter links to an RCA. The zero-count criterion
  uses the empty fixture. These criteria are verified against that fixture, not
  ambient repo state.

## Technical Notes

- `rca` and `issue-research` are the same doc type: display "Root cause
  analysis" / "Root cause analyses", type discriminator `issue-research` (0057),
  registration stem `rca` (0093/0096).
- Touchpoints to register the type, gathered from prior work: server
  phase/category + doc-type emission (0041), the `STEM_TO_GLYPH` map and the
  per-doc-type HSL (hue/saturation/lightness) hue map plus the
  `RelatedArtifacts` render component (0074), the BigGlyph set (0082), search
  (0054), and the templates view (0096). (`STEM_TO_GLYPH` and the hue map are
  frontend constants; `RelatedArtifacts` is a frontend component.)
- Authoritative design reference (full prototype source):
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full`.

## Drafting Notes

- Treated `rca`/`issue-research` as one doc type with the display names above.
  The surfaced labels ("Root cause analyses" plural / "Root cause analysis"
  singular, short "RCA") are now settled by the authoritative prototype
  (`src/data.jsx`, `src/ui.jsx`), so this is no longer an open question.
- Took 0096 as functionally complete (status lag noted by the author), so the
  `rca` glyph/template registration is assumed already present and this work
  verifies rather than re-adds it.
- Placement of the RCA card within Operate and Operate's position in the library
  ordering are taken to be defined by the prototype, not decided here.

## References

- Source: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full`
- Related: 0041, 0054, 0057, 0074, 0082, 0093, 0096
