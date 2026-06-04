---
type: plan
id: "2026-06-02-0090-radius-tokens-consumption"
title: "Radius Tokens Consumption Implementation Plan"
date: "2026-06-02T16:02:57+00:00"
author: "Toby Clemson"
producer: create-plan
status: accepted
work_item_id: "0090"
parent: ""
reviewer: "Toby Clemson"
tags: [design, frontend, tokens, radius]
revision: "947ee021d2441d539e6bbee77227bc4dd21fc3ed"
repository: "visualisation-system"
last_updated: "2026-06-03T00:00:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Radius Tokens Consumption Implementation Plan

## Overview

Adopt consume-tokens-everywhere as the canonical rule for `border-radius` in
current-app component CSS: every radius declaration resolves to a
`var(--radius-*)` token, no literal px/rem/%/`0` values remain. The research
([2026-06-02-0090](../research/codebase/2026-06-02-0090-radius-tokens-consumption.md))
enumerated the full inventory — **25 literal declarations across 12 files**,
spanning **7 distinct values** (`0`, `2px`, `3px`, `6px`, `8px`, `12px`,
`50%`) — materially larger than the work item's two known outliers.

Rather than invent clunky use-case names (`--radius-block`, an unnamed `3px`
token) for the between-step values — the tension the research surfaced and the
work item's AC8 forced — we **rename the whole scale to a self-documenting
px-encoded ladder** (`--radius-0`, `--radius-2`, … `--radius-12`), keeping only
the two shape-intent semantic tokens (`--radius-pill`, `--radius-full`). This
is an authorised deviation from the work item's literal AC1/AC8 naming (see
[Authorised Deviations](#authorised-deviations-from-work-item-acs)); the design
system is ours to name.

Delivery mirrors 0075's **migration-first / enforcement-last** shape: tokens +
ADR + regression spec up front, per-area literal migrations in the middle, the
dedicated literal-ban gate last. Each phase leaves the Vitest harness green and
is independently committable; all phases land together as one atomic PR (the
work item's "no intermediate state on `main`" is about the branch, not the
commits within it).

## Current State Analysis

**The enforcement model is a Vitest harness, not CI grep.** All
literal-consumption rules live in
`skills/visualisation/visualise/frontend/src/styles/migration.test.ts`, run
transitively by CI via `mise run test → test:unit:frontend → vitest run`. There
is no standalone ripgrep step and no lint task (typecheck only). The harness is
a per-occurrence admission ledger with several interlocking mechanisms the plan
must respect (all confirmed by reading the file):

- **`PX_REM_EM_RE`** (`migration.test.ts:31`):
  `/\b(?!0(?:px|rem|em)\b)\d+(?:\.\d+)?(?:px|rem|em)\b/g`. Excludes
  `0px/0rem/0em` and never matches bare `0` (no unit) or `50%`. So the one `0`
  reset and the three `50%` circles are **invisible to the existing gate and
  carry no EXCEPTIONS entry** — but AC3's `[.0-9]` pattern *does* catch them,
  which is precisely why they must be tokenised.
- **EXCEPTIONS ledger** (`:48-264`): every radius literal is budgeted as
  `kind: 'irreducible'` with an exact `(file, literal, count)`. Counts are the
  substring occurrence of the literal across the **whole file**, mixing radius
  and non-radius uses of the same literal.
- **`observed === declared` hygiene gate** (`:448`): forces migration and
  EXCEPTIONS decrement into the **same commit** — migrating a literal without
  decrementing its count makes `declared > observed` and fails.
- **`var(--NAME)` resolution test** (`:361-393`): builds `declared` from
  `RADIUS_TOKENS` keys (`:367`). Any consumer referencing an undeclared token
  name fails — this is the test that proves the rename is complete.
- **`AC5_FLOOR` ratchet** (`const AC5_FLOOR = 427`, last bumped by 0094):
  asserts `AC5_FLOOR <= observed var(--*) count`. Migrating ~25 literals raises
  observed; the floor must be bumped in the same PR per the in-file bump
  protocol.
- **0038 pill allow-list** (`:512-536`): regexes `border-radius:\s*var(--radius-pill)`
  against a 6-file allow-list. Keeping the `--radius-pill` name leaves it
  untouched; routing `50%` to `--radius-full` (not pill) keeps the
  non-allow-listed files (PipelineMini, LibraryTemplatesIndex,
  LifecycleClusterView) clean.

**The scale today** (`global.css:208-212`, `tokens.ts:191-196`):
`--radius-sm: 4px`, `--radius-md: 8px`, `--radius-lg: 12px`,
`--radius-pill: 999px`. Structural gaps by design — nothing below 4px, nothing
between 4 and 8 — which is exactly why 2px/3px/6px were irreducible.

**Already-tokenised consumers:** 43 `var(--radius-sm|md|lg)` sites across 24
files, plus 6 `var(--radius-pill)` sites (verified by ripgrep). Consumption is
already the norm; this work closes the remaining 25-declaration literal gap and
renames the existing 43.

## Desired End State

- Every `border-radius` (shorthand + four longhand corners) in current-app
  `src/` CSS resolves to `var(--radius-*)`. AC3's three ripgrep sweeps return
  **zero matches**.
- The scale is a px-encoded ladder: `--radius-0`, `--radius-2`, `--radius-3`,
  `--radius-4`, `--radius-6`, `--radius-8`, `--radius-12`, plus `--radius-pill`
  (999px) and `--radius-full` (50%). Declared identically in `global.css` and
  `RADIUS_TOKENS`.
- A dedicated `BORDER_RADIUS_LITERAL_RE` Vitest gate fails the build on any
  numeric-led radius literal; a Playwright spec asserts each migrated
  selector's computed radius is unchanged.
- ADR-0039 codifies the rule + naming policy and supersedes ADR-0026 §3's
  "In-between border radii" radius classification via its `supersedes` edge +
  prose. **ADR-0026 is left entirely untouched** (body and frontmatter) — it is
  `accepted`, and ADR-0031 permits no content edits to a non-`proposed` ADR. The
  §3 row physically remains as historical record; it is logically superseded by
  ADR-0039. (This supersedes work-item AC6 — see Authorised Deviations.)
- All radius EXCEPTIONS entries the rule supersedes are deleted or decremented;
  no `kind: 'irreducible'` entry has a `literal`/`reason` tied to a migrated
  `border-radius`.

**Verification:** `mise run test:unit:frontend` green; `mise run
test:e2e:visualiser` green; the three AC3 sweeps return zero; `npm run
typecheck` clean.

### Key Discoveries:

- All 25 literals live in `.module.css`; **zero** bare non-module `.css` or
  longhand-corner literals exist (research §1). Sweep 1 and sweep 2 return the
  same set.
- `8px` (×1, FilterPill `.badge`) already equals `--radius-md`; `12px` (×1,
  EmptyState `.card`) equals `--radius-lg` — pure consume-existing-token
  migrations under px-encoding (`--radius-8`, `--radius-12`).
- **EmptyState `.card` (12px) is not reachable by navigation** — 0074 added
  fixtures for every doc type, so no `/library/<type>` route yields an empty
  listing (research §6). Decision: rely on AC3 as its completeness backstop;
  the Playwright spec omits it.
- `migration.test.ts:42` MarkdownRenderer `border-radius: 0` reset is **not** in
  EXCEPTIONS (`PX_REM_EM_RE` auto-excludes it) — no decrement needed there, but
  AC3's grep still flags it, so it must become `var(--radius-0)`.
- Research §3c flagged `:224` FilterPill `3px` and `:109` Sidebar `4px` as not
  cleanly reconciling; both are now resolved (Phase 5 §2): `:224` is pure-radius
  → **delete**; `:109` is non-radius → **leave at count 4**. All other final
  counts are driven off the hygiene-test failure messages, not hand-computed.

## What We're NOT Doing

- **Not** snapping off-scale values onto existing steps — extend-and-preserve
  means zero intended visual change (2px/3px/6px/50%/0 codified exactly).
- **Not** migrating the prototype or any non-current-app CSS (literal-migration
  scope is current-app `src/`). The token *rename* touches every consumer of
  the old names, but the search confirmed all 43 + the token defs are within
  `frontend/src` — no prototype/`.tsx`/`.json` references exist.
- **Not** adding a real ripgrep CI step — the AC4 gate is a Vitest `describe`
  block following precedent (authorised decision; see Authorised Deviations).
- **Not** adding a fixture/route to mount EmptyState `.card` — AC3 backstops it
  (authorised decision).
- **Not** renaming `--radius-pill`.
- **Not** editing ADR-0026 at all — no row removal, no pointer, no frontmatter
  change. It is `accepted`; ADR-0031 forbids content edits to a non-`proposed`
  ADR. Supersession of its §3 radius classification is recorded solely on the
  new (proposed) ADR-0039 via `supersedes`. *(Out of scope here: ADR-0026's body
  already carries in-place typography supersession pointers from 0075 (`:17`,
  `:112`, `:129`) — that earlier edit appears to have crossed the same
  immutability line. Captured for separate follow-up in
  `meta/notes/2026-06-03-adr-0026-body-edited-in-place-breaks-immutability.md`.)*

## Authorised Deviations from Work Item ACs

These were decided with the author and supersede the literal AC text. The
validation step must treat them as the contract, not as failures:

1. **Naming (supersedes AC1 token names + AC8 scale-vs-use-case policy).** The
   work item names `--radius-xs: 2px` and `--radius-block: 6px` and mandates
   use-case names for between-step values. We instead adopt a canonical
   two-bucket vocabulary: **measurement tokens are px-encoded**
   (`--radius-<px>`, suffix = px value) and **shape-intent tokens are semantic**
   (`--radius-pill`, `--radius-full`). AC8's scale-vs-use-case distinction
   dissolves. ADR-0039 records and *argues* this policy (the use-case-naming
   failures for `3px`/`--radius-block`), states the value-mutation and
   closed-set extension rules, and flags the deliberate divergence from
   `--sp-N` (whose suffix is an ordinal step index, not a px value). The new
   tokens preserve every px value exactly (AC1's substantive guarantee). This
   same vocabulary is used verbatim in the `global.css` comment and the PR
   description.
2. **`50%` → `--radius-full` (new), not `--radius-pill`.** Resolves research
   open question 4. Stores `50%` so computed values are identical.
3. **`0` → `--radius-0` (new).** Resolves research open question 3. Tokenised
   rather than gate-exempted, so AC3's `[.0-9]` sweep can stay strict.
4. **AC4 gate = Vitest `describe` block**, not an executed ripgrep CI step
   (research open question 5; follows the 0075 precedent). The three AC3 sweeps
   are recorded as documentation comments; the in-test regex is authoritative.
   Inserting a literal still fails CI non-zero via `mise run test`.
5. **EmptyState `.card` (12px) discharged by AC3**, not a per-selector AC2
   assertion (research open question 6) — not mountable by navigation.
6. **ADR-0026 is not edited (supersedes AC6's edit/grep requirement).** AC6
   requires ADR-0026 §3 to "no longer carry an active 'In-between border radii'
   row" — deleted or annotated — and verifies it with a grep returning no
   un-struck row. That is a **content edit to an `accepted` ADR, which ADR-0031
   forbids** (only `proposed` ADRs permit content edits; non-`proposed` ADRs
   allow only status transitions + associated metadata). We therefore leave
   ADR-0026 untouched and record the supersession entirely on ADR-0039 (its
   `supersedes: ["adr:ADR-0026"]` edge + Decision prose). The §3 radius row
   stays physically present as historical record; the AC6 grep is **expected to
   still find it** and is not a failure. AC6's substantive intent — the radius
   classification is no longer in force — is satisfied by the ADR-0039
   supersession, not by mutating the older record.

## Implementation Approach

Migration-first / enforcement-last, eight independently-committable phases in
one PR. The always-on harness is the test oracle: the `var(--NAME)` resolution
test drives the rename (Phase 2), and the `observed === declared` hygiene gate
drives every literal migration (Phases 4-6) red→green. The recommended working
procedure for the migration phases (research §3c) is: **migrate the CSS, run
`mise run test:unit:frontend`, read the `observed !== declared` mismatch
report, set EXCEPTIONS counts to the reported observed** — do not hand-compute.

### Token reference maps (used throughout)

**Rename map** (Phase 2, 43 sites): `var(--radius-sm)` → `var(--radius-4)`;
`var(--radius-md)` → `var(--radius-8)`; `var(--radius-lg)` → `var(--radius-12)`;
`var(--radius-pill)` → unchanged.

**Literal-migration map** (Phases 4-6, 25 sites):

| literal | token | sites (file:line) |
|---|---|---|
| `0` | `var(--radius-0)` | MarkdownRenderer:42 |
| `2px` | `var(--radius-2)` | Breadcrumbs:34, RelatedArtifacts:49, FilterPill:189, Sidebar:428, Sidebar:462, Sidebar:472 |
| `3px` | `var(--radius-3)` | FilterPill:84, FilterPill:160, FilterPill:174, Sidebar:107, Sidebar:126, Sidebar:351 |
| `6px` | `var(--radius-6)` | MarkdownRenderer:23, MarkdownRenderer:35, Pipeline:24, Sidebar:300, LifecycleIndex:71, LifecycleClusterView:137, LibraryOverviewHub:42 |
| `8px` | `var(--radius-8)` | FilterPill:41 |
| `12px` | `var(--radius-12)` | EmptyState:14 |
| `50%` | `var(--radius-full)` | PipelineMini:13, LifecycleClusterView:52, LibraryTemplatesIndex:146 |

---

## Phase 1: New ADR-0039 (radius consumption rule)

### Overview

Create the new radius ADR (ID **ADR-0039** — research §5 confirmed the next free
number). It carries the entire supersession of ADR-0026 §3's radius
classification via its `supersedes` edge + prose. **ADR-0026 is not edited** —
it is `accepted`, and ADR-0031 forbids content edits to a non-`proposed` ADR
(see Authorised Deviation 6). Pure documentation; trivially green. Done first so
the `global.css` comment (Phase 2) and the gate comment (Phase 7) have a valid
ADR ID to reference.

### Changes Required:

#### 1. New ADR-0039

**File**: `meta/decisions/ADR-0039-border-radius-consumption-rule.md` (new)
**Changes**: Model on ADR-0036 (the typography consumption-rule ADR — research
§5). Structure: Context / Decision Drivers / Considered Options / Decision /
Consequences / References.

The **Decision Drivers / Considered Options** sections must *argue* the
px-encoded naming choice, not merely state it (it reverses the work item's
written AC1/AC8) — mirroring how ADR-0036 argued numeric names against t-shirt
prefixes. Capture the use-case-naming failure the research surfaced:

- AC8 mandates use-case names for between-step values, but `3px` spans six
  unrelated surfaces (clear-button, scrollbar thumb, facet-option row, three
  kbd chips) with no clean single use-case.
- The work item's own `--radius-block` mis-describes its consumers — only 2 of
  7 `6px` sites are code blocks; the rest are cards, panels, and a tile.
- These tensions are why px-encoding (self-naming by value) was chosen over
  inventing names; record this as the rationale.

The **Decision** section carries bolded-lead clauses:

- **The rule**: *every `border-radius` declaration (shorthand or the four
  longhand corner properties) in current-app CSS under `src/` must resolve to a
  `var(--radius-*)` token reference. No literal px, rem, em, percentage, or
  bare-`0` radius values are permitted.*
- **Scale naming policy**: measurement tokens are **px-encoded**
  (`--radius-<px>`, where the numeric suffix is the literal px value);
  shape-intent tokens are semantic (`--radius-pill` = 999px capsule,
  `--radius-full` = 50% circle). This replaces the t-shirt-style names
  (`--radius-sm/md/lg`) introduced under ADR-0026 and removes the need for
  use-case names. **Note the deliberate divergence from the sibling `--sp-N`
  spacing scale, whose numeric suffix is an ordinal step index (`--sp-1` = 4px),
  not a px value** — radius is px-keyed, spacing is step-keyed; state this
  explicitly so the two numeric-suffix families are not conflated.
- **Scope of supersession**: this is a genuine *replacement* of ADR-0026 §3's
  "In-between border radii" irreducible classification for radius only — hence
  `supersedes` (following ADR-0036's model), not the `relates_to` supplement
  model ADR-0035 used. The 1px/2px border-*width* row is unaffected. State the
  scope at clause level so this prose is the discoverable source of truth, since
  ADR-0026 is left untouched (next clause) and so carries no inline pointer.
  **ADR-0026 is now the target of two `supersedes` edges retiring different
  sections** — ADR-0036 (typography rows) and this ADR (the radius row); name
  that explicitly so a reader traversing the graph can attribute which ADR
  governs which clause. The supersession is recorded **here**: ADR-0026 itself
  is `accepted` and left untouched per ADR-0031; its §3 radius row remains as
  historical record and is governed by this ADR going forward.
- **Scale extension policy**: the ladder enumerates only values actually
  consumed — it is not a complete 1px grid. Off-grid radius values are handled
  by adding a new ladder step (with a recorded rationale and PR/ADR sign-off),
  not by tolerance-band substitution and not by an unreviewed literal.
- **Value-mutation policy**: because a token's name encodes its value, a value
  change is made by adding a new step and re-pointing consumers, **not** by
  mutating an existing token's value in place (which would make the name lie).
- **Escape valve**: none for radius literals — unlike the EXCEPTIONS ledger,
  the dedicated gate admits zero radius literals.

The **Consequences** section should note the accepted tradeoffs: name↔value
coupling (above), the cross-family inconsistency with `--sp-N`, and the
one-time relearning cost of the `sm/md/lg` → px rename.

Frontmatter: `supersedes: ["adr:ADR-0026"]` (the t-shirt naming originated in
ADR-0026, so this edge legitimately covers its replacement; do not cite the
bare work-item number 0033 in the body — References only). References cite
ADR-0030 (the structural template ADR-0036 itself followed), ADR-0031
(immutability/supersession), ADR-0034 (typed linkage), and ADR-0036 (the worked
sibling-rule exemplar whose argumentation shape this ADR mirrors). Keep
work-item IDs out of the body.

#### 2. ADR-0026 — do not edit

**File**: `meta/decisions/ADR-0026-css-design-token-application-conventions.md`
**Changes**: **none.** ADR-0026 is `status: accepted`; ADR-0031 permits content
edits only on `proposed` ADRs, and the sole writes allowed on a non-`proposed`
ADR are status transitions plus their associated metadata. ADR-0026 is only
*partially* superseded (its §2 spacing rule and other sections remain in force),
so it is **not** transitioned to `superseded` either — it stays `accepted`.

Concretely, do not:
- remove or strike the §3 "In-between border radii" row (it stays as historical
  record);
- add an inline/blockquote pointer to ADR-0039;
- touch its frontmatter (its single-ref `superseded_by` stays on ADR-0036; per
  ADR-0034 the ADR-0039 supersession edge is derivable from ADR-0039's
  `supersedes` and needs no reciprocal write).

The supersession is recorded entirely on ADR-0039 (§1). See Authorised
Deviation 6 for why this supersedes work-item AC6.

### Success Criteria:

#### Automated Verification:

- [x] ADR-0039 markdown lints/parses (frontmatter valid): `mise run test` does
      not regress; manual frontmatter check.
- [x] `jj diff --stat` shows ADR-0039 added and **no change to
      `ADR-0026-*.md`** (immutability respected). The
      `rg 'In-between border radii'` row is *expected to remain* in ADR-0026 —
      it is not removed (Authorised Deviation 6).

#### Manual Verification:

- [x] ADR-0039 reads as a tight single-rule ADR consistent with ADR-0036, and
      its Decision Drivers *argue* the px-encoded naming choice.
- [x] ADR-0039 carries `supersedes: ["adr:ADR-0026"]`; ADR-0026 is untouched
      (body and frontmatter — its `superseded_by: "adr:ADR-0036"` unchanged).

---

## Phase 2: Px-encoded scale rename + extension

### Overview

Define the complete final px-encoded ladder in `global.css` and `tokens.ts`,
and rename all 43 existing consumers. New tokens (`--radius-0`, `-2`, `-3`,
`-6`, `-full`) are declared but not yet consumed (harmless). The 25 literals
remain literal — they migrate in Phases 4-6. Suite stays green because the
`var(--NAME)` resolution test validates that every consumer now references a
declared px-encoded name.

### Changes Required:

#### 1. global.css radius block

**File**: `skills/visualisation/visualise/frontend/src/styles/global.css`
**Changes**: replace `:208-212` with the full ladder + AC7 comment.

```css
  /* Radius — consume-tokens-everywhere per ADR-0039: every border-radius in
     current-app CSS must use a var(--radius-*); no literals.
     Naming: measurement tokens are px-encoded (--radius-<px>, suffix = px
     value — NOT an ordinal step like --sp-N); --radius-pill (999px capsule)
     and --radius-full (50% circle) are the two shape-intent semantic tokens.
     New values extend the ladder with sign-off (ADR-0039), never a literal. */
  --radius-0:    0;
  --radius-2:    2px;
  --radius-3:    3px;
  --radius-4:    4px;
  --radius-6:    6px;
  --radius-8:    8px;
  --radius-12:   12px;
  --radius-pill: 999px;
  --radius-full: 50%;
```

#### 2. RADIUS_TOKENS registry + type

**File**: `skills/visualisation/visualise/frontend/src/styles/tokens.ts`
**Changes**: replace `RADIUS_TOKENS` (`:191-196`) in lockstep; update the
`RadiusToken` type (`:351`).

```ts
export const RADIUS_TOKENS = {
  'radius-0':    '0',
  'radius-2':    '2px',
  'radius-3':    '3px',
  'radius-4':    '4px',
  'radius-6':    '6px',
  'radius-8':    '8px',
  'radius-12':   '12px',
  'radius-pill': '999px',
  'radius-full': '50%',
} as const
```

#### 3. Rename 43 consumers

**Files**: 24 `.module.css` files (verified list via `rg -l
'var\(--radius-(sm|md|lg)\)' src`).
**Changes**: apply the rename map. Mechanical find-replace:

```
var(--radius-sm)  → var(--radius-4)
var(--radius-md)  → var(--radius-8)
var(--radius-lg)  → var(--radius-12)
```

Leave `var(--radius-pill)` untouched (6 sites).

#### 4. Update EXCEPTIONS reason prose (if any names old tokens)

**File**: `migration.test.ts` — for any **retained** EXCEPTIONS entry whose
`reason` names `--radius-sm/md/lg` as a comparison floor (e.g. the `1.5px`
ring-width row at `:236`), update the name to its px-encoded equivalent for
accuracy. Do not touch entries being deleted/decremented in later phases.

#### 5. Token-value guard (existing `global.test.ts` parity — confirm, don't duplicate)

**File**: `skills/visualisation/visualise/frontend/src/styles/global.test.ts`.
The value-resolution gap is **already closed by an existing test**: the
`describe.each([… ['radius', RADIUS_TOKENS] …])('tokens.ts ↔ global.css :root
parity')` suite reads each token's value out of `global.css` (`readCssVar`) and
asserts it equals the `RADIUS_TOKENS` value. So a `global.css` typo like
`--radius-12: 1px` (with `tokens.ts` still `'12px'`) fails this parity test —
which is the real guard for **EmptyState `.card` (12px)** and the `--radius-full`
value, neither of which has a Playwright assertion. (The `var(--NAME)`
resolution test in `migration.test.ts` only proves a referenced name *exists*;
it does not prove its value — the parity test does.)

No new assertion is needed; do not add a `migration.test.ts` `it` that asserts
`RADIUS_TOKENS` against a hardcoded table, as that would compare `tokens.ts` to
itself and cannot catch a `global.css` drift. **Action**: confirm the existing
parity suite still iterates `RADIUS_TOKENS` after the Phase 2 §1/§2 lockstep
edits (it picks up the new tokens automatically via `Object.entries`), so all
nine token values are covered.

### Success Criteria:

#### Automated Verification:

- [x] No old token names remain in CSS: rename complete (var-resolution test
      green). The 6 remaining `radius-(sm|md|lg)` mentions are EXCEPTIONS
      reason-prose in `migration.test.ts` for still-present literals, cleared in
      Phases 4-6.
- [x] Unit harness green (var-resolution proves rename completeness):
      `mise run test:unit:frontend` — 2126 passed.
- [x] Types clean: `npm run typecheck` (in `frontend/`).

#### Manual Verification:

- [ ] App renders with no radius regressions (deferred to Phase 8 spot-check;
      computed values guarded by `global.test.ts` parity suite).

---

## Phase 3: Playwright regression spec (baseline guard, AC2)

### Overview

Add `radius-resolved-radii.spec.ts` asserting the computed `border-radius` of
each migrated selector equals its recorded pre-migration px value. Written
**before** the literal migration (TDD baseline-capture): the spec passes
pre-migration (computed = literal) and must still pass post-migration (computed
= `var(--radius-*)` resolved to the same value) — proving zero visual drift.
Excludes EmptyState `.card` (AC3 backstop).

### Changes Required:

#### 1. The spec

**File**:
`skills/visualisation/visualise/frontend/tests/visual-regression/radius-resolved-radii.spec.ts`
(new)
**Changes**: model on `typography-resolved-sizes.spec.ts` (research §6). A
data-driven `CASES` array of `{ route, selector, expected, name, setup? }`; one
`test()` loops it. Read a **corner longhand** (Chromium returns the
`borderRadius` shorthand empty):

```ts
const value = await page.locator(selector).first()
  .evaluate((el) => getComputedStyle(el).borderTopLeftRadius)
expect(value).toBe(expected)
```

Pin the viewport: `test.use({ viewport: { width: 1280, height: 720 } })`. Route
→ component mounting per research §6 table (Breadcrumbs/Sidebar on `/library`
with a search `setup`; MarkdownRenderer on a doc-detail route, including
`.codeblock pre` — the `0` reset, expected `'0px'`; RelatedArtifacts on
`/library/work-items/0099-ac2-coverage`; FilterPill on `/library/plans` with
a trigger-click `setup`; Pipeline/lifecycle on `/lifecycle[/<slug>]`;
PipelineMini on `/kanban`; LibraryOverviewHub on `/library`;
LibraryTemplatesIndex on `/library/templates`). Include the `0` reset as a
case so the spec and Phase 3's "every non-EmptyState selector has one
assertion" checkbox agree.

**Mounting must be verified, not assumed.** For every interaction-gated case
(FilterPill options/checkbox/scrollbar after the trigger click; Sidebar search
UI after typing), the `setup` step ends with an explicit
`await page.locator(selector).waitFor()` on the target (mirroring the 0075 spec
`getByRole(...).first().waitFor()`), so a selector that never mounts fails
loudly rather than silently asserting on the wrong element. **Confirm
RelatedArtifacts `.badge` actually renders at `/library/work-items/0099-ac2-coverage`
during the baseline run** — the existing aside-row spec reaches that fixture's
work-items only as a *cluster sibling* via `/library/decisions/ADR-0099-ac2-coverage`,
so the direct `.badge` mount must be checked before pinning its expected value.

Expected values: px literals assert their exact px string (`'2px'`, `'3px'`,
`'6px'`, `'8px'`, `'0px'`). **For the three `50%` selectors** the computed value
is the box-dependent used px (PipelineMini `.dot` ~`4px`, LifecycleClusterView
`.stage::before` ~`5px`, LibraryTemplatesIndex `.tierPillBullet` ~`2.5px`).
A pinned used-px alone cannot distinguish `--radius-full` (50%) from a
coincidentally-equal px token, so for these three **also assert the element's
box dimensions** (`offsetWidth`/`offsetHeight`) are unchanged — this guards
against a box-size regression, but is not by itself proof of 50%-derivation. The
value-level guarantee that `--radius-full` holds `50%` comes from the existing
`global.test.ts` radius parity suite (Phase 2 §5), which reads the value from
`global.css`.
Capture each by running the spec once against the live server and pin the
returned string — identical before and after migration because the token stores
`50%`.

### Success Criteria:

#### Automated Verification:

- [x] Spec passes (pre-migration baseline): `mise run test:e2e:visualiser` —
      413 passed, incl. 12 new radius cases.
- [x] Spec is collected by the `visual-regression` project (ran under
      `[visual-regression]`).

#### Manual Verification:

- [x] The `50%` cases were measured against the running server. **Finding:**
      Chromium's `getComputedStyle().borderTopLeftRadius` returns the literal
      `"50%"` (not a used px), which directly proves percentage-derivation;
      box dims (8×8, 5×5) are also asserted. Only **two** `50%` selectors
      remain (PipelineMini, LibraryTemplatesIndex) — the plan's third,
      LifecycleClusterView `.stage::before`, was removed by the clustering
      rewrite.
- [~] **Deviation (inventory drift):** the spec uses representative coverage —
      at least one mountable selector per distinct migrated value (0, 2px, 3px,
      6px, 50%), mirroring the typography spec's documented posture — rather
      than one assertion per inventory selector. `8px` (FilterPill badge,
      selection-gated), `1px` (spine pseudo-element), and `12px` (unmountable
      EmptyState) are backstopped by the value-parity suite + the Phase 7
      categorical gate, as documented in the spec header.

---

## Phase 4: Migrate code & pipeline surfaces

### Overview

Migrate MarkdownRenderer (6px ×2 + the `0` reset), Pipeline `.tile` (6px), and
PipelineMini `.dot` (50%). Delete the two pure-radius EXCEPTIONS rows for these
files. Green via the hygiene gate.

### Changes Required:

#### 1. CSS migrations

- `MarkdownRenderer.module.css:23,35` → `var(--radius-6)`; `:42` `0` →
  `var(--radius-0)`.
- `Pipeline.module.css:24` → `var(--radius-6)`.
- `PipelineMini.module.css:13` `50%` → `var(--radius-full)`.

#### 2. EXCEPTIONS cleanup

**File**: `migration.test.ts`
- Delete `:69` MarkdownRenderer `'6px' x2` (pure radius).
- Delete `:78` Pipeline `'6px' x1` (pure radius).
- (PipelineMini `50%` and the `0` reset are not in EXCEPTIONS — no change.)

### Success Criteria:

#### Automated Verification:

- [x] `mise run test:unit:frontend` green (hygiene `observed === declared`
      holds for these files) — 2126 passed.
- [x] `mise run test:e2e:visualiser` green (radius + inline-code specs pass for
      Markdown/Pipeline/PipelineMini selectors).

#### Manual Verification:

- [x] Code-block `<pre>`, pipeline tiles, and mini dots render unchanged
      (computed-radius spec confirms). **Drift:** MarkdownRenderer also had a
      4th literal `border-radius: 3px` (inline-code pill, added by 0094 after
      the research) → migrated to `var(--radius-3)`, its pure-radius EXCEPTIONS
      row deleted.

---

## Phase 5: Migrate chrome components

### Overview

The bulk of the decrement work: Sidebar, FilterPill, Breadcrumbs,
RelatedArtifacts (the `2px`/`3px`/`8px` family across mixed EXCEPTIONS rows).
**Drive final counts off the hygiene-test failures** (research §3c) — migrate
the CSS first, then set each `count` to the reported observed.

### Changes Required:

#### 1. CSS migrations

- `Breadcrumbs.module.css:34` → `var(--radius-2)`.
- `RelatedArtifacts.module.css:49` → `var(--radius-2)`.
- `FilterPill.module.css`: `:189` → `var(--radius-2)`; `:84,:160,:174` →
  `var(--radius-3)`; `:41` → `var(--radius-8)`.
- `Sidebar.module.css`: `:428,:462,:472` → `var(--radius-2)`;
  `:107,:126,:351` → `var(--radius-3)`; `:300` → `var(--radius-6)`.

#### 2. EXCEPTIONS cleanup (decrement + rewrite reason; final counts from test)

Per research §3b: decrement `:90` (RelatedArtifacts 2px), `:107` (Sidebar 2px),
`:108` (Sidebar 3px), `:111` (Sidebar 6px), `:129` (Breadcrumbs 2px), `:222`
(FilterPill 2px), `:231` (FilterPill 8px), removing the radius mention from each
`reason`.

- **`:224` FilterPill `3px` → delete the row (pure radius).** All three `3px`
  occurrences in FilterPill (`:84`, `:160`, `:174`) are `border-radius`; after
  migration the observed `3px` count is 0, so the row is pure-radius like
  `:69`/`:78` and must be **deleted**, not decremented (decrementing to a
  nonzero count leaves `declared > observed` and fails the hygiene gate). The
  `:224` reason's "checkmark height" clause is inaccurate — there is no `3px`
  checkmark height.
- **`:109` Sidebar `4px` → non-radius, leave the row at count 4.** The reason's
  "scrollbar thumb radius" clause is stale: Sidebar's scrollbar-thumb radii are
  already `var(--radius-sm)` (no `border-radius: 4px` exists in the file), and
  the four `4px` occurrences are non-radius (translate offsets + paddings).
  Leave the count unchanged; optionally drop the stale radius clause from the
  reason for accuracy.

Decrement counts and the final `:224` deletion are confirmed by the
hygiene-test failure messages (research §3c) — migrate the CSS first, then set
each surviving `count` to the reported observed.

**Reason rewrite — worked example.** For each decremented row, strip the
migrated-radius clause and keep only the surviving non-radius uses. E.g.
Sidebar `:107` `'2px'` reason "mark border-radius, loadbar height + radius,
list gaps" → after migrating the mark + loadbar radii: "loadbar height, list
gaps" (count 6→4). The Phase 7 §4 assertion enforces that no surviving reason
still mentions `border-radius`.

### Success Criteria:

#### Automated Verification:

- [x] `mise run test:unit:frontend` green; no `observed !== declared`
      mismatches (counts driven off the hygiene report).
- [x] No remaining `kind: 'irreducible'` entry has a `reason` referencing
      `border-radius` for these files (also enforced by Phase 7 §4).
- [x] `mise run test:e2e:visualiser` green (radius + aside-row specs;
      Breadcrumbs/FilterPill assertions incl. trigger-click `setup`).

#### Manual Verification:

- [x] Search highlight/loadbar, kbd chips, facet options, checkbox, focus
      ring, and provenance badge render unchanged (computed-radius spec +
      aside-row spec). **Gate correction:** Sidebar `2px` decremented 6→3 (the
      gate reported 3 migrated radii — searchMark, loadbar track, loadbar fill
      — not the plan's predicted 2); FilterPill `3px` deleted; the stale
      "scrollbar thumb radius" clause on the retained Sidebar `4px` row was
      removed (no `4px` border-radius exists; the four `4px` are non-radius).

---

## Phase 6: Migrate route surfaces

### Overview

Migrate the lifecycle and library route cards/panels/dots: the remaining `6px`
cards/panels, the `12px` EmptyState card, and the two route-level `50%` dots.

### Changes Required:

#### 1. CSS migrations

- `LifecycleIndex.module.css:71` → `var(--radius-6)`.
- `LifecycleClusterView.module.css:137` → `var(--radius-6)`; `:52` `50%` →
  `var(--radius-full)`.
- `LibraryOverviewHub.module.css:42` → `var(--radius-6)`.
- `LibraryTemplatesIndex.module.css:146` `50%` → `var(--radius-full)`.
- `EmptyState.module.css:14` `12px` → `var(--radius-12)`.

#### 2. EXCEPTIONS cleanup

- Decrement `:71` LifecycleClusterView `'6px'` (keep the `:244` spine x-coord
  6px — non-radius); rewrite the reason to drop the panel-radius clause.
- Decrement `:187` LibraryOverviewHub `'6px'`; rewrite the reason.
- Decrement `:251` LifecycleIndex `'6px'` (keep toolbar-gap 6px); rewrite the
  reason to drop the card-radius clause.
- Delete `:198` EmptyState `'12px' x1` (pure radius).
- (The two route `50%` dots carry no EXCEPTIONS entry — no change.)

Final counts from the hygiene-test failure messages; reason rewrites follow the
Phase 5 §2 worked example and are enforced by the Phase 7 §4 assertion.

### Success Criteria:

#### Automated Verification:

- [x] `mise run test:unit:frontend` green — 2126 passed.
- [x] `mise run test:e2e:visualiser` green (radius spec: lifecycle index card,
      cluster panel + timeline tile, overview-hub card, templates tier-bullet;
      EmptyState omitted by design).

#### Manual Verification:

- [x] Lifecycle index/cluster cards & panels, overview-hub cards, template
      tier-bullets render unchanged (computed-radius spec). **Gate corrections
      (clustering-rewrite drift):** LifecycleClusterView migrated two `6px`
      (panel + timeline tile) + one `1px` (spine → `--radius-1`), no `50%`;
      its `1px` row decremented 5→4 and `6px` row 3→1. LifecycleIndex `6px`
      and EmptyState `12px` rows **deleted** (gate showed each was pure-radius;
      the plan's "keep toolbar-gap 6px" was stale — no such literal exists).

---

## Phase 7: Enforcement gate (AC4) + AC5_FLOOR ratchet

### Overview

Add the dedicated radius literal-ban gate as a new `describe` block, with regex
fixtures (TDD: write fixtures, then satisfy the regex), and bump `AC5_FLOOR`.
By now all 25 literals are migrated, so the file-sweep half of the gate is
green on introduction (enforcement-last).

### Changes Required:

#### 1. Gate describe block

**File**: `migration.test.ts` (mirror the 0075 AC2 font-size block at
`:582-629`)
**Changes**:

```ts
// AUTHORITATIVE IMPLEMENTATION of ADR-0039's border-radius consumption
// rule. BORDER_RADIUS_LITERAL_RE is the load-bearing CI guard; the three
// AC3 ripgrep sweeps below are coarser review-time approximations.
//
// AC3 sweeps (review-time grep gate, run from frontend/src):
//   rg --glob '**/*.module.css' 'border-radius:\s*[.0-9]' src
//   rg --glob '**/*.css' --glob '!**/global.css' 'border-radius:\s*[.0-9]' src
//   rg --glob '**/*.css' --glob '!**/global.css' 'border-(top|bottom)-(left|right)-radius:\s*[.0-9]' src

describe('AC4 / 0090: no border-radius literals in module or global CSS', () => {
  // The (?<![\w-]) lookbehind exists to avoid matching custom-property
  // *names* like `--my-radius:` (the `-` before `radius` would otherwise let
  // the property-name fragment match). A side effect is that vendor-prefixed
  // forms (`-webkit-border-radius: 6px`) are NOT matched — see the negative
  // fixture below. Current-app CSS uses no vendor-prefixed radius, so this is
  // an accepted, recorded limitation; the AC3 sweeps share it in practice.
  const BORDER_RADIUS_LITERAL_RE =
    /(?<![\w-])border-(?:(?:top|bottom)-(?:left|right)-)?radius:\s*[.0-9]/g
  // allCss = component module CSS + any *.global.css files. The root
  // styles/global.css token file is NOT in allCss (it is not named
  // *.global.css) and is intentionally out of scope — matching the AC3
  // `!**/global.css` exclusion; its `--radius-*:` defs would not match this
  // border-radius property regex anyway.
  for (const [path, css] of Object.entries(allCss)) {
    it(`${path}: no literal border-radius value`, () => {
      const stripped = stripComments(css)  // reuse the existing helper (Phase 7 §1a)
      const hits = [...stripped.matchAll(BORDER_RADIUS_LITERAL_RE)].map((m) => m[0])
      expect(
        hits,
        `${path}: border-radius must use a var(--radius-*) token per ADR-0039; ` +
          `off-scale values need a new ladder step (with sign-off), not a literal`,
      ).toEqual([])
    })
  }
})
```

##### 1a. Reuse `stripComments`

The existing font-size gate defines a `stripComments` helper inline; the new
block reuses it rather than re-inlining the `css.replace(/\/\*…\*\//g, '')`
regex. If it is not already module-scoped, hoist it to module scope so both
literal-ban gates share one definition.

#### 2. Regex fixtures (TDD)

Add a sibling `describe` (mirror `:631`) with positives — `border-radius: 6px;`,
`border-top-left-radius: 2px;`, `border-radius: 0;`, `border-radius: 50%;`,
`border-bottom-right-radius:.5rem;` — and negatives —
`border-radius: var(--radius-6);`, `--radius-6: 6px;`, `--radius-0: 0;`
(the bare-`0` token definition must not trip the gate via the lookbehind),
`--my-radius: 0;`, and — to actually exercise the documented limitation — a
vendor-prefixed **literal** `-webkit-border-radius: 6px;` asserted as **not
matched** (this is what proves the lookbehind admits prefixed literals; a
`-webkit-border-radius: var(--radius-2);` fixture would be skipped by the
`[.0-9]` class regardless of the lookbehind and so proves nothing).

CSS logical-property longhands (`border-start-start-radius`, etc.) are **out of
scope** — current-app CSS uses none, and the regex intentionally matches only
the physical shorthand + four physical corners. Note this explicitly so the
boundary is a recorded decision; if a logical form is ever introduced it must
be added to both the regex and a positive fixture.

#### 3. Bump AC5_FLOOR

**File**: `migration.test.ts` (the `const AC5_FLOOR` line — currently `427`,
last bumped by 0094). This is a **discipline step, not a red→green fix**: a
stale-low floor does *not* fail (both assertions are `observed >= AC5_FLOOR` /
`AC5_FLOOR <= observed`, which hold whenever the floor is at or below observed).
So there is no failure to read — instead, set `AC5_FLOOR` to the observed
`var(--*)` count the ratchet test prints in its passing name (migrating ~25
literals raises it from 427 to ~452; do not hand-compute — read the printed
observed). Append a bump-protocol comment:
`// 0090: 25 radius literals → var(--radius-*) (+N)`.

#### 4. Mechanical AC5 reason-hygiene assertion

**File**: `migration.test.ts` — the EXCEPTIONS `reason` strings are
human-maintained prose the hygiene gate never validates, so a decrement that
leaves a stale radius mention would pass silently. Add an `it` asserting no
surviving `kind: 'irreducible'` entry's `reason` matches `/border-radius|\bradius\b/i`
— **match the bare word `radius`, not just `border-radius`**, because the
existing reasons phrase radius uses both ways (e.g. "scrollbar thumb radius",
"mark border-radius, loadbar … radius"); a `border-radius`-only check would miss
the bare-`radius` phrasing. This makes AC5's "no reason references a migrated
radius" backstop a test rather than a manual grep, catching reason/count drift
across Phases 4-6 in one place. (If any retained reason legitimately keeps a
non-migrated `radius` word, whitelist it explicitly.)

### Success Criteria:

#### Automated Verification:

- [ ] `mise run test:unit:frontend` green (gate passes — zero literals;
      fixtures pass; ratchet satisfied).
- [ ] Inserting `border-radius: 7px` into any `.module.css` makes the gate exit
      non-zero (verify once, then revert).
- [ ] Three AC3 sweeps return zero matches from `frontend/src`.

#### Manual Verification:

- [ ] Gate regex fixtures cover shorthand, all four corners, `0`, and `%`.

---

## Phase 8: PR description + final verification

### Overview

Author the PR description (AC7) and run the full verification sweep. No code
beyond the description (the `global.css` comment landed in Phase 2).

### Changes Required:

#### 1. PR description

State the **verbatim rule** and a one-paragraph rationale referencing ADR-0039
by ID. The rationale must make the naming model self-explaining from the PR
alone: (a) the px-encoded ladder enumerates only values actually consumed (not
a complete 1px grid), and the numeric suffix is the px value — distinct from
`--sp-N`'s ordinal index; (b) `0` is tokenised as `--radius-0` rather than
gate-exempted, keeping AC3's `[.0-9]` sweep strict; (c) `50%` maps to a new
`--radius-full` (a circle intent), **not** `--radius-pill` (a 999px capsule),
preserving the 0038 pill allow-list; (d) 2px/3px/6px/50%/0 are codified exactly
(zero visual change).

### Success Criteria:

#### Automated Verification:

- [ ] Full suite green: `mise run test`.
- [ ] `mise run test:e2e:visualiser` green.
- [ ] `npm run typecheck` clean.
- [ ] All three AC3 sweeps return zero matches.

#### Manual Verification:

- [ ] PR description states the verbatim rule + rationale + ADR-0039 ID.
- [ ] Visual spot-check across kanban, lifecycle, library, doc-detail routes.

---

## Testing Strategy

### Unit Tests (`migration.test.ts`, via `mise run test:unit:frontend`):

- The always-on oracle: `var(--NAME)` resolution (rename completeness),
  `observed === declared` hygiene (decrement correctness), `AC5_FLOOR` ratchet,
  0038 pill allow-list, and the new `BORDER_RADIUS_LITERAL_RE` gate + fixtures.
- Existing guard reused (Phase 2 §5): the `global.test.ts` `tokens.ts ↔
  global.css :root parity` suite already proves each `RADIUS_TOKENS` value
  matches `global.css` — covering EmptyState's `--radius-12` and the
  `--radius-full` value, which have no Playwright cover. New for 0090: a
  reason-hygiene assertion (Phase 7 §4) proving no surviving irreducible reason
  mentions a radius literal.
- Coverage gap to note: the three `50%` and the `0` migrations carry no
  EXCEPTIONS entry, so they have no per-phase `observed === declared` driver —
  they are guarded by the existing parity suite (value), the Phase 3
  box-dimension check (box-size), and the enforcement-last Phase 7 gate
  (literal-ban), not by the migration-phase hygiene oracle.

### Integration / E2E (Playwright, via `mise run test:e2e:visualiser`):

- `radius-resolved-radii.spec.ts` — per-selector computed-radius assertions
  against the real Rust visualiser binary serving committed fixtures.

### Manual Testing Steps:

1. Run the visualiser; verify chips/cards/pills/code-blocks/search-UI render
   unchanged at the default viewport.
2. Capture the three `50%` computed px values against the live server (Phase 3).
3. Insert a stray `border-radius: 7px` and confirm the gate fails (Phase 7).

## Performance Considerations

None — token indirection has no runtime cost; computed values are identical.

## Migration Notes

- Single atomic PR; eight green commits. The token *rename* (Phase 2) is the
  largest mechanical diff (43 sites) but self-contained and validated by the
  resolution test.
- If the cumulative diff proves unreviewable (0075's documented contingency,
  >~1500 non-test lines), re-scope to an epic with per-area children — a
  fallback, not the plan of record. The 25-declaration / 12-file inventory
  stays comfortably within single-PR size (0075 shipped 35 across 9).

## References

- Work item: `meta/work/0090-radius-tokens-consumption.md`
- Research: `meta/research/codebase/2026-06-02-0090-radius-tokens-consumption.md`
- Pattern precedent: `meta/plans/2026-05-23-0075-typography-size-scale-consumption.md`
- Scale origin: `meta/work/0033-design-token-system.md`
- ADR template: `meta/decisions/ADR-0036-typography-font-size-consumption-rule.md`
- ADR superseded (not edited): `meta/decisions/ADR-0026-css-design-token-application-conventions.md` (§3, `:127` — the radius row, left in place per ADR-0031)
- Immutability rule: `meta/decisions/ADR-0031-skill-level-adr-immutability.md`
- Harness: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`
- Edit points: `…/src/styles/global.css:208-212`, `…/src/styles/tokens.ts:191-196,351`
- Spec template: `…/frontend/tests/visual-regression/typography-resolved-sizes.spec.ts`
