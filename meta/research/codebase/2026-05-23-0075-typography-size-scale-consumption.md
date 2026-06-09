---
date: "2026-05-23T13:51:26+01:00"
researcher: Toby Clemson
revision: "0303ec8773f59a3c7ecaf875a3e6eb4c09ad92f1"
repository: accelerator
topic: "0075 Typography Size-Scale Consumption Reconciliation — codebase state"
tags: [research, codebase, typography, design-tokens, css, migration]
status: complete
last_updated: "2026-05-23T00:00:00+00:00"
last_updated_by: Toby Clemson
work_item_id: "0075"
type: codebase-research
id: "2026-05-23-0075-typography-size-scale-consumption"
title: "Research: 0075 Typography Size-Scale Consumption Reconciliation"
author: Toby Clemson
schema_version: 1
relates_to: ["design-gap:2026-05-21-current-app-vs-claude-design-prototype", "adr:ADR-0026", "work-item:0033", "work-item:0090"]
derived_from: ["codebase-research:2026-05-06-0033-design-token-system", "codebase-research:2026-05-21-0076-code-block-syntax-highlight-palette", "codebase-research:2026-05-21-0078-detail-page-frontmatter-table", "design-inventory:2026-05-21-015231-claude-design-prototype"]
---

# Research: 0075 Typography Size-Scale Consumption Reconciliation

**Date**: 2026-05-23 13:51 BST
**Researcher**: Toby Clemson
**Git Commit**: 0303ec8773f59a3c7ecaf875a3e6eb4c09ad92f1
**Branch**: HEAD
**Repository**: accelerator

## Research Question

What is the actual state of the visualiser frontend's `--size-*` typography
token scale and its consumers, and what does 0075's "consume tokens
everywhere" rule actually require in concrete terms?

## Summary

The work item's named outlier set is wrong on every dimension that matters
for scoping:

1. **The actual outlier count is ~35, not 4.** Nine `.module.css` files
   contain literal `font-size: <number>` declarations. The work item lists
   four (MarkdownRenderer H1, MarkdownRenderer body, Page eyebrow, Page
   subtitle); the other ~31 — across `FilterPill`, `SortPill`, `Sidebar`,
   `Brand`, `EmptyState`, `LibraryOverviewHub`, `LibraryTypeView` — are
   missed entirely. The work item's `Assumptions` explicitly commits Context
   to being the **exhaustive** inventory; that commitment is false.
2. **The MarkdownRenderer body `14.5px` outlier does not exist** in the
   current app. The `.markdown` selector sets no `font-size` and inherits;
   the only relative value is `0.88em` on inline `code:not(pre code)`.
   `14.5px` is a *prototype* value transcribed from the gap analysis.
3. **The AC2 grep paths reference `app/`**; the codebase uses `src/`. The
   greps as written match nothing.
4. **The scale-widening proposal is insufficient.** New tokens for `11px`
   and `13px` would absorb only a fraction of outliers. Real off-grid values
   in use also include `9.5px`, `10px`, `12.5px`, `1.75rem` (28px), and
   `0.88em` (relative). The current 12-token scale already accommodates
   `22px`, `12px`, `14px`, `10.5px`, `11.5px`, and `28px` — so several
   outliers are "off-grid" only by co-location, not by value.
5. **`src/styles/migration.test.ts` is an enforcement harness** with an
   `EXCEPTIONS` array that lists every current outlier as
   `kind: 'irreducible'` with a stated design reason. ADR-0026 §3
   formalises which literal categories belong in `EXCEPTIONS`. 0075
   effectively reverses these decisions for typography. Neither the work
   item nor its review acknowledges this harness or the ADR.

The Playwright `getComputedStyle` regression mechanism the work item
references already has three direct precedents in `tests/visual-regression/`
(`glyph-resolved-fill.spec.ts`, `chip-resolved-colours.spec.ts`,
`code-block-resolved-colours.spec.ts`) — no new tooling is required, and
the pattern transfers cleanly to `.fontSize`.

## Detailed Findings

### 1. The `--size-*` scale as it stands today

Defined in a single file:
`skills/visualisation/visualise/frontend/src/styles/global.css:126-142`
under the bare `/* Typography */` comment (no "eleven-step" wording exists
anywhere in the repo).

