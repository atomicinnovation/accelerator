---
type: work-item-review
id: "0196-accelerator-design-inventory-gap-tooling-cli-review-1"
title: "Work Item Review: accelerator-design: Design Inventory and Gap Tooling CLI"
date: "2026-08-06T00:35:37+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0196"
work_item_id: "0196"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-08-06T00:46:19+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: accelerator-design: Design Inventory and Gap Tooling CLI

**Verdict:** REVISE

This is a well-formed split-out story with a strong Dependencies section,
concrete Acceptance Criteria, and disciplined naming — but it undermines
its own stated purpose. The item's Context explicitly says it exists to fix
a clarity finding from its predecessor (0173): the Playwright-executor's
fate stated inconsistently as a hedged either/or in one place and an
unresolved Open Question in another. Three lenses (clarity, completeness,
scope) independently found the same inconsistency has recurred within 0196
itself — the Summary states the decision as settled while Requirements,
Open Questions, and Acceptance Criteria all still hedge it. Scope also
flags that the two possible outcomes are not equivalent-effort, so the
story's size is not actually bounded until that decision is made.

### Cross-Cutting Themes

- **Playwright-executor decision stated as settled in Summary but open
  everywhere else** (flagged by: clarity, completeness, scope) — the
  Summary asserts run.sh definitely stays a thin wrapper
  ("keeping the Playwright executor (`run.sh`) available per the ADR-0048
  thin-wrapper exception"), while Requirements ("either stays a thin
  wrapper ... or is folded into the binary ... see Open Questions for the
  decision"), Open Questions, and Acceptance Criteria bullets 2 and 4 (which
  hedge with "or its folded equivalent" / "if the thin-wrapper exception is
  exercised") all treat it as unresolved. This is the exact class of
  inconsistency the item's own Context says it was carved out of 0173 to
  fix, and the Drafting Notes' claim that the choice "appears once, in Open
  Questions" is itself inaccurate since Requirements restates it.

### Findings

#### Major

- 🟡 **Clarity / Completeness / Scope**: Summary states the
  Playwright-executor decision as settled, contradicting Requirements,
  Open Questions, and Acceptance Criteria
  **Location**: Summary
  The Summary reads as a settled fact ("keeping the Playwright executor
  (`run.sh`) available"), but every other section that touches the same
  question hedges it as an open either/or to be decided during
  implementation. A reader relying on the Summary alone would conclude the
  opposite of what an implementer reading Requirements or Open Questions
  would conclude.

- 🟡 **Scope**: Playwright-executor either/or leaves the story's size
  unbounded at authoring time
  **Location**: Requirements / Open Questions
  Thin-wrapper-vs-fold-in are not equivalent-effort alternatives — keeping
  `run.sh` as a thin wrapper is near-zero additional work, while folding
  Playwright-driving logic into the Rust binary is a substantial rewrite.
  The parent epic's own resolved Open Questions (0136) describe a "thin
  residual shell surface (launcher bootstrap, hook wrapper, Playwright
  executor)" as the expected end state, suggesting the thin-wrapper outcome
  was already the epic-level expectation, yet 0196 reopens it as a live
  implementation-time choice with no default and no decision gate before
  work starts.

#### Minor

- 🔵 **Testability**: "Same report artefact" has no defined equivalence
  criterion
  **Location**: Acceptance Criteria
  AC2 requires the Playwright-driven subcommand to produce "the same report
  artefact the current shell invocation produces" without defining what
  counts as "the same" (byte-identical, schema-equivalent, or something
  looser), leaving no defined procedure for a verifier to conclusively pass
  or fail this comparison.

- 🔵 **Testability**: "Per subcommand" coverage floor is uncountable without
  an enumerated subcommand list
  **Location**: Acceptance Criteria
  AC1's minimum coverage floor (success path + one failure path per
  subcommand) is a good concrete target in principle, but the work item
  never enumerates the design binary's subcommands, so the denominator
  needed to verify the floor is undefined from this document alone.

- 🔵 **Clarity**: "The design binary's Playwright-driven subcommand" does not
  name which subcommand it refers to
  **Location**: Acceptance Criteria
  AC2 refers to "the Playwright-driven subcommand" without stating whether
  this is `inventory-design`, `analyse-design-gaps`, or both, forcing a
  verifier to infer it rather than being told directly.

- 🔵 **Completeness**: No explicit statement of who/what the migration
  serves
  **Location**: Requirements
  As a Story, the item is expected to name the stakeholder whose need is
  being met. Requirements and Context describe the migration mechanics but
  never state the beneficiary beyond what's implicit in the parent epic.

- 🔵 **Dependency**: Foundational launcher/dispatch infrastructure not named
  among resolved prior blockers
  **Location**: Dependencies
  The item is otherwise scrupulous about naming resolved blockers
  explicitly (0166, 0167, 0187) but omits the earlier foundational items
  (0163 scaffold, 0164 launcher/dispatch) those depend on. Low impact —
  these are very likely already covered transitively by 0167.

- 🔵 **Dependency**: No coordination note for siblings sharing the same
  registration pattern
  **Location**: Dependencies
  If the sub-binary registration checklist (AC5) touches shared state (a
  central dispatch manifest or CI floor config), siblings 0195 and 0197
  registering around the same time could produce merge contention not
  visible from any single item's Dependencies section. Speculative —
  only relevant if the checklist isn't purely additive per-binary.

#### Suggestions

- 🔵 **Clarity**: "Repointed suites" used without a gloss
  **Location**: Acceptance Criteria
  AC1's "repointed suites" is inferable (existing tests redirected to the
  new binary) but never stated explicitly.

