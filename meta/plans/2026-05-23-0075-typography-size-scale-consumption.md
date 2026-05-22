---
date: "2026-05-23T13:42:51Z"
type: plan
skill: create-plan
work-item: "0075"
status: accepted
---

# 0075 Typography Size-Scale Consumption Reconciliation — Implementation Plan

## Overview

Adopt **consume-tokens-everywhere** as the canonical rule for `font-size`
in current-app CSS, widen the `--size-*` scale with five new tokens to
cover off-grid values, rename the chip-prefixed tokens (`--size-chip`,
`--size-chip-md`) to numeric-ladder names so the scale's vocabulary stops
lying when those tokens are consumed outside chip components, migrate all
literal `font-size` declarations (including those embedded in `font:`
shorthand) onto the scale, retire the typography entries in
`migration.test.ts` EXCEPTIONS that the new rule supersedes, and supersede
the typography portion of ADR-0026 with a new ADR (ADR-0036) so the new
rule replaces the prior tolerance-band convention via the project's
documented supersession path (per ADR-0031's immutability convention).
Delivered as a single atomic PR composed of internally-independent
phases, with the new vitest category-level enforcement test landing in
the final phase so the suite stays green throughout the migration.

## Current State Analysis

- **Token scale**: `src/styles/global.css:126-142` declares the
  `--size-*` family (11 type steps + 2 chip tokens, defined under a
  bare `/* Typography */` comment). The block is *not* strictly
  descending: `--size-chip: 10.5px` precedes `--size-chip-md: 11.5px`,
  an inversion this plan corrects as part of the chip rename (see
  Phase 1.1).
- **Consumption**: 37 literal `font-size` sites across **10** files
  remain. The work item's published inventory names 9 files; this plan
  also covers `ActivityFeed.module.css` (two `font:` shorthand sites),
  which the AC2 third sweep catches but the work item's Context missed.
- **Enforcement harness**: `src/styles/migration.test.ts` ships a
  per-occurrence `EXCEPTIONS` array; every current typography outlier
  is admitted with `kind: 'irreducible'`. The literal `count` on some
  entries combines font-size *and* non-font-size uses (e.g. Sidebar's
  `10px` count of 6 mixes a font-size with five padding sites), so this
  plan decrements rather than deletes such entries.
- **ADR-0026 §2** sets a ±2px tolerance band for typography substitution;
  **§3** marks "em-relative font-sizes" and "Heading font-sizes above
  `size-lg`" as permanently irreducible. **Consequences** records the
  heading-token gap as deferred. ADR-0031 forbids in-place edits to
  `accepted` ADRs, so this plan supersedes the typography clauses via a
  new ADR-0036 (see Phase 1.3). The supersession is scope-limited:
  ADR-0026's `status: accepted` is retained, the typed-linkage
  `superseded_by: "adr:ADR-0036"` records the relationship, and the
  superseded sections are identified textually in both ADRs. The
  spacing band continues to apply.
- **Playwright harness**: existing `tests/visual-regression/` directory
  with three close pattern precedents
  (`chip-resolved-colours.spec.ts`, `glyph-resolved-fill.spec.ts`,
  `code-block-resolved-colours.spec.ts`) using
  `locator.evaluate((el) => getComputedStyle(el).<prop>)`.
  `playwright.config.ts` does not pin a viewport — default chromium
  1280×720 will apply unless the new spec sets one.

### Key Discoveries

- `src/styles/global.css:126-142` — single source of truth for
  `--size-*`. AC6a comment must sit directly above the `--size-*`
  block, which means splitting the existing `/* Typography */` comment
  so fonts (`--ac-font-*`) keep theirs and sizes get the new one.
- `src/styles/migration.test.ts:48-250` — EXCEPTIONS array with mixed
  font-size / spacing uses on several literals. Migration is a
  decrement-or-delete decision per entry, not blanket deletion.
- `meta/decisions/ADR-0026-css-design-token-application-conventions.md:107-118`
  (§2 typography band) / `:121-135` (§3 irreducible categories) /
  `:287-290` (Consequences deferral) — exact loci for the amendment.
- `src/components/ActivityFeed/ActivityFeed.module.css` — two
  `font: 500 10.5px/1 var(--ac-font-mono)` sites; out-of-inventory but
  in-scope per AC2 third sweep.
- `src/components/Sidebar/Sidebar.module.css:50,75,185` — three `font:`
  shorthand sites with embedded font-size literals; must be expanded
  into individual properties so the literal disappears.

## Desired End State

- Five new tokens (`--size-4xs`, `--size-3xs`, `--size-eyebrow`,
  `--size-row`, `--size-subtitle`) declared in `global.css`, slotted
  into the `--size-*` block by px ordering.
- The two chip-prefixed tokens are renamed (continuation of the numeric
  ladder below `--size-xxs`):
  - `--size-chip-md: 11.5px` → `--size-xxs-sm: 11.5px`
  - `--size-chip: 10.5px` → `--size-3xs-lg: 10.5px`
- Canonical AC6a comment present directly above the `--size-*` block.
- All three AC2 ripgrep sweeps return zero matches under
  `skills/visualisation/visualise/frontend/src/`.
- Every outlier in Context (and the additional ActivityFeed sites)
  consumes a `var(--size-*)` token.
- `migration.test.ts` carries no EXCEPTIONS entry referencing a
  font-size literal; no remaining entry's `reason` contains the
  substring `font-size`. Each remaining entry's `reason` enumerates the
  concrete remaining use sites (e.g. `card padding-block at .card`),
  not just a category label.
- A new ADR-0036 ("Typography font-size consumption rule") supersedes
  the typography portion of ADR-0026 (scope-limited supersession per
  ADR-0031 + ADR-0034 typed linkage). ADR-0026's frontmatter records
  `superseded_by: "adr:ADR-0036"`; ADR-0026's §2 Typography subsection
  + the two affected §3 rows are deleted and replaced with
  single-line "see ADR-0036" pointers; ADR-0026's Consequences
  "heading font-size gap" deferral is marked resolved by ADR-0036.
- A new vitest category-level enforcement (added in Phase 8, not
  Phase 1) bans `font-size:` literals and `font:` shorthand with
  embedded numeric sizes across every CSS file in `allCss` *and* the
  root `styles/global.css` file; the regex is identical to the AC2
  third sweep.
- New Playwright spec
  `tests/visual-regression/typography-resolved-sizes.spec.ts` asserts
  the post-migration computed `font-size` for every selector listed in
  AC7 plus the two ActivityFeed sites, plus a deliberate-drift
  regression case asserting inline `<code>` inside an H2 resolves to
  `14px`.
- Full vitest + Playwright suites pass (the suite is green at every
  intermediate phase boundary, not just at the end).

### Verification

- `cd skills/visualisation/visualise/frontend && \
   rg --glob '**/*.module.css' 'font-size:\s*[.0-9]' src` → no matches.
- Same with `--glob '**/*.css' --glob '!**/global.css'` → no matches.
- Same with `--glob '**/*.module.css' 'font:\s*[^;]*\s[.0-9]+(px|rem|em)' src` → no matches.
- `npm test` (vitest) green.
- `npm run e2e -- --project=visual-regression` green, including the
  new `typography-resolved-sizes.spec.ts`.
- ADR-0026 visually re-reads without orphaned references.

## What We're NOT Doing

- Not deleting unused heading tokens (`--size-hero`, `--size-h1`,
  `--size-h2`, `--size-h4`) — kept as defensive scaffolding (the AC2
  grep only fails on literals, not on unused tokens).
- Not touching radius literals — `RelatedArtifacts` `2px`, markdown
  `<pre>` `6px` are tracked under 0090.
- Not modifying the prototype HTML (`prototype-standalone.html`) — it
  is an investigative snapshot, not a modification target.
- Not migrating `font-family` or `line-height` declarations; only
  `font-size` is in scope.
- Not changing the ±2px tolerance band for **spacing** (`--sp-*`) —
  ADR-0026 §2's spacing clause remains in force.
- Not splitting delivery into multiple PRs — single atomic PR (with
  documented epic-split contingency, see Implementation Approach).
- Not converting `--size-*` tokens to rem units. The `--size-*` scale
  remains px-anchored as a deliberate project-wide stance; this trades
  user-controllable root-font-size scaling for token-value determinism
  and is documented in ADR-0036 §2 as a known consequence. Heading-tier
  rem-based scaling, if reintroduced later, is tracked under follow-up
  work item **0091** (typography rem-vs-px stance review) created
  alongside this plan.

## Implementation Approach

The plan is **migration-first, enforcement-last**: Phase 1 introduces
the new tokens, renames the chip pair, supersedes ADR-0026's typography
clauses with a new ADR-0036, and scaffolds the Playwright computed-size
spec. The vitest category-level ban is *not* added in Phase 1 — it
lands in Phase 8, after every outlier has been migrated, so the suite
stays green throughout the migration window. Phases 2–7 are still
driven by the AC2 ripgrep sweeps (run locally per phase as the
verification gate) and by the existing per-occurrence EXCEPTIONS
hygiene check, which catches count drift immediately.

Phases 2–7 are **internally independent** — once Phase 1 lands, any
cluster can be migrated next without affecting the others. Each
cluster phase touches only its own CSS files and its own EXCEPTIONS
entries, runs the AC2 ripgrep sweeps for its scope, and runs the
Playwright case (or cases) corresponding to its file group.

Phase 8 lands the new vitest category-level ban (which should be green
on first run because the migration is complete), the final
EXCEPTIONS-reason sweep, the all-three-greps proof, the rollback
strategy, and the PR description deliverable.

The whole sequence ships as one PR; "phase" here is a development /
review chunk, not a separately-merged commit.

### Contingency: epic-split

If the PR proves genuinely unreviewable in one piece — for example, the
reviewer requests a split, or the cumulative diff (excluding generated
files and tests) exceeds ~1500 lines — escalate by re-scoping this work
item to an epic with child stories one per outlier file group
(MarkdownRenderer + Page, FilterPill + SortPill, Sidebar + ActivityFeed,
library routes). The first child story lands the Phase 1 deliverables
(tokens, chip rename, ADR-0036, Playwright scaffold) plus the new
vitest enforcement gated behind a `MIGRATION_IN_PROGRESS` flag (so the
suite stays green); subsequent child stories migrate one cluster each;
the final child story removes the flag and runs Phase 8.

---

## Phase 1: Foundations — Tokens, Rename, ADR Supersession, Playwright Scaffold

### Overview

Land the new tokens and the canonical comment in `global.css`, rename
the chip-prefixed tokens to numeric-ladder names (`--size-chip` →
`--size-3xs-lg`, `--size-chip-md` → `--size-xxs-sm`) at every consumer
site that currently uses them, create the new ADR-0036 superseding the
typography portion of ADR-0026, update ADR-0026 to record the partial
supersession, and scaffold the Playwright computed-size spec.

**Note on TDD ordering:** This phase does *not* add the vitest
category-level ban (moved to Phase 8 so the suite stays green
throughout the migration). Phases 2–7 are driven by the AC2 ripgrep
sweeps and the existing EXCEPTIONS hygiene check; the new ban lands as
the final enforcement step in Phase 8 and should be green on first run.

**Note on phase scope:** Phase 1 is large by design — it bundles six
sub-deliverables (1.1 tokens + 1.1b chip-consumer rename, 1.2
tokens.ts, 1.3 ADR work, 1.4 Playwright scaffold, 1.5 baseline,
1.6 work-item update). For review economy, treat these as separately
reviewable sub-PR-commits *within* the single atomic PR: each sub-step
should land as its own commit on the branch so reviewers can navigate
the diff sub-step by sub-step. If a sub-step proves unworkable in
review (e.g. the ADR-0030 template alignment in 1.3a requires a
sibling ADR), the epic-split contingency in Implementation Approach
applies — start the split with the documentation-only sub-step (1.3)
as the first child story.

### Changes Required

#### 1.1 — Extend the `--size-*` scale and rename the chip pair

**File**: `skills/visualisation/visualise/frontend/src/styles/global.css`
**Lines**: 126-142
**Change**: split `/* Typography */`; insert AC6a comment + five new
tokens; rename the chip pair (`--size-chip-md` → `--size-xxs-sm`;
`--size-chip` → `--size-3xs-lg`); sort the whole block strictly
descending by px. The chip/chip-md pair currently appears in the file
in ascending order (10.5 then 11.5) — the new block corrects this
inversion as part of the rename. Resulting block:

```css
  /* Typography fonts */
  --ac-font-display:  "Sora", system-ui, sans-serif;
  --ac-font-body:     "Inter", system-ui, sans-serif;
  --ac-font-mono:     "Fira Code", ui-monospace, monospace;

  /*
   * font-size consumers: use these tokens — see ADR-0036.
   *
   * Naming: tier names (`--size-{n}xs`/`xs`/`sm`/`md`/`body`/`lg`/`h{n}`/`hero`)
   * form the integer-px scale. `--size-eyebrow`, `--size-row`, and
   * `--size-subtitle` are semantic single-purpose tokens (each has one
   * conceptual consumer in current designs). The `-sm` / `-lg` suffixes
   * mark sub-pixel tweens between integer tiers
   * (e.g. `--size-xxs-sm` = 11.5px sits between `--size-xxs` 12 and
   * `--size-eyebrow` 11).
   */
  --size-hero:        68px;
  --size-h1:          48px;
  --size-h2:          36px;
  --size-h3:          28px;
  --size-h4:          26px;
  --size-lg:          22px;
  --size-body:        20px;
  --size-md:          18px;
  --size-sm:          16px;
  --size-xs:          14px;
  --size-subtitle:    13px;
  --size-row:         12.5px;
  --size-xxs:         12px;
  --size-xxs-sm:      11.5px;  /* renamed from --size-chip-md */
  --size-eyebrow:     11px;
  --size-3xs-lg:      10.5px;  /* renamed from --size-chip */
  --size-3xs:         10px;
  --size-4xs:         9.5px;
  --lh-tight:         1.05;
  --lh-snug:          1.2;
  --lh-normal:        1.5;
  --lh-loose:         1.6;
  --tracking-caps:    0.12em;
```

**AC6a wording:** the canonical comment is the lead sentence of a
comment block above the `--size-*` tokens. The work item's AC6a
currently mandates "`/* font-size consumers: use these tokens — see
ADR-0026 (as amended by 0075) */`" — Phase 1.6 (see below) updates
that AC to expect the multi-line block form above and point at
ADR-0036.

#### 1.1b — Rename chip token consumers (in same commit as 1.1)

The chip rename in 1.1 deletes `--size-chip` and `--size-chip-md` from
`global.css`. Every consumer site must be migrated in the same commit
or the suite goes red. The codebase audit identified six files outside
`global.css`/`tokens.ts` that reference the chip tokens:

**CSS consumers** — rename `var(--size-chip)` → `var(--size-3xs-lg)`
and `var(--size-chip-md)` → `var(--size-xxs-sm)`:

- `src/components/Chip/Chip.module.css:9` — `var(--size-chip)`
- `src/components/Chip/Chip.module.css:19` — `var(--size-chip-md)`
- `src/components/FrontmatterTable/FrontmatterTable.module.css:13` —
  `var(--size-chip-md)`
- `src/components/MarkdownRenderer/MarkdownRenderer.module.css:46` —
  `var(--size-chip)` (separate from the line 9/53 sites Phase 2
  migrates)

**Test consumers** — rename the literal token names in regex
assertions and `it()` descriptions:

- `src/components/Chip/Chip.test.tsx:67` — `it('binds base font-size
  to --size-chip', …)` → `it('binds base font-size to
  --size-3xs-lg', …)`
- `src/components/Chip/Chip.test.tsx:68` —
  `expect(chipCss).toMatch(/\.chip\s*\{[^}]*font-size:\s*var\(--size-chip\)/)`
  → `…var\(--size-3xs-lg\)/`
- `src/components/Chip/Chip.test.tsx:114` — `…font-size:\\s*var\\(--size-chip-md\\)`
  → `…var\\(--size-xxs-sm\\)`
- `src/components/FrontmatterTable/FrontmatterTable.test.tsx:215` —
  `it('uses --size-chip-md for table font-size …', …)` →
  `it('uses --size-xxs-sm for table font-size …', …)`
- `src/components/FrontmatterTable/FrontmatterTable.test.tsx:216` —
  `expect(css).toMatch(/font-size:\s*var\(--size-chip-md\)/)` →
  `…var\(--size-xxs-sm\)/`

After 1.1b, `rg -n 'size-chip' skills/visualisation/visualise/frontend/src`
must return zero matches. The Phase 1 Success Criteria asserts this.

#### 1.2 — Register new tokens and rename chip tokens in `tokens.ts`

**File**: `skills/visualisation/visualise/frontend/src/styles/tokens.ts`
**Change**: in `TYPOGRAPHY_TOKENS` (the declared-tokens set consumed
by migration.test.ts's `var(--NAME) references resolve to declared
tokens` describe block), (a) rename the existing chip keys and (b)
add the five new tokens. The object is ordered px-descending to mirror
the global.css block, so slot the new keys by px ordering, not
alphabetically. Resulting size-* portion:

```ts
'size-hero':       '68px',
'size-h1':         '48px',
'size-h2':         '36px',
'size-h3':         '28px',
'size-h4':         '26px',
'size-lg':         '22px',
'size-body':       '20px',
'size-md':         '18px',
'size-sm':         '16px',
'size-xs':         '14px',
'size-subtitle':   '13px',   // new
'size-row':        '12.5px', // new
'size-xxs':        '12px',
'size-xxs-sm':     '11.5px', // renamed from 'size-chip-md'
'size-eyebrow':    '11px',   // new
'size-3xs-lg':     '10.5px', // renamed from 'size-chip'
'size-3xs':        '10px',   // new
'size-4xs':        '9.5px',  // new
```

Without the rename + new entries, any `var(--size-eyebrow)` etc. (or
the renamed `var(--size-xxs-sm)` / `var(--size-3xs-lg)`) in later
phases would fail the existing harness.

#### 1.3 — Supersede ADR-0026's typography clauses with a new ADR-0036

ADR-0031 forbids in-place edits to `accepted` ADRs and prescribes a
supersession-by-new-ADR path for content changes. Rather than amending
ADR-0026 in place (which the original draft of this plan specified),
this phase creates **ADR-0036** scoped to the typography rule and
transitions ADR-0026 to record the partial supersession.

##### 1.3a — Create ADR-0036

**File** (new): `meta/decisions/ADR-0036-typography-font-size-consumption-rule.md`

Use the canonical ADR template per ADR-0030. Frontmatter:

- `status: accepted`
- `supersedes: ["adr:ADR-0026"]` — per ADR-0034's identity-value
  contract (`<doc-type>:<id>` quoted strings). This is a
  scope-limited supersession (typography clauses only); ADR-0036's
  body explicitly states which sections of ADR-0026 it supersedes.

Body sections (matching ADR-0030's hybrid template — Context /
Decision Drivers / Considered Options / Decision / Consequences /
References):

- **Context** — describe the prior state without coupling to ephemeral
  work-item IDs: ADR-0026 §2 set a ±2px tolerance band that worked for
  spacing but admitted 35 typography outliers as `irreducible`
  EXCEPTIONS by mid-2026; ADR-0026 §3 listed em-relative font-sizes
  and heading sizes above `--size-lg` as permanently irreducible. The
  triggering codebase audit is referenced in the References section
  by file path (not by work-item ID in the narrative).

- **Decision Drivers** — enumerate the forces:
  - Token-system value-prop: a defined-but-not-consumed scale
    contradicts its own purpose.
  - Drift surface: per-occurrence EXCEPTIONS accumulate over time and
    erode the rule.
  - Enforceability: a categorical rule is mechanically checkable;
    tolerance bands require judgement.
  - Spacing context: the spacing tolerance band is empirically
    successful and should not be disturbed.

- **Considered Options** — at least two:
  1. **Retain ADR-0026's tolerance band; widen the scale only.**
     Rejected because the band-plus-scale combination admits drift
     indefinitely.
  2. **Adopt consume-tokens-everywhere; widen the scale to absorb
     every used off-grid value.** Chosen — see Decision.

- **Decision** —
  > **Typography (`font-size`) consumption rule**: every `font-size`
  > declaration in current-app CSS (component modules and global
  > stylesheets under `skills/visualisation/visualise/frontend/src/`)
  > must resolve to a `var(--size-*)` token reference. No literal
  > `px`, `rem`, or `em` `font-size` values are permitted, including
  > those embedded in `font:` shorthand. Off-grid values are handled
  > by extending the scale rather than by tolerance-band substitution.
  >
  > **Scope of supersession**: this ADR supersedes the *typography
  > portion* of ADR-0026 — specifically ADR-0026 §2's Typography
  > subsection and §3's "em-relative font-sizes" and "Heading
  > font-sizes above `--size-lg`" rows. The ±2px tolerance band
  > documented in ADR-0026 §2 for **spacing** (`--sp-*`) continues to
  > apply unchanged.
  >
  > **Scale extension policy**: new sub-pixel `-sm` / `-lg` tweens
  > between integer tiers (e.g. `--size-xxs-sm` 11.5 between
  > `--size-xxs` 12 and `--size-eyebrow` 11) require a design-review
  > justification that the existing integer tier cannot be used. The
  > scale is not infinitely extensible — sub-pixel tweens are
  > admitted only when design intent demands them.
  >
  > **Escape valve**: a `FONT_SIZE_LITERAL_EXCEPTIONS` array beside
  > the vitest category-level ban (per-occurrence shape mirroring
  > `EXCEPTIONS`) may admit specific literal sites in genuinely
  > exceptional cases (third-party CSS injection, transient
  > migrations). Each entry must reference an ADR or work item
  > documenting why the exception is justified, and a target-removal
  > date or condition. Entries older than 12 weeks without a
  > documented removal blocker should be migrated or escalated.
  > Routine use is not permitted.
  >
  > **Why a separate array from `EXCEPTIONS`**: the existing
  > `EXCEPTIONS` ledger in `migration.test.ts` is a per-occurrence
  > admission ledger for the AC4 hygiene test (every literal in
  > scope of AC4 must declare an exemption with a kind, count, and
  > reason). It is the *expected steady-state* shape — every literal
  > the rule admits flows through it. `FONT_SIZE_LITERAL_EXCEPTIONS`,
  > by contrast, is a category-level escape valve that should remain
  > near-empty: any non-empty state is an exception requiring a
  > documented removal plan. Keeping the two separate preserves
  > "EXCEPTIONS = admitted literals, FONT_SIZE_LITERAL_EXCEPTIONS =
  > escape valve" as two distinct signals; merging them would lose
  > the policy distinction (routine vs exceptional admissions).

- **Consequences**:
  - **Positive**: enforceable by `rg`/vitest as a category-level
    invariant; design intent is recoverable from token name (chip
    rename eliminates the misleading naming); single ADR rather than
    per-occurrence EXCEPTIONS for typography.
  - **Negative**: legitimate one-off literals (third-party CSS, hot
    fixes) require the escape valve. The escape valve has overhead;
    routine use would erode the rule.
  - **Neutral / known**: `--size-*` tokens are intentionally
    px-anchored. This trades user-controllable root-font-size scaling
    for token-value determinism. Browser-level zoom still works; users
    who customise default font-size in their browser for accessibility
    lose font-size-only scaling for typography. A future review of
    the px-vs-rem stance remains open; see References.

- **References**:
  - `meta/decisions/ADR-0026-css-design-token-application-conventions.md`
    — partially superseded by this ADR (typography clauses only).
  - `meta/decisions/ADR-0030-adr-template.md` — template followed.
  - `meta/decisions/ADR-0031-skill-level-adr-immutability.md` —
    supersession convention applied.
  - `meta/decisions/ADR-0034-typed-linkage-vocabulary.md` — `supersedes`
    linkage shape.
  - `meta/research/codebase/2026-05-23-0075-typography-size-scale-consumption.md`
    — codebase audit that motivated this ADR.
  - `meta/work/0075-typography-size-scale-consumption.md` — work item
    that landed the rule.
  - `meta/work/0091-typography-rem-vs-px-stance.md` — follow-up review
    of the px-anchored stance.

**Why "0075" doesn't appear in the ADR body**: ADRs are durable
artefacts; work-item IDs are ephemeral. The References section
records the work item that landed the rule; the body describes the
decision in self-contained terms.

##### 1.3b — Update ADR-0026

**File**: `meta/decisions/ADR-0026-css-design-token-application-conventions.md`

Per ADR-0031's supersession convention, record that the typography
portion has been superseded by ADR-0036. The specific updates:

- **Frontmatter**: add `superseded_by: "adr:ADR-0036"` per ADR-0034's
  identity-value contract (single ref form, quoted string). Keep
  `status: accepted` — ADR-0031 does not currently define a
  `partially_superseded` status, and ADR-0034's `superseded_by`
  linkage is sufficient to convey scope-limited supersession at the
  graph level. ADR-0026's body identifies the superseded sections
  textually (see next bullet); ADR-0036's body identifies them
  symmetrically.
- **Status section / top note**: add a paragraph: *"The typography
  portion of this ADR (§2's Typography subsection and §3's
  typography rows) is superseded by ADR-0036. The spacing rule in §2
  and all other clauses remain in force."* Avoid the framing
  "partially superseded" as a coined lifecycle state — it isn't one;
  the supersession is scope-limited and described textually.
- **§2 Typography subsection (lines 107-118)** — delete the
  `Typography: substitute…` paragraph and its table. Replace with the
  single line: *"Typography (`font-size`) consumption is governed by
  ADR-0036."*
- **§3 (lines 121-135)** — delete the two rows *"em-relative
  font-sizes | `0.88em`, `1.4em` | …"* and *"Heading font-sizes above
  `size-lg` | `1.6rem`, `1.75rem` | …"*. Add a single line under the
  table: *"`font-size` literals (any unit) are not irreducible — see
  ADR-0036."*
- **Consequences §Neutral (lines 287-290)** — strike the "heading
  font-size gap" paragraph and replace with: *"The heading font-size
  gap (originally deferred to a future type-scale extension) is
  resolved by ADR-0036: heading sizes consume `--size-h3` /
  `--size-h4` and irreducible heading EXCEPTIONS no longer exist."*

**Why this path rather than full-file replacement**: ADR-0026 still
owns the spacing tolerance band and other clauses. A full
supersession would orphan that content; a scope-limited supersession
via explicit `superseded_by` linkage + section-level "see ADR-0036"
pointers preserves the surviving content's status while routing
typography questions to the new authority.

#### 1.4 — Scaffold the Playwright spec

**File**: `skills/visualisation/visualise/frontend/tests/visual-regression/typography-resolved-sizes.spec.ts`
**Change**: new file. Pattern from `chip-resolved-colours.spec.ts` but
asserting `.fontSize`. Pin a viewport explicitly to remove `1rem`
sensitivity. Selectors and expected values match AC7 plus the
ActivityFeed extension *and* the deliberate-drift regression case
(inline `<code>` inside an H2).

```ts
import { test, expect } from '@playwright/test'

// Pin viewport so rem-derived values resolve at 16px root.
test.use({ viewport: { width: 1280, height: 720 } })

type Case = {
  route: string
  // Optional render precondition (e.g. open a menu) run after `goto` and
  // before the size assertion. Use it for any selector that isn't
  // immediately visible after navigation.
  setup?: (page: import('@playwright/test').Page) => Promise<void>
  selector: string
  expected: string
  name: string
}

const CASES: Case[] = [
  // Filled in during the placeholder-resolution sub-step below.
]

for (const c of CASES) {
  test(`computed font-size: ${c.name}`, async ({ page }) => {
    await page.goto(c.route)
    if (c.setup) await c.setup(page)
    const fs = await page
      .locator(c.selector)
      .first()
      .evaluate((el) => getComputedStyle(el).fontSize)
    expect(fs).toBe(c.expected)
  })
}
```

**Placeholder-resolution sub-step (must be done during Phase 1, before
the phase closes):**

For each of the cases below, capture concretely the `{route, setup?,
selector, expected, name}` quadruple. The cases are:

1. MarkdownRenderer H1 → 28px
2. MarkdownRenderer inline code (in body) → 14px
3. MarkdownRenderer inline code *inside an H2* → 14px (deliberate-drift
   regression case — captures the H2 vs body contrast that the rest of
   the spec leaves untested)
4. Page .eyebrow → 11px
5. Page .subtitle → 13px
6. Sidebar .phaseHeading → 9.5px
7. Brand .brandSub → 10px
8. SortPill .menuItem → 12.5px (likely needs `setup` to open the menu)
9. FilterPill .option → 12.5px (likely needs `setup` to open the menu)
10. EmptyState .title → 22px (needs a route in an empty state)
11. LibraryTypeView .row → 13px
12. ActivityFeed heading → 10.5px
13. ActivityFeed live badge → 10.5px

For each case, the resolution procedure is:

a. Use the **browser-locator** agent (or read the routes config /
   manual ripgrep of `import` chains in `App.tsx` / route files) to
   identify a deterministic route that mounts the target component
   without user interaction beyond what `setup` covers.
b. Prefer reusing an existing route's natural rendering over creating
   a new showcase. If no route mounts the component, the fallback
   sequence is: (i) extend an existing showcase route (precedent:
   `/chip-showcase`); (ii) add a minimal new showcase route only if
   nothing else suffices.
c. Capture a deterministic selector. Prefer `[data-testid="…"]` if
   already present. If the component has no test id, add one as part
   of this phase (one-line edit per component); do not rely on
   structural CSS selectors that may shift.
d. For menus / popovers (SortPill, FilterPill), the `setup` callback
   opens the menu via `page.getByRole('button', { name: … }).click()`
   then awaits the menu visibility before the size assertion.

The Success Criteria below require this enumeration to be recorded in
the plan (or in an attached working note committed alongside the
plan) before Phase 1 is marked complete; otherwise downstream phases
have no failing or passing target to verify against.

#### 1.5 — Baseline verification on `main`

**Purpose**: anchor the EXCEPTIONS decrement targets in Phases 2-7
against the actual file state at `main`, surfacing any pre-existing
miscount in `EXCEPTIONS` before the migration starts.

**Steps**:

1. From `main` (clean working copy), `cd
   skills/visualisation/visualise/frontend && npm test --
   migration.test.ts` and confirm the full suite passes, including the
   `declared count equals observed count` hygiene block.
2. For each of the literals this plan will decrement (Sidebar `10px`,
   FilterPill `10px`, EmptyState `22px` / `14px` / `12px`,
   LibraryOverviewHub `14px`, LibraryTypeView `12px`), run an `rg -nc
   '<literal>' <file>` and confirm the count matches the EXCEPTIONS
   entry. Record the baseline counts in a one-table working note
   alongside the plan (or as a comment in the PR description).
3. If any literal's observed count differs from its EXCEPTIONS entry,
   stop and reconcile before proceeding — the discrepancy is either a
   pre-existing harness bug (file the fix as a precursor commit) or
   the plan's decrement target is wrong.

**VCS workspace note**: the baseline check must run against pristine
`main`, not the implementer's working tree. The project uses `jj`
workspaces; create a fresh workspace at `main` for the baseline run:

```bash
jj workspace add ../typography-baseline --revision main
cd ../typography-baseline/skills/visualisation/visualise/frontend
npm install  # if needed
npm test -- migration.test.ts
# … capture rg counts here …
```

Record the workspace path beside the captured baseline counts so a
reviewer can verify the baseline came from a pristine checkout.

#### 1.6 — Update work item 0075 to match the supersession path

The plan's structural changes (ADR-0036 supersession of ADR-0026
typography clauses; chip rename) obsolete several work item ACs as
originally drafted. Update `meta/work/0075-typography-size-scale-consumption.md`
in the same PR so the merged state has work item, plan, and code
aligned. Concrete edits:

- **AC5** ("ADR-0026 is amended in place to address each of …") →
  reframe as: *"ADR-0036 is created with the canonical font-size
  consumption rule, and ADR-0026 is updated to record the typography
  portion's supersession per ADR-0031."* The three sub-ACs (AC5a/b/c)
  reframe accordingly:
  - **AC5a**: the consume-tokens-everywhere rule is codified in
    ADR-0036 §Decision.
  - **AC5b**: the §3 "em-relative font-sizes" and "Heading font-sizes
    above `size-lg`" rows in ADR-0026 are removed or marked
    superseded, and ADR-0036 supersedes them.
  - **AC5c**: ADR-0026's Consequences "heading font-size gap"
    deferral is updated to record resolution by ADR-0036 (matching
    the literal text the plan's Phase 1.3b applies, not coupling the
    AC to the ephemeral work-item ID).
- **AC6a** wording → update to: *"`src/styles/global.css` carries a
  comment block above the `--size-*` tokens whose lead sentence is
  `/* font-size consumers: use these tokens — see ADR-0036 */`,
  followed by a naming-convention paragraph (numeric tiers, semantic
  single-purpose tokens, `-sm`/`-lg` sub-pixel suffixes)."* The
  grep-able pass condition is the substring `font-size consumers:
  use these tokens — see ADR-0036`.
- **AC6b** wording → replace `ADR-0026` with `ADR-0036` in the
  rationale-paragraph clause.
- **Decisions** §"Existing `--size-chip` (10.5px) and `--size-chip-md`
  (11.5px) are reused outside chip components" → strike. Replace with:
  *"The chip-prefixed tokens are renamed to numeric-ladder names in
  this PR (`--size-chip` → `--size-3xs-lg`; `--size-chip-md` →
  `--size-xxs-sm`). The rename is mechanical and same-PR per the
  consumer enumeration in Phase 1.1b of the plan."*
- **Assumptions** §"Pre- and post-migration computed `font-size`
  values are identical for every selector …" — review for
  H1-rem-to-px implications (assumption holds only at default browser
  font-size; ADR-0036 documents the px-anchored stance).
- **References** → add ADR-0036 and follow-up work item 0091.

Verify after editing: `rg -n 'ADR-0026 \(as amended by 0075\)' meta/work/0075-typography-size-scale-consumption.md` returns 0 matches; `rg -n 'amended in place' meta/work/0075-typography-size-scale-consumption.md` returns 0 matches.

#### 1.7 — Scaffold follow-up work item 0091

ADR-0036 References and Phase 1.6 References both point at
`meta/work/0091-typography-rem-vs-px-stance.md`. The stub is created
as part of this PR so the references resolve.

**File** (new): `meta/work/0091-typography-rem-vs-px-stance.md`

Frontmatter (mirror the project's work-item convention from
0075/0090):

```yaml
---
work_item_id: "0091"
title: "Typography rem-vs-px stance review"
date: "<ISO timestamp at PR-creation time>"
author: <author>
type: spike
status: backlog
priority: low
parent: ""
tags: [design, frontend, tokens, typography, accessibility]
---
```

Body (minimum content for backlog status):

- **Summary**: one paragraph stating that this work item revisits
  ADR-0036's px-anchored stance for `--size-*` tokens.
- **Context**: cite ADR-0036's Consequences §Neutral (px-anchored
  stance), noting the accessibility trade-off and that this work
  item exists so the trade-off is not "shipped and forgotten".
- **Acceptance Criteria** (placeholder for spike output): (1)
  investigate real-world impact of the px-anchored stance; (2)
  decide one of {keep px-anchored, reintroduce rem for headings,
  reintroduce rem family-wide} with a new ADR or child story; (3)
  spike output is either a new ADR or a deferred child story.
- **Dependencies**: blocked-by 0075 (the migration must land first
  for the stance to be evaluated in production).
- **References**: ADR-0036, work item 0075, this plan.

The stub is sufficient for `status: backlog` — full refinement
happens when the spike is picked up.

### Success Criteria

#### Automated Verification

- [ ] `cd skills/visualisation/visualise/frontend && rg -n '--size-4xs|--size-3xs|--size-eyebrow|--size-row|--size-subtitle|--size-xxs-sm|--size-3xs-lg' src/styles/global.css` returns 7 lines (5 new + 2 renamed).
- [ ] `cd skills/visualisation/visualise/frontend && rg -n 'size-chip' src` returns 0 matches (rename complete across global.css, tokens.ts, Chip.module.css, Chip.test.tsx, FrontmatterTable.module.css, FrontmatterTable.test.tsx, and MarkdownRenderer.module.css:46).
- [ ] `rg -n 'font-size consumers: use these tokens' src/styles/global.css` returns 1 line.
- [ ] `npm test -- migration.test.ts` passes on first run (no new test block has been added yet; existing AC4 hygiene + tokens-declared blocks pass because tokens.ts is updated in lockstep with global.css).
- [ ] `npm run lint` passes.
- [ ] `npm run typecheck` passes.
- [ ] `npx playwright test --list --project=visual-regression | grep typography-resolved-sizes` lists every CASES entry (proves the spec is wired into the project run, not silently filtered).

#### Manual Verification

- [ ] ADR-0036 is present and self-contained (a reader can understand the rule + rationale + escape valve without needing to consult ADR-0026 first).
- [ ] ADR-0026 frontmatter records `superseded_by: "adr:ADR-0036"` (single ref, quoted string per ADR-0034's identity-value contract); the spacing rule and other non-typography clauses still read as authoritative.
- [ ] ADR-0026 §2 Typography subsection is a single "see ADR-0036" pointer; §3 no longer references "em-relative" or "Heading font-sizes" rows; Consequences no longer mentions the heading-token deferral as open.
- [ ] Playwright spec compiles; every CASES entry has its `{route, setup?, selector, expected}` quadruple resolved (no `<placeholder>` strings remain).
- [ ] Baseline verification working note records observed-vs-EXCEPTIONS counts for every literal to be decremented in Phases 2-7.
- [ ] Work item 0075 updated per Phase 1.6: `rg -n 'ADR-0026 \(as amended by 0075\)\|amended in place' meta/work/0075-typography-size-scale-consumption.md` returns 0 matches; AC5/AC6/Decisions/References reflect ADR-0036 + chip rename.
- [ ] Work item 0091 stub exists at `meta/work/0091-typography-rem-vs-px-stance.md` with `status: backlog`.

---

## Phase 2: Migrate `MarkdownRenderer`

### Overview

Replace the two MarkdownRenderer font-size literals. This is the
single deliberate-drift case: inline `<code>` moves from `0.88em` to
`var(--size-xs)` (`14px`), accepting that inline code in headings will
no longer scale up with its surrounding text.

**Per-heading-level drift magnitude** (inline `<code>` computed
`font-size` before vs after this migration):

- Inside H1 (post-migration `var(--size-h3)` = 28px): ~24.6px → 14px
  (~10px reduction).
- Inside H2 (post-migration `var(--size-lg)` = 22px): ~19.4px → 14px
  (~5px reduction).
- Inside H3 (post-migration `var(--size-sm)` = 16px): ~14.1px → 14px
  (~0.1px reduction — visually identical).
- Inside body text: ~14.1px → 14px (visually identical).

The trade-off is accepted to eliminate the em-relative outlier;
deliberate-drift before/after screenshots in **both H1 and H2**
(captured during Phase 2 and embedded in the PR description) document
the change. The H1 `1.75rem` (28px) maps cleanly to `var(--size-h3)`
with no visible delta at default browser font-size.

**Accessibility note**: migrating H1 from `1.75rem` to a fixed-px
token removes user-controllable root-font-size scaling for this
heading. ADR-0036 documents the px-anchored stance as deliberate;
follow-up work item 0091 tracks revisiting rem-based heading tokens
should the accessibility regression matter in practice.

### Changes Required

#### 2.1 — Replace literals

**File**: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css`

- Line 9 (`.markdown h1`): `font-size: 1.75rem;` →
  `font-size: var(--size-h3);`
- Line 53 (`.markdown code:not(pre code)`): `font-size: 0.88em;` →
  `font-size: var(--size-xs);`

#### 2.2 — Update EXCEPTIONS

**File**: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`

Delete these two entries (both font-size-only, count: 1):

- `{ file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '1.75rem', count: 1, ... reason: 'h1 font-size …' }`
- `{ file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '0.88em', count: 1, ... reason: 'relative em font-size on inline code …' }`

### Success Criteria

#### Automated Verification

- [ ] `npm test -- migration.test.ts` passes (AC4 hygiene accepts the file: no orphan EXCEPTIONS, no new outliers).
- [ ] `rg -n 'font-size:' src/components/MarkdownRenderer/MarkdownRenderer.module.css` returns only `var(--size-*)` references.
- [ ] `rg --glob '**/*.module.css' 'font-size:\s*[.0-9]' src/components/MarkdownRenderer` returns zero matches (this file's AC2 sweep is now clean).
- [ ] Playwright `typography-resolved-sizes` MarkdownRenderer cases pass (H1, inline code in body, inline code in H2).

#### Manual Verification

- [ ] Render a markdown doc containing inline code inside a heading — visually confirm inline code is now fixed-size (the documented deliberate drift). Capture a before/after screenshot for the PR description.
- [ ] Render a markdown doc with H1 — visually identical (1.75rem === 28px === `var(--size-h3)`).

---

## Phase 3: Migrate `Page` and `Brand`

### Overview

Two unrelated single-file migrations grouped for review economy. Both
introduce new tokens from Phase 1.

### Changes Required

#### 3.1 — `Page.module.css`

**File**: `skills/visualisation/visualise/frontend/src/components/Page/Page.module.css`

- Line 35 (`.eyebrow`): `font-size: 11px;` → `font-size: var(--size-eyebrow);`
- Line 56 (`.subtitle`): `font-size: 13px;` → `font-size: var(--size-subtitle);`

#### 3.2 — `Brand.module.css`

**File**: `skills/visualisation/visualise/frontend/src/components/Brand/Brand.module.css`

- Line 28 (`.brandSub`): `font-size: 10px;` → `font-size: var(--size-3xs);`

#### 3.3 — Update EXCEPTIONS

Delete (font-size-only entries):

- `Page.module.css` `11px` count: 1 — reason "eyebrow font-size from design …"
- `Page.module.css` `13px` count: 1 — reason "subtitle font-size from design …"
- `Brand.module.css` `10px` count: 1 — reason "VISUALISER sub-label …"

Brand's `2px` entry (text stack gap) is unrelated; leave intact.

### Success Criteria

#### Automated Verification

- [ ] `rg -n 'font-size:' src/components/Page/Page.module.css src/components/Brand/Brand.module.css` returns only `var(--size-*)` references.
- [ ] `npm test -- migration.test.ts` — AC4 still passes; AC2/0075 has 3 fewer red cases.
- [ ] Playwright `Page .eyebrow` / `Page .subtitle` / `Brand .brandSub` cases pass.

#### Manual Verification

- [ ] Visual diff at default viewport: Page eyebrow / subtitle / Brand sub-label identical to pre-migration.

---

## Phase 4: Migrate `Sidebar` (includes `font:` shorthand expansion)

### Overview

Sidebar has both `font-size:` literals (4 sites) and `font:` shorthand
sites (3) with embedded font-size literals. The three shorthand sites
must be expanded into individual `font-family` / `font-weight` /
`font-size` / `line-height` properties so the literal cannot hide
inside a compound declaration (work item Decisions).

### Changes Required

#### 4.1 — Direct `font-size:` migrations

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.module.css`

- Line 89 (`.libraryHeading`): `font-size: 10.5px;` → `font-size: var(--size-3xs-lg);`
- Line 107 (`.libraryHeadingHint`): `font-size: 10px;` → `font-size: var(--size-3xs);`
- Line 143 (`.sectionHeading`): `font-size: 10.5px;` → `font-size: var(--size-3xs-lg);`
- Line 160 (`.phaseHeading`): `font-size: 9.5px;` → `font-size: var(--size-4xs);`

#### 4.2 — Expand `font:` shorthand sites

For each of the three sites (lines 50, 75, 185 in the 2026-05-23
snapshot — locate by content, not by line number, as edits in 4.1
may shift them):

The CSS `font:` shorthand has well-defined reset semantics: any
properties not explicitly named are reset to their initial values
(`font-style: normal; font-variant: normal; font-stretch: normal;
font-size-adjust: none; font-kerning: auto;`). Expansion to
font-family/weight/size/line-height does NOT reset those properties —
they fall back to inheritance. To preserve the shorthand's reset
behaviour exactly, the expansion explicitly sets the most commonly
inheritable resets (`font-style`, `font-variant`, `font-stretch`) to
`normal`. (The current Sidebar/ActivityFeed contexts have no ancestor
setting italic/small-caps/condensed, so omitting the resets would
likely cause no visible delta — but the explicit reset is safer and
documents the intent.)

- Line 50: `font: 400 13px/1.5 var(--ac-font-body);`
  → split into:
  ```css
  font-family: var(--ac-font-body);
  font-style: normal;
  font-variant: normal;
  font-stretch: normal;
  font-weight: 400;
  font-size: var(--size-subtitle);
  line-height: var(--lh-normal); /* --lh-normal: 1.5 (global.css) */
  ```
- Line 75: `font: 400 11px/1 var(--ac-font-mono);`
  → split into:
  ```css
  font-family: var(--ac-font-mono);
  font-style: normal;
  font-variant: normal;
  font-stretch: normal;
  font-weight: 400;
  font-size: var(--size-eyebrow);
  line-height: 1;
  ```
  (line-height `1` has no token; ADR-0036 does not require
  unit-less line-heights to consume a `--lh-*` token. Leave as `1`.)
- Line 185: `font: 400 13px/1.5 var(--ac-font-body);` → same expansion
  as line 50.

#### 4.3 — Update EXCEPTIONS

Delete or decrement entries on `components/Sidebar/Sidebar.module.css`:

- `10.5px` count: 2 — reason "library and section heading font-size from design …" → **delete** (both occurrences are now migrated; no other 10.5px exists in this file).
- `9.5px` count: 1 — reason "phase heading font-size from design …" → **delete**.
- `11px` count: 1 — reason "kbd chip font-size — 1px under --size-xxs (12px)" → **delete** (this is the line-75 kbd shorthand we just migrated).
- `13px` count: 2 — reason "nav item label and search input font-size from design …" → **delete** (lines 50 and 185 shorthand sites).
- `10px` count: 6 — reason "heading, search-row, library-heading-hint and nav-item horizontal padding / font-size from design …" → **decrement to 5** and **rewrite reason** enumerating the concrete remaining use sites (e.g. `library heading inline padding, search-row padding, nav-item horizontal padding (×3)`). Do not just trim "font-size" from the existing text — replace it with a specific enumeration so Phase 8.1's reason audit has unambiguous content to verify.

**Cross-check before committing the decrement**:
- `rg -nc '10.5px' src/components/Sidebar/Sidebar.module.css` should return 0 (after migration).
- `rg -nc '9.5px' src/components/Sidebar/Sidebar.module.css` should return 0.
- `rg -nc '13px' src/components/Sidebar/Sidebar.module.css` should return 0.
- `rg -nc '10px' src/components/Sidebar/Sidebar.module.css` should return 5 (matches the decremented count).
- `rg -nc '11px' src/components/Sidebar/Sidebar.module.css` should return 0.

If any count diverges from the expected post-migration value, stop and reconcile — either the migration missed a site, or the pre-existing EXCEPTIONS entry was wrong (which Phase 1.5's baseline verification should have caught).

### Success Criteria

#### Automated Verification

- [ ] `rg -n 'font-size:' src/components/Sidebar/Sidebar.module.css` returns only `var(--size-*)` references.
- [ ] `rg -n 'font:' src/components/Sidebar/Sidebar.module.css` returns no shorthand with an embedded numeric size; the one remaining shorthand (line 223 `font: 400 var(--size-xxs)/1 …`) already uses a token and stays.
- [ ] `npm test -- migration.test.ts` — both AC4 entries for Sidebar accounted for; AC2/0075 has 7 fewer red cases (4 direct + 3 shorthand).
- [ ] Playwright `Sidebar .phaseHeading` case passes.

#### Manual Verification

- [ ] Sidebar visually unchanged: library heading, section heading, phase heading, nav item, search input, kbd chip, libraryHeadingHint.
- [ ] Keyboard shortcut chip (kbd) still renders at 11px / mono.

---

## Phase 5: Migrate `ActivityFeed` (`font:` shorthand)

### Overview

Discovered scope expansion: AC2 third sweep catches two
`font: 500 10.5px/1 …` shorthand sites in `ActivityFeed.module.css`
that the work item Context inventory missed. Migrate them on the same
shorthand-expansion pattern as Phase 4.

### Changes Required

#### 5.1 — Expand shorthand sites

**File**: `skills/visualisation/visualise/frontend/src/components/ActivityFeed/ActivityFeed.module.css`

Both `font: 500 10.5px/1 var(--ac-font-mono);` sites → split into (with
explicit `normal` resets for the inheritable properties the shorthand
would otherwise reset — see Phase 4.2 for rationale):

```css
font-family: var(--ac-font-mono);
font-style: normal;
font-variant: normal;
font-stretch: normal;
font-weight: 500;
font-size: var(--size-3xs-lg);
line-height: 1;
```

#### 5.2 — Update EXCEPTIONS

- `components/ActivityFeed/ActivityFeed.module.css` `10.5px` count: 2
  — reason "ACTIVITY heading + LIVE badge font-size — mirrors Sidebar
  section-heading sub-pixel value" → **delete** entirely.

### Success Criteria

#### Automated Verification

- [ ] `rg -n 'font:' src/components/ActivityFeed/ActivityFeed.module.css` returns no shorthand with an embedded numeric size.
- [ ] `npm test -- migration.test.ts` — AC4 still passes; AC2/0075 has 2 fewer red cases.
- [ ] Playwright `ActivityFeed heading` and `ActivityFeed live badge` cases pass.

#### Manual Verification

- [ ] ACTIVITY heading + LIVE badge visually unchanged on whichever route surfaces the activity feed.

---

## Phase 6: Migrate `SortPill` and `FilterPill`

### Overview

Pill components share patterns and EXCEPTIONS structure. SortPill has
3 font-size sites; FilterPill has 9. Both files also carry many
non-font-size literals that stay untouched.

### Changes Required

#### 6.1 — `SortPill.module.css`

**File**: `skills/visualisation/visualise/frontend/src/components/SortPill/SortPill.module.css`

- `.trigger`: `font-size: 12px;` → `font-size: var(--size-xxs);`
- `.menuHeader`: `font-size: 10.5px;` → `font-size: var(--size-3xs-lg);`
- `.menuItem`: `font-size: 12.5px;` → `font-size: var(--size-row);`

#### 6.2 — `FilterPill.module.css`

**File**: `skills/visualisation/visualise/frontend/src/components/FilterPill/FilterPill.module.css`

- `.trigger`: `font-size: 12px;` → `font-size: var(--size-xxs);`
- `.badge`: `font-size: 10px;` → `font-size: var(--size-3xs);`
- `.menuHeader`: `font-size: 10.5px;` → `font-size: var(--size-3xs-lg);`
- `.clearButton`: `font-size: 11px;` → `font-size: var(--size-eyebrow);`
- `.facetHeading`: `font-size: 10.5px;` → `font-size: var(--size-3xs-lg);`
- `.search input`: `font-size: 12px;` → `font-size: var(--size-xxs);`
- `.option`: `font-size: 12.5px;` → `font-size: var(--size-row);`
- `.optionCount`: `font-size: 11px;` → `font-size: var(--size-eyebrow);`
- `.noMatches`: `font-size: 11.5px;` → `font-size: var(--size-xxs-sm);`

#### 6.3 — Update EXCEPTIONS

**SortPill** (`components/SortPill/SortPill.module.css`):

- `12px` count: 1 — reason "trigger font-size from design …" → **delete**.
- `12.5px` count: 1 — reason "menu-item font-size from design …" → **delete**.
- `10.5px` count: 1 — reason "menu-header font-size from design …" → **delete**.

**FilterPill** (`components/FilterPill/FilterPill.module.css`):

- `14px` count: 1 — reason "checkbox column track — fixed pixel; no token equivalent" → **leave** (not font-size).
- `13px` count: 2 — reason "checkbox width/height …" → **leave** (not font-size).
- `12px` count: 2 — reason "trigger + search input font-size from design — equals --size-xxs but co-located" → **delete** (both occurrences are now migrated; the reason names *only* font-size uses).
- `12.5px` count: 1 — reason "option font-size from design …" → **delete**.
- `10.5px` count: 2 — reason "menu-header + facet-heading font-size from design …" → **delete**.
- `11px` count: 2 — reason "clear-button + option-count font-size from design …" → **delete**.
- `11.5px` count: 1 — reason "no-matches font-size from design — sub-pixel" → **delete**.
- `10px` — **arithmetic reconciliation required before this phase
  runs**: the current EXCEPTIONS entry reads `count: 3, reason:
  'trigger + badge + menu-header horizontal padding from design'` —
  i.e. names three padding uses. But the file also contains `.badge {
  font-size: 10px }`, which the entry does not separately attribute.
  Either (i) the actual file has 4 `10px` occurrences and the
  `declared == observed` hygiene test is currently failing (caught by
  Phase 1.5's baseline verification), or (ii) one of the "three
  padding" descriptions in the reason is in fact the badge font-size
  and the reason text mislabels it.

  **Procedure**:

  1. On pristine `main` (via the Phase 1.5 baseline workspace), run
     `rg -nc '10px' src/components/FilterPill/FilterPill.module.css`
     and record the observed count.
  2. **If observed == 3**: the EXCEPTIONS reason text mis-attributes
     the `.badge` font-size occurrence as "badge horizontal padding";
     the three observed sites are in fact two real paddings (lines 6,
     71) plus one font-size on `.badge` (line 45). The count of 3 is
     correct, but the reason currently mislabels one site.
     Post-migration count is **2** (the two paddings remain after the
     badge font-size is migrated to `var(--size-3xs)`). Rewrite the
     EXCEPTIONS reason to enumerate the two concrete remaining
     padding sites by selector (e.g. `trigger padding-inline at
     .trigger; search-area padding at .search`).
  3. **If observed == 4**: the EXCEPTIONS count was undercounted (the
     existing `declared == observed` hygiene test should already be
     failing). Land a precursor fix as the first commit in this PR,
     bringing the entry to `count: 4` with a complete enumeration of
     all four sites, before applying this phase's decrement.
     Post-migration count is then **3** (4 minus the badge font-size).
  4. In either case, the post-migration reason must enumerate concrete
     remaining use sites by selector, not just a category label. The
     Phase 8.3 audit verifies this.
  5. **If observed is neither 3 nor 4** (e.g. 2, or 5+), or if
     `.badge { font-size: 10px }` is not present in the file as the
     plan assumes: **stop and re-scope this phase**. The file has
     drifted since the 2026-05-23 codebase audit and the migration
     targets need a fresh walk before any commits land in Phase 6.

  Until step 1 is run, do not commit a decrement value — both `2` and
  `3` are plausible, and step 5 covers the case where neither
  applies.

### Success Criteria

#### Automated Verification

- [ ] `rg -n 'font-size:' src/components/SortPill/SortPill.module.css src/components/FilterPill/FilterPill.module.css` returns only `var(--size-*)` references.
- [ ] `npm test -- migration.test.ts` — AC4 still passes; AC2/0075 has 12 fewer red cases (3 SortPill + 9 FilterPill).
- [ ] Playwright `SortPill .menuItem` and `FilterPill .option` cases pass.

#### Manual Verification

- [ ] SortPill trigger / menu header / menu items visually unchanged.
- [ ] FilterPill trigger / badge / menu header / clear button / facet heading / search input / option / option count / no-matches messaging visually unchanged.

---

## Phase 7: Migrate library routes

### Overview

`EmptyState` (5 sites), `LibraryOverviewHub` (4 sites), and
`LibraryTypeView` (5 sites) all live under `src/routes/library/`.
Grouped because they share the route directory and the same review
cadence; otherwise independent.

### Changes Required

#### 7.1 — `EmptyState.module.css`

**File**: `skills/visualisation/visualise/frontend/src/routes/library/EmptyState.module.css`

- `.eyebrow`: `font-size: 11.5px;` → `font-size: var(--size-xxs-sm);`
- `.title`: `font-size: 22px;` → `font-size: var(--size-lg);`
- `.lede`: `font-size: 14px;` → `font-size: var(--size-xs);`
- `.foot`: `font-size: 12px;` → `font-size: var(--size-xxs);`
- `.pathInline`: `font-size: 11.5px;` → `font-size: var(--size-xxs-sm);`

EXCEPTIONS updates (`routes/library/EmptyState.module.css`). Rewrite reasons to enumerate concrete remaining use sites; do not just trim "font-size" from the existing text.

- `11.5px` count: 2 — reason "eyebrow + path-inline font-size …" → **delete**.
- `22px` count: 2 → **decrement to 1**; new reason: `card responsive padding-block at .card`.
- `14px` count: 2 → **decrement to 1**; new reason: `foot padding-top at .foot`.
- `12px` count: 2 → **decrement to 1**; new reason: `card border-radius at .card`.

**Cross-check before committing**:
- `rg -nc '11.5px' src/routes/library/EmptyState.module.css` should return 0.
- `rg -nc '22px' src/routes/library/EmptyState.module.css` should return 1.
- `rg -nc '14px' src/routes/library/EmptyState.module.css` should return 1.
- `rg -nc '12px' src/routes/library/EmptyState.module.css` should return 1.

#### 7.2 — `LibraryOverviewHub.module.css`

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryOverviewHub.module.css`

- `.phaseHeading`: `font-size: 11px;` → `font-size: var(--size-eyebrow);`
- `.cardLabel`: `font-size: 14px;` → `font-size: var(--size-xs);`
- `.cardCount`: `font-size: 11px;` → `font-size: var(--size-eyebrow);`
- `.cardLatest`: `font-size: 11.5px;` → `font-size: var(--size-xxs-sm);`

EXCEPTIONS updates (`routes/library/LibraryOverviewHub.module.css`). Use the actual class name `.cardLabel` (not "card title") when rewriting reasons:

- `11px` count: 2 — reason "phase-heading + card-count font-size …" → **delete**.
- `14px` count: 2 → **decrement to 1**; new reason: `card padding-block at .card` (verify by grep that the remaining `14px` site is in fact the card padding, not another forgotten font-size).
- `11.5px` count: 1 — reason "card subtitle font-size …" → **delete**.

**Cross-check before committing**:
- `rg -nc '11px' src/routes/library/LibraryOverviewHub.module.css` should return 0.
- `rg -nc '14px' src/routes/library/LibraryOverviewHub.module.css` should return 1.
- `rg -nc '11.5px' src/routes/library/LibraryOverviewHub.module.css` should return 0.

#### 7.3 — `LibraryTypeView.module.css`

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.module.css`

- `.headerRow`: `font-size: 10.5px;` → `font-size: var(--size-3xs-lg);`
- `.row`: `font-size: 13px;` → `font-size: var(--size-subtitle);`
- `.firstCol`: `font-size: 12px;` → `font-size: var(--size-xxs);`
- `.slug`: `font-size: 11.5px;` → `font-size: var(--size-xxs-sm);`
- `.mtime`: `font-size: 11.5px;` → `font-size: var(--size-xxs-sm);`

EXCEPTIONS updates (`routes/library/LibraryTypeView.module.css`):

- `10.5px` count: 1 — reason "column-header font-size …" → **delete**.
- `13px` count: 1 — reason "row body font-size …" → **delete**.
- `11.5px` count: 2 — reason "slug + mtime font-size …" → **delete**.
- `12px` count: 3 → **decrement to 2**; new reason: `header-row padding-inline at .headerRow; row padding-inline at .row` (enumerate the two remaining concrete sites).

**Cross-check before committing**:
- `rg -nc '10.5px' src/routes/library/LibraryTypeView.module.css` should return 0.
- `rg -nc '13px' src/routes/library/LibraryTypeView.module.css` should return 0.
- `rg -nc '11.5px' src/routes/library/LibraryTypeView.module.css` should return 0.
- `rg -nc '12px' src/routes/library/LibraryTypeView.module.css` should return 2.

### Success Criteria

#### Automated Verification

- [ ] `rg -n 'font-size:' src/routes/library/EmptyState.module.css src/routes/library/LibraryOverviewHub.module.css src/routes/library/LibraryTypeView.module.css` returns only `var(--size-*)` references.
- [ ] `npm test -- migration.test.ts` — AC4 still passes; AC2/0075 has 14 fewer red cases.
- [ ] Playwright `EmptyState .title` and `LibraryTypeView .row` cases pass.

#### Manual Verification

- [ ] EmptyState (e.g. an empty library/by-type route) visually unchanged.
- [ ] LibraryOverviewHub `/library` index visually unchanged.
- [ ] LibraryTypeView (any `/library/<type>` route) visually unchanged.

---

## Phase 8: Land enforcement test, final sweep, rollback strategy, and PR description

### Overview

All migrations are done by this phase. The remaining work is to land
the new vitest category-level ban (which should be green on first
run), prove zero matches across all three AC2 sweeps, audit
EXCEPTIONS reasons (AC4b), document the rollback strategy in the PR
description, and assemble the PR description (AC6b).

### Changes Required

#### 8.1 — Add new vitest enforcement: ban font-size literals

**File**: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`

Append a new `describe` block at the end of the file asserting that no
module CSS or global CSS contains a `font-size:` literal or a `font:`
shorthand with an embedded numeric size. This is a category-level ban
that bypasses `EXCEPTIONS` entirely.

```ts
// Load styles/global.css through the same Vite glob mechanism that
// already loads *.module.css and *.global.css — avoids needing
// node:fs + ESM __dirname handling, and keeps the test self-contained
// in the Vite module graph.
const globalCssModules = import.meta.glob('../styles/global.css', {
  eager: true,
  query: '?raw',
  import: 'default',
}) as Record<string, string>

// Assert at module load that the glob resolved exactly one file. If
// global.css is moved or the glob pattern drifts, this fires loudly
// instead of silently dropping global.css from the test scope.
if (Object.keys(globalCssModules).length !== 1) {
  throw new Error(
    `Expected exactly one match for global.css glob, got ${Object.keys(globalCssModules).length}: ` +
    `${Object.keys(globalCssModules).join(', ')}. ` +
    `Update Phase 8.1's glob path if styles/global.css was moved.`,
  )
}

describe('AC2 / 0075: no font-size literals in module or global CSS', () => {
  // Matches `font-size: <number><unit>` with optional leading-dot
  // decimals (.88em as well as 0.88em). The lookbehind `(?<![\w-])`
  // requires `font-size:` to be at a property-position boundary, so
  // it does NOT match custom-property declarations whose name happens
  // to contain `font-size:` as a substring (e.g. `--my-font-size: 12px`
  // or `--ac-font-size-base: 14px`). Word and hyphen are excluded
  // because CSS custom-property names use `[a-zA-Z0-9_-]`.
  const FONT_SIZE_LITERAL_RE = /(?<![\w-])font-size:\s*(\d+(?:\.\d+)?|\.\d+)\s*(px|rem|em)\b/g

  // Matches `font:` shorthand containing a numeric size literal in the
  // font-size slot. The negated class `[^;/]*?` consumes everything
  // up to (but not including) the line-height separator `/` or the
  // declaration terminator `;`. This structurally prevents the engine
  // from advancing into the line-height position, so even fractional
  // line-heights with units (e.g. `/1.5rem`) cannot be matched.
  //
  // The lookbehind `(?<![\w-])` enforces property-position so
  // `font-family:`, `font-weight:`, etc. don't accidentally match
  // (they don't contain `font:` as a literal substring, but the
  // lookbehind defends against future property names with `font:`
  // as a prefix-substring).
  //
  // Cases the regex covers (positive):
  //   font: 400 12px/1.5 …      (canonical form)
  //   font:12px/1 …             (no space after colon)
  //   font:.5px/1 …             (leading-dot decimal, colon-adjacent)
  //   font: italic 400 .9em/1 … (leading-dot decimal in shorthand)
  //   font: bold 14px sans;     (no line-height slot)
  //
  // Cases the regex correctly skips (negative):
  //   font: 400 var(--size-xxs)/1.5rem …
  //     (the `[^;/]*?` cannot cross the `/` to reach the line-height)
  //   font: 400 var(--size-xxs)/1 …      (no unit after `/` anyway)
  //   font-family: "Inter", …            (different property)
  //   --ac-font-size-base: 12px;         (custom property; lookbehind rejects)
  //
  // The AC2 third sweep is a less strict variant; this regex is the
  // authoritative implementation per Phase 8.2's comment.
  const FONT_SHORTHAND_RE = /(?<![\w-])font:[^;/]*?(\d+(?:\.\d+)?|\.\d+)(px|rem|em)\b/g

  // allCss is the existing modules + *.global.css glob. The root
  // styles/global.css is intentionally additional — without it,
  // cssGlobals (which only picks up *.global.css) would leave
  // global.css uncovered.
  const allCssWithRoot = {
    ...allCss,
    ...Object.fromEntries(
      Object.entries(globalCssModules).map(([_, css]) => ['styles/global.css', css]),
    ),
  }

  for (const [path, css] of Object.entries(allCssWithRoot)) {
    it(`${path}: no font-size literal or shorthand with embedded size`, () => {
      const fontSizeHits = [...css.matchAll(FONT_SIZE_LITERAL_RE)].map((m) => m[0])
      const shorthandHits = [...css.matchAll(FONT_SHORTHAND_RE)].map((m) => m[0])
      expect({ fontSize: fontSizeHits, shorthand: shorthandHits }).toEqual({
        fontSize: [],
        shorthand: [],
      })
    })
  }
})
```

Notes on the regex choices, justifying each shape difference from the
original AC2 sweep:

- **Leading-dot decimals**: `(?:\d+(?:\.\d+)?|\.\d+)` matches both
  `0.88em` and `.88em` (the AC2 sweep's `[.0-9]+` matches both
  accidentally but also matches a lone `.` which the test should not
  treat as a literal).
- **`font-size:` anchor**: prevents matching `--my-size: 12px` (custom
  property declarations whose name happens to start with `--`-prefixed
  text other than `font-size`).
- **Shorthand negated class `[^;/]*?`**: structurally bounds the
  region the engine can search to the pre-`/` portion of the
  shorthand. CSS `font:` shorthand syntax places the line-height (if
  present) immediately after `/` following the font-size; the font-size
  itself must therefore appear before any `/`. Excluding `/` from the
  consumable class prevents the engine from ever reaching the
  line-height position, including the subtle case where a fractional
  line-height (e.g. `/1.5rem`) could otherwise be partially matched at
  the `.5rem` boundary. A negative lookbehind alternative was tried
  in an earlier revision (`(?<!\/)`) but failed because lookbehind only
  inspects one character — it correctly excluded matches starting at
  the digit immediately after `/`, but the lazy quantifier could still
  advance past the `1` of `1.5rem` and start matching at the `5`.

- **Property-position lookbehind `(?<![\w-])` on both regexes**:
  prevents matching CSS custom-property declarations whose name
  contains `font-size:` or `font:` as a substring. CSS custom-property
  names use `[a-zA-Z0-9_-]` (word characters plus hyphen), so a
  preceding word-or-hyphen character means we're inside a custom
  property name, not at a property declaration. This defends against
  hypothetical declarations like `--my-font-size: 12px;` (matches
  `font-size:` substring) and `--ac-font: bold 14px sans;` (matches
  `font:` substring).
- **Single `it()` per file with structured object expectation**:
  collapses the previous two-tests-per-file shape into one, and the
  object expectation makes the failure message identify *which*
  category (font-size vs shorthand) failed without doubling test
  count.
- **`import.meta.glob` for global.css**: uses Vite's existing module
  graph (matching how `allCss` and `cssGlobals` are constructed in the
  same test file), avoiding `node:fs` + the ESM-vs-CJS `__dirname`
  ambiguity. No additional imports are needed.

Add fixture-style edge-case unit tests in the same describe (or a
sibling describe) exercising the regex against synthetic CSS strings.
Each fixture is a positive (must match) or negative (must not match)
case:

**Positive cases (the regex must flag these):**

- `font-size: 0.88em;`
- `font-size: .5px;`
- `font-size:14px;` (no space after colon)
- `font: 400 12px/1.5 var(--ac-font-body);` (canonical shorthand)
- `font:12px/1 sans;` (no space after `font:`)
- `font: italic 400 .9em/1 sans;` (leading-dot decimal in shorthand)
- `font:.5px/1 sans;` (colon-adjacent leading-dot)
- `font: bold 14px sans;` (shorthand with no line-height slot)

**Negative cases (the regex must NOT flag these):**

- `--size-foo: 11px;` (custom property declaration; the `font-size:`
  property-position lookbehind must reject)
- `--my-line-height: 1.5rem;` (custom property; must not match the
  `font:` regex either, since `font:` isn't present)
- `--my-font-size: 12px;` (custom property whose name ends in
  `font-size:` — the `(?<![\w-])` property-position lookbehind on
  FONT_SIZE_LITERAL_RE must reject because the preceding char `-` is
  in the excluded class. This is the explicit lookbehind exerciser
  for the literal regex.)
- `--ac-font-size-base: 14px;` (custom property whose name *contains*
  `font-size` followed by `-base` — the literal substring `font-size:`
  doesn't appear because `-base` intervenes, so the regex never begins
  matching. Included as a defence-in-depth fixture against future
  property-name shapes.)
- `--ac-font: bold 14px sans;` (custom property ending in `font:` —
  the property-position lookbehind on FONT_SHORTHAND_RE must reject
  because the preceding char `-` is in the excluded class. This is the
  explicit lookbehind exerciser for the shorthand regex.)
- `/* migrated font-size: 12px to var(--size-xxs) */` (CSS comment;
  must not match because comments are stripped before scanning. See
  `stripComments` in the loop body.)
- `font-family: "Inter", sans-serif;` (must not match `font:`)
- `font: 400 var(--size-xxs)/1 var(--ac-font-mono);` (token-bearing
  shorthand, no unit on line-height)
- `font: 400 var(--size-xxs)/1.5rem var(--ac-font-body);` (token
  font-size + unit-bearing line-height — the `[^;/]*?` negated class
  cannot cross the `/` to reach `1.5rem`)
- `font: 14pxsans;` (typo with no boundary between unit and family;
  the `\b` after the unit must reject)
- `border-radius: 12px;` (unrelated property)
- `transform: scale(1.2em);` (unrelated property; the `font-size:`
  anchor must reject)

**Escape valve (`FONT_SIZE_LITERAL_EXCEPTIONS`)**: the implementation
ships with an empty array beside the describe block, with the loop
subtracting matched literals before the equality check. This makes the
ADR-0036 escape-valve contract executable (rather than aspirational)
and gives Phase 8.4's surgical-revert posture a one-line addition
path. The exception shape mirrors `EXCEPTIONS` with a `file`, a
`literal` (the bare size+unit, e.g. `'12px'`), a `count` for
per-occurrence semantics, plus `reason` and `reference` for
documentation. Matching is done against the captured size+unit
pair from the regex (`m[1] + m[2]`), not the full match string, so
the entry's `literal` value matches the natural shape implementers
will write:

```ts
const FONT_SIZE_LITERAL_EXCEPTIONS: ReadonlyArray<{
  file: string
  literal: string  // bare size+unit, e.g. '12px' or '0.88em'
  count: number   // per-occurrence; matches EXCEPTIONS semantics
  reason: string
  reference: string  // ADR or work item ID
}> = []  // routine use not permitted; see ADR-0036 §Escape valve

// Strip CSS comments before matching so that documentation comments
// referencing literals (e.g. `/* migrated font-size: 12px to var(--size-xxs) */`)
// don't false-positive. The regexes work on the comment-stripped form.
const stripComments = (css: string) => css.replace(/\/\*[\s\S]*?\*\//g, '')

// Capture group 1 holds the size, group 2 holds the unit. The
// exemption key is the bare `size+unit` string (e.g. `'12px'`,
// `'0.88em'`), matching the natural shape implementers will write in
// `FONT_SIZE_LITERAL_EXCEPTIONS` entries.
const cssNoComments = stripComments(css)
const fontSizeHits = [...cssNoComments.matchAll(FONT_SIZE_LITERAL_RE)].map((m) => `${m[1]}${m[2]}`)
const shorthandHits = [...cssNoComments.matchAll(FONT_SHORTHAND_RE)].map((m) => `${m[1]}${m[2]}`)

const exemptForFile = new Map<string, number>()
for (const e of FONT_SIZE_LITERAL_EXCEPTIONS) {
  if (e.file === path) {
    exemptForFile.set(e.literal, (exemptForFile.get(e.literal) ?? 0) + e.count)
  }
}

// Shared `consumed` Map across both category subtractions so that a
// single `count: 1` entry exempts ONE occurrence total (whether it's
// a `font-size:` literal or a `font:` shorthand), not one of each. If
// a file genuinely has both a literal and a shorthand of the same
// value that both need exempting, set `count: 2`.
const consumed = new Map<string, number>()
function subtractExemptions(hits: string[]): string[] {
  const remaining: string[] = []
  for (const hit of hits) {
    const budget = exemptForFile.get(hit) ?? 0
    const used = consumed.get(hit) ?? 0
    if (used < budget) {
      consumed.set(hit, used + 1)
    } else {
      remaining.push(hit)
    }
  }
  return remaining
}

expect({
  fontSize: subtractExemptions(fontSizeHits),
  shorthand: subtractExemptions(shorthandHits),
}).toEqual({ fontSize: [], shorthand: [] })
```

The per-occurrence `count` field lets a file have multiple instances
of the same literal where only some are exempted — matching the
existing `EXCEPTIONS` semantics so the convention is consistent. The
shared `consumed` Map means the budget is "occurrences of this
literal in this file, regardless of which regex matched it"; an
implementer adding an exemption thinks in source-site terms, not
regex-category terms.

#### 8.2 — Run all three AC2 sweeps locally

```bash
cd skills/visualisation/visualise/frontend
rg --glob '**/*.module.css' 'font-size:\s*[.0-9]' src
rg --glob '**/*.css' --glob '!**/global.css' 'font-size:\s*[.0-9]' src
rg --glob '**/*.module.css' 'font:\s*[^;]*\s[.0-9]+(px|rem|em)' src
```

All three must return zero matches. Any hit is a bug in an earlier
phase. The vitest assertion from 8.1 is the authoritative
implementation of these sweeps.

##### Authoritative-impl comment (mandatory)

Add a comment block immediately above the Phase 8.1 `describe` in
`migration.test.ts`:

```ts
// AUTHORITATIVE IMPLEMENTATION of ADR-0036's font-size consumption
// rule. The FONT_SIZE_LITERAL_RE and FONT_SHORTHAND_RE regexes here
// are the load-bearing CI guard. The three AC2 ripgrep sweeps below
// are coarser approximations used at the review-time grep gate; the
// regexes above are the authoritative test of compliance. The AC2
// sweeps may flag additional candidates (e.g. unit-bearing
// line-heights inside `font:` shorthand) that this test correctly
// excludes via the negative-lookbehind on the shorthand regex; any
// such grep hit must be inspected and dismissed only if confirmed to
// be the line-height slot. Any structural edit to the regexes here
// requires re-deriving the AC2 sweep guidance in the same commit.
//
// AC2 sweeps (review-time grep gate):
//   rg --glob '**/*.module.css' 'font-size:\s*[.0-9]' src
//   rg --glob '**/*.css' --glob '!**/global.css' 'font-size:\s*[.0-9]' src
//   rg --glob '**/*.module.css' 'font:\s*[^;]*\s[.0-9]+(px|rem|em)' src
```

This is non-optional — without it, the next contributor editing
either side has no breadcrumb that the two must move together.

##### CI script (optional uplift)

Optionally, add a small CI script that runs the three `rg` sweeps and
fails the build on any hit — a redundant safety net beside the vitest
test. Skip if CI configuration is non-trivial to extend in this PR;
the authoritative-impl comment above suffices as the minimum bar.

#### 8.3 — EXCEPTIONS reason audit (AC4b)

**File**: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`

Run `rg -n 'font-size' src/styles/migration.test.ts`. For every
remaining hit inside an EXCEPTIONS `reason` field, rewrite the reason
to enumerate the concrete remaining use sites (e.g.
`grid gap at .cardGrid; card top-row gap at .cardTopRow`) rather than
just trimming "font-size". The Phase 8.1 audit is the last chance to
catch reason rot from the incremental decrements in Phases 4/6/7.

**Additional checklist**: each remaining EXCEPTIONS entry's reason
should answer the question "if a future contributor adds another
literal of value X to this file, can they tell from the reason
whether their case is already covered?". If the answer is no, rewrite
the reason.

#### 8.4 — Rollback strategy

Decide the rollback posture before merge and document it in the PR
description. Because the new vitest test (8.1) blocks reintroduction
of literals, a partial revert (one file's migration only) cannot
land cleanly without also adding a `FONT_SIZE_LITERAL_EXCEPTIONS`
entry (see ADR-0036's escape valve, implemented in 8.1).

**Decision rubric** (apply to the most-significant regression
identified post-merge):

| Regression signal | Posture | Decision authority |
|---|---|---|
| Visual-regression Playwright spec fails post-merge | Default (full revert) | PR author |
| `npm test` failure attributable to this PR | Default (full revert) | PR author |
| User-reported visual issue, root cause identified within 24h, affecting **exactly one** file | Surgical | PR author + 1 reviewer |
| User-reported visual issue, root cause unclear, OR affecting > 1 file | Default (full revert) | PR author |
| Accessibility complaint about heading sizing | Default (full revert) + escalate to 0091 | PR author + a11y reviewer |

**Default posture (full revert) — command sequence**:

```bash
# 1. Identify the merge commit for the 0075 PR.
jj log -r 'description(glob:"*0075*") & merges()' --no-graph --limit 1 -T 'change_id ++ "\n"'
#    Capture the change_id into MERGE_ID for the next steps.

# 2. Create a backout change. `jj backout` produces a new change whose
#    diff reverses the named change; the new change starts with a
#    default description (e.g. "Back out commit <id>").
jj backout --revision $MERGE_ID

# 3. Capture the backout change's id and redescribe it explicitly.
BACKOUT_ID=$(jj log -r 'description(glob:"Back out*0075*")' --no-graph --limit 1 -T 'change_id')
jj describe -r $BACKOUT_ID -m "Revert 0075 typography migration — <regression summary>"

# 4. Push to remote.
jj git push
```

Because ADR-0036 was created by the merged PR, the revert also removes
ADR-0036 and restores ADR-0026's typography content (the textual
typography subsection is restored alongside the supersession linkage
being removed). The created-but-empty work item 0091 stub remains in
the repository — leave it for the follow-up; do not also revert 0091.

**Surgical posture (partial revert via escape valve) — command sequence**:

1. Identify the offending file and the literal that needs restoring
   (e.g. `Sidebar.module.css` `.libraryHeading` `10.5px`).
2. Restore the pre-migration version of just that file via `jj file`
   (do *not* use `jj split`, which is interactive and not the right
   tool here):
   ```bash
   # Restore the file from the parent of the merge commit:
   jj file show -r $MERGE_ID- <file> > <file>
   # Or restore from main if pre-merge state is known to be on main:
   jj file show -r main- <file> > <file>
   # Create a new commit with the restored file:
   jj commit -m "Restore <selector> literal in <file> pending follow-up"
   ```
   Then **also** un-migrate any related token consumer changes in the
   same file that depended on the literal being absent (e.g. the
   EXCEPTIONS entry's count needs to go back up by the restored
   occurrence).
3. Add a `FONT_SIZE_LITERAL_EXCEPTIONS` entry per the shape in 8.1:
   ```ts
   {
     file: 'components/<Component>/<Component>.module.css',
     literal: '12px',
     count: 1,
     reason: 'reverting <selector> migration — <one-line regression summary>',
     reference: 'work-item:0XXX (follow-up to remove within 1 week)',
   }
   ```
4. File the follow-up work item immediately. The follow-up must
   reach `done` status within **1 week** — note that this is shorter
   than the 12-week ADR-0036 escape-valve sunset which governs
   *general* exceptions (third-party CSS, etc.); a surgical-revert
   exception has a tighter SLA because it is patching over a known
   regression rather than admitting a legitimate one-off literal. If
   the 1-week SLA slips, escalate to the default posture (full revert).

The PR description must (a) state which posture applies if a
regression is detected post-merge, and (b) note the decision
authority for the surgical posture (typically the PR author plus one
reviewer who can verify the localisation claim).

#### 8.5 — PR description

Compose the PR description body. Must contain:

- The verbatim rule statement *"every `font-size` declaration in
  current-app CSS must resolve to a `var(--size-*)` token"*.
- A one-paragraph rationale referencing ADR-0036 by ID (and noting
  ADR-0026 as the scope-limited supersession source).
- A "Deliberate drift" subsection with before/after screenshots of
  inline `<code>` inside an H1 *and* an H2 (captured during Phase 2);
  enumerate the per-heading-level deltas (H1: ~24.6 → 14px; H2: ~19.4
  → 14px; H3: ~14.1 → 14px).
- A "Phases" subsection summarising the eight phases. Minimum content
  per phase: one line naming the affected files and the migration
  verb (e.g. `Phase 4 — Sidebar: 4 direct migrations + 3 font-shorthand
  expansions`). Mirror the plan's phase headers; mechanical to write.
- A "Rollback strategy" subsection per 8.4.

### Success Criteria

#### Automated Verification

- [ ] All three AC2 ripgrep sweeps return zero matches.
- [ ] `rg -n 'font-size' src/styles/migration.test.ts` — no hits inside `reason:` strings (matches in comments / test-name strings are acceptable).
- [ ] `npm test` — full vitest suite green, including the new AC2/0075 describe block (green on first run because migration is complete).
- [ ] `npm run e2e -- --project=visual-regression` — full visual-regression suite green, including `typography-resolved-sizes.spec.ts`.
- [ ] `npm run lint` and `npm run typecheck` green.

#### Manual Verification

- [ ] Visual smoke-test of each migrated component / route at default viewport — no surprises beyond the documented deliberate drift on inline `<code>`.
- [ ] PR description contains verbatim rule statement, ADR-0036 reference, before/after screenshots in H1 and H2, phases summary, and rollback strategy.
- [ ] ADR-0036 reads coherently end-to-end; ADR-0026's surviving content (spacing, etc.) is still authoritative and clearly scoped.

---

## Testing Strategy

### Unit / Integration Tests (vitest)

- New AC2/0075 describe block in `migration.test.ts` (added in
  **Phase 8**, not Phase 1) is the category-level enforcement: any
  reintroduced `font-size` literal in any module or global CSS
  (including the root `styles/global.css`, which is added explicitly
  to the test's scope) fails the suite. It is the authoritative
  implementation of the AC2 ripgrep sweeps — any edit to either side
  must be mirrored.
- The block lands in Phase 8 (not Phase 1) so the vitest suite stays
  green throughout Phases 2-7. Phases 2-7 are driven instead by the
  AC2 ripgrep sweeps (run per phase) and by the existing `declared
  count equals observed count` hygiene check on EXCEPTIONS.
- Existing AC3 (hex) / AC4 (px/rem/em) / `var(--*)` describe blocks
  continue to enforce per-occurrence EXCEPTIONS for *non*-font-size
  literals.
- `prototype-tokens.fixture.test.ts` does not assert on `--size-*`
  tokens (per work item Assumptions), so the five new tokens plus the
  two renamed chip tokens will not break it. Confirm with a single
  dry test run after Phase 1.

### End-to-End Tests (Playwright)

- `typography-resolved-sizes.spec.ts` asserts the post-migration
  computed `font-size` for one representative selector per outlier
  file group plus the two value-transition cases (MarkdownRenderer H1
  `1.75rem` → `28px`; inline code `0.88em` → `14px`), the ActivityFeed
  extension, and the deliberate-drift regression case (inline `<code>`
  inside an H2 → `14px`).
- Viewport pinned to 1280×720 inside the spec.
- Theme not pinned: the `--size-*` scale is theme-invariant in this
  codebase (no `data-theme="dark"` overrides for sizes), so
  single-theme coverage suffices. A header comment in the spec
  documents this asymmetry against the `chip-resolved-colours.spec.ts`
  precedent (which loops both themes because chip colours diverge).
- Expected px values are currently hardcoded. If a future legitimate
  token-scale tweak fires this spec, prefer either (a) re-deriving the
  expected px from `TYPOGRAPHY_TOKENS` in the spec setup, or (b)
  updating the hardcoded value with the token change in the same PR.
  The spec header comment notes this maintenance contract.

- **Per-selector Playwright coverage gap**: the spec covers one
  representative selector per outlier file group (plus the two
  value-transition cases and the deliberate-drift regression). It does
  *not* cover every migrated `font-size` site — FilterPill alone has
  9 migrated sites covered by 1 Playwright case. The vitest Phase 8.1
  ban only asserts *no literal exists*; it does not assert *the right
  token was chosen*. A wrong-token migration (e.g. `.menuHeader`
  accidentally using `--size-3xs` instead of `--size-3xs-lg`) would
  pass vitest and AC2 sweeps but only Phase 6's Manual Verification
  visual smoke-test would catch the ~0.5–1px delta.

  This is accepted as the cost of representative coverage — exhaustive
  Playwright cases would roughly triple the spec's size for marginal
  protection on a one-time mechanical migration. The risk is bounded
  by: (a) the mapping table in Phases 2-7 is reviewer-checkable from
  the diff alongside the CSS edits, (b) Phase 1.5's baseline
  verification surfaces token-value mismatches against the EXCEPTIONS
  ledger, (c) Phase 8.3's reason audit forces every migrated site to
  be enumerated. If a future contributor changes the migration
  approach (e.g. a re-tier of the scale), they should re-evaluate
  this coverage stance.

### Manual Testing Steps

1. From the frontend dir: `npm run dev` and open each affected route
   (Page-rendered routes, library index, library by-type, kanban if
   it surfaces Brand, anywhere markdown renders).
2. Compare against `main` on each component: Sidebar, Brand, Page,
   ActivityFeed, MarkdownRenderer body + heading + inline code,
   SortPill / FilterPill triggers and menus, EmptyState, LibraryHub,
   LibraryTypeView.
3. Capture before/after screenshots of inline `<code>` inside an
   **H1 and an H2** for the PR description (deliberate drift — H1 is
   where the magnitude is largest at ~24.6 → 14px, H2 at ~19.4 → 14px).
4. Inspect the rendered Sidebar `font:` shorthand sites — confirm the
   visual output is identical to pre-migration (shorthand expansion
   must preserve all four constituent properties).

## Performance Considerations

None. Tokens resolve at CSS parse time; no runtime cost.

## Migration Notes

- `prototype-standalone.html` is unchanged. The drift fixture for
  prototype-tokens does not include `--size-*` tokens.
- No data migration; no backwards-compatibility concerns.
- ADR-0026 has the typography portion of its content superseded by new
  ADR-0036 per ADR-0031's immutability convention; the scope-limited
  supersession is recorded via ADR-0034 typed linkage
  (`superseded_by: "adr:ADR-0036"`) and identified textually in both
  ADRs. ADR-0026 retains authority over its spacing rule and other
  non-typography clauses; ADR-0036 owns the typography font-size
  consumption rule.
- The chip token rename (`--size-chip` → `--size-3xs-lg`,
  `--size-chip-md` → `--size-xxs-sm`) lands in the same PR as the
  consumption migration; every consumer the plan touches uses the new
  names. No deprecation alias is added — the rename is mechanical and
  the AC2 sweeps would never have admitted a `var(--size-chip)`
  reference once the rule lands.
- Follow-up work item 0091 (typography rem-vs-px stance review) is
  created alongside this plan but not part of this PR.

## References

- Work item: `meta/work/0075-typography-size-scale-consumption.md`
- Follow-up work item: `meta/work/0091-typography-rem-vs-px-stance.md` (created alongside this plan)
- Codebase research: `meta/research/codebase/2026-05-23-0075-typography-size-scale-consumption.md`
- New ADR (created by this PR): `meta/decisions/ADR-0036-typography-font-size-consumption-rule.md`
- Convention with typography clauses superseded by ADR-0036: `meta/decisions/ADR-0026-css-design-token-application-conventions.md`
- Immutability convention applied: `meta/decisions/ADR-0031-skill-level-adr-immutability.md`
- Token source: `skills/visualisation/visualise/frontend/src/styles/global.css:126-142`
- Enforcement harness: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`
- Token registry: `skills/visualisation/visualise/frontend/src/styles/tokens.ts`
- Playwright pattern: `skills/visualisation/visualise/frontend/tests/visual-regression/chip-resolved-colours.spec.ts`
- Related work items: 0033 (token system, `done`), 0076 (code-block
  syntax-highlight, `ready` — rebases onto rationalised `<pre>` CSS),
  0090 (radius tokens consumption, `draft` — reuses this work item's
  pattern verbatim), 0091 (typography rem-vs-px stance review,
  follow-up).
