---
date: "2026-05-15T10:49:11Z"
type: plan-review
producer: review-plan
target: "plan:2026-05-15-0038-generic-chip-component"
review_number: 1
verdict: COMMENT
lenses: [architecture, code-quality, test-coverage, correctness, standards, usability]
review_pass: 3
status: complete
id: "2026-05-15-0038-generic-chip-component-review-1"
title: "2026-05-15-0038-generic-chip-component-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-15T10:49:11Z"
last_updated_by: Toby Clemson
---

## Plan Review: 0038 — Generic Chip Component

**Verdict:** REVISE

The plan is structurally strong: phases are independently shippable, the test-first ordering is rigorous, and the design honours the canonical SseIndicator/OriginPill pattern (data-* variants, `?raw` CSS-source assertions, token-passive theming). However, four lenses independently identified a hard blocker — the proposed Chip `color-mix()` rules violate the locked-in convention enforced by `migration.test.ts`/ADR-0026 — and several other directives ship unresolved decisions ("decide at implementation time") that should be settled in the plan. Notable secondary themes include accessibility gaps in the FrontmatterChips restyle, under-specified kanban-subtitle insertion, status-mapping correctness issues (omits the observed `Approve w/ changes` string), and tautological "legacy class removed" tests that cannot fail against CSS-module hashed class names.

### Cross-Cutting Themes

- **`color-mix()` convention violation** (flagged by: code-quality, standards, correctness, test-coverage) — the Chip CSS uses `color-mix(in srgb, var(--ac-X) 10%, transparent)` (and 22%, 25%) but `migration.test.ts:233-249` locks the convention to percentages `{8, 18, 30}` blended against `var(--ac-bg)`. This is the single most consequential issue; it makes Phase 2's stated automated-verification step (`npm test src/styles/migration.test.ts`) unachievable as written.
- **Deferred-to-implementation decisions** (flagged by: architecture, code-quality, correctness, standards) — `--size-xxs` (12px) vs prototype 10.5px, off-scale paddings (2/3/10px), migration-ledger exception choices. Each of these is a phase-spanning architecture/standards decision that should resolve in the plan, not at PR review.
- **Kanban subtitle under-specified** (flagged by: architecture, correctness, standards, usability) — Phase 5e leaves target file unidentified, doesn't specify which of the three KanbanBoard render branches gets the chip, and introduces a new visible UI element (the "live" chip) that may warrant an ADR or work-item update.
- **Accessibility regression in FrontmatterChips** (flagged by: test-coverage, standards, usability) — replacing `<dl>/<dt>/<dd>` with flat chips loses key context for screen readers; Chip exposes no `aria-label` hook; no tests pin the accessible-name story.
- **Status mapping correctness and DX** (flagged by: code-quality, correctness, usability) — silent fallback to neutral masks typos and new statuses; the observed `Approve w/ changes` string (with literal `/`) is not stripped by the current normaliser and maps to neutral rather than amber; INDIGO/AMBER buckets conflate `reviewed`/`revised` without documented semantic axis.
- **Variant scope `red`/`violet`** (flagged by: architecture, usability) — two variants ship with no in-scope consumer and no prescriptive mapping, pressuring future consumers into subjective colour choices.
- **Showcase cell count inconsistency** (flagged by: code-quality, test-coverage) — Desired End State says 36 cells, Phase 6 says 24 (6 × 2 × 2). Internal inconsistency suggests scope churn.

### Tradeoff Analysis

- **Prototype fidelity vs token-discipline convention**: matching the prototype's verbatim `rgba()` math requires 10%/22%/25% over `transparent`, but the codebase's locked convention is `{8, 18, 30}` over `var(--ac-bg)`. Recommendation: snap to the convention ladder and accept a minor visual delta, *unless* the plan explicitly proposes (and justifies) widening the convention in `migration.test.ts` as part of Phase 1.
- **Semantic HTML vs prototype layout**: `<dl>` keyed pairs are accessible but key-less chips match the prototype's compact density. Recommendation: keep the visual restyle but compensate with per-chip `aria-label={`${key}: ${value}`}` so screen-reader semantics survive.
- **API minimalism vs DX ergonomics**: requiring `variant` keeps Chip a thin presentational primitive; adding a `status` convenience prop reduces consumer boilerplate but couples Chip to the status vocabulary. Recommendation: keep `variant` required for Chip itself but consider a `<StatusChip status={…}>` thin wrapper if the boilerplate proves heavy at consumer sites.

### Findings

#### Critical

- 🔴 **Code Quality**: Proposed `color-mix()` rules violate the locked-in convention enforced by `migration.test.ts`
  **Location**: Phase 2 §3 (Chip.module.css variant rules)
  The plan uses `color-mix(in srgb, var(--ac-ok) 8%, transparent)` plus 10%/22%/25%, but the convention regex requires `var(--ac-bg)` as the second argument and percentages in `{8, 18, 30}`. Phase 2's success criterion is unachievable as written.

- 🔴 **Correctness**: `--size-xxs` is 12px, not the prototype's 10.5px
  **Location**: Phase 2 §3 (Chip.module.css font-size)
  Binding `font-size: var(--size-xxs)` produces a chip ~14% larger than the prototype intended. The plan flags this as "verify" but proceeds anyway; the resolved-colour spec in Phase 6 will subtly fail.

- 🔴 **Standards**: New token hex values use uppercase, breaking the file-wide lowercase convention
  **Location**: Phase 1 §1 (DARK_COLOR_TOKENS additions in `tokens.ts` and `global.css`)
  Every existing hex in `tokens.ts`/`global.css` is lowercase; the plan adds `#79D9A6`/`#E4B76E`/`#E86A6B` (uppercase). Parity tests case-fold so they pass, but the source diverges visibly.

#### Major

- 🟡 **Architecture**: Domain status vocabulary co-located with the presentational primitive
  **Location**: Phase 3 (`src/components/Chip/statusVariant.ts`)
  `statusToChipVariant` is a domain function consumed by routes — placing it in the Chip folder couples future non-chip consumers to a presentational component's folder.

- 🟡 **Architecture**: Broadening `--ac-ok/--ac-warn/--ac-err` semantics without regression guards
  **Location**: Phase 1
  Existing consumers (OriginPill pulse, malformed banner, SseIndicator) will visibly change in dark mode; the plan only specifies manual eyeball verification.

- 🟡 **Architecture**: Two variants (`red`, `violet`) have no in-scope consumer
  **Location**: Phase 2 (variant set vs observed consumer set)
  CSS rules, showcase cells, and visual-regression snapshots are paid for surface area that no surface emits today.

