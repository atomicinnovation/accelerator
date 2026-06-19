---
type: work-item-review
id: "0118-reconcile-0007-backfill-sentinel-with-validator-review-1"
title: "Work Item Review: Reconcile 0007 Backfill Sentinel With Its Validator"
date: "2026-06-19T23:35:19+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0118"
parent: "work-item:0115"
relates_to: []
work_item_id: "0118"
reviewer: Toby Clemson
verdict: "APPROVE"
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: [migrate, interactive-migration, agent-invocation, tooling]
last_updated: "2026-06-19T23:47:28+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Reconcile 0007 Backfill Sentinel With Its Validator

**Verdict:** REVISE

This is a tightly-scoped, structurally complete task: every standard section
is present and substantively filled, the single concern (write an accepted
sentinel on the no-derivable-default backfill path) holds consistently across
Summary, Requirements, and Acceptance Criteria, and the Technical Notes pin
exact source and validator line numbers. The findings that push this to
REVISE are not about what the item *is* but about under-specified contract
details: the sentinel token is left exemplary rather than fixed (which
ripples into testability, scope, and downstream-consumer concerns), the
end-to-end abort symptom is not asserted by any criterion, and the
concurrent-edit relationship with sibling 0114 is recorded only as "relates
to" despite a flagged live contradiction.

### Cross-Cutting Themes

- **Unfixed sentinel token** (flagged by: testability, scope, dependency) —
  The sentinel value is written as "(e.g. `pending`)" everywhere and never
  committed to. This single ambiguity surfaces three ways: a verifier cannot
  assert a definite literal (testability), the "per-extra sentinel needed?"
  assumption can silently grow the unit of work (scope), and the token becomes
  a persisted corpus-wide value that downstream consumers will read
  (dependency). Fixing the literal — or explicitly declaring "any non-empty
  token, assert presence + non-emptiness only" — resolves all three at once.
- **0114 relationship ambiguity** (flagged by: clarity, dependency) — The item
  both treats 0114 as owning live "broader backfill completeness work" and
  states this fix "remains required despite 0114 being considered complete,"
  while both items mutate the same 0007 backfill branch. The status reads
  inconsistently (clarity) and the concurrent-edit coupling is under-recorded
  (dependency).

### Findings

#### Critical

_None._

#### Major

- 🟡 **Dependency**: Concurrent-edit coupling with 0114 captured only as
  'relates to'
  **Location**: Dependencies
  0118 and 0114 both mutate the same artefact — 0007's required-extras backfill
  (0007:502-512) — yet the relationship is recorded only as "Relates to: 0114"
  with "Blocked by: none." The Assumptions section flags a live conflict, so an
  implementer could land this sentinel write on top of (or have it reverted by)
  0114's changes to the same branch.

- 🟡 **Testability**: Sentinel value is exemplary, not fixed, leaving the pass
  condition under-specified
  **Location**: Acceptance Criteria
  The criteria and Requirements refer to "an accepted sentinel value" /
  "(e.g. `pending`)" without committing to one concrete token. A verifier
  cannot write a definitive assertion (e.g. grep for `pr_number: pending`)
  without knowing the exact value to expect.

- 🟡 **Testability**: No criterion verifies the end-to-end abort no longer
  occurs
  **Location**: Acceptance Criteria
  Criterion 1 asserts `self_validate_structural` passes, but no criterion
  exercises the original failure path end-to-end — running 0007 against a
  corpus with a tracker-key PR file and asserting the run no longer aborts with
  `FAIL: ... MISSING-EXTRA` before the interactive stage (the precise reported
  symptom).

#### Minor

- 🔵 **Clarity**: Contradictory statements about whether 0114 is complete
  **Location**: Assumptions
  Requirements and Dependencies say 0114 *owns the broader backfill completeness
  work* (implying outstanding work), while the second Assumption says this fix
  "remains required despite 0114 being considered complete." A reader cannot
  tell whether 0114 is done or in progress.

- 🔵 **Clarity**: Bare letter 'C' / 'fix C of 0115' has an ambiguous referent
  **Location**: Summary
  The item identifies itself as "fix C of 0115" but never states what the "C"
  lettering enumerates. The research uses "C" as one of five fix *options*
  (A–E) while 0115 adopts a different direction, so a reader cannot tell whether
  "fix C of 0115" means the research's option C or a sub-fix internal to 0115.

- 🔵 **Testability**: Criterion 3 regression guard lacks a concrete
  derivable-default input
  **Location**: Acceptance Criteria
  Criterion 3 verifies a required extra with a derivable default is written
  unchanged, but names no concrete triggering input, leaving the verifier to
  invent the case rather than exercising the real derivation branch.

