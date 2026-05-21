---
date: "2026-05-23T12:50:00+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-22-0081-status-badge-component.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, standards, usability, compatibility]
review_pass: 3
status: complete
---

## Plan Review: 0081 — StatusBadge / VerdictBadge / ResultBadge Implementation Plan

**Verdict:** REVISE

The plan delivers a well-decomposed, TDD-first component family with clear
single-responsibility seams and accurate cross-phase compatibility (importer
line numbers verified, atomic rename, suite green at every step). Vocabulary
isolation is principled and well-tested, and the deviation from the work-item
AC is acknowledged explicitly. The plan requires revision before
implementation, however: a new project-wide `data-component` observability
convention is introduced without anchoring, the four-layer composition stack
(Chip → FrontmatterChip → ValueBadge → wrappers) carries an abstraction whose
necessity multiple lenses question, several route-level ACs are only verified
manually, and a couple of dispatch tests under-assert their contract.

### Cross-Cutting Themes

- **`data-component` introduced as a new convention without anchoring**
  (flagged by: standards, usability, architecture, code-quality, compatibility) —
  Zero prior art in the frontend; `data-testid` is the established hook. The
  plan loads behaviour-critical dispatch tests onto an attribute that is
  invented in this story, with no ADR, README, or rationale linking it back
  to the codebase's existing observability conventions. Several lenses
  independently suggest either switching to `data-testid` or capturing
  `data-component` as a documented standard.

- **`ValueBadge` layer and three thin wrappers carry abstraction the plan
  hasn't justified** (flagged by: architecture, code-quality, usability) —
  Three lenses independently noted that `ValueBadge` could collapse into
  `FrontmatterChip` (which already accepts variant + dataComponent), and that
  the three wrappers are ~10-line files whose only difference is the mapping
  and the `dataComponent` string. The selection rubric for a future developer
  (when to pick FrontmatterChip vs ValueBadge vs StatusBadge) is missing.

- **`ChipProps` index signature `[dataAttr: \`data-${string}\`]: unknown`
  weakens typing** (flagged by: code-quality, correctness, standards,
  compatibility) — Accepting `unknown` for data-* values means a caller
  passing `data-meta={someObject}` type-checks cleanly but produces
  `[object Object]` in the DOM. Four lenses recommend tightening to
  `string | number | boolean | undefined`, or replacing the open signature
  with explicit named props (`'data-component'?: string`).

- **`verdictToVariant` defensive coverage of validation tokens blurs the
  vocabulary-isolation rationale** (flagged by: architecture, code-quality,
  test-coverage) — `pass`/`fail`/`partial` live in both `verdictToVariant`
  and `resultToVariant`, contradicting the "vocabularies stay independent"
  motivation, and the AC integration fixture (`verdict: pass → green`)
  becomes brittle to any future narrowing of `verdictToVariant`.

### Tradeoff Analysis

- **Abstraction vs extensibility**: The wrappers add named call sites and a
  composition seam (good for future vocabularies that diverge in behaviour)
  at the cost of YAGNI (each new vocabulary needs a new wrapper + test file).
  Recommendation: keep wrappers if at least one is expected to grow
  behaviour beyond mapping injection; otherwise collapse to a config-driven
  dispatcher.
- **Strict typing vs caller flexibility**: A narrow `data-component`-only
  prop catches typos and documents intent; an open index signature lets
  Chip carry arbitrary observability hooks without re-editing. Multiple
  lenses lean toward narrow.

### Findings

#### Critical

_(none)_

#### Major

- 🟡 **Test Coverage**: Route-level surface ACs only verified manually
  **Location**: Phase 8: Manual Verification (work-item-review and validation surfaces)
  The work item has explicit ACs for plan-review, work-item-review, and validation surfaces, yet the plan verifies them only via manual smoke checks. A doc-loader regression that stripped `verdict`/`result` from frontmatter would pass the suite while breaking the surface ACs.

- 🟡 **Test Coverage**: Case-insensitive dispatch test under-asserts which badge wins
  **Location**: Phase 8: Add — `dispatches tone keys case-insensitively`
  The `it.each(['Status','STATUS','Verdict','VERDICT','Result','RESULT'])` block only asserts that *some* tone component renders. A bug routing 'Verdict' to StatusBadge would not be caught — mutation testing fails here.