- 🟡 **Code Quality / Correctness / Standards**: Off-scale rem and font-size decisions deferred to implementation
  **Location**: Phase 2 §3
  `--size-xxs` choice, `0.125rem`/`0.1875rem`/`0.625rem` padding admittance, and migration-ledger exceptions are left open. These rebound into multiple phases (tokens, ledger, visual baselines) and break Phase 2's "self-contained commit" guarantee.

- 🟡 **Code Quality**: `STATUS_KEYS` detection diverges from `statusToChipVariant` normalisation
  **Location**: Phase 4 §2 (FrontmatterChips rewrite)
  `STATUS_KEYS = new Set(['status'])` does strict-equality on the raw key; a `Status:` (capital S) bypasses colour-coding silently while the value mapper handles case normalisation.

- 🟡 **Test Coverage**: `querySelector('.statusBadge')` cannot match CSS-module class names
  **Location**: Phase 5a/5b/5c/5d "does not render the legacy class" assertions
  CSS Modules hash class names — the literal selector never matches before or after migration. These assertions are tautologies and provide no migration safety net.

- 🟡 **Test Coverage**: Cell count inconsistency (24 vs 36) and unspecified theme-cell construction
  **Location**: Desired End State §8 vs Phase 6
  Desired End State says 36 cells; Phase 6 says 24. The plan also doesn't decide between per-cell `[data-theme]` overrides on one page vs Glyph-style theme toggle across two passes.

- 🟡 **Test Coverage**: Resolved-colour Playwright spec doesn't specify hex→rgba/color-mix conversion
  **Location**: Phase 6 `chip-resolved-colours.spec.ts`
  `getComputedStyle` returns normalised `rgb(...)`/`rgba(...)` strings, not hex. The plan documents hex foregrounds and rgba backgrounds in the same table without a conversion helper.

- 🟡 **Test Coverage**: Phase 5 test snippets are stubs with `// ...` placeholders
  **Location**: Phase 5 sub-phases
  Fixture mutations aren't specified; the kanban-subtitle test even concedes the target file is unknown. TDD value is lost when tests are written from scratch at implementation time.

- 🟡 **Test Coverage / Standards / Usability**: No accessibility name strategy for chips
  **Location**: Phase 2 / Phase 4
  Chip has no `aria-label` prop; FrontmatterChips drops `<dl>/<dt>` semantics with no aria compensation. WCAG 1.4.1 concern (colour-only status signal).

- 🟡 **Correctness**: `statusToChipVariant` table includes speculative inclusions and omits the observed string
  **Location**: Phase 3
  `complete`/`shipped`/`final`/`abandoned`/`superseded` are not in the Current State observed-status table but are mapped. Meanwhile the observed string `Approve w/ changes` (literal `/`) is not stripped by the current normaliser and maps to neutral, not amber.

- 🟡 **Correctness**: Banner text silently changed
  **Location**: Phase 4 §2
  Current text: "Frontmatter unparseable — showing raw content." Plan rewrites to: "Frontmatter is malformed and could not be parsed." Not flagged as a copy change.

- 🟡 **Correctness / Architecture / Standards / Usability**: Kanban subtitle insertion point is under-specified
  **Location**: Phase 5e
  No existing page-subtitle component; `KanbanBoard.tsx` has three render branches each with their own `<h1>Kanban</h1>`. The plan leaves placement, branch selection, and whether to extract `PageSubtitle` to implementation time.

- 🟡 **Standards**: `statusVariant.ts` location and naming break the component-folder convention
  **Location**: Phase 3
  Component folders contain `Foo.tsx`/`Foo.module.css`/`Foo.test.tsx` exclusively, in PascalCase. A `statusVariant.ts` (camelCase) inside `Chip/` breaks both.

- 🟡 **Standards**: Introducing the kanban "live" chip may warrant ADR coverage or a work-item amendment
  **Location**: Phase 5e
  Work item 0038 scopes itself to *replacing* open-coded pills; the kanban "live" chip is an *introduction*, not in AC1–AC4. Open Question 2 from research ("does live nest .ac-pulse?") is closed by fiat.

- 🟡 **Standards**: ChipSize rename (`sm | default` → `sm | md`) deviates from work item without explicit migration note
  **Location**: Phase 2 (Chip.tsx type)
  Work item AC2 specifies `sm` (default). The plan flips to `sm | md` with `sm` as default. Plausible, but should be reflected in the work item or in Migration Notes.

- 🟡 **Usability**: `variant` required forces repeated boilerplate at every status call site
  **Location**: Phase 2 (ChipProps)
  Four+ call sites repeat `<Chip variant={statusToChipVariant(status)}>{status}</Chip>`. A `status` convenience prop (or a thin `StatusChip` wrapper) would collapse the boilerplate.

- 🟡 **Usability**: Silent fallback to neutral hides typos and new statuses
  **Location**: Phase 3
  Indistinguishable from "this isn't a status" and "this is a misspelled status." No dev-only warning channel; visual diff can't catch the regression.

#### Minor

- 🔵 **Architecture**: Missing explicit Consumer Contract section (Glyph plan parity)
  **Location**: Plan-wide
  The Glyph plan's Consumer Contract section codifies downstream invariants; Chip has five consumers and inherits none.

- 🔵 **Architecture / Code Quality**: `STATUS_KEYS` as a single-element Set is awkward
  **Location**: Phase 4
  Either inline `key === 'status'`, or lift the allow-list next to `statusToChipVariant`.

- 🔵 **Architecture**: Phase 5 vs Phase 7 ambiguity on CSS deletion responsibility
  **Location**: Phase 5 / Phase 7
  Same deletions are scheduled in both phases. Pick one.

- 🔵 **Code Quality**: Boolean / empty-string formatChipValue behaviour untested
  **Location**: Phase 4
  `String(false)` → `'false'`, `String('')` → empty chip. Pin behaviour with tests.

- 🔵 **Code Quality**: Status mapping conflates `reviewed`/`revised` without comments
  **Location**: Phase 3
  Annotate semantic axis above each Set.

- 🔵 **Code Quality**: `leading` slot is YAGNI without in-scope consumer
  **Location**: Phase 2
  Defer until OriginPill lift.

- 🔵 **Code Quality**: LibraryTypeView passes variant from `frontmatter.status` but label from `statusCellValue` — hidden coupling
  **Location**: Phase 5b
  Extract `chipForEntry(entry)` returning `{ label, variant }`.

- 🔵 **Test Coverage**: Array/object values not tested against the mapper
  **Location**: Phase 3
  Add `expect(statusToChipVariant(['accepted'])).toBe('neutral')`.

- 🔵 **Test Coverage**: `leading={null}` / `leading={false}` edge cases not tested
  **Location**: Phase 2
  Tighten the guard and add cases.

