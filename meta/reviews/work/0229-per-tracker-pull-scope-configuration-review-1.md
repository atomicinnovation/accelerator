---
type: "work-item-review"
id: "0229-per-tracker-pull-scope-configuration-review-1"
title: "Work Item Review: Per-Tracker Pull Scope Configuration"
date: "2026-09-09T22:55:05+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0229"
work_item_id: "0229"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 5
tags: []
last_updated: "2026-09-10T00:20:02+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Per-Tracker Pull Scope Configuration

**Verdict:** REVISE

The story is structurally complete and well-bounded — every expected section is substantively populated, the config vocabulary is named consistently across Summary, Requirements, and Acceptance Criteria, and the single hard blocker (0228) is cleanly captured. It needs revision before implementation for one reason: the **filter surface** — the feature's core new behaviour — is under-specified across three lenses at once, with no verifiable criterion for its AND/OR semantics, no enumerated schema, and an unresolved config-plumbing prerequisite. Two further stated behaviours (stable reconcile ordering, required-key validation) lack testable coverage.

### Cross-Cutting Themes

- **Filter surface is under-specified** (flagged by: completeness, dependency, testability, clarity) — the `filters` bag is the primary new mechanism, yet its accepted keys are never enumerated (completeness), its AND-across-keys / OR-within-key semantics have no verifiable criterion (testability), structured-value support may be an unbuilt config-plumbing prerequisite living only as an Open Question (dependency), and the remote-existence boundary around named filter entities reads ambiguously (clarity). These reinforce each other: the filter mechanism needs grounding before the story is ready.
- **Remote-existence check: this story vs 0227** (flagged by: dependency, clarity) — the boundary between what 0229 validates at `configure`/sync and what it delegates to 0227's `config validate` is captured in only one direction and reads ambiguously. An implementer cannot tell whether to build remote-resolution/abort logic here or defer it entirely.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: Filter AND/OR normalisation semantics have no verifiable criterion
  **Location**: Acceptance Criteria (item 1) / Requirements: Filters
  The Requirements specify 'keys are AND'd; multiple values within a key are OR'd', but AC 1 asserts only that 'discovery is broadened accordingly'. The core new behaviour could be implemented incorrectly and still pass every listed criterion.

- 🟡 **Testability**: Stable reconcile ordering requirement is uncovered by any criterion
  **Location**: Requirements: Result handling
  'Discovered issues are reconciled in stable order by tracker identifier' has no Acceptance Criterion — AC 11 covers deduplication only. An arbitrary-order implementation would satisfy all criteria while violating the requirement.

- 🟡 **Testability**: Required-filter-key failure case cannot be constructed given the all-optional assumption
  **Location**: Acceptance Criteria (item 4) / Assumptions
  AC 4 requires a missing required filter key to fail at `configure`, but the Assumptions state all filter keys are optional. With no schema declaring a required key, no input triggers the branch, making that half of the criterion untestable in practice.

- 🟡 **Dependency**: Potential config-plumbing prerequisite for structured `pull.filters` values captured only as an open question
  **Location**: Open Questions
  The catalogue registers scalar defaults today; the first Open Question asks whether the plumbing must be extended to carry structured `filters` values. If it is a separate foundational change, the team hits a hidden blocker at implementation time, invalidating ready-to-start status.

#### Minor

- 🔵 **Dependency**: Coupling with 0227's config validate command captured in only one direction
  **Location**: Dependencies
  0229 introduces a new validatable config surface but the Dependencies section records only the remote-existence delegation to 0227, not that 0227's command must learn to validate the `pull` block, nor whether 0227 is a prerequisite for 0229's config-time validation.

- 🔵 **Scope**: Ceiling model and truncation-to-error change is a partially separable increment
  **Location**: Requirements: Scope resolution and ceilings
  The story folds in per-tracker `max_items`/`max_pages` defaults and promotes silent `Discovery { complete: false }` truncation to a hard error — a behaviour change to existing discovery semantics that could ship as its own increment. The coupling is defensible but its deliberateness is not stated.

- 🔵 **Clarity**: Dedup rule shifts identifier terms and buries the copy-retention case
  **Location**: Requirements: Result handling
  Terms drift — 'tracker work-item identifier', 'tracker identifier', 'dedup by tracker identifier'; 'tracker identifier' reads ambiguously as the tracker's identity rather than the issue's remote ID, and the copy-retention clause is hard to parse on first read.

