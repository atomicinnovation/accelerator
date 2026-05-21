---
date: "2026-05-22T22:55:08+01:00"
researcher: Toby Clemson
git_commit: e8ed1ce002796e05ce8d787294858ba042020195
branch: HEAD
repository: accelerator
topic: "Decomposing FrontmatterChips into chip-list renderer + FrontmatterChip + StatusBadge (work item 0081)"
tags: [research, codebase, frontmatter-chips, status-badge, chip-primitive, verdict, frontend]
status: complete
last_updated: 2026-05-22
last_updated_by: Toby Clemson
---

# Research: Decomposing `FrontmatterChips` into chip-list renderer + `FrontmatterChip` + `StatusBadge`

**Date**: 2026-05-22T22:55:08+01:00
**Researcher**: Toby Clemson
**Git Commit**: e8ed1ce002796e05ce8d787294858ba042020195
**Branch**: HEAD (build-system jj workspace)
**Repository**: accelerator

## Research Question

For work item `meta/work/0081-status-badge-component.md` (StatusBadge —
Decompose `FrontmatterChips` and Map Status + Verdict to Tone): what is
the current shape of `FrontmatterChips`, its dependencies (`Chip` primitive,
`statusToChipVariant`), every consumer/surface, and the upstream verdict
emitters? Where are the gaps between the AC table and the live code that
implementation will need to resolve?

## Summary

The decomposition is **surgically narrow**: a single React file
(`FrontmatterChips.tsx`, 48 lines), one call site (`LibraryDocView.tsx`),
and one tone helper (`status-variant.ts`). The work item's "three surfaces"
(validation, plan-review, work-item-review) all share a single detail-page
component (`LibraryDocView`) routed by `type`, so verdict tone applies in
one place. The `Chip` primitive (0038) ships exactly the six variants
0081 references and is type-safe but **does not pass through arbitrary
`data-*` attributes** — only `aria-label` is plucked from `...rest`, which
will require either a Chip extension or a wrapping span to satisfy the
`data-component` AC.

Three implementation hazards surfaced:

1. **The AC's `status` tone table is a strict subset of the live
   mapping.** The work item enumerates ~9 status values; `statusToChipVariant`
   actually covers ~22 values across GREEN/INDIGO/AMBER/**RED** sets plus a
   `normalise()` that strips whitespace, underscores, hyphens, and slashes.
   Implementations that follow the AC table literally — rather than
   "preserve current mapping exactly" — will regress the RED set
   (`blocked`/`rejected`/`deprecated`/`superseded`/`abandoned`) and several
   amber/green values. The Technical Notes' "absorbed into `StatusBadge`
   or retained as a helper" is materially safer in the retain-as-helper
   form because three *other* consumers also import `statusToChipVariant`.
2. **Validation pages do not emit `verdict:` today.** They emit
   `result: {pass | partial | fail}` plus `status: complete`. The work
   item's Assumption that "Validation pages already surface `verdict` as
   a top-level frontmatter key" is **invalid** against the current
   `validate-plan` skill template. The validation-vocabulary ACs
   (`verdict: pass` → green, `verdict: fail` → red) cannot pass without
   an upstream change to `skills/planning/validate-plan/SKILL.md` to
   either rename `result` → `verdict` or emit both.
3. **Plan-review and work-item-review skills emit `APPROVE | REVISE |
   COMMENT` — never `REQUEST_CHANGES`.** `REQUEST_CHANGES` is a PR-review
   vocabulary item (per ADR-0007). The plan-review AC for `verdict:
   REQUEST_CHANGES` → red is verifiable as a unit-test against a synthetic
   fixture, but no live document in the corpus will exercise it.

## Detailed Findings

### Current `FrontmatterChips` component (the thing being decomposed)

- File: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx` (48 lines).
- Shape — discriminated union on `state`:
  - `{ state: 'absent' }` → renders `null`
  - `{ state: 'malformed' }` → renders an `role="alert"` banner
  - `{ state: 'parsed'; frontmatter: Record<string, unknown> }` → renders chips
- Render loop (`FrontmatterChips.tsx:26-46`):
  ```tsx
  const entries = Object.entries(props.frontmatter).filter(([, v]) => {
    if (v === null || v === undefined) return false
    if (typeof v === 'string' && v === '') return false
    return true
  })
  // ...
  {entries.map(([key, value]) => {
    const text = formatChipValue(value)
    const variant = isStatusKey(key) ? statusToChipVariant(value) : 'neutral'
    return (
      <Chip key={key} variant={variant} aria-label={`${key}: ${text}`}>
        {text}
      </Chip>
    )
  })}
  ```
- **Iteration order is source order** via `Object.entries(...)` — no
  sorting. Satisfies the work item's chip-list-renderer source-order AC
  without any change.
- **Filter rules** (kept by the decomposition):
  - Drop `null` and `undefined`.
  - Drop empty-string values (but keep `false`, `0`, and other falsy
    non-strings — confirmed by tests at `FrontmatterChips.test.tsx:83-89`).
- **Value formatting** (`formatChipValue`, lines 10-14):
  - `Array.isArray(v)` → `v.join(', ')` (tags rendered as one
    comma-joined chip, not per-tag chips).
  - `typeof v === 'object' && v !== null` → `JSON.stringify(v)`.
  - Anything else → `String(v)`.
- **`status` tone branch** (line 38): `isStatusKey(key) ? statusToChipVariant(value) : 'neutral'`.
  No other key gets a non-neutral tone today. `verdict` therefore
  renders as a `neutral` `Chip` with the literal value (`pass`, `APPROVE`, etc.).

### Test file — what will need reshuffling

- File: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.test.tsx` (130 lines, 13 cases).
- Cases that stay with the chip-list renderer (i.e. `FrontmatterChips` post-decomposition):
  - "skips null and undefined values" (`:64-73`)
  - "skips empty-string values" (`:75-81`)
  - "renders a Chip for each non-null frontmatter value" (`:8-17`)
  - `absent` state (`:103-108`), `malformed` state (`:110-120`), and CSS-source assertions (`:122-129`).
- Cases that move to `StatusBadge` tests:
  - "renders the status field with the colour-coded variant" (`:19-24`)
  - "colour-codes status case-insensitively (Status, STATUS)" (`:26-31`)
- Cases that move to `FrontmatterChip` tests:
  - "renders non-status fields with variant='neutral'" (`:33-38`)
  - "renders the value text" (`:59-62`)
  - "renders boolean and numeric values as strings" (`:83-89`)
  - "joins array values with ', '" (`:91-100`)
  - "attaches an aria-label of '${key}: ${value}'" (`:48-57`)
  - "does not render keys (e.g. the literal text 'status:')" (`:40-46`)
- New cases needed (per AC):
  - `StatusBadge` verdict-tone matrix (validation + plan-review vocabularies, both case variants).
  - `StatusBadge` neutral fallback for unmapped values on both keys.
  - `StatusBadge` `data-component` observable hook.
  - `FrontmatterChip` `data-component` observable hook.
  - `FrontmatterChip` `frontmatterKey="status" value="Accepted"` rendering as `neutral` (proves no domain-specific branching).
  - Source-order multi-key fixture (verdict-first ordering).

### `statusToChipVariant` — the canonical tone helper (THE MAIN HAZARD)

- File: `skills/visualisation/visualise/frontend/src/api/status-variant.ts` (27 lines).
- The live mapping is substantially broader than the AC table in 0081:

  | Set | Normalised values | Variant |
  |-----|------------------|---------|
  | GREEN  | `done`, `complete`, `accepted`, `approved`, `implemented`, `final`, `shipped` | `green` |
  | INDIGO | `inprogress`, `reviewed`, `ready`, `active`, `proposed`, `live` | `indigo` |
  | AMBER  | `approvewithchanges`, `approvewchanges`, `review`, `revised` | `amber` |
  | RED    | `blocked`, `rejected`, `deprecated`, `superseded`, `abandoned` | `red` |
  | (fallback) | anything else | `neutral` |

- **Normalisation** (`status-variant.ts:10-13`):
  ```ts
  function normalise(value: unknown): string {
    if (typeof value !== 'string') return ''
    return value.trim().toLowerCase().replace(/[\s_\-/]+/g, '')
  }
  ```
  So `Approve w/ changes` → `approvewchanges` → amber. The AMBER set
  contains *both* `approvewithchanges` (no "/") and `approvewchanges`
  (with "/" originally) to cover both spellings.
- **Key matching** (`status-variant.ts:24-26`): `isStatusKey` lowercases
  and trims; only the literal `status` key matches.
- **AC vs live mismatch (must resolve in planning)** — the work item's
  AC table:
  - omits the **RED** set entirely (`blocked`, `rejected`, `deprecated`,
    `superseded`, `abandoned`).
  - omits `complete`, `approved`, `implemented`, `final`, `shipped` from
    GREEN.
  - omits `reviewed`, `ready` from INDIGO.
  - omits `review`, `revised` from AMBER.
  - lists `Todo` and `absent` as neutral. `Todo` is correctly neutral
    (not in any set). `absent` is also neutral, but only by virtue of
    falling through — and review pass 3 already flagged that
    `absent` reads ambiguously (literal string vs missing-key sentinel).
  - The Requirements line "All values below are literal string matches
    against the `status` frontmatter value (case-insensitive, per the
    lookup rule)" is *almost* right but understates the normalisation:
    it strips whitespace/underscores/hyphens/slashes too, so
    `IN PROGRESS`, `in_progress`, and `in-progress` all match `inprogress`.

### The `Chip` primitive (0038) — `data-component` pass-through is the API gap

- File: `skills/visualisation/visualise/frontend/src/components/Chip/Chip.tsx`.
- Types (`Chip.tsx:4-6`):
  ```ts
  export type ChipVariant = 'neutral' | 'indigo' | 'green' | 'amber' | 'red' | 'violet'
  export type ChipSize = 'sm' | 'md'
  ```
  → Six variants ship. The work item's "no extension to 0038 required"
  claim is correct (the earlier review pass-2 worry about a missing
  `red` variant was a misread).
- Props (`Chip.tsx:8-14`):
  ```ts
  export interface ChipProps {
    variant: ChipVariant         // required, no default
    size?: ChipSize              // defaults to 'sm'
    leading?: ReactNode
    'aria-label'?: string
    children: ReactNode          // required
  }
  ```
- **API gap**: the implementation spreads `...rest` on the destructure
  but only reads `aria-label` from it (`Chip.tsx:16,23`); other props
  including `data-*` attributes are **silently dropped**. The 0081 AC
  requires "an observable hook (e.g. a `data-component` attribute)
  identifying which component produced it". Options:
  1. Extend `Chip` to pass through `data-*` attributes (a 1-line change
     to the spread, but a 0038 API contract change).
  2. Have `StatusBadge` / `FrontmatterChip` wrap `Chip` in an outer
     `<span data-component="StatusBadge">` carrying the attribute.
  3. Render the `data-component` attribute inside `Chip` itself via a
     new prop and let composers pass it through.
- **No default variant**: callers must specify one. `FrontmatterChip`
  will need to explicitly pass `variant="neutral"` when key isn't
  status/verdict.
- Rendering: CSS-modules with `data-variant`/`data-size` attribute
  selectors against design tokens (`Chip.module.css:33-52`); no
  Tailwind or inline styles.

### All consumers and "surfaces"

- **The decomposition only affects one call site**:
  `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:10` (import) and `:95-100` (usage):
  ```tsx
  subtitle = (
    <FrontmatterChips
      frontmatter={entry.frontmatter as Record<string, unknown>}
      state={entry.frontmatterState}
    />
  )
  ```
- **All three "surfaces" share one component**: `LibraryDocView` is
  routed under `library/$type/$fileSlug` (`router.ts:109-112`).
  Validations, plan-reviews, and work-item-reviews are URL configurations
  of the same React component, not separate page files. Verdict-tone
  changes apply once, everywhere.
- **`statusToChipVariant` has four importers** — DO NOT delete or rename
  the helper:
  - `components/FrontmatterChips/FrontmatterChips.tsx:2,38`
  - `routes/library/LibraryTypeView.tsx:21,252`
  - `routes/lifecycle/LifecycleClusterView.tsx:14,139`
  - `components/FilterPill/FilterPill.tsx:4,151`
  Test: `api/status-variant.test.ts`. The Technical Notes' "retained
  as a helper consumed by `StatusBadge`" is the only safe path; the
  "absorbed into `StatusBadge`" alternative would need to update the
  three other consumers.
- **No pre-existing `StatusBadge` or singular `FrontmatterChip`** — both
  are net-new files. Closest existing surfaces:
  - `Chip` primitive (the wrapped target).
  - `FilterPill` (a separate sidebar-filter chip with its own logic,
    not relevant).

### Upstream verdict emitters (where `verdict:` comes from)

- **Plan review** — `skills/planning/review-plan/SKILL.md`:
  - Frontmatter template at line 417-428 emits `verdict: {APPROVE | REVISE | COMMENT}`.
  - Re-review overwrites only `verdict`, `review_pass`, `date` (lines 532-534).
  - **`REQUEST_CHANGES` is NOT in the plan-review vocabulary**. The
    work item lists it ("plan-review verdicts are `APPROVE` / `REVISE`
    / `REQUEST_CHANGES` / `COMMENT`") but this is incorrect against
    the current SKILL template and against
    `meta/decisions/ADR-0007-divergent-verdict-semantics-for-plan-and-pr-reviews.md`,
    which assigns `REQUEST_CHANGES` to PR reviews only.
- **Work-item review** — `skills/work/review-work-item/SKILL.md`:
  - Frontmatter template at line 352-363 emits `verdict: {APPROVE | REVISE | COMMENT}`.
  - Same three-value vocabulary as plan-review (per ADR-0007). The
    work item's work-item-review AC ("verdict value from the plan-review
    vocabulary") is satisfied by the actually-emitted three values.
- **PR review** — `skills/github/review-pr/SKILL.md` (line 460):
  emits `APPROVE | REQUEST_CHANGES | COMMENT`. **Not** a surface
  touched by 0081, but the source of the `REQUEST_CHANGES` value the
  work item miscategorised as plan-review.
- **Validation** — `skills/planning/validate-plan/SKILL.md`:
  - Frontmatter template at lines 131-142 emits `result: {pass | partial | fail}` and `status: complete`.
  - **Does NOT emit `verdict`**. The validation-vocabulary ACs in 0081
    cannot pass against current corpus output. Three resolution paths:
    1. Update `validate-plan` to emit `verdict:` (separate work item, upstream of 0081).
    2. Extend `StatusBadge` to also recognise `result` with the same `pass`/`fail` tone mapping.
    3. Rename `result` → `verdict` corpus-wide.
  - The work item's Coordinates-with entry ("Validation-page verdict
    emitter (no work item currently named; assumed to already surface
    `verdict` as a top-level frontmatter key)") flags this but does not
    block on it — planning should pick one of the three paths.
- ADR for verdict semantics: `meta/decisions/ADR-0007-divergent-verdict-semantics-for-plan-and-pr-reviews.md`.

### Page data plumbing (no changes needed but useful context)

- `LibraryDocView` fetches the type's docs list via
  `useQuery({ queryKey: ['docs', type], queryFn: () => fetchDocs(type) })`
  (`LibraryDocView.tsx:44-52`) and finds the entry by slug. Frontmatter
  arrives as an already-parsed `Record<string, unknown>` on each
  `IndexEntry` (`api/types.ts:64-80`) with a discriminant
  `frontmatterState: 'parsed' | 'absent' | 'malformed'`.
- `fetchDocs` hits `GET /api/docs?type=<type>` (`api/fetch.ts:66-71`),
  served by the Rust visualiser server (no Python).
- The body-side `FrontmatterTable` (`LibraryDocView.tsx:136-142`,
  shipped by 0078) renders the same frontmatter as a key/value table;
  it does not apply verdict tone, and 0081 does not require it to.
  Question for planning: should the table reuse `FrontmatterChip` /
  `StatusBadge` for its value cells, or is that 0084's concern?

## Code References

- `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx:1-47` — the component being decomposed
- `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.test.tsx:1-130` — existing tests to reshuffle
- `skills/visualisation/visualise/frontend/src/api/status-variant.ts:3-26` — canonical status tone mapping (broader than the AC table)
- `skills/visualisation/visualise/frontend/src/components/Chip/Chip.tsx:4-24` — primitive props, variants, `data-*` swallow bug
- `skills/visualisation/visualise/frontend/src/components/Chip/Chip.module.css:33-52` — variant rendering
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:95-100` — sole call site of `FrontmatterChips`
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:136-142` — adjacent `FrontmatterTable` from 0078
- `skills/visualisation/visualise/frontend/src/router.ts:109-112` — shared route for all detail surfaces
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.tsx:21,252` — second importer of `statusToChipVariant`
- `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleClusterView.tsx:14,139` — third importer
- `skills/visualisation/visualise/frontend/src/components/FilterPill/FilterPill.tsx:4,151` — fourth importer
- `skills/planning/review-plan/SKILL.md:417-428,532-534` — plan-review verdict emission
- `skills/work/review-work-item/SKILL.md:352-363,466-468` — work-item-review verdict emission
- `skills/planning/validate-plan/SKILL.md:131-142` — validation emits `result:`, not `verdict:`
- `skills/github/review-pr/SKILL.md:460` — PR review (source of `REQUEST_CHANGES`)
- `meta/decisions/ADR-0007-divergent-verdict-semantics-for-plan-and-pr-reviews.md` — verdict-vocabulary boundaries

## Architecture Insights

- **One detail page, not three**: All review/validation/work-item-review
  surfaces share `LibraryDocView` driven by a route param. Component-level
  changes touch one render path; per-surface behaviour comes from
  frontmatter shape and the dispatch inside the chip-list renderer.
- **Composition over inheritance is already the codebase norm**: the
  work item's "concrete `FrontmatterChip`" framing (`StatusBadge`
  *composes* `FrontmatterChip` and resolves tone, rather than
  subclassing) aligns with how `FrontmatterChips`-the-original composes
  `Chip` today.
- **CSS-modules + `data-*` attribute selectors** is the established
  styling pattern. The Chip's `data-variant` attribute drives both
  rendering and the existing test query selectors
  (`container.querySelector('[data-variant="green"]')`). Adding
  `data-component` to chips fits the same pattern.
- **`statusToChipVariant` aspires to be the corpus's single tone map**:
  four consumers, one test file. Keep it. New verdict logic should
  sit beside it (e.g. `verdictToChipVariant` + `isVerdictKey` in the
  same file) rather than being inlined into `StatusBadge`.
- **The detail-page chip strip is unbounded today** (0084 will cap it
  at 4 chips). 0081 is a prerequisite for 0084's "status / verdict /
  date / author only" cap because verdict has no semantic colour to
  carry until 0081 ships.

## Historical Context

- `meta/work/0038-generic-chip-component.md` — Chip primitive work
  item that produced the six-variant `Chip`. Pass-2 reviewer of 0081
  mistakenly thought `red` was missing; the misread was corrected in
  pass 3 after re-reading 0038's Desired End State.
- `meta/work/0005-plan-review-verdict-semantics.md` — defines
  plan-review verdict semantics; promoted from Coordinates-with to
  Blocked-by between 0081 review passes.
- `meta/decisions/ADR-0007-divergent-verdict-semantics-for-plan-and-pr-reviews.md`
  — codifies why plan/work-item reviews use `APPROVE | REVISE | COMMENT`
  while PR reviews use `APPROVE | REQUEST_CHANGES | COMMENT`. The work
  item appears to have collapsed both sets when listing the plan-review
  vocabulary.
- `meta/work/0066-update-review-skills-inline-frontmatter.md` — ongoing
  work on review-skill frontmatter format; review-skills already inline
  YAML today, so 0066 is likely format normalisation. The work item's
  ordering concern ("if 0066 lands a different vocabulary first…") is
  surfaced explicitly in Open Questions.
- `meta/work/0084-detail-page-chip-strip-cap.md` — downstream consumer
  blocked by 0081; will cap the chip strip at 4 chips (status, verdict,
  date, author).
- `meta/work/0078-detail-page-frontmatter-table.md` — adjacent shipped
  work that added `FrontmatterTable` next to `FrontmatterChips` in
  `LibraryDocView`. Open question for the planner: should the table's
  value cells consume the same per-key components?
- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
  — source design-gap doc; the "Implement StatusBadge" entry under
  §Net-New Features (lines 446-450) and the §Component Drift "We need
  a StatusBadge component" paragraph (lines 167-175) seeded the work
  item.
- `meta/reviews/work/0081-status-badge-component-review-1.md` — four
  review passes. Pass 4 verdict was APPROVE; ready for implementation.
  Key historical refinement: the original draft framed the work as a
  helper extension, was rewritten as a wrapper, then settled (pass 3)
  on a full three-component decomposition with composition (not
  subclassing).
- `meta/research/codebase/2026-05-15-0038-generic-chip-component.md` —
  prior research on the Chip primitive; useful background for the
  variant inventory.
- `meta/research/codebase/2026-05-21-0078-detail-page-frontmatter-table.md`
  — prior research on the adjacent table component.

## Open Questions

1. **`data-component` mechanism**: extend `Chip` to pass-through
   `data-*` attributes, wrap `Chip` in an outer span, or add a
   dedicated `dataComponent` prop to `Chip`? The work item's AC says
   "e.g. a `data-component` attribute" — phrasing leaves room for an
   alternative observable hook, but `data-component` is the most
   consistent with the existing `data-variant` pattern.
2. **`statusToChipVariant` AC fidelity**: the AC table in 0081 is a
   strict subset of the live mapping. Does "preserve current mapping
   exactly" trump the AC table, or should the AC table be expanded to
   match the live code before implementation begins? Recommend
   expanding the AC table (the work item's Technical Notes already
   names `statusToChipVariant` as canonical SoT).
3. **Validation `verdict:` vs `result:`**: the work item's
   Assumption is wrong against the current `validate-plan` skill.
   Three resolution paths (see Detailed Findings → Upstream verdict
   emitters). Recommend resolving in planning (likely path 2 — extend
   `StatusBadge` to also recognise `result` — to avoid a corpus
   migration).
4. **`REQUEST_CHANGES` in plan-review vocabulary**: per ADR-0007 and
   the live SKILL templates, plan-review verdicts are `APPROVE | REVISE
   | COMMENT` only. The work item's AC for `verdict: REQUEST_CHANGES`
   → red on the plan-review page is unit-testable but exercises a
   value the live emitter never produces. Drop the AC, retain it as a
   defensive case, or recategorise as PR-review (out of 0081 scope)?
5. **`FrontmatterTable` interaction**: should the table's value cells
   reuse `FrontmatterChip` / `StatusBadge`, or is the table out of
   scope until 0084? Currently the table has no tone awareness
   (`LibraryDocView.tsx:136-142`).
6. **`absent` literal vs sentinel**: pass-3 reviewer noted ambiguity
   in the status table — "`absent`" as a literal string (in the
   neutral row) reads like a sentinel for missing-key. The AC says
   "literal string matches"; if a planner takes it at face value, no
   action needed.

## Related Research

- `meta/research/codebase/2026-05-15-0038-generic-chip-component.md` — Chip primitive background
- `meta/research/codebase/2026-05-21-0078-detail-page-frontmatter-table.md` — adjacent table component
- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md` — gap analysis seeding 0081
- `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/inventory.md` — prototype source
- `meta/research/design-inventories/2026-05-21-004250-current-app/inventory.md` — current-app source
