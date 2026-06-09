---
id: "0088"
title: "Markdown Body Width Harmonisation"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
kind: story
status: done
priority: low
tags: [design, frontend, markdown, tokens]
type: work-item
schema_version: 1
last_updated: "2026-05-21T09:16:34+00:00"
last_updated_by: Toby Clemson
source: "design-gap:2026-05-21-current-app-vs-claude-design-prototype"
relates_to: ["work-item:0076", "plan:2026-05-26-0088-markdown-body-width-harmonisation"]
---

# 0088: Markdown Body Width Harmonisation

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

Standardise the markdown body on a measure-based reading cap
(`72ch`, sourced from a new `--ac-content-max-width-prose`
token) and prototype-aligned prose typography (`--size-prose`
14.5px / `--lh-prose` 1.65 / `font-weight: 300` / colour
`--ac-fg`), applied once on `MarkdownRenderer` so that every
consumer inherits a single source of truth for prose width,
size, weight and rhythm. Step h2 down to `--size-md` (18px) to
match the prototype's heading scale.

## Context

The current app caps the markdown body at a hard-coded
`max-width: 720px` in `MarkdownRenderer.module.css:2` and does
not set an explicit body `font-size` on `.markdown` — paragraph
text inherits from the document root. The Claude design prototype
caps at `max-width: 72ch` on `.ac-md` with a 14.5px / line-height
1.65 body. The two produce different visual reading widths and
neither matches the project's typography-token discipline.

The blocking groundwork is now in place:

- 0033 shipped the size and width token scales, including the
  `--size-*` family (excluding `--size-prose` and `--lh-prose`,
  which this story adds) and the `--ac-content-max-width{,-narrow}`
  family in `global.css`.
- 0075 landed the size-scale consumption rule, with vitest
  enforcement to prevent literal-px regressions.

Two surfaces consume `MarkdownRenderer`: the library detail page
(`LibraryDocView`) and the dev-only `CodeSyntaxShowcase` Playwright
fixture. The templates preview (`LibraryTemplatesView`) uses a
separate monospace renderer (`TemplateHighlight`, hljs) and is
out of scope for this work item.

## Requirements

- Add `--ac-content-max-width-prose: 72ch` to the design-token
  catalogue: declared in the same `:root` block as
  `--ac-content-max-width` and `--ac-content-max-width-narrow` in
  `global.css`, and exported from the same width-token section of
  `tokens.ts`. Bake the `ch` unit into the token value (i.e. the
  token stores `72ch`, not the bare number `72`) so consumers
  cannot accidentally re-unit it.
- Add prose typography tokens to `TYPOGRAPHY_TOKENS` (mirrored in
  `global.css`): `--size-prose: 14.5px` and `--lh-prose: 1.65`.
  These are off-scale prototype-derived values; precedent in the
  existing scale (`--size-xxs-sm: 11.5px`, `--size-row: 12.5px`,
  etc.) supports off-scale literals where the prototype demands.
- In `MarkdownRenderer.module.css`, replace the literal
  `max-width: 720px` with
  `max-width: min(var(--ac-content-max-width-prose), 100%)` on
  `.markdown`. The `100%` ensures the cap never overflows the
  parent grid track at narrower viewports.
- In `MarkdownRenderer.module.css`, set on `.markdown`:
  `font-size: var(--size-prose)`, `font-weight: 300`,
  `line-height: var(--lh-prose)`, and `color: var(--ac-fg)`
  (down from `--ac-fg-strong`). Headings (`h1`/`h2`/`h3`) retain
  the strong colour via an explicit `color: var(--ac-fg-strong)`
  on the shared headings rule.
- Step `.markdown h2` down from `var(--size-lg)` (22px) to
  `var(--size-md)` (18px) to match the prototype's heading scale.
  `h1` (28px / `--size-h3`) and `h3` (16px / `--size-sm`) are
  unchanged.
