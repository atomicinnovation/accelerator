---
work_item_id: "0075"
title: "Typography Size-Scale Consumption Reconciliation"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
type: story
status: ready
priority: medium
parent: ""
tags: [design, frontend, tokens, typography]
---

# 0075: Typography Size-Scale Consumption Reconciliation

**Type**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Adopt token consumption as the single canonical rule for the typography
size scale: every `font-size` declaration in current-app component CSS
resolves to a `var(--size-*)` token. Widen the scale with five new tokens
to cover off-grid values, expand `--size-*` consumption across all 35
outliers, retire the `migration.test.ts` typography EXCEPTIONS entries
that the rule supersedes, and amend ADR-0026 so the new rule replaces the
prior tolerance-band convention.

## Context

**Terms used below.** *Current app* refers to the production frontend
under `skills/visualisation/visualise/frontend/src/`. *Prototype* refers
to the investigative HTML snapshot at
`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`,
captured for the 2026-05-21 gap analysis and not a modification target.
A *deliberate-drift screenshot* is a screenshot captured into the PR
description to document a knowingly-introduced visual change so
reviewers can sanity-check it.

The brand size scale (`--size-hero` … `--size-chip-md`) is defined in
both the current app and the prototype stylesheets at identical px
values. Consumption in the current app diverges sharply: roughly two
thirds of `font-size` declarations resolve to `var(--size-*)`, but 35
declarations across 9 component and route CSS modules use literal px or
rem values. The prototype is an investigative snapshot, not a
modification target — its hard-codes are evidence of the inconsistency
this work item resolves on the current-app side.

The full current-app outlier inventory (sweep performed 2026-05-23,
research at
`meta/research/codebase/2026-05-23-0075-typography-size-scale-consumption.md`):

- `MarkdownRenderer` — `.markdown h1` `1.75rem` (28px),
  `.markdown code:not(pre code)` `0.88em` (em-relative)
- `Page` — `.eyebrow` `11px`, `.subtitle` `13px`
- `Brand` — `.brandSub` `10px`
- `Sidebar` — `.libraryHeading` `10.5px`, `.libraryHeadingHint` `10px`,
  `.sectionHeading` `10.5px`, `.phaseHeading` `9.5px`; plus three `font:`
  shorthand sites containing embedded font-size literals: `.searchInput`
  (`13px`), `.kbd` (`11px`), `.link` (`13px`) — not caught by the
  `font-size:` grep, addressed by AC2's third sweep. Line numbers
  50/75/185 are a 2026-05-23 snapshot; the AC2 third sweep is the
  authoritative locator
- `SortPill` — `.trigger` `12px`, `.menuHeader` `10.5px`, `.menuItem`
  `12.5px`
- `FilterPill` — `.trigger` `12px`, `.badge` `10px`, `.menuHeader`
  `10.5px`, `.clearButton` `11px`, `.facetHeading` `10.5px`,
  `.search input` `12px`, `.option` `12.5px`, `.optionCount` `11px`,
  `.noMatches` `11.5px`
- `EmptyState` (library route) — `.eyebrow` `11.5px`, `.title` `22px`,
  `.lede` `14px`, `.foot` `12px`, `.pathInline` `11.5px`
- `LibraryOverviewHub` — `.phaseHeading` `11px`, `.cardLabel` `14px`,
  `.cardCount` `11px`, `.cardLatest` `11.5px`
- `LibraryTypeView` — `.headerRow` `10.5px`, `.row` `13px`, `.firstCol`
  `12px`, `.slug` `11.5px`, `.mtime` `11.5px`

Of the 35 literal `font-size:` declarations, several are already on-scale
by value (off-grid only by co-location): `22px = --size-lg`,
`14px = --size-xs`, `12px = --size-xxs`, `10.5px = --size-chip`,
`11.5px = --size-chip-md`, `1.75rem = --size-h3`. The genuinely off-grid
values are `9.5px`, `10px`, `11px`, `12.5px`, `13px`, and the relative
`0.88em`.

## Requirements

- Adopt the **consume-tokens-everywhere rule** as canonical: every
  `font-size` declaration (literal or embedded in a `font:` shorthand) in
  current-app CSS — component modules and global stylesheets under
  `skills/visualisation/visualise/frontend/src/` — must resolve to a
  `var(--size-*)` reference. No literal px, rem, or em `font-size` values.
- Widen the `--size-*` scale by adding five new tokens for the off-grid
  values (slotted into the existing scale by px ordering):
  - `--size-4xs: 9.5px`
  - `--size-3xs: 10px`
  - `--size-eyebrow: 11px`
  - `--size-row: 12.5px`
  - `--size-subtitle: 13px`
