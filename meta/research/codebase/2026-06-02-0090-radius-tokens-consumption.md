---
type: codebase-research
id: "2026-06-02-0090-radius-tokens-consumption"
title: "Research: Radius Tokens Consumption (0090) — current-app radius inventory and migration context"
date: "2026-06-02T15:45:49+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0090"
topic: "Full current-app border-radius inventory, pre-migration computed values, EXCEPTIONS cleanup scope, ADR + gate + Playwright-spec context for radius token consumption"
tags: [research, codebase, visualiser, frontend, css, design-tokens, radius]
revision: "ed3be5b886672d63f708b91cd5baa70dba792430"
repository: "visualisation-system"
last_updated: "2026-06-02T15:45:49+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
relates_to: ["work-item:0075", "plan:2026-05-23-0075-typography-size-scale-consumption", "work-item:0033", "adr:ADR-0036", "adr:ADR-0026"]
derived_from: ["codebase-research:2026-05-23-0075-typography-size-scale-consumption"]
---

# Research: Radius Tokens Consumption (0090)

**Date**: 2026-06-02T15:45:49+00:00
**Author**: Toby Clemson
**Git Commit**: ed3be5b886672d63f708b91cd5baa70dba792430
**Branch**: visualisation-system (jj workspace; detached HEAD)
**Repository**: visualisation-system

## Research Question

This is the **pre-implementation codebase research** that work item
`meta/work/0090-radius-tokens-consumption.md` names as a hard upstream
dependency. It must:

1. Enumerate the **full current-app `border-radius` inventory** (every
   shorthand and longhand-corner declaration in `.module.css` / `.css`
   under the frontend `src/`).
2. Record the **pre-migration computed values** AC2's Playwright spec
   asserts against.
