---
type: work-item-review
id: "0086-kanban-drag-and-drop-review-1"
title: "Work Item Review: Kanban Drag-and-Drop with Toast Confirmations"
date: "2026-06-06T12:04:20+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0086"
work_item_id: "0086"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 3
tags: [design, frontend, kanban, accessibility]
last_updated: "2026-06-06T12:54:32+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Kanban Drag-and-Drop with Toast Confirmations

**Verdict:** REVISE

This is a strong, unusually well-authored story: the scope pivot from
"build drag-and-drop" to a quality pass on already-shipped infrastructure is
stated clearly and consistently, every expected section is densely populated,
dependencies are actively resolved rather than left implicit, and the
acceptance criteria are mostly framed as concrete Given/When/Then behaviours.
The verdict is REVISE solely because three **testability** gaps — all rooted in
the deliberately open-ended "iterate against the prototype design" framing —
mean parts of the work cannot produce a definitive pass/fail as written. These
are tractable wording fixes, not structural problems; the story is close to
ready.

### Cross-Cutting Themes

- **Unbounded "iterate against the prototype design" in Requirement A**
  (flagged by: testability, scope, completeness) — The drag-interaction section
  declares its three defects "known examples, not an exhaustive list" and
  directs the implementer to "fix any further inconsistencies." This is a
  defensible authoring choice (the Drafting Notes justify it), but no criterion
  bounds it, so the section's true size is undefined and its done-state is
  unverifiable. This single decision drives one major finding and two
  suggestions.
- **Prototype as authoritative design source is unconfirmed**
  (flagged by: dependency, testability) — The whole of Requirement A depends on
  `view-kanban.jsx` being the current source of truth, yet whether it has been
  superseded is itself an Open Question, and A1's visual values ("translucent",
  "rotated") have no cited reference values. Resolving the design-source
  authority would close both a dependency risk and a testability gap.
- **Keyboard "verify-and-harden" criteria are underspecified**
  (flagged by: testability, completeness) — C1/C2 are framed as verification
  activities with no defined expected announcement content/keys and no stated
  deliverable when nothing is found to fix, leaving "done" ambiguous for the
  keyboard cluster.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: Requirement A's non-exhaustive defect list has no
  verification procedure
  **Location**: Requirements: A. Drag-interaction fixes
  The three drag issues are "known examples, not an exhaustive list" with an
  open-ended "fix any further inconsistencies" directive that no acceptance
  criterion bounds, so the drag interaction can be argued as both passed and
  failed for any given board state.

- 🟡 **Testability**: "any configured column set" criterion is unbounded
  **Location**: Acceptance Criteria (final bullet)
  "All drag, drop, toast, and keyboard behaviours hold regardless of column
  count or labels" uses unbounded language with no defined configurations to
  test against, so it cannot produce a definitive pass/fail and provides no
  verification value as written.

- 🟡 **Testability**: keyboard announcement content and key sequence are
  unspecified
  **Location**: Acceptance Criteria (keyboard bullet) / Requirements C1–C2
  The criterion confirms announcements "fire" on pick-up/column-change/drop/
  cancel but specifies neither expected announcement content nor activation
  keys, so an announcement that fires but conveys wrong information would still
  pass.

#### Minor

- 🔵 **Clarity**: "the moved card" is contradictory in the failed-revert case
  **Location**: Requirements: C3 / matching Acceptance Criterion
  After a failed write the card reverts and was not moved, so "focus returns to
  the moved card" has no clear referent in that branch.

