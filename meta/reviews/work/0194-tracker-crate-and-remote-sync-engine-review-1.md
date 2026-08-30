---
type: "work-item-review"
id: "0194-tracker-crate-and-remote-sync-engine-review-1"
title: "Work Item Review: Tracker Crate and Remote Sync Engine"
date: "2026-08-05T19:37:49+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0194"
work_item_id: "0194"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-08-05T19:56:06+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Tracker Crate and Remote Sync Engine

**Verdict:** COMMENT

0194 is a well-formed, clearly-scoped split-off from 0170 with strong
Acceptance Criteria (concrete Given/When/Then statements anchored to existing
bash parity fixtures) and a thorough Context/Drafting Notes trail explaining
the split rationale and the reversed sequencing it introduced. The findings
below are mostly minor polish and one major gap: the "side effect first,
baseline write last" resumability contract — the highest-risk property in a
sync engine — is asserted in Acceptance Criteria without a verification
procedure, and two lenses independently flag consequences of the sequencing
reversal that the document doesn't fully carry through.

### Cross-Cutting Themes

- **Underspecified resumability contract** (flagged by: testability, clarity)
  — the "side effect first, baseline write last" resumability contract named
  in AC1 is never defined or given a verification method anywhere in the
  document, and clarity independently flags it as a term used without a
  citation or inline gloss (alongside "the decision table" and
  `work.integration`).
- **Reversed sequencing has loose ends** (flagged by: scope, dependency) — the
  split now has 0194 precede 0170 rather than depend on it, but two
  consequences of that reversal aren't fully worked through: the Dependencies
  section frames 0170/0171's blocking need narrower ("the port is stable")
  than the item's actual Acceptance Criteria gate (the full sync command,
  test suites, and script removal), and 0194 doesn't carry the 0187
  sub-binary registration checklist that its sibling 0170 references, even
  though 0194 may now be the first story to exercise it.

### Findings

#### Major

- 🟡 **Testability**: Resumability contract in AC1 names a property without a
  verification procedure
  **Location**: Acceptance Criteria
  AC1 requires the write sequence to honour the "side effect first, baseline
  write last" resumability contract but doesn't specify how that would be
  checked — e.g. via write-order assertions on a recording fake, or a
  crash-injection test.

#### Minor

- 🟡 **Scope**: Blocking dependency is framed narrower than the item's own
  Acceptance Criteria gate
  **Location**: Dependencies
  Dependencies says 0170/0171 are blocked only until "the port is stable,"
  but this item's Acceptance Criteria bundle the port together with the full
  sync command, four characterization suites, and script removal — none of
  which the blocked siblings actually need.

- 🟡 **Dependency**: Sub-binary registration coupling (0187 checklist)
  referenced by 0170 but not by 0194
  **Location**: Technical Notes / Dependencies
  0170 cites the 0187 registration checklist for wiring the dispatch token;
  0194, which may now land first and first exercise the work binary's
  composition root, doesn't mention it.

- 🔵 **Completeness**: Requirements section omits scope that only appears in
  Acceptance Criteria
  **Location**: Requirements
  AC4's nextest-filter partitioning and AC6's script-removal/floor-decrement
  work are introduced only in Acceptance Criteria, with no mirroring
  Requirements bullet.

- 🔵 **Clarity**: Several domain terms/config keys are used without inline
  definition or link
  **Location**: Requirements; Acceptance Criteria
  `work.integration`, "the decision table," and the "side effect first,
  baseline write last" resumability contract are each used with a definite
  article but never defined or cited within the document.

- 🔵 **Clarity**: The sync-stage internal-function boundary is presented as
  both settled and open
  **Location**: Technical Notes; Open Questions
  Technical Notes states unconditionally that the five stages stay internal
  functions; Open Questions immediately asks whether that same boundary
  "holds" — leaving unclear whether it's fixed or provisional.

- 🔵 **Dependency**: Assumption about workspace-wide `reqwest` implicitly
  depends on the launcher (0164) without naming it in Dependencies
  **Location**: Assumptions
  The Assumptions section relies on 0164 having landed workspace-wide
  `reqwest`, but 0164 isn't listed in Dependencies alongside 0166/0187.

#### Suggestions

- 🔵 **Testability**: AC5 conflates two distinct verification mechanisms
  **Location**: Acceptance Criteria
  A dependency-graph check and a public-signature check are different
  procedures with different failure modes; AC5 reads as if one inspection
  covers both.