- 🔵 **Clarity**: 'Credentialed-team fallback' used without definition
  **Location**: Context
  `all_teams` 'suppresses the credentialed-team fallback', but that term is never defined here — it originates in 0220. A reader of 0229 alone cannot tell what whole-workspace Linear search changes.

- 🔵 **Clarity**: Unclear whether this story checks remote existence of named entities
  **Location**: Requirements: Filters / Scope resolution and ceilings
  'remote-existence of named entities is delegated to 0227' sits against 'a named entity that cannot be resolved on the remote at sync time errors and aborts'. Read together it is ambiguous whether this story performs any remote-existence handling.

- 🔵 **Clarity**: Ambiguous whether `max_items` bounds discovered issues or reconciled items
  **Location**: Requirements: Scope resolution and ceilings
  `max_items` is defined as the default for the `max_pulls` reconcile ceiling, yet the trigger is framed as 'discovery crosses `max_items`'. The distinction determines whether the all-or-nothing error fires at discovery or reconcile.

- 🔵 **Testability**: 'Spans everything the credential can see' lacks an observable check
  **Location**: Acceptance Criteria (item 2)
  Unboundedness cannot be observed directly. The Context already defines the observable seam (Jira omits the `project =` clause; Linear drops the team filter); AC 2 should reframe to that seam.

- 🔵 **Testability**: 'unlimited ... does not bound discovery' gives no scenario to observe unboundedness
  **Location**: Acceptance Criteria (item 9)
  Verifying the absence of a bound needs a setup whose discovery would otherwise exceed the default ceiling; AC 9 specifies no such input.

#### Suggestions

- 🔵 **Completeness**: Filter schema accepted keys are not enumerated
  **Location**: Requirements (Filters)
  The Requirements state each tracker 'declares a filter schema (accepted keys; required keys, if any)' but no section enumerates those keys for Jira and Linear, leaving the concrete schema surface implicit.

- 🔵 **Scope**: Result-handling scope (dedup, stable ordering) is absent from the Summary
  **Location**: Summary
  The Summary enumerates only the config surface; a reader sizing the work from it alone under-counts the dedup/ordering behaviour.

- 🔵 **Scope**: Story is on the large end for a single increment
  **Location**: Requirements
  New structured `pull` block, per-tracker filter schema + validation, reserved-key handling, two ceiling mechanisms, dedup, and stable ordering across both adapters. The filter schema/validation is a natural split seam if planning strain emerges.

- 🔵 **Clarity**: JQL acronym not expanded on first use
  **Location**: Context
  'JQL' appears without expansion; spelled out in 0220 but not here.

### Strengths

- ✅ Config-key vocabulary (`additional_*`, `all_*`, `filters`, `max_items`/`max_pages`) is named identically across Summary, Requirements, and Acceptance Criteria — no drift in the primary nouns.
- ✅ Exceptionally complete for a story: every expected section present and substantively populated, with genuinely populated Open Questions, Assumptions, and Technical Notes rather than placeholders.
- ✅ Boundaries are crisp and explicit — pull-side only, Jira/Linear only, GitHub deferred to 0050, push untouched — making in-scope vs out-of-scope unambiguous.
- ✅ The single hard blocker (0228) is captured in frontmatter, the Dependencies section, and justified in Context; epic-internal ordering (0220 → 0228 → 0229) is coherent and traceable.
- ✅ The all-or-nothing ceiling behaviours (AC 7, AC 8), the exact reserved-key error string (AC 5), the mutual-exclusivity rule (AC 3), and the no-config default path (AC 12) are precisely specified and directly verifiable.
- ✅ Remote-existence validation is delegated to 0227 rather than duplicated, drawing a clean boundary; reserved `all`/`any` grouping keys anticipate future nested-filter work with no config migration.

### Recommended Changes

1. **Ground the filter semantics with a concrete input→output criterion** (addresses: Filter AND/OR normalisation has no verifiable criterion; Filter schema keys not enumerated)
   Replace AC 1's 'broadened accordingly' with an asserted output, e.g. 'Given filters `{label: [a, b], state: [open]}`, the emitted query constrains `state = open AND label IN (a, b)`'. Enumerate the accepted filter keys per tracker (even indicatively) in Requirements, or state explicitly that the schema is deferred to implementation.

2. **Resolve the structured-filters config-plumbing question before scheduling** (addresses: config-plumbing prerequisite captured only as an open question)
   Either confirm the catalogue extension for structured `filters` values is in-scope for this story and say so in Requirements, or split it into a prerequisite work item and add it to Dependencies as a blocker.

