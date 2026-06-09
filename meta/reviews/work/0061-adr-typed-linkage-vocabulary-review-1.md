---
date: "2026-05-19T23:57:41+00:00"
type: work-item-review
producer: review-work-item
target: "work-item:0061"
review_number: 1
verdict: COMMENT
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 2
status: complete
id: "0061-adr-typed-linkage-vocabulary-review-1"
title: "0061-adr-typed-linkage-vocabulary-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-19T23:57:41+00:00"
last_updated_by: Toby Clemson
---

## Work Item Review: ADR: Typed Linkage Vocabulary

**Verdict:** REVISE

The work item is structurally complete and well-scoped — a single ADR to be produced with clearly bounded decisions, deferring producer-skill, template, and migration work to sibling stories. Across the five lenses, clarity surfaced two major issues that risk silent contradiction with the just-accepted ADR-0033 (the status of `parent` as a linkage key, and the quoting shape of composite references). The remaining findings are minor or suggestions, mostly about tightening acceptance criteria into checklist-explicit form and making implicit downstream couplings explicit.

### Cross-Cutting Themes

- **Relationship to ADR-0033 base schema** (flagged by: clarity) — Two findings centre on how this ADR coexists with ADR-0033: `parent` is already in the base schema, and references must follow the quoted-string identity contract. The ADR-to-be-produced needs to make these boundaries explicit, and the work item should set up that expectation.
- **Acceptance criteria phrased as documentation goals rather than checkable bars** (flagged by: testability, clarity) — Several criteria require something to be "documented", "decided", or "stated" without naming the minimum content. A verifier could pass them against thin or trivially-decided ADR content.
- **Downstream consumers identified by category, not by ID** (flagged by: dependency) — Once the sibling stories under 0057 are written, this work item's `Blocks` entry will need backfilling so the gating relationship is discoverable from either side.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Clarity**: `parent` listed as a typed-linkage key conflicts with ADR-0033 treating it as base frontmatter
  **Location**: Requirements
  The Requirements section lists `parent` as one of the typed linkage keys this ADR will define, but ADR-0033 already lists `parent` in the work-item frontmatter (and the work item's own frontmatter uses `parent: "0057"`). A reader cannot tell whether this ADR is re-defining a key in the base schema, lifting `parent` out of work-item-specific extras into the generic vocabulary, or merely cataloguing it for completeness.

- 🟡 **Clarity**: Reference-shape example `doc-type:id` contradicts ADR-0033's quoted-string identity contract
  **Location**: Requirements
  Requirements give `plan:0042` as an example of the `doc-type:id` reference shape, but the same bullet states 'Referenced `id` values follow ADR-0033's identity-value shape contract (quoted YAML strings).' It is unclear whether the composite reference is `plan:0042` (bare), `"plan:0042"` (quoted composite), or `plan:"0042"` (quoted id portion only).

#### Minor

- 🔵 **Clarity**: 'the seven above' miscounts the listed keys
  **Location**: Open Questions
  Open Questions asks about keys 'beyond the seven above', but Requirements enumerates eight key entries (or arguably more once bidirectional pairs are counted). The miscount is small but the intent — whether `parent` is in or out of the count — is genuinely ambiguous.

- 🔵 **Clarity**: 'this ADR' shifts referent between the work item and the artifact-to-be-produced
  **Location**: Summary / Acceptance Criteria
  Several lines use 'this ADR' to mean the deliverable, but lines like 'The ADR cross-references ADR-0033' in Acceptance Criteria sit alongside frontmatter that describes the work item, where the antecedent is implicit.

- 🔵 **Clarity**: `target` semantics are under-specified for non-review consumers
  **Location**: Requirements
  The `target` bullet says 'what this artifact is *about* (reviews, validations)'. The parenthetical could be read as exhaustive ('only reviews/validations may use it') or as examples ('primary use'). The ADR author will inherit the ambiguity.

- 🔵 **Clarity**: 'qualifier mechanism' for design-gap inventory keys is undefined
  **Location**: Requirements
  The final Requirements bullet mentions a 'qualifier mechanism' without prior definition, and Open Questions later refers to an 'optional qualifier (e.g. tag/role)'. A reader cannot evaluate the trade-off because one side of it ('qualifier mechanism') is undefined.

- 🔵 **Dependency**: Downstream 'Blocks' entry names categories, not specific work item IDs
  **Location**: Dependencies
  'producer-skill / template / migration stories under 0057' is category-level. Once those sibling stories exist, this Blocks line should be backfilled with explicit IDs so the gating relationship is discoverable from either direction.

- 🔵 **Testability**: Verification of 'each typed linkage key' criterion lacks an explicit key list
  **Location**: Acceptance Criteria
  AC #1 says the ADR must define 'each typed linkage key, its cardinality, and its semantics' without naming the expected set. A verifier must cross-reference Requirements to know what 'each' covers.

- 🔵 **Testability**: 'States which forms are valid where' is open-ended
  **Location**: Acceptance Criteria
  The reference-shape AC requires the ADR to 'state which forms are valid where' without defining the axis (per key? per artifact type? per context?). A single sentence could claim satisfaction.

- 🔵 **Testability**: Type-pair semantic table criterion lacks a minimum content bar
  **Location**: Acceptance Criteria
  'The ADR documents the (source-type, key, target-type) → semantic-label table' is verifiable in principle but does not state what rows the table must contain at minimum.

#### Suggestions

- 🔵 **Dependency**: Sibling work item 0060 is referenced in ADR-0033 but not acknowledged here
  **Location**: Dependencies
  0060 produced the supplementing base-schema ADR; from 0061's side the relationship is currently implicit (mediated through ADR-0033). Could be added under Dependencies as 'Related' for traceability.

- 🔵 **Dependency**: Open question about additional keys may imply unnamed consumer coupling
  **Location**: Open Questions
  The `reviews:` question implies coupling to review skills, validation skills, and the future visualiser graph. Naming the affected consumers would make the resolution path explicit.

- 🔵 **Scope**: Design-gap inventory key disposition is adjacent to the core vocabulary decision
  **Location**: Requirements
  The design-gap question is genuinely contingent on the generic vocabulary, but the resulting ADR should note explicitly that this is a corollary decision rather than co-equal vocabulary.

- 🔵 **Testability**: Disposition criterion does not constrain which disposition is valid
  **Location**: Acceptance Criteria
  Could be strengthened to require a recorded rationale, not just a recorded decision.

- 🔵 **Testability**: Open questions are not tied to acceptance criteria
  **Location**: Open Questions
  An AC such as 'The ADR explicitly resolves or defers each open question listed in this work item, with rationale' would prevent silent omission.

### Strengths

- ✅ Every standard section is present and substantively populated — no empty placeholders.
- ✅ Each linkage key is named with a cardinality and a one-line semantic gloss; `derived_from` is explicitly flagged as a list.
- ✅ Bidirectional-pair handling is explained twice (Requirements and Technical Notes) with consistent reasoning, and the rationale (ADR-0033's immutability rule forces single-direction) is recorded.
- ✅ Drafting Notes captures the deferred decisions and the reframing of "implicit inference" into the (source-type, key, target-type) → semantic-label table — readers see which choices are intentionally open.
- ✅ ADR-0033 is named explicitly as the upstream 'Builds on' dependency, with the specific facets (`id` field, identity-value shape, immutability rule) called out — a high-quality upstream coupling capture.
- ✅ Scope is tight: all requirements converge on producing one ADR with a bounded set of decisions; producer-skill, template, and migration work is correctly deferred.
- ✅ Type (`task`) is appropriate; parentage to epic 0057 is explicit; frontmatter is complete with recognised values.

### Recommended Changes

1. **Clarify `parent`'s status relative to ADR-0033** (addresses: `parent` listed as a typed-linkage key conflicts with ADR-0033)
   In the `parent` bullet under Requirements, state explicitly whether this ADR (a) generalises `parent` from work-item-specific to corpus-wide, (b) inherits it from ADR-0033 and only catalogues it here, or (c) re-defines it. Reflect the same choice in the body H1 framing if applicable.

2. **Pin the reference-shape quoting example to ADR-0033's contract** (addresses: Reference-shape example contradicts identity contract)
   Replace `plan:0042` with the quoted form the ADR is expected to adopt (e.g. `"plan:0042"`), or explicitly state that the quoting rule applies to the id substring only. Either choice resolves the ambiguity.

3. **Tighten acceptance criteria into checklist-explicit form** (addresses: 'each typed linkage key' lacks explicit list; 'states which forms are valid where' is open-ended; type-pair table lacks minimum content bar; disposition criterion does not require rationale)
   - AC #1: enumerate the keys the ADR must cover.
   - AC #4: name the axis (per key? per artifact type? per context?) along which validity must be stated.
   - AC #2: state a minimum content bar for the type-pair table (e.g. "at least one row per linkage key").
   - AC #5: require a recorded rationale, not just a recorded decision.

4. **Fix the count and define 'qualifier mechanism'** (addresses: 'the seven above' miscount; 'qualifier mechanism' undefined)
   Either restate the open question as "beyond those listed above" or fix the count. Briefly describe what a qualifier looks like in the Requirements bullet (e.g. `{id: "...", role: current_inventory}`).

5. **Disambiguate 'this ADR' references in Acceptance Criteria** (addresses: 'this ADR' shifts referent)
   On first reference within Acceptance Criteria, name the deliverable explicitly ("The new ADR (the deliverable of this task)…") and use 'the ADR' for subsequent references.

6. **Sharpen `target` semantics** (addresses: `target` semantics under-specified)
   State whether `target` is open-domain or restricted, and replace the parenthetical with either 'primary use:' or 'used only by:' accordingly.

7. **Add an acceptance criterion that ties the Open Questions to the deliverable** (addresses: open questions not tied to acceptance criteria)
   Add: "The ADR explicitly resolves or defers each open question listed in this work item, with rationale recorded."

8. **Backfill `Blocks` IDs and note affected consumers** (addresses: 'Blocks' entry names categories; open question about additional keys may imply unnamed consumer coupling)
   Lower-priority: when sibling stories under 0057 are created, return to backfill the Blocks line with explicit IDs. Optionally annotate the `reviews:` open question with affected consumers (review skills, validate-plan, future visualiser-graph epic).

9. **Optional: add 0060 as Related under Dependencies** (addresses: sibling 0060 not acknowledged)
   Minor traceability improvement; current implicit linkage via ADR-0033 is acceptable.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is generally clear, with well-defined linkage keys, consistent terminology, and explicit cross-references to ADR-0033. However, there is internal inconsistency around the `parent` key, the `id` reference shape contradicts ADR-0033's identity-value contract, and a few referents introduce ambiguity that a reader without author context would have to resolve.

**Strengths**:
- Each linkage key is named, given a cardinality, and accompanied by a one-line semantic gloss.
- Bidirectional-pair handling is explained consistently across Requirements and Technical Notes.
- Drafting Notes captures decisions deferred to the ADR (reference shape, design-gap key disposition).
- References disambiguates roles (parent epic 0057, base-schema ADR-0033, related 0040).

**Findings**: 2 major, 4 minor (see consolidated findings above).

### Completeness

**Summary**: The work item is structurally complete for a task-type ADR-production item: all expected sections are present and substantively populated. Frontmatter contains type, status, priority, parent, and tags. Type-appropriate content is well-articulated, and the seven acceptance criteria map cleanly back to the requirements.

**Strengths**:
- Every standard section is present and substantively populated.
- Acceptance Criteria contains seven specific items that map directly to requirements.
- Context explains motivation and cites the prior decision that scopes this work.
- Frontmatter is complete with recognised values.
- Type-specific content for a task is clear.

**Findings**: None.

### Dependency

**Summary**: The work item captures its primary upstream dependency on ADR-0033 explicitly and lists the categories of downstream consumers. External and cross-team couplings are not relevant. The main dependency-mapping weakness is that downstream 'Blocks' entries are stated only as categories rather than as enumerated work item IDs.

**Strengths**:
- ADR-0033 is named with the specific facets being depended on (`id` field, identity-value shape, immutability rule).
- Parent epic 0057 is named and reinforced via the frontmatter `parent` field.
- Downstream blocking is acknowledged at the category level.
- Visualiser-graph epic correctly treated as future/deferred.
- Cross-references between Requirements, Acceptance Criteria, and Technical Notes consistently point back to ADR-0033.

**Findings**: 1 minor, 2 suggestions (see consolidated findings above).

### Scope

**Summary**: The work item is a well-scoped, coherent task to produce a single ADR defining the typed linkage vocabulary. All requirements serve one unified purpose; boundaries are explicit; and the task complements ADR-0033 cleanly. Sizing is appropriate for a 'task' type.

**Strengths**:
- All requirements and acceptance criteria converge on a single deliverable.
- Scope boundaries are explicit and well-drawn.
- Relationship to ADR-0033 is clearly bounded.
- Type (`task`) is appropriate.
- Parentage to epic 0057 is explicit.

**Findings**: 1 suggestion (see consolidated findings above).

### Testability

**Summary**: The work item's acceptance criteria are mostly deliverable-oriented and verifiable by inspecting the produced ADR. There are a few criteria where verification leans on the verifier's judgement of whether a topic is 'documented' versus genuinely decided, and one criterion that uses 'where' qualifiers that admit interpretation.

**Strengths**:
- Acceptance criteria enumerate concrete artefacts and decisions the ADR must contain.
- Criteria align well with the Requirements section.
- `derived_from` list-shape decision is isolated and independently verifiable.
- Cross-reference to ADR-0033 is itself an acceptance criterion.

**Findings**: 3 minor, 2 suggestions (see consolidated findings above).

## Re-Review (Pass 2) — 2026-05-19T23:57:41+00:00

**Verdict:** COMMENT

Work item is acceptable but could be improved — see new findings below.

### Previously Identified Issues

#### Major (Pass 1)
- 🟡 **Clarity**: `parent` listed as typed-linkage key conflicts with ADR-0033 — **Resolved**. The `parent` bullet now states explicitly that this ADR owns the corpus-wide vocabulary semantic while ADR-0033 lists the field by name on work items. (A minor residual referent ambiguity around "Defined here" is flagged as a new finding below.)
- 🟡 **Clarity**: Reference-shape example contradicts quoted-string identity contract — **Resolved**. The Requirements bullet now uses `"plan:0042"` and explicitly pins "the whole reference is a single quoted YAML string". Drafting Notes records the pinning.

#### Minor (Pass 1)
- 🔵 **Clarity**: "the seven above" miscounts — **Resolved**. Replaced with "those listed above".
- 🔵 **Clarity**: "this ADR" shifts referent — **Partially resolved**. Acceptance Criteria now names "the new ADR (the deliverable of this task)" on first reference, but Context and Requirements still use "this ADR" in places where a reader has to resolve the referent from context. Re-flagged as a smaller residual finding below.
- 🔵 **Clarity**: `target` semantics under-specified — **Resolved**. Now explicitly open-domain with reviews/validations as primary use.
- 🔵 **Clarity**: "qualifier mechanism" undefined — **Resolved**. Concrete example (`{ref: ..., role: ...}`) inlined.
- 🔵 **Dependency**: Blocks names categories not IDs — **Partially resolved**. Backfill obligation noted; reviewer suggests stronger fix (placeholder per-category line + named actor). Re-flagged as a minor below.
- 🔵 **Testability**: AC #1 lacks explicit key list — **Resolved**. AC #1 now enumerates the seven keys.
- 🔵 **Testability**: "states which forms are valid where" open-ended — **Resolved**. AC #4 now names the axis (per linkage key, producers/consumers).
- 🔵 **Testability**: Type-pair table lacks content bar — **Resolved**. AC #2 now requires "at least one row per linkage key". (Reviewer suggests strengthening further — see new minor below.)

#### Suggestions (Pass 1)
- 🔵 **Dependency**: Sibling 0060 not acknowledged — **Resolved**. Added under Related.
- 🔵 **Dependency**: Open question's implied consumers not enumerated — **Resolved**. Consumer list added inline (review skills, validate-plan, future visualiser-graph epic).
- 🔵 **Scope**: Design-gap disposition adjacent to core decision — **Resolved by design choice**. Kept bundled per user's decision; ADR is expected to note the corollary nature.
- 🔵 **Testability**: Disposition criterion lacks rationale requirement — **Resolved**. AC #5 now requires recorded rationale.
- 🔵 **Testability**: Open questions not tied to AC — **Resolved**. New AC #8 added.

### New Issues Introduced

#### Major
- 🟡 **Dependency**: Future visualiser-graph epic named as the headline downstream consumer but absent from `Blocks`
  **Location**: Dependencies
  Summary calls this ADR "the precondition for the future visualiser-graph epic"; Assumptions reiterates it. But the Blocks line only mentions producer-skill/template/migration stories under 0057, and 0040 is filed under Related rather than Blocks. The strongest case for prioritisation is invisible in the dependency graph.

#### Minor
- 🔵 **Clarity**: "Defined here" in the `parent` Requirements bullet is ambiguous — `here` could mean this work item or the new ADR. (Location: Requirements)
- 🔵 **Clarity**: "by name on work items" phrasing risks contradicting the "any artifact type may carry it" line that immediately precedes it. (Location: Requirements)
- 🔵 **Dependency**: Affected consumers named in Open Questions (review skills, validate-plan) not reflected in Dependencies as Related. (Location: Open Questions)
- 🔵 **Testability**: AC #1's "semantics" check has no minimum bar (one sentence vs worked example). (Location: Acceptance Criteria #1)
- 🔵 **Testability**: AC #7 cross-reference criterion has no measurable content requirement. (Location: Acceptance Criteria #7)
- 🔵 **Testability**: AC #2 verifies table presence but not coverage of the motivating multi-row examples called out in Requirements. (Location: Acceptance Criteria #2)
- 🔵 **Testability**: AC #3 "specifies how inverses are derived" has no concrete bar (pseudo-code? prose? worked example?). (Location: Acceptance Criteria #3)

#### Suggestions
- 🔵 **Clarity**: "Backfill with explicit work-item IDs" in Dependencies has no named actor. (Location: Dependencies)
- 🔵 **Scope**: Type-pair table could inflate the task if interpreted maximally; clarify "illustrative" vs "exhaustive". (Location: Requirements)
- 🔵 **Testability**: AC #1's fixed key list interacts with the open question on adding new keys — bind them explicitly. (Location: Acceptance Criteria vs Open Questions)

### Assessment

The two original major findings — `parent`'s status against ADR-0033 and the reference-quoting contradiction — are cleanly resolved, and 11 of the remaining 13 lesser findings are fully resolved. The dependency lens surfaced one new major (visualiser-graph epic absent from Blocks) that should be addressed before this work item moves to `ready`. The new minors are all incremental tightening — none would block implementation. Verdict drops from REVISE to COMMENT.