- Migrate every outlier listed in Context onto the scale, including the
  three `font:` shorthand sites in `Sidebar.module.css` (lines 50 / 75 /
  185 — expand each shorthand into its constituent properties so the
  font-size literal disappears).
- Migrate the `MarkdownRenderer` inline `<code>` size from `0.88em` to
  `var(--size-xs)` — see Decisions for rationale. This changes inline
  `<code>` to a fixed size rather than scaling with its enclosing
  element; the visual delta is documented in Decisions.
- Retire `migration.test.ts` EXCEPTIONS entries that the new rule
  supersedes: every entry with `kind: 'irreducible'` whose `literal`
  appears in a `font-size:` declaration of one of the migrated files. The
  remaining typography entries to keep are those tied to non-`font-size`
  uses of the literal (e.g. padding) — those entries' `reason` strings
  must be updated to drop the "font-size from design" framing.
- Amend ADR-0026:
  - §2 typography tolerance band → replaced for `font-size:` by the new
    consume-tokens-everywhere rule (the ±2px band remains in force for
    spacing).
  - §3 irreducible categories → drop "em-relative font-sizes" and
    "Heading font-sizes above `size-lg`" rows; both no longer apply.
  - Consequences (line 287–290) "heading font-size gap" deferral → mark
    as resolved by 0075.
- Land the migration as a **single atomic PR**. All token additions,
  outlier migrations, `migration.test.ts` cleanup, ADR-0026 amendment,
  and the new Playwright spec ship together. No intermediate state on
  main where consumption is partial.
- Document the canonical rule in the PR description (with the verbatim
  rule statement and reasoning).
- Add the canonical comment to the tokens stylesheet (verbatim wording
  in AC6 below) so the rule is discoverable from `global.css`.

## Acceptance Criteria

- [ ] **AC1.** Five new tokens are present in `src/styles/global.css`
  slotted into the `--size-*` block by px ordering:
  - `--size-4xs: 9.5px`
  - `--size-3xs: 10px`
  - `--size-eyebrow: 11px`
  - `--size-row: 12.5px`
  - `--size-subtitle: 13px`
- [ ] **AC2.** All three of the following ripgrep sweeps return zero
  matches when run from `skills/visualisation/visualise/frontend/`:
  - `rg --glob '**/*.module.css' 'font-size:\s*[.0-9]' src`
  - `rg --glob '**/*.css' --glob '!**/global.css' 'font-size:\s*[.0-9]' src`
  - `rg --glob '**/*.module.css' 'font:\s*[^;]*\s[.0-9]+(px|rem|em)' src`
  (The third sweep catches `font:` shorthand sites with embedded size
  literals.)
- [ ] **AC3.** Every outlier enumerated in Context consumes a
  `var(--size-*)` token in the migrated file.
- [ ] **AC4.** `src/styles/migration.test.ts` EXCEPTIONS cleanup:
  - [ ] **AC4a.** Every `EXCEPTIONS` entry whose `literal` appears in
    a `font-size:` declaration of one of the migrated files is deleted.
  - [ ] **AC4b.** No remaining `EXCEPTIONS` entry's `reason` field
    contains the substring `font-size` (grep-able pass condition).
- [ ] **AC5.** ADR-0026 is amended in place to address each of:
  - [ ] **AC5a.** The consume-tokens-everywhere rule for `font-size` is
    documented as the canonical rule (replacing §2's tolerance band for
    typography).
  - [ ] **AC5b.** The "em-relative font-sizes" and "Heading font-sizes
    above `size-lg`" rows in §3 are removed (or marked superseded).
  - [ ] **AC5c.** The Consequences section's "heading font-size gap"
    deferral note is marked resolved by 0075 (locate by content, not by
    line number — line numbers shift as part of the same amendment).
- [ ] **AC6.** Documentation deliverables:
  - [ ] **AC6a.** `src/styles/global.css` carries the verbatim comment
    `/* font-size consumers: use these tokens — see ADR-0026 (as
    amended by 0075) */` above the `--size-*` block.
  - [ ] **AC6b.** The PR description contains both (i) the canonical
    rule statement *"every `font-size` declaration in current-app CSS
    must resolve to a `var(--size-*)` token"* and (ii) a one-paragraph
    rationale that references `ADR-0026` by ID.