3. **Add acceptance criteria for stable ordering and the unlimited/all-workspace observable seams** (addresses: stable reconcile ordering uncovered; 'spans everything' lacks observable check; 'unlimited' gives no scenario)
   Add an ascending-identifier ordering criterion; reframe AC 2 to the observable seam (no project/team scope constraint on the emitted search); give AC 9 a triggering set larger than the default ceiling.

4. **Reconcile AC 4 with the all-optional assumption** (addresses: required-key failure case cannot be constructed)
   Either specify a test-fixture schema declaring a required key so the branch is exercisable, or narrow AC 4 to the unsupported-key case the current schema can produce.

5. **State the 0229 ↔ 0227 relationship in both directions** (addresses: 0227 coupling captured in only one direction; unclear whether this story checks remote existence)
   Note that 0227 must validate the `pull`-block surface (or that 0229 adds a consume-site validator 0227 aggregates), and clarify that 0229 adds no proactive config-time remote-existence check but does abort at sync when a named entity fails to resolve.

6. **Tighten the ceiling and dedup wording** (addresses: `max_items` bounds discovered vs reconciled ambiguity; dedup term drift; ceiling change deliberateness)
   State which count `max_items` limits and align the definition with the crossing behaviour; use one identifier term consistently and restate the copy-retention case plainly; add a one-line note on why the ceiling/truncation change ships in the same increment.

## Per-Lens Results

### Clarity

**Summary**: Dense but largely unambiguous — config-key names, the `pull` block vocabulary, and the Given/When/Then criteria map cleanly across sections, and pronouns generally resolve to explicit referents. Remaining gaps are localised: a couple of shifting or undefined domain terms, one obscure sentence in the dedup rule, and a cross-requirement wording tension around where remote-existence and item-count checks fire. None rise above minor.

**Strengths**:
- Config-key vocabulary named identically across Summary, Requirements, and Acceptance Criteria — no drift in the primary nouns.
- The first Context sentence bridges near-synonyms by stating 'The keyed creation entity is always the implicit base scope'.
- Acceptance Criteria phrased as Given/When/Then with named triggers, keeping actor and point of evaluation clear.
- The 'effective (post-override) block' term is consistently anchored to the personal-replaces-team rule.

**Findings**:
- 🔵 minor (confidence: medium) — Requirements: Result handling — **Dedup rule shifts identifier terms and buries the copy-retention case in obscure phrasing.** Terms drift ('tracker work-item identifier' → 'tracker identifier' → 'dedup by tracker identifier'); 'tracker identifier' can read as the tracker's identity rather than the issue's remote ID, and the copy-retention clause is hard to parse. Use one term consistently and restate the copy case plainly.
- 🔵 minor (confidence: medium) — Context — **'Credentialed-team fallback' used without definition.** The term originates in 0220; a reader of 0229 alone cannot tell what `all_teams` suppresses. Define it in a short clause or link to 0220.
- 🔵 minor (confidence: medium) — Requirements: Filters / Scope resolution — **Unclear whether this story checks remote existence of named entities.** 'delegated to 0227' sits against 'cannot be resolved on the remote at sync time errors and aborts'. State that this story adds no proactive config-time check but does abort at sync.
- 🔵 minor (confidence: medium) — Requirements: Scope resolution and ceilings — **Ambiguous whether `max_items` bounds discovered issues or reconciled items.** Defined as the `max_pulls` reconcile ceiling default, yet the trigger is framed as 'discovery crosses `max_items`'. State which count it limits.
- 🔵 suggestion (confidence: low) — Context — **JQL acronym not expanded on first use.** Spelled out in 0220 but not here. Expand or link.

### Completeness

**Summary**: Exceptionally complete for its kind — every expected section present (Summary with an explicit user role, Context, richly subdivided Requirements, twelve Given/When/Then criteria, Open Questions, Dependencies, Assumptions, Technical Notes, Drafting Notes, References), all substantively populated. Frontmatter is intact with a recognised `kind: story`, `status: draft`, `priority`, and correct linkage. The only mild gap: the per-tracker filter schema is described as a mechanism without any section enumerating its accepted keys.

**Strengths**:
- Summary states the work as a clear user story with a named beneficiary.
- Twelve specific Given/When/Then criteria map closely onto the Requirements, including the no-config default case.
- Context explains why the work is needed rather than restating the Summary.
- Requirements organised into focused, actionable subsections.
- Frontmatter complete and internally consistent with the References section.
- Optional sections (Open Questions, Assumptions, Dependencies) genuinely populated, not placeholders.

