---
work_item_id: "0090"
title: "Radius Tokens Consumption"
date: "2026-05-23T00:00:00+00:00"
author: Toby Clemson
type: story
status: ready
priority: low
parent: ""
tags: [design, frontend, tokens, radius]
---

# 0090: Radius Tokens Consumption

**Type**: Story
**Status**: Ready
**Priority**: Low
**Author**: Toby Clemson

## Summary

Adopt token consumption as the canonical rule for radius: every
`border-radius` declaration in current-app component CSS resolves to a
`var(--radius-*)` token. Extend the existing 0033 radius scale with the few
steps current-app values require (preserving pixel values exactly), retire
the `migration.test.ts` radius EXCEPTIONS the rule supersedes, and amend
ADR-0026 plus create a new radius-rule ADR — mirroring the
consume-tokens-everywhere pattern 0075 established for typography sizes.

## Context

0075 carved radius outliers out of its consumption-reconciliation scope.
Two known current-app outliers are documented there:

- `RelatedArtifacts` badge `border-radius: 2px`
- Markdown `<pre>` `border-radius: 6px`

0033 (Design Token System, done) already ships a four-step radius scale —
`--radius-sm: 4px`, `--radius-md: 8px`, `--radius-lg: 12px`,
`--radius-pill: 999px` — so the scale itself exists; the work here is
consumption plus the handful of new steps the outliers need. Neither
known outlier (2px, 6px) lands on an existing step.

ADR-0026 §3 ("Irreducible literal categories") currently classifies the
6px `<pre>` radius as **irreducible** — listed verbatim as "In-between
border radii: `6px`, between `--radius-sm` (4px) and `--radius-md` (8px)"
— and §3 records such literals in the `migration.test.ts` EXCEPTIONS
harness with `kind: 'irreducible'`. The consume-tokens rule this story
adopts directly supersedes that classification for radius, so the ADR
clause and its EXCEPTIONS entry must be retired as part of the work —
exactly as 0075 retired ADR-0026's typography rows and their EXCEPTIONS
entries. (The §3 "Border / outline widths: 1px, 2px" row is border
*widths*, not radius, and is unaffected.)

Beyond the two known outliers, the full current-app radius inventory has
not been enumerated. That enumeration — and the pre-migration computed
values the regression check asserts against — is produced by the
codebase research that precedes planning for this story; it is not
embedded here.

## Requirements

- Adopt the **consume-tokens-everywhere rule** for radius as canonical:
  every `border-radius` declaration in current-app CSS under `src/`
  (module and non-module `.css`; the shorthand or the four longhand corner
  properties) must resolve to a `var(--radius-*)` reference. No literal px
  or rem radius values remain.
- Extend the existing 0033 `--radius-*` scale with the steps needed to
  cover current-app values exactly, preserving current pixel values (no
  visual change). Known additions:
  - `--radius-xs: 2px` (scale-based name)
  - `--radius-block: 6px` (use-case-named exception, named for the code-block
    `<pre>` surface it serves — see naming note)
  Any further off-scale values surfaced by the pre-implementation research
  each get a token and a naming decision under the same rules.