- [ ] **AC7.** Computed `font-size` regression check via a Playwright
  `getComputedStyle` spec at
  `tests/visual-regression/typography-resolved-sizes.spec.ts`. For each
  selector below, the spec asserts the post-migration computed
  `font-size` equals the expected px value at the default viewport (set
  in `playwright.config.ts`) on a route that mounts the component:
  - `MarkdownRenderer` H1 → `28px`
  - `MarkdownRenderer` inline code → `14px` (was `0.88em`; see Decisions)
  - `Page` `.eyebrow` → `11px`
  - `Page` `.subtitle` → `13px`
  - `Sidebar` `.phaseHeading` → `9.5px`
  - `Brand` `.brandSub` → `10px`
  - `SortPill` `.menuItem` → `12.5px`
  - `FilterPill` `.option` → `12.5px`
  - `EmptyState` `.title` → `22px`
  - `LibraryTypeView` `.row` → `13px`

  **Selection rule for the selector list:** one selector per outlier
  file group plus the two value-transition cases (inline code
  `0.88em` → `14px`; MarkdownRenderer H1 `1.75rem` → `28px` as the
  relative-unit case). Remaining outliers are covered by AC2; AC7
  provides the per-value regression guarantee for representative
  cases. If a future maintainer adds an outlier file group, extend the
  spec by the same rule.

## Decisions

- **Inline code (`MarkdownRenderer`) moves from `0.88em` to `--size-xs`
  (`14px`).** `0.88em` evaluates against the current parent font-size,
  so inline `<code>` currently scales with its surrounding element
  (larger inside an `<h2>`, smaller inside body text). Pinning to
  `--size-xs` freezes it at one size. The trade-off is accepted to
  eliminate the em-relative outlier; deliberate-drift screenshots
  documenting `<code>` in headings are part of the PR description.
- **Existing `--size-chip` (10.5px) and `--size-chip-md` (11.5px) are
  reused outside chip components.** Sidebar `.libraryHeading` /
  `.sectionHeading`, SortPill `.menuHeader`, FilterPill `.menuHeader` /
  `.facetHeading`, LibraryTypeView `.headerRow` consume `--size-chip`;
  EmptyState `.eyebrow` / `.pathInline`, LibraryOverviewHub `.cardLatest`,
  LibraryTypeView `.slug` / `.mtime`, FilterPill `.noMatches` consume
  `--size-chip-md`. Renaming the chip tokens is out of scope — they are
  re-conceptualised as "small-text" tokens that chips happen to consume.
- **Dead heading tokens (`--size-hero`, `--size-h1`, `--size-h2`,
  `--size-h4`) are kept** even though no `font-size` consumer references
  them after migration. They remain available for future component work;
  the AC2 grep only fails on literals, not on unused tokens. The
  defined-but-not-consumed concern flagged in the original work item is
  acknowledged but accepted as defensive scaffolding for the next
  heading-tier component.
- **The five new tokens use mixed semantic/numeric naming:** `--size-4xs`
  and `--size-3xs` extend the numeric scale below `--size-xxs`;
  `--size-eyebrow`, `--size-row`, and `--size-subtitle` are semantic
  names because they have clear single-purpose consumers (eyebrow line,
  list rows, page subtitle).
- **`Sidebar` `font:` shorthand sites are expanded into individual
  properties** rather than left as shorthand, so the literal cannot hide
  inside a compound declaration.

## Dependencies

- Blocked by: 0033 (size scale tokens defined).
- Blocks:
  - 0076 (code-block syntax-highlight palette — rebases onto the
    rationalised `<pre>` CSS once this migration lands; see 0076's
    Dependencies for the fallback sequence).
  - 0090 (radius tokens consumption — pattern-reuse coupling: 0090
    adopts this work item's canonical rule wording, EXCEPTIONS
    retirement pattern, and ADR amendment style verbatim for
    `border-radius`. 0090 must not begin implementation until 0075
    lands, so the pattern is established rather than re-invented).
- Tooling: Playwright harness at
  `skills/visualisation/visualise/frontend/tests/visual-regression/`
  (existing). The new `typography-resolved-sizes.spec.ts` follows the
  pattern of `chip-resolved-colours.spec.ts`,
  `glyph-resolved-fill.spec.ts`, and `code-block-resolved-colours.spec.ts`.
- Convention: ADR-0026 (`meta/decisions/ADR-0026-css-design-token-application-conventions.md`)
  must be amended as part of the same PR.

## Assumptions

- The chosen rule applies uniformly to current-app CSS; no per-component
  carve-outs.
