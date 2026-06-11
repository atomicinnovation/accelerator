---
type: work-item-review
id: "0104-add-rejected-to-adr-status-vocabulary-review-1"
title: "Work Item Review: Add rejected to the ADR Status Vocabulary in the Unified Schema"
date: "2026-06-11T12:51:55+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0104"
relates_to: ["work-item:0103"]
work_item_id: "0104"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: [frontmatter, schema, adr, status, validator]
last_updated: "2026-06-11T12:56:01+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Add rejected to the ADR Status Vocabulary in the Unified Schema

**Verdict:** COMMENT

This is a tightly-scoped, exceptionally well-specified task work item. All five
lenses converge on the same assessment: the work is a single coherent unit
(adding `rejected` to one schema vocabulary and propagating it through coupled
re-encodings and tests), every claim is anchored to a named file and line, and
the consistency claims against ADR-0031 and ADR-0042 check out against the
referenced source documents. No critical or major findings surfaced. The
handful of minor findings and suggestions concern referent precision and the
fragility of line-number anchors rather than any substantive gap — the item is
acceptable as-is and ready for implementation.

### Cross-Cutting Themes

- **Line/id-keyed references may drift** (flagged by: clarity, testability) —
  The clarity lens notes "this id" / "this work item's id" never states the
  literal key (`0104`) an implementer must grep for, and the testability lens
  notes the `templates/adr.md:8` acceptance criterion pins to a line number
  that becomes ambiguous if the template shifts. Both point at the same
  underlying fragility: references that are precise today but positionally
  brittle. Cheap to harden by quoting the literal value/content instead.

### Findings

#### Critical

_None._

#### Major

_None._

#### Minor

- 🔵 **Dependency**: Coupling to 0103 captured only as upstream blocker, not as the deferral this completes
  **Location**: Dependencies
  This work item completes a deferral that 0103 deliberately left open (0103
  created the skip_test keyed to this id so 0104 would later flip it live), but
  only the "Blocked by" direction is recorded — the closed loop is visible from
  one side only.

- 🔵 **Testability**: Final criterion ("stay green") is broad enough to pass without the new rejected coverage
  **Location**: Acceptance Criteria
  "Suites stay green" could be satisfied by suites that never exercise a
  rejected ADR; green-ness alone does not prove the new behaviour is covered.
  The new fixture and flipped conformance assertion carry the real verification
  weight.

- 🔵 **Testability**: Line-number anchor in the templates criterion may drift, weakening verifiability
  **Location**: Acceptance Criteria
  The `templates/adr.md:8` criterion pins verification to a specific line; if
  the template shifts, the line-8 anchor stops resolving and the check becomes
  ambiguous. Phrasing against content keeps it verifiable.

#### Suggestions

- 🔵 **Clarity**: "this id" / "this work item's id" relies on the reader knowing the item's own id
  **Location**: Requirements
  The `skip_test` is "keyed to this work item's id" without naming the literal
  value (`0104`) — the concrete string an implementer must grep for. Only the
  Technical Notes comment ("Flips to a live assert_check when 0104 lands")
  disambiguates it.

- 🔵 **Clarity**: ADR/TSV acronyms used without first-use expansion
  **Location**: Context
  `ADR`, `TSV`, and `BAD-STATUS` are used without expansion. All are recoverable
  from project context and surrounding file references; none is defined at first
  mention.

- 🔵 **Clarity**: "the producer" used in two slightly different senses
  **Location**: Technical Notes
  "Producer" appears both as the frontmatter field (`producer: create-work-item`)
  and as a role (`review-adr` is "the producer" the schema aligns to). Context
  disambiguates, but the dual usage could momentarily conflate the two.

- 🔵 **Dependency**: Sibling-under-0057 ordering relative to this child is not positioned
  **Location**: Dependencies
  As a child of epic 0057, this item may have siblings each fixing one
  divergence; the section names the parent but does not state whether this child
  is independent of siblings that might touch the same TSV row or fixture files.

### Strengths

- ✅ Every technical claim is pinned to an explicit referent (file path plus
  line number), eliminating pronoun ambiguity and making the item
  implementer-ready.