#### Suggestions

- 🔵 **Scope**: Single-vs-per-extra sentinel assumption could change the unit
  of work
  **Location**: Assumptions
  The Assumptions note that if a per-extra sentinel is needed, "scope expands
  slightly" — the one place the otherwise crisp boundary is conditional on an
  unverified premise.

- 🔵 **Dependency**: Sentinel value is a shared contract other consumers may
  read
  **Location**: Requirements
  The sentinel becomes a persisted value across the corpus; any consumer that
  reads these extras (the interactive linkage stage 0007 later runs, or future
  migrations) will encounter `pending` rather than a real value, but no
  downstream consumer is named.

- 🔵 **Clarity**: Required extra named three ways interchangeably
  **Location**: Requirements
  The same concept appears as "type-extra," "required extra," and "required
  type-extra" across sections — comprehension is preserved but each variant
  forces a small reconciliation.

### Strengths

- ✅ Structurally complete for its kind: every standard section (Summary,
  Context, Requirements, Acceptance Criteria, Open Questions, Dependencies,
  Assumptions, Technical Notes, Drafting Notes, References) is present and
  substantively populated, with valid frontmatter (kind: task, status: draft,
  priority: high, parent/relates_to links, schema_version).
- ✅ Exemplary scope discipline: Summary, Requirements, and Acceptance Criteria
  all describe the same atomic single-branch fix with no drift, and the kind
  (task) matches the declared S size.
- ✅ Context explains the *why* — the backfill leaves an extra absent while
  `self_validate_structural` treats it as a MISSING-EXTRA violation — rather
  than restating the Summary.
- ✅ Technical Notes pin exact file/line locations and both validator gates
  (MISSING-EXTRA at :345, EMPTY-PLACEHOLDER at :348-359) the sentinel must
  clear, making the cross-script coupling visible and the failing input
  reproducible.
- ✅ Acceptance Criteria are framed as Given/When/Then with concrete
  preconditions and observable outcomes, including an explicit no-regression
  guard for the derivable-default path.
- ✅ Blocked-by/Blocks correctly empty — the research confirms the 0007
  self-contradiction is independent and bites the current tree on its own.

### Recommended Changes

1. **Fix the sentinel literal (or declare it free)** (addresses: "Sentinel
   value is exemplary, not fixed", "Single-vs-per-extra sentinel assumption",
   "Sentinel value is a shared contract")
   In Requirements and Acceptance Criteria, either commit to a concrete token
   (e.g. "the backfill writes `pending` for the absent required extra") or
   state explicitly that any non-empty token is acceptable and tests assert
   only presence + non-emptiness. Resolve the per-extra-sentinel assumption up
   front so the boundary stays fixed, and note whether the interactive linkage
   stage or future migrations consume the sentinel value.

