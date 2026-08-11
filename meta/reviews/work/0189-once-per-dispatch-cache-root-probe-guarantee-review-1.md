---
type: work-item-review
id: "0189-once-per-dispatch-cache-root-probe-guarantee-review-1"
title: "Work Item Review: Once-Per-Dispatch Guarantee for the Launcher's Cache-Root Probe"
date: "2026-08-11T11:30:14+00:00"
author: "Toby Clemson"
producer: review-work-item
status: complete
target: "work-item:0189"
work_item_id: "0189"
reviewer: "Toby Clemson"
verdict: "REVISE"
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: [cli, launcher, testability, dependency]
last_updated: "2026-08-11T13:21:34+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Once-Per-Dispatch Guarantee for the Launcher's Cache-Root Probe

**Verdict:** REVISE

0189 is an honestly re-scoped item with strong provenance — its Context
narrates what 0169 already delivered, names the tests that discharge two of its
original criteria, and openly retracts an earlier amendment's incorrect claim.
All five lenses recognised that quality. The verdict is REVISE because two
problems recur across four lenses each: the item rests its entire performance
justification on a 0169 gate that 0169 itself records as *deferred and
release-blocked*, and every count-based acceptance criterion depends on a
counting seam left undecided, whose leading candidate cannot express the value
one of those criteria demands.

### Cross-Cutting Themes

- **The 0169 latency gate is not evidence** (flagged by: completeness, scope,
  dependency, clarity) — Assumptions asserts in the present tense that 0169's
  Phase 10 warm-dispatch gate "is sufficient evidence that the latency this
  item was raised to remove is gone". 0169 records that criterion as unchecked
  with B, G, ratio, payload, fixture and host all *pending*, and its plan marks
  Phase 10 "Deferred, not merely unattempted" because measuring it needs a
  minisign-signed `accelerator-vcs` release asset that does not exist
  pre-release. Meanwhile 0169 and 0186 both still carry live prose naming 0189
  as the dominant unaddressed cost. Each item points at the other, and no
  document owns proving the saving reaches a warm `accelerator vcs guard`.

- **The counting seam is undecided, and the cheap option may not work**
  (flagged by: completeness, scope, testability, clarity) — three of five
  criteria specify probe counts (1 / 1 / 0) without specifying how the count is
  observed, and the Open Question defers the choice with no stated default. The
  candidate named as simple, the per-process `SEQUENCE` atomic, counts for the
  lifetime of the process, so the "zero times" warm-hit criterion is
  unsatisfiable if the cache was warmed by an earlier probing resolve in the
  same process. The alternative, injecting the probe behind a port, is a
  resolver restructure the item itself calls larger than the invariant warrants
  — so the item's size is unbounded until the seam is chosen.

- **The refetch criterion does not say which outcome it means** (flagged by:
  testability, clarity) — AC2 says a cached binary "fails re-verification and
  is refetched", but Context motivates the criterion from
  `CorruptCacheAndRefetchFailed`, the case where the refetch *fails*. Both
  readings satisfy the wording, so the branch the item was amended to protect
  can go untested behind a green criterion.

- **AC4 contradicts Technical Notes** (flagged by: clarity, testability) —
  AC4 requires all four re-homed assertions "still made", while Technical Notes
  invites dropping the read-only case as redundant with an existing test. Two
  verifiers reach opposite conclusions on the same implementation.

### Findings

#### Critical

None.

#### Major

- 🟡 **Dependency**: Reliance on 0169's Phase 10 latency gate is an unrecorded
  dependency on a deferred, release-blocked measurement
  **Location**: Assumptions
  0169's Phase 10 is marked "Deferred, not merely unattempted" — measuring G
  through the real dispatch path requires a real, minisign-signed
  `accelerator-vcs` release asset, which does not exist pre-release. That
  release cut is owned by whoever performs epic-0136 releases and needs the
  signing key. None of this coupling appears in 0189's Dependencies section.