- 🔵 **Completeness**: No Assumptions section despite implicit assumptions
  underpinning the Acceptance Criteria
  **Location**: Dependencies
  Predecessor item 0173 carried an explicit Assumptions section; 0196 has
  none, though its AC rest on premises (e.g. that repointed suites plus
  characterization tests establish sufficient behavioural parity) that
  could be made explicit.

### Strengths

- ✅ Cleanly carries forward only the design-domain slice of the abandoned
  0173, matching the granularity of siblings 0195 and 0197 — a concrete
  example of right-sizing a previously bundled story.
- ✅ Acceptance Criteria are concrete and checkable: an explicit minimum
  test-coverage floor, a reference to a specific external registration
  checklist (AC5), and a lockstep floor-decrement coupling to 0174 (AC4)
  that avoids a dangling "suites are updated" claim.
- ✅ Dependencies is unusually thorough — it names all three prior blockers
  as explicitly resolved rather than leaving them silently absent, states
  the specific coupling mechanism to the downstream blocked item (0174),
  and correctly distinguishes the pre-existing external Node/Playwright
  coupling from anything newly introduced.
- ✅ Consistent naming discipline throughout: `accelerator-design` (binary
  name) versus `accelerator design ...` (subcommand invocation) is never
  conflated.
- ✅ Context clearly states the motivating 0173 review-1 finding, giving
  the reader provenance for why the item is structured the way it is.

### Recommended Changes

