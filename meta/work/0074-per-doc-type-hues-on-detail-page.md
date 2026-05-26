---
work_item_id: "0074"
title: "Per-Doc-Type Hues on Detail Page"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
kind: story
status: in-progress
priority: medium
parent: ""
tags: [design, frontend, detail-page, tokens]
---

# 0074: Per-Doc-Type Hues on Detail Page

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a viewer of a project document in the visualiser, I want the detail
page to carry per-doc-type colour cues (the page eyebrow icon — the
small icon above the page heading established by 0041 — and the
related-document row icons in the right-hand aside) so I can identify
the document's type at a glance — matching the cue language already
used in the sidebar and library hub.

The current app declares `--ac-doc-<key>` and `--ac-doc-bg-<key>` tokens
(introduced by 0037) but restricts them to the sidebar and library hub.
This story surfaces those tokens on the detail page. Hero illustration
tint is owned by 0082 and is out of scope here.

## Context

The current app declares `--ac-doc-{key}` and `--ac-doc-bg-{key}` tokens
at `src/styles/global.css:98-124` but explicitly does not consume them on
the detail page — they are restricted to the sidebar and library hub.
The prototype defines a thirteen-entry HSL map in `src/ui.jsx:264-278`
(`work` hue 12, `decisions` 355, `research` 28, `plans` 220, `notes` 50,
`design-inventories` 185, `design-gaps` 95, …) consumed by `TypeGlyph`,
`StageTile`, `BigGlyph`, and the empty-page tint.

The `DocTypeKey` enumeration has 13 entries: 12 are non-virtual
(`decisions`, `work-items`, `plans`, `research`, `plan-reviews`,
`pr-reviews`, `work-item-reviews`, `validations`, `notes`,
`pr-descriptions`, `design-gaps`, `design-inventories`) and one
(`templates`) is a **virtual** key — a synthesised pseudo-type with no
on-disk documents. The 12 `--ac-doc-<key>` tokens at
`global.css:98-124` cover the non-virtual keys; the virtual `templates`
key intentionally has no per-type token and is treated as a neutral-
fallback case on the detail page. The prototype's 13-entry hue map
assigns a hue to all 13 keys including `templates`; this story
consumes only the 12 non-virtual hues and leaves the `templates` hue
unused.

## Requirements

- Surface `--ac-doc-<key>` on the detail page eyebrow icon (slot
  `[data-slot="eyebrow"]` in `src/components/Page/Page.tsx`) and on
  the related-document row icons in the right-hand aside (rendered by
  `src/components/RelatedArtifacts/RelatedArtifacts.tsx`), for the 12
  non-virtual `DocTypeKey` values. The eyebrow label text and the
  aside row's background and borders retain their pre-change computed
  values.
- For the virtual `templates` key, the eyebrow icon and aside row
  icons resolve to exactly `var(--ac-text-muted)` and no `--ac-doc-*`
  token is applied.
- If `RelatedArtifacts` does not currently render a per-row doc-type
  icon, adding that icon is in scope (still icon-only — no background
  or border changes).
- Keep sidebar and library hub consumption unchanged (no regression
  in computed colour values).
- Hero illustration / BigGlyph tint is out of scope — owned by 0082.

## Acceptance Criteria

All three criteria below are verified by a single automated Playwright
end-to-end (e2e) spec under `skills/visualisation/visualise/frontend/e2e/`,
modelled on the existing `e2e/chip-resolved-colours.spec.ts` and
`e2e/glyph-resolved-fill.spec.ts` (computed-style assertions, not pixel
snapshots). The spec embeds literal RGB string constants captured from
the `--ac-doc-*` declarations at `src/styles/global.css:98-124` at
story start; it does not read those values from the live stylesheet at
test time (so the spec is insulated from later 0073 brand-layer churn).

- [ ] For each of the 12 non-virtual `DocTypeKey` values, navigating
  to the corresponding detail-page fixture route (one fixture per doc
  type under `server/tests/fixtures/meta/`; new fixtures added as
  needed to cover all 12 non-virtual types) renders the eyebrow icon
  within `[data-slot="eyebrow"]` with computed `color` equal to the
  literal RGB string captured for `--ac-doc-<key>`. The eyebrow label
  text's computed `color` value is identical pre- and post-change. For
  the virtual `templates` key, the eyebrow icon's computed `color`
  resolves to exactly the literal RGB string captured for
  `--ac-text-muted` and no `--ac-doc-*` token is applied.
- [ ] For each of the 12 non-virtual `DocTypeKey` values, at least one
  detail-page fixture must include a `RelatedArtifacts` row linking to
  a document of that type; that row's icon has computed `color` equal
  to the literal RGB string captured for `--ac-doc-<key>`. The row
  container's computed `background-color`, `border-color`, and
  `border-width` are identical pre- and post-change. (Templates is
  descoped from this criterion — see Drafting Notes.)
- [ ] For each of the 12 non-virtual doc types, the elements in the
  sidebar (`TypeGlyph`) and library hub (`StageTile`) that consume
  `--ac-doc-<key>` resolve to the same literal RGB strings as captured
  above (sidebar and library hub consumption unchanged).

## Dependencies

