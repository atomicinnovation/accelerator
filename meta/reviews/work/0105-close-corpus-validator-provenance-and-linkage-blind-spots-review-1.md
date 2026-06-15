---
type: work-item-review
id: "0105-close-corpus-validator-provenance-and-linkage-blind-spots-review-1"
title: "Work Item Review: Close the Corpus Validator Provenance and Linkage Blind Spots"
date: "2026-06-15T00:38:36+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
parent: "work-item:0057"
target: "work-item:0105"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 3
work_item_id: "0105"
tags: [frontmatter, schema, validator, provenance, linkage]
last_updated: "2026-06-15T07:41:38+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Close the Corpus Validator Provenance and Linkage Blind Spots

**Verdict:** REVISE

This is a strong, structurally complete task: every section is present and
substantively populated, the two blind spots are defined precisely and
re-verified at source, and the dependency analysis (decoupling the two
validator rules from the 0103-gated helper-collapse) is unusually careful. The
verdict is REVISE only because two **major testability** findings agree that
two acceptance criteria — the bare/path-shaped linkage rejection and the
helper-collapse — lack a pinned pass/fail boundary, which would leave a
verifier (and the fixture author) without a settled definition of "done". These
are tightening edits, not structural rework.

### Cross-Cutting Themes

- **Under-specified linkage-rejection boundary** (flagged by: testability,
  clarity) — testability flags that AC2 never enumerates which value forms
  must reject vs. accept; clarity flags that the contract for a valid
  `"doc-type:id"` reference is carried entirely by the un-glossed symbol
  `FM_TYPED_REF_RE`. Both point at the same gap: the acceptance bar for the
  linkage rule is expressed through a code identifier rather than an
  enumerated, reader-checkable set of inputs.
- **"Single oracle / authority" resolution is provisional** (flagged by:
  testability, clarity) — testability flags the disjunctive
  "removed or reduced to liveness checks" pass condition in AC4; clarity flags
  that the Drafting Notes themselves record the "single oracle" interpretation
  as conditional. Both converge on the same open decision: whether helpers are
  deleted outright or reduced to a *defined* liveness assertion.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: Helper-collapse criterion has a disjunctive,
  under-specified pass condition
  **Location**: Acceptance Criteria (4th bullet)
  AC4 states helpers "are removed or reduced to liveness checks" — a disjunction
  where the "reduced" branch has no defined success condition, so a verifier
  cannot tell whether a residual stub counts as passing or as incomplete
  collapse.

- 🟡 **Testability**: Bare/path-shaped linkage rejection lacks an enumerated set
  of inputs to reject and accept
  **Location**: Acceptance Criteria (2nd bullet)
  AC2 requires that a bare/path-shaped linkage value is rejected but does not
  enumerate the reject vs. accept set; borderline forms (bracketed-unquoted
  `[plan:0042]`, empty value, multi-element list with one bad element) are left
  undefined, so fixture author and verifier may disagree on the boundary.

#### Minor

- 🔵 **Dependency**: Potential parallel coupling with sibling work item 0104 on
  shared contract files left uncaptured
  **Location**: Dependencies
  The source 0103 record names sibling 0104 (the `adr` `status_vocab`
  follow-on) as a parallel child of epic 0057 touching the same schema/validator
  contract files. 0105's Dependencies names 0057, 0070, and the ADRs but not
  0104, so the merge-ordering relationship is not visible to whoever sequences
  the epic's children.

- 🔵 **Testability**: Fixture criterion does not specify the expected
  diagnostic, only that fixtures exist
  **Location**: Acceptance Criteria (3rd bullet)
  AC3 verifies fixture existence but not that each asserts the correct
  diagnostic; a fixture that rejects for the wrong reason would still satisfy
  the criterion as written.

- 🔵 **Testability**: "Stay green" criterion is a regression guard, not an
  outcome verifying this work item's intent
  **Location**: Acceptance Criteria (5th bullet)
  AC5 is mechanically checkable but tautological — claimable by any change that
  does not break the suites, including one that adds neither new rule. The
  substantive intent lives in AC1–AC4.