| Token            | Value     | Notes                                              |
|------------------|-----------|----------------------------------------------------|
| `--size-hero`    | `68px`    | Never consumed via `font-size`                     |
| `--size-h1`      | `48px`    | Never consumed via `font-size`                     |
| `--size-h2`      | `36px`    | Never consumed via `font-size`                     |
| `--size-h3`      | `28px`    | Consumed once — `Page.module.css:46` (page title)  |
| `--size-h4`      | `26px`    | Never consumed via `font-size`                     |
| `--size-lg`      | `22px`    | NoResultsPanel `.title`                            |
| `--size-body`    | `20px`    | KanbanBoard                                        |
| `--size-md`      | `18px`    | TopbarIconButton                                   |
| `--size-sm`      | `16px`    | Brand, MarkdownRenderer h3, NoResultsPanel, etc.   |
| `--size-xs`      | `14px`    | Widely consumed                                    |
| `--size-xxs`     | `12px`    | Widely consumed                                    |
| `--size-chip`    | `10.5px`  | Chip, MarkdownRenderer code chip                   |
| `--size-chip-md` | `11.5px`  | Chip, FrontmatterTable                             |

11 typographic steps + 2 chip-specific tokens = 13 total. The work item's
"eleven-step scale" wording refers to the 11 type-step subset.

Tokens that are *defined-but-not-consumed* via `font-size:` anywhere in the
app: `--size-hero`, `--size-h1`, `--size-h2`, `--size-h4`. Only `--size-h3`
is used (Page title) — every other heading-tier token has no in-app
consumer.

### 2. Actual outlier inventory (vs work item's stated inventory)

**Total: 35 `font-size: <literal>` declarations across 9 files** (Sweep:
`rg --glob '**/*.module.css' 'font-size:\s*[0-9]' src/`).

The work item names four outliers; the actual set is:

| File | Selectors / values |
| --- | --- |
| `src/components/MarkdownRenderer/MarkdownRenderer.module.css:9,53` | `.markdown h1` `1.75rem`; `.markdown code:not(pre code)` `0.88em` |
| `src/components/Page/Page.module.css:35,56` | `.eyebrow` `11px`; `.subtitle` `13px` |
| `src/components/Brand/Brand.module.css:28` | `.brandSub` `10px` |
| `src/components/Sidebar/Sidebar.module.css:89,107,143,160` | `.libraryHeading` `10.5px`; `.libraryHeadingHint` `10px`; `.sectionHeading` `10.5px`; `.phaseHeading` `9.5px` |
| `src/components/SortPill/SortPill.module.css:11,33,56` | `.trigger` `12px`; `.menuHeader` `10.5px`; `.menuItem` `12.5px` |
| `src/components/FilterPill/FilterPill.module.css:11,45,67,79,101,124-135,176,227,233` | `.trigger` `12px`; `.badge` `10px`; `.menuHeader` `10.5px`; `.clearButton` `11px`; `.facetHeading` `10.5px`; `.search input` `12px`; `.option` `12.5px`; `.optionCount` `11px`; `.noMatches` `11.5px` (9 hits) |
| `src/routes/library/EmptyState.module.css:62,81,89,101,110` | `.eyebrow` `11.5px`; `.title` `22px`; `.lede` `14px`; `.foot` `12px`; `.pathInline` `11.5px` |
| `src/routes/library/LibraryOverviewHub.module.css:12,94,100,106` | `.phaseHeading` `11px`; `.cardLabel` `14px`; `.cardCount` `11px`; `.cardLatest` `11.5px` |
| `src/routes/library/LibraryTypeView.module.css:17,27,45,51,60` | `.headerRow` `10.5px`; `.row` `13px`; `.firstCol` `12px`; `.slug` `11.5px`; `.mtime` `11.5px` |

**The work item also missed three `font:` shorthand outliers in Sidebar**
(lines 50, 75, 185): `font: 400 13px/1.5 …`, `font: 400 11px/1 …`,
`font: 400 13px/1.5 …`. These contain embedded font-size literals that the
AC2 grep regex (`font-size:\s*[0-9]`) will not catch — Pass-2 re-review
flagged this as a `minor / testability` suggestion but not as a blocker.

**The MarkdownRenderer body `14.5px` listed in Context does not exist.**
`MarkdownRenderer.module.css:1-5` defines `.markdown` with `line-height: 1.6`
and no `font-size`, so body text inherits from `body` (browser default,
~16px). The `14.5px` value is the *prototype's* body size, copied verbatim
from the gap analysis at `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md:73`
during work-item drafting. The work item's `Decisions` section commits to a
`14.5px → 14px` normalisation that has no current-app value to normalise.