- 🔵 **Testability**: Composition-root provider wiring has no corresponding
  Acceptance Criterion
  **Location**: Requirements
  The Requirements' claim that provider selection is wired per
  `work.integration` has no Acceptance Criterion verifying the selection
  logic itself.

- 🔵 **Testability**: No criterion verifies the fake `RemoteTracker` conforms
  to the same contract as future real implementations
  **Location**: Assumptions
  Nothing establishes that the fake and 0171's eventual real implementation
  must satisfy an identical shared contract test.

- 🔵 **Scope**: Item combines novel port/design work with command build-out
  and script retirement
  **Location**: Requirements
  A wide set of distinct engineering activities for one story, mirroring its
  sibling's granularity but with schedule risk concentrated in the
  not-yet-finalized port design (see Open Questions).

- 🔵 **Clarity**: Bare "(resolved Q2)" citation has no inline referent
  **Location**: Assumptions
  The `reqwest` assumption cites "Q2" without naming what it refers to;
  confirming it requires opening the linked research document.

### Strengths

- ✅ Acceptance Criteria are consistently phrased as concrete Given/When/Then
  statements anchored to existing bash parity fixtures and named scripts,
  giving an unambiguous, externally-verifiable definition of done.
- ✅ Context and Drafting Notes give a well-evidenced rationale for the split
  from 0170 (tracker crate depends only on already-built shared crates) and
  are cross-checked as consistent with 0170's own Context, Dependencies, and
  Drafting Notes.
- ✅ Both downstream consumers of the shared `RemoteTracker` port (0170,
  0171) are named as Blocks with the specific coupling mechanism, and
  provider-specific concerns are deliberately kept out of scope — even
  verified by an Acceptance Criterion checking the crate's public API for
  provider/HTTP types.
- ✅ Technical Notes enumerates the exact source-bash inventory being
  ported/retired and pre-empts an obvious implementer question about
  `work-item-fetch-remote.sh`'s existing test coverage.

### Recommended Changes