- ✅ Strong internal consistency: Summary, Requirements, Acceptance Criteria,
  and Technical Notes all describe the same narrow change with no section over-
  or under-reaching.
- ✅ The Context section cleanly distinguishes the three actors in the
  divergence (ADR-0031 as source of truth, `review-adr` as producer, the TSV as
  incomplete schema), so "the producer is right and the schema source is
  incomplete" resolves unambiguously.
- ✅ The Technical Notes read-site map distinguishes data-driven
  (auto-propagating) sites from hand-coupled re-encodings, surfacing every
  coupling the edit touches.
- ✅ Open Questions cleanly fences the out-of-scope filtering/visualisation
  concern away from the in-scope schema change.
- ✅ Acceptance Criteria are concrete and observable: literal vocab strings to
  grep for, a fixture that asserts acceptance, a specific skip_test to remove,
  named suites that must pass.
- ✅ The `task` kind matches the scope: a localised, indivisible schema
  correction with deterministic test verification.

### Recommended Changes

These are all optional polish — none blocks implementation.

1. **State the literal skip_test key inline** (addresses: "this id" relies on
   reader knowing the item's own id) — In Requirements, write "the `skip_test`
   is keyed to id `0104`" so the grep target is explicit at the point of action.

2. **Re-phrase the templates and "stay green" criteria against content rather
   than position** (addresses: line-number anchor may drift; "stay green" too
   broad) — Change the `templates/adr.md:8` criterion to assert the `status:`
   vocab comment contains `rejected` and verbatim-matches the TSV cell; cross-
   reference the "suites stay green" criterion to the new rejected fixture and
   live conformance assertion so green-ness implies the new coverage exists.

3. **Note the 0103 loop closure in Dependencies** (addresses: coupling to 0103
   captured only as upstream blocker) — Add a one-line "completes 0103's
   deferred `rejected` conformance axis" alongside the "Blocked by" entry so the
   relationship reads as a closed loop.

4. **Optionally note sibling independence under 0057** (addresses: sibling
   ordering not positioned) — If this child is isolated to the `adr` TSV row,
   say so in Dependencies; if a sibling touches the same row/fixture, note the
   intended ordering.

## Per-Lens Results

### Clarity

**Summary**: This work item communicates with exceptional precision: nearly
every claim is anchored to a named file, line number, or document reference, and
the Summary, Context, Requirements, and Acceptance Criteria all describe the same
narrow change with no cross-section contradictions. The consistency claims
against ADR-0031 and ADR-0042 check out against the referenced source documents.
A few unexpanded acronyms/terms and one shifting referent for "this id" are the
only minor clarity concerns in an otherwise unambiguous item.

**Strengths**:
- Every technical claim is pinned to an explicit referent (file path plus line
  number), eliminating pronoun ambiguity throughout.
- Internal consistency is strong: the Summary's narrow scope is matched exactly
  by Requirements, Acceptance Criteria, and Technical Notes.
- Context explicitly distinguishes the three actors in the divergence so "the
  producer is right and the schema source is incomplete" resolves unambiguously.
- Open Questions cleanly fences out-of-scope concerns from the in-scope schema
  change.

**Findings**:
- 🔵 suggestion (confidence: medium) — **ADR/TSV acronyms used without first-use
  expansion** (Context): `ADR`, `TSV`, and `BAD-STATUS` are used without
  expansion. Each is recoverable from project context and surrounding file
  references; none is defined or linked at first mention. Optionally expand
  `ADR`/`TSV` on first use; no change needed for `BAD-STATUS` (shown as a literal
  token).
- 🔵 suggestion (confidence: medium) — **"this id" / "this work item's id"
  relies on the reader knowing the item's own id** (Requirements): the phrase
  refers to id `0104` without naming it inline. The prose never states the
  literal value the `skip_test` is keyed to — the concrete thing an implementer
  must grep for. State the literal key inline once.
- 🔵 suggestion (confidence: low) — **"the producer this aligns to" uses
  "producer" in two slightly different senses** (Technical Notes): "producer"
  appears both as the frontmatter field and as a role (`review-adr`). Context
  disambiguates, but consider qualifying the role usage on first mention.

### Completeness

**Summary**: This task work item is exceptionally complete for its kind. Every
standard section (Summary, Context, Requirements, Acceptance Criteria, Open
Questions, Dependencies, Technical Notes, References) is present and
substantively populated, and the frontmatter carries all required fields with
recognised values. The work to be done, why it is needed, and how to verify
completion are all clearly stated, leaving no obvious gaps.

**Strengths**:
- Summary is a clear, unambiguous action statement naming the exact change and
  the files affected.
- Context fully explains the motivation with specific citations rather than
  restating the summary.
- Requirements are concrete and implementer-ready, distinguishing hand-edited
  coupled re-encodings from auto-propagating data-driven read sites.
- Acceptance Criteria contains five specific, enumerated criteria that map
  directly to the requirements.
- Frontmatter is fully populated and valid.
- Open Questions and Dependencies carry genuinely relevant content rather than
  placeholders.

**Findings**: None.

### Dependency

**Summary**: From the dependency lens this work item is very thoroughly mapped:
the single upstream blocker (work item 0103) is explicitly named with the exact
mechanism of the coupling, and the parent epic, originating migration, and
governing ADRs are all captured as relations. There are no external systems or
cross-team actions implied by a purely internal schema/test edit, and the one
downstream consideration (visualiser filtering) is deliberately scoped out with
a note that no current consumer is blocked. The only minor gaps are an unstated
direction on the 0103 coupling and the unpositioned sibling ordering under 0057.

**Strengths**:
- The sole upstream blocker (0103) is captured with the precise mechanism of
  coupling — the skip_test left keyed to this work item's id.
- All contextual relations (0057, 0070, ADR-0031, ADR-0042) are named and
  cross-referenced, leaving no implied governing document uncaptured.
- Open Questions correctly distinguishes an out-of-scope future consumer
  (visualiser filtering) from a live downstream coupling.
- The Technical Notes read-site map makes every internal coupling the edit
  touches explicit.

**Findings**:
- 🔵 minor (confidence: medium) — **Coupling to 0103 captured only as upstream
  blocker, not as the deferral this completes** (Dependencies): 0103 created the
  skip_test keyed to this id precisely so this work item would later flip it
  live, making 0103 both an upstream blocker and a downstream beneficiary whose
  deferred axis this closes out. Only the blocker direction is recorded. Note in
  Dependencies that completing this item closes 0103's deferred axis.
- 🔵 suggestion (confidence: low) — **Sibling-under-0057 ordering relative to
  this child is not positioned** (Dependencies): as a child of epic 0057, this
  item likely has siblings each fixing one divergence; the section names the
  parent but does not state any ordering relationship. If siblings touch the
  same TSV/fixture, an uncaptured ordering could surface under concurrent
  scheduling. State independence, or note the intended ordering.

### Scope

**Summary**: This work item describes one coherent, atomic unit of work: adding
the value `rejected` to a single schema vocabulary and propagating that single
change through its coupled re-encodings and tests. All requirements serve one
purpose — making a rejected ADR schema-valid — and the boundaries are explicitly
stated, with the filtering/visualisation question fenced off as out of scope.
The declared `task` kind is appropriate for the scope described.

**Strengths**:
- All five requirements serve a single unified purpose; no bundling of
  independent concerns.
- Summary, Requirements, and Acceptance Criteria describe the same scope
  consistently with no drift.
- Scope boundaries are stated explicitly — Open Questions fences off the
  distinct filtering/styling concern.
- Context justifies why this is a standalone unit (a deferred child of the 0103
  audit under epic 0057).
- The `task` kind matches the scope: a localised, indivisible schema correction
  with deterministic test verification.

**Findings**: None.

### Testability

**Summary**: This task carries an unusually testable specification: every
Acceptance Criterion names a concrete file, line, or test suite and a definite
pass/fail check. The criteria collectively cover the Summary's intent — making a
rejected ADR schema-valid — and one criterion is strengthened by an explicit
example of the expected vocab string. Minor gaps: one criterion relies on a
moving line-number reference, and "stay green" could be tightened to name the
assertions that must exist.

**Strengths**:
- Each Acceptance Criterion defines a concrete, observable outcome.
- The first two criteria quote the exact expected string, giving a verifier a
  literal target to grep for.
- The criteria collectively cover the Summary's intent including both the schema
  source and the coupled re-encoding.
- Verification procedures are spelled out in Requirements and Technical Notes.

**Findings**:
- 🔵 minor (confidence: medium) — **Final criterion ("stay green") is broad
  enough to pass without the new rejected coverage** (Acceptance Criteria):
  green-ness does not by itself prove the new behaviour is covered; the new
  fixture and flipped conformance assertion carry the real verification weight.
  Cross-reference the green-suite criterion to the rejected fixture/assertion.
- 🔵 suggestion (confidence: medium) — **Line-number anchor in the templates
  criterion may drift** (Acceptance Criteria): the `templates/adr.md:8`
  criterion pins to a line number that becomes ambiguous if the template shifts.
  Phrase the check against content (vocab comment contains `rejected` and
  verbatim-matches the TSV cell) instead of line.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-06-11

**Verdict:** COMMENT

All seven findings from the initial pass are resolved by the edits. The
re-run lenses raised no critical or major issues; the new findings are minor/
suggestion-level and mostly surface pre-existing prose (glosses, an untracked
deferred decision) rather than regressions introduced by the edits.

### Previously Identified Issues

- 🔵 **Clarity**: "this id" relies on the reader knowing the item's own id — **Resolved.** Requirements now reads "keyed to this work item's id (`0104`)".
- 🔵 **Clarity**: ADR/TSV acronyms without first-use expansion — **Resolved / not re-raised.** No longer flagged; the clarity lens this pass praised inline term definitions.
- 🔵 **Clarity**: "the producer" used in two senses — **Resolved.** "the producer (`review-adr`)" is now cited as a strength (key terms defined inline).
- 🔵 **Dependency**: 0103 coupling captured only as upstream blocker — **Resolved.** Dependencies now states completing 0104 discharges the deferral 0103 left open; cited as a strength this pass.
- 🔵 **Dependency**: sibling-under-0057 ordering not positioned — **Resolved.** The sibling-scope note (isolated to the `adr` TSV row, sequence if a sibling collides) is now cited as a strength.
- 🔵 **Testability**: "stay green" too broad — **Resolved.** AC5's anti-tautology clause is now cited as a strength ("anticipates the classic tautology trap").
- 🔵 **Testability**: line-number anchor may drift — **Resolved.** The templates criterion is now content-based (verbatim-match the TSV cell, "regardless of line drift").

### New Issues Introduced

None are regressions from the edits, except where noted:

- 🔵 **Testability** (minor): AC5's "present" condition lacks a defined verification procedure — partly a consequence of the new anti-tautology clause; the clause demands the fixture/assertion be "present and not skipped" without saying how a verifier confirms it (e.g. skip count dropped to zero in the run output).
- 🔵 **Testability** (minor): AC4 verifies removal of the `skip_test`, not that the replacement assertion actually feeds `status: rejected` through `assert_check`/`assert_accepts` — pre-existing wording, newly surfaced.
- 🔵 **Testability** (suggestion): No single criterion restates the Summary's headline outcome (validator no longer emits `BAD-STATUS` for `status: rejected`); covered collectively by AC2–AC4.
- 🔵 **Dependency** (suggestion): the deferred visualiser-filtering decision in Open Questions names a downstream consumer but no tracking item — pre-existing Open Question, newly surfaced.
- 🔵 **Clarity** (suggestion): `BAD-STATUS`, `0070`, and "axis" used without inline glosses — pre-existing prose, newly surfaced; all recoverable from context.

### Assessment

The work item is ready for implementation. Every previously identified finding
is resolved, and no regressions were introduced. The new findings are all
minor/suggestion-level and reflect the lenses surfacing fresh angles on
pre-existing prose rather than defects in the edits — the kind of polish that
yields diminishing returns.

**Verdict updated to APPROVE** after the AC4 reword landed (the conformance
criterion now pins the `status: rejected` input and the accept outcome as the
pass condition, closing the last substantive testability gap). The work item is
approved for implementation.