### 3. Outlier values vs the existing scale

Of the 35 outlier values, several are already on-scale (off-grid by
co-location only, not by px value):

| Literal | Existing token | Outliers using it |
| --- | --- | --- |
| `22px` | `--size-lg` | EmptyState `.title` |
| `14px` | `--size-xs` | EmptyState `.lede`, LibraryOverviewHub `.cardLabel` |
| `12px` | `--size-xxs` | EmptyState `.foot`, LibraryTypeView `.firstCol`, SortPill `.trigger`, FilterPill `.trigger`/`.search input` |
| `10.5px` | `--size-chip` | LibraryTypeView `.headerRow`, Sidebar `.libraryHeading`/`.sectionHeading`, SortPill `.menuHeader`, FilterPill `.menuHeader`/`.facetHeading` |
| `11.5px` | `--size-chip-md` | EmptyState `.eyebrow`/`.pathInline`, LibraryOverviewHub `.cardLatest`, LibraryTypeView `.slug`/`.mtime`, FilterPill `.noMatches` |
| `1.75rem` (=28px) | `--size-h3` | MarkdownRenderer `h1` |

Genuinely off-grid (no current token at this value):

| Literal | Locations | Work-item proposal |
| --- | --- | --- |
| `9.5px` | Sidebar `.phaseHeading` | **Not addressed** |
| `10px` | Brand `.brandSub`, Sidebar `.libraryHeadingHint`, FilterPill `.badge` | **Not addressed** |
| `11px` | Page `.eyebrow`, LibraryOverviewHub `.phaseHeading`/`.cardCount`, FilterPill `.clearButton`/`.optionCount` | New `--size-eyebrow` |
| `12.5px` | SortPill `.menuItem`, FilterPill `.option` | **Not addressed** |
| `13px` | Page `.subtitle`, LibraryTypeView `.row` | New `--size-subtitle` |
| `0.88em` | MarkdownRenderer inline code | **Not addressed**; ADR-0026 §3 marks em-relative as permanently irreducible |

The work item's proposed scale extension (just `--size-eyebrow` 11px and
`--size-subtitle` 13px) leaves at least three off-grid pixel values
(`9.5px`, `10px`, `12.5px`) and the `0.88em` relative value without a
token. Under the AC2 "no numeric literals" rule, these would need either:
- (a) additional new tokens (`--size-9-5`, `--size-10`, `--size-12-5`, or
  similar; the existing chip-named convention doesn't generalise to these
  cases),
- (b) rounding to the nearest existing token, accepting some pixel drift
  (e.g. `10px → --size-chip` 10.5px; `9.5px → --size-chip` 10.5px;
  `12.5px → --size-xxs` 12px), or
- (c) an explicit carve-out for the `0.88em` em-relative case (consistent
  with ADR-0026 §3).

### 4. `migration.test.ts` — the existing enforcement harness

`src/styles/migration.test.ts` is a Vitest-driven CSS-literal harness with
a hand-curated `EXCEPTIONS` array. **Every typography outlier listed in
section 2 is already an explicit `EXCEPTIONS` entry** with
`kind: 'irreducible'` and a stated design reason. Sampled entries (from
the relevant grep on the file):

- `components/MarkdownRenderer/MarkdownRenderer.module.css` `1.75rem` —
  *"h1 font-size (28px) — 6px above size-lg ceiling; no heading token"*
- `components/MarkdownRenderer/MarkdownRenderer.module.css` `0.88em` —
  *"relative em font-size on inline code — not a rem scale value"*
- `components/Page/Page.module.css` `11px` — *"eyebrow font-size from
  design — 1px under --size-xxs (12px)"*
- `components/Page/Page.module.css` `13px` — *"subtitle font-size from
  design — 1px under --size-xs (14px)"*
- `components/Sidebar/Sidebar.module.css` `9.5px` — *"phase heading
  font-size from design — sub-pixel, below --size-xxs (12px)"*
- … and 30+ similar entries across the nine outlier files.

Two implications the work item does not address:

1. **The MarkdownRenderer H1 reason is now stale.** The harness comment says
   "no heading token", but `--size-h3` was added in 0033 and equals 28px
   exactly. The literal is reducible now; the EXCEPTIONS entry should be
   removed, not kept.