**Findings**:
- 🔵 suggestion (confidence: low) — Requirements (Filters) — **Filter schema accepted keys not enumerated.** The item states each tracker 'declares a filter schema (accepted keys; required keys, if any)' but no section lists those keys for Jira and Linear. Add an indicative accepted-key set per tracker, or note the schema is deferred to implementation.

### Dependency

**Summary**: The hard upstream blocker (0228) is correctly captured in both frontmatter and the Dependencies section with a clear rationale, and the epic-internal ordering (0220 done → 0228 → 0229) is coherent and traceable. Two couplings are under-captured: a potential prerequisite on config plumbing for structured `pull.filters` values that lives only as an unresolved Open Question, and the bidirectional relationship with 0227's `config validate` command, captured only in the remote-existence-delegation direction.

**Strengths**:
- The single hard blocker (0228) is captured in `blocked_by`, the Dependencies section, and justified in Context/Requirements.
- Epic-internal ordering is coherent and traceable across 0146, 0228, and 0229.
- Remote-existence validation explicitly delegated to 0227 rather than duplicated.
- Forward-compatibility with future nested-filter work anticipated via reserved `all`/`any` grouping keys, mirrored in 0146's Future Candidates.

**Findings**:
- 🟡 major (confidence: medium) — Open Questions — **Potential config-plumbing prerequisite for structured `pull.filters` values captured only as an open question.** The catalogue registers scalar defaults today; if structured-value support is a separate foundational change, the team hits a hidden blocker at implementation time. Resolve before scheduling — confirm in-scope or split into a prerequisite blocker.
- 🔵 minor (confidence: medium) — Dependencies — **Coupling with 0227's config validate command captured in only one direction.** 0229 introduces a new validatable config surface; the Dependencies section records only the delegation direction, not that 0227 must validate the `pull` block, nor whether 0227 is a prerequisite for 0229's config-time validation. State the relationship in both directions.

### Scope

**Summary**: A coherent, well-bounded story — a single capability (making per-tracker pull discovery scope configurable) with explicit non-goals and an always-implicit base scope. It sits cleanly as one of four children under epic 0146 and does not span ownership boundaries. It is on the larger end for a story and folds in an adjacent ceiling/truncation-behaviour change that is arguably separable, but the pieces are defensibly cohesive around the one 'bound discovery' theme.

**Strengths**:
- Boundaries stated explicitly and crisply, making in-scope vs out-of-scope unambiguous.
- All requirements orbit a single theme: bounding/broadening per-tracker pull discovery.
- Fits its declared kind and place in the decomposition, with clean seams to 0228 and 0227.
- Summary, Requirements, and Acceptance Criteria tightly aligned on the config surface.

**Findings**:
- 🔵 minor (confidence: medium) — Requirements: Scope resolution and ceilings — **Ceiling model and truncation-to-error change is a partially separable increment.** Per-tracker `max_items`/`max_pages` defaults and the silent-truncation-to-hard-error promotion touch existing discovery semantics and could ship as their own increment. Defensible, but note in the item why the ceilings must ship in the same increment.
- 🔵 suggestion (confidence: medium) — Summary — **Result-handling scope (dedup, stable ordering) is absent from the Summary.** A reader sizing from the Summary alone under-counts the work. Add a clause noting broadening across multiple scopes necessitates dedup and stable-order reconciliation.
- 🔵 suggestion (confidence: low) — Requirements — **Story is on the large end for a single increment.** New structured `pull` block, filter schema + validation, reserved-key handling, two ceilings, dedup, ordering, across both adapters. No action required if judged one deliverable; the filter schema/validation is a natural split seam.

### Testability

**Summary**: A Story whose Acceptance Criteria are mostly well-formed Given/When/Then triples with observable outcomes, including strong negative cases and an exact expected error string. The main gaps are two Requirements-level behaviours — filter AND/OR normalisation semantics and stable reconcile ordering — that no criterion pins down, plus one criterion whose triggering input cannot be constructed given the item's own all-keys-optional assumption. A few criteria lean on unbounded phrasing a verifier cannot turn into definitive pass/fail.

**Strengths**:
- Most criteria are concrete Given/When/Then pairs with a definite outcome — mutual exclusivity (AC 3), configure-time vs sync-time placement (AC 4), the exact reserved-key error (AC 5).
- All-or-nothing ceiling behaviours precisely specified and directly verifiable (AC 7, AC 8).
- The negative/default path explicitly covered (AC 12), so the base-scope invariant is testable.
- Deduplication given an observable rule tied to a concrete merge scenario (AC 11).

