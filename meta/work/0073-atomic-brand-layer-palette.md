---
id: "0073"
title: "Atomic Brand-Layer Palette"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
kind: story
status: done
priority: medium
tags: [design, frontend, tokens, foundation]
type: work-item
schema_version: 1
last_updated: "2026-05-21T09:16:34+00:00"
last_updated_by: Toby Clemson
blocked_by: ["work-item:0033"]
blocks: ["work-item:0082"]
source: "design-gap:2026-05-21-current-app-vs-claude-design-prototype"
relates_to: ["adr:ADR-0026"]
external_id: PP-95
---

# 0073: Atomic Brand-Layer Palette

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a design system maintainer, I want the `--ac-*` semantic layer to
reference a brand-layer single source of truth, so that brand updates flow
through component CSS automatically and downstream illustrative work
(BigGlyph, type-tinted iconography) has named brand tokens to consume.

Introduce the `--atomic-*` brand-layer palette declared in the prototype's
`prototype-standalone.html` (inline `:root` block; the prototype is the
canonical enumeration — captured exhaustively in the
`prototype-tokens.json` fixture) and rearchitect the stylesheet so the
brand palette feeds the existing `--ac-*` semantic layer rather than hex
literals being baked directly into the semantic layer. BigGlyph (the
large illustrative glyph system; see 0082) and type-tinted iconography
are the principal downstream consumers.

## Context