2. **0075's consume-everywhere rule is a wholesale reversal of EXCEPTIONS
   for typography.** Either the EXCEPTIONS entries for every outlier file
   are deleted as part of the migration, or `migration.test.ts` is reworked
   so it stops accepting typography literals as a category. The work item
   does not name this change and does not include it in scope.

### 5. ADR-0026 — the convention 0075 supersedes

`meta/decisions/ADR-0026-css-design-token-application-conventions.md` is
the canonical decision governing token consumption. Relevant clauses:

- **§2 Typography tolerance bands (line 108–118):** "substitute with the
  nearest `--size-*` token when pixel drift is within ±2px… `em`-based
  values are structurally irreducible regardless of drift". Under this
  rule, several outliers should already be substituted (`9.5px → --size-chip`
  10.5px is 1px drift, qualifies; `10px → --size-chip` 10.5px is 0.5px
  drift, qualifies). They are currently in `EXCEPTIONS` anyway — the
  harness is being applied more strictly than the ADR mandates.

- **§3 Irreducible literal categories (line 121–135):**
  - *"em-relative font-sizes"* → `0.88em` permanently irreducible. 0075
    must either grant this an explicit carve-out or break the ADR.
  - *"Heading font-sizes above `size-lg`"* → `1.75rem`. This category was
    correct in 0033 but obsolete now that `--size-h3` exists.
  - *Negative consequence* (line 287–290): "The heading font-size gap (no
    token above `--size-lg`) is deferred to a future type-scale extension;
    irreducible heading sizes accumulate in EXCEPTIONS until then." 0075 is
    that future extension, partially (`--size-h1`…`--size-h4` already
    landed in 0033; 0075 is the consumption follow-through).

- **ADR-0026 is not referenced anywhere in work item 0075 or its review.**
  This is the single most relevant prior decision and must be either
  superseded or amended by 0075's landing.

### 6. Playwright harness for the AC computed-style check

The work item's AC requires capturing `getComputedStyle(...).fontSize` per
migrated selector. The harness is already in place at
`skills/visualisation/visualise/frontend/`:

- Config: `playwright.config.ts` (one project; default viewport set there).
- Test root: `tests/visual-regression/` (style assertions + screenshot
  baselines), with `e2e/` for functional specs.
- Three existing specs use the exact pattern the AC requires:
  - `tests/visual-regression/chip-resolved-colours.spec.ts`
  - `tests/visual-regression/glyph-resolved-fill.spec.ts`
  - `tests/visual-regression/code-block-resolved-colours.spec.ts`
  Each calls `locator.evaluate((el) => getComputedStyle(el).color)` (or
  `.backgroundColor`, `.borderTopColor`); `.fontSize` slots in unchanged.
- `README.md:82` documents the recommended `page.waitForFunction`
  wait-pattern for `getComputedStyle`-based assertions after theme/font
  changes.
- The Pass-2 re-review flagged that the AC does not name a specific
  viewport or route — for `1.75rem` (a relative unit), the computed px
  depends on root font-size at the render context. This needs resolving in
  the plan: either pin the viewport+route, or assert `'28px'` exactly
  (independent of viewport) because `1.75rem × 16px = 28px` is fixed.

### 7. Related work items and dependencies

From `meta/work/` (paths abbreviated):

| ID | Title | Status | Relationship to 0075 |
| --- | --- | --- | --- |
| 0033 | Design Token System | done | Defined the `--size-*` scale; the basis 0075 builds on. |
| 0073 | Atomic Brand-Layer Palette | draft | Sibling token-system expansion (colours). |
| 0074 | Per-Doc-Type Hues on Detail Page | draft | Sibling colour-token consumption. |
| 0076 | Code-Block Syntax-Highlight Tokens | ready | **Depends on 0075** per Dependencies; will rebase onto the rationalised `<pre>` CSS. |
| 0077 | Shadow and Dark-Accent Token Audit | draft | Sibling audit. |
| 0090 | Radius Tokens Consumption | draft | **Explicitly mirrors 0075's pattern** for `border-radius` — the radius outliers (`RelatedArtifacts` 2px, `<pre>` 6px) carved out of 0075 live here. |

Pass-2 work-item review (`meta/reviews/work/0075-typography-size-scale-consumption-review-1.md`)
verdict is **APPROVE** with several suggestion-grade open items:
- Playwright tooling named in Technical Notes but not in Dependencies.
- Viewport / route for the snapshot not pinned.
- Tension between "single PR series" and "exhaustive outlier inventory"
  if the grep surfaces surprises (which it now has — see section 2).
