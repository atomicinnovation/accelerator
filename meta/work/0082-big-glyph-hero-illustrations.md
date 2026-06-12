---
id: "0082"
title: "BigGlyph Hero Illustration Set"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
kind: story
status: done
priority: medium
tags: [design, frontend, components, illustrations]
type: work-item
schema_version: 1
last_updated: "2026-06-09T18:21:39+00:00"
last_updated_by: Toby Clemson
blocked_by: ["work-item:0073", "work-item:0074"]
blocks: ["work-item:0083"]
source: "design-gap:2026-05-21-current-app-vs-claude-design-prototype"
relates_to: ["work-item:0041", "work-item:0037"]
---

# 0082: BigGlyph Hero Illustration Set

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a user landing on a doc type with no documents yet, I want a
recognisable, type-specific hero illustration so the empty page reads as a
designed, on-brand surface rather than a bare line of text. Ship a
`BigGlyph` component — one bespoke per-doc-type illustration drawn with a
seven-tone palette derived from the doc type's hue — and use it as the hero
of the per-type empty state.

## Context

The design prototype ships per-doc-type hero illustrations in
`big-glyphs.jsx`: bespoke, hand-fitted SVG shapes on an 80×80 viewBox for
all thirteen doc types, each rendered through a `bigPalette(hue)` helper
that derives a seven-tone palette from the doc type's single hue, plus a
`DEFAULT_BIG` paper fallback for unknown types. In the prototype these
illustrations are the hero of the full-page per-type empty state
(`LibraryIndexEmpty`, rendered at 96px) and are also showcased on the dev
design-system page.

