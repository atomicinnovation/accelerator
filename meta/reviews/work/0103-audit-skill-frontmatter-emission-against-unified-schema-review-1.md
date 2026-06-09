---
type: work-item-review
id: "0103-audit-skill-frontmatter-emission-against-unified-schema-review-1"
title: "Work Item Review: Audit Skill Frontmatter Emission Against the Unified Schema"
date: "2026-06-09T15:49:00+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0103"
work_item_id: "0103"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-06-09T18:15:15+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Audit Skill Frontmatter Emission Against the Unified Schema

**Verdict:** REVISE

This is a strong, well-structured task work item — every section is substantively populated, the motivating drift problem is clearly explained, and the central validate-plan example is stated with disambiguating precision. It is held back from APPROVE by two `major` testability findings on the same root cause: two acceptance criteria (AC2 "every emitted attribute is shown conforming" and AC4 "a representative emission") describe verification subjects without defining the procedure or selection criterion that makes them checkable. A recurring secondary theme — the audit set's exact membership being left approximate ("~18 SKILL.md files") — surfaces across clarity and testability and weakens the verifiability of the "every skill" criteria.

### Cross-Cutting Themes

- **Verification subjects defined without a checkable procedure** (flagged by: testability) — AC2 and AC4 name what must conform ("every emitted attribute", "a representative emission") but not how conformance is demonstrated or which emission shape is sampled, so a verifier cannot conclusively decide pass/fail.
- **Audit-set membership left approximate** (flagged by: clarity, testability) — the "~18 SKILL.md files across work/, planning/, …" scoping fixes neither the exact set nor a discovery rule, so the "Every frontmatter-writing skill is listed" criterion depends on a boundary the work item never pins, and completeness of coverage can't be independently checked.
- **Audit-and-fix vs net-new CI guard as one unit** (flagged by: scope, dependency) — the one-time audit/fix and the reusable conformance-guard harness are bundled under a single task; scope questions whether they're one deliverable, dependency notes the guard structurally presupposes the audit's output.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: AC2 'every emitted attribute is shown conforming' lacks a defined verification procedure
  **Location**: Acceptance Criteria
  "Shown conforming" has no defined procedure — unclear whether conformance is demonstrated by running the validator over emitted output, by a per-attribute cross-check table, or by reviewer inspection. The per-axis (vs whole-file) result is not bound to any executable check.

- 🟡 **Testability**: AC4 'a representative emission' has no selection criterion
  **Location**: Acceptance Criteria
  "Representative" is undefined; a skill may emit different frontmatter by input (anchored vs not, with/without linkage). Since the Requirements call out conditional axes (provenance-iff-anchored, omit-when-empty), a single sample may not exercise the conditional branches.

#### Minor

- 🔵 **Clarity**: Claim that `complete` 'belongs only to plan-validation' is imprecise
  **Location**: Context
  `complete` is the status value for several types (plan-validation, plan-review, pr-description, review types). The intended meaning is narrower: among the artifacts `/validate-plan` writes, `complete` is valid for its report but not the plan.

- 🔵 **Clarity**: Type name `plan-validation` vs `plan-review` may confuse against ADR-0042
  **Location**: Requirements
  The work item names validate-plan's report `plan-validation`; cited ADR-0042 discusses `plan-review`. Both exist with vocab `complete`, so a reader following the link to verify the claim lands on a differently-named type.

- 🔵 **Clarity**: '~18 SKILL.md files' leaves the audit set's exact membership unstated
  **Location**: Requirements
  The approximate count plus directory list describe but never pin the set's membership, and AC1 ("Every frontmatter-writing skill is listed") then depends on a boundary the work item does not fix.

- 🔵 **Dependency**: Implied internal ordering between audit/fix and guard work is uncaptured
  **Location**: Requirements
  The guard task structurally presupposes the audit's findings (it must drive the same skills and assert against templates-schema.tsv), but this ordering is deducible rather than stated.