- 🟡 **Test Coverage**: AC integration fixture couples to defensive cross-vocab coverage
  **Location**: Phase 8: AC integration fixture
  The fixture asserts `verdict: pass → green` which only passes because `verdictToVariant` defensively covers validation tokens. Any future narrowing of `verdictToVariant` (entirely consistent with the plan's "vocabularies are independent" stance) breaks the most load-bearing AC test.

- 🟡 **Standards**: `data-component` observability hook has no prior art — new convention introduced silently
  **Location**: Phase 1 + every subsequent phase that depends on `data-component`
  Zero matches for `data-component` in `frontend/src`. `data-testid` is the established hook (Chip.test.tsx, ActivityFeed, FilterPill, Popover). The plan adds a new project-wide convention without ADR, testing-guide entry, or rationale.

- 🟡 **Usability**: No documented path for adding a new vocabulary
  **Location**: Desired End State / Phase 8 (BADGE_FOR_KEY)
  Adding tone for a new key (`priority`, `severity`) requires creating an api/<key>-variant.ts, a <Key>Badge/, registering in BADGE_FOR_KEY, and replicating the test matrix — none of this is documented. The abstraction earns no leverage over the next vocabulary, and the recipe is non-obvious.

- 🟡 **Usability**: Five-component family lacks a selection rubric
  **Location**: Overview / Desired End State
  When should a future developer reach for FrontmatterChip vs ValueBadge vs StatusBadge? Is ValueBadge a public component or an internal composition helper? Why are some "Chip" and some "Badge"? Without a rubric, the family will be misused — most likely via ad-hoc inline mappings against ValueBadge bypassing vocabulary files.

- 🟡 **Usability**: `data-component` convention introduced without anchoring as a project standard
  **Location**: Overview / Phase 1
  Future devs won't know whether to add `data-component` to new components, how to name it (PascalCase? export name?), or how it relates to `data-slot` / `data-variant` / `data-size` / `data-testid` already in use.

#### Minor

- 🔵 **Architecture**: Four-layer composition may be one layer deeper than necessary
  **Location**: Phase 4 (ValueBadge) + Phases 5–7 (wrappers)
  `FrontmatterChip` already accepts variant + dataComponent; `ValueBadge` collapses to ~3 lines. Each wrapper could compose `FrontmatterChip` directly.

- 🔵 **Architecture**: Dispatch registry closed to extension
  **Location**: Phase 8: `BADGE_FOR_KEY`
  Adding a fourth tone key requires editing FrontmatterChips. Mild open/closed violation; acceptable today, becomes a refactor when a fourth key appears.

- 🔵 **Architecture**: `verdictToVariant` defensive coverage blurs vocabulary boundaries
  **Location**: Phase 6
  Including `pass`/`fail`/`partial` in `verdictToVariant` weakens the cohesion argument used to justify three separate mappings.

- 🔵 **Architecture**: Chip index signature widens the public surface
  **Location**: Phase 1: `[dataAttr: \`data-${string}\`]: unknown`
  Couples Chip to an extensibility contract it didn't need; risks collisions with future component-managed data attributes.

- 🔵 **Architecture**: `normaliseValue` is a generic utility living under `api/`
  **Location**: Phase 2
  `api/` overloads its meaning if it houses both domain mappings and string utilities. Rename or relocate.

- 🔵 **Architecture**: Case-insensitive key dispatch beyond AC requirement
  **Location**: Phase 8: `badgeFor`
  Lowercasing keys before dispatch conceals corpus drift; AC only requires case-insensitive *value* lookup, not *key* lookup.

- 🔵 **Code Quality**: Three near-identical wrapper components may be premature abstraction
  **Location**: Phases 5/6/7
  Each wrapper is ~10 lines; a `Record<string, {mapping, dataComponent}>` consumed by the dispatcher achieves the same with one fewer abstraction.

- 🔵 **Code Quality**: `ChipProps` index signature uses `unknown`
  **Location**: Phase 1
  Tighten to `string | number | boolean | undefined` to match HTML attribute semantics.

- 🔵 **Code Quality**: `__SETS_FOR_TEST` export carried forward without revisiting
  **Location**: Phase 2: status-variant.ts
  Sibling files (`verdict-variant.ts`, `result-variant.ts`) don't add an analogous backdoor — inconsistent across the three siblings. Either remove or replicate.

- 🔵 **Code Quality**: `badgeFor` lowercases on every dispatch
  **Location**: Phase 8
  Either inline the lookup into the `entries.map` body or annotate that `BADGE_FOR_KEY` keys must be lowercase.

- 🔵 **Correctness**: Plan-review verdict vocabulary description omits REQUEST_CHANGES
  **Location**: Current State Analysis lines 65–68
  Prose lists `verdict: APPROVE | REVISE | COMMENT` but the work item AC mandates `APPROVE | REVISE | REQUEST_CHANGES | COMMENT`. Implementation is correct; only the documentation is wrong, and it risks misleading a future maintainer into deleting the `REQUEST_CHANGES → red` mapping.

- 🔵 **Correctness**: Falsy non-empty-string values (`0`, `false`) pass the filter but are unspecified
  **Location**: Phase 8 entry filter
  Existing behaviour passes `0` and `false` through; not asserted at the list level. Add a lock-in test.

- 🔵 **Correctness**: AC fixture mixes a status-shaped value with a validation-shaped verdict
  **Location**: Phase 8 AC integration fixture
  `status: 'Accepted'` + `verdict: 'pass'` together is a synthetic combination unlikely to occur in a real document. Either annotate or split.

- 🔵 **Standards**: `frontmatterKey` prop name has no prior art
  **Location**: Phase 3 / 4 / 5 / 6 / 7
  Components only carry `key`/`value` pairs; the `frontmatter` qualifier is informational. Consider `name`/`value` or `label`/`value`.

- 🔵 **Standards**: Rename drops the `Chip` namespacing without documented rationale
  **Location**: Phase 2
  `statusToChipVariant` → `statusToVariant` is defensible (uniform suffix), but worth a one-line rationale.

- 🔵 **Standards**: Index signature weakens type safety on a primitive component
  **Location**: Phase 1: ChipProps
  `data-componenet` (typo) compiles cleanly. Prefer explicit `'data-component'?: string`.

- 🔵 **Standards**: `__SETS_FOR_TEST` preserved on status-variant only — sibling helpers don't replicate it
  **Location**: Phase 2 vs Phases 6/7

- 🔵 **Usability**: Three-wrapper split deviates from a single-StatusBadge AC without a migration trail
  **Location**: Deviation from the work-item AC
  ACs that read "Given StatusBadge is rendered with frontmatterKey=verdict" are now satisfied by VerdictBadge. Either update the work item or surface the mapping more prominently (callout, not paragraph).

- 🔵 **Usability**: Case-insensitive key dispatch is a hidden affordance
  **Location**: Phase 8: badgeFor
  Document the case-insensitive intent; reconcile asymmetry with value normalisation.

- 🔵 **Test Coverage**: Object aria-label parity untested
  **Location**: Phase 3
  Array aria-label parity is asserted; object analogue isn't. A future refactor stringifying value but leaving aria-label as `String(value)` would slip past.

- 🔵 **Test Coverage**: `as never` casts hide the reserved-attribute contract from the type system
  **Location**: Phase 1
  Adds a code comment so future maintainers understand RESERVED_DATA_ATTRS is the runtime contract.

- 🔵 **Test Coverage**: Wrapper badge tests largely mirror their pure-function mapping tests
  **Location**: Phases 5/6/7
  Vocabulary matrices duplicated at mapping layer and wrapper layer; thin smoke cases at wrapper layer would suffice.

- 🔵 **Test Coverage**: `__SETS_FOR_TEST` regex `^[a-z]+$` is too tight
  **Location**: Phase 2
  Will fail on first numeric-bearing status. Prefer `^[a-z0-9]+$` or assert the round-trip property.

- 🔵 **Test Coverage**: Source-order test really asserts `Object.entries` order
  **Location**: Phase 8
  Either rename the test or annotate that upstream YAML is the source-order authority.

- 🔵 **Test Coverage**: Dispatch tests have no count assertion (accidental double-attribution untested)
  **Location**: Phase 8 AC fixture
  Add `querySelectorAll('[data-component]').length === 4`.

- 🔵 **Compatibility**: ChipProps index signature uses `unknown` for data-* values
  **Location**: Phase 1
  Same concern multiple lenses raise — narrow the type.

- 🔵 **Compatibility**: `aria-label` threaded through Record cast in Chip
  **Location**: Phase 1
  Destructure `'aria-label': ariaLabel` explicitly; remaining `...rest` then unambiguously contains data-* only.

#### Suggestions

- 🔵 **Code Quality**: `formatChipValue` may need to be shared
  **Location**: Phase 3
  Acknowledge it's intentionally private, or proactively extract to `api/format-chip-value.ts`.

- 🔵 **Code Quality**: Verdict and result vocabulary overlap not annotated in code
  **Location**: Phase 6 / Phase 7
  Add a one-line cross-reference comment in each so future readers see the relationship.

- 🔵 **Correctness**: `normaliseValue` does not specify Unicode normalisation
  **Location**: Phase 2
  Document that Unicode-typographic separators (en-dash, em-dash, U+2010) are not collapsed.

- 🔵 **Correctness**: TS index signature may not enforce intent
  **Location**: Phase 1
  Document or tighten.

- 🔵 **Usability**: `ValueBadge` requires `dataComponent` with no default
  **Location**: Phase 4
  Either default to `'ValueBadge'` or mark ValueBadge as internal-only in module JSDoc.

- 🔵 **Compatibility**: Consider temporary re-export alias for in-flight branches
  **Location**: Phase 2
  Optional; skip if no in-flight branches reference the helper.

### Strengths

- ✅ Eight-phase TDD decomposition with explicit dependency edges, every phase ending green and shippable.
- ✅ Composition over inheritance applied cleanly: Chip → FrontmatterChip → ValueBadge → wrappers.
- ✅ Shared `normaliseValue` helper eliminates the risk of vocabulary case/separator drift.
- ✅ Vocabulary isolation is principled and tested with explicit cross-leakage cases on every badge.
- ✅ Reserved-attribute protection on Chip (`data-variant`, `data-size`) is tested explicitly.
- ✅ All four `statusToChipVariant` importers (verified by grep) are migrated atomically in Phase 2 — no broken window.
- ✅ Importer line numbers in the plan match the current codebase exactly.
- ✅ Status vocabulary sets (GREEN/INDIGO/AMBER/RED) reproduced verbatim from the live source, including dual `approvewithchanges`/`approvewchanges` aliases.
- ✅ Deviation from the work-item AC is explicitly acknowledged in its own section with rationale.
- ✅ Single render path (LibraryDocView serves all three review surfaces) identified up front.
- ✅ Phase 8 dispatch table (BADGE_FOR_KEY) is a flat, readable structure.
- ✅ Non-string inputs (null, undefined, number, boolean, array, object) covered for every mapping at both unit and wrapper layers.

### Recommended Changes

Ordered by impact. Each entry references the finding(s) it addresses.

1. **Resolve the `data-component` convention** (addresses: Standards/`data-component` no prior art, Usability/no anchoring, Architecture/Chip surface widening, Code-Quality/`unknown` typing, Compatibility/index signature) — pick one of:
   - (a) Replace `data-component` with `data-testid` to align with the established convention.
   - (b) Keep `data-component`, but capture it as a documented standard (short ADR or testing-guide section) and replace the open `[dataAttr: \`data-${string}\`]: unknown` index signature on `ChipProps` with an explicit `'data-component'?: string` prop. This narrows the API to its only documented use.

2. **Decide on the `ValueBadge` layer** (addresses: Architecture/four-layer composition, Code-Quality/premature abstraction, Usability/selection rubric, Usability/`ValueBadge` required prop) — pick one of:
   - (a) Collapse `ValueBadge` and have each wrapper compose `FrontmatterChip` directly (one-line wrappers).
   - (b) Keep `ValueBadge` but make its intended audience explicit: mark it internal-only (JSDoc) and add a 1-paragraph "When to use which" section to the plan covering FrontmatterChip vs ValueBadge vs StatusBadge/VerdictBadge/ResultBadge.

3. **Add route-level integration tests for the three surfaces** (addresses: Test-Coverage/route-level ACs only manual) — Phase 8 should add three minimal tests that mount `LibraryDocView` (or the relevant route harness) with fixtures matching the three skills' actual frontmatter shape, and assert the expected `data-component` + `data-variant` appear.

4. **Tighten dispatch and integration tests** (addresses: Test-Coverage/case-insensitive under-asserts, Test-Coverage/AC fixture coupling, Test-Coverage/no count assertion) —
   - Replace the case-folded dispatch `it.each` with a parameterised `[key, expectedComponent]` matrix.
   - Split the AC integration fixture into one plan-review variant (`verdict: 'APPROVE'`) and one validation variant (`verdict: 'pass'`), or annotate the existing fixture as deliberately exercising defensive cross-vocab coverage.
   - Add a count assertion (`querySelectorAll('[data-component]').length === 4`) before the per-component assertions.

5. **Fix the Current State Analysis vocabulary description** (addresses: Correctness/REQUEST_CHANGES omission) — update lines 65–68 to read `verdict: APPROVE | REVISE | REQUEST_CHANGES | COMMENT` for both plan-review and work-item-review, matching the work item AC and the actual Phase 6 implementation.

6. **Reconcile `verdictToVariant` defensive coverage with the isolation rationale** (addresses: Architecture/blurred vocabulary boundaries, Code-Quality/overlap not annotated) — pick one of:
   - (a) Drop `pass`/`fail`/`partial` from `verdictToVariant` and let dispatch decide (validation emits `result:`, not `verdict:`, so the defensive coverage is unused in practice).
   - (b) Keep the coverage but annotate it in code with a cross-reference to `result-variant.ts` and reframe the rationale: the overlap is by deliberate corpus convention, not happenstance.

7. **Audit `__SETS_FOR_TEST` once across the three sibling helpers** (addresses: Code-Quality/`__SETS_FOR_TEST` carried forward, Standards/sibling helpers don't replicate, Test-Coverage/regex too tight) — decide whether to remove it from `status-variant.ts` (and rewrite the consuming test to assert via the public API) or replicate it on the two new helpers. Either way, loosen the `^[a-z]+$` regex to `^[a-z0-9]+$` or to assert `normaliseValue(k) === k`.

8. **Reconsider the `frontmatterKey` prop name** (addresses: Standards/no prior art) — rename to `name` or `label` across `FrontmatterChip` / `ValueBadge` / wrappers; the `frontmatter` qualifier belongs on the chip-list renderer's `frontmatter: Record<string, unknown>` prop and nowhere else.

9. **Document remaining minor inconsistencies** (addresses: assorted minors) — add brief notes for the rename rationale (Phase 2), case-insensitive dispatch intent (Phase 8 `badgeFor`), Unicode normalisation scope (Phase 2 `normaliseValue`), and object aria-label parity test (Phase 3).

10. **Update the work item to reflect the implemented component family** (addresses: Usability/AC migration trail) — either edit work item 0081's verdict ACs to name `VerdictBadge` (instead of `StatusBadge` with `frontmatterKey="verdict"`), or add a callout in the plan's deviation section listing AC → component mappings explicitly.

---

*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan demonstrates strong architectural thinking — a clean
decomposition of FrontmatterChips into a chip-list dispatcher, a generic
FrontmatterChip, a generic ValueBadge, and three thin vocabulary-specific
wrappers, with shared normalisation extracted. Dependency direction is sound
(api/ → no UI; primitive Chip depends on nothing; wrappers depend on the
generic), and coupling/cohesion improvements are clear. The deviation from
the work-item AC (splitting StatusBadge into three wrappers) is acknowledged
and architecturally justified, though it introduces a four-layer composition
stack whose layering necessity is worth scrutinising.

**Strengths**:
- Clean separation of concerns across chip-list rendering, per-key chip rendering, variant resolution, and vocabulary mappings.
- Dependency inversion via injected `valueToVariant` function.
- Vocabulary mappings kept independent.
- Shared `normaliseValue` helper unifies a previously implicit policy.
- Phasing respects dependency direction; each phase ends green and shippable.
- AC deviation explicitly acknowledged.
- Single render path (LibraryDocView) identified up front.
- `data-*` pass-through bounded by reserved-attribute allowlist.

**Findings**:
- Minor (medium): Four-layer composition may be one layer deeper than necessary — Phase 4 + Phases 5–7.
- Minor (high): Dispatch registry closed to extension — Phase 8: BADGE_FOR_KEY.
- Minor (medium): `verdictToVariant` defensive coverage blurs vocabulary boundaries — Phase 6.
- Minor (high): Index signature widens Chip primitive's public surface — Phase 1.
- Minor (medium): `normaliseValue` is a generic utility living under `api/` — Phase 2.
- Minor (low): Case-insensitive dispatch beyond AC requirement — Phase 8.

### Code Quality

**Summary**: Well-decomposed component family with clear single-responsibility
seams, TDD per phase, consistent naming, shared normaliseValue enforcing DRY,
and a clean testability seam via data-component. Main concerns: three
near-identical wrappers may be premature abstraction, Chip prop typing
weakness, and a small dead-code/test-only export already in the codebase that
the plan carries forward without revisiting.

**Strengths**:
- Eight-phase decomposition with explicit dependency edges; suite stays green.
- Composition over inheritance applied cleanly.
- Shared `normaliseValue` helper eliminates duplication.
- `data-component` gives DOM-level dispatch assertions without leaking implementation into prop APIs.
- Reserved-attribute protection on Chip tested explicitly.
- Vocabulary isolation asserted with cross-leakage tests.
- Single-commit rename avoids a broken window.
- `BADGE_FOR_KEY` is a flat, readable structure.

**Findings**:
- Minor (medium): Three near-identical wrapper components may be premature abstraction — Overview / Phases 5/6/7.
- Minor (high): `ChipProps` index signature uses `unknown` — Phase 1.
- Minor (medium): `__SETS_FOR_TEST` carried forward without revisiting — Phase 2.
- Minor (high): `badgeFor` lowercases on every dispatch — Phase 8.
- Suggestion (medium): `formatChipValue` may need to be shared — Phase 3.
- Suggestion (low): Verdict/result vocabulary overlap not annotated in code — Phases 6/7.

### Test Coverage

**Summary**: Testing-first with strong vocabulary coverage via it.each tables
and clear AC-to-test traceability across phases. However, meaningful
duplication between mapping tests and badge-component tests, brittle DOM
selectors, hidden coupling to dispatch order in the AC fixture, and a gap in
surface-level coverage: work-item-review and validation surfaces are only
verified manually despite having dedicated ACs.

**Strengths**:
- Test-first ordering explicit in every phase.
- Vocabulary mappings covered at both unit and rendered-component layers.
- Vocabulary isolation tested for every wrapper.
- Non-string inputs covered for every mapping and wrapper.
- Phase 8 includes an AC integration fixture mirroring the work item.
- Existing case/separator-insensitivity coverage preserved verbatim.
- Reserved-attribute protection tested explicitly.

**Findings**:
- Major (high): Route-level surface ACs only verified manually — Phase 8.
- Major (high): Case-insensitive dispatch test under-asserts — Phase 8.
- Major (medium): AC integration fixture relies on defensive cross-vocab coverage — Phase 8.
- Minor (high): Object aria-label parity untested — Phase 3.
- Minor (high): `as never` casts hide reserved-attribute contract from type system — Phase 1.
- Minor (medium): Wrapper badge tests largely mirror their pure-function mapping tests — Phases 5/6/7.
- Minor (medium): `__SETS_FOR_TEST` regex `^[a-z]+$` too tight — Phase 2.
- Minor (medium): Source-order test really asserts Object.entries order — Phase 8.
- Minor (low): No count assertion in dispatch tests — Phase 8.

### Correctness

**Summary**: Largely correct: preserves live status vocabulary verbatim,
correctly externalises `normaliseValue` with consistent semantics,
per-vocabulary mappings prevent cross-leakage, dispatch is sound. Two
correctness concerns: a documentation mismatch in Current State Analysis
omits `REQUEST_CHANGES` from plan-review/work-item-review verdict vocabulary
even though the implementation correctly includes it, and the plan doesn't
specify behaviour for `0`/`false` values flowing through `FrontmatterChips`.

**Strengths**:
- Status vocabulary sets reproduced verbatim from live source.
- `normaliseValue` centralised and tested for non-string inputs.
- Per-vocabulary mappings explicitly prevent cross-leakage.
- Dispatch uses lowercase normalisation against a lowercase-keyed table.
- Reserved data-variant / data-size protection tested.
- Source-order preservation tested with AC integration fixture.

**Findings**:
- Minor (high): Plan-review verdict vocabulary description omits REQUEST_CHANGES — Current State Analysis lines 65–68.
- Minor (high): Falsy non-empty-string values (`0`, `false`) unspecified at list level — Phase 8 entry filter.
- Minor (medium): AC fixture mixes status-shaped value with validation-shaped verdict — Phase 8.
- Suggestion (medium): `normaliseValue` does not specify Unicode normalisation behaviour — Phase 2.
- Suggestion (low): TS index signature `unknown` may not enforce intent — Phase 1.

### Standards

**Summary**: Plan respects established frontend component layout, kebab-case
api helper file naming, and CSS-module patterns. However, it introduces two
new conventions — `data-component` observability attribute and
`frontmatterKey`/`dataComponent` prop pair — with no prior art and without
flagging the new-convention status. The deviation from the work item's AC is
called out in a dedicated section.

**Strengths**:
- File/folder layout matches existing visualiser convention exactly.
- New api helper files follow established kebab-case pattern.
- Deviation from work item AC explicitly flagged.
- All four importers migrated in single change.
- Phase 1 protects Chip-managed data-variant/data-size from caller override.

**Findings**:
- Major (high): `data-component` observability hook has no prior art — Phase 1+.
- Minor (high): `frontmatterKey` prop name has no prior art — Phases 3/4/5/6/7.
- Minor (medium): Rename drops `Chip` namespacing without documented rationale — Phase 2.
- Minor (medium): Index signature weakens type safety on a primitive — Phase 1.
- Minor (high): `__SETS_FOR_TEST` preserved on status-variant only — Phase 2 vs Phases 6/7.

### Usability

**Summary**: Reasonable composition hierarchy with clean separation of
concerns, but the API surface presents real DX friction: five overlapping
component names whose distinctions aren't documented, three near-identical
vocabulary mapping files with no discoverability when a new vocabulary
appears, an unanchored `data-component` convention, and an AC deviation
downstream readers will need to reconcile.

**Strengths**:
- Composition over inheritance — wrappers reduce surface area.
- Vocabulary isolation is principled and well-tested.
- Shared normaliseValue eliminates case/separator drift risk.
- Phase 2 atomic rename — no half-renamed window.
- Reserved data-variant / data-size protection is a defensive least-surprise touch.

**Findings**:
- Major (high): No documented path for adding a new vocabulary — Desired End State / Phase 8.
- Major (high): Five-component family lacks a selection rubric — Overview / Desired End State.
- Major (high): `data-component` introduced without anchoring as project standard — Overview / Phase 1.
- Minor (medium): Three-wrapper split deviates from single-StatusBadge AC without migration trail — Deviation section.
- Minor (medium): Case-insensitive key dispatch is a hidden affordance — Phase 8: badgeFor.
- Suggestion (medium): ValueBadge requires dataComponent with no default — Phase 4.

### Compatibility

**Summary**: API contract changes handled well within an internal frontend
module — four importers accurately identified with correct line numbers,
migrated in a single phase. The Chip primitive change is additive and does
not break existing call sites. No external consumers exist, so the
single-commit rename and `isStatusKey` deletion are safe.

**Strengths**:
- All four importers migrated in one phase — textbook safe internal rename.
- Verified by grep: only the four documented importers reference statusToChipVariant.
- Importer line numbers in the plan match the codebase exactly.
- Chip data-* pass-through is purely additive; no JSX-spread callers exist.
- Reserved-attribute protection explicitly tested.
- Phase ordering preserves cross-phase compatibility.

**Findings**:
- Minor (medium): ChipProps index signature uses `unknown` for data-* values — Phase 1.
- Minor (medium): `aria-label` threaded through Record cast — Phase 1.
- Suggestion (high): Consider temporary re-export alias for in-flight branches — Phase 2.

---

## Re-Review (Pass 2) — 2026-05-23

**Verdict:** COMMENT

All 10 recommended changes from Pass 1 were applied, including a substantive reshape: `ValueBadge` collapsed (4-component family), `data-component` → `data-testid` for the observable hook, `frontmatterKey` → `name` with wrappers hard-coding their key, `verdictToVariant` defensive cross-vocab coverage removed, `__SETS_FOR_TEST` replicated across siblings with round-trip assertion, route-level integration tests added, dispatch tests tightened with two surface-shaped AC fixtures, the work item rewritten to match the implementation (Deviation section deleted from the plan). The plan is now substantially in good shape; one new convention issue surfaced and a handful of minor latent concerns were identified.

### Previously Identified Issues

#### Original Majors

- 🟢 **Test Coverage**: Route-level surface ACs only verified manually — **Resolved** (new `LibraryDocView.dispatch.test.tsx` covers all three review surfaces).
- 🟢 **Test Coverage**: Case-insensitive dispatch test under-asserts which badge wins — **Resolved** (parameterised `[key, expectedComponent]` matrix).
- 🟢 **Test Coverage**: AC integration fixture couples to defensive cross-vocab coverage — **Resolved** (split into plan-review-shaped and validation-shaped fixtures using canonical vocabularies; count assertion added).
- 🟢 **Standards**: `data-component` observability hook has no prior art — **Resolved** (switched to `data-testid`).
- 🟢 **Usability**: `data-component` convention introduced without anchoring as project standard — **Resolved** (using existing convention).
- 🟡 **Usability**: No documented path for adding a new vocabulary — **Still present, re-flagged at minor severity** by architecture (dispatch registry closed to extension) and usability lenses. The lens explicitly notes this is acceptable today and is a "documentation, not code" gap.
- 🟢 **Usability**: Five-component family lacks selection rubric — **Resolved at major; downgraded to suggestion** (family reduced to 4 components; the selection question is now smaller — Chip → FrontmatterChip → \*Badge — but documentation of the rubric is still encouraged).

#### Original Minors (Spot Check)

- 🟢 Architecture/Code-Quality/Compatibility — ChipProps index signature `unknown`: **Resolved** (replaced with explicit named `'data-testid'?: string` prop; no index signature, no `RESERVED_DATA_ATTRS`, no `pickDataAttrs` helper).
- 🟢 Architecture/Code-Quality — Four-layer composition: **Resolved** (ValueBadge collapsed; three wrappers compose FrontmatterChip directly).
- 🟢 Architecture/Code-Quality/Test-Coverage — `verdictToVariant` defensive coverage blurs boundaries: **Resolved** (pass/partial/fail removed from verdictToVariant; vocabulary-isolation tests added asserting result-shaped tokens stay neutral under verdict).
- 🟢 Correctness — REQUEST_CHANGES omitted from Current State Analysis: **Skipped** (empirically verified plan-review skill emits APPROVE | REVISE | COMMENT only; plan prose was correct).
- 🟢 Standards — `__SETS_FOR_TEST` preserved on status-variant only — sibling helpers don't replicate it: **Resolved** (now replicated on all three sibling helpers with consistent invariant tests).
- 🟢 Standards — `frontmatterKey` prop name has no prior art: **Resolved** (renamed to `name` on FrontmatterChip; removed entirely from wrappers).
- 🟢 Test-Coverage — Object aria-label parity untested: **Resolved** (assertion added; see note below about CSS selector mechanics).
- 🟢 Test-Coverage — Source-order test description mismatch: covered by the new fixtures.
- 🟢 Test-Coverage — `__SETS_FOR_TEST` regex `^[a-z]+$` too tight: **Partially resolved** (replaced with round-trip property; correctness lens notes a complementary regex check would strengthen invariant — see new minor below).

### New Issues Introduced

#### Major

- 🟡 **Code-Quality + Standards**: PascalCase `data-testid` values diverge from established kebab-case convention.
  **Location**: Phase 1 + every test that queries `[data-testid="StatusBadge"]` etc.
  Every existing `data-testid` in `frontend/src` uses kebab-case (`filter-pill-fetching`, `activity-live-badge`, `glyph-cell-*`, `template-preview-pane`, etc.). The plan introduces a PascalCase style (`StatusBadge`, `VerdictBadge`, `ResultBadge`, `FrontmatterChip`, `CustomBadge`) — flagged independently by two lenses. The Phase 1 prose claims alignment with the existing convention but the casing diverges. Resolution requires changing the data-testid values to kebab-case (`status-badge`, `verdict-badge`, `result-badge`, `frontmatter-chip`) in: Phase 1 tests; Phase 3/4/5/6 default + hard-coded values; Phase 7 dispatch tests; Phase 7 AC integration fixtures; the new `LibraryDocView.dispatch.test.tsx`.

#### Minor

- 🔵 **Architecture + Code-Quality**: Wrappers discard original frontmatter key casing — `StatusBadge` always passes `name="status"` so `Status: Accepted` becomes `aria-label="status: Accepted"`. Behaviour change from current code; not tested explicitly. Suggested resolution: add a test locking the new behaviour, or pass the original key through (`<StatusBadge name={key} value={value} />`).
- 🔵 **Code-Quality**: `formatChipValue` has no guard for null/undefined/circular objects — minor robustness gap if `FrontmatterChip` is used outside the dispatcher.
- 🔵 **Code-Quality**: Test name strings contain literal template tokens (`'attaches aria-label of "${key}: ${value}"'`) — readability nit; not interpolated.
- 🔵 **Code-Quality**: Three vocabulary-helper files have near-identical Set-lookup boilerplate — acknowledged trade-off; refactor candidate when a fourth vocabulary appears.
- 🔵 **Test-Coverage**: JSON aria-label CSS selector (`[aria-label="meta: {\"x\":1}"]`) may be fragile across selector parsers — use `screen.getByLabelText` instead.
- 🔵 **Test-Coverage + Correctness**: Round-trip invariant `normaliseValue(k) === k` is weaker than the original `/^[a-z]+$/` regex against coordinated bugs in `normaliseValue` + Set keys; consider keeping both assertions.
- 🔵 **Test-Coverage**: Route-level integration matrix only covers green path per surface — `REQUEST_CHANGES`, `fail`, `partial`, `COMMENT` not exercised at the route layer (unit coverage exists). Consider parametrising the dispatch tests.
- 🔵 **Correctness**: Empty-string `status:` / `verdict:` / `result:` is filtered before reaching the badge — work-item AC implies it should render neutral; document the layering or loosen the filter.
- 🔵 **Correctness**: Whitespace-only and empty-array values pass the filter and render visually-empty chips — same as current behaviour but uncovered by tests.
- 🔵 **Correctness**: Dispatcher key matching is case-folded + trimmed but not separator-normalised — asymmetric with value normalisation; add a comment.
- 🔵 **Correctness**: `COMMENT` is a known canonical token but lives in the fallback path; if a future merge accidentally adds it to a colour Set, the invariant test would not catch the regression.
- 🔵 **Standards**: `__SETS_FOR_TEST` test-only convention should be made explicit (JSDoc `@internal` comment or relocate to internal module).
- 🔵 **Standards**: Route-integration file name uses a `.dispatch.test.tsx` infix not used elsewhere — consider folding into `LibraryDocView.test.tsx` for consistency.
- 🔵 **Standards**: `normaliseValue` non-string-to-empty-string coercion is undocumented — add a JSDoc.
- 🔵 **Usability**: Prop name inconsistency — `Chip` uses `data-testid`, `FrontmatterChip` uses `testId` — pick one for the family.
- 🔵 **Usability**: Badge wrappers expose no `testId` override; multi-instance pages can't distinguish badges by testid.
- 🔵 **Usability**: `BadgeProps` dispatch contract is implicit and unexported.
- 🔵 **Compatibility**: External consumers outside the visualiser frontend not surveyed for the rename — single-line repo-wide grep note would close this.
- 🔵 **Compatibility**: `data-testid` attribute is unconditionally emitted (relies on React stripping `undefined`) — cosmetic forward-compat note; matches existing `aria-label` style.

### Assessment

The plan is now in **good shape for implementation**. The substantial concerns from Pass 1 (route ACs only manual, dispatch under-assertion, data-component invention, vocabulary cross-leakage, work-item AC drift) are all resolved. The one new major — PascalCase data-testid values — is a mechanical fix: rename the testid string values to kebab-case across the plan. No structural change required.

The remaining minors are a mix of:
- Genuinely new concerns surfaced by lenses examining the revised shape (PascalCase testids, wrapper key-casing loss, prop name inconsistency).
- Latent concerns the lenses caught while looking fresh (empty-string filter mismatch, COMMENT in fallback, BadgeProps unexported).
- Documentation gaps (selection rubric, vocabulary extension path, `__SETS_FOR_TEST` convention).

None blocks implementation. The PascalCase testid is the only finding I'd recommend addressing before kicking off Phase 1; the rest can be threaded through the implementation phases or deferred to a follow-up. Implementation can begin once the testid casing is reconciled.

---

## Re-Review (Pass 3) — 2026-05-23

**Verdict:** APPROVE

The Pass 2 sole major finding — PascalCase `data-testid` values diverging from the codebase's kebab-case convention — has been applied across the plan. All `data-testid` string values are now kebab-case (`status-badge`, `verdict-badge`, `result-badge`, `frontmatter-chip`, `custom-badge`); test-suite `describe(...)` labels remain PascalCase since they name the component, not the testid value. The plan is approved for implementation.

### Previously Identified Issues

- 🟢 **Code-Quality + Standards**: PascalCase `data-testid` values — **Resolved.** All testid string values in Phase 1 tests, Phase 3 (`FrontmatterChip` default), Phase 4/5/6 wrappers, Phase 7 dispatcher tests, Phase 7 AC integration fixtures, and `LibraryDocView.dispatch.test.tsx` are now kebab-case. `describe()` test-suite labels preserved as PascalCase (component names).

The remaining Pass 2 minors are accepted as known low-priority follow-ups:
- Wrapper key-casing normalisation in aria-label (behaviour change worth a test).
- Round-trip invariant complementary regex (defence-in-depth).
- Route-level integration matrix breadth (currently green-path only).
- Empty-string filter inconsistency vs badge fallback contract (document the layering).
- Prop name parity (`Chip` `data-testid` vs `FrontmatterChip` `testId`).
- BadgeProps unexported / `__SETS_FOR_TEST` `@internal` annotation / vocabulary extension recipe — all documentation gaps that can land alongside the next vocabulary or in a follow-up cleanup.

### Assessment

The plan is in good shape for implementation. The convention break has been corrected, the work item and plan are aligned, and no findings remain that block kicking off Phase 1. Implementation can begin.
