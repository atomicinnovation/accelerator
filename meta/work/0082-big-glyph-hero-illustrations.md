---
id: "0082"
title: "BigGlyph Hero Illustration Set"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
kind: story
status: draft
priority: medium
tags: [design, frontend, components, illustrations]
type: work-item
schema_version: 1
last_updated: "2026-05-21T09:16:34+00:00"
last_updated_by: Toby Clemson
blocked_by: ["work-item:0073", "work-item:0074"]
source: "design-gap:2026-05-21-current-app-vs-claude-design-prototype"
relates_to: ["work-item:0041"]
---

# 0082: BigGlyph Hero Illustration Set

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Ship per-doc-type hero illustrations (`BigGlyph`) with a seven-tone
palette derived from a single hue per doc type, and surface them as the
hero in `LibraryIndexEmpty` and any other empty-state or landing card
slot.

## Context

The prototype ships per-doc-type hero illustrations (`src/big-glyphs.jsx`)
with a 7-tone palette derived from a single hue (`bigPalette(hue)`) and
a default fallback, used in `LibraryIndexEmpty` and the landing card
hero. The current app has no equivalent illustration system; 0037
delivered the small `Glyph` (16/24/32 px icon) but no large-scale hero
illustration.

0041 already ships an empty-state card on the per-type list view that
uses the small Glyph at large size. This story replaces that surface
with the dedicated `BigGlyph` hero, so the screen does not collapse to a
bare line of text on freshly-initialised repos.

## Requirements

- Implement a `BigGlyph` React component accepting a `docType` prop
  drawn from `GlyphDocTypeKey` (0037's twelve-key union).
- Each doc type is illustrated by a hand-fitted SVG hero shape rendered
  via a 7-tone palette generated from the doc type's hue using a
  `bigPalette(hue)` helper.
- Hue input is the per-doc-type hue token introduced by 0074 (or
  derived from the existing `--ac-doc-<key>` colour).
- Surface `BigGlyph` in the empty-state card on `/library/{type}`
  (replacing the current small-Glyph hero shipped by 0041) and on the
  library landing's empty-card variant.

## Acceptance Criteria

- [ ] A `BigGlyph` component renders a distinct hero illustration for
  each of the twelve `GlyphDocTypeKey` values.
- [ ] The seven-tone palette per doc type is derived from a single hue
  via `bigPalette(hue)`; no hard-coded per-tone hex values appear in
  component CSS.
- [ ] `BigGlyph` is rendered as the hero in the per-type empty state on
  `/library/{type}` and in the library landing's empty-card variant.
- [ ] A Playwright visual-regression spec captures all twelve heroes in
  both light and dark themes.

## Open Questions

- What is the hero illustration shape for each doc type — is there a
  prototype asset we can trace, or do shapes need to be drafted from
  scratch?
- Does `bigPalette(hue)` need accessibility tuning per-tone, or are the
  prototype tones acceptable as-is?

## Dependencies

- Blocked by: 0073 (brand palette), 0074 (per-doc-type hues consumable
  outside sidebar), 0037 (canonical `GlyphDocTypeKey` union).
- Related: 0041 (existing empty-state card that this story replaces the
  hero of).
- Blocks: none.

## Assumptions

- Doc-type hue mapping is shared with the small `Glyph` component
  (0037), so colour identity stays consistent across surfaces.

## Technical Notes

- `LibraryIndexEmpty` is already implemented by 0041 with a small-Glyph
  hero — replacement of the hero element is the integration point.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0037, 0041, 0073, 0074