- 🔵 **Test Coverage**: Single-quote regex coupling is brittle
  **Location**: Phase 2 CSS source assertions
  Use `['"]` character class.

- 🔵 **Test Coverage**: No assertion that `--ac-violet` remains theme-invariant
  **Location**: Phase 1
  Add a focused parity assertion.

- 🔵 **Test Coverage**: Repo-wide pill guard uses `glob.sync` instead of vitest `import.meta.glob`
  **Location**: Phase 7
  Mirror the existing migration-harness pattern.

- 🔵 **Test Coverage**: No darwin/linux Playwright baseline guidance
  **Location**: Phase 6
  Borrow the Glyph plan's Docker recipe.

- 🔵 **Correctness**: Per-Chip "no hardcoded hex" assertion duplicates migration guard
  **Location**: Phase 2 test file
  Rely on the global guard or align regexes.

- 🔵 **Correctness**: Boolean/numeric frontmatter status — text renders but variant goes neutral
  **Location**: Phase 4
  Document contract; test the divergence.

- 🔵 **Correctness**: Empty status produces a neutral chip containing just `—`
  **Location**: Phase 5b
  Skip chip rendering when value is empty.

- 🔵 **Standards**: `0.625rem` (10px) padding sits inside `--sp-3` ±2px tolerance
  **Location**: Phase 2
  Substitute `var(--sp-3)` per ADR-0026.

- 🔵 **Standards**: `data-chip-leading` attribute convention is one-off
  **Location**: Phase 2
  Use `data-slot="leading"` or class-based detection.

- 🔵 **Standards**: FrontmatterChips loses `<dl>/<dt>/<dd>` semantics
  **Location**: Phase 4
  Compensate with `aria-label` per chip.

- 🔵 **Standards**: Phase 7's pill-radius allow-list duplicates the `EXCEPTIONS` mechanism
  **Location**: Phase 7
  Extend EXCEPTIONS or split into a new test file.

- 🔵 **Standards**: Normalisation contract for `statusToChipVariant` Sets isn't documented
  **Location**: Phase 3
  JSDoc the separator-stripping rule; assert Set keys are separator-free.

- 🔵 **Usability**: `leading` prop name is non-discoverable
  **Location**: Phase 2
  Consider `iconBefore`/`startAdornment` or add JSDoc.

- 🔵 **Usability**: Six variants without prescriptive semantics
  **Location**: Phase 2 / Phase 3
  Ship four observed variants; add red/violet when a consumer arrives.

- 🔵 **Usability**: `size='sm' | 'md'` reads counterintuitively
  **Location**: Phase 2
  Add a JSDoc comment naming `sm` as the prototype baseline.

#### Suggestions

- 🔵 **Usability**: `statusToChipVariant(value: unknown)` is too permissive
  **Location**: Phase 3
  Narrow to `string | null | undefined`; let consumers coerce explicitly.

- 🔵 **Code Quality**: Reconcile the 24 vs 36 cell count
  **Location**: Phase 6 / Desired End State §8
  Pick one and update both references.

### Strengths

- ✅ Phase ordering correctly respects dependency direction: tokens → primitive → utility → first consumer → migrations → showcase → cleanup; each phase leaves the build green.
- ✅ Chip is theme-passive — binds to CSS custom properties, lets the `[data-theme="dark"]` cascade reach it without React-level theme coupling. Matches canonical SseIndicator/OriginPill.
- ✅ Variant selection via `data-variant`/`data-size` (not className concatenation) is consistent with the established house style.
- ✅ CSS-source `?raw` regex assertions are present for every variant→token binding, locking the architectural contract against drift.
- ✅ `statusToChipVariant` is implemented as a single pure function with explicit normalisation; unit tests cover case/whitespace/separator variations and `null`/`undefined`/`number` edges.
- ✅ Explicit "What We're NOT Doing" section is well-scoped (OriginPill stays, WorkItemCard out of scope, no parallel `--ac-status-*` namespace).
- ✅ Phase 7's repo-wide pill-radius guard with an explicit allow-list turns a one-off cleanup into a durable invariant.
- ✅ Phase 6's resolved-colour Playwright spec catches token drift that pixel diff alone would tolerate (good defence-in-depth design).
- ✅ Status mapping is correctly centralised, preventing the per-surface drift the work item warned about.

### Recommended Changes

Ordered by impact:

1. **Resolve the `color-mix()` convention conflict in Phase 1 or Phase 2** (addresses: critical color-mix finding from code-quality/standards/correctness/test-coverage)
   Two options: (a) snap variant tints to the `{8, 18, 30}` ladder composed against `var(--ac-bg)` (cleanest, accept minor visual delta from prototype), or (b) extend `migration.test.ts`'s `ALLOWED_PERCENTAGES` and surface allow-list as a planned change in Phase 1 with documented rationale. Pick one in the plan and update Phase 2's CSS skeleton and `?raw` regex assertions accordingly.

2. **Commit on the chip font-size in Phase 1 or Phase 2** (addresses: critical `--size-xxs` finding; standards/code-quality deferred-decision findings)
   Either accept `--size-xxs` (12px) as the chip size and update the resolved-fill table, or add literal `10.5px` with an EXCEPTIONS ledger entry following the Sidebar/ActivityFeed precedent, or introduce a `--size-chip: 10.5px` token in Phase 1. Don't ship "decide at implementation time."

3. **Lowercase the new dark-theme hex values** (addresses: critical hex-casing finding)
   Write `#79d9a6`/`#e4b76e`/`#e86a6b` to match the existing convention in `tokens.ts` and `global.css`.

4. **Fix tautological CSS-module class assertions in Phase 5** (addresses: major test-coverage finding)
   Replace `querySelector('.statusBadge')` with raw-CSS assertions (`expect(css).not.toMatch(/\.statusBadge\b/)`) or remove them entirely — the positive `[data-variant]` assertions are sufficient.

5. **Add accessibility hooks to Chip and aria-label per chip in FrontmatterChips** (addresses: a11y theme across test-coverage/standards/usability)
   Extend `ChipProps` with an optional `aria-label`; in FrontmatterChips, pass `aria-label={`${key}: ${value}`}` per chip to compensate for the lost `<dl>/<dt>` semantics. Add a test pinning this contract.

6. **Resolve Phase 5e kanban-subtitle insertion concretely** (addresses: kanban-subtitle theme across architecture/correctness/standards/usability)
   Name the file, the render branch, and whether a `PageSubtitle` extraction is in scope. If "live" is a new design element, either capture an ADR or amend work item 0038's AC list.