- Add `public/fonts/Inter-Light.woff2` (real 300-weight latin
  subset, sourced from Google Fonts' Inter v20 release) with an
  `@font-face { font-weight: 300 }` declaration. Update
  `SHA256SUMS`, `LICENSE-fonts.md`, and
  `EXPECTED_FONT_FILES` in `src/styles/fonts.test.ts` so the
  supply-chain ratchet stays green.
- Remove the markdown prose `max-width` from the `irreducible`
  list in `migration.test.ts:70` (it is no longer irreducible).
- Capture refreshed Playwright visual baselines for the affected
  routes so the new body styling is treated as the canonical
  state. At minimum, baselines must update for the
  `library-doc-view` and `code-syntax-showcase` Playwright specs.

## Acceptance Criteria

- Given the design tokens, when inspecting `global.css` and
  `tokens.ts`, then `--ac-content-max-width-prose` is defined
  with the value `72ch`, declared in the same `:root` block as
  `--ac-content-max-width` and `--ac-content-max-width-narrow` in
  `global.css` and exported from the same width-token section of
  `tokens.ts`.
- Given `MarkdownRenderer.module.css`, when reading the `.markdown`
  rule, then `max-width` is
  `min(var(--ac-content-max-width-prose), 100%)` and no literal
  `720px` (or any other px width) appears anywhere in the file.
- Given `MarkdownRenderer.module.css`, when reading the `.markdown`
  rule, then `font-size` is `var(--size-prose)`, `font-weight` is
  `300`, `line-height` is `var(--lh-prose)`, and `color` is
  `var(--ac-fg)`; no literal-px `font-size` and no literal
  `line-height` appear anywhere in the file.
- Given `MarkdownRenderer.module.css`, when reading the headings
  rule (`h1, h2, h3`), then `color` is explicitly
  `var(--ac-fg-strong)` so headings remain at the strong colour
  despite the `.markdown` rule's `--ac-fg` parent colour.
- Given the library detail page rendered at a 1440px viewport with
  a long-form article, when inspecting `.markdown`, then its
  computed `max-width` resolves to `72ch` in its own font context
  (≈520px at the `--size-prose` 14.5px body; the `ch` unit is
  the advance width of the `0` glyph, which fits roughly 95–100
  alphabetic characters per line at this size in Inter), the
  rendered prose column's width does not exceed that cap, and the
  prose column does not extend into the ~260px aside grid track.
- Given the library detail page rendered at an 800px viewport,
  when inspecting `.markdown`, then the `100%` branch of the
  `min()` is in effect — i.e. the rendered prose column width
  equals the parent grid track's computed width — and the prose
  column does not overflow the page wrapper.
- Given the migration test suite, when running `migration.test.ts`,
  then the markdown prose `max-width` is no longer listed as
  irreducible and the suite passes.
- Given the Playwright suite, when running the visual-regression
  jobs covering the library detail page and `CodeSyntaxShowcase`,
  then the commit includes refreshed baseline images for at least
  the `library-doc-view` and `code-syntax-showcase` Playwright
  specs and the suite passes against them.

## Dependencies

- Builds on: 0033 (size and width tokens — complete), 0075
  (size-scale consumption rule — complete).
- Related: 0076 (code-block palette ships on the same markdown
  surface). No hard ordering with this story — both touch
  `MarkdownRenderer.module.css`, so expect a rebase if landed in
  parallel.
- Blocks: none.

## Assumptions

- The same cap and body-size apply uniformly to every
  `MarkdownRenderer` consumer (`LibraryDocView` and
  `CodeSyntaxShowcase`); no per-surface carve-outs.
- The templates preview is a separate concern (monospace via
  `TemplateHighlight`, not markdown via `MarkdownRenderer`) and
  is intentionally out of scope. A future work item can decide
  its width policy independently.
- The visual change introduced by explicit prose typography on
  `.markdown` (`--size-prose` 14.5px / `font-weight` 300 /
  `--lh-prose` 1.65 / colour `--ac-fg`) — replacing
  inherited-from-root behaviour — is accepted as the canonical
  prose styling for the markdown renderer, in preference to the
  initially-drafted `--size-body` (20px). See Drafting Notes for
  the change of direction.

## Technical Notes

- Current literal: `max-width: 720px` at
  `MarkdownRenderer.module.css:2`; listed as "irreducible — no
  token equivalent" at `migration.test.ts:70`. Both pointers go
  away with this change.
- `MarkdownRenderer` is consumed by `LibraryDocView` (library
  detail route, default 1200px page wrapper, inside a `1fr |
  260px` grid giving the body column ~830px of horizontal
  budget at desktop and fluid below) and by
  `CodeSyntaxShowcase` (dev-only Playwright fixture). The
  templates preview uses `TemplateHighlight` and is not affected
  by this change.
- `1ch` is the advance width of the `0` glyph in the element's
  *own* computed font and font-size. The cap must therefore live
  on `.markdown` (the element that owns the font-family and
  size), which is already where the existing literal lives.
- At `--size-prose: 14.5px`, `72ch ≈ 520px` of container width.
  Because `1ch` measures the (relatively wide) `0` digit while
  average alphabetic glyphs in Inter are narrower, this still
  fits roughly 95–100 alphabetic characters per line — a
  comfortable reading measure. The width is therefore visibly
  narrower than the prior 720px literal, and the body size /
  weight / colour are the dominant visible deltas.
- `font-weight: 300` requires a real 300-weight Inter face;
  browsers do not synthesise lighter weights, so without a Light
  woff2 the declaration silently renders at 400. The repo's
  pre-existing `Inter-{Regular,Medium,SemiBold,Bold}.woff2` are
  byte-identical placeholders (shared SHA256), so the Light face
  is the first genuinely distinct Inter weight shipped.
- New token name `--ac-content-max-width-prose` follows the
  existing `--ac-content-max-width{,-narrow}` family and bakes
  the `ch` unit into the value (the token stores `72ch`, not the
  bare number `72`) so consumers cannot accidentally re-unit it.

## Drafting Notes

- Chose `72ch` over the 65ch reading-comfort default to preserve
  visual continuity with the current 720px hard-coded cap; a
  future revisit to 65ch can be made on reading-comfort grounds
  without re-litigating the token architecture.
- **Body size revised from 20px to 14.5px (post-draft).** The
  story initially chose `--size-body` (20px), but on review
  against the design prototype the 20px body read as much larger
  than the prototype's prose (`.ac-md` is 14.5px / lh 1.65 /
  weight 300 / colour `--ac-fg`). Direction changed to match the
  prototype, introducing `--size-prose: 14.5px` and
  `--lh-prose: 1.65` as off-scale prototype-derived tokens. The
  prototype's `0`-width digits are not in the scale at exactly
  these values, mirroring the existing precedent for off-scale
  literals (`--size-xxs-sm: 11.5px`, etc.).