3. Determine the **`migration.test.ts` EXCEPTIONS cleanup scope** (AC5) —
   which radius literals are recorded as irreducible, and specifically
   whether the `RelatedArtifacts` 2px badge is one (0090's open question).
4. Surface **any off-scale values beyond the two known outliers** (2px,
   6px), each needing a token + a naming decision under AC8.
5. Capture the surrounding context the plan needs: the existing
   `--radius-*` scale, ADR-0026 §3 + the ADR-0036 template, the
   `*-resolved-*` Playwright pattern, and where the CI gate lives.

## Summary

**Headline finding: the inventory is materially larger than the work
item's two known outliers, and surfaces three value classes 0090 did not
anticipate.** There are **25 literal `border-radius` declarations across 12
component CSS files**, spanning **seven distinct values**: `0`, `2px`,
`3px`, `6px`, `8px`, `12px`, and `50%`.

- The two **known** outliers are confirmed: `2px` (RelatedArtifacts badge
  + others) and `6px` (Markdown `<pre>` + others).
- **`8px`** already equals `--radius-md` and **`12px`** equals
  `--radius-lg` — these are pure consume-the-existing-token migrations.
- **Three unanticipated value classes** each need a token + an AC8 naming
  decision the work item does not pre-resolve:
  - **`3px`** — 6 occurrences (FilterPill, Sidebar). Off-scale (between
    `xs` 2px and `sm` 4px); needs a *use-case* name.
  - **`50%`** — 3 occurrences (circular dots). Off the px scale entirely;
    needs a percentage/semantic token (e.g. `--radius-full: 50%`). It must
    **not** be mapped to `--radius-pill` (999px) — see the 0038 pill test.
  - **`0`** — 1 occurrence (Markdown nested `<pre>` reset). The AC3 grep
    pattern `border-radius:\s*[.0-9]` **matches `0`**, so a bare `0` would
    fail the gate; it must be tokenised (`--radius-none: 0`) or the gate
    regex amended.

So the planned **2 new tokens (`--radius-xs`, `--radius-block`) become ~5**
(`--radius-none`, `--radius-xs`, the `3px` token, `--radius-block`,
`--radius-full`). This is exactly the "further off-scale values" and
"sizing contingency" branches 0090 anticipates — but it remains a bounded,
single-PR-sized job (25 declarations, 12 files; 0075 stayed a single PR at
35 declarations across 9 files).

**Three implementation hazards the plan must handle (all in
`migration.test.ts`):**

1. **Mixed EXCEPTIONS entries → decrement, don't delete.** Most radius
   literals share an EXCEPTIONS `(file, literal)` row with non-radius uses
   of the same literal (e.g. Sidebar `'2px' x6` covers a `<mark>` radius
   *and* list-gaps *and* a loadbar height). A "declared count must equal
   observed count" hygiene gate forces you to **decrement the count and
   rewrite the reason**, not blanket-delete.
2. **AC5_FLOOR ratchet.** Migrating ~25 literals onto `var(--radius-*)`
   raises the `var(--*)` reference count, so `AC5_FLOOR` (currently `426`)
   must be bumped in the same commit or its test fails.
3. **`RADIUS_TOKENS` registry.** Every new `--radius-*` token must be added
   to `RADIUS_TOKENS` in `src/styles/tokens.ts`, or the
   "`var(--NAME)` resolves to a declared token" test fails.

**The CI gate is not a shell grep today.** All literal-consumption rules
are enforced purely by the Vitest harness `src/styles/migration.test.ts`;
the ripgrep sweeps for typography (0075) live only as *comments* documenting
a review-time approximation, with the in-test regex as the authoritative
guard. 0090's AC4 ("CI gate runs exactly AC3's three sweeps") therefore
needs a deliberate decision: follow precedent (a new Vitest `describe`
block + documented sweeps) or genuinely add ripgrep to CI (a departure).

**One AC2 blocker:** the `EmptyState` `.card` (12px) is **not reachable by
navigation** — story 0074 added fixtures for every doc type, so no
`/library/<type>` route yields an empty listing to mount it. Its computed
value cannot be asserted via a route today.

## Detailed Findings

### 1. The complete radius inventory (25 declarations, 12 files)

Authoritative ripgrep sweeps (AC3 patterns) run from
`skills/visualisation/visualise/frontend`. Sweep 2 is a superset of sweep 1
and returned an identical set → **all radius literals live in
`.module.css` files; no bare non-module `.css` carries one; there are zero
longhand-corner literals.** No multi-value shorthands exist; every
declaration is a single value.

| # | file:line | selector | purpose | value | computed `border-*-radius` (pre-migration) |
|---|-----------|----------|---------|-------|--------------------------------------------|
| 1 | `components/Breadcrumbs/Breadcrumbs.module.css:34` | `.link:focus-visible` | focus-ring corner rounding | `2px` | `2px` |
| 2 | `components/RelatedArtifacts/RelatedArtifacts.module.css:49` | `.badge` | provenance badge pill | `2px` | `2px` |
| 3 | `components/FilterPill/FilterPill.module.css:189` | `.checkbox` | custom checkbox box (13×13) | `2px` | `2px` |
| 4 | `components/Sidebar/Sidebar.module.css:428` | `.searchMark` | `<mark>` search highlight | `2px` | `2px` |
| 5 | `components/Sidebar/Sidebar.module.css:462` | `.searchLoadbar` | loading bar track (h:2px) | `2px` | `2px` |
| 6 | `components/Sidebar/Sidebar.module.css:472` | `.searchLoadbar::after` | loading bar fill | `2px` | `2px` |
| 7 | `components/FilterPill/FilterPill.module.css:84` | `.clearButton` | "Clear" text button | `3px` | `3px` |
| 8 | `components/FilterPill/FilterPill.module.css:160` | `.optionListScroll::-webkit-scrollbar-thumb` | custom scrollbar thumb | `3px` | `3px` |
| 9 | `components/FilterPill/FilterPill.module.css:174` | `.option` | facet option row | `3px` | `3px` |
| 10 | `components/Sidebar/Sidebar.module.css:107` | `.kbd` | keyboard-hint chip | `3px` | `3px` |
| 11 | `components/Sidebar/Sidebar.module.css:126` | `.searchClear` | search clear "×" (20×20) | `3px` | `3px` |
| 12 | `components/Sidebar/Sidebar.module.css:351` | `.searchHintKbd` | small kbd hint chip | `3px` | `3px` |
| 13 | `components/MarkdownRenderer/MarkdownRenderer.module.css:23` | `.markdown pre` | code block `<pre>` chrome | `6px` | `6px` |
| 14 | `components/MarkdownRenderer/MarkdownRenderer.module.css:35` | `.codeblock` | fenced code-block wrapper | `6px` | `6px` |
| 15 | `components/Pipeline/Pipeline.module.css:24` | `.tile` | pipeline stage tile | `6px` | `6px` |
| 16 | `components/Sidebar/Sidebar.module.css:300` | `.searchPanel` | inline search-results panel | `6px` | `6px` |
| 17 | `routes/lifecycle/LifecycleIndex.module.css:71` | `.card` | lifecycle index card | `6px` | `6px` |
| 18 | `routes/lifecycle/LifecycleClusterView.module.css:137` | `.pipelinePanel` | pipeline panel in cluster view | `6px` | `6px` |
| 19 | `routes/library/LibraryOverviewHub.module.css:42` | `.card` | overview-hub card | `6px` | `6px` |
| 20 | `components/FilterPill/FilterPill.module.css:41` | `.badge` | count badge (16×16) | `8px` | `8px` (= `--radius-md`) |
| 21 | `routes/library/EmptyState.module.css:14` | `.card` | full-page empty-state card | `12px` | `12px` (= `--radius-lg`) |
| 22 | `components/PipelineMini/PipelineMini.module.css:13` | `.dot` | mini status dot (8×8) | `50%` | ~`4px` (measure) |
| 23 | `routes/lifecycle/LifecycleClusterView.module.css:52` | `.stage::before` | timeline spine dot (`--dot-size:10px`) | `50%` | ~`5px` (measure) |
| 24 | `routes/library/LibraryTemplatesIndex.module.css:146` | `.tierPillBullet` | tier-pill bullet (0.3125rem=5px) | `50%` | ~`2.5px` (measure) |
| 25 | `components/MarkdownRenderer/MarkdownRenderer.module.css:42` | `.codeblock pre` | nested `<pre>` radius **reset** | `0` | `0px` |

Distinct-value counts: `2px`×6, `3px`×6, `6px`×7, `8px`×1, `12px`×1,
`50%`×3, `0`×1 = **25**.

**Computed-value notes.**
- For px literals, computed `border-radius` equals the declared px exactly;
  read it from a corner longhand (`borderTopLeftRadius`), because
  Chromium's `getComputedStyle` returns the shorthand `borderRadius` empty.
- For `50%`, the used value depends on element box size (dot 8px→4px,
  10px→5px, 5px→2.5px). **Migrating these to a token that stores `50%`
  preserves the computed value identically** — so AC2 passes whatever the
  browser returns, provided the token holds `50%` (not a px). The three
  `50%` selectors are the only ones whose computed value should be
  *measured* rather than read off the CSS; the rest are self-evident.
- `0` → `0px`.

**Already-tokenised radius declarations** (not in scope; ~40 sites already
use `var(--radius-sm|md|lg|pill)`): Chip, OriginPill, KanbanBoard/Column,
WorkItemCard, FrontmatterTable/Chips, SortPill, Toaster, Popover, Glyph,
RootLayout, TopbarIconButton, NoResultsPanel, ActivityFeed, LibraryTemplates
{Index,View}, LibraryTypeView, LibraryDocView, several FilterPill/Sidebar
rules, and the showcase routes. These confirm consumption is already the
norm; 0090 closes the remaining 25-declaration gap.

### 2. Naming decisions required (AC8)

AC8: a value equal to an existing token, or a regular end-step extension of
the `sm/md/lg/pill` ladder, takes a **scale-based** name; a value *between*
two ladder steps (no ladder slot) takes a **use-case** name with a one-line
rationale.

| value | occ. | ladder position | AC8 category | proposed token | note |
|-------|------|-----------------|--------------|----------------|------|
| `0` | 1 | terminal (no radius) | scale-based | `--radius-none: 0` | new — needed because AC3 grep matches `0` |
| `2px` | 6 | below `sm` (4px), regular end step | scale-based | `--radius-xs: 2px` | **work item** ✓ |
| `3px` | 6 | between `xs` (2px) and `sm` (4px) | **use-case** | **DECISION NEEDED** (e.g. `--radius-control`) | unanticipated; spans clear-button, scrollbar thumb, option row, kbd chips |
| `6px` | 7 | between `sm` (4px) and `md` (8px) | use-case | `--radius-block: 6px` | **work item** ✓ — but see naming tension below |
| `8px` | 1 | equals `md` | scale-based (exists) | consume `--radius-md` | no new token |
| `12px` | 1 | equals `lg` | scale-based (exists) | consume `--radius-lg` | no new token + retire EXCEPTIONS row |
| `50%` | 3 | off the px scale (circle) | use-case/semantic | `--radius-full: 50%` | new; **not** `--radius-pill` |

**Two naming tensions to resolve in planning:**

1. **`3px` has no clean single use-case.** Its six sites are unrelated
   surfaces (a text button, a scrollbar thumb, a facet-option row, and
   three keyboard-hint chips). AC8 still mandates a use-case name because
   it's between-steps; the implementer must pick one and record a rationale.
   Candidate framings: `--radius-control` (small interactive controls),
   `--radius-chip-sm`, or similar. This is a genuine open decision the work
   item never anticipated.
2. **`--radius-block` is the *most common* radius in the app (7 sites) yet
   named after the code-block `<pre>`.** Only 2 of its 7 sites are code
   blocks (MarkdownRenderer); the rest are cards, panels, and a pipeline
   tile. Naming it `block` after a minority consumer is arguably misleading.
   Consider a more generic use-case name (`--radius-card`, `--radius-panel`)
   — but this contradicts the work item's stated `--radius-block` choice, so
   flag it for the author's sign-off rather than silently changing it.

### 3. `migration.test.ts` EXCEPTIONS cleanup (AC5) — the trickiest part

File: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`.
Every EXCEPTIONS entry is `kind: 'irreducible'`. **There is no single entry
captioned "In-between border radii: 6px"** (that wording is ADR-0026 §3, not
the harness). Instead the harness records radius literals across many
entries, most **mixed** with non-radius uses of the same literal.

#### 3a. Answer to 0090's open question — is the 2px badge an irreducible entry?

**Yes, but never as a standalone radius entry.** `2px` radius uses are
folded into *mixed* `(file, '2px')` rows alongside border/outline widths:
- `:90` RelatedArtifacts `'2px' x3` — "border-left widths **and badge
  border-radius**" (1 of 3 is the badge radius).
- `:107` Sidebar `'2px' x6` — includes "**mark border-radius**" + "loadbar …
  **radius**" (2 of 6 are radii).
- `:129` Breadcrumbs `'2px' x3` — "outline width/offset, **border-radius**"
  (1 of 3).
- `:222` FilterPill `'2px' x4` — "**checkbox radius** + …" (1 of 4).

So ADR-0026 §3 has **no `2px` radius row** (only the unrelated 1px/2px
border-*width* row), but the harness *does* admit the 2px radii inside mixed
width entries. Cleanup = decrement + reason-rewrite, not deletion.

#### 3b. Radius EXCEPTIONS entries and the action each needs

| line | entry | radius occ. | action |
|------|-------|-------------|--------|
| `:69` | MarkdownRenderer `'6px' x2` | 2 (both `<pre>`/wrapper radii) | **delete row** (pure radius) |
| `:71` | LifecycleClusterView `'6px' x1` | 1 (pipeline panel radius) | **decrement** → keep `:244` (spine x-coord, 6px, non-radius) |
| `:78` | Pipeline `'6px' x1` | 1 (tile radius) | **delete row** (pure radius) |
| `:90` | RelatedArtifacts `'2px' x3` | 1 (badge radius) | decrement 3→2, rewrite reason |
| `:107` | Sidebar `'2px' x6` | 2 (mark, loadbar) | decrement 6→4, rewrite reason |
| `:108` | Sidebar `'3px' x4` | 3 (kbd, hint-kbd, clear-button radii) | decrement 4→1, rewrite reason |
| `:109` | Sidebar `'4px' x4` | 1 (scrollbar thumb radius) | decrement 4→3, rewrite — **NOTE: 4px is not in our literal sweep** (see 3c) |
| `:111` | Sidebar `'6px' x8` | 1 (search panel radius) | decrement 8→7, rewrite reason |
| `:129` | Breadcrumbs `'2px' x3` | 1 (focus-ring radius) | decrement 3→2, rewrite reason |
| `:187` | LibraryOverviewHub `'6px' x3` | 1 (card radius) | decrement 3→2, rewrite reason |
| `:198` | EmptyState `'12px' x1` | 1 ("card border-radius … equals `--radius-lg` but co-located") | **delete row** (pure radius) |
| `:222` | FilterPill `'2px' x4` | 1 (checkbox radius) | decrement 4→3, rewrite reason |
| `:224` | FilterPill `'3px' x3` | ≥1 (option radius; clear-button likely) | decrement, rewrite — **reconcile against file** (see 3c) |
| `:231` | FilterPill `'8px' x8` | 1 (badge radius) | decrement 8→7, rewrite reason |
| `:251` | LifecycleIndex `'6px' x2` | 1 (card radius) | decrement 2→1, rewrite — keep toolbar-gap 6px |

**Explicitly NOT radius (leave untouched), to contrast:** `:236`
LifecycleClusterView `'1.5px'` "coloured ring **widths** — below
`--radius-sm`/`--sp-1` floor" (mentions `--radius-sm` only as a comparison
floor; it is a border width). Likewise all `'1px'` rows, `:88`/`:225`
`'1.5px'` stroke widths, and box-shadow/scrollbar `'3px'` rows at `:102`,
`:132`, `:165`.

The `0` reset at MarkdownRenderer:42 is **not** in EXCEPTIONS — `PX_REM_EM_RE`
auto-excludes `0`-resets — so no cleanup is needed there, but see §5 for why
the AC3 grep still flags it.

#### 3c. Reconciliation caveat (must verify during planning)

The EXCEPTIONS `count` is the **substring occurrence of the literal in the
whole file**, mixing radius and non-radius properties (e.g. `height: 3px`,
`box-shadow … 3px`). Two rows did not cleanly reconcile against the literal
sweep and must be re-read before final counts are set:
- **`:108` / `:224` FilterPill `3px`** vs the 3 FilterPill `border-radius:
  3px` sites (`:84`, `:160`, `:174`) — the entry reasons mention "checkmark
  height" (a non-radius 3px) so the exact radius-vs-other split needs the
  file open.
- **`:109` Sidebar `4px`** names a "scrollbar thumb radius", but no `4px`
  `border-radius` appears in the literal sweep — likely a
  `::-webkit-scrollbar-thumb` radius the AC3 pattern catches differently, or
  a stale reason. Verify.

**Recommended procedure:** migrate the CSS first, then run
`mise run test:unit:frontend`; the hygiene gate's `observed !== declared`
failure messages report the exact expected counts, making the final
decrement values self-correcting. Do not hand-compute blind.

#### 3d. Other harness mechanisms that bite

- **`var(--NAME)` resolution test** (`:361-393`, uses `RADIUS_TOKENS` at
  `:367`): any new `--radius-*` token **must** be added to `RADIUS_TOKENS`
  in `tokens.ts` or this fails with the new name in `unknown`.
- **AC5_FLOOR ratchet** (`:402`, `const AC5_FLOOR = 426`): the test at
  `:416-418` asserts `AC5_FLOOR <= observed var(--*) count`. Migrating ~25
  literals raises observed; **bump `AC5_FLOOR` in the same commit** (follow
  the in-file bump-protocol comment, e.g. `// 0090: 25 radius literals →
  var(--radius-*) (+N)`).
- **0038 pill test** (`:512-536`): regexes `border-radius:\s*var(--radius-pill)`
  against an allow-list (Chip, OriginPill, Sidebar, FilterPill,
  KanbanColumn, LifecycleIndex). New non-pill `--radius-*` tokens are
  invisible to it — **safe**. The hazard is only if you route a `50%` circle
  onto `--radius-pill` in a non-allow-listed file (PipelineMini,
  LibraryTemplatesIndex, LifecycleClusterView are **not** on the list). This
  is the concrete reason `50%` needs its own `--radius-full`, not
  `--radius-pill`.
- **Self-test fixture** at `:652` literally contains `'border-radius: 12px;'`
  to prove the *font-size* regex skips it — unaffected by radius work, but
  don't be surprised to see the string there.

### 4. Existing scale and the two canonical edit points

`src/styles/global.css:208-212`:
```
  /* Radius */
  --radius-sm:   4px;
  --radius-md:   8px;
  --radius-lg:   12px;
  --radius-pill: 999px;
```
`src/styles/tokens.ts:191-195` (the registry the harness + specs import):
```
export const RADIUS_TOKENS = {
  'radius-sm':   '4px',
  'radius-md':   '8px',
  'radius-lg':   '12px',
  'radius-pill': '999px',
} ... // RadiusToken type at :351
```
**Both must be edited in lockstep** for each new token, and AC7 requires a
comment above the `global.css` `--radius-*` block referencing the new ADR.
Proposed slotting by px ordering: `--radius-none: 0`, `--radius-xs: 2px`,
`--radius-<3px>: 3px`, `--radius-sm: 4px` (existing), `--radius-block: 6px`,
`--radius-md: 8px` (existing), `--radius-lg: 12px` (existing),
`--radius-pill: 999px` (existing), `--radius-full: 50%` (semantic, place at
end). 0033 defined the original four steps with **no documented per-step
rationale** and **no value between 4 and 8 or below 4** — which is precisely
why 2px/3px/6px were irreducible.

### 5. ADR work (AC6) — ADR-0026 §3 amendment + new ADR-0039

ADR-0026 §3 ("Irreducible literal categories"),
`meta/decisions/ADR-0026-css-design-token-application-conventions.md:114-129`,
table row to retire (`:127`):
```
| In-between border radii | `6px` | Between `--radius-sm` (4px) and `--radius-md` (8px) |
```
The adjacent `:121` row (`Border / outline widths | 1px, 2px | Below --sp-1
floor`) is **border widths — leave it.** §3's intro (`:116-117`) hard-codes
"these always land in EXCEPTIONS with `kind: 'irreducible'`", which the new
rule contradicts for radius.

**Amendment precedent (mirror exactly).** ADR-0026 was already amended twice:
- *Typography (0075/ADR-0036):* a blockquote partial-supersession note under
  the title (`:16-18`), inline "governed by ADR-0036" pointers (`:112`,
  `:129`), and frontmatter `superseded_by: "adr:ADR-0036"` (`:6`). The
  superseded §3 typography rows were **removed in place**, leaving a pointer.
- *Code-block (0076):* a Consequences bullet stating **"The §3 table row is
  removed"** (`:245-247`).

Either style satisfies AC6 ("deleted, or annotated as superseded … a grep
for `In-between border radii` returns no un-struck table row"). Recommended:
remove the `:127` row in place + add a blockquote/inline pointer to the new
ADR + add the reciprocal frontmatter `superseded_by`.

**New ADR — template and ID.** Model on ADR-0036
(`ADR-0036-typography-font-size-consumption-rule.md`), a tight ~139-line
single-rule ADR: Context / Decision Drivers / Considered Options / Decision
(with bolded-lead clauses incl. **Scope of supersession**, **Scale extension
policy**, **Escape valve**, **Why a separate array**) / Consequences /
References. Verbatim rule to adapt (ADR-0036:50-56):
> *every `font-size` declaration in current-app CSS … must resolve to a
> `var(--size-*)` token reference. No literal px, rem, or em … values are
> permitted … Off-grid values are handled by extending the scale rather than
> by tolerance-band substitution.*

Frontmatter linkage to mirror: new ADR gets `supersedes: ["adr:ADR-0026"]`;
ADR-0026 gets `superseded_by: "adr:ADR-00NN"`. Cite ADR-0030 (template),
ADR-0031 (immutability/supersession), ADR-0034 (typed linkage) in References;
keep work-item IDs out of the body (References only).

**ID: the next free number is `ADR-0039`.** (Highest on disk is ADR-0038;
the 0090 work item's guess of "ADR-0037" predates ADR-0037/0038 landing.)

### 6. Playwright regression spec (AC2)

Specs live in
`skills/visualisation/visualise/frontend/tests/visual-regression/`. The
closest template is **`typography-resolved-sizes.spec.ts`** (it reads a
computed numeric px value, not a colour). Pattern:
- A data-driven `CASES` array of `{ route, selector, expected, name,
  setup? }`; one `test()` loops it.
- `await page.goto(route)` (origin from `baseURL`, served by the real Rust
  visualiser binary against committed fixtures under
  `server/tests/fixtures/meta`); optional `setup(page)` to open menus first.
- `await page.locator(selector).first().evaluate(el =>
  getComputedStyle(el).borderTopLeftRadius)` — **read a corner longhand**,
  not the empty `borderRadius` shorthand (mirrors existing
  `borderTopColor` / `borderTopWidth` reads).
- `expect(value).toBe('Npx')` — exact string match.
- Pin the viewport with `test.use({ viewport: { width: 1280, height: 720 }})`
  (the typography spec does this so rem-derived values resolve at a 16px
  root). Default Chromium viewport is 1280×720; `playwright.config.ts` sets
  no global viewport. The spec **must** live under `tests/visual-regression/`
  to be picked up by the `visual-regression` project (`workers: 1`).

**Route → component mounting** (for the AC2 selector list):

| selector source | route to mount | note |
|-----------------|----------------|------|
| Breadcrumbs, Sidebar (`.searchMark`, `.searchLoadbar`, `.kbd`, `.searchHintKbd`, `.searchClear`, `.searchPanel`) | any route, e.g. `/library`; Sidebar search UI needs typing into the search box (a `setup` step) | Topbar/Sidebar mount on every page via RootLayout |
| MarkdownRenderer (`.markdown pre`, `.codeblock`, `.codeblock pre`) | `/library/plans/<slug>` doc-detail, or `/code-syntax-showcase` | easiest data route |
| RelatedArtifacts `.badge` | `/library/work-items/0099-ac2-coverage` (anchor fixture with an inferred cluster) | renders only with related/cluster siblings |
| FilterPill (`.badge`, `.clearButton`, `.option`, `.checkbox`, scrollbar thumb) | `/library/plans` (populated listing) | options/scrollbar render only after clicking the trigger → `setup` step |
| Pipeline `.tile` | `/lifecycle` (card variant) and `/lifecycle/<slug>` (panel variant) | data-dependent on lifecycle clusters |
| PipelineMini `.dot` | `/kanban` | one per work-item card |
| LifecycleIndex `.card`; LifecycleClusterView `.pipelinePanel`, `.stage::before` | `/lifecycle`, `/lifecycle/<slug>` | data-dependent |
| LibraryOverviewHub `.card` | `/library` | the `/library` index |
| LibraryTemplatesIndex `.tierPillBullet` | `/library/templates` | direct route |
| **EmptyState `.card` (12px)** | **none reachable** | 0074 added fixtures for every type; no empty listing exists. **AC2 cannot mount this via a route** — flag for a fixture or showcase, or rely on AC3 as its completeness backstop |

### 7. Where the gate lives (0090 Open Question) — precedent is "no shell grep"

**There is no standalone ripgrep CI step and no grep shell script in the
repo.** All literal-consumption enforcement is the single Vitest file
`src/styles/migration.test.ts`, run transitively by CI via
`mise run test → test:unit:frontend → npm run test → vitest run`
(`.github/workflows/main.yml:30-31`; `mise.toml:80-83,150-152`;
`tasks/test/unit.py:27-31`; `package.json:14`). There is **no lint task**,
only `typecheck` (`tsc --noEmit`), and it isn't wired into mise.

0075's three font-size ripgrep sweeps exist **only as comments**
(`migration.test.ts:543-558`) explicitly labelled "coarser approximations
used at the review-time grep gate; the regexes above are the authoritative
test." The authoritative guard is `FONT_SIZE_LITERAL_RE` + an AC4 vitest
suite. Border-radius is *already partially* enforced: the existing
`PX_REM_EM_RE` AC4 sweep (`:324-331`) flags any non-`var()` px/rem
`border-radius` as a side effect (which is why the 6px radii are carried as
EXCEPTIONS today).

**Implication for AC4.** 0090's AC4 says "the CI gate runs exactly AC3's
three sweeps." Two faithful options:
1. **Follow precedent (recommended):** add a new `describe` block (mirroring
   the `AC2 / 0075` font-size block at `:582-629`) with a
   `BORDER_RADIUS_LITERAL_RE` + the four-corner longhand alternation, record
   the three `rg` sweeps as documentation comments, and let vitest be the
   authoritative gate. No new file, no `mise.toml`/CI/shell change.
2. **Literal ripgrep in CI (a departure):** add an executed `rg` step to
   `main.yml` or a mise task. Nothing in the repo does this today; it would
   be net-new infrastructure.
This is a genuine decision for the plan, matching 0090's stated open
question. Note also that AC3's `[.0-9]` pattern **matches `0` and `50%`** —
so `border-radius: 0` (MarkdownRenderer:42) and the three `50%` circles fail
the sweep until tokenised; the in-test regex should align with whatever
exclusions are intended (the existing `PX_REM_EM_RE` deliberately excludes
`0`-resets, so the new radius regex must decide whether `0` is allowed bare
or must become `--radius-none`).

## Code References

- `skills/visualisation/visualise/frontend/src/styles/global.css:208-212` — existing `--radius-*` scale (edit point; AC1/AC7).
- `skills/visualisation/visualise/frontend/src/styles/tokens.ts:191-195,351` — `RADIUS_TOKENS` registry + `RadiusToken` type (must extend in lockstep).
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:48-264` — EXCEPTIONS array (radius rows at :69,71,78,90,107,108,109,111,129,187,198,222,224,231,251).
- `…/migration.test.ts:315-331` — AC3/AC4 literal sweeps (the de-facto gate).
- `…/migration.test.ts:361-393` — `var(--NAME)` resolution test (uses `RADIUS_TOKENS`).
- `…/migration.test.ts:395-427` — AC5_FLOOR ratchet (`AC5_FLOOR = 426` at :402).
- `…/migration.test.ts:439-470` — `observed === declared` hygiene gate (decrement enforcement).
- `…/migration.test.ts:512-536` — 0038 `--radius-pill` allow-list test.
- `…/migration.test.ts:543-558` — documented ripgrep sweeps (review-time only) + authoritative-regex note.
- The 12 component CSS files in the §1 inventory table (selectors + lines).
- `meta/decisions/ADR-0026-css-design-token-application-conventions.md:114-129` (§3; row to retire at :127; frontmatter :6; amendment precedents :16-18, :245-247).
- `meta/decisions/ADR-0036-typography-font-size-consumption-rule.md` — template for the new ADR (rule at :50-56; scope clause :59-62).
- `skills/visualisation/visualise/frontend/tests/visual-regression/typography-resolved-sizes.spec.ts` — closest spec template.
- `skills/visualisation/visualise/frontend/playwright.config.ts` — viewport/baseURL/webServer/project config.
- `skills/visualisation/visualise/frontend/src/router.ts` — route → component mapping.
- `.github/workflows/main.yml:30-31`, `mise.toml:80-83,150-152`, `tasks/test/unit.py:27-31` — CI/test wiring (no grep gate).

## Architecture Insights

- **Consume-tokens-everywhere is enforced by one Vitest harness, not CI
  grep.** `migration.test.ts` is a per-occurrence admission ledger:
  literals are allowed only if budgeted in EXCEPTIONS with an exact count,
  and a parallel "var(--*) count must stay ≥ floor" ratchet prevents
  backsliding. Any token migration is a four-touch change: CSS +
  `global.css` + `tokens.ts` (`RADIUS_TOKENS`) + `migration.test.ts`
  (decrement EXCEPTIONS + bump AC5_FLOOR).
- **EXCEPTIONS entries are keyed by (file, raw-literal-substring), not by
  property.** This is why radius cleanup is decrement-and-reword, not
  delete: a single `'2px'` row can cover a radius, two border widths, and a
  gap. The hygiene test's exact-count requirement makes the harness itself
  the oracle for the correct post-migration counts.
- **The scale has structural gaps by design** (nothing below 4px, nothing
  between 4 and 8), so "extend-and-preserve" necessarily adds below-`sm`
  and between-`sm`/`md` steps. The percentage case (`50%`) and the reset
  (`0`) sit *outside* a px ladder entirely and force semantic tokens.
- **Naming policy (AC8) collides with reality for `3px` and `6px`:** the
  between-steps values that AC8 forces into use-case names are exactly the
  ones used most broadly across unrelated surfaces, so a single use-case
  name is a compromise. This is inherent to extend-and-preserve over a
  sparse ladder.

## Historical Context

- `meta/work/0075-typography-size-scale-consumption.md` (done) — the direct
  pattern precedent. Its inventory ballooned 4→35 declarations across 9
  files yet stayed a **single atomic PR**; epic-split was a documented
  contingency only (triggered if the diff is "genuinely unreviewable" or
  >~1500 non-test lines). It carved radius out explicitly (Assumptions,
  Drafting Notes) and ordered "0090 must not begin until 0075 lands."
  Phasing was **migration-first, enforcement-last**: tokens + ADR + spec
  scaffold up front, per-file-group migrations in the middle, the failing
  vitest ban + final sweeps + PR description last (so the suite stays green
  throughout).
- `meta/plans/2026-05-23-0075-typography-size-scale-consumption.md` — the
  8-phase plan to mirror for structure.
- `meta/work/0033-design-token-system.md` (done) — defined the four-step
  `--radius-*` scale with no per-step rationale and a ±1px radius migration
  tolerance; collapsed a `9999px` pill literal to `999px`.
- `meta/decisions/ADR-0036` + the ADR-0026 amendment — the ADR pairing
  template (new scoped-supersession ADR; ADR-0026 stays `accepted` with
  textual supersession + reciprocal frontmatter linkage).
- `meta/work/0041-…` and `meta/work/0077-…` — weaker siblings than 0090
  implies: 0041 only *consumes* spacing tokens (no consume-everywhere rule,
  no gate, no EXCEPTIONS retirement); 0077 is an audit that mirrors the
  `*-resolved-*` `getComputedStyle` spec pattern and a consumer-enumeration
  grep, but deliberately created **no ADR and no gate** (do not mirror that
  posture — 0090 needs both).

## Related Research

- `meta/research/codebase/2026-05-23-0075-typography-size-scale-consumption.md`
  — the 0075 audit (typography), the structural sibling of this document.
- No prior 0090/radius research existed before this document (confirmed by
  glob); this file is the upstream deliverable 0090's Dependencies/
  Assumptions require before planning can begin.

## Open Questions

1. **`3px` use-case name.** AC8 forces a use-case name for the 6
   between-steps `3px` sites, but they span unrelated surfaces (button,
   scrollbar thumb, option row, kbd chips). Which name + rationale?
   (`--radius-control`? `--radius-chip-sm`?)
2. **`--radius-block` naming.** 5 of 7 `6px` sites are non-code-block
   cards/panels/tiles. Keep the work item's `--radius-block`, or pick a more
   representative use-case name? Needs author sign-off (the work item names
   `--radius-block` explicitly).
3. **`0` handling.** Tokenise `border-radius: 0` as `--radius-none: 0`, or
   exempt bare `0` in the gate regex (as `PX_REM_EM_RE` already does for
   `0`-resets)? Affects AC1/AC3 wording.
4. **`50%` token.** Confirm `--radius-full: 50%` as a percentage/semantic
   token (must not be `--radius-pill`); confirm AC2 records the *measured*
   computed px per dot rather than the percentage.
5. **AC4 gate mechanism.** New Vitest `describe` block (precedent) vs an
   executed ripgrep CI step (departure)? The work item's "CI gate runs
   exactly AC3's three sweeps" reads as ripgrep, but no such step exists in
   the repo today.
6. **EmptyState `.card` (12px) AC2 coverage.** Not mountable by navigation —
   add an empty fixture/showcase, or accept AC3 as its only backstop?
7. **EXCEPTIONS count reconciliation** for FilterPill `3px` (:108/:224) and
   Sidebar `4px` (:109) — verify exact radius-vs-other splits against the
   files (recommend driving final counts off the hygiene-test failures).
8. **Sizing.** 25 declarations / 12 files / ~5 new tokens is bounded; stays
   a single atomic PR per 0075 precedent unless the cumulative diff proves
   unreviewable — the epic-split contingency remains a documented fallback.