**Findings**:
- 🔴 major (confidence: high) — Acceptance Criteria (item 1) / Requirements: Filters — **Filter AND/OR normalisation semantics have no verifiable criterion.** AC 1 asserts only 'discovery is broadened accordingly'; the core AND-across-keys / OR-within-key behaviour could be implemented wrongly and still pass. Replace with a concrete input→output criterion asserting the emitted filter.
- 🟡 major (confidence: medium) — Requirements: Result handling — **Stable reconcile ordering requirement is uncovered by any criterion.** AC 11 covers dedup only; an arbitrary-order implementation would satisfy all criteria. Add an ascending-identifier ordering criterion.
- 🟡 major (confidence: medium) — Acceptance Criteria (item 4) / Assumptions — **Required-filter-key failure case cannot be constructed given the all-optional assumption.** With no schema declaring a required key, no input triggers the branch. Specify a test-fixture schema with a required key, or narrow AC 4 to the unsupported-key case.
- 🔵 minor (confidence: medium) — Acceptance Criteria (item 2) — **'Spans everything the credential can see' lacks an observable check.** Reframe to the observable seam (no project/team scope constraint on the emitted search), keeping the ceiling bound as a separate assertion.
- 🔵 minor (confidence: low) — Acceptance Criteria (item 9) — **'unlimited ... does not bound discovery' gives no scenario to observe unboundedness.** Add a triggering set larger than the default ceiling.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-09-09

**Verdict:** COMMENT

All four pass-1 major findings are resolved. One new major surfaced as a
regression from the pass-1 fix, and three new minors (two are wording
artefacts of the new/edited criteria). A single major is below the REVISE
threshold of two, so the item is acceptable as-is; the regression is worth a
quick correction.

### Previously Identified Issues

- 🟡 **Testability**: Filter AND/OR normalisation had no verifiable criterion — Resolved (concrete lowering criterion added; but see new tracker-specificity issue below)
- 🟡 **Testability**: Stable reconcile ordering uncovered — Resolved (ascending-identifier ordering AC added)
- 🟡 **Testability**: Required-key failure untestable — Resolved (AC narrowed to the unsupported-key case; Filters + Assumptions aligned)
- 🟡 **Dependency**: Structured-`filters` config-plumbing prerequisite — Resolved (declared in scope in Requirements: Config surface; Open Questions closed)
- 🔵 **Dependency**: 0227 coupling captured one-directionally — Resolved (Dependencies entry now bidirectional with not-a-hard-blocker rationale)
- 🔵 **Scope**: Ceiling/truncation change deliberateness unstated — Resolved (Drafting Note records the co-ship rationale)
- 🔵 **Clarity**: Dedup identifier term drift — Resolved (unified on "remote work-item identifier"; copy case restated plainly)
- 🔵 **Clarity**: `credentialed-team fallback` undefined — Resolved (defined inline, attributed to 0220)
- 🔵 **Clarity**: Remote-existence handling ambiguous — Resolved (config-time vs sync-time checks now explicitly distinguished)
- 🔵 **Clarity**: `max_items` count ambiguity — Resolved (states it bounds the discovered-issue count)
- 🔵 **Testability**: AC 2 "everything the credential can see" unobservable — Resolved (reframed to the no-scope-constraint seam)
- 🔵 **Testability**: AC 9 `unlimited` had no scenario — Partially resolved (triggering set added; precondition now ambiguous about one vs both ceilings — see new minor)
- 🔵 **Completeness / Scope / Clarity suggestions** (filter keys, Summary dedup clause, JQL expansion) — Resolved
- 🔵 **Scope**: Story on the large end — Still present (suggestion; author cohesion rationale now recorded)

### New Issues Introduced

