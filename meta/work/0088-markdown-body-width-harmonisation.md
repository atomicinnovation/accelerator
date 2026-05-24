---
work_item_id: "0088"
title: "Markdown Body Width Harmonisation"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
kind: story
status: ready
priority: low
parent: ""
tags: [design, frontend, markdown, tokens]
---

# 0088: Markdown Body Width Harmonisation

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

Standardise the markdown body on a measure-based reading cap
(`72ch`, sourced from a new `--ac-content-max-width-prose`
token) and an explicit body-text size (`--size-body`, 20px),
applied once on `MarkdownRenderer` so that every consumer
inherits a single source of truth for prose width and size.

## Context

The current app caps the markdown body at a hard-coded
`max-width: 720px` in `MarkdownRenderer.module.css:2` and does
not set an explicit body `font-size` on `.markdown` — paragraph
text inherits from the document root. The Claude design prototype
caps at `max-width: 72ch` on `.ac-md` with a 14.5px / line-height
1.65 body. The two produce different visual reading widths and
neither matches the project's typography-token discipline.

The blocking groundwork is now in place:

- 0033 shipped the size and width token scales, including
  `--size-body: 20px` and the `--ac-content-max-width{,-narrow}`
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
- In `MarkdownRenderer.module.css`, replace the literal
  `max-width: 720px` with
  `max-width: min(var(--ac-content-max-width-prose), 100%)` on
  `.markdown`. The `100%` ensures the cap never overflows the
  parent grid track at narrower viewports.
- In `MarkdownRenderer.module.css`, set
  `.markdown { font-size: var(--size-body); }` so prose body text
  has an explicit token-sourced size rather than inheriting from
  the document root.
- Remove the markdown prose `max-width` from the `irreducible`
  list in `migration.test.ts:70` (it is no longer irreducible).
- Capture refreshed Playwright visual baselines for the affected
  routes so the new body size is treated as the canonical state.
  At minimum, baselines must update for the `library-doc-view` and
  `code-syntax-showcase` Playwright specs.

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
  rule, then `font-size` is `var(--size-body)` and no literal
  px `font-size` appears anywhere in the file.
- Given the library detail page rendered at a 1440px viewport with
  a long-form article, when inspecting `.markdown`, then its
  computed `max-width` resolves to `72ch` in its own font context
  (≈720px at the `--size-body` 20px body), the rendered prose
  column's width does not exceed that cap, and the prose column
  does not extend into the ~260px aside grid track.
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
- The visual change introduced by an explicit
  `font-size: var(--size-body)` (20px) on `.markdown` —
  replacing inherited-from-root behaviour — is accepted as the
  canonical paragraph size for the markdown renderer.

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
- At `--size-body: 20px`, `72ch ≈ 720px`. The width change vs
  the current cap is therefore near-zero; the visible delta is
  the explicit body `font-size`.
- New token name `--ac-content-max-width-prose` follows the
  existing `--ac-content-max-width{,-narrow}` family and bakes
  the `ch` unit into the value (the token stores `72ch`, not the
  bare number `72`) so consumers cannot accidentally re-unit it.

## Drafting Notes

- Chose `72ch` over the 65ch reading-comfort default to preserve
  visual continuity with the current 720px hard-coded cap; a
  future revisit to 65ch can be made on reading-comfort grounds
  without re-litigating the token architecture.
- Chose `--size-body` (already defined as 20px in `global.css`)
  for the explicit body font-size — the token's inline rationale
  already names it as the prose tier, so no new size token is
  introduced.
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
  also pinning the body size to `--size-body` would leave the
  effective measure dependent on the inherited document-root
  size. Shipping them together makes the cap meaningful.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0033, 0075, 0076