- Migrate every current-app `border-radius` declaration onto the scale.
- Retire the `migration.test.ts` EXCEPTIONS entries the new rule
  supersedes — at minimum the "in-between border radii (6px)" irreducible
  entry, plus any other radius literal recorded as irreducible (mirrors
  0075's EXCEPTIONS cleanup).
- ADR work (mirroring 0075's ADR-0036 + ADR-0026 amendment):
  - Create a new ADR codifying the radius consume-tokens-everywhere rule
    (its ADR ID is allocated at creation time; referred to throughout this
    work item as "the new radius ADR").
  - Amend ADR-0026 §3 to remove or mark superseded the "In-between border
    radii: 6px" irreducible row, recording its supersession by the new ADR.
- Add a single CI grep gate that fails the build on any literal px/rem
  radius value in current-app CSS, covering `border-radius` and the four
  longhand corner properties (exact pattern in Acceptance Criteria).
- Add a Playwright `getComputedStyle` regression spec asserting each
  migrated selector's computed `border-radius` equals its recorded
  pre-migration px value, following the existing `*-resolved-*` spec
  pattern.
- Document the canonical rule: a comment above the `--radius-*` block in
  `global.css` referencing the new ADR, and the verbatim rule plus a
  one-paragraph rationale in the PR description.
- Land as a single atomic PR — token additions, outlier migrations,
  `migration.test.ts` cleanup, ADR-0026 amendment, new ADR, and the
  Playwright spec ship together. No intermediate state on main where
  consumption is partial.

**Naming note.** *Scale-based* names are the `sm`/`md`/`lg`/`pill` ladder
from 0033 (extended below `sm` with `xs`). A *use-case* name
(`--radius-block`) is used only where a value has no slot on that ladder.

## Acceptance Criteria

- **AC1.** The `--radius-*` block in `global.css` contains, at minimum,
  `--radius-xs: 2px` and `--radius-block: 6px`, slotted by px ordering, and
  one token at the exact px value of every radius value listed in the
  research inventory table (once it lands). Completeness for any value not
  individually enumerated is delegated to AC3 — an unmigrated off-scale
  literal would fail AC3's zero-match gate.
- **AC2.** A Playwright `getComputedStyle` regression spec contains one
  assertion per selector in the research inventory's radius table (the
  inventory is attached to the plan before AC2 is evaluated), each
  asserting the post-migration computed `border-radius` equals that
  selector's recorded pre-migration px value, read at the
  `playwright.config.ts` default viewport on a route that mounts the
  component (mirroring 0075 AC7). "No computed-value change" is discharged
  entirely by these per-selector exact-px assertions — this story declares
  no deliberate drift, so no separate screenshot tolerance applies. AC3
  remains the completeness backstop for any selector not individually
  enumerated.
- **AC3 (self-sufficient completeness gate).** The migration is complete
  iff the following ripgrep sweeps return zero matches when run from the
  current-app frontend CSS root
  (`skills/visualisation/visualise/frontend/src`). These sweeps are
  filesystem-determined — they do **not** depend on the research inventory,
  so a zero-match result is authoritative no matter how many values the
  inventory surfaced:
  - `rg --glob '**/*.module.css' 'border-radius:\s*[.0-9]' src`
  - `rg --glob '**/*.css' --glob '!**/global.css' 'border-radius:\s*[.0-9]' src`
  - `rg --glob '**/*.css' --glob '!**/global.css' 'border-(top|bottom)-(left|right)-radius:\s*[.0-9]' src`
  (`global.css` is excluded uniformly because it holds only `--radius-*:`
  token definitions, which these `border-radius:` / `border-*-radius:`
  patterns never match.) The `[.0-9]` prefix catches every numeric-led
  px/rem/em literal; an allowed `var(--radius-*)` reference is non-numeric-led
  and so never matches.
- **AC4.** The CI gate runs exactly AC3's three sweeps over the same scope
  (module and non-module `.css` under `src/`, `global.css` excluded).
  Inserting a literal radius (e.g. `border-radius: 7px`) into any current-app
  `.css` file causes the gate step to exit non-zero; with no literals
  present the gate passes.
- **AC5.** `migration.test.ts` EXCEPTIONS cleanup: the "in-between border
  radii (6px)" irreducible entry — and any other radius literal entry the
  new rule supersedes — is deleted. Backstop (mirroring 0075 AC4a): no
  remaining `kind: 'irreducible'` entry has a `literal` that appears in a
  migrated `border-radius` / `border-*-radius` declaration, and no
  remaining entry's `reason` references `border-radius`.
- **AC6.** A new ADR codifies the radius consume-tokens rule, and
  ADR-0026 §3 no longer carries an active "In-between border radii" row:
  it is either deleted, or annotated as superseded by the new radius ADR's
  ID (a grep for `In-between border radii` returns no un-struck table row).
- **AC7.** Documentation: `global.css` carries a comment above the
  `--radius-*` block referencing the new ADR, and the PR description states
  the verbatim rule and a one-paragraph rationale referencing the new ADR
  by ID.
- **AC8.** New radius tokens take a **scale-based** name when the value
  either equals an existing `--radius-*` token's px value or extends the
  `sm`/`md`/`lg`/`pill` ladder at a regular end step (e.g. `--radius-xs:
  2px` below `sm`). A value that falls **between** two existing ladder
  steps — and so has no ladder position (e.g. `6px`, between `sm` 4px and
  `md` 8px) — takes a **use-case** name, and the implementer records a
  one-line naming rationale in the PR. ("Off-scale", used in Requirements
  and AC1, means this between-steps case.)

## Open Questions

- Where the gate lives — an existing CI lint step versus a dedicated check
  (the pattern itself is specified in AC3).
- Whether the 2px badge radius is currently recorded as an EXCEPTIONS /
  irreducible entry (ADR-0026 §3's "1px/2px" row is border *widths*, not
  radius) — to confirm during the pre-implementation research.

## Dependencies

- Blocked by: 0075 (done) — established the consume-tokens-everywhere
  pattern, the EXCEPTIONS-retirement approach, and the ADR style this story
  reuses for radius; 0075's stated "0090 must not begin until 0075 lands"
  ordering is now satisfied. 0033 (done) — defines the `--radius-*` scale
  this story extends.
- Upstream input (not yet performed), tracked at
  `meta/research/codebase/<date>-0090-radius-tokens-consumption.md`: the
  pre-implementation codebase research that enumerates the full current-app
  radius inventory and records pre-migration computed values. It finalises
  AC2's selector list and the EXCEPTIONS-cleanup scope in AC5 (which
  literals are deleted, including whether the 2px badge is an irreducible
  entry); AC3's gate is filesystem-determined and does not depend on it.
  Implementation cannot begin until this research lands — see Assumptions.