- 🟡 **Testability**: The filter-lowering criterion pins Jira JQL (`state = open AND label IN (a, b)`) for a behaviour spanning both trackers; the Linear path lowers to a structured known-key object and cannot check the literal string. Split into per-tracker expected outputs, or assert the AND-across-keys / OR-within-key semantic with one concrete example per tracker.
- 🔵 **Testability**: No criterion verifies config-time validation of ceiling *values* — the Summary claims `max_items`/`max_pages` are 'validated at config time', but a malformed value (negative, non-integer, unknown sentinel) has no rejecting AC.
- 🔵 **Testability**: The `unlimited` criterion names both ceilings in its precondition while 'that ceiling' is singular; a set exceeding both defaults still errors on whichever ceiling stays bounded. State that both are `unlimited`, or split per ceiling.
- 🔵 **Clarity**: The base-scope entity is named three ways ('keyed creation entity', 'implicit base scope', 'keyed base entity'); 'creation' is unexplained. Standardise on one term.
- 🔵 **Dependency**: The prose obligates 0227 to validate the `pull` block, but the Dependencies `Blocks` field reads 'none'; a planner reading 0227 alone would miss the ordering. Add a qualified Blocks-style entry.

### Assessment

The work item is ready for planning. Every blocking pass-1 finding is
resolved, and completeness now reports zero findings. The one new major is a
narrow tracker-specificity slip in an acceptance criterion — correcting it
(per-tracker expected outputs) and tightening the two AC-wording minors would
leave the criteria fully verifiable, but none of these block implementation.

## Re-Review (Pass 3) — 2026-09-09

**Verdict:** REVISE

Ran clarity, dependency, scope, and testability (completeness was clean in
pass 2 and was not re-run). The three pass-2 fixes all held — testability
explicitly praised the per-tracker query shapes and the new ceiling-value
criterion. The verdict moved back to REVISE because two fresh majors surfaced
that no earlier pass had flagged, not because any fix regressed.

### Previously Identified Issues (pass 2)

- 🟡 **Testability**: Filter-lowering criterion pinned Jira-specific string — Resolved (AC 2/3 now give per-tracker expected outputs)
- 🔵 **Testability**: `unlimited` precondition one-vs-both ambiguity — Resolved (both ceilings named)
- 🔵 **Testability**: No config-time ceiling-value validation — Resolved (configure-time rejection AC added)
- 🔵 **Clarity**: Base-scope term drift — Still present (recurs below as a suggestion)
- 🔵 **Dependency**: `Blocks: none` vs prose obligation on 0227 — Still present (recurs below as a suggestion)
- 🔵 **Scope**: Large story — Escalated (see new major below)

### New Issues Introduced

- 🟡 **Scope**: The story bundles four substantial mechanisms — scope broadening, the filters bag plus a config-catalogue structured-value extension, configurable ceilings plus the truncation-to-hard-error change, and result dedup/ordering — into one 15-AC increment that has grown beyond the narrower unit epic 0146 assigned to 0229 (the epic lists only `additional_*`/`all_*`/`filters`/schema). The filters-plus-catalogue seam is the most separable. Escalated from a pass-1/2 suggestion.
- 🟡 **Testability**: The reconciliation-ordering criterion ("ascending remote work-item identifier order") does not define the comparison — Jira `PP-2` vs `PP-10` sort differently under lexical vs numeric-suffix order, so two verifiers can reach opposite verdicts on the case most likely to break. (Clarity flags the same ambiguity independently as a minor.)
- 🔵 **Testability**: No criterion pins that a *numeric* configured `max_items`/`max_pages` overrides the built-in default — AC 9/10 can be satisfied by the pre-existing defaults (25 / 20) firing, so a regression ignoring the config value would pass.
- 🔵 **Testability**: AC 1 (`additional_*` broadening) is less concrete than AC 2/3 — it says "discovery is broadened" without naming the observable (emitted query vs discovered set).
- 🔵 **Clarity**: GitHub deferral names 0050, but parent 0146's Technical Notes assign GitHub Issues to 0181 — a cross-document conflict on which future item owns GitHub.
- 🔵 **Scope / Testability / Clarity suggestions**: truncation-to-hard-error is separable; `max_items: 0` has undefined runtime semantics; base-scope term drift; `Blocks: none` vs prose.

### Assessment

Not blocked on quality of expression — the item is clear, complete, and
well-dependency-mapped. The two majors are substantive: the ordering-comparison
gap is a one-line fix to an acceptance criterion (define numeric-suffix order
with a `PP-2`/`PP-10` example), while the scope-bundling major is a genuine
decomposition decision — split the filters + catalogue-extension mechanism into
its own increment, or record an explicit rationale for keeping the enlarged unit
and reconcile it with 0146's child description. The ordering fix is
mechanical; the split is a judgement call for the team.

## Re-Review (Pass 4) — 2026-09-09

**Verdict:** REVISE

