---
date: "2026-05-22T15:25:00+00:00"
type: work-item-review
producer: review-work-item
target: "work-item:0081"
work_item_id: "0081"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 4
status: complete
id: "0081-status-badge-component-review-1"
title: "0081-status-badge-component-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-22T15:25:00+00:00"
last_updated_by: Toby Clemson
---

## Work Item Review: 0081 — StatusBadge — Map Both Status and Verdict to Chip Tone

**Verdict:** COMMENT

Work item 0081 is structurally complete, well-populated, and tightly scoped to a single coherent concern: extending the chip-tone mapping so both `status` and `verdict` frontmatter keys resolve to coloured Chip variants. Acceptance Criteria are in clean Given/When/Then form covering both verdict vocabularies (validation `pass`/`fail` and plan-review `APPROVE`/`REVISE`/`REQUEST_CHANGES`/`COMMENT`) plus the neutral fallback. The work item is acceptable as-is but could be tightened — see the major and minor findings, especially around the title-vs-scope ambiguity and a few coverage gaps in the ACs.

### Cross-Cutting Themes

- **StatusBadge component vs. helper extension** (flagged by: clarity, scope) — Title and Summary frame the deliverable as a `StatusBadge` component (or wrapper), while Drafting Notes commit to extending `statusToChipVariant`/`FrontmatterChips`. Acceptance Criteria sit between the two framings. Both implementations could satisfy the ACs, leaving the shape of the deliverable ambiguous.
- **Verdict vocabulary churn / work-item-review coverage** (flagged by: scope, testability, dependency) — Requirements call for changes across plan-review, work-item-review, and validation detail pages, but ACs only exercise the first and third. Open Question 2 admits the work-item-review verdict vocabulary is unsettled (per 0066), so the third surface's coverage is genuinely conditional.
- **Case-sensitivity unresolved** (flagged by: clarity, dependency, testability) — Assumptions and Open Questions note that upstream normalisation may lowercase plan-review verdicts. No AC pins down the expected behaviour for off-case input, and no upstream module is named as the dependency that would clarify whether normalisation occurs.
- **Status set never enumerated** (flagged by: clarity, testability) — The final AC and Requirements refer to "status: existing set" without enumerating it. Only `Accepted → green` is exercised; regressions on `Draft`, `Merged`, etc. would not be caught.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Clarity**: Title says "StatusBadge" but body scopes the work as a helper extension
  **Location**: Summary / Title / Drafting Notes
  The title and Summary frame the deliverable as a `StatusBadge`-shaped component, but Requirements, Technical Notes, and Drafting Notes assume the work extends `statusToChipVariant`/`FrontmatterChips`. A reader cannot tell whether the deliverable is a new component or a helper change; an implementer could ship either and both could plausibly satisfy the ACs.

#### Minor

- 🔵 **Clarity**: "green / amber / red / neutral variants" never tied to named Chip variants
  **Location**: Requirements / Acceptance Criteria
  Colour names are used throughout but never mapped to the 0038 `Chip` primitive's actual variant identifiers (which may be `success`/`warning`/`danger`/`neutral` or similar, not the literal colour names). ACs cannot be verified mechanically without resolving this mapping.

- 🔵 **Clarity**: "The verdict vocabulary is not single-sourced" referent is unclear
  **Location**: Context
  Framing reads as a defect statement that may or may not need to be fixed by this work item. Reword to make clear this is descriptive context, not an additional requirement.

- 🔵 **Clarity**: First open question conflates API shape with collision safety
  **Location**: Open Questions
  Single function vs. two siblings is one decision; how to handle namespace collision between status and verdict is another. Split into two questions.

- 🔵 **Clarity**: "status: existing set" is undefined within the work item
  **Location**: Acceptance Criteria (final bullet)
  Verdict vocabularies are enumerated precisely; the status side is left to the reader. Either enumerate the canonical status values or link to the definition.

- 🔵 **Completeness**: Frontmatter uses `type` rather than `kind`
  **Location**: Frontmatter
  Drafting Notes acknowledge this and defer to a separate migration. Consider adding `kind: story` alongside `type: story` for forward compatibility, or link to the migration that owns the corpus-wide rename.