7. **Trim `statusToChipVariant` to observed mappings and fix the `/` regex** (addresses: correctness omissions/inclusions and `Approve w/ changes` regex bug)
   Either trim INDIGO/AMBER/RED to only the strings in the Current State observed table, or cite the source for each speculative inclusion. Extend the normaliser to strip `/` so `'Approve w/ changes'` reaches the AMBER bucket; add a test case for the literal observed string.

8. **Relocate `statusVariant.ts` outside `src/components/Chip/`** (addresses: architecture coupling + standards folder-convention findings)
   Move to `src/utils/status-variant.ts` or `src/api/status-variant.ts`; rename to kebab-case if mirroring `src/api/` style.

9. **Drop `violet` and reconsider `red`** (addresses: architecture variant scope finding; usability prescriptive-semantics finding)
   No in-scope consumer for either. Defer until a concrete need arrives.

10. **Reconcile the cell count to 24 throughout and decide the showcase theme strategy** (addresses: test-coverage and code-quality consistency findings)
    Update Desired End State §8 from 36 to 24. Decide between per-cell `[data-theme]` overrides on one page and a two-pass theme toggle (the latter matches Glyph plan parity).

11. **Spell out Phase 5 fixtures and test setup** (addresses: stubbed-snippets finding)
    For lifecycle, library, templates, and kanban, name the fixture extension and the exact assertion shape; resolve the kanban target file before Phase 5e.

12. **Decide CSS-deletion responsibility — Phase 5 per-surface or Phase 7 sweep** (addresses: architecture finding on Phase 5/7 overlap)
    Recommend Phase 5 per-surface (consistent with "each commit shippable"); leave Phase 7 for the migration-ledger assertion and grep guard only.

13. **Resolve `--size-xxs` precedent for `--ac-violet` invariance** (addresses: correctness `--ac-violet` finding)
    Either declare `ac-violet` in `DARK_COLOR_TOKENS` (locking the parity test) or add a focused assertion that `DARK_COLOR_TOKENS['ac-violet']` is undefined.

14. **Add a Consumer Contract section** (addresses: minor architecture finding)
    Mirror the Glyph plan: enumerate downstream invariants (no `className` override, no block-level children, status routing through the mapper, `leading` slot sizing).

15. **Preserve the malformed banner text** (addresses: correctness banner-text finding)
    Keep the existing copy verbatim or call out the change as a deliberate scope item.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: Structurally sound and mirrors the Glyph plan's TDD-first, token-driven, data-attribute variant pattern. Main concerns: coupling between presentational primitive and domain vocabulary, absence of an explicit consumer contract, broadening three semantic feedback tokens system-wide as a side-effect of one component's needs, and deferred-to-implementation decisions.

**Strengths**: Phase independence; theme-passive Chip; data-* variant selectors; CSS-source `?raw` assertions; deferred-leading slot honours open-closed; explicit "What We're NOT Doing" section.

**Findings**:
- 🟡 major/high — Domain status vocabulary co-located with presentational primitive (Phase 3 file placement)
- 🟡 major/medium — Broadening `--ac-ok/--ac-warn/--ac-err` for one component changes system-wide semantics (Phase 1)
- 🟡 major/medium — Two variants (`red`, `violet`) have no in-scope consumer (Phase 2)
- 🔵 minor/high — Missing explicit consumer contract section (Plan-wide)
- 🔵 minor/high — `STATUS_KEYS` as single-element Set (Phase 4)
- 🔵 minor/high — Phase 5e leaves the target file unidentified (Phase 5e)
- 🔵 minor/medium — Deferred decisions on off-scale rem and `--size-xxs` weaken phase isolation (Phase 2)
- 🔵 minor/medium — CSS deletion responsibility split across Phase 5 and Phase 7

### Code Quality

**Summary**: Well-structured, test-first, follows canonical patterns. Core design choices are pragmatic. However, proposed CSS variant rules violate the locked-in `color-mix` convention enforced by `migration.test.ts`, and several smaller code-smell and DRY concerns will create implementation friction.

**Strengths**: TDD with CSS-source assertions; single pure mapping function; data-* attribute variants; explicit "NOT Doing" section; Phase 7 repo-wide pill guard.

**Findings**:
- 🔴 critical/high — Proposed `color-mix()` rules violate locked-in convention (Phase 2 §3)
- 🟡 major/high — Open-ended `--size-xxs` / off-scale padding decision deferred (Phase 2 §3)
- 🟡 major/high — Status-key detection diverges from `statusToChipVariant`'s normalisation (Phase 4 §2)
- 🔵 minor/high — Boolean and falsy formatChipValue handling silently changed (Phase 4)
- 🔵 minor/medium — INDIGO/AMBER sets conflate similar concepts (Phase 3)
- 🔵 minor/medium — `leading` slot is YAGNI without in-scope consumer (Phase 2)
- 🔵 minor/medium — Variant selection and label text derived from different sources (Phase 5b)
- 🔵 suggestion/medium — 24 vs 36 cell count is inconsistent (Phase 6)

### Test Coverage

**Summary**: Unusually thorough on testing strategy. Several concrete gaps and contradictions risk leaving the plan green-on-paper: cell count is inconsistent, resolved-colour Playwright assertions need explicit rgba/color-mix handling, Phase 5 test snippets are stubs, and a key assertion cannot fail against CSS-module class names. Accessibility and array-status edge cases are also untested.

**Strengths**: Test-first sequencing; CSS-source `?raw` assertions for every variant; "no hardcoded hex" backstop; comprehensive Phase 3 status-mapping coverage; Phase 4 covers malformed/absent/array states; Phase 6 resolved-fill spec; Phase 7 migration guard.

**Findings**:
- 🟡 major/high — `querySelector('.statusBadge')` cannot match CSS-module class names (Phase 5a/b/c/d)
- 🟡 major/high — Cell count inconsistency (24 vs 36) and unspecified theme-cell construction (Phase 6)
- 🟡 major/high — Resolved-colour assertion logic against rgba/color-mix not specified (Phase 6)
- 🟡 major/high — Chip CSS will fail the existing `color-mix()` convention test (Phase 2)
- 🟡 major/medium — Phase 5 test snippets are stubs with `// ...` placeholders (Phase 5)
- 🟡 major/medium — No tests assert an accessible-name strategy for colour-coded status chips (Phase 2/4)
- 🔵 minor/high — Array and object frontmatter values not tested against the mapper (Phase 3)
- 🔵 minor/high — `leading={null}` and `leading={false}` edge cases not tested (Phase 2)
- 🔵 minor/medium — Single-vs-double-quote regex coupling is brittle (Phase 2)
- 🔵 minor/medium — No assertion that `--ac-violet` remains theme-invariant (Phase 1)
- 🔵 minor/medium — Phase 7 pill guard uses `glob.sync` instead of `import.meta.glob` (Phase 7)
- 🔵 minor/medium — No baseline-generation guidance for darwin vs linux (Phase 6)