Ran clarity, dependency, scope, and testability. Both pass-3 majors are
resolved — scope praised the recorded bundle rationale and the 0146
reconciliation ("well-formed, appropriately-sized, atomic unit"), and
testability praised the exact `PP-2`/`PP-10` reconcile criterion. Two fresh
majors surfaced, each a narrower layer of a concern the pass-3 fix addressed
only partially.

### Previously Identified Issues (pass 3)

- 🟡 **Scope**: Story bundles four mechanisms — Resolved (rationale in Drafting Notes; 0146 child + Stories reconciled to match; scope lens now reports the unit well-formed)
- 🟡 **Testability**: Ordering criterion lacked comparison semantics — Resolved for the single-prefix case (AC now gives `PP-10`,`PP-2` → `PP-2`,`PP-10`); reopens for multi-prefix (new major below)
- 🔵 **Testability**: Numeric configured ceiling never pinned — Partially resolved (`max_items` override AC added; `max_pages` still missing — new major below)
- 🔵 **Testability**: AC 1 vague observable — Resolved (names the emitted per-tracker search)
- 🔵 **Clarity**: GitHub 0050/0181 conflict — Resolved (0229 cites 0050 under the 0181 epic; 0146 agrees)
- 🔵 **Clarity/Dependency suggestions** (base-scope term, `Blocks: none`) — Still present (unchanged by choice)

### New Issues Introduced