#### Suggestions

- 🔵 **Clarity**: Identifier-jargon (`FM_PROVENANCE_FIELDS`, `FM_TYPED_REF_RE`,
  diagnostic codes) used without inline gloss
  **Location**: Requirements
  The acceptance bar for the linkage rule is expressed partly through the
  un-glossed regex name `FM_TYPED_REF_RE`; the References link reaches the
  definition, but a one-line gloss at first use would let the requirement be
  evaluated without opening the script.

- 🔵 **Clarity**: Provenance over-emission relies on "iff" and "anchored" jargon
  used before definition
  **Location**: Summary
  The Summary's second blind-spot bullet uses "anchored" and "iff … anchored ⇒
  present direction" before `code_state_anchored` is named later; a top-down
  reader cannot resolve "anchored" until reaching Context.

- 🔵 **Clarity**: "Single oracle / authority" definition is
  acknowledged-but-conditional
  **Location**: Drafting Notes
  The conceptual spine term "single oracle" is defined provisionally — the
  Drafting Notes flag that if belt-and-braces coverage was intended, the
  helper-collapse requirement should be softened, leaving the central term's
  meaning resting on an unconfirmed interpretation.

- 🔵 **Completeness**: Frontmatter status (draft) consistent with body, but
  verify against in-flight readiness
  **Location**: Frontmatter: status vs body
  `status: draft` is consistent with the body header, but the item is otherwise
  fully fleshed out (detailed Requirements, six AC, verified line-references) —
  content typical of a ready item. Confirm `draft` is intentional.

- 🔵 **Scope**: Two validator rules are independently deliverable but bundled
  **Location**: Requirements
  The non-anchored provenance rule and the bare-linkage rule touch different
  code regions, carry distinct diagnostics, and could each ship/revert
  independently. Cohesion under one oracle-restoration goal justifies the
  bundle; noted only so it is a conscious choice.

### Strengths

- ✅ Both blind spots are defined precisely at first mention and re-explained
  with source-line detail in Context, so the core subjects never require the
  reader to guess; both line references were re-verified at source.
- ✅ Scope is consistent across Summary, Requirements, and Acceptance Criteria —
  both rules, both fixtures, the helper-collapse, and the green-suite guard
  recur across all three with no drift.
- ✅ The Assumptions section performs precise ordering analysis, decoupling the
  two validator rules (shippable early) from the helper-collapse step (the only
  part genuinely gated on 0103), so a 0103 slip is explicitly handled.
- ✅ Relates-to coverage is thorough — parent epic 0057, the originating
  migration 0070, and the governing ADRs (0033/0034/0040) are all named.
- ✅ Open Questions defer genuine implementation choices (delete vs.
  reduce-to-liveness; reuse vs. new diagnostic) as bounded decisions within the
  work rather than expanding scope.
- ✅ Correctly declared as a `task` with concrete, observable outcomes (the
  validator rejects specific bad cases) rather than vague desired properties.

### Recommended Changes