The current `src/styles/global.css` declares only the semantic `--ac-*`
layer with no brand-named source-of-truth, so type-tinted iconography and
illustrative artwork have nowhere to pull from. 0033 (Design Token System)
explicitly excluded the `--atomic-*` brand palette from scope (see 0033
lines 113-114: "'Brand palette — `--atomic-*`' is excluded; raw brand
colours not consumed by components"). The new gap analysis identifies this
exclusion as the missing brand layer that downstream BigGlyph and
illustrative work depends on.

The prototype's brand palette is declared inline in
`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`
(starting ~line 183) — there is no separate `assets/tokens.css`. It
includes named tokens such as `--atomic-night`, `--atomic-ink`,
`--atomic-indigo`, `--atomic-marigold`, `--atomic-aquamarine`,
`--atomic-cream-can`, plus the aliases (`--atomic-violet`,
`--atomic-teal`) and overlays (`--atomic-overlay-ink`,
`--atomic-stroke-light`, `--atomic-shadow-soft`).

ADR-0026 (CSS Design Token Application Conventions) governs how tokens are
authored and consumed across `global.css` and `tokens.ts`.

## Requirements

- Introduce the full `--atomic-*` brand palette (named colours, aliases,
  and overlays) in `src/styles/global.css` `:root` and mirror it in
  `tokens.ts` under a brand-layer constant (e.g. `BRAND_COLOR_TOKENS`),
  following the existing bare-key naming convention used by
  `LIGHT_COLOR_TOKENS` / `DARK_COLOR_TOKENS`. The fixture
  `src/styles/fixtures/prototype-tokens.json` is the canonical
  enumeration; its `--atomic-*` section is populated exhaustively from
  `prototype-standalone.html` and the fixture test enforces parity
  between fixture and prototype.
- Rearchitect the stylesheet so the `--ac-*` semantic layer references
  `--atomic-*` brand tokens via `var()`, rather than baking hex literals
  into the semantic layer. Comparison rule: hex values on both sides are
  normalised to lowercase six-digit form (expanding any shorthand) and
  compared as strings. The reference snapshot for "resolved value" is
  the state of `src/styles/global.css` on `main` at the point the
  implementation branch is cut. Only `--ac-*` tokens whose normalised
  resolved hex matches an `--atomic-*` value exactly are rewritten;
  remaining literals stay as-is and are documented in the PR
  description.
- Preserve resolved `--ac-*` values to keep visual output identical
  against the current app where mappings are clean.
- Extend `src/styles/global.test.ts` to assert CSS↔TS parity over the
  brand layer using the existing `readCssVar()` regex pattern.
- Extend `src/styles/prototype-tokens.fixture.test.ts` and
  `src/styles/fixtures/prototype-tokens.json` to drift-detect `--atomic-*`
  against `prototype-standalone.html`, alongside the existing `--code-*`
  and `--tk-*` coverage.

## Acceptance Criteria

- [ ] `src/styles/fixtures/prototype-tokens.json` enumerates every
  `--atomic-*` declaration parsed from `prototype-standalone.html`;
  `prototype-tokens.fixture.test.ts` asserts the two sets are equal
  (no missing tokens, no extras). The fixture is the canonical
  expected-token list for the brand layer.
- [ ] Every token listed under `--atomic-*` in `prototype-tokens.json`
  is defined in `global.css` `:root` and mirrored in `tokens.ts` under
  a brand-layer constant with bare-key naming consistent with
  `LIGHT_COLOR_TOKENS`; `global.test.ts` asserts CSS↔TS parity over the
  brand layer.
- [ ] Every `--ac-*` semantic token whose resolved hex value
  (normalised to lowercase six-digit form, compared against the state
  of `global.css` on `main` at branch-cut) exactly matches an
  `--atomic-*` value is rewritten as `var(--atomic-X)`; any `--ac-*`
  token left as a hex literal is enumerated with rationale in the PR
  description.
- [ ] `npm test -- src/styles/global.test.ts
  src/styles/prototype-tokens.fixture.test.ts` passes locally and the
  PR's CI run is green.
- [ ] `tests/visual-regression/tokens.spec.ts` passes without
  `--update-snapshots` for every captured route × theme; any baseline
  that requires regeneration is justified in the PR with a side-by-side
  comparison demonstrating max per-pixel ΔE2000 < 5 over the changed
  regions, computed via a CIEDE2000 implementation (e.g. `culori`'s
  `differenceCiede2000`).

## Non-Goals

- Theme-dependent overlay/shadow token decisions. If any of the overlay
  tokens (`--atomic-overlay-ink`, `--atomic-stroke-light`,
  `--atomic-shadow-soft`) prove theme-dependent in practice — i.e. they
  would need per-theme overrides at the brand layer to be honest about
  their behaviour — they are deferred to 0077 and the affected `--ac-*`
  tokens remain hex literals in this story (mirroring AC2's "stays a
  literal, documented in the PR" escape hatch).
- Colour harmonisation or near-match consolidation. Only exact
  normalised-hex matches are rewritten; near-matches stay as literals.
- Brand-layer dark-mode overrides. The brand palette is theme-invariant
  (see Assumptions); per-theme overrides remain at `--ac-*`.

## Open Questions

- Are `--atomic-violet` / `--atomic-teal` aliases identical in hex to any
  existing `--ac-*` semantic value, or do they introduce new brand
  variants?
- Should the TS-side brand layer be a new export (e.g.
  `BRAND_COLOR_TOKENS`) in the existing `tokens.ts`, or a sibling file
  (`brand-tokens.ts`)?
- Are any of the overlay tokens (`--atomic-overlay-ink`,
  `--atomic-stroke-light`, `--atomic-shadow-soft`) actually
  theme-dependent in the prototype, contradicting the theme-invariance
  assumption?

## Dependencies

- Blocked by: 0033 (semantic token layer must exist before brand layer
  feeds it).
- Blocks: 0082 (BigGlyph illustrations depend on brand palette tokens).
- Related / ordering: 0077 (Shadow and Dark-Accent Token Audit) — this
  story introduces overlay tokens (`--atomic-overlay-ink`,
  `--atomic-stroke-light`, `--atomic-shadow-soft`) that overlap 0077's
  audit surface. Whichever story ships first sets the convention the
  other must follow; if 0073 lands first, 0077 must reconcile any
  changes it proposes against the brand-layer declarations established
  here.
- Governed by: ADR-0026 (CSS Design Token Application Conventions) —
  any deviation from the `var(--atomic-X)` rewrite rule requires
  consulting the ADR and is in scope only if the ADR permits it.
- Source-of-truth artefact:
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`
  is treated as frozen for the duration of implementation. Any
  designer-driven change to inline `--atomic-*` declarations during
  implementation invalidates the fixture and visual-diff baselines and
  must be coordinated explicitly.
- Tooling pre-condition: the Playwright visual-regression project
  (workers=1, baselines under
  `tests/visual-regression/__screenshots__/`) must be runnable in the
  implementer's environment; AC5's regeneration pathway depends on this
  suite being executable end-to-end.

## Assumptions

- The brand palette is theme-invariant; per-theme overrides remain at the
  `--ac-*` semantic layer.
- Every current `--ac-*` hex literal traces to exactly one `--atomic-*`
  value — no semantic token is a blend of two brand tokens. If this
  proves false during implementation, the offending token stays a
  literal and is documented in the PR.
- The prototype's brand palette is the source of truth for this story;
  designer changes to brand values during implementation would
  invalidate the visual-diff baselines and require re-baselining.

## Technical Notes

- Source provenance: `--atomic-*` tokens are declared inline in
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`
  starting ~line 183. There is no `assets/` subdirectory in the
  prototype inventory.
- TS-side pattern: `tokens.ts` exports `LIGHT_COLOR_TOKENS` /
  `DARK_COLOR_TOKENS` as bare-key objects (`'ac-bg': '#fbfcfe'`). The
  brand layer should follow the same convention so the existing
  `readCssVar()` regex pattern in `global.test.ts` extends without new
  test infrastructure.
- Drift detection: `src/styles/prototype-tokens.fixture.test.ts`
  already reads `prototype-standalone.html` and compares against
  `src/styles/fixtures/prototype-tokens.json` for `--code-*` / `--tk-*`.
  Extension point is the parameterised describe block plus a fixture
  JSON addition.
- Visual-regression infrastructure: Playwright `toHaveScreenshot()`
  driven from `tests/visual-regression/tokens.spec.ts`, baselines under
  `tests/visual-regression/__screenshots__/`. Covers six routes
  (`/kanban`, `/library`, `/library/plans`, `/library/decisions`,
  `/library/templates`, `/lifecycle/first-plan`) × light + dark themes,
  plus a `lifecycle-cluster-after-click` variant and a
  `prefers-color-scheme: dark` sanity case on `/library`. The
  `visual-regression` Playwright project runs before `chromium` with
  workers forced to 1.
- ADR-0026 governs token application conventions and should be
  consulted before introducing any exception to the
  `var(--atomic-X)` rewrite rule.
- 0033's AC1 (lines 113-114) explicitly excluded `--atomic-*`; this
  story closes that gap.

## Drafting Notes

- Frontmatter `type` renamed to `kind` to align with the work item
  template schema.
- User-story line framed for the design-system maintainer
  (component-author and brand-owner framings were also viable; this one
  emphasises the rearchitecture motive over the consumer motive).
- Corrected source provenance: previously cited
  `assets/tokens.css:34-105`, but the prototype is a single
  self-contained HTML file with `--atomic-*` declarations inline.
- AC2 rewrite rule is "exact normalised-hex match only" against the
  state of `global.css` on `main` at branch-cut; near-matches stay as
  literals and are documented in the PR rather than being snapped to
  the closest brand token. Normalisation (lowercase six-digit) is
  specified so two reviewers cannot produce different rewrite sets from
  the same source.
- AC1 pins the expected token set to `prototype-tokens.json` rather
  than a free-floating "~30 tokens" figure; the fixture is populated
  from the prototype and the fixture test enforces parity, so the
  expected set is frozen at story open rather than re-derived each
  review.
- ΔE acceptance: existing Playwright `toHaveScreenshot()` baselines
  must remain valid without `--update-snapshots`. Where regeneration is
  unavoidable, AC5 requires a per-pixel ΔE2000 < 5 justification using
  a CIEDE2000 implementation (e.g. `culori`'s `differenceCiede2000`)
  rather than an informal eyeball comparison.
- Drift-detector extension committed in-scope (AC4) rather than
  deferred, on the basis that the marginal cost is low and shipping
  brand tokens without drift coverage would leave a known gap.

## References

- Source: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`
  (inline `--atomic-*` declarations, ~line 183)
- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0033, 0077, 0082
- Convention: `meta/decisions/ADR-0026-css-design-token-application-conventions.md`