- 🔵 **Dependency**: Plan-review verdict-emission ordering treated as coordination, not blocker
  **Location**: Dependencies
  ACs 4–7 assert behaviour for the uppercase verdict set produced by 0005 (listed only as Coordinates-with). If 0005 has not shipped canonical verdict emission, those ACs can only be verified against synthetic fixtures, not live plan-review output.

- 🔵 **Dependency**: Case-normalisation coupling depends on an unidentified upstream layer
  **Location**: Assumptions
  The conditional dependency on an upstream normaliser is named but the owning module/work item is not, so the coupling cannot be coordinated.

- 🔵 **Dependency**: Validation page verdict emission not captured as an upstream assumption
  **Location**: Acceptance Criteria
  ACs 2–3 require validation pages to surface `verdict` through `FrontmatterChips`. If they do not currently, an upstream change is required that is not captured here.

- 🔵 **Scope**: Title and Summary frame a component; Drafting Notes pivot to a helper extension
  **Location**: Summary / Drafting Notes
  (See Major finding for primary statement of this issue — scope lens reinforces it from the unit-of-delivery angle: downstream consumer 0084 may depend on a name that doesn't exist.)

- 🔵 **Testability**: Status criterion samples only one value (Accepted)
  **Location**: Acceptance Criteria
  A regression in any other status value (e.g., Draft, In Progress, Done) would not be caught. Add a table or per-value criteria.

- 🔵 **Testability**: Case-sensitivity behaviour is unspecified for verification
  **Location**: Assumptions / Acceptance Criteria
  Two implementations could both claim to pass the ACs while differing on case-insensitive vs. fallback behaviour. Add an AC that pins down the expected behaviour for at least one off-case input.

- 🔵 **Testability**: "Apply across plan-review, work-item-review, and validation detail pages" is not directly covered by an AC
  **Location**: Acceptance Criteria
  Work-item-review is not exercised by any AC. A verifier could pass every listed AC while leaving that page rendering verdicts as neutral chips.

#### Suggestions

- 🔵 **Completeness**: Summary mixes user-story framing with implementation alternatives
  **Location**: Summary
  The parenthetical "(or introduce a `StatusBadge`-shaped wrapper…)" blurs the Summary. Move the alternative entirely into Drafting Notes or Open Questions.

- 🔵 **Scope**: "Apply across plan-review, work-item-review, and validation detail pages" touches three routes
  **Location**: Requirements / Acceptance Criteria
  Consider narrowing the story to the two confirmed verdict vocabularies and noting work-item-review coverage follows once 0066 lands, or accept the risk explicitly in Assumptions.

### Strengths

- ✅ Verdict vocabularies (validation `pass`/`fail` and plan-review `APPROVE`/`REVISE`/`REQUEST_CHANGES`/`COMMENT`) are explicitly enumerated and used consistently across Context, Requirements, and Acceptance Criteria.
- ✅ ACs use Given/When/Then form with named actors (FrontmatterChips, validation detail page, plan-review page, chip-tone helper) and observable outcomes (named chip variants).
- ✅ Drafting Notes proactively call out both the verdict-vocabulary reconciliation and the StatusBadge-vs-helper interpretation — the reader isn't left to guess intent.
- ✅ All expected sections for a Story kind are present and substantive (Summary, Context, Requirements, ACs, Open Questions, Dependencies, Assumptions, Technical Notes, Drafting Notes, References).
- ✅ Dependencies are well-mapped: 0038 as Blocked-by, 0084 as Blocks, 0005/0066 as Coordinates-with, each with a clear rationale.
- ✅ Single unified purpose — every requirement and AC serves the one goal of mapping `status` and `verdict` to coloured chip tones; appropriately sized as a story.
- ✅ Builds on 0038's existing Chip primitive with no new variant required, keeping scope tight.
- ✅ The final AC targets the chip-tone helper directly as a unit-testable function, complementing the page-level integration criteria.

### Recommended Changes

1. **Resolve the StatusBadge-vs-helper ambiguity in Summary and Title** (addresses: clarity major finding, scope minor finding)
   Pick one canonical deliverable shape. Either rename the work item to something like "Extend chip-tone mapping to cover verdict values" and remove the "StatusBadge wrapper" alternative from the Summary, or commit Requirements to producing a `StatusBadge` component. Drafting Notes already chose helper-extension — promote that choice into Summary and remove the alternative.

2. **Pin down the 0038 Chip variant names in Requirements or Technical Notes** (addresses: clarity "green/amber/red/neutral" finding)
   Add a table or inline mapping: `green → Chip variant="…"`, etc. — so the ACs are mechanically verifiable.

3. **Resolve case-sensitivity Open Question and add a corresponding AC** (addresses: clarity Open Questions split, dependency case-normalisation, testability case-sensitivity)
   Decide whether the helper is case-sensitive or normalises input, name the upstream module if normalisation is its responsibility, and add an AC fixing the expected behaviour for at least one off-case input.

4. **Resolve work-item-review vocabulary scope** (addresses: scope work-item-review surface, testability missing AC, scope suggestion)
   Either add an AC exercising the work-item-review page (conditional on Open Question 2's resolution) or narrow the Requirements/ACs explicitly to plan-review and validation, deferring work-item-review to a follow-up once 0066 settles.

5. **Enumerate the existing status set** (addresses: clarity "existing set", testability status coverage)
   Either list the canonical status values and their expected chip variants in Requirements/ACs, or reference `statusToChipVariant`'s current mapping as the authoritative source-of-truth that must be preserved.

6. **Clarify plan-review verdict-emission sequencing** (addresses: dependency 0005 ordering)
   Either promote 0005 from Coordinates-with to Blocked-by, or state explicitly in Dependencies that plan-review ACs are verified against fixtures and live integration follows 0005.

7. **Confirm validation page verdict surfacing** (addresses: dependency validation emission)
   Add an Assumption confirming the validation detail page already renders `verdict` through `FrontmatterChips`, or add the validation-frontmatter emitter as a coordination dependency.

8. **Minor polish** (addresses: clarity Context referent, completeness Summary mixing, completeness `kind` field)
   Reword "The verdict vocabulary is not single-sourced" to read as descriptive context; move the wrapper alternative out of Summary; add `kind: story` alongside `type: story` (or link the migration that owns the rename).

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is largely clear: the actor (the chip-tone helper / FrontmatterChips) and outcomes (specific verdict→variant mappings) are explicit, and the canonical verdict vocabularies are enumerated. The main clarity issues are a mismatch between the title's `StatusBadge` naming and the body's `FrontmatterChips`/helper framing, an ambiguous primary deliverable (extend helper vs. introduce a wrapper component), and a few referent/term issues around `Chip variant` colour names and the meaning of "the 0038 primitive".

**Strengths**:
- Verdict vocabularies explicitly enumerated and consistent across sections.
- ACs use Given/When/Then with named actors.
- Drafting Notes proactively call out reconciliations.
- Assumptions section names the case-sensitivity concern explicitly.

**Findings**:
- **major / high — Title says "StatusBadge" but body scopes the work as a helper extension** (Summary / Title / Drafting Notes): Title frames a component; Drafting Notes commit to extending `statusToChipVariant`/`FrontmatterChips`. ACs sit between both framings, so two materially different implementations could both pass.
- **minor / high — "green / amber / red / neutral variants" never tied to named Chip variants** (Requirements / ACs): 0038 variant identifiers not given.
- **minor / medium — "The verdict vocabulary is not single-sourced" referent is unclear** (Context): Reads as defect statement; reword as descriptive.
- **minor / medium — First open question conflates two decisions** (Open Questions): API shape vs. collision safety should be split.
- **minor / medium — "status: existing set" is undefined within the work item** (ACs final bullet): Enumerate or link.

### Completeness

**Summary**: Work item 0081 is structurally complete and well-populated across all expected sections for a Story kind. Frontmatter includes the required fields. The story identifies the user, explains motivation, and offers eight specific verifiable criteria.

**Strengths**:
- Eight Given/When/Then ACs covering both vocabularies and the fallback.
- Context clearly explains the prototype-vs-current-app gap.
- User and benefit are identified.
- All expected sections substantively populated.
- Frontmatter intact with type/status/priority/title/author/tags/parent.

**Findings**:
- **minor / medium — Frontmatter uses `type` rather than `kind`** (Frontmatter): Drafting Notes defer to a separate migration; add `kind: story` for forward compatibility or link the migration.
- **suggestion / low — Summary mixes user-story framing with implementation alternatives** (Summary): Move wrapper alternative to Drafting Notes/Open Questions.

### Dependency

**Summary**: Dependencies are well-mapped overall: 0038 blocker is explicit, 0084 downstream consumer is named, and 0005/0066 are captured under Coordinates-with. The main gap is around verdict-emission ordering — the ACs assume plan-review pages already emit the canonical uppercase set (from 0005) and validation pages already emit `pass`/`fail`, treated as coordination rather than hard ordering.

**Strengths**:
- 0038 named as Blocked-by with clear rationale.
- 0084 captured under Blocks.
- Coordination dependencies distinguished from hard blockers.
- Precise upstream artefacts named in Technical Notes.

**Findings**:
- **minor / medium — Plan-review verdict-emission ordering treated as coordination, not blocker** (Dependencies): ACs 4–7 depend on 0005's canonical verdict emission. Promote to Blocked-by or note fixture-based verification.
- **minor / medium — Case-normalisation coupling depends on an unidentified upstream layer** (Assumptions): The upstream module is not named.
- **minor / low — Validation page verdict emission not captured as an upstream assumption** (ACs): ACs 2–3 presuppose validation pages surface `verdict` via `FrontmatterChips`.

### Scope

**Summary**: The work item is well-scoped to a single coherent concern. Requirements, ACs, and Summary are tightly aligned, and the work is appropriately sized for a story. One mild scope tension exists between the title (`StatusBadge` as a component) and the Drafting Notes' chosen implementation path (extending `FrontmatterChips`'s helper).

**Strengths**:
- Single unified purpose; every requirement serves the one goal.
- Scope boundaries explicit (builds on 0038's existing Chip primitive).
- Two distinct verdict vocabularies reconciled under one mapping.
- Drafting Notes anticipate ambiguity without expanding scope.
- Downstream/coordination items surfaced as an atomic increment.

**Findings**:
- **minor / medium — Title and Summary frame a component; Drafting Notes pivot to a helper extension** (Summary / Drafting Notes): Unit-of-delivery shape is ambiguous; 0084 may depend on a name that does not exist.
- **suggestion / medium — "Apply across plan-review, work-item-review, and validation detail pages" touches three routes** (Requirements / ACs): Work-item-review vocabulary is unsettled per 0066. Consider narrowing scope or accepting risk explicitly.

### Testability

**Summary**: ACs are in clear Given/When/Then format with specific frontmatter inputs and named chip-variant outcomes, making them highly verifiable. The criteria cover both verdict vocabularies and the fallback case, with gaps around the status criterion's input coverage, the work-item-review surface, and case-sensitivity behaviour.

**Strengths**:
- Each AC follows explicit Given/When/Then structure.
- Both verdict vocabularies plus fallback are covered.
- Final AC targets the helper directly as a unit-testable function.
- Outcomes stated as observable chip variants, not implementation instructions.

**Findings**:
- **minor / high — Status criterion samples only one value (Accepted)** (ACs): Regression on other status values would not be caught.
- **minor / high — Case-sensitivity behaviour is unspecified for verification** (Assumptions / ACs): Two compliant implementations could differ materially.
- **minor / medium — "Apply across plan-review, work-item-review, and validation detail pages" is not directly covered by an AC** (ACs): Work-item-review is not exercised; passing all ACs would not catch neutral chips there.

## Re-Review (Pass 2) — 2026-05-22

**Verdict:** REVISE

The rewrite resolved every finding from pass 1 cleanly — title/scope ambiguity is gone, the decomposition is committed, case-sensitivity and verdict-vocabulary ordering are pinned, and the `red`-variant gap was surfaced honestly in the work item itself. However, the surfacing created new majors: the blocking `red`-variant follow-up has no tracked WI, two ACs assert against the unavailable `red` variant (currently unverifiable), and two other ACs delegate enumeration to external state or to unresolved Open Questions. Net: the work item is meaningfully better but has crossed the major-finding threshold and needs one more pass focused on the `red` predecessor and AC tightening.

### Previously Identified Issues

- 🟡 **Clarity**: Title says "StatusBadge" but body scopes the work as a helper extension — **Resolved** (work item commits to a concrete `StatusBadge` component via full decomposition).
- 🔵 **Clarity**: "green / amber / red / neutral variants" never tied to named Chip variants — **Partially resolved** (now uses 0038's variant names, but exposed that `red` is not part of 0038's shipped set).
- 🔵 **Clarity**: "The verdict vocabulary is not single-sourced" referent unclear — **Resolved** (reworded as descriptive Context).
- 🔵 **Clarity**: First open question conflates API shape with collision safety — **Resolved** (OQ removed; component-based design makes it moot).
- 🔵 **Clarity**: "status: existing set" undefined — **Partially resolved** (now references `statusToChipVariant` as canonical source; full set still not enumerated inline).
- 🔵 **Completeness**: Frontmatter uses `type` rather than `kind` — **Resolved** (`kind: story` added alongside `type: story`).
- 🔵 **Completeness**: Summary mixes user-story framing with implementation alternatives — **Resolved** (wrapper alternative removed; Summary commits to decomposition direction).
- 🔵 **Dependency**: Plan-review verdict-emission ordering treated as coordination, not blocker — **Resolved** (0005 promoted from Coordinates-with to Blocked-by).
- 🔵 **Dependency**: Case-normalisation coupling depends on unidentified upstream layer — **Resolved** (case-insensitive lookup eliminates the conditional coupling).
- 🔵 **Dependency**: Validation page verdict emission not captured as upstream assumption — **Resolved** (added as an explicit Assumption).
- 🔵 **Scope**: Title/Summary frame a component; Drafting Notes pivot to helper extension — **Resolved**.
- 🔵 **Scope**: Three-route reach should be narrowed or risk accepted — **Resolved** (kept three surfaces; added conditional work-item-review AC plus Open Question).
- 🔵 **Testability**: Status criterion samples only one value — **Partially resolved** (added a no-regression AC, but it delegates enumeration to the external `statusToChipVariant` file — see new finding).
- 🔵 **Testability**: Case-sensitivity behaviour unspecified — **Resolved** (case-insensitive AC added).
- 🔵 **Testability**: Work-item-review surface not covered by any AC — **Resolved** (conditional AC added, though see new finding on its testability).

### New Issues Introduced

- 🟡 **Dependency**: Red-variant follow-up is a blocker but has no work item ID — Requirements/Open Questions say 0081 is blocked on extending 0038 with a `red` variant, but the follow-up has no WI number. The most material upstream coupling is invisible to planning.
- 🟡 **Testability**: `red` variant precondition leaves `verdict: fail` and `verdict: REQUEST_CHANGES` ACs unverifiable — those ACs assert a `red` variant that 0038 does not ship. Cannot pass until the predecessor lands.
- 🟡 **Testability**: Work-item-review AC is conditional on an unresolved Open Question — the "assumed for now per Open Questions" wording leaves the test condition ambiguous (must-satisfy vs. provisional).
- 🟡 **Testability**: "Any other status value currently handled" AC delegates enumeration to `statusToChipVariant` — the AC's input set is not pinned; will silently expand/shrink with the external file.
- 🔵 **Clarity**: "extends" vs "composes" for `StatusBadge` is left as a dual interpretation (Requirements says "extends", Technical Notes says "extends/composes") — pick one verb.
- 🔵 **Clarity**: Conditional work-item-review AC wording is ambiguous about whether it is must-satisfy or tentative.
- 🔵 **Dependency**: Work-item-review AC has an implicit ordering dependency on 0066 (Coordinates-with) that should be stated.
- 🔵 **Dependency**: Consumer page surfaces (validation, plan-review, work-item-review) are not captured as coordinated work — no signal whether they're stable or under concurrent modification.
- 🔵 **Testability**: "COMMENT or any unmapped value" mixes a concrete case with an unbounded clause — split, and enumerate 2–3 representative unmapped inputs.
- 🔵 **Testability**: "no `status`/`verdict`-specific branching in its code path" is an implementation-detail assertion, not an observable behaviour — reframe as a behavioural contract.
- 🔵 **Testability**: First decomposition AC lacks a concrete multi-key fixture — specify the example document.
- 🔵 **Suggestion (Clarity)**: Anchor "0038" as `0038 (the Chip primitive)` on first use rather than relying on Dependencies to disambiguate later.
- 🔵 **Suggestion (Scope)**: Red-variant work could be a separable predecessor WI (or 0081 could commit to remapping onto an existing variant) — currently straddling primitive-library and consumer concerns.

### Assessment

The previous pass produced clear direction; the rewrite faithfully captured that direction and exposed a real upstream gap (the missing `red` variant) that had been hiding under an incorrect "no new variant required" assumption. To clear REVISE, one more iteration should:

1. **Create the `red`-variant predecessor WI** (or commit in Requirements to remapping onto an existing variant) and list it explicitly under Blocked-by.
2. **Inline the canonical status mapping** in ACs rather than delegating to `statusToChipVariant`, so the AC is self-contained.
3. **Pick a single verb** ("extends" or "composes") for `StatusBadge`'s relationship to `FrontmatterChip` and use it uniformly.
4. **Tighten the work-item-review AC** — either upgrade 0066 to a partial blocker for that AC, or reframe the AC as a contract with named fixtures.
5. **Split the COMMENT/unmapped AC** into two — one concrete, one with enumerated edge inputs.
6. **Replace the "no branching" decomposition AC** with a behavioural assertion (e.g. `FrontmatterChip` rendered directly with `status: Accepted` produces a neutral chip).

After these are addressed, the work item should be ready for implementation.

## Re-Review (Pass 3) — 2026-05-22

**Verdict:** COMMENT

The pass-2 major findings are all resolved. The `red`-variant gap turned out to be a misread — 0038 ships six variants including `red` and `violet`, per the 0038 plan's Desired End State. Status mapping is now inlined as a canonical table with per-bucket ACs; the work-item-review AC has explicit Option-B framing; "extends" was replaced with "composes" throughout; the COMMENT/unmapped AC was split. Remaining findings are minor polish — no major issues, no criticals. Acceptable for implementation; the minor items below are optional improvements.

### Previously Identified Issues

- 🟡 **Dependency**: Red-variant follow-up blocker has no WI ID — **Resolved** (red already ships in 0038; OQ and "blocked by extension" framing removed).
- 🟡 **Testability**: `red` precondition leaves `fail`/`REQUEST_CHANGES` ACs unverifiable — **Resolved** (red is shipped; ACs are verifiable).
- 🟡 **Testability**: Work-item-review AC conditional on unresolved OQ — **Partially resolved** (Option B framing applied; ambiguity reduced but testability lens still flags residual conditional wording — see new finding).
- 🟡 **Testability**: "Any other status value" delegates enumeration to external file — **Resolved** (canonical status mapping now inlined as a table; per-bucket ACs cover each tone).
- 🔵 **Clarity**: "extends" vs "composes" dual interpretation — **Resolved** ("composes" used uniformly).
- 🔵 **Dependency**: Work-item-review AC implicit ordering on 0066 — **Partially resolved** (AC and Drafting Notes name the supersession path; Dependencies entry could be more explicit — see new finding).
- 🔵 **Testability**: "COMMENT or any unmapped value" mixed concrete with unbounded — **Resolved** (split into one concrete AC + one enumerated unmapped-value AC).
- 🔵 **Testability**: "no branching" was implementation-detail — **Resolved** (replaced with behavioural AC: `FrontmatterChip` with `status: Accepted` renders neutral).
- 🔵 **Testability**: First decomposition AC lacked concrete fixture — **Resolved** (named fixture: `status: Accepted`, `verdict: pass`, `priority: medium`, `tags: [design, frontend]`).
- 🔵 Suggestion **Clarity**: Anchor "0038" on first use — **Resolved** ("0038 (the `Chip` primitive)" used on first reference).

### New / Residual Issues

- 🔵 **Clarity**: `absent` status value is ambiguous — literal string `"absent"` vs missing-key sentinel? Same applies to "dates, author names, unknown" in the neutral row.
- 🔵 **Clarity**: Relationship between `FrontmatterChips` (existing name) and the new "chip-list renderer" is implicit — renamed, replaced, or co-existing?
- 🔵 **Clarity** / **Testability**: Work-item-review AC still mixes assertion with the supersession contingency in a single bullet. Could be split — move the "if 0066…" clause into Open Questions and leave the AC unconditional, or restate as `Given the work-item-review page emits the plan-review vocabulary, …`.
- 🔵 **Dependency**: Validation-page verdict-emission assumption is not surfaced in Dependencies — if validation pages don't already emit `verdict` at the top level, there's no named follow-up to escalate to.
- 🔵 **Dependency**: 0066 ordering implication could be inlined into the Dependencies "Coordinates with" entry rather than only being implied by Open Questions and the AC parenthetical.
- 🔵 **Testability**: First decomposition AC asserts "first two via `StatusBadge`, latter two via `FrontmatterChip`" — that's internal-dispatch language. Reframe as observable rendered behaviour (variants applied, or a `data-component` test hook).
- 🔵 **Testability**: Unmapped-value AC covers `verdict` only; add a parallel `status: SomeUnknownValue` → neutral AC to cover the Requirements claim that "any unmapped value falls back to neutral" for status too.
- 🔵 **Testability**: Chip-list renderer's ordering contract (e.g. "frontmatter source order") is not captured by an AC — Requirements assign it ordering responsibility but no test oracle exists.
- 🔵 **Suggestion** (Clarity): "AC" acronym used without expansion (minor polish only).
- 🔵 **Suggestion** (Scope): Decomposition + verdict-tone are bundled — separable in theory; acceptable here given the natural seam, but worth noting in Drafting Notes as a deliberate choice.

### Assessment

The work item is in good shape and ready for implementation. The remaining items are minor refinements:

- **Optional cleanups before draft → ready**:
  - Disambiguate `absent` / "dates, author names, unknown" in the status table.
  - State whether the chip-list renderer renames, replaces, or co-exists with `FrontmatterChips`.
  - Split the work-item-review AC into an unconditional assertion plus a separate Open-Question entry for the supersession path.
  - Add the symmetric `status` unmapped-value AC and a chip-list-renderer ordering AC.
  - Inline the 0066 ordering implication into the Dependencies "Coordinates with" entry.

None of these block implementation — they tighten verification and reduce reader friction. The work item has cleared REVISE and is acceptable as-is.

## Approval (Pass 4) — 2026-05-22

**Verdict:** APPROVE

All pass-3 residual polish items have been applied to the work item:

- `absent` and "any other value" disambiguated in the status table (literal string matches; non-status-string values fall through to neutral).
- Chip-list renderer's relationship to `FrontmatterChips` made explicit — the renderer is the refactored `FrontmatterChips` in place; name and call sites preserved.
- Work-item-review AC reframed as an unconditional assertion; the supersession contingency now lives only in Open Questions.
- Validation-page verdict emitter added under Coordinates-with so the conditional upstream change is visible to planners.
- 0066 ordering implication inlined into the Coordinates-with entry (both lands-first and lands-after outcomes named).
- First decomposition AC reframed as observable behaviour (variants in source order); dispatch contract exposed via a `data-component` test hook.
- Status fallback AC added (`status: SomeUnknownValue` / date / author-shaped → neutral).
- Chip-list source-order AC added.

The work item is approved for implementation.