1. **Enumerate the linkage reject/accept set in AC2** (addresses: "Bare/path-shaped
   linkage rejection lacks an enumerated set of inputs", "Identifier-jargon …
   without inline gloss")
   List the concrete inputs the rule must reject (bare scalar `parent: 0042`,
   path-shaped `parent: docs/x.md`, bracketed-unquoted `[plan:0042]`) and at
   least one it must still accept (quoted `parent: "work-item:0042"`,
   omitted/empty). A one-line gloss of `FM_TYPED_REF_RE` (the regex matching a
   quoted `"doc-type:id"` token) closes the clarity half of the same gap.

2. **Define the "reduced" branch of AC4** (addresses: "Helper-collapse criterion
   has a disjunctive, under-specified pass condition", "'Single oracle /
   authority' definition is acknowledged-but-conditional")
   State the assertable condition for a retained helper — e.g. "each retained
   helper asserts only that `validate-corpus-frontmatter.sh` rejects the
   previously-uncaught bad fixture, and no longer re-derives the rule
   independently". Confirming the intended interpretation also lets the "single
   oracle" spine term be restated definitionally and the Drafting Notes hedge
   dropped.

3. **Tie AC3 fixtures to their diagnostics** (addresses: "Fixture criterion does
   not specify the expected diagnostic")
   Require each new fixture to assert rejection with the specific diagnostic for
   its rule (`FORBIDDEN-PROVENANCE-NONANCHORED`; the chosen linkage code per the
   Open Question), so a fixture cannot pass while exercising the wrong failure
   path.

4. **Add sibling 0104 to the Relates-to line** (addresses: "Potential parallel
   coupling with sibling work item 0104")
   Note 0104 as the sibling schema-vocab follow-on under 0057 touching the same
   contract files, so the shared-file/merge-ordering coupling is on the record.

5. **Note AC5 as a regression gate, not a sufficiency criterion** (addresses:
   "'Stay green' criterion is a regression guard")
   Optionally strengthen it to require that the new AC3 fixtures are actually
   run by `test:integration:config`, tying suite greenness to the new rules
   executing.

6. **Confirm the `draft` status is intentional** (addresses: "Frontmatter status
   … verify against in-flight readiness")
   If the specification is settled (after the above tightening), advance to
   `ready` so the frontmatter reflects the item's maturity. This is a separate
   workflow decision, not part of the review edit.

## Per-Lens Results

### Clarity

**Summary**: Work item 0105 is exceptionally clear for a deeply technical
schema-oracle task: the two blind spots are named, defined inline, and
reinforced consistently across Summary, Context, Requirements, Acceptance
Criteria, and Technical Notes with no detectable cross-section contradiction.
The only clarity risks are heavy reliance on undefined identifier-jargon
(`FM_PROVENANCE_FIELDS`, `FM_TYPED_REF_RE`, `BAD-LINKAGE-SHAPE`,
`code_state_anchored`) that a reader leans on links to resolve, and a single
self-aware terminology gap around what "single oracle / authority" means.

**Strengths**:
- Both blind spots are defined precisely at first mention in the Summary, then
  re-explained with source-line detail in Context.
- Scope is consistent across sections: Summary's two-fix framing maps
  one-to-one onto two Requirements bullets and two Acceptance Criteria.
- The Assumptions section pre-empts a likely consistency question by separating
  the two validator rules (independent of 0103) from the helper-collapse step.
- Outcomes are stated as observable system states, not vague desired properties.

**Findings**:
- 🔵 suggestion (medium) — **Summary**: Provenance over-emission relies on "iff"
  and "anchored" jargon used before definition. The Summary's second blind-spot
  bullet hinges on "anchored" and "the 'iff' is enforced only in the anchored ⇒
  present direction", both appearing before `code_state_anchored` is named
  later. Suggestion: gloss "anchored" as "a type whose `code_state_anchored` is
  `yes`" on first use.
- 🔵 suggestion (high) — **Requirements**: Identifier-jargon used without inline
  gloss. The acceptance bar ("rejected unless a quoted `"doc-type:id"`
  reference") is expressed partly through the un-glossed regex name
  `FM_TYPED_REF_RE`. Suggestion: keep the References link or add a one-line
  gloss at first use.
- 🔵 suggestion (low) — **Drafting Notes**: "Single oracle / authority"
  definition is acknowledged-but-conditional. The conceptual spine term is
  defined provisionally; once intent is confirmed, restate it definitionally
  and drop the conditional hedge.

### Completeness

**Summary**: Work item 0105 is a structurally complete, kind-appropriate task.
Every expected section is present and substantively populated: a precise
two-part Summary, a Context that explains the three-authority motivation,
specific Requirements, six concrete Acceptance Criteria, populated Open
Questions, Dependencies, Assumptions, Technical Notes, and References.
Frontmatter is well-formed with a recognised `kind: task` and valid
`status: draft`. No completeness gaps would force an implementer to ask
follow-up questions.

**Strengths**:
- Summary states the work as an unambiguous action and enumerates both blind
  spots concretely.
- Context explains why the work is needed (the deliberate temporary
  three-authority state) rather than restating the Summary.
- Requirements are specific and actionable — each new rule, the fixtures, the
  helper-collapse, and the green-build constraint are individually called out.
- Six Acceptance Criteria are present, each mapping back to a distinct
  requirement.
- Open Questions, Dependencies, Assumptions, and Technical Notes are genuinely
  populated, and the `kind: task` frontmatter is complete and valid.

**Findings**:
- 🔵 suggestion (medium) — **Frontmatter: status vs body**: Frontmatter status
  (draft) and body status header (Draft) agree, but the item is otherwise fully
  fleshed out — content typical of a ready item. Suggestion: confirm `draft` is
  intentional; if settled, advance to `ready`.

### Dependency

**Summary**: This task captures its couplings unusually well: the upstream
blocker (0103) is named in both frontmatter (`blocked_by`) and the Dependencies
body, the Relates-to set spans the parent epic, the originating migration, and
the governing ADRs, and the Assumptions section explicitly decouples the two
validator rules from the helper-collapse step. The only uncaptured coupling is
a potential parallel edit conflict with sibling work item 0104, which the source
0103 record names as another child of the same epic touching the same contract
files.

**Strengths**:
- The single upstream blocker (0103) is captured consistently in both
  frontmatter and the Dependencies body, with the Drafting Notes recording the
  deliberate `relates_to` → `blocked_by` move.
- The Assumptions section performs precise ordering analysis, distinguishing the
  two validator rules (shippable early) from the 0103-gated helper-collapse.
- Relates-to coverage is thorough — epic 0057, migration 0070, and ADRs
  0033/0034/0040 are all named.

**Findings**:
- 🔵 minor (medium) — **Dependencies**: Potential parallel coupling with sibling
  work item 0104 on shared contract files left uncaptured. The 0103 record names
  0104 (the `adr` `status_vocab` follow-on) as a parallel child of 0057 touching
  the same schema source; 0105 edits the closely-related validator and stays
  data-driven from the same files, so the two changes touch the same contract
  surface concurrently. Suggestion: add 0104 to the Relates-to line.

### Scope

**Summary**: Work item 0105 is a well-bounded task that folds two related
validator blind spots into a single oracle, plus collapses the corresponding
bespoke guard helpers introduced by 0103. The two validator rules and the
helper-collapse share one coherent purpose — restoring single-authority
enforcement — and the work item explicitly anticipates the partial-independence
of its three sub-deliverables. Sizing fits the declared `task` kind; the only
mild scope signal is that the two validator rules are functionally independent
and could be delivered separately, though their thematic cohesion keeps them
defensibly bundled.

**Strengths**:
- Tightly scoped to one theme — both blind spots are validator-oracle gaps on
  the same frontmatter contract, all serving the single goal of restoring
  single-authority enforcement.
- The Assumptions section explicitly reasons about the internal dependency
  structure (rules independent of 0103; only helper-collapse gated).
- Summary, Requirements, and Acceptance Criteria describe the same scope
  consistently with no drift.
- Correctly declared as a task rather than a spike — concrete deliverable, not
  open-ended investigation.
- Open Questions defer genuine implementation choices without expanding scope.

**Findings**:
- 🔵 suggestion (medium) — **Requirements**: Two validator rules are
  independently deliverable but bundled. They touch different code regions, carry
  distinct diagnostics, and could each ship/test/revert independently. Impact is
  minimal given the shared theme; noted only so the bundling is a conscious
  choice. No action needed unless the rules are expected to land at materially
  different times.

### Testability

**Summary**: This is a well-specified task with five Acceptance Criteria that
mostly map to concrete, verifiable outcomes — two new rejection behaviours,
fixture additions, a helper-collapse, and green test suites. The chief
testability gap is that two criteria (the bare/path-shaped linkage rejection and
the helper-collapse) admit ambiguity about the exact input that must be rejected
and what "liveness check" conclusively means, leaving a verifier without a fully
pinned-down pass/fail boundary. The "stay green" criteria are tautology-adjacent
but acceptable as regression guards.

**Strengths**:
- AC1 and AC2 specify observable rejection outcomes by a named oracle, so a
  verifier can construct a fixture and check for non-zero exit / diagnostic.
- The Requirements and Technical Notes provide concrete input examples that
  ground the otherwise abstract rejection criteria.
- AC3 demands failure-mode fixtures mirroring an existing `assert_rejects`
  pattern, giving the verification mechanism a defined home and shape.
- The work item correctly defers the delete-vs-reduce-to-liveness choice to an
  Open Question rather than baking an unverifiable "should" into a criterion.

**Findings**:
- 🟡 major (high) — **Acceptance Criteria (4th bullet)**: Helper-collapse
  criterion has a disjunctive, under-specified pass condition. "Removed or
  reduced to liveness checks" leaves the "reduced" branch with no defined
  success condition, so a residual stub could be argued as passing or as
  incomplete collapse. Suggestion: state the assertable condition for the
  "reduced" branch (helper asserts only that the validator rejects the bad
  fixture, no longer re-deriving the rule).
- 🟡 major (medium) — **Acceptance Criteria (2nd bullet)**: Bare/path-shaped
  linkage rejection lacks an enumerated set of inputs to reject and accept.
  Borderline forms (bracketed-unquoted `[plan:0042]`, empty value, multi-element
  list with one bad element) are not stated as must-reject or must-pass.
  Suggestion: enumerate the concrete reject inputs and at least one accept input.
- 🔵 minor (medium) — **Acceptance Criteria (3rd bullet)**: Fixture criterion
  does not specify the expected diagnostic, only that fixtures exist. A fixture
  that rejects for the wrong reason would still satisfy the criterion.
  Suggestion: tie the criterion to the diagnostic each fixture must assert.
- 🔵 minor (high) — **Acceptance Criteria (5th bullet)**: "Stay green" criterion
  is a regression guard, not an outcome verifying intent. It is claimable by any
  change that does not break the suites. Suggestion: keep as a regression gate
  but note it is not a sufficiency criterion, or require the new fixtures to be
  run by `test:integration:config`.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-06-15T00:44:59+00:00

**Verdict:** COMMENT

Both pass-1 major findings are resolved by the AC2/AC4 edits, and completeness
is now clean. One **new major** was introduced as a side-effect of the edit: the
0104 coupling was added to the Dependencies *prose* but not to the typed
frontmatter `relates_to`, so the linkage graph still omits it. With a single
major and no criticals, the verdict relaxes from REVISE to COMMENT — the work
item is acceptable to implement, but closing the frontmatter-linkage gap is
recommended.

### Previously Identified Issues
- 🟡 **Testability**: Helper-collapse criterion has a disjunctive,
  under-specified pass condition — **Resolved** (AC4 now defines the "reduced"
  branch: a retained helper asserts only that the validator rejects the bad
  fixture and no longer re-derives the rule; testability downgraded it to minor).
- 🟡 **Testability**: Bare/path-shaped linkage rejection lacks an enumerated set
  of inputs — **Resolved** (AC2 now enumerates the reject set — bare scalar,
  path-shaped, bracketed-unquoted, mixed list — and the accept set; testability
  now records it as a strength).
- 🔵 **Testability**: Fixture criterion does not specify the expected diagnostic
  — **Resolved** (AC3 now requires each fixture to assert its specific
  diagnostic, not merely non-zero exit).
- 🔵 **Testability**: "Stay green" criterion is a regression guard — **Resolved**
  (AC5 now self-describes as a regression gate, not a sufficiency criterion, and
  ties greenness to the new fixtures executing).
- 🔵 **Dependency**: Parallel coupling with 0104 uncaptured — **Partially
  resolved** (named in Dependencies prose with a merge-ordering rationale, but
  not yet in typed frontmatter `relates_to` — see new finding below).
- 🔵 **Clarity**: "iff"/"anchored" jargon before definition — **Resolved**
  (Summary now glosses "non-anchored" as `code_state_anchored` not `yes` and
  spells out the forward/reverse direction).
- 🔵 **Clarity**: `FM_TYPED_REF_RE` without gloss — **Partially resolved**
  (glossed at AC2; clarity notes the gloss still trails the first use in
  Requirements — minor).
- 🔵 **Clarity**: "single oracle" provisional definition — **Still present**
  (minor; the Drafting Notes reconcile it but Summary/Context reach the reader
  first — left as a deliberate deferral tied to the Open Question).
- 🔵 **Completeness**: `status: draft` vs readiness — **Open** (intentionally not
  changed during review; a separate workflow decision).
- 🔵 **Scope**: Two rules bundled — **Unchanged** (suggestion only; cohesion
  justifies the bundle, no action recommended).

### New Issues Introduced
- 🟡 **Dependency**: 0104 merge-ordering coupling stated in prose but absent from
  typed frontmatter linkage — the Dependencies prose now says the two items
  "should be merge-ordered rather than landed blind in parallel", but the
  frontmatter carries only `blocked_by` and `parent`, so tooling reading the
  typed graph sees no edge to 0104. Fix: add `relates_to: ["work-item:0104"]`
  (optionally also 0070 and the ADRs) to the frontmatter.
- 🔵 **Testability**: AC3 linkage-diagnostic assertion is only conditionally
  specified (deferred to the Open Question) — minor; resolvable by recording the
  chosen diagnostic before marking ready.
- 🔵 **Testability** / **Scope**: "no re-encoded contract" data-driven
  requirement has no verification procedure — suggestion to add a criterion
  (adding a `FM_PROVENANCE_FIELDS` member forbids that field with no validator
  edit).

### Assessment
The work item is in good shape and acceptable to implement as a COMMENT. The
only finding worth acting on before implementation is the dependency major —
promoting the 0104 coupling from prose into typed frontmatter `relates_to` so
the merge-ordering constraint is visible to planning tooling. The remaining
minors/suggestions are polish (pull the `FM_TYPED_REF_RE` gloss forward, record
the linkage diagnostic, optionally add a data-driven verification criterion).

## Re-Review (Pass 3) — 2026-06-15T07:41:38+00:00

**Verdict:** APPROVE

The pass-2 dependency major and the remaining actionable findings have been
addressed in the work item, so the verdict advances to APPROVE.

### Previously Identified Issues
- 🟡 **Dependency**: 0104 merge-ordering coupling absent from typed linkage —
  **Resolved** (`relates_to: ["work-item:0104", "work-item:0070", "adr:ADR-0033",
  "adr:ADR-0034", "adr:ADR-0040"]` added to the work item frontmatter).
- 🔵 **Testability**: AC3 linkage-diagnostic only conditionally specified —
  **Resolved** (the linkage Open Question is settled — reuse `BAD-LINKAGE-SHAPE`
  — and AC3 now names the diagnostic concretely).
- 🔵 **Clarity**: `FM_TYPED_REF_RE` gloss trailed first use — **Resolved**
  (gloss pulled forward to its first appearance in Requirements).
- 🔵 **Clarity**: "single oracle" framing — **Accepted as-is** (deliberately
  tied to the open delete-vs-reduce decision; reconciled in Drafting Notes).
- 🔵 **Testability/Scope**: "no re-encoded contract" verification procedure —
  **Accepted as-is** (optional polish; the Assumptions section already implies
  the data-driven behaviour).

### New Issues Introduced
- None.

### Assessment
All majors across all passes are resolved and no new issues remain. The work
item is approved and ready for implementation. (Also of note: investigating the
linkage Open Question surfaced a framing correction — the existing
"bare-number"/"path-shape" fixtures are quoted and already caught; the genuine
blind spot is the *unquoted* forms — now captured in the Technical Notes so the
implementer targets the right gap.)