1. **Specify a verification procedure for the resumability contract**
   (addresses: "Resumability contract in AC1 names a property without a
   verification procedure") — add a concrete mechanism to AC1, e.g. a
   write-order assertion against a recording fake store, or a
   crash-injection test that interrupts between the side-effect write and
   the baseline write and confirms a resumed run reaches the same terminal
   state.

2. **Reconcile the Dependencies blocking claim with the AC gate, or split
   the port-stabilization milestone out** (addresses: "Blocking dependency
   is framed narrower than the item's own Acceptance Criteria gate") —
   clarify whether 0170/0171 can proceed once the port trait compiles, or
   accept that they wait for the full bundle.

3. **Add a 0187 registration-checklist cross-reference to Technical Notes**
   (addresses: "Sub-binary registration coupling referenced by 0170 but not
   by 0194") — mirror 0170's note now that 0194 may exercise the
   composition root first.

4. **Mirror AC4/AC6 scope in Requirements** (addresses: "Requirements
   section omits scope that only appears in Acceptance Criteria") — add
   bullets covering the nextest-filter partitioning and script
   removal/floor decrement.

5. **Gloss or cite undefined terms** (addresses: "Several domain terms/config
   keys are used without inline definition or link", "Bare '(resolved Q2)'
   citation has no inline referent") — add short parentheticals or citations
   for `work.integration`, "the decision table," the resumability contract,
   and the Q2 reference.

6. **Resolve the settled-vs-open tension on the internal-function boundary**
   (addresses: "The sync-stage internal-function boundary is presented as
   both settled and open") — either soften the Technical Notes statement as
   provisional or narrow the Open Question.

## Per-Lens Results

### Clarity

**Summary**: 0194 is largely clear and internally consistent: the tracker
crate, the RemoteTracker port, and the five-stage sync pipeline are named
once and then referred to consistently across Summary, Context,
Requirements, and Acceptance Criteria, and its account of the split from
0170 matches 0170's own text almost exactly. A small number of domain terms
and cross-references are asserted with a definite article or a bare
citation but never defined or linked within the document itself, and one
Technical Notes statement reads as settled while the adjacent Open Question
treats the same point as still unresolved.

**Strengths**:
- Terminology for the crate, the port, and the pipeline stages is used
  consistently throughout.
- The item's narrative of the 0170 split is cross-checked against and
  matches 0170's own Context, Dependencies, and Drafting Notes.
- Acceptance criteria consistently name the actor and trigger and tie each
  expected outcome to a specific named existing script for comparison.

**Findings**:
- 🔵 Suggestion (medium confidence) — Bare "(resolved Q2)" citation has no
  inline referent. Location: Assumptions.
- 🔵 Minor (medium confidence) — Several domain terms/config keys are used
  without inline definition or link. Location: Requirements; Acceptance
  Criteria.
- 🔵 Minor (medium confidence) — The sync-stage internal-function boundary
  is presented as both settled and open. Location: Technical Notes; Open
  Questions.

### Completeness

**Summary**: 0194 is an unusually complete work item for a freshly-split
story: every expected section is present and substantively populated, the
Acceptance Criteria are expressed as detailed Given/When/Then statements,
and Context/Drafting Notes trace the split rationale and dependency
reversal in full. Frontmatter is intact and kind-appropriate. The only gap
found is a minor asymmetry between the Requirements section and the
Acceptance Criteria.

**Strengths**:
- Acceptance Criteria are all phrased as specific Given/When/Then statements
  referencing concrete artefacts.
- Context section explains both the mechanical and process motivation for
  the item's existence.
- Dependencies section explicitly confirms both pre-split blockers are
  resolved and states the direction and reason for outbound blocking
  relationships.
- Technical Notes enumerates the exact source-bash inventory being
  ported/retired and pre-empts an obvious implementer question.

**Findings**:
- 🔵 Minor (medium confidence) — Requirements section omits scope that only
  appears in Acceptance Criteria. Location: Requirements.

### Dependency

**Summary**: 0194 captures its primary couplings well: upstream blockers
(0166, 0187) are named and correctly marked resolved, and the two
downstream consumers of the RemoteTracker port (0170, 0171) are both
explicit Blocks entries with a clear rationale. The main gap is a
second-order ordering question the split introduced: since 0194 now
precedes 0170, it may be the first story to scaffold the accelerator-work
binary, but it doesn't carry the sub-binary registration coupling that
0170's own Technical Notes reference.

**Strengths**:
- Dependencies section explicitly names both prior blockers, states they
  are done, and gives the precise reason the story is unblocked.
- Both downstream consumers of the shared RemoteTracker port artefact are
  listed as Blocks with the specific coupling mechanism named, verified
  consistent with 0170's own frontmatter.
- The story deliberately keeps provider-specific/external-system coupling
  out of its own scope, correctly deferring that coupling to 0171.

**Findings**:
- 🟡 Minor (medium confidence) — Sub-binary registration coupling (0187
  checklist) referenced by 0170 but not by 0194, despite 0194 now preceding
  it. Location: Technical Notes / Dependencies.
- 🔵 Minor (low confidence) — Assumption about workspace-wide reqwest
  implicitly depends on the launcher (0164) without naming it in
  Dependencies. Location: Assumptions.

### Scope

**Summary**: 0194 is a well-formed split-off from 0170's originally
epic-scale story: the tracker crate and the sync command form one
coherent, service-boundary-respecting unit, and the Summary/Requirements/
Acceptance Criteria stay in alignment throughout. The one scope-relevant
wrinkle is that the item's own Dependencies section distinguishes between
"the port being stable" and the item's full Acceptance Criteria gate,
bundling these as a single indivisible deliverable.

**Strengths**:
- Context and Drafting Notes document a concrete, evidence-based rationale
  for the split, rather than an arbitrary halving of a large story.
- Summary, Requirements, and Acceptance Criteria describe the same scope
  throughout, with no mismatched or bolted-on capability.
- The item explicitly keeps provider-specific concerns out of scope,
  cleanly respecting the service boundary with 0171.

**Findings**:
- 🟡 Minor (medium confidence) — Blocking dependency is framed narrower
  than the item's own Acceptance Criteria gate. Location: Dependencies.
- 🔵 Suggestion (low confidence) — Item combines novel port/state-machine
  design with command build-out and script retirement. Location:
  Requirements.

### Testability

**Summary**: Most Acceptance Criteria are strong: they anchor verification
to concrete external oracles and bound the characterization-test criterion
to documented flag/argument combinations plus at least one error path. The
main weaknesses are AC1's resumability-contract clause, which names a
property without specifying how compliance is checked, and AC5's
verification method, which conflates a dependency-graph check with a
public-API-signature check as if they were one procedure.

**Strengths**:
- AC1 and AC2 anchor verification to existing, named external oracles,
  giving a definitive pass/fail procedure.
- AC3 bounds the characterization-test obligation concretely.
- AC4 gives a clear, mechanically checkable separation between the default
  no-network test run and a separately tagged contract/integration suite.
- AC6 is a concrete, binary check rather than a vague "cleanup is done"
  claim.

**Findings**:
- 🟡 Major (medium confidence) — Resumability contract in AC1 names a
  property without a verification procedure. Location: Acceptance
  Criteria.
- 🔵 Minor (medium confidence) — AC5 conflates two distinct verification
  mechanisms. Location: Acceptance Criteria.
- 🔵 Suggestion (low confidence) — Composition-root provider wiring has no
  corresponding Acceptance Criterion. Location: Requirements.
- 🔵 Suggestion (low confidence) — No criterion verifies the fake
  RemoteTracker conforms to the same contract as future real
  implementations. Location: Assumptions.


## Re-Review (Pass 2) — 2026-08-05T19:56:06+00:00

**Verdict:** APPROVE (verdict overridden by reviewer from COMMENT)

### Previously Identified Issues

- 🟡 **Testability**: Resumability contract in AC1 names a property without a
  verification procedure — Resolved. AC1 now specifies a concrete two-part
  verification mechanism: a write-order assertion via a fake store, plus a
  crash-then-rerun determinism check.
- 🟡 **Scope**: Blocking dependency is framed narrower than the item's own
  Acceptance Criteria gate — Resolved. Both Blocks entries now explicitly
  state the blocking milestone is the `RemoteTracker` port signature
  compiling, not the full AC gate.
- 🟡 **Dependency**: Sub-binary registration coupling (0187 checklist)
  referenced by 0170 but not by 0194 — Resolved. Technical Notes now cites
  the same 0187 registration checklist that 0170 references, tied to the
  reversed-sequencing rationale.
- 🔵 **Completeness**: Requirements section omits scope that only appears in
  Acceptance Criteria — Resolved. Requirements now has bullets mirroring
  AC4's test-partitioning and AC6's removal/floor-decrement scope.
- 🔵 **Clarity**: Several domain terms/config keys used without inline
  definition or link — Resolved. `work.integration`, "the decision table,"
  and the resumability contract all carry inline glosses at first use.
- 🔵 **Clarity**: The sync-stage internal-function boundary presented as
  both settled and open — Resolved. Technical Notes now hedges the claim as
  provisional and cross-references Open Questions.
- 🔵 **Clarity**: Bare "(resolved Q2)" citation has no inline referent —
  Resolved. The Assumptions bullet now names the research document and
  Open Question 2 explicitly.
- 🔵 **Dependency**: Assumption about workspace-wide `reqwest` implicitly
  depends on the launcher (0164) without naming it in Dependencies — Still
  present. Not selected for fixing; low-confidence, non-blocking per the
  dependency lens.
- 🔵 **Testability**: AC5 conflates two distinct verification mechanisms —
  Still present. Not selected for fixing.
- 🔵 **Testability**: Composition-root provider wiring has no corresponding
  Acceptance Criterion — Still present. Not selected for fixing.
- 🔵 **Testability**: No criterion verifies the fake `RemoteTracker`
  conforms to the same contract as future real implementations — Still
  present. Not selected for fixing.

### New Issues Introduced

- 🔵 **Clarity**: Hedge scope ("the five stages") and the Open Question it
  cites (naming only `work-item-sync-decide.sh`) don't match — the
  Technical Notes hedge now points to an Open Question that only names one
  of the five stages, leaving unclear whether the other four are settled.
- 🔵 **Clarity**: "The reversed sequencing" is used in Technical Notes before
  its rationale is spelled out in Drafting Notes, several sections later.

### Assessment

The work item is ready for implementation. The major finding (the
resumability contract's missing verification procedure) is resolved, and
all four selected minor/suggestion fixes landed cleanly with no regressions
in the areas they touched. The two new clarity wrinkles are minor polish
introduced by the hedge itself, not correctness gaps, and the four
remaining still-present items were explicitly deferred by the user's
choice rather than missed. No further re-review is required unless the
remaining open items are picked up later.

---
*Review generated by /accelerator:review-work-item*