- 🟡 **Scope**: The performance dimension is descoped onto an unverified gate
  and lands nowhere
  **Location**: Assumptions / Dependencies
  The 131.97 ms per-dispatch cost that motivated the item has been descoped out
  of 0189 without landing anywhere else, so no work item now owns demonstrating
  that the saving actually reaches a warm `accelerator vcs guard` — the point
  of the epic-level latency budget.

- 🟡 **Completeness**: The dropped measurement criterion is delegated to a 0169
  gate the referenced document records as pending
  **Location**: Assumptions
  The performance outcome the item was originally raised to deliver currently
  has no recorded evidence in either work item, so 0189 can close with its
  motivating benefit unverified anywhere.

- 🟡 **Clarity**: "0169's Phase 10 gate is sufficient evidence" reads as
  discharged, but 0169 records it as pending
  **Location**: Assumptions
  Assumptions and Drafting Notes both use the present tense, implying the
  evidence already exists. A reader cannot tell whether the item carries a
  residual verification obligation or none at all — which determines whether
  the five criteria are genuinely the whole of "done".

- 🟡 **Completeness**: The sole Open Question leaves the primary deliverable's
  mechanism undecided with no stated default
  **Location**: Open Questions
  An implementer cannot start the majority of the work without returning to the
  author for a design decision that materially changes the crate's public
  surface or its resolver structure — the exact follow-up a complete work item
  should pre-empt.

- 🟡 **Testability**: The count criteria rest on an undecided seam, and the
  leading candidate cannot express "zero"
  **Location**: Acceptance Criteria / Open Questions
  The per-process `SEQUENCE` atomic counts probes for the lifetime of the
  process, so AC3's "zero times" is unsatisfiable if the warm cache was
  populated by an earlier probing resolve in the same test process — the
  counter would read 1, not 0. Restating the three criteria as a delta captured
  either side of the single resolve call under test (1 / 1 / 0) resolves this
  in the criteria rather than deferring it.

- 🟡 **Testability**: The no-memoisation Requirement is undetectable by every
  criterion
  **Location**: Requirements / Acceptance Criteria
  A memoising implementation passes all three count criteria: a fresh process
  doing one cold-miss resolution still probes once, the refetch path still
  totals one, and a warm hit still probes zero. The Requirement stated most
  emphatically is the one nothing can catch. A criterion over two successive
  cold-miss resolutions in one process (total 2) fails under memoisation and
  passes under the structural property the item wants.

- 🟡 **Testability**: Nothing demonstrates the new guard is capable of failing
  **Location**: Acceptance Criteria
  The invariant already holds in the current tree, so every count criterion
  passes before any change is made — including against a test whose counter is
  never wired up. A miscounting seam yields a permanently green, permanently
  vacuous guard, which is precisely what the item exists to prevent.

- 🟡 **Testability**: AC2 states neither the fault-injection mechanism nor
  whether the refetch succeeds
  **Location**: Acceptance Criteria (criterion 2)
  A verifier can satisfy the wording with the successful-refetch path while
  leaving the failure path — the exact branch the 2026-08-06 amendment alleged
  double-probes — unexercised.

- 🟡 **Clarity**: AC2's refetch scenario does not say whether the refetch
  succeeds or fails
  **Location**: Acceptance Criteria
  "Is refetched" and "resolution completes" are each satisfiable by both a
  successful refetch and a failed one that returns an error, while Context
  motivates the criterion from `CorruptCacheAndRefetchFailed`.

- 🟡 **Clarity**: The probe is named two different ways with no statement that
  they are the same function
  **Location**: Technical Notes
  Context says the probe was "renamed `verify_writable`"; Technical Notes then
  attributes the `SEQUENCE` atomic to `probe_writable_and_executable` without
  saying that is the pre-rename name. The Open Question's proposed seam hangs
  off the name the reader has just been told no longer applies.

