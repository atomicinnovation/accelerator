---
date: "2026-05-26T15:58:09+00:00"
author: Toby Clemson
revision: "745a54caf27eb8e5344023cc826231aacda82356"
change_id: lswwkotttlkmvnxlrymqkqtqtmrkzmwl
repository: accelerator
topic: "Detail-Page Chip Strip Cap (Status, Date, Author) — codebase landscape for work item 0084"
tags: [research, codebase, frontend, visualiser, frontmatter-chips, detail-page, status-badge, frontmatter-table, adr-0033]
status: complete
last_updated: "2026-05-26T00:00:00+00:00"
last_updated_by: Toby Clemson
type: codebase-research
id: "2026-05-26-0084-detail-page-chip-strip-cap"
title: "Research: Detail-Page Chip Strip Cap (Status, Date, Author) — codebase landscape for work item 0084"
schema_version: 1
relates_to: ["codebase-research:2026-05-22-0081-status-badge-component", "codebase-research:2026-05-21-0078-detail-page-frontmatter-table", "codebase-research:2026-05-14-0038-generic-chip-component", "adr:ADR-0033", "work-item:0084"]
derived_from: ["codebase-research:2026-05-22-0081-status-badge-component", "codebase-research:2026-05-21-0078-detail-page-frontmatter-table", "codebase-research:2026-05-14-0038-generic-chip-component", "design-gap:2026-05-21-current-app-vs-claude-design-prototype", "design-inventory:2026-05-21-015231-claude-design-prototype", "adr:ADR-0033"]
---

# Research: Detail-Page Chip Strip Cap (Status, Date, Author) — codebase landscape for work item 0084

**Date**: 2026-05-26 15:58:09 UTC
**Author**: Toby Clemson
**Git Commit**: 745a54caf27eb8e5344023cc826231aacda82356
**Change ID**: lswwkotttlkmvnxlrymqkqtqtmrkzmwl
**Repository**: accelerator

## Research Question

Map the codebase landscape that work item 0084 ("Detail-Page Chip Strip
Cap") will land on: the current behaviour of `FrontmatterChips`, the
adjacent components delivered by its three named dependencies (0038
chip primitive, 0078 frontmatter table, 0081 StatusBadge), the page
shell that hosts the chip strip in its subtitle slot, the doc-kind
machinery, the schema authority (ADR-0033), and the source design-gap
document the work item cites. Surface every concrete fact an
implementer needs to plan the change, plus any gaps between the work
item's requirements and what the current code actually does.

## Summary

The chip strip is implemented by `FrontmatterChips` at
`skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx`
(co-located CSS module and test file). It has exactly one runtime
consumer, `LibraryDocView`, which is the single detail-page component
all twelve doc kinds funnel through. The component currently iterates
**every** non-null frontmatter key in source order with no whitelist
and dispatches three known keys (`status`, `verdict`, `result`) to
their badge components and the rest to a neutral `FrontmatterChip`.
There is no chip cap, no fixed ordering across kinds, no caller-facing
prop that selects keys, and — most relevant to the work item's
"empty container preserving height" requirement — it
`return null`s in both the `'absent'` state and the parsed-but-zero
case, so the subtitle wrapper around it collapses to 0 height.

The three named dependencies are all delivered and in place: the
`Chip` primitive exposes a six-tone `variant` API (0038); the
`FrontmatterTable` renders all frontmatter keys without filtering
(0078); `StatusBadge` wraps `FrontmatterChip` with `statusToVariant`
to colour status (0081). After 0084's cap is in place, `status`,
`date`, and `author` will appear in **both** the chip strip and the
frontmatter table — the work item does not state whether this
duplication is intended, but the prototype's design (per the
2026-05-21 design-gap inventory) explicitly tolerates it, so this
seems acceptable.

Two material gaps between the work item and the live codebase
warrant attention before implementation:

1. **The work item's framing of the prototype is slightly wrong**: it
   says the prototype "caps at four chips" but the design-inventory
   shows the prototype has no cap at all — it conditionally renders
   the four keys it knows about (status, verdict, date, author) in
   fixed order. The work item's three-chip cap is genuinely new
   behaviour, not a recreation. The review (review-1) already noted
   this nit. This does not change implementation but should colour
   the test assertions: there is no prior "what the prototype does"
   to copy.