- Blocked by: 0037 (per-doc-type tokens already delivered).
- In-scope prerequisites (no external blocker): create new e2e
  fixtures for `work-item-reviews`, `design-gaps`, and
  `design-inventories` under
  `skills/visualisation/visualise/server/tests/fixtures/meta/` so AC #1
  can iterate over all 12 non-virtual types; introduce a per-row
  doc-type icon in `RelatedArtifacts` if absent today (see Technical
  Notes).
- Blocks: 0082 (BigGlyph hero illustration is explicitly blocked by
  this story); 0079 (Aside Region Redesign — must preserve the
  per-doc-type icon colour established by this story when restructuring
  the aside surface).
- Related: 0073 (Atomic Brand-Layer Palette — feeds the `--ac-*`
  semantic layer; this story can proceed independently because AC #3
  captures its baseline from the `--ac-doc-*` declarations current at
  story start, insulating the spec from any later brand-layer churn),
  0075 (eyebrow sizing reconciliation — independent of this story
  because colour and sizing properties do not overlap), 0041 (Library
  Page Wrapper — establishes the eyebrow pattern this story mirrors;
  already delivered).

## Technical Notes

- 0037 introduced the `--ac-doc-*` token namespace; this story is
  consumption-only (no new tokens; the virtual `templates` key is
  not extended into the namespace — see Drafting Notes).
- Existing tokens are declared at `src/styles/global.css:98-124`.
- Eyebrow component: `src/components/Page/Page.tsx` — the eyebrow
  slot is identified by `[data-slot="eyebrow"]`. Current
  `EyebrowLabel` usage renders a `<Glyph>` icon followed by uppercase
  label text; this story tints the glyph only.
- Aside related-row component:
  `src/components/RelatedArtifacts/RelatedArtifacts.tsx` — rows are
  rendered as `<li>` elements inside `<ul className={styles.groupList}>`.
  The current row may not include a per-doc-type icon (existing
  badges show declared/inferred relationship kind, not doc type); if
  absent, adding the icon is in scope (see Requirements).
- A `data-doc-type="<key>"` attribute on the page root is one viable
  scoping mechanism (lets attribute selectors apply tokens without
  per-component conditionals). Direct token consumption per component
  is equally acceptable — implementer's choice.
- E2e fixtures under
  `skills/visualisation/visualise/server/tests/fixtures/meta/`
  currently cover `work-items`, `decisions`, `notes`, `plans`,
  `pr-descriptions`, `pr-reviews`, `plan-reviews`, `validations`, and
  `research`. New fixture documents are needed to cover
  `work-item-reviews`, `design-gaps`, and `design-inventories` for
  AC #1's "12 non-virtual values" coverage claim.
- The prototype source at
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`
  is the authoritative reference for the full thirteen-entry hue map
  and the per-doc-type consumption patterns (`TypeGlyph`, `StageTile`,
  `BigGlyph`, empty-page tint).
- The non-regression spec for AC #3 should follow the existing pattern
  in `e2e/chip-resolved-colours.spec.ts` and
  `e2e/glyph-resolved-fill.spec.ts` (computed-style assertions, not
  pixel snapshots).

## Drafting Notes

- Hero illustration / BigGlyph tint explicitly scoped out — owned by
  0082 (which is blocked by this story).
- Aside-row tint surface pinned to "row icon only" during review-1
  (was previously left to implementation). Icon-only is the most
  surgical surface and minimises conflict with 0079's aside redesign.
- Eyebrow tint pinned to "icon only" during review-1 (icon vs label
  vs both was previously unresolved). Choice is consistent with the
  aside-row decision.
- Virtual `templates` doc-type uses `var(--ac-text-muted)` exactly as
  the fallback on the detail page (pinned during review-2; review-1
  hedged with "or equivalent neutral"). Extending the `--ac-doc-*`
  token namespace to cover virtual keys is explicitly out of scope
  (would push into 0037 territory).
- 0079 sequencing resolved during review-1: this story blocks 0079
  (rather than waiting on it) so the per-doc-type icon colour is
  established before the aside redesign begins.
- `RelatedArtifacts` may not currently render a doc-type icon per row
  (review-2 found only relationship-kind badges). Adding the icon is
  in scope if absent (Requirements). 0079's redesign may further
  restructure this surface, which is why this story Blocks 0079.
- E2e fixture coverage for `work-item-reviews`, `design-gaps`, and
  `design-inventories` is missing today (review-2); adding fixtures
  is part of AC #1's coverage claim.
- AC #3 baseline strategy pinned during review-2: assert against the
  literal RGB values produced by today's `--ac-doc-*` declarations
  (not a separately captured snapshot run); insulates the spec from
  later 0073 brand-layer value churn.
- `data-doc-type` attribute approach kept as suggested technical
  approach rather than acceptance criterion — implementer's choice.
- Frontmatter `type:` field renamed to `kind:` per 0063 (done).
- AC #2 templates row descoped during implementation: the backend never
  emits a `templates` row in any `RelatedArtifacts` response — templates
  are excluded from indexing (`indexer.rs`) and inferred clustering
  (`clusters.rs`) by design (no on-disk fixtures). AC #2 therefore covers
  the 12 non-virtual doc types only; templates Glyph rendering is verified
  indirectly by the detail-page eyebrow spec (templates detail route) and
  the listing-route eyebrow spec (`/library/templates`).

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Prototype: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`
- Related: 0037, 0041, 0073, 0075, 0079, 0082