### Correctness

**Summary**: Core component design is logically sound and well-scoped, but several concrete implementation directives produce code that does not pass the codebase's existing token-discipline tests. CSS color-mix compositions violate the strict convention, font-size token does not equal the prototype's 10.5px, and the status-mapping omits observed status strings. Subtler gaps in test claims, banner-text behaviour, and the under-specified kanban subtitle insertion point.

**Strengths**: Phase ordering places tokens before consumers; `statusToChipVariant` has solid normalisation; theme cascade reasoning is sound; `LibraryTypeView` fallback handling is correct; AC3 diagnosis is accurate.

**Findings**:
- 🔴 critical/high — Chip `color-mix()` calls violate the locked convention (Phase 2 §3)
- 🔴 critical/high — `--size-xxs` is 12px not 10.5px (Phase 2 §3)
- 🟡 major/high — `statusToChipVariant` omits `draft`/`todo` from observed inputs; speculative inclusions (Phase 3)
- 🟡 major/high — Malformed-banner text silently changed (Phase 4 §2)
- 🟡 major/medium — `--ac-violet` parity asymmetry not pinned by tests (Phase 1)
- 🟡 major/medium — Kanban subtitle insertion point under-specified (Phase 5e)
- 🔵 minor/high — "No hardcoded hex" regex duplicates global migration guard (Phase 2)
- 🔵 minor/high — `normalise` collapses `'approve-with-changes'` but not `'approve w/ changes'` — observed string maps to neutral (Phase 3)
- 🔵 minor/medium — Boolean/numeric frontmatter status renders text but variant goes neutral (Phase 4)
- 🔵 minor/medium — Empty status produces a bordered `—` chip the prototype does not show (Phase 5b)

### Standards

**Summary**: Follows the core house style — folder layout, data-* variants, `?raw` assertions, named exports, no `index.ts`. Violates the locked color-mix convention from ADR-0026, proposes hardcoded uppercase hex literals breaking the lowercase convention, and introduces a new kanban "live" chip without flagging ADR coverage. Smaller naming/file-placement choices drift (`statusVariant.ts` location, ChipSize rename).

**Strengths**: Canonical component-folder convention; data-* attribute pattern; `?raw` regex assertions; respects MIRROR-A/MIRROR-B invariant; centralised status mapping; explicit ADR-0026 reference.

**Findings**:
- 🔴 critical/high — `color-mix()` usage violates ADR-0026 locked-in convention (Phase 2 §3)
- 🔴 critical/high — New token hex values use uppercase (Phase 1 §1)
- 🟡 major/high — Off-scale padding values bypass ±2px tolerance rules (Phase 2 §3)
- 🟡 major/high — `data-chip-leading` attribute-naming convention is a one-off (Phase 2 §3)
- 🟡 major/high — Co-locating `statusVariant.ts` inside the Chip folder breaks folder-as-component convention (Phase 3)
- 🟡 major/medium — Kanban "live" chip introduction may warrant ADR (Phase 5e)
- 🟡 major/high — ChipSize rename from `sm | default` to `sm | md` not flagged in Migration Notes (Phase 2)
- 🔵 minor/high — `--size-xxs` is 12px not 10.5px — silent visual drift (Phase 2 §3)
- 🔵 minor/medium — Switching `<dl>` → `<span>` loses semantic frontmatter pairing (Phase 4)
- 🔵 minor/medium — Pill-radius allow-list duplicates existing migration-ledger structure (Phase 7)
- 🔵 minor/high — INDIGO set normalisation contract not documented (Phase 3)

### Usability

**Summary**: Well-structured primitive aligned with house style. Several DX choices add friction or create silent-failure traps: required `variant` prop even when consumers derive it from status, silent `statusToChipVariant` fallback hides typos, Chip exposes no accessibility hooks, and the FrontmatterChips restyle abandons semantic `<dl>` with no aria compensation. Six variants without prescriptive mappings pressure consumers into subjective choices.

**Strengths**: Variant selection consistency with SseIndicator; centralised status mapping; resolution of `sm`/`md` ambiguity; explicit migration ledger touchpoints; incremental phase commits.

**Findings**:
- 🟡 major/high — `variant` required even when most consumers derive from status (Phase 2)
- 🟡 major/high — Silent fallback to neutral hides typos and new statuses (Phase 3)
- 🟡 major/high — Chip exposes no a11y hooks; status chips don't announce as status (Phase 2)
- 🟡 major/medium — Dropping key labels loses "what is this value?" affordance (Phase 4)
- 🔵 minor/high — `leading` prop name is non-discoverable; no a11y guidance (Phase 2)
- 🔵 minor/medium — Six variants pressure consumers into subjective colour choices (Phase 2/3)
- 🔵 minor/medium — `size='sm'` default is correct but undocumented in the type (Phase 2)
- 🔵 minor/high — Vague kanban file location forces consumer to spelunk (Phase 5e)
- 🔵 suggestion/medium — Mapper accepts `unknown`, encouraging unsafe calls (Phase 3)

## Re-Review (Pass 2) — 2026-05-15T10:29:50Z

**Verdict:** REVISE

The edits substantively resolve every prior critical and almost every prior major finding from pass 1 (color-mix convention, `--size-chip` token, lowercase hex, CSS-module tautology, cell count, a11y hooks, kanban subtitle via PageSubtitle extraction, status-variant relocation, normaliser `/` fix, banner-text preservation, EXCEPTIONS ledger entries, Consumer Contract section, Phase 5/7 deletion separation, etc.). The plan is dramatically tighter and now passes the architectural and standards bars it failed on pass 1.

However, the edits introduced three new majors plus one high-confidence minor that is effectively a Phase 2 verification blocker, so the configured REVISE threshold (≥3 majors) still applies. All four are small, self-contained fixes — addressing them should be quick and the resulting plan should clear without further revision.

### Previously Identified Issues

#### Critical (all resolved)

- ✅ 🔴 **Code Quality / Standards / Correctness**: `color-mix()` violated the locked convention — **Resolved**. Chip CSS now uses `color-mix(in srgb, var(--ac-X) {8|30}%, var(--ac-bg))` per ADR-0026; tests assert it.
- ✅ 🔴 **Correctness / Standards**: `--size-xxs` was 12px not 10.5px — **Resolved**. New `--size-chip` (10.5px) / `--size-chip-md` (11.5px) tokens added in Phase 1; Chip binds to them.
- ✅ 🔴 **Standards**: Uppercase hex in new tokens — **Resolved**. All new hex literals lowercase.