- Artefact couplings (same PR): ADR-0026 amendment (§3 radius row) and a
  new radius-consumption ADR; `migration.test.ts` EXCEPTIONS cleanup for
  radius literals (cleanup set finalised by the research above).
- Tooling: Playwright regression harness at
  `skills/visualisation/visualise/frontend/tests/visual-regression/`
  (existing `*-resolved-*` specs are the pattern to follow).
- Blocks: none currently — the CI gate and the new radius ADR establish a
  standing constraint that future current-app CSS must obey, but no
  in-flight work item depends on this one.
- Related: 0041 (page-level spacing-token consumption — sibling pattern),
  0077 (shadow/accent token audit — sibling pattern).

## Assumptions

- The pre-implementation codebase research enumerates the full current-app
  radius inventory and records pre-migration computed values; AC2's
  selector list and AC3's file globs derive from it.
- "Current app" means the production component CSS, excluding the
  prototype.
- Extend-and-preserve means zero intended visual change — 2px and 6px are
  codified as exact tokens rather than rationalised onto the scale. A
  reviewer expecting cleanup of off-scale values won't get that here.
- The grep/lint gate covers all radius properties, including the longhand
  corner forms (`border-top-left-radius`, etc.), not just the shorthand.

## Technical Notes

- Existing 0033 scale (done): `--radius-sm: 4px`, `--radius-md: 8px`,
  `--radius-lg: 12px`, `--radius-pill: 999px`.
- Planned additions: `--radius-xs: 2px`, `--radius-block: 6px`.
- Known outliers to migrate: `RelatedArtifacts` badge `2px` →
  `--radius-xs`; Markdown `<pre>` `6px` → `--radius-block`.
- ADR clause to supersede: ADR-0026 §3 "Irreducible literal categories"
  lists "In-between border radii: `6px`"
  (`meta/decisions/ADR-0026-css-design-token-application-conventions.md`).
  The 1px/2px border-*width* row in the same table is unrelated and stays.
- Enforcement harness:
  `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`
  (radius literals recorded here as `kind: 'irreducible'`).
- Playwright harness:
  `skills/visualisation/visualise/frontend/tests/visual-regression/`;
  follow the `chip-resolved-colours.spec.ts` / `glyph-resolved-fill.spec.ts`
  / `code-block-resolved-colours.spec.ts` pattern, asserting on
  `getComputedStyle(el).borderTopLeftRadius` (etc.).
- Grep gate pattern: shorthand `border-radius:\s*[.0-9]` plus the longhand
  alternation `border-(top|bottom)-(left|right)-radius:\s*[.0-9]`.
- Model the migration and the gate on sibling precedents: 0075 (typography
  consumption), 0041 (page spacing), 0077 (shadow/accent audit) — all done.

## Drafting Notes

- The ADR amendment is required, not pattern-mimicry: reading ADR-0026 §3
  confirmed it explicitly classifies the 6px `<pre>` radius as irreducible,
  which the consume-tokens rule directly contradicts. The new ADR + §3
  amendment mirror 0075's ADR-0036 + ADR-0026 approach.
- The full inventory and the pixel-perfect baseline are deferred to the
  codebase research that precedes planning, per process; they are not
  embedded in this story. The grep gate (AC3) is the standing completeness
  enforcement; AC2 provides the per-selector regression guarantee.
- Reframed the original "define a `--radius-*` scale" requirement to
  "extend the existing 0033 scale" after discovering 0033 already ships
  `--radius-*`.
- Naming categories: *scale-based* = the sm/md/lg/pill ladder from 0033
  (extended with xs); *use-case-named* = a single-purpose name (e.g.
  `block`) used only where a value has no ladder slot.
- Honoured the extend-and-preserve choice — codifies 2px and 6px exactly
  rather than snapping them onto existing steps.
- Sizing contingency (borrowing 0075's framing): this is scoped as a
  single atomic-PR story. If the pre-implementation research returns an
  unexpectedly large radius inventory, re-scope to an epic with
  per-file-group children, the first child landing rule + tokens + harness
  + ADR. This is a fallback, not the planned-of-record delivery shape.

## References

- Source carve-out: `meta/work/0075-typography-size-scale-consumption.md`
  (Assumptions, Drafting Notes) — pattern precedent for the rule,
  EXCEPTIONS retirement, ADR amendment, and regression spec.
- Scale definition: `meta/work/0033-design-token-system.md`.
- Convention to amend: `meta/decisions/ADR-0026-css-design-token-application-conventions.md`
  (§3 irreducible "In-between border radii: 6px" row).
- Related: 0041, 0077 (sibling consume-tokens patterns); 0091 (typography
  rem-vs-px stance — adjacent token-authoring spike, not a dependency).