- 🟡 **Clarity**: AC4 requires all four assertions preserved while Technical
  Notes invites dropping one as redundant
  **Location**: Acceptance Criteria
  An implementer who follows the Technical Note and drops the redundant
  read-only test cannot tell whether AC4 then passes or fails, so the
  definition of done for that criterion is genuinely contested.

#### Minor

- 🔵 **Scope**: The item's size is unbounded until the seam is chosen
  **Location**: Open Questions
  A low-priority regression guard could turn into a ports-and-adapters refactor
  of `FetchVerifyCacheResolver`, which is a different unit of work from the one
  described.

- 🔵 **Dependency**: No Blocks entry, and the live upstream claims in 0169 and
  0186 that this item gates their latency work are left unretracted
  **Location**: Dependencies
  Anyone planning from 0169 or 0186 will still believe the launcher probe cost
  is outstanding and that 0189 must land first, which is no longer true.

- 🔵 **Dependency**: Ordering between the seam decision, the invariant test and
  the deletion of `cache_root::resolve` is unstated
  **Location**: Requirements
  A per-process counter is only a reliable per-resolution observable once the
  second probe call site is gone, so the deletion and the counting test are
  ordered against each other rather than independent.

- 🔵 **Completeness**: The no-memoisation Requirement has no corresponding
  acceptance criterion
  **Location**: Acceptance Criteria
  A Requirement the author considered important enough to write down explicitly
  has nothing in the definition of done that would surface its violation.

- 🔵 **Clarity**: Three counting units — dispatch, resolution, process — are
  used for one invariant, and the title asserts "once" where the common case is
  zero
  **Location**: Title
  The three are equated only transitively and only "in production", via an
  Assumption. Separately the title promises a "Once-Per-Dispatch Guarantee"
  while AC3 requires zero probes on a warm hit, the dominant production case.

- 🔵 **Clarity**: Requirements say "at most one probe" while Acceptance
  Criteria mandate exact counts
  **Location**: Requirements
  A test written to the Requirement (count ≤ 1) would not satisfy the criteria;
  a test written to the criteria is strictly stronger.

- 🔵 **Clarity**: The proposed per-process counter cannot obviously answer the
  per-resolution criteria
  **Location**: Open Questions
  Nothing states whether a per-process counter is deemed adequate for a
  per-resolution criterion, or under what test conditions.

- 🔵 **Clarity**: "It describes code that no longer exists" contradicts the same
  section's account of the probe
  **Location**: Context
  The probe survives as `verify_writable` on the miss path; what no longer
  exists is the unconditional warm-path invocation.

- 🔵 **Clarity**: A referenced 2026-08-06 amendment to this item is not present
  in the item
  **Location**: Context
  The amendment is the sole motivation for AC2, and a reader has no way to
  reach its original reasoning.

- 🔵 **Clarity**: "Root" refers to both the plugin root and the cache root
  without their relationship being stated
  **Location**: Acceptance Criteria
  AC4's labels say "plugin-root" and "read-only-root" in an item about the
  cache root, and two similarly-named read-only tests exist.

- 🔵 **Testability**: AC4's pass condition for the read-only assertion is
  contested
  **Location**: Acceptance Criteria (criterion 4) / Technical Notes
  Whether pointing at the pre-existing equivalent test discharges the criterion,
  or whether a newly re-homed assertion is required, is not stated.

- 🔵 **Testability**: AC1 and AC3 give no fixture preconditions
  **Location**: Acceptance Criteria (criteria 1 and 3)
  Different setups (pre-populating the cache on disk versus warming via a prior
  resolve) yield different observed counts for the same correct implementation,
  so a failure would be ambiguous between a real regression and a fixture
  choice.

#### Suggestions

- 🔵 **Completeness**: The Summary describes the situation rather than naming
  the two deliverables
  **Location**: Summary
  A reader scanning the Summary alone learns the problem's history but not what
  will be built.