- 🔵 **Testability**: success-toast technical body line is not captured in any
  criterion
  **Location**: Acceptance Criteria (success-toast bullet)
  Context specifies a technical body line ("PATCH … → 204 · fresh ETag
  received") as part of the design target, but no acceptance criterion verifies
  it, so an implementation that drops it would still pass.

- 🔵 **Testability**: A1's "translucent" / "rotated" states have no measurable
  values or cited source
  **Location**: Acceptance Criteria (translucent-clone bullet) / Requirement A1
  No opacity/rotation thresholds or explicit "matches view-kanban.jsx" assertion
  target are given, so reviewers could disagree on whether a state satisfies the
  criterion.

- 🔵 **Completeness**: Requirement A's residual fix set is not enumerated
  **Location**: Requirements: A. Drag-interaction fixes
  Work beyond A1–A3 is deferred to "iterate against the prototype design," so the
  full extent cannot be scoped from the work item alone without studying
  `view-kanban.jsx`.

- 🔵 **Completeness**: C1/C2 verification deliverable is unclear
  **Location**: Requirements: C. Keyboard accessibility
  C1/C2 are framed as "verify…" activities; it is unclear what constitutes
  "done" when verification finds nothing to fix beyond C3's hardening.

- 🔵 **Dependency**: 0040 "done-but-status-unmarked" is an unrecorded soft
  ordering coupling
  **Location**: Dependencies
  The dismissal of the WorkItemCard.tsx merge-conflict risk rests on an
  out-of-band assumption that 0040's code has landed, with no tracked dependency
  edge to surface it at planning time.

- 🔵 **Dependency**: Toaster-variant extension (B3) is a shared-artefact change
  with uncaptured consumers
  **Location**: Requirements: B3
  B3 modifies the globally-mounted Toaster (0039); the Blocks field is "none",
  so any other current/future toast consumers affected by the variant API change
  are invisible in the record.

- 🔵 **Dependency**: design-source authority for the prototype is unresolved and
  gates the design-match work
  **Location**: Open Questions
  Requirement A depends on `view-kanban.jsx` being the current source of truth,
  but whether a newer reference supersedes it is unresolved; design-match work
  could otherwise run against a stale target.

#### Suggestions

- 🔵 **Clarity**: actor named two ways — "someone managing work items" vs "a
  user"
  **Location**: Summary / Acceptance Criteria
  The same single human actor is labelled two ways; the equivalence is never
  stated explicitly.

- 🔵 **Clarity**: a handful of acronyms used without first-use expansion
  **Location**: Requirements: C2 / Context
  "a11y" and "SSE" appear without inline expansion; surrounding prose usually
  disambiguates, so comprehension is not blocked.

- 🔵 **Scope**: the toast-confirmation loop (B) is a separable sub-thread
  **Location**: Requirements: B. Toast-confirmation loop
  B1–B3 plus the net-new Toaster variants could ship independently of the A
  drag fixes; bundling is coherent but optional if delivery cadence matters.

- 🔵 **Scope**: Requirement A's open-ended fix clause lacks a size guardrail
  **Location**: Requirements: A. Drag-interaction fixes
  A large undiscovered drift could expand the story beyond one increment;
  consider a guardrail to split non-localised fixes into follow-on items.

### Strengths

- ✅ The scope pivot (from "build drag-and-drop" to a quality pass on an
  already-shipped feature) is stated explicitly and consistently across Summary,
  Context, and Drafting Notes, removing the most likely source of misreading.
- ✅ Context is exceptionally rich: it corrects the original two-app assumption,
  enumerates exactly what already exists and is "not to be rebuilt", and
  describes the prototype interaction target in concrete behavioural terms.
- ✅ Dependencies are actively resolved with reasoning (0039 done, 0044
  abandoned, 0040 related-but-complete) rather than left implicit, and the
  0040 coupling names its specific shared surface (WorkItemCard.tsx).
- ✅ The ADR-0024 configurable-columns constraint is captured consistently
  across Context, Out of scope, Acceptance Criteria, and Assumptions.
- ✅ Acceptance Criteria are mostly precise Given/When/Then behaviours with
  observable outcomes, including the bidirectional click-vs-drag case (drag must
  not navigate AND a genuine click must navigate) and a concrete focus
  post-condition (AC8).
- ✅ Story kind and sizing fit: a bounded quality pass against existing
  infrastructure, confined to a single application and team surface, deliverable
  as a standalone increment.

### Recommended Changes

1. **Bound the Requirement A verification** (addresses: A's non-exhaustive list
   has no verification procedure; A's residual fix set not enumerated; A's
   open-ended clause lacks a guardrail)
   Keep the iterate-against-design intent, but make done-ness checkable: either
   constrain verification to A1–A3 as the definition of done, or add a concrete
   checkpoint such as "a side-by-side comparison against `view-kanban.jsx` is
   recorded and each deviation is either fixed or logged as a follow-up item."
   Add a guardrail noting that any discovered inconsistency requiring more than a
   localised fix is split into a follow-on work item.

2. **Replace the unbounded column-set criterion with representative cases**
   (addresses: "any configured column set" criterion is unbounded)
   Name the configurations the test must cover, e.g. "behaviours verified against
   (a) the prototype's 3-column set and (b) a 5-column set with long labels,"
   so the criterion produces a definitive pass/fail.

3. **Specify expected keyboard announcement content and keys** (addresses:
   keyboard announcement content/keys unspecified; C1/C2 verification deliverable
   unclear)
   State the expected announcement text/pattern for each event (e.g. "on drop:
   'Moved {item} to {column}'") and the activation keys, and note that C1/C2
   produce a verification record (e.g. a passing keyboard/a11y test) as their
   deliverable even when no defect is found.

4. **Resolve the design-source authority and cite A1's visual values**
   (addresses: design-source authority unresolved; A1 has no measurable values)
   Confirm `view-kanban.jsx` is the current reference (or identify the
   superseding one) and record it in References; cite the specific opacity/
   rotation values from the prototype, or state that A1 is verified by visual
   parity with the named reference.

5. **Capture the soft couplings in Dependencies** (addresses: 0040
   done-but-unmarked coupling; Toaster shared consumers)
   Add a "sequenced after 0040's WorkItemCard changes are confirmed merged"
   precondition, and note whether any other surfaces consume the Toaster today
   (or confirm the variant addition is purely additive and non-breaking).

6. **Tidy minor clarity items** (addresses: "the moved card" contradiction;
   actor naming; acronyms; success-toast body line)
   Reword C3's focus target so it holds in both branches (e.g. "the card that
   was being dragged"); pick one term for the human actor; expand "a11y" and
   "SSE" on first use; and decide whether the success-toast technical body line
   is required (add it to the criterion) or optional (mark it so).

## Per-Lens Results

### Clarity

**Summary**: The work item is unusually clear and internally consistent: the
scope-pivot from greenfield build to quality pass is stated explicitly in both
Summary and Drafting Notes, referents like "the card" and "the prototype"
resolve unambiguously, and requirements name their actors and observable
outcomes. The few clarity concerns are minor: a couple of acronyms used without
first-use expansion, a subtle actor-shift between "someone managing work items"
and "a user", and one phrase ("the moved card") that is slightly contradictory
in the failed-revert case.

**Strengths**:
- The scope pivot is stated explicitly and consistently in the Summary, Context,
  and Drafting Notes, removing the most likely source of misreading.
- Requirements A1–A3, B1–B3, and C1–C3 each name the actor or trigger and an
  observable outcome, so responsibility and the done-state are concrete.
- Context explicitly enumerates what already exists and is "not to be rebuilt",
  and Out of scope reinforces the boundary.
- The prototype-vs-live-app distinction is defined up front in Context,
  pre-empting an ambiguity the source design-gap document left open.

**Findings**:
- 🔵 minor (confidence: medium) — **"the moved card" is contradictory in the
  failed-revert case** — _Requirements: C3_. C3 and its matching criterion state
  focus returns to "the moved card" after "a successful move or a failed-write
  revert", but in the failed-revert case the card was not moved, so the referent
  is unclear. An implementer could place focus on the wrong element. Suggestion:
  use a referent that holds in both branches, e.g. "the card that was being
  dragged".
- 🔵 suggestion (confidence: medium) — **Actor named two ways: "someone managing
  work items" vs "a user"** — _Summary / Acceptance Criteria_. The same single
  actor is labelled two ways and the equivalence is never stated. Pick one term
  and use it consistently.
- 🔵 suggestion (confidence: low) — **A handful of acronyms used without
  first-use expansion** — _Requirements: C2 / Context_. "a11y" and "SSE" appear
  without inline expansion; adjacent prose usually disambiguates. Expand on first
  use.

### Completeness

**Summary**: This is a substantively complete, well-populated story. Every
expected section is present and densely populated: a clear user-voice Summary, a
rich Context explaining the as-shipped state and the prototype reference,
specific Requirements grouped into three labelled clusters with an explicit
out-of-scope list, nine acceptance criteria, plus populated Open Questions,
Dependencies, Assumptions, Technical Notes, Drafting Notes, and References.
Frontmatter is well-formed for a story. The only minor gaps are judgement calls
about whether two requirement clusters carry enough self-contained content to
act on without referring to external source documents.

**Strengths**:
- Summary is a clear user-voice statement identifying who, what, and why, then
  frames the work as a quality pass on an already-shipped feature.
- Context is exceptionally rich: corrects the two-app assumption, enumerates what
  already exists and is not to be rebuilt, and describes the prototype target
  concretely.
- Acceptance Criteria contains nine Given/When/Then bullets mapping cleanly onto
  the lettered requirement clusters.
- Frontmatter is complete and well-formed for a story.
- Optional sections (Open Questions, Dependencies, Assumptions, Technical Notes,
  Drafting Notes, References) all carry substantive content rather than
  placeholders.

**Findings**:
- 🔵 minor (confidence: medium) — **Requirement A's residual fix set is not
  enumerated** — _Requirements: A. Drag-interaction fixes_. Cluster A is
  explicitly non-exhaustive and defers the full set to "iterate against the
  prototype design", so the full extent cannot be scoped from the work item
  alone. Deliberate choice; if more determinacy is wanted, capture a short
  checklist of prototype behaviours to verify against.
- 🔵 minor (confidence: low) — **C1/C2 verification deliverable is unclear** —
  _Requirements: C. Keyboard accessibility_. C1/C2 are framed as "verify…"
  activities; unclear what "done" means when verification finds nothing to fix.
  Note explicitly that they produce a verification record (e.g. a passing test).

### Dependency

**Summary**: Dependency capture in this work item is unusually thorough and
well-reasoned: the Dependencies section explicitly resolves each candidate
coupling (0039 Toaster done, 0044 spike abandoned, 0040 related-but-complete)
rather than leaving them implicit, and the prototype design reference and
ADR-0024 column-set constraint are named in Context, Assumptions, and
References. The only residual concerns are soft couplings the body acknowledges
but does not formalise: the 0040 "done-but-status-not-updated" state, the
dependence on the prototype as authoritative design source (itself an Open
Question), and the net-new Toaster-variant work other consumers could be
affected by.

**Strengths**:
- Dependencies enumerates and dismisses each candidate blocker with reasoning
  (0039 done, 0044 abandoned) — no hidden upstream blockers.
- The 0040 relation is named with its specific coupling surface
  (routes/kanban/WorkItemCard.tsx) and the merge-conflict risk is addressed.
- The ADR-0024 configurable-columns constraint is captured consistently across
  Context, Out of scope, Acceptance Criteria, and Assumptions.
- The prototype design reference is named with an exact path in Context and
  References, making the design-source coupling traceable.

**Findings**:
- 🔵 minor (confidence: high) — **0040 "done-but-status-unmarked" is an
  unrecorded soft ordering coupling** — _Dependencies_. The dismissal rests on an
  unverified out-of-band assumption that 0040's code has landed; if not, the
  WorkItemCard merge-conflict risk re-materialises with no recorded edge.
  Suggestion: record a "sequenced after 0040 lands" note or a precondition.
- 🔵 minor (confidence: medium) — **Toaster-variant extension (B3) is a
  shared-artefact change with uncaptured consumers** — _Requirements: B3_. B3
  modifies the globally-mounted Toaster; the Blocks field is "none", so affected
  downstream consumers are invisible. Note other consumers or confirm the change
  is purely additive.
- 🔵 minor (confidence: medium) — **Design-source authority for the prototype is
  unresolved and gates the design-match work** — _Open Questions_. Requirement A
  depends on `view-kanban.jsx` being current; if superseded, the work runs
  against a stale target. Resolve before the design-match work begins and record
  the confirmed source.

### Scope

**Summary**: Work item 0086 is a well-scoped, coherent quality-pass story: every
requirement (drag-interaction fixes, toast-confirmation loop, keyboard a11y)
serves the single unified purpose of making the existing kanban drag-and-drop
reliable and on-design. The scope is bounded by an explicit Out-of-scope
section, lives within a single application and team surface, and is correctly
sized as a story rather than an epic. The only scope-adjacent considerations are
the toast loop being a logically linked sub-thread and the deliberately
non-exhaustive "iterate against the design" clause, neither of which rises to a
delivery-risk concern.

**Strengths**:
- All three requirement groups converge on one deliverable outcome: a reliable,
  on-design board interaction with confirmation feedback.
- Explicit Out-of-scope section names what is excluded (column-set config,
  intra-column reordering, non-status frontmatter mutation).
- Confined to a single application and surface with no cross-service or
  cross-team orchestration required.
- Story kind fits the scope: a bounded quality pass, not an epic, not a chore.
- Dependencies are resolved, so the story can be delivered as a standalone
  increment.

**Findings**:
- 🔵 suggestion (confidence: medium) — **The toast-confirmation loop (B) is a
  separable sub-thread** — _Requirements: B. Toast-confirmation loop_. B1–B3 plus
  the net-new Toaster variants could ship independently of the A drag fixes;
  bundling is coherent but the B work could be split into a follow-on story if
  delivery cadence matters.
- 🔵 suggestion (confidence: medium) — **Requirement A's open-ended fix clause
  means the size is not fully bounded by its own text** — _Requirements: A.
  Drag-interaction fixes_. "Fix any further inconsistencies" is reasonable, but a
  large undiscovered drift could expand the story. Add a guardrail to split
  non-localised fixes into follow-on items.

### Testability

**Summary**: Most Acceptance Criteria are well-framed as Given/When/Then pairs
with concrete, observable outcomes (translucent clone follows cursor, success
toast names card and column, focus returns to moved card), which is strong for a
story. However, two structural testability gaps undermine verification:
Requirement A explicitly declares its defect list non-exhaustive with an
open-ended "fix any further inconsistencies" directive that no criterion can
bound, and the keyboard "verify-and-harden" criteria plus the "any configured
column set" criterion contain unbounded language that cannot produce a definitive
pass/fail.

**Strengths**:
- Acceptance Criteria are consistently expressed as Given/When/Then observable
  behaviours with specific outcomes (translucent clone, rotated source card,
  success toast naming card and column, error toast plus revert, no inline
  banner).
- The click-vs-drag distinction (AC2) is specified bidirectionally.
- The toast contract is concrete enough to verify: success on 204 naming card
  and column, error on failure/412 with revert and banner removal.
- AC8 (focus returns to the moved card after move or revert) is a precise,
  observable post-condition.

**Findings**:
- 🟡 major (confidence: high) — **Requirement A's non-exhaustive list has no
  verification procedure** — _Requirements: A. Drag-interaction fixes_. The three
  issues are "known examples, not an exhaustive list" with an open-ended fix
  directive that no criterion bounds, so the requirement can be argued as both
  passed and failed for any board state. Suggestion: constrain verification to
  A1–A3, or add a concrete side-by-side checkpoint against `view-kanban.jsx`.
- 🟡 major (confidence: high) — **"any configured column set" criterion is
  unbounded** — _Acceptance Criteria (final bullet)_. "All behaviours hold
  regardless of column count or labels" cannot produce a definitive pass/fail.
  Suggestion: replace with specific representative cases (e.g. a 3-column and a
  5-column-with-long-labels set).
- 🟡 major (confidence: medium) — **Keyboard announcement content and key
  sequence unspecified** — _Acceptance Criteria (keyboard bullet) / Requirements
  C1–C2_. The criterion confirms announcements "fire" but specifies neither
  expected content nor activation keys, so a wrong-but-firing announcement would
  pass. Suggestion: specify the expected announcement text/pattern and the keys.
- 🔵 minor (confidence: medium) — **Success-toast technical body line not
  captured in any criterion** — _Acceptance Criteria (success-toast bullet)_.
  Context specifies a technical body line ("PATCH … → 204 · fresh ETag
  received") but no criterion verifies it. Add it to the criterion if required,
  or mark it optional.
- 🔵 minor (confidence: medium) — **A1's "translucent" / "rotated" states have no
  measurable values or cited source** — _Acceptance Criteria (translucent-clone
  bullet) / Requirement A1_. No opacity/rotation thresholds or explicit "matches
  view-kanban.jsx" target. Cite the prototype's values or assert visual parity
  with the named reference.

---

## Re-Review (Pass 2) — 2026-06-06

**Verdict:** REVISE

The edits from pass 1 landed well: the large majority of first-pass findings are
resolved, and two of the three original majors are fully closed. The verdict
remains REVISE because the re-run surfaced four major findings — two are
*escalations* of first-pass items (the design-parity and convergence-loop
bounding tensions are now seen as needing a concrete oracle, not just a recorded
pass), and two are *latent consistency issues* newly surfaced (the column-model
wording and the dnd-kit-vs-open-library-decision tension). None are structural;
all are tractable wording fixes. Notably, two of the four majors stem from the
inherent tension in design-convergence work (visual parity and an open-ended
loop) that you deliberately chose to retain — so a COMMENT verdict is defensible
if you accept those as conscious trade-offs.

### Previously Identified Issues

- 🟡 **Testability**: Requirement A non-exhaustive list has no verification
  procedure — **Partially resolved**. The recorded exit condition bounds the
  loop, but testability now flags the exit clause as self-asserting and
  recommends an enumerable aspect checklist (see new issues).
- 🟡 **Testability**: "any configured column set" criterion unbounded —
  **Resolved**. Replaced with two representative cases. (Surfaced a new
  column-model clarity issue and a "long label" threshold gap — see new issues.)
- 🟡 **Testability**: keyboard announcement content/keys unspecified —
  **Resolved**. Now names activation keys and asserts against `announcements.ts`
  strings; flagged as a strength this pass.
- 🔵 **Clarity**: "the moved card" contradictory on revert — **Resolved**. Now
  "the card that was being dragged… target on success, source on revert".
- 🔵 **Testability**: success-toast technical body line uncaptured —
  **Resolved**. Added to B1 and the success criterion. (New minor: exact
  body-line form left ambiguous — see new issues.)
- 🔵 **Testability**: A1 "translucent"/"rotated" not measurable — **Partially
  resolved / escalated**. Now names the prototype as source of truth, but the
  artefact still carries no measurable threshold, so testability re-raised this
  at major severity.
- 🔵 **Completeness**: Requirement A residual fix-set not enumerated —
  **Resolved**. Bounded by the recorded convergence loop.
- 🔵 **Completeness**: C1/C2 verification deliverable unclear — **Resolved**.
  Both now state a recorded verification as the deliverable.
- 🔵 **Dependency**: 0040 done-but-unmarked coupling — **Partially resolved**. A
  soft-ordering precondition was added; still flagged minor because it rests on
  an unverified work-item status.
- 🔵 **Dependency**: Toaster-variant shared consumers — **Partially resolved**. An
  additive/backward-compatible note was added; still flagged minor because the
  actual `use-toast.ts` call sites are not enumerated.
- 🔵 **Dependency**: design-source authority unresolved — **Resolved**. Confirmed
  `view-kanban.jsx` authoritative in Open Questions, Context, and References.
- 🔵 **Clarity**: actor named two ways — **Resolved**. Unified to "a user".
- 🔵 **Clarity**: acronyms unexpanded — **Resolved**. "a11y" expanded on first use.
- 🔵 **Scope**: Requirement A open-ended clause lacks a guardrail — **Resolved**.
  Split-to-follow-on guardrail baked into the exit condition.
- 🔵 **Scope**: toast loop (B) separable — **Still present** (suggestion-level,
  unchanged; an accepted cohesion choice).

### New Issues Introduced

- 🟡 **Clarity** (major): Authoritative column model is internally inconsistent —
  the representative-case wording anchors to "the prototype's three-column set
  (Todo / In progress / Done)" while Assumptions treat the live board's
  configurable set as authoritative; the live board's actual lanes
  (Draft / Ready / In progress / Done, per the source gap doc) are never
  reconciled. Recommend expressing both fixtures in terms of the live board's
  configured columns and stating that the prototype's three columns are only a
  test fixture, not a target column set.
- 🟡 **Clarity** (major): C1/C2 presuppose dnd-kit while the library decision is
  still open — if the open dnd-kit-vs-HTML5 decision flips, "the existing dnd-kit
  keyboard sensor" and `announcements.ts` become contradictory. Recommend noting
  C1/C2 assume the dnd-kit-retention outcome (which the decision rule favours), or
  phrasing them mechanism-agnostically.
- 🟡 **Testability** (major): A1 visual parity has no concrete oracle — recommend
  either stating target opacity/rotation values or defining verification as a
  named visual-regression snapshot diff against the prototype. (This project
  already maintains visual-regression baselines, so a snapshot oracle is a
  natural fit.)
- 🟡 **Testability** (major): convergence-loop exit condition is self-asserting —
  "no remaining discrepancy" can be claimed regardless of comparison depth.
  Recommend bounding the comparison to an enumerable aspect checklist (drag
  affordance, click-vs-drag, defer-to-drop, drop animation, empty-column copy,
  cursor state, …) each carrying a fixed/follow-up/parity verdict.
- 🔵 **Testability** (minor): "long, wrapping label" has no length/wrap threshold;
  pin a concrete fixture (e.g. ≥30 chars wrapping to ≥2 lines).
- 🔵 **Testability** (minor): success body-line form ambiguous (verbatim
  prototype string vs status+ETag in any format) — state which is required.
- 🔵 **Testability** (minor): "reliable / without bugs" in the Summary is not
  covered by any criterion for repeated/concurrent drags (drop during in-flight
  write-back, etc.) — add a concurrency criterion or note it out of scope.
- 🔵 **Clarity** (minor): the 0040 state is hedged three ways ("confirmed
  merged" / "complete in the codebase" / "out-of-band assumption … may prove
  false") — pick one characterisation and one verification step.
- 🔵 **Clarity** (minor): the convergence loop's "localised enough" / "most
  significant" split boundary is undefined — add a one-line heuristic.
- 🔵 **Clarity** (minor): success body-line verbatim-vs-illustrative unclear
  (same root as the testability minor above).
- 🔵 **Dependency** (minor): Toaster-variant consumers not enumerated (carried
  from partial resolution above).
- 🔵 **Dependency** (suggestion): convergence-loop follow-up work items are an
  implied forward coupling — note that any follow-ons created on closure should
  reference 0086 as their origin.
- 🔵 **Scope** (minor/suggestion): section A in-scope size is discovered during
  execution (consider an expected order-of-magnitude planning signal); B3 is a
  shared-primitive increment riding inside a feature story (confirm the variant
  API is designed as a general capability); story-vs-chore framing for section C
  is a team-norms judgement call.

### Assessment

The work item is materially stronger than at pass 1 — every first-pass finding is
either resolved or improved, and the keyboard-accessibility cluster in particular
is now exemplary. The remaining majors cluster into two themes: (1) **concrete
oracles for design-convergence work** (A1 parity, the convergence-loop checklist)
— inherent to the iterate-against-the-design approach you deliberately retained,
best closed with a visual-regression snapshot and an enumerable aspect checklist;
and (2) **two latent consistency points** (column model, the open library
decision presupposed by C1/C2) that predate this story and are quick wording
fixes. If you treat the design-convergence oracle questions as conscious
trade-offs, the item is acceptable as-is (COMMENT); if you want a fully
self-verifying artefact before planning, a short third pass on the four majors
would close it out.

---

## Re-Review (Pass 3) — 2026-06-06

**Verdict:** COMMENT

The pass-2 edits closed the four majors: testability now reports **zero majors**
and explicitly lists the visual-regression oracle, the bounded convergence
checklist, the mechanism-agnostic C1/C2 framing, and the two pinned column
configurations as *strengths*. Only one major remains (clarity), which sits below
the 2-major REVISE threshold — so the verdict moves to COMMENT. The item is
acceptable for planning as-is; the residual findings are wording-fidelity nuances
with diminishing returns.

### Previously Identified Issues (pass-2 majors)

- 🟡 **Clarity**: column model internally inconsistent — **Resolved**. Both
  fixtures now framed as live-board configs from `GET /api/kanban/config`; the
  prototype's columns demoted to an interaction reference, live columns
  authoritative.
- 🟡 **Clarity**: C1/C2 presuppose dnd-kit — **Partially resolved**. A
  mechanism-agnostic lead-in was added (testability now counts it a strength), but
  clarity still flags that the *named artefacts* (`announcements.ts`, activation
  keys) remain provisional until the open Library decision resolves (re-raised,
  see below).
- 🟡 **Testability**: A1 visual parity had no oracle — **Resolved**. Now a
  visual-regression snapshot against the approved baseline. (Residual minor: the
  baseline-vs-prototype derivation is itself not independently checkable.)
- 🟡 **Testability**: convergence-loop exit self-asserting — **Resolved**. Now a
  7-aspect checklist with explicit parity/fixed/follow-up verdicts. (Residual
  minor: aspects 4–7 lack their own oracle, so verdicts are still partly
  self-attested.)
- 🔵 Pass-2 minors (long-label threshold, "localised" heuristic, follow-up
  traceability) — **Resolved** via the checklist and the pinned ≥30-char label.

### New / Residual Issues

- 🟡 **Clarity** (major): Section C is conditionally specified against the
  unresolved Library decision — until it resolves, a reader has two readings of
  which keyboard keys / announcement source / assertion strings apply, and the AC
  asserts against `announcements.ts`, which may not exist under the HTML5 outcome.
  Suggestion: sequence the Library decision as a hard precondition to section C,
  or make the mechanism-agnostic behaviour primary with the dnd-kit artefacts as
  one named instantiation. (Below the REVISE threshold; a quick wording fix.)
- 🔵 **Clarity** (minor): "parity" is overloaded — the verdict value "parity"
  (matched without changes) vs A1's "parity passes when the snapshot matches the
  baseline" (matched after a fix). Distinguish the two so A1's verdict isn't
  mislabelled.
- 🔵 **Clarity** (minor): actor drifts between "a user" and passive,
  actor-less Given clauses ("a card is being dragged", "when invoked"). Use a
  consistent actor and name what invokes the Toaster.
- 🔵 **Testability** (minor): convergence verdict has no defined arbiter for
  aspects 4–7 (no per-aspect oracle); require each verdict to cite evidence (a
  screenshot pair, or a follow-up work-item ID).
- 🔵 **Testability** (minor): baseline-vs-prototype fidelity not checkable —
  record the concrete opacity/rotation values from `view-kanban.jsx` so the
  baseline's faithfulness is itself verifiable.
- 🔵 **Testability** (minor): success-toast body line has no asserted exact form
  — pin the literal template (`PATCH … → 204 · fresh ETag received`) or the
  required tokens (`204` + `ETag`).
- 🔵 **Testability** (minor): wrapped-label config is environment-dependent —
  pin the test viewport/board width so the wrap precondition is deterministic.
- 🔵 **Clarity** (suggestion): 0040 status is hedged three ways across
  Precondition / Related / Drafting Notes — state it once and cross-reference.
- 🔵 **Clarity** (suggestion): optional `ETag`/`If-Match` gloss on first use.

### Assessment

The work item has converged. Across three passes the original three majors and
their pass-2 successors are all resolved, and the verdict has moved REVISE →
REVISE → COMMENT. What remains is a single clarity major (the Library-decision
provisionality in section C, a quick wording fix or a sequencing note) and a set
of minor wording-fidelity refinements with clearly diminishing returns. The story
is ready for planning; the section-C provisionality is the one item worth a
final touch, ideally by gating the dnd-kit-vs-HTML5 decision ahead of the
keyboard work so the named artefacts are unambiguous when section C is built.

### Post-Pass-3 Edits (no re-run)

Two targeted edits were applied to the work item after pass 3 to clear the
residual clarity major and its closest minor (no further lens run; verdict stays
COMMENT, as only minors would remain):

- **Section C clarity major — addressed.** Section C now states the Library
  decision is a **hard precondition** to the section, makes the mechanism-agnostic
  behaviour primary, and frames the dnd-kit artefacts (keyboard sensor +
  `announcements.ts`) as the concrete instantiation under the favoured
  dnd-kit-retention outcome. An implementer reaching section C now has a fixed
  mechanism and a single reading.
- **"parity" overload minor — addressed.** The convergence-loop verdict values
  are now glossed inline (parity = matches unchanged; fixed = matches after a
  change; follow-up = logged separately), and A1's snapshot test no longer reuses
  the word "parity" for its post-fix match.

Remaining minors (convergence aspects 4–7 oracle, baseline-vs-prototype value
record, success body-line exact form, wrapped-label viewport pinning, actor
consistency, 0040 single-statement, ETag gloss) are left as optional polish — all
below the REVISE threshold and with diminishing returns.

### Final Decision — APPROVED (2026-06-06)

Verdict overridden to **APPROVE** by the reviewer. Across review 1 (three passes
plus a post-pass-3 touch-up) every major finding was resolved and the lens-suggested
verdict had settled at COMMENT with only optional minor polish remaining. The work
item is accepted as ready for planning; the residual minors are recorded above as
optional improvements and do not block. The work item status was moved
`draft` → `ready` alongside this approval.

---
*Review generated by /accelerator:review-work-item*