#### Major (mostly resolved)

- ✅ 🟡 **Architecture**: `statusToChipVariant` co-located with Chip — **Resolved**. Moved to `src/api/status-variant.ts` with kebab-case naming.
- ✅ 🟡 **Architecture**: red/violet variants with no consumer — **Resolved by deliberate scope decision**. Documented in Migration Notes as a user-directed ship-all-six choice.
- 🟡 **Architecture**: Broadening `--ac-ok/--ac-warn/--ac-err` semantics without regression guards — **Partially resolved**. Documented as expected visual delta; still only manual checklist verification at consumer sites (OriginPill, SseIndicator, banner).
- ✅ 🟡 **Code Quality**: Open-ended `--size-xxs` / off-scale padding deferred — **Resolved**. Three EXCEPTIONS entries documented; new `--size-chip` token absorbs the font-size question.
- ✅ 🟡 **Code Quality**: `STATUS_KEYS` divergence — **Resolved**. New `isStatusKey()` helper normalises case.
- ✅ 🟡 **Test Coverage**: `querySelector('.statusBadge')` tautology — **Resolved**. All Phase 5 sub-phases now use raw-CSS source checks.
- ✅ 🟡 **Test Coverage**: Cell count 24 vs 36 — **Resolved**. Reconciled to 12 cells × 2 themes = 24 baselines per platform.
- ✅ 🟡 **Test Coverage**: Resolved-colour rgba/color-mix handling — **Partially resolved**. `rgbFromHex` helper added; "tinted-between-bounds" approach described — see new major below.
- ✅ 🟡 **Test Coverage**: Phase 5 test stubs — **Resolved**. Fixtures and assertions are now concrete.
- ✅ 🟡 **Test Coverage / Standards / Usability**: No a11y name strategy — **Resolved**. Chip exposes `aria-label`; FrontmatterChips uses `${key}: ${value}`; tests pin both.
- ✅ 🟡 **Correctness**: Banner text silently changed — **Resolved**. Verbatim original text preserved with a regression test.
- ✅ 🟡 **Correctness**: `--ac-violet` parity asymmetry — **Resolved**. Focused theme-invariance assertion added in `global.test.ts`.
- ✅ 🟡 **Correctness / Architecture / Standards / Usability**: Kanban subtitle under-specified — **Resolved**. PageSubtitle component extracted with concrete adoption on the kanban loaded branch.
- ✅ 🟡 **Standards**: Off-scale padding bypass — **Resolved**. Three concrete EXCEPTIONS entries documented (though see new schema-mismatch major).
- ✅ 🟡 **Standards**: `data-chip-leading` one-off — **Resolved**. Replaced by `data-slot="leading"` / `data-slot="subtitle"` matching established Topbar convention.
- ✅ 🟡 **Standards**: `statusVariant.ts` placement broke folder convention — **Resolved**. Relocated to `src/api/status-variant.ts`.
- 🟡 **Standards**: Kanban "live" chip / PageSubtitle introduction may warrant ADR — **Partially resolved**. Migration Notes documents the deviation; work-item amendment is described but not yet applied; no ADR proposed.
- ✅ 🟡 **Standards**: ChipSize rename not flagged — **Resolved**. Migration Notes covers it.
- 🟡 **Usability**: Required `variant` prop friction — **Not addressed**. Plan keeps `variant` required per the "thin presentational primitive" choice but doesn't document this as a deliberate trade-off.
- 🟡 **Usability**: Silent neutral fallback hides typos — **Not addressed**. JSDoc documents the contract but no dev-mode warning channel.
- ✅ 🟡 **Usability**: Dropping key labels — **Resolved**. Compensated via `aria-label`.

### New Issues Introduced

#### Major

- 🟡 **Code Quality / Test Coverage / Correctness**: `__SETS_FOR_TEST` silently no-ops when the export is absent
  **Location**: Phase 3 — `src/api/status-variant.ts` and its test
  The internal-invariants test reads the hook via dynamic import + `unknown` cast + `?? []` — if the export is removed, the for-loop iterates over an empty array and the test passes with zero assertions. Static import or an explicit `expect(internal).toBeDefined()` would fix this. Three lenses independently flagged it.

- 🟡 **Test Coverage**: "Tinted-between-bounds" assertion described but not codified
  **Location**: Phase 6 — `chip-resolved-colours.spec.ts`
  Phase 6 names the approach (check that channel values sit between `--ac-bg` and the full semantic colour) but provides no concrete `expectChannelBetween(channel, low, high)` helper or example assertion. Different implementers will produce different tolerance ranges; some may collapse to a weaker single-inequality check that fails to catch percentage drift.