- 🔵 **Scope**: The remaining scope is defined entirely by a transient state of
  the tree
  **Location**: Requirements
  Its scope has already been invalidated once this way. A one-line pick-up check
  — re-confirm `cache_root::resolve` still has no production caller and that
  every branch of `resolve` still returns — would catch a second round.

- 🔵 **Dependency**: 0164, which created the resolver and probe this item
  guards, is in frontmatter but absent from the Dependencies prose
  **Location**: Dependencies
  A reviewer checking whether "exactly one probe per resolution" is a legitimate
  invariant of the cache-resolution contract has no pointer to the item that
  defined that contract.

### Strengths

- ✅ The item was honestly re-scoped rather than left asserting work that no
  longer exists; Summary, Requirements and Acceptance Criteria all describe the
  same residual, and Drafting Notes record every narrowing decision with its
  rationale.
- ✅ Context openly retracts a prior amendment's incorrect claim rather than
  silently dropping it, and names the two specific tests (with file:line) that
  already discharge two original criteria.
- ✅ Requirement 3 is an explicit anti-requirement with its reason attached, so
  an obvious-looking optimisation cannot be reintroduced unknowingly.
- ✅ The two deliverables are genuinely cohesive: deleting `cache_root::resolve`
  is what makes `fetch_verify_store` the single probe entry point, which is the
  structural half of the guarantee the test asserts.
- ✅ AC3 pre-empts weak verification by demanding an asserted count rather than
  an inferred permission side effect.
- ✅ Technical Notes name the exact four unit tests to re-home and flag one as
  likely redundant, so the removal needs no discovery pass.
- ✅ Kind and priority were adjusted downward to match the reduced scope, and
  the title was changed away from a claim no longer true of the code.
- ✅ Scope boundaries are stated at module granularity — one crate, one module,
  one production call site — with no cross-component spread.

### Recommended Changes

1. **Stop claiming 0169's gate as discharged evidence** (addresses: all four
   0169-theme findings)
   Rewrite the Assumptions entry to say the coverage is *delegated and still
   outstanding*, not already taken. Add a Dependencies entry naming the real
   coupling: the latency claim is discharged only by 0169's Phase 10 Validation
   Results, which is itself blocked on the epic-0136 release cut and its signing
   key. Then decide explicitly whether 0189 restores a cheap launcher-local
   measurement or accepts the claim staying unverified until the release.

2. **Resolve the counting seam in the item, not at implementation time**
   (addresses: the seam findings from four lenses)
   State a default in the Open Question so the item is startable, and restate
   AC1–AC3 as a **delta captured immediately before and after the single
   `FetchVerifyCacheResolver::resolve` call under test** (1 / 1 / 0). A delta is
   observable through the per-process counter and sidesteps the "zero is
   unreachable in a warm process" problem. Add that if the seam cannot be added
   without restructuring the resolver, that restructure is out of scope and
   becomes its own item.

3. **Add a criterion that fails under memoisation** (addresses: the
   no-memoisation findings)
   For example: two successive cold-miss resolutions in one process increment
   the probe count once each, total 2.

4. **Add a mutation check so the guard is shown to guard** (addresses: the
   vacuous-guard finding)
   Deliberately introduce a second `verify_writable` call at the resolver,
   confirm the cold-miss and refetch cases go red, revert, and record the
   command and output.

5. **Disambiguate AC2** (addresses: the refetch findings from two lenses)
   Name the fault-injection mechanism and split the criterion into the
   refetch-succeeds case and the refetch-fails
   (`CorruptCacheAndRefetchFailed`) case, each asserting a probe delta of
   exactly 1.

6. **Settle AC4 against Technical Notes** (addresses: the AC4 contradiction)
   State the pass condition once: each of the four assertions is discharged by a
   named test against `candidate` or `verify_writable`, the read-only case may
   be discharged by the existing `verify_writable_rejects_a_read_only_directory`,
   and the old-test → discharging-test mapping is recorded.