2. **Add an end-to-end abort criterion** (addresses: "No criterion verifies the
   end-to-end abort no longer occurs")
   Add: "Given a corpus containing a required-extra-bearing file with no
   derivable default, when 0007 runs in full, then it completes its
   mechanical/self-validate stages without aborting on MISSING-EXTRA" — so the
   reported whole-run abort symptom is covered, not just the unit-level
   validator behaviour.

3. **Capture the 0114 coupling and resolve its status** (addresses:
   "Concurrent-edit coupling with 0114", "Contradictory statements about whether
   0114 is complete")
   State 0114's status once, unambiguously (e.g. "0114 is complete but did not
   address the no-derivable-default path, so this fix is still required"), and
   add a coordination note in Dependencies that both items edit the same 0007
   backfill branch so the concurrent-edit risk is visible at scheduling time.

4. **Add a concrete input to Criterion 3** (addresses: "Criterion 3 regression
   guard lacks a concrete derivable-default input")
   E.g. "Given a PR file named with a numeric stem (`pr-42-...`), when the
   backfill runs, then `pr_number: 42` is written unchanged."

5. **Disambiguate 'fix C' and unify terminology** (addresses: "Bare letter 'C'
   has an ambiguous referent", "Required extra named three ways")
   On first use, bind "C" to its source (e.g. "research fix option C"), and pick
   one canonical term ("required type-extra") used consistently throughout.

## Per-Lens Results

### Clarity

**Summary**: The work item is largely clear and internally consistent: the
Summary, Context, Requirements, and Acceptance Criteria all describe the same
narrow fix. The main clarity risks are an overloaded bare letter 'C' whose
referent depends on which document the reader trusts, an apparent contradiction
about whether sibling item 0114 is complete or still owns outstanding work, and
a few domain terms used interchangeably. None block comprehension for a domain
reader, but the 'C' referent and the 0114 status tension could each send a
reader to the wrong assumption.

**Strengths**:
- The core problem statement in Context is precise and unambiguous: it names
  both disagreeing parties (the 0007 backfill vs. self_validate_structural), the
  exact tolerated state, and the exact failure (MISSING-EXTRA abort inside
  set -euo pipefail).
- Requirements name their actor explicitly ("have the backfill write...") and
  state the outcome as an observable system state.
- Scope boundaries are drawn clearly and repeated consistently across Summary,
  Requirements, and Dependencies.

**Findings**:
- 🔵 minor (confidence: medium) — **Bare letter 'C' / 'fix C of 0115' has an
  ambiguous referent** — Location: Summary. The item repeatedly identifies
  itself as "fix C of 0115" but never states what the "C" lettering enumerates.
  The research uses "C" as one of five fix *options* (A–E), while parent 0115
  adopts option A and rejects option D — so a reader cannot tell whether "fix C
  of 0115" means the research's option C or a separate sub-fix internal to 0115.
  A reader reconciling this child against 0115 may assume a mismatched mental
  model of the parent. Suggestion: bind the letter to its source on first use.
- 🔵 minor (confidence: medium) — **Contradictory statements about whether 0114
  is complete** — Location: Assumptions. Requirements and Dependencies imply
  0114 owns live outstanding work, while the second Assumption says this fix
  "remains required despite 0114 being considered complete." State 0114's status
  once, unambiguously, and make the Dependencies line consistent with it.
- 🔵 suggestion (confidence: low) — **Required extra named three ways
  interchangeably** — Location: Requirements. The concept appears as
  "type-extra," "required extra," and "required type-extra." Pick one canonical
  term and use it consistently.

### Completeness

**Summary**: This is a tightly-scoped task work item that is structurally
complete and well-populated for its kind. Every standard section is present and
substantively filled: a clear Summary, a Context explaining the contradiction,
specific Requirements, three concrete Acceptance Criteria, populated
Dependencies/Assumptions/Open Questions, and Technical Notes with exact code
locations. Frontmatter is complete and valid (kind: task, status: draft,
priority: high). For a task kind the lens bar is a clear definition of the work
to be done, and that bar is comfortably met.

**Strengths**:
- Summary is a single unambiguous action statement.
- Context explains the *why* — real motivation, not a restatement of the
  Summary.
- Acceptance Criteria contains three specific Given/When/Then criteria,
  including a no-regression criterion.
- Frontmatter is complete and valid.
- Dependencies, Assumptions, and Open Questions are all explicitly populated
  (Open Questions correctly marked "None"), and Technical Notes pin exact
  file/line locations and the two validator gates.

**Findings**: None.

### Dependency

**Summary**: 0118 is a tightly-scoped task whose couplings are mostly
well-captured: the Dependencies section correctly names 0115 (parent) and 0114
(the adjacent backfill-completeness owner it scopes around), and the Technical
Notes pin the exact validator gates. The most material gap is an uncaptured
coupling with sibling 0114: both work items mutate the same 0007 required-extras
backfill, and the relationship is named only as "relates to" even though the
Assumptions flag a live contradiction. A secondary observation is that the
cross-script reconciliation depends on the corpus validator's two gates
remaining stable.

**Strengths**:
- The validator coupling is explicitly captured (MISSING-EXTRA at
  validate-corpus-frontmatter.sh:345, EMPTY-PLACEHOLDER at :348-359).
- The parent (0115) and adjacent scoping owner (0114) are both named, making the
  decomposition relationships traceable.
- Blocked-by/Blocks are correctly empty: the research confirms the 0007
  self-contradiction is independent and bites the current tree on its own.

**Findings**:
- 🟡 major (confidence: high) — **Concurrent-edit coupling with 0114 captured
  only as 'relates to'** — Location: Dependencies. 0118 and 0114 both mutate the
  same artefact (0007:502-512), yet the relationship is recorded only as
  "Relates to: 0114" with "Blocked by: none." The Assumptions section flags a
  live conflict. Without a captured ordering or merge-coordination note, an
  implementer could land 0118's sentinel write on top of (or have it reverted
  by) 0114's changes. Suggestion: add an explicit ordering/coordination note.
- 🔵 minor (confidence: medium) — **Sibling fix-set ordering under 0115 not
  captured** — Location: Dependencies. The research recommends C+D with B as
  mitigation, and 0118 is fix C among siblings (0116 = B, 0117 = A, 0119 = E,
  0120 = prevention). If 0120's prevention/lint cross-check is meant to guard
  the exact reconciliation 0118 performs, an uncaptured ordering could let one
  land without the other's safeguard. Suggestion: note the Blocks relationship
  or confirm independence.
- 🔵 suggestion (confidence: medium) — **Sentinel value is a shared contract
  other consumers may read** — Location: Requirements. The sentinel becomes a
  persisted value across the corpus; downstream consumers (the interactive
  linkage stage, future migrations) will encounter `pending` rather than a real
  value, but none are named. Suggestion: note whether any downstream stage
  consumes these extras.

### Scope

**Summary**: 0118 is a tightly scoped, single-concern task: make migration
0007's no-derivable-default backfill write an accepted sentinel placeholder so
its own structural self-validation stops contradicting the tolerated state. The
requirements, acceptance criteria, and summary all describe the same atomic unit
of work, and the kind (task) fits a low-effort, single-branch reconciliation.
The only mild scope signal is that the work item names two scripts and shares a
boundary with sibling 0114, but it handles both explicitly.

**Strengths**:
- Exemplary scope discipline: Summary, Requirements, and Acceptance Criteria all
  describe the same single concern with no drift.
- Boundaries are stated explicitly and defended (no-derivable-default path vs.
  0114's broader completeness vs. 0115 the parent).
- Kind (task) matches the declared S size and the genuinely atomic nature of the
  change.
- The cross-script touch is acknowledged as a deliberate reconciliation rather
  than two independent deliverables.

**Findings**:
- 🔵 suggestion (confidence: medium) — **Single-vs-per-extra sentinel assumption
  could change the unit of work** — Location: Assumptions. The Assumptions note
  that if a per-extra sentinel is needed, "scope expands slightly" — the one
  place the otherwise crisp boundary is conditional on an unverified premise.
  Suggestion: confirm a single token suffices up front, or pre-commit to keeping
  any per-extra need as a separate follow-up.

### Testability

**Summary**: The work item is a task with three well-formed Acceptance Criteria
expressed as Given/When/Then behaviours, each naming a concrete precondition,
action, and observable outcome that maps onto a definite validator gate. The
criteria are largely verifiable and tie cleanly to the two named validator
checks. The main testability gaps are an unspecified concrete sentinel value
left as "e.g." rather than fixed, and no criterion asserting the prior
hard-abort no longer occurs end-to-end through the migration run.

**Strengths**:
- All three Acceptance Criteria are framed as Given/When/Then with a concrete
  precondition, defined action, and observable pass/fail outcome.
- Criterion 2 ties the expected outcome to the two exact named validator gates,
  giving a precise dual-axis pass condition.
- The third criterion supplies an explicit regression guard.
- Technical Notes pin the exact source locations and validator line numbers, and
  a concrete trigger example is given.

**Findings**:
- 🟡 major (confidence: high) — **Sentinel value is exemplary, not fixed,
  leaving the pass condition under-specified** — Location: Acceptance Criteria.
  The criteria refer to "an accepted sentinel value" / "(e.g. `pending`)"
  without committing to one token, and the Assumptions leave open whether a
  per-extra sentinel is needed. A verifier cannot write a definitive assertion
  without knowing the exact value. Suggestion: fix the literal, or state that
  any non-empty token is acceptable and assert only presence + non-emptiness.
- 🟡 major (confidence: medium) — **No criterion verifies the end-to-end abort
  no longer occurs** — Location: Acceptance Criteria. Criterion 1 asserts
  `self_validate_structural` passes, but no criterion exercises the original
  failure path end-to-end (running 0007 in full and asserting it no longer
  aborts with `FAIL: ... MISSING-EXTRA` before the interactive stage).
  Suggestion: add a full-run criterion.
- 🔵 minor (confidence: medium) — **Criterion 3 regression guard lacks a
  concrete derivable-default input** — Location: Acceptance Criteria.
  Criterion 3 names no concrete triggering input, leaving the verifier to invent
  the case. Suggestion: add a concrete example (e.g. `pr-42-...` → `pr_number:
  42`).

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-06-19

**Verdict:** COMMENT

Re-ran the four lenses that had findings (clarity, dependency, scope,
testability). All three Pass-1 major findings are resolved, and most minors
and suggestions are closed. The sentinel value is now fixed to `unknown` (with
its `0007:219-220` precedent), an end-to-end abort criterion was added, the
0114 coupling and status are captured unambiguously, and per-extra scope and
downstream-consumer concerns are settled. One new major surfaced — the chosen
token `unknown` diverges from the `pending` example in the source research
without an in-item note reconciling the two — but as a single major it falls
below the REVISE threshold (2). The work item is acceptable to proceed; the
remaining items are polish.

### Previously Identified Issues

- 🟡 **Dependency**: Concurrent-edit coupling with 0114 captured only as
  'relates to' — **Resolved** (Dependencies now records the shared
  `0007:502-512` region, 0114's complete-but-untouched status, and the reopen
  reconciliation obligation).
- 🟡 **Testability**: Sentinel value is exemplary, not fixed — **Resolved**
  (fixed to `unknown` throughout; ACs assert the literal).
- 🟡 **Testability**: No criterion verifies the end-to-end abort no longer
  occurs — **Resolved** (new AC3 asserts a full run clears
  `self_validate_structural` without `FAIL: … MISSING-EXTRA`).
- 🔵 **Clarity**: Contradictory statements about whether 0114 is complete —
  **Resolved** (Assumptions now states 0114 is complete but did not address the
  no-derivable-default path).
- 🔵 **Clarity**: Bare letter 'C' has an ambiguous referent — **Resolved**
  (now "research fix option C … as a child of 0115").
- 🔵 **Testability**: Criterion 3 regression guard lacks a concrete input —
  **Resolved** (new AC4 names a numeric-stem / `pr`-segment input).
- 🔵 **Scope**: Single-vs-per-extra sentinel assumption — **Resolved**
  (committed to a single `unknown` token; per-extra explicitly out of scope).
- 🔵 **Dependency**: Sentinel value is a shared contract — **Resolved**
  (downstream-consumers note added to Dependencies).
- 🔵 **Dependency**: Sibling fix-set ordering under 0115 not captured —
  **Partially resolved** (the 0114 coupling is now explicit, but sibling 0120,
  the prevention cross-check that asserts this task's invariant, is still not
  named as a `Blocks` edge).
- 🔵 **Clarity**: Required extra named three ways — **Partially resolved**
  (Requirements/Assumptions are consistent; the Context section still mixes
  "type-extra" / "required extra" / "extra").

### New Issues Introduced

- 🟡 **Clarity** (major): Sentinel token diverges from the referenced research
  (`unknown` here vs. the research's `e.g. pending`) without an in-item note
  reconciling the two. Deliberate and justified (parity with `verdict`/`lenses`
  at `0007:219-220`), but a one-line note that `unknown` supersedes the
  research's illustrative `pending` would close it. Also flagged faintly by the
  dependency lens as a cross-decomposition contract divergence (vs. parent 0115).
- 🔵 **Dependency** (minor): Corpus validator (0105) reconciled against is not
  named as a `Relates to` coupling, though the parent 0115 lists it.
- 🔵 **Testability** (minor): AC3 states only the absence of an abort string,
  with no positive "reached interactive stage / emitted PROMPT" observable.
- 🔵 **Testability** (minor): No single criterion ties the backfill-emitted
  `unknown` to the same value the validator accepts in one run (AC1 and AC2 can
  be satisfied against separate fixtures).
- 🔵 **Clarity** (minor): "both gates" in Assumptions and the `0007:NNN`
  shorthand assume retained/prior knowledge; restating the two gates inline and
  expanding the script name once on first use would help a cold reader.
- 🔵 **Scope** (suggestion): State explicitly that the validator is unchanged
  (the sentinel is chosen precisely so the existing validator accepts it), so
  the "cross-script" framing isn't read as implying a second edit.
- 🔵 **Testability** (suggestion): Pin an expected derived value for at least
  one AC4 example (e.g. `0042-foo.md` ⇒ `pr_number: 0042`).

### Assessment

The work item is ready to proceed to planning. Every Pass-1 major is resolved
and the acceptance criteria are now concrete and falsifiable. The lone new
major (the `unknown`-vs-`pending` divergence from the source research) is a
documentation gap, not a design problem — the chosen token is correct — and is
closed by a single sentence. The remaining minors and suggestions are optional
polish that can be folded in during planning if desired.

### Approval

After Pass 2, the two highest-value closers were applied to the work item: the
`unknown`-supersedes-`pending` note (resolving the lone Pass-2 major) and a
`Blocks: 0120` edge recording the prevention cross-check's dependency on this
task's invariant. With the only major resolved, the reviewer marked this review
**APPROVE** — the work item is cleared for planning. The remaining minors and
suggestions are optional polish, not blockers.