- 🟡 **Clarity**: The reconcile-ordering rule is defined only for identifiers sharing one prefix. Multi-scope discovery (the story's purpose) routinely mixes prefixes (`PP-5`, `XX-3`); ordering by the numeric component alone leaves cross-prefix interleaving and numeric-tie resolution (`PP-2` vs `XX-2`) undefined, and "a stable order" in the Summary is itself ambiguous (deterministic vs discovery-order-preserving).
- 🟡 **Testability**: No criterion verifies a configured `max_pages` distinct from the fixed default (20) is honoured — AC10 checks only reaching the cap, AC12 only `unlimited`. The `max_items` override AC (AC11) has no `max_pages` twin, so ignoring configured `max_pages` would pass.
- 🔵 **Testability**: AC8 (personal-replaces-team) restates the rule without a discriminating scenario — no field present in team-only exercises the whole-replacement.
- 🔵 **Dependency**: Linear `catalogue.json` key→UUID resolution is a runtime prerequisite for `additional_teams` (0220 prior art) not captured; a missing mapping is a local gap the story frames as "remote".
- 🔵 **Clarity/Testability/Dependency suggestions**: AC11 positioning example; base-scope term drift; `Blocks: none` vs 0227 prose; external-API availability coupling.

### Assessment

Quality remains high and rising — clarity, completeness, scope, and dependency
all describe a well-formed unit; the residue is narrowing acceptance-criterion
precision. Both majors are one-line fixes: define cross-prefix ordering (prefix
then numeric, ties broken deterministically) and add a `max_pages`-override
criterion mirroring AC11. Note the convergence pattern: each pass resolves its
predecessor's majors and surfaces a strictly narrower successor. This is
sharpening, not churn, but it has clear diminishing returns — the item is
already implementable, and a COMMENT-level stop is defensible once these two
criteria are tightened.

### Post-Pass-4 Edits (applied, not re-reviewed)

Both pass-4 majors were fixed after this pass; no pass-5 re-review was run, so
the standing verdict above remains REVISE for the record. The edits:

- 🟡 **Clarity (ordering)** — reconcile order now specified as a total,
  deterministic order: ascending by identifier prefix (lexical), then ascending
  by numeric sequence within a prefix (`PP-2`, `PP-10`, `XX-3`). Summary
  "stable order" reworded to match; the AC now uses a multi-prefix example.
- 🟡 **Testability (`max_pages`)** — added an AC verifying a configured
  `max_pages` distinct from the default (20) is honoured, mirroring the
  `max_items` override AC; both override ACs now carry worked positioning
  examples (the AC11-positioning suggestion, folded in).

Standing minors — subsequently all addressed (applied, not re-reviewed):

- **Base-scope term drift** — Context canonicalised to "keyed base entity (the creation home resolved from 0228's canonical key)"; Summary "each validated at config time" qualified to "structurally validated".
- **`Blocks: none` vs 0227 prose** — replaced with an explicit Blocks entry gating the pull-block-validation portion of 0227.
- **Linear `catalogue.json` prerequisite** — Technical Note added: Linear entity resolution is catalogue-backed, a missing key is a local mapping gap not a remote absence.
- **AC8 discriminating scenario** — rephrased to team `additional_teams: [X]` + personal `[Y]` resolving to `Y` only, `X` dropped.
- **External-API availability** — Dependencies note added for the Jira/Linear remote-API coupling (inherited from the sync engine).

## Re-Review (Pass 5) — 2026-09-10

**Verdict:** REVISE

Ran clarity, dependency, scope, and testability. The five pass-4 minors and
both pass-4 majors are resolved. Three fresh majors surfaced — all three were
introduced by the pass-4/post-pass-4 edits themselves, not by any pre-existing
defect.

### Previously Identified Issues

- 🟡 **Clarity (multi-prefix ordering)** — Resolved (total order: prefix then numeric; multi-prefix AC example)
- 🟡 **Testability (`max_pages` override)** — Resolved as a *criterion existing*, but the new criterion is internally contradictory (see new major)
- 🔵 **Base-scope term / `Blocks` / catalogue / AC8 / external-API** (the five minors) — All addressed; two of the fixes introduced new majors (AC8, `Blocks`)

### New Issues Introduced (by the fixes)

- 🟡 **Dependency (high)**: The `Blocks` entry claims 0229 is "cross-referenced from 0227's Dependencies", but 0227 records `Blocks: none` and never mentions 0229 — the asserted reciprocal edge does not exist. (Also minor: 0228 does not record its reciprocal `Blocks: 0229` edge.)
- 🟡 **Testability (high)**: AC8 (team `[X]` + personal `[Y]` → `[Y]`) cannot distinguish whole-block replacement from a field-level override merge — both yield `[Y]`. A field present in team-only and omitted in personal is needed to exercise "wholly replaces".
- 🟡 **Testability (medium) + Clarity (medium)**: AC11/AC12 state ceilings "bound"/"cap" discovery to the configured value, but AC9/AC10 make crossing a ceiling an all-or-nothing *error*. With `max_items: 3` against a set of 5, the pull must error, not return 3 — the worked examples contradict the fail-loud semantics.

### Standing minors/suggestions (unchanged)

`max_items: 0` runtime behaviour undefined; ceiling "default" vs runtime-flag
precedence unstated; base-scope near-synonyms; catalogue prerequisite sits in
Technical Notes not Dependencies; total-order guarantee exercised by one example.

### Assessment

Convergence has turned adversarial: the fix→re-review loop is now seeding
majors as fast as it clears them, and every pass-5 major is an artefact of a
pass-4 edit. The work item has been implementable since pass 2. Recommended
path: one careful, holistic revision of the three contradictory/insufficient
acceptance criteria (not incremental one-liners), reconcile the 0227/0228
reciprocal edges, then stop — a sixth automated pass is likely to find the next
one-line successor rather than a substantive defect.

### Post-Pass-5 Edits (applied, not re-reviewed) — review closed here

All three pass-5 majors fixed with one careful, internally-consistent revision;
no pass 6 was run (deliberate stop). The edits:

- 🟡 **AC8 (whole-block replacement)** — team block now also sets `filters: {label: [a]}` which the personal block omits; the AC asserts both `X` and the team `filters` are dropped, so a field-level merge no longer passes.
- 🟡 **AC11/AC12 (ceiling semantics)** — reframed to the error path: `max_items: 3` against 5 discovered *errors* (default 25 would have completed); `max_pages: 5` against 6 pages *errors* (default 20 would have completed). Now consistent with AC9/AC10's all-or-nothing/fail-loud rule.
- 🟡 **0227/0228 reciprocal edges** — added the real edges: 0227 Dependencies now records the bidirectional 0229 relationship (its pull-block validation blocked by 0229); 0228 Dependencies now records `Blocks: 0229`. 0229's "cross-referenced from 0227" claim is now accurate.

Remaining non-blocking items left open (documented above): `max_items: 0`
runtime behaviour; ceiling "default" vs runtime-flag precedence; base-scope
near-synonyms; catalogue prerequisite placement; total-order single-example
coverage; and 0228's `Blocks` edge to 0220 (not added — 0220 is done, and its
blocked_by was not verified in this pass). These are acceptance-criterion polish
and dependency-graph completeness, not implementation blockers.

**Verdict: APPROVE** — set by the reviewer on 2026-09-10 after the post-pass-5
fixes, accepting them without a further automated pass. The pass-5 REVISE
reflected three defects introduced by the pass-4 edits; all three are now
resolved, and the remaining open items are non-blocking polish. The work item
is approved for planning; its status is moved to `ready`.