7. **Fix the naming and unit inconsistencies** (addresses: the clarity minors)
   Use one name for the probe and mark the other as the pre-rename name; define
   *dispatch* once near the Summary; align Requirement 1's "at most one" with
   the criteria's exact counts; qualify which root each AC4 label refers to; and
   soften "describes code that no longer exists" to name the warm-path
   invocation specifically.

8. **Retract the stale downstream claims** (addresses: the Blocks finding)
   Record "Blocks: none" with the reason, and append a dated note to 0169 and
   0186 retracting the "dominant unaddressed cost" framing now that 0169's
   Phase 5 absorbed the fix.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Unusually well-provenanced — Drafting Notes explain every
reinterpretation and Context openly restates what is now stale — but the central
subject is named inconsistently and the binding contract shifts between
sections. The probe is `verify_writable` in Context and
`probe_writable_and_executable` in Technical Notes with no statement that these
are the same function; Requirements say "at most one probe" while Acceptance
Criteria mandate exact counts of 1/1/0; and AC4's "all still made" conflicts
with Technical Notes' invitation to drop one assertion as redundant. Three
counting units — dispatch, resolution, process — are used interchangeably for
the headline invariant, equated only transitively and only "in production".

**Strengths**: Summary and Context are explicit about what is stale and why;
AC3 precisely disambiguates itself from the already-passing test; Requirement 3
records the rejected approach with its reason; the Open Question names both
candidate seams with concrete trade-offs; Technical Notes name all four unit
tests exactly so AC4's shorthand labels resolve; Drafting Notes explain every
terminology and scope change.

**Findings**: 4 major (probe named two ways, high; AC4 vs Technical Notes,
high; AC2 succeed-or-fail, medium; Assumptions reads discharged, medium),
6 minor (three counting units and the title's "once" vs zero, high;
Requirements "at most one" vs exact counts, medium; per-process counter vs
per-resolution criteria, medium; "code that no longer exists" self-contradiction,
medium; the referenced amendment is absent from the item, medium; plugin root
vs cache root ambiguity, medium).

### Completeness

**Summary**: Structurally complete — every expected section is present and
substantively populated, frontmatter is intact and coherent, and Context is
exceptionally strong. Two gaps limit actionability: the sole Open Question names
the test seam as undecided with no stated default even though building that seam
is one of only two deliverables; and the item drops its original measurement
criterion in favour of a 0169 gate the referenced work item records as pending,
leaving no document carrying evidence for the outcome the item was raised to
achieve. Minor gaps: a Requirement with no corresponding criterion, and a
Summary that states the situation rather than naming the deliverables.

**Strengths**: Context explains the forces and retracts a prior incorrect
amendment; Requirements include an explicit anti-requirement with rationale;
Acceptance Criteria enumerate three distinct resolution paths plus the dead-code
removal and the build gate; Drafting Notes record every interpretation;
frontmatter is complete with the low priority justified in-body; Technical Notes
name the four tests and flag one as likely redundant.

**Findings**: 2 major (Open Question with no default, high; measurement
delegated to a pending gate, medium), 1 minor (no-memoisation Requirement has no
criterion, medium), 1 suggestion (Summary names no deliverables, medium).

### Dependency

**Summary**: For a task this small the coupling record is mostly good —
Dependencies explains how 0169's Phase 5 discharged the original requirements,
names the tests satisfying two original criteria, records the parent epic and
the pattern-setting relation to 0186, and Requirement 3 captures the internal
coupling to the multi-threaded concurrent-first-use tests. The significant gap
is upstream: the Assumptions rest the entire justification for dropping the
measurement criterion (and for lowering priority) on 0169's Phase 10 gate, which
the 0169 plan records as explicitly *deferred* and blocked on a signed
`accelerator-vcs` release artefact that does not yet exist. Secondarily the
reciprocal downstream edge is one-sided: 0169 and 0186 both still carry live
prose describing 0189 as the dominant unaddressed cost.