- 🟡 **Standards**: New EXCEPTIONS entries don't match the existing schema
  **Location**: Phase 2 §4 — `migration.test.ts` EXCEPTIONS additions
  The three new entries omit the required `count` field (every existing entry includes it) and use `file: 'src/components/Chip/...'` while existing entries use `file: 'components/Chip/...'` (no `src/` prefix per the file's own comment). As written, the entries will fail TypeScript compilation and the harness will fail to match the literals.

#### Minor (high-confidence — effectively a Phase 2 blocker)

- 🔵 **Correctness**: `gap: 0.25rem` will fail `PX_REM_EM_RE` unless substituted with `var(--sp-1)`
  **Location**: Phase 2 §3 — Chip.module.css gap declaration
  The CSS contains `gap: 0.25rem` and the plan tells the implementer to "verify the value at implementation time and inline `var(--sp-1)` if exact." But `0.25rem` is not in the EXCEPTIONS additions, and `--sp-1` is `4px = 0.25rem` in `global.css` — so Phase 2's stated automated-verification step (`npm test src/styles/migration.test.ts`) fails as written. Substitute `gap: var(--sp-1)` directly in the snippet.

#### Minor (recommended)

- 🔵 **Code Quality / Correctness / Usability**: `chipForEntry(entry)` invoked three times in the Phase 5b JSX snippet
  **Location**: Phase 5b — LibraryTypeView migration
  The snippet calls the helper three times with non-null assertions, then prose-notes that the implementation should hoist. Showing the hoisted form (`const chip = chipForEntry(entry); return chip ? … : '—'`) directly removes the wasted invocations and `!` assertions.

- 🔵 **Test Coverage**: color-mix-ladder regex only enforced on the `green` variant
  **Location**: Phase 2 §1 — Chip.test.tsx CSS source assertions
  The locked-convention test pins `green`'s background but not `amber` / `red` / `violet`. Use `it.each` to cover all four variants.

- 🔵 **Code Quality**: Test regex `(8|18|30)%` is broader than the implementation (which uses only 8 and 30)
  **Location**: Phase 2 §1 — variant background regex
  Tighten to `(8|30)%` to lock the design intent.

- 🔵 **Correctness**: `PageSubtitle` subtitle-slot guard inconsistent with Chip's leading-slot guard
  **Location**: Phase 5e — PageSubtitle.tsx
  Chip's `hasLeading` collapses `null`/`false`/`undefined`; PageSubtitle's `children !== undefined` does not. Conditional children like `{loaded && <Chip/>}` would emit an empty subtitle wrapper.

- 🔵 **Code Quality / Architecture**: Two small deferred-to-implementation decisions remain (Phase 1 token-map placement; Phase 5e PageSubtitle spacing)
  Resolve in plan: name the exact token map, or pre-declare PageSubtitle's spacing tokens.

- 🔵 **Code Quality**: `isStatusKey` normalisation duplicates a contract owned by `status-variant.ts`
  **Location**: Phase 4 §2
  Lift `isStatusKey` (or `isStatusField`) into `src/api/status-variant.ts` so the case/separator policy for both sides of the mapping lives in one module.

- 🔵 **Test Coverage**: PageSubtitle lacks CSS-source `?raw` assertions
  Every other new/migrated component in the plan has them; PageSubtitle is the exception.

- 🔵 **Standards**: `--size-chip` / `--size-chip-md` are component-namespaced inside an otherwise generic `--size-*` scale
  Consider whether a generic `--size-xxxs` (10.5px) would slot more cleanly alongside `--size-xxs` (12px) — and would also collapse the existing Sidebar/ActivityFeed 10.5px EXCEPTIONS entries.

- 🔵 **Architecture / Standards**: Work-item amendment for kanban "live" chip + PageSubtitle is described but not yet applied; no ADR proposed
  Either complete the work-item amendment as part of plan acceptance, or add a focused ADR (e.g. `0027-page-subtitle-as-chip-host.md`).

- 🔵 **Usability**: Required `variant` and silent neutral fallback are kept by deliberate user choice but undocumented as design decisions
  Add a one-liner to "What We're NOT Doing" or Migration Notes capturing each so future contributors don't reopen the debate ad hoc.

- 🔵 **Usability**: `ChipVariant` type lacks JSDoc for direct consumers (semantic intent currently only lives in `status-variant.ts`'s Set comments)
  Annotate each variant inline so a developer hovering `ChipVariant` in IntelliSense sees the prescriptive guidance.

- 🔵 **Test Coverage / Correctness**: No aria-label test for array values; AMBER `'approvewchanges'` key looks like a typo without context
  Small completeness gaps — add the missing test case and a one-line comment on the AMBER key.

### Assessment

The plan is materially closer to ready than after pass 1. Every previously-critical issue is resolved, the structural concerns (architecture coupling, missing Consumer Contract, kanban under-specification, deferred decisions) are tightened, and the test pattern is now consistent with the canonical SseIndicator/migration-harness conventions.

The three new majors (and the `gap: 0.25rem` PX_REM_EM trap) are all small, self-contained edits — none rise to "redesign the approach"; all are mechanical fixes to specific code snippets. A short follow-up edit pass plus a final re-review of the same six lenses should clear the plan to **APPROVE**.

Recommended next-edit priority (highest first):
1. Substitute `gap: var(--sp-1)` in Phase 2 §3
2. Fix the three EXCEPTIONS entries (add `count: 1`, drop `src/` prefix)
3. Replace the `__SETS_FOR_TEST` test pattern with a static import + existence assertion (or drop the invariant test entirely — the behavioural tests already cover it)
4. Codify the "tinted-between-bounds" helper in Phase 6 with a concrete `expectChannelBetween` example
5. Show the hoisted `chipForEntry` form in Phase 5b
6. Tighten the variant background regex to `(8|30)%` and extend it to all four colour variants via `it.each`
7. Sweep the smaller minors (PageSubtitle guard alignment, `isStatusKey` co-location, PageSubtitle CSS-source assertion, missing JSDoc on `ChipVariant`, undocumented design decisions)

## Re-Review (Pass 3) — 2026-05-15T10:49:11Z

**Verdict:** COMMENT

Plan is acceptable but could be improved — see the one major finding below.

The pass-2 edits cleared all three previously-flagged majors and the `gap: 0.25rem` verification blocker. The plan now contains a static-import `__SETS_FOR_TEST` with explicit existence guards, a concrete `expectChannelsBetween` helper, hoisted `chipForEntry`, `it.each`-extended color-mix regex coverage across all four colour variants, an `isStatusKey` co-located with `statusToChipVariant`, `aria-label` convention codification in the Consumer Contract, rich JSDoc on `ChipVariant`, and explicit "What We're NOT Doing" entries for the required `variant` prop and silent neutral fallback.

One pass-2 fix introduced a fresh standards/correctness regression that the pass-2 reviewer did not flag: the EXCEPTIONS schema requires `kind: 'to-migrate' | 'irreducible'` (pass-3 reviewers verified by reading `migration.test.ts:44-46`). The pass-2 edit removed `kind` based on the pass-2 review's incorrect schema description. This is mechanical to fix and is the only remaining blocker before Phase 2 ships green.

### Previously Identified Issues (pass-2 carryover)

#### Pass-2 Majors (all resolved)

- ✅ 🟡 **Code Quality / Test Coverage / Correctness**: `__SETS_FOR_TEST` silently no-ops — **Resolved**. Static import; `toBeDefined()`, `length > 0`, `size > 0` guards.
- ✅ 🟡 **Test Coverage**: "Tinted-between-bounds" assertion not codified — **Resolved**. `expectChannelsBetween` helper now concrete with full parse + 6 channel inequalities + 2 endpoint exclusions.
- ⚠️ 🟡 **Standards**: EXCEPTIONS entries don't match existing schema — **Partially resolved**. `count` added, `src/` prefix dropped — but `kind: 'irreducible'` was incorrectly removed (see new major below).

#### Pass-2 Minors / Verification Blocker

- ✅ 🔵 **Correctness**: `gap: 0.25rem` will fail PX_REM_EM — **Resolved**. Now `gap: var(--sp-1)`.
- ✅ 🔵 **Code Quality / Correctness / Usability**: `chipForEntry` 3x in JSX — **Resolved**. Hoisted form shown.
- ✅ 🔵 **Test Coverage**: color-mix regex only on `green` — **Resolved**. `it.each` across all 4 colour variants for background and border.
- ✅ 🔵 **Code Quality**: Regex `(8|18|30)%` over-permissive — **Resolved**. Tightened to literal `8%`/`30%`.
- ✅ 🔵 **Correctness**: PageSubtitle guard inconsistent with Chip — **Resolved**. `hasChildren` mirrors `hasLeading`.
- ✅ 🔵 **Code Quality / Architecture**: Phase 1 / PageSubtitle deferred decisions — **Resolved**. `TYPOGRAPHY_TOKENS` named; concrete PageSubtitle CSS with `--sp-2`, `--sp-3`, `--sp-4`, `--size-h2`, `--size-sm`.
- ✅ 🔵 **Code Quality**: `isStatusKey` duplicated `status-variant.ts`'s normalisation — **Resolved**. `isStatusKey` exported from `status-variant.ts`.
- ✅ 🔵 **Test Coverage**: PageSubtitle lacks CSS-source assertions — **Partially resolved**. One `?raw` source check added (but uses dynamic-import idiom inconsistent with rest of codebase — see new minor).
- ✅ 🔵 **Standards / Usability**: ChipVariant JSDoc / convention documentation — **Resolved**.
- ✅ 🔵 **Usability**: Required `variant` and silent fallback undocumented — **Resolved**. Both captured in "What We're NOT Doing".
- ✅ 🔵 **Usability**: aria-label format for arrays — **Resolved**. Test pins `aria-label="tags: design, frontend"`.
- ⚠️ 🔵 **Architecture**: OriginPill / SseIndicator / banner manual-eyeball regression — **Still present**. Pass 3 confirms the verification step remains a manual smoke test.
- ⚠️ 🔵 **Architecture / Standards**: Kanban "live" / PageSubtitle work-item amendment / ADR — **Still present** (deliberate deferral).

### New Issues Introduced

#### Major

- 🟡 **Standards / Correctness**: New EXCEPTIONS entries omit the required `kind` field
  **Location**: Phase 2 §4 — `migration.test.ts` EXCEPTIONS additions
  The EXCEPTIONS array's actual type at `migration.test.ts:46` is `ReadonlyArray<Exception & { kind: 'to-migrate' | 'irreducible' }>`. Every existing entry carries `kind: 'irreducible'`. The pass-2 edit dropped `kind` from the three new Chip entries based on an incorrect schema description in the pass-2 review. As written, `npm run typecheck` and `npm test src/styles/migration.test.ts` will fail — Phase 2's stated success criteria are unreachable.
  Fix: add `kind: 'irreducible'` to each of the three entries.

#### Minor (high-confidence)

- 🔵 **Correctness**: `chipForEntry` divergence test uses fixture where both label sources resolve to `'accepted'`
  **Location**: Phase 5b — "pairs label and variant from chipForEntry consistently" test
  Entry has `{ status: 'accepted', date: '2026-04-05' }`. `statusCellValue` returns 'accepted' (preferring status when present), and `frontmatter.status` is also 'accepted'. A broken implementation that swapped the sources would still produce `[data-variant="green"]` + `textContent === 'accepted'`. Add a second case (status absent, only date present) asserting variant=neutral and label=date so both directions are pinned.

- 🔵 **Correctness**: Phase 6 success-criteria arithmetic contradicts body
  **Location**: Phase 6 Success Criteria
  Body: "12 cells × 2 themes = 24 screenshots". Success criterion: "24 cells × 2 platforms = 48 baseline files". Should read "12 cells × 2 themes × 2 platforms = 48".

- 🔵 **Test Coverage / Standards**: PageSubtitle uses dynamic `await import('...?raw')` instead of static top-level import
  **Location**: Phase 5e — PageSubtitle test
  Every other `?raw` test in the plan uses `import css from './X.module.css?raw'` at the top of the file. PageSubtitle uniquely uses an inline dynamic import with an `as string` cast. Hoist to a static top-level import for consistency.

#### Minor (lower priority — discretionary)

- 🔵 **Test Coverage**: `expectChannelsBetween` endpoint-inequality uses string equality on `rgb()` strings that may not match `getComputedStyle`'s exact format — compare parsed `[r,g,b]` tuples instead so endpoint-collapse failures are actually caught.
- 🔵 **Test Coverage**: KanbanBoard fixture API uses two different prop shapes (`data={loadedFixture}` vs `state="loading"`) without a "TBD" note.
- 🔵 **Test Coverage / Code Quality**: Templated color-mix regex `\\${token}` interpolation deserves a one-line smoke check at implementation time.
- 🔵 **Correctness / Test Coverage**: INDIGO `reviewed` vs AMBER `revised` near-miss not pinned by a direct contrast test.
- 🔵 **Code Quality / Standards**: `ChipVariant` JSDoc and `status-variant.ts` Set comments duplicate semantic intent — accept the duplication for two-sided discoverability or add a cross-reference comment.
- 🔵 **Code Quality**: `expectChannelsBetween` packs parse + 6 inequalities + 2 endpoint exclusions in 20 lines — could split into smaller helpers if reused later.
- 🔵 **Architecture**: `status-variant.ts` module name slightly understates its widened responsibility (now also owns `isStatusKey`).
- 🔵 **Architecture**: OriginPill / SseIndicator / banner regression still relies on manual verification (carry-over).
- 🔵 **Architecture / Standards**: PageSubtitle extraction / kanban "live" chip still pending work-item amendment or ADR (carry-over; deliberate deferral acceptable but worth completing at plan-merge).
- 🔵 **Usability**: Silent-neutral rationale in "What We're NOT Doing" could be sharpened — the semantic argument ("called on inputs that are not statuses by design") is stronger than the tactical "test-suite noise" framing currently captured.
- 🔵 **Usability**: `violet` JSDoc reads as an invitation to adopt ("reserved for future use") — sharper "do not pick directly" phrasing would reinforce the Consumer Contract.
- 🔵 **Usability**: `data-slot="<name>"` convention not named in the Consumer Contract — adding it makes the cross-primitive pattern discoverable.

### Assessment

The plan is materially ready. The one remaining major (`kind` field on EXCEPTIONS entries) is a mechanical TypeScript fix that takes a single Edit operation. The three high-confidence minors are also mechanical. With those four fixes the plan reaches **APPROVE** without further structural changes.

The lower-priority minors are either deliberate trade-offs the user has accepted, carry-overs the plan has documented but not closed, or sharpening opportunities that don't block implementation.

Recommended final-edit priority:
1. Add `kind: 'irreducible'` to all three Phase 2 §4 EXCEPTIONS entries
2. Fix Phase 6 success-criteria arithmetic (`12 cells × 2 themes × 2 platforms = 48`)
3. Add the second `chipForEntry` divergence test case (status absent, date present → neutral chip with date label)
4. Hoist PageSubtitle's `?raw` import to top-level
5. (Optional) Apply the discretionary minors as a follow-up sweep