- AC grep regex `[0-9]` misses leading-dot literals (`.875rem`), `font:`
  shorthand, and `calc()`/`clamp()` embedding.

### 8. Where the migration would actually touch

Files that must change to satisfy AC2 (zero `font-size:\s*[0-9]` hits):

- `src/styles/global.css` (add tokens + comment per AC3).
- `src/styles/migration.test.ts` (delete the 30+ typography EXCEPTIONS
  entries; possibly rework the harness to forbid font-size literals at
  the category level).
- 9 component/route CSS modules listed in section 2.
- Plus 3 `font:` shorthand sites in `Sidebar.module.css` (lines 50, 75,
  185) if the regex is broadened — currently undetected by AC2.
- Possibly `MarkdownRenderer.module.css:53` (`0.88em`) depending on the
  em-relative carve-out decision.

Tests that may need updating:
- `src/styles/migration.test.ts` (EXCEPTIONS curation as above).
- Existing component-level CSS tests that may assert specific computed
  font-sizes (not surveyed — worth a sweep during planning).
- New Playwright `getComputedStyle` regression spec under
  `tests/visual-regression/typography-resolved-sizes.spec.ts` or similar.

ADR work:
- ADR-0026 must be amended (or superseded by a new ADR) to either:
  - Promote "consume tokens everywhere" from "tolerance band" to
    "mandatory" for typography, removing the irreducible categories that
    no longer apply (heading sizes), or
  - Record the explicit carve-outs (em-relative font-sizes; any drift
    accepted on the rounded values).

## Code References