- The current outlier inventory listed in Context is a 2026-05-23
  snapshot captured by the research sweep referenced in Dependencies. The
  AC2 grep sweep is the merge-time enforcement against drift between
  authoring and merge.
- Pre- and post-migration computed `font-size` values are identical for
  every selector whose outlier value already matched an existing token
  exactly (22px, 14px, 12px, 10.5px, 11.5px, 28px). Pre- and
  post-migration values are also identical for selectors moving to new
  tokens at the same px value (9.5px → `--size-4xs`, 10px → `--size-3xs`,
  11px → `--size-eyebrow`, 12.5px → `--size-row`, 13px → `--size-subtitle`).
  The single deliberate-drift case is `MarkdownRenderer` inline code
  (`0.88em` → `--size-xs` 14px) documented under Decisions.
- The prototype HTML/CSS (`src/styles/fixtures/prototype-tokens.json` and
  associated drift test) does not assert on `--size-*` tokens, so adding
  five new size tokens will not break
  `prototype-tokens.fixture.test.ts`.
- Radius outliers (`RelatedArtifacts` badge `2px`, markdown `<pre>` `6px`)
  are out of scope — they are radius, not typography size, and are
  tracked under 0090.

## Technical Notes

- Tokens stylesheet:
  `skills/visualisation/visualise/frontend/src/styles/global.css:126-142`.
- Enforcement harness:
  `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`.
  Currently lists every outlier as `kind: 'irreducible'` with a
  `font-size from design` reason. Entries to retire are enumerated in the
  Requirements section.
- Playwright harness lives at
  `skills/visualisation/visualise/frontend/tests/visual-regression/`.
  Existing computed-style precedents:
  - `chip-resolved-colours.spec.ts`
  - `glyph-resolved-fill.spec.ts`
  - `code-block-resolved-colours.spec.ts`
  The new spec follows the same `locator.evaluate((el) => getComputedStyle(el).fontSize)`
  pattern.
- Default viewport set in
  `skills/visualisation/visualise/frontend/playwright.config.ts`. For
  selectors using relative units (e.g. `1.75rem` pre-migration), assert
  on the computed px value (`28px`), which is viewport-independent.
- Scale widening should preserve the existing naming convention (numeric
  for `xxs` → `xs`-style extensions; semantic for component-keyed
  tokens). New tokens slot in by px ordering, not appended arbitrarily.
- 0033 introduced the eleven-step `--size-*` scale (plus two chip
  tokens); that scope did not enforce uniform consumption.

## Drafting Notes

- Chose the consume-tokens-everywhere rule over dropping the scale
  because 0033 is `done`, sibling work items (0073, 0074, 0076, 0077)
  expand the token system rather than contract it, and a token system
  that is defined-but-not-consumed contradicts its own purpose.
- Constrained scope to current-app CSS only — the prototype is an
  investigative snapshot, not a modification target.
- Stripped radius items (`RelatedArtifacts` badge `2px`, markdown
  `<pre>` `6px`) from scope — they are radius, not typography size,
  and are now tracked under 0090.
- 2026-05-23 scope expansion: original draft committed Context to a
  4-outlier inventory and four outlier files; the actual codebase
  sweep returned 35 outliers across 9 files. Context, Requirements,
  and Acceptance Criteria rewritten to reflect the true inventory.
  Scale widening grew from 2 to 5 new tokens, and the migration now
  also touches `migration.test.ts` EXCEPTIONS and ADR-0026 §2/§3.
- Kept `type: story` despite the expanded surface. The migration is a
  single atomic PR with a single guiding rule; the increased file count
  is mechanical, not conceptual. **Contingency only**: if the PR proves
  genuinely unreviewable in one piece, escalate by re-scoping this work
  item to an epic with child stories one per outlier file group
  (MarkdownRenderer + Page, FilterPill + SortPill, Sidebar, library
  routes), with the first child landing rule + tokens + harness + ADR
  and remaining children landing per cluster. This is a fallback path,
  not the planned-of-record delivery shape.
- Kept `priority: medium` — no external deadline; this is trajectory
  work, not a blocker.
- Status moved back to `draft` because the 2026-05-23 rewrite is a
  substantive scope change since the Pass-2 APPROVE review; a Pass-3
  re-review is warranted before implementation kicks off.

## References

- Source gap analysis: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Codebase audit: `meta/research/codebase/2026-05-23-0075-typography-size-scale-consumption.md`
- Convention to amend: `meta/decisions/ADR-0026-css-design-token-application-conventions.md`
- Related work items: 0033, 0073, 0074, 0076, 0077, 0090