**Strengths**: The upstream discharge is recorded with unusual precision;
Requirement 3 captures a genuine internal coupling and uses it as the stated
reason memoisation is ruled out; the absence of external-system entries is
correct rather than an omission; the Open Question carries its own explicit
ordering constraint; the parent epic and the 0186 relation are both recorded.

**Findings**: 1 major (reliance on a deferred, release-blocked measurement,
high), 2 minor (no Blocks entry and stale upstream claims unretracted, high;
ordering between seam decision, test and deletion unstated, medium),
1 suggestion (0164 absent from Dependencies prose, medium).

### Scope

**Summary**: A small, coherent task — pin a once-per-resolution probe invariant
with a test and delete the dead `cache_root::resolve` that keeps a second probe
path alive in the same module. The two deliverables share one purpose, and the
`kind: task` / `priority: low` framing fits the residual scope after 0169
absorbed the original fix. The main concerns are that the performance dimension
has been descoped onto a 0169 criterion the referenced document shows as
unverified, and that the size of the remaining test work is left undetermined by
an Open Question whose two options differ materially in cost.

**Strengths**: Honestly re-scoped rather than left asserting work that no longer
exists; explicit negative scope in Requirement 3; the two deliverables are
genuinely cohesive rather than bundled; kind and priority adjusted downward to
match; scope boundaries stated at module granularity with no cross-component
spread.

**Findings**: 1 major (performance dimension descoped onto an unverified gate,
high), 1 minor (size unbounded until the seam is chosen, high), 1 suggestion
(scope defined by a transient state of the tree; add a pick-up check, medium).

### Testability

**Summary**: The Acceptance Criteria are unusually count-explicit for a
regression-guard task — AC3 in particular insists on an asserted count rather
than an inferred permission side effect — and the three named resolution paths
map cleanly onto three criteria. The weakness is that every count-based
criterion rests on a counting seam the item deliberately leaves undecided, and
the leading candidate (the per-process `SEQUENCE` atomic) cannot express AC3's
"zero" without a stated delta convention. Two further gaps: the refetch
criterion never states how re-verification failure is induced or whether the
refetch succeeds, and the "do not memoise" Requirement is satisfied by an
implementation that passes all three count criteria.

**Strengths**: AC3 pre-empts the classic weak verification here; Context
discharges two original criteria by naming exact tests and locations;
Requirements enumerate the three resolution paths and the criteria cover exactly
those three; Drafting Notes record why the original latency gate was dropped;
AC4 names four specific assertions rather than saying "the unit tests are
preserved"; the Open Question surfaces the seam decision explicitly.

**Findings**: 4 major (count criteria rest on an undecided seam and the leading
candidate cannot express zero, high; no-memoisation Requirement undetectable,
high; AC2 fault injection and outcome unstated, medium; nothing shows the guard
can fail, medium), 2 minor (AC4 pass condition contested, medium; AC1/AC3
fixture preconditions absent, medium).

## Re-Review (Pass 2) — 2026-08-11

**Verdict:** REVISE

All twelve major findings from pass 1 are resolved. Three new majors were
introduced by the revision itself, each narrower than what it replaced and each
fixable in a few lines. The item moved from "claims it cannot support and
criteria that cannot be executed" to "a well-specified guard with three loose
ends", two of which are consequences of the fixes applied.

### Previously Identified Issues

- 🟡 **Dependency**: Reliance on a deferred, release-blocked measurement —
  **Resolved**. Dependencies now names the document that would discharge it, why
  it cannot be taken pre-release, and who owns the release cut.
- 🟡 **Scope**: Performance dimension descoped onto an unverified gate —
  **Resolved**. Both Dependencies and Assumptions state plainly that the claim
  is undischarged and that no measurement is taken or awaited here.
- 🟡 **Completeness**: Measurement delegated to a pending gate — **Resolved**.
- 🟡 **Clarity**: "0169's gate is sufficient evidence" reads as discharged —
  **Resolved**. Both statements now read in the negative.