- 🔵 **Dependency**: Guard-wiring coupling to test:integration:config not framed as a dependency
  **Location**: Acceptance Criteria
  The new test must integrate with the existing `test:integration:*` harness/task — the seam most likely to collide with other test-suite changes — but this is recorded only as an acceptance criterion, not a named dependency surface.

- 🔵 **Testability**: AC5 fixture vs documented-emission verification mode left as an either/or
  **Location**: Requirements
  "Drive each skill over a fixture (or assert against its documented emission)" leaves the mode unresolved; the two produce materially different guarantees (runtime emission vs SKILL.md text matching the contract).

- 🔵 **Testability**: AC1/AC2 unbounded 'every' is scoped but the boundary lives only in Requirements
  **Location**: Acceptance Criteria
  The "~" count and directory-list-as-scope mean the enumeration's completeness is not independently checkable; a missed producer would silently pass AC1.

#### Suggestions

- 🔵 **Clarity**: 'Fix each divergence' relies on an actor/scope left implicit
  **Location**: Requirements
  Unclear whether a fix means editing SKILL.md prose only, or whether a divergence rooted in the schema TSV is in or out of scope for this audit.

- 🔵 **Scope**: Audit-and-fix and the net-new CI guard are two deliverables under one task
  **Location**: Requirements
  The one-time audit/fix and the reusable producer-conformance harness could ship and roll back independently; bundling a ~18-file sweep with new CI infrastructure may exceed a typical task and blur "done".

- 🔵 **Scope**: Per-axis fixes across many producers could span several skills as one unit
  **Location**: Requirements
  If the audit surfaces divergences in several skills, the fix portion becomes a wide multi-skill change whose pieces are independently deployable. Appropriate to keep as one task today (only validate-plan is confirmed); revisit if the audit reveals more.

### Strengths