The current app has no equivalent illustration system. 0037 delivered the
small `Glyph` (16/24/32px icon) but no large-scale hero. The current
per-type empty state (`EmptyState.tsx`, the app's equivalent of the
prototype's `LibraryIndexEmpty`) renders a generic `PaperFold` hero at 72px,
hue-tinted but not type-specific. This story replaces that `PaperFold` hero
with the dedicated, type-specific `BigGlyph`.

## Requirements

- Implement a `BigGlyph` React component accepting a `docType` prop drawn
  from `DocTypeKey` (`src/api/types.ts`). The canonical thirteen keys are
  `decisions`, `work-items`, `plans`, `research`, `plan-reviews`,
  `pr-reviews`, `work-item-reviews`, `validations`, `notes`,
  `pr-descriptions`, `design-gaps`, `design-inventories`, and `templates`;
  the `DOC_TYPE_KEYS` array in `src/api/types.ts` is the authoritative
  source the illustration set must be reconciled against.
- Each doc type is illustrated by a bespoke SVG hero shape traced from the
  prototype's `big-glyphs.jsx` (80×80 viewBox, scalable to any pixel size),
  with a `DEFAULT_BIG` fallback for any key not in the thirteen-key set.
- Tones are derived at render time from the doc type's hue via a
  `bigPalette(hue)` helper that returns exactly seven tones for every doc
  type: six hue-derived tones plus a fixed `white`. Per-doc-type tones are
  not hard-coded. The green/red diff tints used by the `pr-reviews`
  illustration are *not* members of the seven-tone palette — they are
  additional structural constants applied only by that one illustration,
  and should match the prototype's `pr-reviews` diff tints. The only
  constant colours anywhere are therefore `white` (part of every palette)
  and those two `pr-reviews` diff tints.
- The hue input is a numeric HSL hue (0–360). Source it the same way
  `EmptyState` already does (`copy.hue` → `--ac-empty-page-hue`), keeping
  colour identity consistent with the small `Glyph` (0037) and the
  per-doc-type hue tokens surfaced by 0074.
- Surface `BigGlyph` as the hero of the per-type empty state
  (`EmptyState.tsx`), replacing the current `PaperFold` hero, rendered at
  96px to match the prototype.
- The illustration is decorative: render it `aria-hidden="true"`; the
  empty state's existing accessible text (title, lede, footer) is unchanged.

## Acceptance Criteria

- [ ] A `BigGlyph` component renders a hero illustration for each of the
  thirteen `DocTypeKey` values listed in Requirements (reconciled against
  `DOC_TYPE_KEYS` in `src/api/types.ts`), and falls back to `DEFAULT_BIG`
  for any key not in that set. Each hero's SVG path data is traced from the
  corresponding shape in the prototype's `big-glyphs.jsx` at the 80×80
  viewBox; tracing fidelity is confirmed by signing off the visual-regression
  baselines (final criterion below) against side-by-side renders of the
  prototype shape at the same viewBox, and per-doc-type distinctness is
  likewise verified by those baselines rather than by subjective judgement.
- [ ] `bigPalette(hue)` returns exactly seven tones (six hue-derived plus a
  fixed `white`) and each illustration's tones are derived from a single
  hue; no per-doc-type tone is hard-coded. This is verified by unit-testing
  that `bigPalette(hue)` returns tones whose HSL hue component equals the
  input hue for at least two distinct sample hues, and by code review
  confirming no per-type colour literals exist outside `white` and the two
  `pr-reviews` diff tints. Those diff tints are the only constant colours
  besides `white`, are not members of the seven-tone palette, and equal the
  exact diff-tint constants in the prototype's `big-glyphs.jsx`.
- [ ] `BigGlyph` is rendered as the hero of the per-type empty state
  (`EmptyState.tsx`) at 96px, replacing the previous `PaperFold` hero.
- [ ] The illustration is exposed as decorative (`aria-hidden="true"`), and
  the empty state retains its accessible title, lede, and footer copy.
- [ ] A Playwright visual-regression spec covers all 26 hero × theme
  combinations (thirteen doc types × light and dark). The criterion is met
  only when correct, design-approved baselines exist for every combination
  and the spec passes against them — not merely when snapshots have been
  captured. Sign-off on those baselines is what operationalises the
  Summary's "recognisable, on-brand" intent.

## Dependencies

- Blocked by: 0073 (brand palette tokens — `done`), 0074 (per-doc-type hues
  consumable outside the sidebar — `done`). Both blockers are complete, so
  this story is unblocked.
- Related: 0037 (small `Glyph`, the icon-scale ancestor and shared hue
  source), 0041 (owns the per-type empty-state card whose hero this
  replaces).
- Blocks: 0083 (DevDesignSystem reference page) — its BigGlyph showcase
  consumes the component this story delivers, so 0083 cannot land until
  0082 ships.
- Coordination: 0108 (local Docker visual-regression baselines) — soft
  sequencing edge, not a hard block. 0082 captures new visual-regression
  baselines; prefer landing 0108's single-Linux baseline collapse first, so
  0082's baselines are generated under the Docker regime and do not need
  regenerating afterwards.

## Assumptions

- Doc-type hue mapping is shared with the small `Glyph` component (0037) and
  the existing `EmptyState` hue plumbing, so colour identity stays
  consistent across surfaces.

## Technical Notes

- `EmptyState.tsx` is the integration point: it already resolves a numeric
  `copy.hue` and sets `--ac-empty-page-hue`, so the numeric hue `bigPalette`
  needs is already available. The change is swapping the `<PaperFold>` hero
  element for `<BigGlyph docType={docType} size={96} />`.
- `bigPalette(hue)` builds `hsl(...)` tones from a bare hue number, so the
  hue source must be numeric (0–360), not a resolved colour token.
- The prototype also showcases `BigGlyph` on its dev design-system page;
  that surface belongs to 0083 and is out of scope here.

## Drafting Notes

- The original draft claimed the current empty state uses "the small Glyph
  at large size" (per 0041); the live code uses a generic `PaperFold` hero,
  which is what BigGlyph replaces. Corrected.
- The original draft scoped BigGlyph onto the library landing's empty-card
  variant. The prototype's `LibraryLandingEmptyCard` uses the *small*
  `TypeGlyph` (34px), not BigGlyph, so the landing card has been removed
  from scope to follow the designs.
- The original draft referenced a `GlyphDocTypeKey` "twelve-key" union; the
  real type is `DocTypeKey` with thirteen keys (including `templates`).
- Open questions about shape provenance and palette accessibility were
  resolved against the prototype source: all shapes are traceable (nothing
  drafted from scratch), and the illustration is decorative
  (`aria-hidden`), so per-tone contrast tuning is not required.
- Completed dependencies (0073, 0074) are kept as `blocked_by` edges rather
  than demoted to `relates_to`, since they record real build-order
  dependencies now satisfied; their `done` status is what unblocks 0082.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Prototype: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/src/big-glyphs.jsx`
- Related: 0037, 0041, 0073, 0074, 0083