- **Heading scale aligned to prototype (post-draft).** Stepped
  `h2` from `--size-lg` (22px) to `--size-md` (18px) to match the
  prototype's `.ac-md-h2`. `h1`/`h3` already matched closely
  enough (28px; 16px vs the prototype's 15px — the nearest scale
  tier) to leave unchanged.
- Scoped the templates preview *out* on the basis that it is a
  monospace highlighting surface (`TemplateHighlight`/hljs), not
  markdown, and a `ch`-based reading measure is not the right
  policy for code preview content.
- Picked the `min(var(--token), 100%)` CSS pattern over a bare
  `max-width: 72ch` so the cap composes cleanly with the parent
  grid track at any viewport without overflow.
- Migrated frontmatter `type:` → `kind:` to match the current
  work-item template schema.
- Bundled the width tokenisation and the explicit body
  `font-size` into one increment because they share a file, a
  renderer, and an interdependence — `72ch` is sized against the
  computed body font, so introducing the `ch`-based cap without
  also pinning the body size to `--size-prose` would leave the
  effective measure dependent on the inherited document-root
  size. Shipping them together makes the cap meaningful.
- Sourced the Light face from Google Fonts' Inter v20 latin
  subset (OFL-1.1, same provenance as the other bundled faces).
  Out of scope: replacing the four byte-identical placeholder
  Inter faces with genuinely distinct Regular/Medium/SemiBold/
  Bold variants — flagged as a possible follow-up but not blocking
  this story.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Prototype prose styling (`.ac-md` / `.ac-md-h2`):
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`
- Plan: `meta/plans/2026-05-26-0088-markdown-body-width-harmonisation.md`
  (see its **Post-Implementation Deviations** section)
- Related: 0033, 0075, 0076