2. **The "empty container at one-chip height" requirement is a new
   invention with no precedent in either the prototype or the source
   design-gap document**. The source doc only observes that
   "notes render with no chips, leaving H1 sitting directly over the
   divider" — there is no spacing/sizing precedent for what the
   preserved height should be. Implementer judgment will be needed
   for the exact min-height value; the cleanest token-aligned
   approach is `min-height: calc(1em * 1.5)` (matching the subtitle
   slot's existing line-height) on the chip-strip container.

The work item's whitelist (`status`, `date`, `author`) aligns
precisely with three of the eleven unified base frontmatter fields
mandated by ADR-0033. `date` and `last_updated`/`last_updated_by` are
distinct base fields by design (creation- vs mutation-anchored), so
the work item's precedence rules (prefer `date` over `last_updated`,
`author` over `last_updated_by`) match the ADR's intent without
needing to re-specify it.

## Detailed Findings

### `FrontmatterChips` — current behaviour

**File**: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx`

#### Props

Discriminated-union prop shape (`FrontmatterChips.tsx:8-11`):

```ts
type FrontmatterChipsProps =
  | { state: 'absent' }
  | { state: 'malformed' }
  | { state: 'parsed'; frontmatter: Record<string, unknown> }
```

There is no `keys`/`include`/`exclude`/`renderChip` prop. The only
implicit escape hatch is that the component iterates every key in
the `frontmatter` object — closing that hatch is exactly what 0084
proposes.

#### Key extraction and dispatch (current behaviour)

`FrontmatterChips.tsx:40-44` filters out null/undefined/empty-string
values and iterates the rest in `Object.entries` order
(YAML-insertion order). Dispatch table at
`FrontmatterChips.tsx:20-28`:

- `status` → `StatusBadge`
- `verdict` → `VerdictBadge`
- `result` → `ResultBadge`
- everything else → `FrontmatterChip` (defaults to `variant="neutral"`
  via `FrontmatterChip.tsx:19`)

So `date` and `author` already render as neutral chips today — the
only structural change for 0084 is to whitelist them and reject all
non-canonical keys. **Status already routes through `StatusBadge`**,
matching the work item's Technical Note that "Status chip rendering
already goes through `StatusBadge` (0081); this story does not change
`StatusBadge`".

Case-folding: `badgeFor()` lowercases and strips separators
(`FrontmatterChips.tsx:26-28`), so `Status`/`STATUS` all dispatch to
`StatusBadge`. The whitelist should reuse this normalisation.

#### Empty / missing handling (current)

Three null-paths:

- `state === 'absent'` → `return null` (`FrontmatterChips.tsx:31`)
- `state === 'malformed'` → renders a `role="alert"` banner
  (`FrontmatterChips.tsx:32-38`)
- `state === 'parsed'` with zero entries after filtering →
  `return null` (`FrontmatterChips.tsx:46`)

In the parsed/empty case, **no DOM container is rendered at all**.
The work item's "empty container with non-zero rendered height"
requirement directly contradicts this — implementation must change
both `return null` paths in the parsed branch to render the
container (and arguably also the `'absent'` path, depending on
whether the subtitle slot should reserve height for un-frontmattered
files at all).

#### Container layout (current)

`FrontmatterChips.module.css:1-6`:

```css
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
  margin: 0 0 var(--sp-4);
}
```

No `min-height`, no `line-height`. For the empty case to preserve a
one-chip rendered height, add a `min-height` here matching the
chip's line-box. The container inherits `line-height: 1.5` from the
ancestor `.subtitle` (`Page.module.css:57`), so
`min-height: calc(1em * 1.5)` would match a single line of chip
text. The chip's outer height adds `2 × 0.125rem` (padding) + `2 ×
1px` (border) on top — a more precise token would be
`calc(1em * 1.5 + 0.25rem + 2px)`, but the simpler `calc(1em * 1.5)`
matches the subtitle slot's existing line-height rhythm and is what
the H1/divider spacing was tuned against.

#### Existing tests (will need rewriting)

`FrontmatterChips.test.tsx` enumeration:

- **Parsed state** (`:7-37`): 3 chips from `status`/`date`/`author`;
  skips null/undefined/empty-string. — *Survives.*
- **Absent state** (`:39-44`): asserts `container.firstChild` is null.
  — *Changes if 'absent' must render the height-preserving
  placeholder.*
- **Malformed state** (`:46-56`): `role="alert"`, banner text. —
  *Survives.*
- **CSS source assertions** (`:58-65`): `.chip` rule absent;
  `.banner` rule present. — *Survives (extend with min-height
  assertion).*
- **Dispatch** (`:67-112`): `status`/`verdict`/`result` → badge
  components; case-folded variants accepted. — *Partially survives:
  `verdict` and `result` dispatch tests must be removed because
  those keys are no longer in the strip's whitelist.*
- **Source-order** (`:114-128`): asserts `verdict, status, priority`
  render in input order. — *Must be replaced with a "fixed canonical
  order" test using `[status, date, author]` and asserting reordered
  input still renders in canonical order.*
- **AC integration fixtures** (`:130-174`): plan-review fixture
  asserts `status, verdict, priority, tags` all render. — *Conflicts
  directly with the whitelist; must be replaced with assertions that
  `priority` and `tags` are **excluded** and verdict appears only in
  the table.*

The work item's AC for parameterised verification across 12 doc
kinds maps cleanly onto an extension of the existing fixture-driven
test style at `:130-174` — same approach, but iterating a fixture
matrix of 12 kinds each with an extra non-canonical key.

#### Sole consumer

`skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx`
imports `FrontmatterChips` at line 10 and renders it at lines 93-100
inside a `subtitle` local, then passes that to `<Page>` at line 153.
The component is the single detail-page surface for all 12 doc kinds
— there is no per-kind variant (confirmed by `LibraryDocView.dispatch.test.tsx`
which parametrises by `type` against the same component). The chip
strip is wired identically across kinds.

### `Page` subtitle slot and divider — relevant to the height requirement

**File**: `skills/visualisation/visualise/frontend/src/components/Page/Page.tsx`

The subtitle slot at `Page.tsx:32-34`:

```tsx
{subtitle !== undefined && (
  <div className={styles.subtitle} data-slot="subtitle">{subtitle}</div>
)}
```

Gate is `subtitle !== undefined`, not truthiness — so passing the
`<FrontmatterChips>` element always satisfies the gate. The slot's
CSS (`Page.module.css:52-60`) declares
`display: inline-flex; gap: var(--sp-2); line-height: 1.5;
margin-top: 4px;` with no `min-height`.

In the zero-chip case today, `LibraryDocView` always passes the
`<FrontmatterChips>` element (chips local is unconditionally set in
the success branch at `LibraryDocView.tsx:93-100`), so the
`.subtitle` wrapper renders. But `FrontmatterChips` returns null
internally, so the wrapper has no children and collapses to height 0
— which is the exact "H1 sitting on the divider" failure mode the
source design-gap doc describes (line 286-287 of
`meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`).

The divider at `Page.tsx:41` is an `<hr>` with no top margin
(`Page.module.css:68-72`). The gap between subtitle and divider
comes from `.header { padding-block: var(--sp-5) var(--sp-4); }`
(`Page.module.css:20`). When subtitle collapses, the divider sits
just `var(--sp-4)` below the H1 baseline.

#### Where to put the `min-height`

Two clean options:

- **(A) On `.chips` in `FrontmatterChips.module.css`** — aligns with
  the work item's wording ("the chip-strip container renders with
  the same vertical height as a one-chip strip"). Requires removing
  the parsed-branch `return null` so the container is always in the
  DOM. *Recommended.*
- **(B) On `.subtitle` in `Page.module.css`** — preserves the
  subtitle slot height regardless of what's inside. Cleaner
  separation, but changes Page's contract for all subtitle uses
  (currently only `LibraryDocView`), and the work item specifically
  says "chip-strip container" not "subtitle slot".

A third option (always pass a placeholder `<span>` from
`LibraryDocView`) was considered and rejected — the work item
explicitly states "The cap is a property of `FrontmatterChips`
itself … No prop opens the whitelist back up" (Technical Notes); the
height-preservation belongs in the component, not the caller.

#### One-chip rendered height — concrete value

The chip primitive's intrinsic outer height (`Chip.module.css:1-15`
for `size="sm"`, the default used by `FrontmatterChip` /
`StatusBadge`):

- `padding: 0.125rem var(--sp-2)` → 2 × 2px vertical
- `border: 1px solid` → 2 × 1px
- `font-size: var(--size-3xs-lg)` with inherited `line-height: 1.5`
  from `.subtitle`

So a one-chip line-box ≈ `font-size × 1.5 + 4px (padding) + 2px
(border)`. The container's `.chips` adds `margin-bottom: var(--sp-4)`
but no top margin. The simplest token-friendly expression is
`min-height: 1lh` (one line-height) on `.chips` — yields exactly the
height of the chip's text line-box in the inherited cascade, with no
hard-coded magic numbers. If browser support for `1lh` is a concern
(it is well-supported in evergreen browsers; check the project's
browserslist), `min-height: calc(var(--size-3xs-lg) * 1.5)` is the
equivalent fallback.

### `FrontmatterTable` (0078) — destination surface, with duplication implication

**File**: `skills/visualisation/visualise/frontend/src/components/FrontmatterTable/FrontmatterTable.tsx`

The table renders every key from `Object.entries(frontmatter)` in
source order — no whitelist, no exclusion list, no de-duplication
against the chip strip (`FrontmatterTable.tsx:94-99`). Its test at
`FrontmatterTable.test.tsx:21-52` locks this in by asserting nine
rows for a record containing `status`, `date`, and `author`.

**Implication for 0084**: after the cap is applied, `status`, `date`,
and `author` will appear in both surfaces. The work item does not
flag this and the prototype's design (per
`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/inventory.md:511`)
explicitly tolerates the same duplication (plan capture shows
`title, type, status, date, last_updated, author, slug` in the table
alongside the chip strip). So the duplication is consistent with
the design intent and does not need a code change in 0084 — but it
is worth calling out so that nobody adds an exclusion to
`FrontmatterTable` thinking it's the obvious cleanup.

### `StatusBadge` (0081), `Chip` (0038), `statusToVariant` — unchanged by 0084

- `StatusBadge` props: just `{ value: unknown }` — internally
  delegates to `FrontmatterChip` with `variant={statusToVariant(value)}`
  (`StatusBadge.tsx:4-17`). The work item's whitelist sends `status`
  here exactly as it does today; no change.
- `Chip` variants:
  `'neutral' | 'indigo' | 'green' | 'amber' | 'red' | 'violet'`
  (`Chip.tsx:4`). `neutral` is what `FrontmatterChip` defaults to
  (`FrontmatterChip.tsx:19`); the date/author chips already use it.
- `statusToVariant` exports a stable mapping
  (`status-variant.ts:11`), normalised via `normaliseValue`. Other
  consumers: `LifecycleClusterView`, `LibraryTypeView`, `FilterPill`
  — none of which are detail-page surfaces, so they're unaffected
  by 0084's cap.

### Chip primitive `data-*` pass-through limitation

The chip primitive's `...rest` spread reads `aria-label` but
silently drops `data-*` attributes (from the 0038 implementation,
confirmed in
`meta/research/codebase/2026-05-22-0081-status-badge-component.md`
lines 204-219). The wrapper components do attach `data-testid`
manually (`status-badge`, `verdict-badge`, `result-badge`,
`frontmatter-chip`), and the chip itself emits `data-variant`
attributes for CSS targeting. Test assertions for the whitelist
should use these existing data hooks — `getAllByTestId('status-badge')`,
`querySelector('[data-variant]')`, or `aria-label` — rather than
attempting to add new `data-*` markers.

### Doc-kind machinery — fully generic frontmatter

The 12 doc kinds are enumerated in the Rust server
(`skills/visualisation/visualise/server/src/docs.rs`:
`enum DocTypeKey`, `DocTypeKey::all()` returning the array) and
mirrored on the frontend
(`skills/visualisation/visualise/frontend/src/api/types.ts:4-14`:
`DocTypeKey` string-literal union + `DOC_TYPE_KEYS` runtime array).

**Important for 0084**: there is **no per-kind TypeScript type or
Zod schema for frontmatter** — every consumer treats it as
`Record<string, unknown>`. The work item's "verified across the 12
doc kinds" AC therefore lands at the runtime/render layer, not at a
type layer. Per-kind fixtures in the parameterised test are the
natural approach.

A 13th key (`templates`) exists as a `VIRTUAL_DOC_TYPE_KEYS` entry
(`types.ts:30`) but it is not a real doc kind — it lists templates
for creating new docs. The work item's 12-kind list is correct.

### ADR-0033 — schema alignment

`status`, `date`, and `author` are three of the eleven unified base
fields the ADR mandates on every artifact (ADR-0033 lines 113-125).
Critically:

- `date` = **creation** timestamp; `last_updated` = mutation
  timestamp (ADR-0033 lines 118, 123-124, 259-260). This matches
  the work item's precedence rule (date chip uses `date`, not
  `last_updated`).
- `author` = **human creator**; `last_updated_by` = mutation actor.
  Matches the work item's `author` precedence rule.
- `producer` is the **skill/automated agent** identifier (separate
  field) — explicitly not chip-eligible under 0084's whitelist.

The work item's whitelist therefore pins three of the ADR's
identity-anchored mandatory fields — schema-consistent, not arbitrary.

The work item's frontmatter omits ADR-0033's `id`, `type`,
`schema_version`, `last_updated`, `last_updated_by`, `producer`. The
review (review-1, recommendation 8) called this out and the work
item now defers full alignment to "the corpus migration tracked
under epic 0057" (Open Questions). Independent of 0084's
implementation.

### Source design-gap document — discrepancies with the work item

`meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
is cited by the work item as its source. Two discrepancies:

1. **Cap count**: source recommends a **four-chip** cap
   (status + verdict + date + author, lines 291-293, 587-588);
   work item adopts **three-chip** (drops verdict). The work item's
   Context and Drafting Notes acknowledge this divergence with a
   rationale ("verdict beside status creates two coloured-tone slots
   that compete for attention"). Review-1 approved. Verdict moves
   to the frontmatter table.
2. **Source treats verdict as semantically important**: lines
   167-175 explicitly state "we need a single component that maps
   both [status and verdict] keys identically, so review and
   validation pages signal their outcome at a glance." Dropping
   verdict from the chip strip undoes part of the source's intent
   — but it does not block 0084's implementation, since the
   `verdict` value still renders (just in the table rather than as
   a coloured chip). If review/validation pages need at-a-glance
   verdict signalling later, a follow-up story can re-introduce a
   verdict surface elsewhere.

### Prototype design-inventory — the cap is genuinely new

The work item's Context says "the prototype's chip row caps at four
chips" but the inventory at
`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/inventory.md`
shows the prototype's chip row at `view-library.jsx:343-348`
**conditionally renders the four known keys** — it has no enforced
cap, just a known set of consumers. There is no notes-file
placeholder behaviour in the prototype; notes render
`date + author` and the container collapses just like the current
app's does.

**Implication**: the "empty container preserving one-chip height" is
a genuinely new behaviour invented by 0084, not a recreation of
prototype behaviour. There is no source precedent for the exact
height value — implementer judgment will be needed.

## Code References

### Implementation surface (changes land here)

- `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx:8-11` — discriminated-union props (no escape hatch to add).
- `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx:20-28` — `BADGE_FOR_KEY` map and `badgeFor()` normalisation; whitelist should reuse the normalisation pattern.
- `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx:31` — `'absent'` early-return; revisit if the height-preserving placeholder must render here too.
- `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx:40-49` — parsed-branch filter, dispatch loop, and container; change to whitelist + canonical-order iteration, remove the `return null` at line 46, keep the container always rendered.
- `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.module.css:1-6` — `.chips` rule; add `min-height` for the zero-chip case.
- `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.test.tsx:67-174` — dispatch and AC fixture tests need rewriting; parameterised 12-kind test is a new addition.

### Adjacent surfaces (read-only references)

- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:10,93-100,153` — sole runtime consumer of `FrontmatterChips`; subtitle wiring.
- `skills/visualisation/visualise/frontend/src/components/Page/Page.tsx:32-34,41` — subtitle slot DOM and divider.
- `skills/visualisation/visualise/frontend/src/components/Page/Page.module.css:52-60,68-72` — subtitle and divider styles, line-height that the chip-strip min-height must match.
- `skills/visualisation/visualise/frontend/src/components/Chip/Chip.module.css:1-15` — chip intrinsic dimensions.
- `skills/visualisation/visualise/frontend/src/components/StatusBadge/StatusBadge.tsx:4-17` — status chip wrapper (unchanged).
- `skills/visualisation/visualise/frontend/src/components/FrontmatterChip/FrontmatterChip.tsx:19` — neutral default for non-badge chips.
- `skills/visualisation/visualise/frontend/src/components/FrontmatterTable/FrontmatterTable.tsx:94-99` — renders all keys; will duplicate status/date/author with chip strip (acceptable, matches prototype).
- `skills/visualisation/visualise/frontend/src/api/types.ts:4-14` — `DocTypeKey` enumeration for parameterised 12-kind test.

## Architecture Insights

- **Single-component detail page**: all 12 doc kinds route through
  `LibraryDocView`; there is no per-kind component fork. This makes
  the cap a single-component change and supports a parameterised
  test rather than a per-kind switch.
- **Generic frontmatter typing**: `Record<string, unknown>` end to
  end; the only normalisation lives in case-folded key lookups
  (`badgeFor()`, `isStatusKey`). The 0084 whitelist should adopt the
  same case-folding pattern.
- **Chip primitive `data-variant` is the existing observability
  hook** for test assertions about chip identity/colour; `data-*`
  spread is silently dropped by the chip primitive (0038
  limitation). Use `data-testid` on wrapper components.
- **Source-order rendering is incidental, not contractual**: tests
  pin it, but the 0084 cap replaces it with a fixed canonical
  order. The existing source-order test will be the one that flips
  signs — replace it with a fixed-order test that explicitly
  scrambles input keys and asserts canonical output order.
- **`StatusBadge` is reused outside the chip strip**
  (`LifecycleClusterView`, `LibraryTypeView`, `FilterPill`), so any
  refactor of `statusToVariant` would have a blast radius beyond
  detail pages — but 0084 doesn't touch it.
- **The `Page` subtitle wrapper renders on `subtitle !== undefined`,
  not on truthiness**: the empty-container behaviour for 0084
  doesn't require any Page change.

## Historical Context

- `meta/research/codebase/2026-05-22-0081-status-badge-component.md`
  explicitly names 0084 as the downstream consumer of `StatusBadge`
  (lines 343-345). It also flags an open question (lines 416-419)
  on `FrontmatterTable` tone-awareness — 0084 sidesteps this by not
  changing the table.
- `meta/research/codebase/2026-05-21-0078-detail-page-frontmatter-table.md`
  records 0084 as the cap story (lines 302-305) and confirms the
  table has no chip-duplication-aware filter — exactly what this
  research found again.
- `meta/research/codebase/2026-05-14-0038-generic-chip-component.md`
  documents the six-variant API and the `data-*` pass-through
  limitation. Both still hold.
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`
  (Accepted, 2026-05-19) authoritatively defines `status`, `date`,
  `author` as three of eleven mandatory base fields, and pins the
  `date` vs `last_updated` and `author` vs `last_updated_by`
  semantic split that 0084's precedence rules depend on.
- `meta/reviews/work/0084-detail-page-chip-strip-cap-review-1.md`
  (APPROVE, 2026-05-26) has already absorbed ten recommendations
  into the work item, including the parameterised-test specification
  and the dependency-on-ADR-0033 entry.

## Related Research

- `meta/research/codebase/2026-05-22-0081-status-badge-component.md`
  — StatusBadge implementation landscape.
- `meta/research/codebase/2026-05-21-0078-detail-page-frontmatter-table.md`
  — FrontmatterTable implementation landscape (the chip strip's
  pair surface).
- `meta/research/codebase/2026-05-14-0038-generic-chip-component.md`
  — Chip primitive landscape and variant API.
- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
  — Source design-gap document; the citation for the cap concept.
- `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/inventory.md`
  — Prototype runtime captures (chip strip and frontmatter table).
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` —
  Schema authority for the canonical key set.

## Open Questions

1. **Empty container scope: parsed-only or all states?** The work
   item's Acceptance Criteria focus on the parsed branch with zero
   qualifying keys. Should the `'absent'` state (no frontmatter at
   all) also render the height-preserving container, or stay
   `null`? The two are visually equivalent today (both produce a
   collapsed slot), but the work item only explicitly addresses the
   parsed branch. Treating them identically (always render the
   container in `'parsed'`, keep `null` in `'absent'`) is a defensible
   reading; treating both identically (render in both) is also
   defensible. The work item is silent — flag in implementation.

2. **Exact min-height token**: `1lh` (one line-height) is the
   cleanest CSS expression matching the work item's "same vertical
   height as a one-chip strip" wording, but `1lh` browser support
   is evergreen-only (Chrome 99+, Firefox 109+, Safari 16.4+).
   `calc(var(--size-3xs-lg) * 1.5)` is an equivalent fallback. The
   project's browserslist should be checked before committing to
   `1lh`. (Outside this research's scope to fix.)

3. **Verdict resurfacing strategy**: the source design-gap doc
   wants verdict shown with a tone-mapped chip; 0084 moves it to
   the table where it loses tone (table has no tone awareness). If
   this becomes a real UX gap on review/validation pages, a
   follow-up story is needed. Not blocking, but worth recording.

4. **Schema-alignment follow-up story ID**: work item 0084 names
   this as "TBD" — capturing it as a concrete work item (covering
   `author` on plans / pr-descriptions / plan-reviews /
   work-item-reviews / pr-reviews) would let 0084 fill in the
   "Enables (once created)" line and make the Blocks edge real.
   Independent of implementation.