- ✅ The pivotal status example is stated with disambiguating precision — validate-plan legitimately emits `status: complete` for its own report (`SKILL.md:161`) but wrongly for the plan (`:187`).
- ✅ Specialist terms (provenance bundle, typed-linkage, code_state_anchored, forbidden_own_id_key, omit-when-empty) are each tied to the contract surfaces enumerated in References/Technical Notes, pointing the reader to a single authoritative definition.
- ✅ Strong internal consistency: "every axis the corpus validator enforces — not just status" is faithfully expanded across Summary, Context, Requirements, and Acceptance Criteria.
- ✅ Every expected section is present and substantively populated (no placeholders); frontmatter is complete with recognised values.
- ✅ Kind is correctly chosen and explicitly defended in Drafting Notes (`task`, not `spike`: a concrete fix plus a guard, not an investigation).
- ✅ The upstream prerequisite (0070's validator, emission-rules helper, schema TSV) is named and correctly classified as already-shipped, justifying "Blocked by: none".
- ✅ AC3 and AC5 are fully testable — AC3 specifies both observable outcomes of the validate-plan fix; AC5 demands a negative test proving the guard actually catches drift.
- ✅ The parent epic 0057 is a genuine coherence fit — 0103 is a producer-conformance follow-on, not a grab-bag child.

### Recommended Changes

1. **Define the conformance-verification procedure for AC2** (addresses: AC2 lacks a defined verification procedure; AC1/AC2 unbounded 'every'). Specify the artefact and check, e.g. "a per-skill table maps each emitted attribute to the validator rule it satisfies (or the fix applied), and a sample emission for each type passes `scripts/validate-corpus-frontmatter.sh` with zero diagnostics."

2. **Replace 'a representative emission' with a defined fixture set in AC4** (addresses: AC4 has no selection criterion; AC5 fixture-vs-documented mode). State that, for each producer, the emission(s) exercising every conditional axis it can emit (anchored/non-anchored, with/without linkage) pass the validator — and fix the AC5 mode either/or so the guard verifies runtime emission (or doc-emission only where the value is a verbatim literal).

3. **Make the audit-set boundary determinate** (addresses: '~18 SKILL.md files' membership; AC1 unbounded 'every'). Give a precise membership/discovery rule, e.g. "any SKILL.md whose instructions write or substitute YAML frontmatter into a produced artifact, as found by `<grep pattern>`," so AC1 is re-runnable rather than trusted.

4. **Tighten the `complete` / type-naming claims in Context** (addresses: `complete` 'belongs only to plan-validation' imprecise; plan-validation vs plan-review). Scope the claim to validate-plan's own outputs and add a clause distinguishing the `plan-validation` report type from the `plan-review` type ADR-0042 reconciles.

5. **Surface the implied couplings in Dependencies** (addresses: internal audit→guard ordering; guard-wiring to test:integration:config). Note that the guard depends on the producer enumeration being settled, and that the work integrates with the existing `test:integration:*` task wiring and the `test-skill-frontmatter-population.sh` harness pattern.

6. **(Optional) Confirm the audit+guard bundling** (addresses: two deliverables under one task). Decide whether to keep as one task or split the guard harness into a sibling so the immediate fixes (validate-plan `complete → done`) can land independently.

## Per-Lens Results

### Clarity

**Summary**: The work item is dense and heavily jargon-laden but the specialist vocabulary is consistently anchored to the linked contract surfaces, so a domain reader can resolve each term. The intent is internally coherent across Summary, Context, Requirements, and Acceptance Criteria, and the central status example is unambiguous and corroborated by ADR-0042. The only real clarity risks are a slightly imprecise claim that `complete` 'belongs only to plan-validation' and a couple of underspecified referents ('~18 SKILL.md files', 'each divergence').

**Strengths**:
- The pivotal status example is stated with exact, disambiguating precision (SKILL.md:161 vs :187).
- Specialist terms are tied to the contract surfaces in References/Technical Notes.
- Internal consistency is strong; scope reads identically across sections.
- Assumptions names the validator as the single authoritative oracle, fixing the referent for "conforming"/"the contract".

**Findings**:
- 🔵 minor (medium): Claim that `complete` 'belongs only to plan-validation' is imprecise (Context) — `complete` is the vocab value for several types; intended meaning is narrower (validate-plan's own outputs). Reword to scope the claim.
- 🔵 minor (low): Type name `plan-validation` vs `plan-review` may confuse against ADR-0042 (Requirements) — both type names exist with vocab `complete`; add a one-clause note distinguishing them.
- 🔵 minor (medium): '~18 SKILL.md files' leaves the audit set's exact membership unstated (Requirements) — define the membership rule precisely so the set is determinate.
- 🔵 suggestion (low): 'Fix each divergence' relies on an actor/scope left implicit (Requirements) — state whether fixes are confined to producing-skill text or may reach the schema source.

### Completeness

**Summary**: A well-structured `task` work item with every expected section present and substantively populated. Summary clearly states the work, Context explains the motivating drift problem, Requirements are concrete and actionable, and Acceptance Criteria define done with multiple specific bullets including a negative-path guard. No completeness gaps of consequence were found.

**Strengths**:
- Summary is a single, unambiguous action statement, with the scope-broadening rationale in Drafting Notes.
- Context explains the why thoroughly with a concrete leak example.
- Requirements are specific enough to start work (enumerated set, per-axis checklist, concrete remediation example).
- Acceptance Criteria contain six specific bullets covering enumeration, per-attribute conformance, the validate-plan fix, validator pass, a negative-path guard, and CI staying green.
- Kind is correctly chosen and justified.
- Optional sections all populated with genuine content; frontmatter complete with recognised values.

**Findings**: _None._

### Dependency

**Summary**: Couplings are well captured for a task of this kind. The single hard prerequisite (0070-shipped validator, emission-rules helper, schema) is explicitly named and correctly characterised as already-satisfied, and relationships to 0070, 0057, and ADR-0042 are recorded across frontmatter and Dependencies/References. The only gaps are internal sequencing within the task's own requirements and a downstream guard-wiring coupling that is implied but not framed as a dependency.

**Strengths**:
- The upstream prerequisite is explicitly named and correctly classified as already-shipped, justifying "Blocked by: none".
- The ADR-0042 relationship is captured and is load-bearing for the concrete fix.
- Cross-links are consistent across frontmatter, Dependencies, and References.

**Findings**:
- 🔵 minor (medium): Implied internal ordering between audit/fix and guard work is uncaptured (Requirements) — the guard presupposes the audit's findings; note the ordering constraint.
- 🔵 minor (medium): Guard-wiring coupling to test:integration:config not framed as a dependency (Acceptance Criteria) — add a Dependencies note that the work integrates with the existing `test:integration:*` task wiring and the `test-skill-frontmatter-population.sh` pattern.

### Scope

**Summary**: A well-bounded task with a single unifying purpose: confirm frontmatter-emitting skills conform to the validator contract, fix divergences, and add a drift guard. The audit, fixes, and conformance guard form one coherent producer-side delivery unit closing a gap 0070 left open, and the `task` kind fits. The only tension is whether the net-new conformance guard is the same unit of work as the one-time audit-and-fix, but the bundling is defensible.

**Strengths**:
- Single coherent purpose; no unrelated concerns bundled.
- Summary, Requirements, and Acceptance Criteria describe the same scope.
- Kind correctly justified in Drafting Notes.
- Scope boundaries explicit (producers vs already-validated corpus, citing 0070).
- Parent epic 0057 is a genuine coherence fit.

**Findings**:
- 🔵 suggestion (medium): Audit-and-fix and the net-new CI guard are two deliverables under one task (Requirements) — confirm the combined scope still reads as one task; consider splitting the guard if it proves substantial.
- 🔵 suggestion (low): Per-axis fixes across many producers could span several skills as one unit (Requirements) — appropriate to keep as one task today; revisit if the audit reveals large divergences.

### Testability

**Summary**: Largely testable: it anchors verification on a concrete oracle (the corpus validator), gives a fully specified before/after for the validate-plan fix, and requires a negative-path guard that proves the test catches drift. The main gaps are AC2 and AC4, where the verification subjects ('every emitted attribute is shown conforming', 'a representative emission') lack a defined procedure or selection criterion.

**Strengths**:
- AC3 is fully testable — both observable outcomes specified, tied to the exact bug (SKILL.md:187 vs :161).
- AC5 demands a negative test, converting a green-path-only guard into a verifiable assertion.
- Assumptions/Technical Notes name a single runnable oracle and the per-type fact source.
- AC6 is a directly executable binary check.

**Findings**:
- 🟡 major (medium): AC2 'every emitted attribute is shown conforming' lacks a defined verification procedure (Acceptance Criteria) — bind "shown" to an executable per-axis check; specify the artefact (e.g. per-skill conformance table + sample emission passing the validator).
- 🟡 major (medium): AC4 'a representative emission' has no selection criterion (Acceptance Criteria) — replace with a defined fixture set exercising every conditional axis (anchored/non-anchored, with/without linkage).
- 🔵 minor (medium): AC5 fixture vs documented-emission verification mode left as an either/or (Requirements) — state which mode the test must use; doc-emission acceptable only where the value is a verbatim literal.
- 🔵 minor (low): AC1/AC2 unbounded 'every' is scoped but the boundary lives only in Requirements (Acceptance Criteria) — make the enumeration verifiable via a stated discovery procedure.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-06-09

**Verdict:** COMMENT

Re-ran the four lenses that had findings (clarity, dependency, scope, testability; completeness had none). Both `major` testability findings that drove the REVISE verdict are addressed — AC4 dropped to `minor` and the AC5 mode either/or is resolved — leaving a single residual `major`. Every other Pass 1 finding is resolved or downgraded, and the new findings are lower-severity refinements. With one major and no criticals, the work item clears the REVISE threshold (≥2 majors) and is acceptable as-is.

### Previously Identified Issues

- 🟡 **Testability**: AC2 lacks a defined verification procedure — **Partially resolved.** AC2 now requires a per-attribute conformance table mapping each emitted attribute to the validator rule it satisfies. Residual (re-flagged `major`): the table is a manually-authored artefact with no mechanical check that it is itself complete/correct. Suggested remedy: tie table completeness to the validator's enforced attribute set (`templates-schema.tsv`).
- 🟡 **Testability**: AC4 'a representative emission' has no selection criterion — **Resolved** (downgraded to `minor`). AC4 now names the conditional-axis matrix (anchored/non-anchored, with/without typed-linkage, omit-when-empty). Residual minor: "every conditional axis" is not declared exhaustive.
- 🔵 **Testability**: AC5 fixture vs documented-emission mode either/or — **Resolved.** The guard requirement now mandates verifying the actual emitted value, with documented-emission allowed only for verbatim literals.
- 🔵 **Testability**: AC1/AC2 unbounded 'every' — **Resolved** (residual minor). AC1 now references a re-runnable discovery procedure; residual is only the "~18" approximate count vs an exact procedure-defined set.
- 🔵 **Clarity**: `complete` 'belongs only to plan-validation' imprecise — **Resolved** (now a strength: author proactively disambiguates).
- 🔵 **Clarity**: plan-validation vs plan-review naming — **Resolved** (now a strength).
- 🔵 **Clarity**: '~18 SKILL.md files' membership — **Resolved.** Determinate membership rule + `grep -rl` discovery procedure added.
- 🔵 **Clarity**: 'Fix each divergence' scope implicit — **Resolved** (now a strength: explicit scope boundary; schema-source divergences referred out).
- 🔵 **Dependency**: internal audit→guard ordering uncaptured — **Resolved** (now a strength; captured in Dependencies).
- 🔵 **Dependency**: guard-wiring to test:integration:config not a dependency — **Resolved** (now a strength; "Integrates with" entry added).
- 🔵 **Scope**: audit+guard two deliverables — **Resolved** (Drafting Note now records the deliberate single-task decision + split trigger).
- 🔵 **Scope**: per-axis fixes multi-skill — **Resolved** (split trigger documented).

### New Issues Introduced

- 🔵 **Testability** (minor): AC5 "out-of-contract attribute" does not specify which mutation(s) prove the guard trips across contract axes — suggest one negative mutation per major axis.
- 🔵 **Dependency** (minor): the "referred out" schema-source divergence path names no destination (follow-on item / owner) where a referral lands.
- 🔵 **Dependency** (minor): the Drafting-Note split-out fan-out is prose only, not surfaced in Dependencies where scheduling is read.
- 🔵 **Clarity** (minor): the second Context paragraph compresses ~5 distinct claims into one dense passage; the per-type caveat risks being skimmed.
- 🔵 **Clarity** (suggestions): schema-internal terms used without local gloss; 0057-epic / 0070-story identifiers easy to conflate; "documented-emission mode" named without definition.
- 🔵 **Scope** (suggestions): both Pass-1 scope notes recur as "acceptable as one task" — surface the split trigger into Dependencies/AC as a tracked checkpoint rather than a buried note (optional).

### Assessment

The work item is ready for implementation. The two REVISE-driving majors are addressed; the lone residual major (AC2 table self-completeness) and the new minors are quality refinements that can be folded in during planning rather than blockers. Highest-value optional polish: tie the AC2 conformance table's completeness to the validator's enforced attribute set so the audit's core deliverable is mechanically checkable, and name a destination for referred-out schema-source divergences.

## Verdict Update (Pass 3) — 2026-06-09

**Verdict:** APPROVE

The residual `major` (AC2 conformance-table self-completeness) was resolved by post-Pass-2 edits: the table is now defined as complete when its per-type attribute set equals the attribute set the validator enforces (derived from `templates-schema.tsv` / `frontmatter-emission-rules.sh`), making it mechanically checkable. The referred-out schema-source divergence path now names a destination (child work items under epic 0057) in both Requirements and Dependencies, and the conditional split-out fan-out is surfaced in Dependencies. With no remaining critical or major findings, the author approves the work item for implementation. The remaining minor/suggestion items (AC5 per-axis negative mutations, dense Context paragraph, schema-term glosses, 0057/0070 naming) are non-blocking and may be folded in during planning.