1. **Resolve the Summary/Requirements contradiction** (addresses: Summary
   states the Playwright-executor decision as settled, contradicting
   Requirements, Open Questions, and Acceptance Criteria) — Rewrite the
   Summary to hedge consistently with Requirements and Acceptance Criteria
   (e.g., "migrate ... while resolving whether the Playwright executor
   `run.sh` stays a thin wrapper or is folded into the binary, per
   ADR-0048"). Also remove the either/or restatement from either
   Requirements or Open Questions so the decision genuinely appears once,
   matching what the Drafting Notes already claim.

2. **Decide or gate the Playwright-executor choice before implementation
   starts** (addresses: Playwright-executor either/or leaves the story's
   size unbounded at authoring time) — Either state a default outcome
   (thin wrapper, consistent with the parent epic's resolved Open
   Questions) with fold-in treated as an explicit re-scope trigger, or
   require the decision to be confirmed before implementation begins
   rather than during it.

3. **Define the AC2 equivalence check** (addresses: "Same report artefact"
   has no defined equivalence criterion) — State the comparison method,
   e.g. byte-identical output for a fixed fixture input, or schema
   validation plus equivalent findings.

4. **Enumerate or reference the subcommand set** (addresses: "Per
   subcommand" coverage floor is uncountable; "The design binary's
   Playwright-driven subcommand" does not name which subcommand) — List
   the expected subcommands in Requirements or Technical Notes (or state
   explicitly that the set is whatever `inventory-design/scripts/*` and
   `analyse-design-gaps/scripts/*` resolve to), and name which one is
   Playwright-driven.

## Per-Lens Results

### Clarity

**Summary**: This work item is largely precise and internally disciplined
— it correctly distinguishes the `accelerator-design` binary name from the
`accelerator design` subcommand invocation throughout, and cross-references
sibling work items unambiguously. However, it undermines its own stated
purpose (resolving 0173's Playwright-executor ambiguity) by having the
Summary assert a settled outcome that contradicts the hedged either/or
preserved verbatim in Requirements, Open Questions, and Acceptance
Criteria, and the Drafting Notes' claim that the choice now "appears once,
in Open Questions" does not match the item's own content. A secondary,
lower-impact ambiguity leaves unclear which of the two migrated
subcommands is the "Playwright-driven" one referenced in Acceptance
Criteria bullet 2.

**Strengths**:
- Consistent naming discipline throughout: `accelerator-design` (sub-binary
  name) versus `accelerator design ...` (subcommand invocation) is never
  conflated across Summary, Requirements, and Acceptance Criteria.
- Cross-references to sibling and predecessor work items (0136, 0166,
  0167, 0173, 0174, 0187) are consistently identified and their relevance
  stated.
- Context is explicit about exactly which 0173 review-1 finding motivated
  this item's structure.

**Findings**:
- 🟡 Major (high confidence) — Summary states the Playwright-executor
  decision as settled, contradicting the hedge in Requirements/Open
  Questions/AC. Location: Summary.
- 🔵 Minor (medium confidence) — "The design binary's Playwright-driven
  subcommand" does not name which subcommand it refers to. Location:
  Acceptance Criteria.
- 🔵 Suggestion (low confidence) — "Repointed suites" used without a gloss.
  Location: Acceptance Criteria.

### Completeness

**Summary**: 0196 is a well-populated story: Context justifies the split
from 0173, Requirements are specific, Acceptance Criteria enumerate five
concrete checkable outcomes, and Dependencies is unusually thorough. The
main gap is a self-contradiction the item explicitly claims to have fixed:
the Summary asserts the Playwright-executor question is resolved while the
Requirements and Open Questions sections state it is still undecided. Two
smaller, lower-severity gaps round out the review: no explicit framing of
who/what needs the migration, and no Assumptions section.

**Strengths**:
- Acceptance Criteria contains five specific, checkable items, including
  exact test-coverage expectations and a reference to a concrete
  registration checklist.
- Dependencies distinguishes blocked-by (with each prior blocker's
  resolution stated), blocks (with the concrete lockstep coupling to 0174
  named), an external runtime coupling, and the parent epic.
- Context clearly explains the motivating force rather than merely
  restating the Summary.
- Frontmatter is fully populated and internally consistent with the
  referenced parent and split-from items.

**Findings**:
- 🔴 Major (high confidence) — Summary states the Playwright-executor
  decision as settled while Requirements and Open Questions state it is
  still undecided. Location: Summary.
- 🔵 Minor (medium confidence) — No explicit statement of who/what the
  migration serves. Location: Requirements.
- 🔵 Suggestion (low confidence) — No Assumptions section despite implicit
  assumptions underpinning the Acceptance Criteria. Location: Dependencies.

### Dependency

**Summary**: 0196's Dependencies section is unusually disciplined for a
split-out story: it explicitly resolves what could have been ambiguous
(naming three prior blockers as done), names the downstream consumer
(0174) with the reason it is blocked, and captures the pre-existing
Node/Playwright external coupling. The remaining observations are
low-confidence completeness nuances rather than missing hard blockers.

**Strengths**:
- Dependencies explicitly lists the three prior blockers as resolved
  (0166, 0167, 0187) rather than leaving Blocked-by empty.
- The Blocks entry for 0174 names the specific mechanism of coupling, and
  it is cross-referenced consistently in the Acceptance Criteria.
- The Node/Playwright external runtime coupling is correctly distinguished
  as pre-existing, carried forward from the bash scripts.
- Context and Drafting Notes explicitly state that this item and its
  siblings (0195, 0197) are functionally independent with no ordering
  constraint.

**Findings**:
- 🔵 Minor (low confidence) — Foundational launcher/dispatch infrastructure
  (0163, 0164) not named among resolved prior blockers. Location:
  Dependencies.
- 🔵 Minor (low confidence) — No coordination note for siblings sharing the
  same registration pattern, in case the checklist touches shared state.
  Location: Dependencies.

### Scope

**Summary**: This is a well-executed split: it carries forward exactly the
design-domain slice of the abandoned 0173, matches the granularity of its
siblings (0195, 0197), and its Requirements/Acceptance Criteria are tightly
coupled around one coherent deliverable. The one real scope concern is that
the Playwright-executor's fate is left as a decision to be made during
implementation, and those two outcomes represent materially different
amounts of work, so the story's actual size is not fully bounded at
authoring time.

**Strengths**:
- Cleanly carries forward only the design-domain slice of 0173, matching
  the granularity of siblings 0195 (corpus) and 0197 (collaboration).
- Requirements, Acceptance Criteria, and Dependencies all describe the
  same single unit of work with no unrelated capabilities smuggled in.
- The "story" kind fits the scope: a single team-owned subdomain migration
  deliverable and verifiable as one increment.

**Findings**:
- 🔴 Major (medium confidence) — Playwright-executor fate is an unresolved
  either/or with materially different scope outcomes; the parent epic's
  own resolved Open Questions suggest the thin-wrapper outcome was already
  expected. Location: Requirements / Open Questions.
- 🔵 Minor (low confidence) — Summary states the Playwright-executor
  outcome as settled, contradicting Open Questions. Location: Summary.

### Testability

**Summary**: The Acceptance Criteria are largely well-specified for a
migration story: AC5 anchors to a concrete, enumerable external checklist,
AC1 sets an explicit minimum test-coverage floor, and AC4 ties script
removal to a lockstep floor decrement tracked in a sibling work item. Two
gaps reduce measurability: the equivalence criterion for "the same report
artefact" in AC2 is undefined, and AC1's "per subcommand" coverage floor
cannot be counted from this document because the subcommand set is never
enumerated.

**Strengths**:
- AC5 anchors verification to a concrete, enumerable external artefact
  rather than a vague "is properly registered" claim.
- AC1 sets an explicit minimum floor (success path plus one failure path
  per subcommand) — a concrete, countable target rather than unbounded
  language.
- AC4's floor-decrement requirement is made verifiable by explicit
  cross-reference to work-item:0174.
- The Open Question is scoped so it doesn't undermine testability of AC2,
  which already covers both outcomes ("run.sh or its folded equivalent").

**Findings**:
- 🔵 Minor (medium confidence) — "Same report artefact" has no defined
  equivalence criterion. Location: Acceptance Criteria.
- 🔵 Minor (medium confidence) — "Per subcommand" coverage floor is
  uncountable without an enumerated subcommand list. Location: Acceptance
  Criteria.

---

## Re-Review (Pass 2) — 2026-08-06

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Clarity / Completeness / Scope**: Summary states the
  Playwright-executor decision as settled, contradicting Requirements, Open
  Questions, and Acceptance Criteria — Resolved
- 🟡 **Scope**: Playwright-executor either/or leaves the story's size
  unbounded at authoring time — Resolved
- 🔵 **Testability**: "Same report artefact" has no defined equivalence
  criterion — Resolved
- 🔵 **Testability**: "Per subcommand" coverage floor is uncountable
  without an enumerated subcommand list — Partially resolved (the
  deferral is now explicit, but a new finding below notes the recorded
  mapping isn't cross-checked against delivered tests)
- 🔵 **Clarity**: "The design binary's Playwright-driven subcommand" does
  not name which subcommand it refers to — Resolved
- 🔵 **Completeness**: No explicit statement of who/what the migration
  serves — Resolved
- 🔵 **Dependency**: Foundational launcher/dispatch infrastructure not
  named among resolved prior blockers — Resolved (a new suggestion below
  notes the subsumed item IDs could still be spelled out)
- 🔵 **Dependency**: No coordination note for siblings sharing the same
  registration pattern — Resolved
- 🔵 **Clarity**: "Repointed suites" used without a gloss — Resolved
- 🔵 **Completeness**: No Assumptions section despite implicit assumptions
  underpinning the Acceptance Criteria — Resolved

### New Issues Introduced

- 🔴 **Testability** (major, medium confidence): The restructured-format
  branch of AC2 ("schema-valid and equivalent in findings") has no defined
  verification procedure — no schema is named and "equivalent in findings"
  is undefined, so this branch has no pass/fail test if the format is
  ever restructured. Location: Acceptance Criteria.
- 🔵 **Clarity** (minor, high confidence): AC1's "the enumerated set (see
  Requirements)" points to a Requirements bullet that explicitly disclaims
  having an enumeration yet — the pointer resolves to the wrong section.
  Location: Acceptance Criteria.
- 🔵 **Testability** (minor, medium confidence): The Playwright-executor
  decision resolution has no corresponding Acceptance Criterion — nothing
  in the AC checklist would catch an unresolved or unrecorded decision.
  Location: Open Questions.
- 🔵 **Completeness** (minor, high confidence): Context explains the
  item's split provenance but not the underlying migration motivation
  (bash 3.2 floor removal, who benefits), unlike sibling work-item:0195.
  Location: Context.
- 🔵 **Scope** (minor, medium confidence): The fold-in alternative for the
  Playwright executor has no stated re-scoping trigger — if confirmed, it's
  unclear whether the rewrite stays in this item or is split out. Location:
  Open Questions.
- 🔵 **Testability** (suggestion, low confidence): The per-subcommand
  coverage bar depends on a mapping deferred to implementation time, with
  no AC step cross-checking the recorded mapping against delivered tests.
  Location: Acceptance Criteria.
- 🔵 **Dependency** (suggestion, low confidence): The launcher/dispatch
  scaffold reference could name work-item:0163/0164 explicitly, matching
  the naming discipline used for the other resolved blockers. Location:
  Dependencies.
- 🔵 **Clarity** (suggestion, medium confidence): Context/Drafting Notes'
  claim that the choice "appears once, in Open Questions" is no longer
  quite accurate, since the Summary also states the either/or (now
  consistently hedged rather than settled). Location: Context.

### Assessment

Every finding that drove the original REVISE verdict is resolved — the
structural contradiction between Summary and Requirements/Open
Questions/AC is gone, and the story's size is now bounded by a stated
default and a pre-implementation confirmation gate. The edit pass
introduced one new major finding: the "restructured format" branch added
to AC2's equivalence check is itself underspecified. That, plus the
AC1 cross-reference pointing to the wrong section, are worth one more
quick tightening pass before implementation begins; the remaining minor
and suggestion items are polish rather than blockers. The item is
acceptable but could be improved — see the major finding above.

### Manual Verdict Update — 2026-08-06

**Verdict:** APPROVE (updated from COMMENT by reviewer, no new lens pass)

After Pass 2, the sole major finding (AC2's underspecified
restructured-format branch) was fixed directly: AC2 now commits to
byte-identical output only, deferring format restructuring to a future
follow-up item, and AC1's cross-reference was corrected to point at
Drafting Notes rather than Requirements. The reviewer accepted the item
in this state; the remaining Pass 2 minor/suggestion findings (Context's
missing migration motivation, the fold-in re-scoping trigger, explicit
0163/0164 naming, the per-subcommand mapping cross-check, and the
Drafting Notes "appears once" phrasing) were left unaddressed as
non-blocking polish.

---
*Review generated by /accelerator:review-work-item*