- 🟡 **Completeness**: Open Question with no default — **Resolved**. The default
  seam is stated with an explicit out-of-scope escape hatch.
- 🟡 **Testability**: Count criteria rest on an undecided seam; the candidate
  cannot express zero — **Resolved** by the delta convention.
- 🟡 **Testability**: No-memoisation Requirement undetectable — **Resolved** by
  AC5 (two cold-miss resolutions, total 2).
- 🟡 **Testability**: Nothing shows the guard can fail — **Resolved** by AC6's
  mutation check.
- 🟡 **Testability** / 🟡 **Clarity**: AC2 refetch outcome and fault injection
  unstated — **Resolved**. Split into AC3 (refetch succeeds) and AC4 (refetch
  fails with `CorruptCacheAndRefetchFailed`).
- 🟡 **Clarity**: Probe named two ways — **Resolved**. The rename and the
  delegation are both stated.
- 🟡 **Clarity**: AC4 versus Technical Notes contradiction — **Resolved**. AC7
  states the pass condition once and permits the pre-existing test to discharge
  the read-only case.
- 🔵 Minor and suggestion items from pass 1 — the counting-unit definition,
  Requirements/criteria count alignment, the plugin-root/cache-root derivation,
  the "code that no longer exists" wording, the superseded amendment, the
  Blocks entry, the 0164 relation, the pick-up check and the Summary's
  deliverables were all addressed.

### New Issues Introduced

- 🟡 **Clarity**: The ordering Requirement's justification contradicts Context
  and the criteria. It says the seam is untrustworthy "until the second probe
  call site is gone", but Context says `cache_root::resolve` has no production
  caller, and every criterion specifies a delta unconditionally rather than as
  an interim measure. The real mechanism — that `cache_root::resolve`'s own unit
  tests can probe in a shared test process — is never named.
- 🟡 **Testability**: The process-wide counter delta has no isolation
  precondition. Nothing states that no other probe may be in flight in the same
  process while a delta is captured, and Requirement 3 notes the
  concurrent-first-use tests deliberately probe from several threads. A delta
  of 2 would be ambiguous between a regression and cross-test interference.
- 🟡 **Dependency**: The delegated latency proof is routed to a closed work
  item. 0169 is `status: done` with its Phase 10 criterion unchecked and every
  validation figure pending, so no open item owns the measurement and nothing
  will surface it when the epic-0136 release cut happens.

Two further consequences of the revision, both minor:

- 🔵 **Dependency**: Removing the 2026-08-06 amendment falsifies 0169's Phase 10
  hand-off record, which states as complete that dated amendments were
  grep-verified onto 0125, 0172, 0183 and 0189.
- 🔵 **Clarity**: Bare `resolve` is ambiguous between `cache_root::resolve` and
  `FetchVerifyCacheResolver::resolve`, most consequentially in "main.rs calls
  `candidate`, not `resolve`" — the sentence carrying the item's central premise.

Recurring across lenses: AC5 lacks the fixture precondition its siblings carry
(how the second resolution stays a cold miss), and AC6/AC7 require evidence to
be "recorded" with no named destination.

### Assessment

The item is close. Nothing outstanding challenges its scope, its structure or
its value — the three majors are a muddled justification, a missing isolation
sentence, and an ownership gap for work that sits outside this item entirely.
The first two are edits to 0189. The third is a decision about where the epic's
latency verification lives now that 0169 is closed, which cannot be settled
inside this work item.

### Disposition — 2026-08-11

All pass-2 findings were applied to 0189 after this pass, and **not
re-reviewed** — the verdict above stands unverified against the current text.
The ownership question was resolved by the author bringing the latency
measurement back into 0189, which now inherits 0169's `G ≤ 1.1 × B` gate, names
0191 as a co-requisite, and records that the item cannot close before the
epic-0136 release cut. The stale "dominant unaddressed cost" prose in 0169 and
0186 was retracted in place the same day, as dated notes beside the original
text.