- `skills/visualisation/visualise/frontend/src/styles/global.css:126-142` —
  `--size-*` scale source of truth.
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:67-222` —
  EXCEPTIONS array; every typography outlier currently listed
  `kind: 'irreducible'`.
- `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css:9` —
  `.markdown h1 { font-size: 1.75rem; }` (no body `14.5px` outlier exists).
- `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css:53` —
  inline code `font-size: 0.88em` (em-relative; ADR-0026 §3 irreducible).
- `skills/visualisation/visualise/frontend/src/components/Page/Page.module.css:35,46,56` —
  `.eyebrow` 11px, page title `var(--size-h3)`, `.subtitle` 13px.
- `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.module.css:50,75,89,107,143,160,185` —
  4 `font-size:` outliers + 3 `font:` shorthand outliers.
- `skills/visualisation/visualise/frontend/src/components/FilterPill/FilterPill.module.css` —
  9 outliers (largest single concentration).
- `skills/visualisation/visualise/frontend/tests/visual-regression/chip-resolved-colours.spec.ts` —
  `getComputedStyle().color` precedent; `.fontSize` substitutes
  unchanged.
- `meta/decisions/ADR-0026-css-design-token-application-conventions.md:108-135` —
  Typography tolerance band (§2) and irreducible categories (§3) that
  0075 will modify.

## Architecture Insights

- **The current state has an internally consistent rule**: ADR-0026 says
  "use tokens when within ±2px, otherwise mark irreducible". The harness
  enforces it via EXCEPTIONS. The work item proposes replacing that rule
  with a stricter "always use tokens; widen the scale to fit". The
  trade-off is more tokens vs fewer literals — the work item's analysis
  doesn't surface this trade-off because it treats the existing scale as
  defined-but-not-consumed (it isn't — it's defined-and-partially-consumed
  with documented carve-outs).
- **Tokens fall into two functional layers** within `--size-*`: the
  11-step "type" scale (`hero`…`xxs`) and the 2-token "chip" scale
  (`chip` 10.5px, `chip-md` 11.5px). Several outliers exactly match chip
  tokens but live outside chip components (e.g. Sidebar headings at
  10.5px). The migration choice on these is "rename `chip` to something
  more semantic" vs "leave naming and let any small-text component
  consume `--size-chip`".
- **Heading tokens are nearly all dead.** `--size-hero`, `--size-h1`,
  `--size-h2`, `--size-h4` have zero `font-size:` consumers. Only
  `--size-h3` (28px) is used (Page title). The scale was sized for
  prototype-style use but the current app prefers `--size-lg` (22px) and
  smaller for page-level type. 0075's "consume tokens everywhere" rule
  doesn't change this fact; the heading tokens remain unused after
  migration unless the new MarkdownRenderer H1 → `--size-h3` assignment
  becomes the second consumer.
- **The `font:` shorthand is a known gap in the AC regex.** Sidebar uses
  it three times. If the migration leaves these untouched, the codebase
  contains hidden font-size literals that AC2 cannot detect.

## Historical Context

- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md:65-79` —
  The source of the work item's outlier list. Misidentifies the
  MarkdownRenderer body `14.5px` as a current-app outlier (it's actually
  the prototype's value); the work item inherited that error.
- `meta/decisions/ADR-0026-css-design-token-application-conventions.md` —
  Canonical token-application conventions (May 2026); 0075 supersedes
  parts of §2 and §3 for typography.
- `meta/work/0033-design-token-system.md` (status: done) — Introduced the
  full token system and `migration.test.ts`. Status note: 0033's
  acceptance was *"hardcoded literals replaced with tokens where a
  semantic match exists OR explicit EXCEPTIONS recorded"*, so the
  EXCEPTIONS-array approach is the current contract.
- `meta/work/0090-radius-tokens-consumption.md` (status: draft) —
  Sibling pattern for radius; will share whatever scaffolding 0075
  introduces (e.g. `migration.test.ts` rule changes).

## Related Research

- `meta/research/codebase/2026-05-06-0033-design-token-system.md` —
  Original token system research.
- `meta/research/codebase/2026-05-21-0076-code-block-syntax-highlight-palette.md` —
  Downstream consumer of 0075's `<pre>` rationalisation.
- `meta/research/codebase/2026-05-21-0078-detail-page-frontmatter-table.md` —
  Touches `--size-chip-md` consumption.
- `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/inventory.md` —
  Prototype reference values (source of the gap analysis).

## Open Questions

1. **Does 0075 absorb the additional ~30 outliers, or only the four
   named?** The work item's `Assumptions` says Context is the exhaustive
   inventory; the codebase says otherwise. Three possible resolutions:
   - (a) Expand 0075 scope to the actual 35-outlier set (single big PR
     series; may push the work item from "story" into "epic" territory).
   - (b) Split: 0075 absorbs the four named outliers + the AC2 grep
     enforcement; a follow-up work item per component cluster handles the
     rest, accepting an interim state where AC2 fails until follow-ups
     land.
   - (c) Narrow 0075 to scale-widening only; defer the consume-everywhere
     migration entirely. (Defeats the work item's stated purpose.)
2. **What is the policy on the `0.88em` em-relative font-size?** ADR-0026
   §3 says em-relative is permanently irreducible. AC2 grep
   (`font-size:\s*[0-9]`) does not match `0.88em`, so the AC passes today
   for that literal. Decision needed: explicit carve-out documented in
   the migration, or rewrite the renderer to use a px-based value?
3. **What is the policy on the `font:` shorthand in Sidebar?** AC2 grep
   misses it. Either broaden the regex (and migrate the three sites) or
   accept the gap as a known limitation.
4. **Which `--size-h*` tokens stay and which are dropped?** `--size-hero`,
   `--size-h1`, `--size-h2`, `--size-h4` have zero consumers. If 0075
   commits to "consume tokens everywhere", the inverse — "drop
   never-consumed tokens" — is the obvious counterpart. The work item
   doesn't address this.
5. **How does the migration interact with `migration.test.ts` EXCEPTIONS?**
   The work item does not name this file or its 30+ typography entries.
   The plan must explicitly include either (a) deletion of EXCEPTIONS
   entries as migration proceeds, or (b) a category-level rule change so
   typography literals can no longer be exception-listed.
6. **How is ADR-0026 amended/superseded?** The current ADR is
   `accepted`. 0075's rule replaces parts of §2 and §3. Either ADR-0026
   gets an amendment entry, or a new ADR is filed documenting the
   replacement. This should be a planned deliverable, not an
   afterthought.
7. **What new tokens are needed beyond `--size-eyebrow` and
   `--size-subtitle`?** Options for the off-grid values `9.5px`, `10px`,
   `12.5px`:
   - Add four more named tokens (proliferation).
   - Rename the `--size-chip*` family to general small-text tokens and
     bridge gaps with one or two new tokens (e.g. `--size-2xs` for 10px).
   - Round to nearest existing token (introduces deliberate drift; needs
     ADR coverage).
