---
type: work-item-review
id: "0120-prevention-tests-for-agent-invocation-path-review-1"
title: "Work Item Review: Prevention Tests for the Agent-Invocation Path"
date: "2026-06-20T12:45:43+00:00"
author: "Toby Clemson"
producer: review-work-item
status: complete
target: "work-item:0120"
work_item_id: "0120"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 3
tags: [migrate, interactive-migration, agent-invocation, tooling]
last_updated: "2026-06-20T13:48:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Prevention Tests for the Agent-Invocation Path

**Verdict:** REVISE

0120 is a structurally sound, well-bounded prevention task: every expected
section is present and substantive, the two-test scope is consistent across
Summary/Requirements/Acceptance Criteria, and both upstream blockers (0116,
0118) are named with their rationale. The blocker to approval is testability —
the acceptance criteria lean on the undefined observable "structured deferral"
and on a "check" whose form, artefact, and triggering input are all left open,
so a verifier cannot decide pass/fail and could satisfy the wording with a test
that proves nothing (exactly the failure mode the research warns about). With
the criteria pinned to concrete observables and a triggering fixture, this is a
clean, narrowly-scoped task.

### Cross-Cutting Themes

- **Acceptance criteria describe relationships, not observable pass/fail
  conditions** (flagged by: testability, scope, completeness) — AC1's "structured
  deferral" and AC2's "a check prevents it" name *what relationship to check*
  but not the concrete signal a verifier inspects (exit code, deferral marker,
  emitted resume command) nor the fixture that triggers it. The third AC partly
  restates earlier content. This is the dominant issue and the reason for the
  REVISE.
- **The unresolved Open Question leaks into scope and verifiability** (flagged
  by: scope, testability) — where the cross-check lives (0007 suite vs. shared
  helper vs. standalone lint over all backfill/validator pairs) is undecided;
  the "standalone lint" option would widen scope beyond the stated M, and the
  open form means AC2 has no fixed artefact to verify.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: 'Structured deferral' has no defined observable, so pass/fail is undecidable here
  **Location**: Acceptance Criteria
  AC1 requires the test to assert "the structured deferral," but neither AC1 nor
  the Requirements define what that concretely looks like (exit code, deferral
  marker, listed pending keys, emitted resume command). A test that merely checks
  "did not complete via decisions file" would satisfy the wording while proving
  nothing — recreating the class of test that "proved the wrong thing."

- 🟡 **Testability**: AC2 'a check prevents it' does not specify the verifiable artefact or its inputs
  **Location**: Acceptance Criteria
  AC2 does not state what the check is (unit test? lint over backfill/validator
  pairs? assertion in the 0007 suite?) nor what input it runs against (which
  corpus/fixture, which tolerated extra). The Open Question explicitly leaves the
  check's location undecided, so the check could pass vacuously and the
  regression still reappear.

#### Minor

- 🔵 **Testability**: AC3 ('without requiring a real TTY') risks being argued as always met
  **Location**: Acceptance Criteria
  A green CI run satisfies AC3 regardless of whether the no-TTY condition was
  actually exercised, so it provides little independent verification value beyond
  AC1's precondition.

- 🔵 **Testability**: Cross-validation requirement lacks a defined input that triggers the failure
  **Location**: Requirements
  The second Requirement describes the relationship to check but not the input
  that exercises it. The research identifies the exact trigger (a required extra
  like `pr_number` with no derivable default, e.g. a tracker-key-named filename),
  but that fixture is not carried into the requirement or criteria.

- 🔵 **Clarity**: Inconsistent label for the 0116 behaviour: 'deferral' vs 'stall'
  **Location**: Summary
  The behaviour is the "structured deferral" in Summary/Requirements/AC but the
  "structured stall" in Technical Notes; the research issue uses both. A reader
  cannot be certain the two phrases name the same 0116 outcome.

- 🔵 **Dependency**: Upstream blocker 0116 does not reciprocally name 0120 as a downstream consumer
  **Location**: Dependencies
  0120 declares "Blocked by: 0116", but 0116's own Dependencies records "Blocks:
  none" — unlike 0118, which correctly lists "Blocks: 0120". Closing 0116 will
  not visibly signal that 0120 is unblocked the way closing 0118 does.

#### Suggestions

- 🔵 **Clarity**: Dense domain phrases used without a definition or link
  **Location**: Requirements
  Terms like "tolerated state", "DIVERGE", and "ambiguous-band linkage
  transformation" appear without an in-item gloss, grounded only implicitly via
  file:line citations and the linked research issue.

- 🔵 **Completeness**: Third acceptance criterion partly restates Requirements/Assumptions
  **Location**: Acceptance Criteria
  The third AC ("execute in the existing migrate test suites without requiring a
  real TTY") largely re-expresses Requirements and Assumptions content rather
  than adding a distinct done-condition.

- 🔵 **Scope**: Two independently-deliverable test additions bundled in one task
  **Location**: Requirements
  The no-input deferral test (depends on 0116) and the backfill-vs-validator
  cross-check (depends on 0118) exercise unrelated subsystems with disjoint
  blockers, so either could ship as its dependency lands. Acceptable given both
  are small, but worth a note or a split.

- 🔵 **Scope**: Open Question's 'standalone lint' option could widen scope beyond M
  **Location**: Open Questions
  The "standalone lint over backfill/validator pairs" option would generalise the
  check across every migration, materially exceeding the two-test, single-failure-
  class boundary the Summary establishes.

- 🔵 **Dependency**: 0117 (invoker-contract sibling) absent from the relates_to set
  **Location**: Frontmatter: relates_to
  The research lists four prevention items; 0120 implements two and the other two
  map to sibling 0117, which 0120's `relates_to` ([0116, 0118]) does not name —
  a traceability gap, not a scheduling blocker.

### Strengths

- ✅ All expected sections for a task are present and substantively populated
  (Summary, Context, Requirements, Acceptance Criteria, Open Questions,
  Dependencies, Assumptions, Technical Notes, Drafting Notes, References), with
  complete, coherent frontmatter.
- ✅ Summary, Requirements, and Acceptance Criteria describe the same two pieces
  of work with no scope contradiction; the Requirements actively guard against
  creep by stating what is *not* in scope (the decisions-file path).
- ✅ Technical Notes anchor every domain term and behaviour to a concrete
  file:line citation (`read_decision` at `interactive-lib.sh:262`, backfill at
  `0007:507-510`, validator at `validate-corpus-frontmatter.sh:345`), giving an
  implementer concrete starting points.
- ✅ Both real upstream blockers (0116, 0118) are named with explicit rationale,
  the 0118 coupling is reciprocally consistent, and the ordering ("sequenced
  after 0116 and 0118") is stated redundantly and correctly.
- ✅ The Assumptions section names a concrete, reproducible mechanism for the
  no-input precondition (closing fd 0 / redirecting from `/dev/null`,
  `ACCELERATOR_MIGRATE_DECISIONS_FILE` unset, no pseudo-TTY).

### Recommended Changes

1. **Pin AC1 to a concrete deferral observable** (addresses: "'Structured
   deferral' has no defined observable", "Inconsistent label deferral vs stall")
   Rewrite AC1 to name the exact signal the test must assert against the 0116
   contract — e.g. "asserts a non-zero deferral exit code AND that the output
   names the pending decision keys and the exact resume command." Settle on one
   term (deferral or stall) matching 0116 and use it everywhere.

2. **Make AC2 fixture- and artefact-specific** (addresses: "AC2 'a check
   prevents it' does not specify the verifiable artefact or its inputs",
   "Cross-validation requirement lacks a defined input")
   Rephrase around a concrete fixture and observable — e.g. "Given a corpus
   containing a tracker-key-named PR file whose `pr_number` cannot be derived,
   when 0007 runs end-to-end, then it completes without a MISSING-EXTRA
   hard-fail." State the minimal triggering fixture in the Requirement too.

3. **Resolve the Open Question before implementation** (addresses: "Open
   Question's 'standalone lint' option could widen scope", and unblocks
   change #2)
   Decide where the cross-check lives. If the generalised-lint route is chosen,
   carve it out as a separate work item so 0120 stays bounded to the two targeted
   tests.

4. **Tighten or fold AC3** (addresses: "AC3 risks being argued as always met",
   "Third acceptance criterion partly restates Requirements/Assumptions")
   Tie it to a positive, inspectable signal (e.g. "the test explicitly closes
   fd 0 / redirects from `/dev/null` per Assumptions and the suite passes in
   headless CI") or fold it into AC1/AC2.

5. **Tidy the dependency/traceability records** (addresses: "0116 does not
   reciprocally name 0120", "0117 absent from relates_to")
   Add 0120 to 0116's "Blocks" entry (or downgrade 0120's dependency on 0116 to
   "Relates to" if it is ordering-only), and add `work-item:0117` to `relates_to`
   to record the sibling carrying the invoker-contract prevention items.

6. **Optionally gloss dense terms / note the intentional bundling** (addresses:
   "Dense domain phrases", "Two independently-deliverable test additions bundled")
   Add a one-line gloss for "tolerated state"/"DIVERGE", and a Drafting Notes
   line stating the two tests are intentionally kept together for cohesion (or
   split them into two children of 0115).

## Per-Lens Results

### Clarity

**Summary**: The work item communicates its two-test scope clearly and
consistently across Summary, Requirements, and Acceptance Criteria, with
referents grounded in 0115/0116/0118 and the research issue, and Technical Notes
anchoring every domain term to a specific file:line. The main clarity wrinkles
are a quietly shifting label for the 0116 behaviour ('structured deferral' vs
'structured stall') and a couple of dense, undefined domain phrases ('tolerated
state', 'ambiguous-band linkage', 'DIVERGE') that a reader who has not read the
research issue would have to take on faith.

**Strengths**:
- Summary, Requirements, and Acceptance Criteria all describe the same two pieces
  of work with no scope contradiction between sections.
- Technical Notes anchor every potentially opaque term to a concrete file:line
  citation, giving each piece of jargon an unambiguous referent.
- Actors in the Acceptance Criteria are explicitly named (the test suite, CI, the
  migration's own validation).

**Findings**:
- 🔵 **minor** (high) — *Inconsistent label for the 0116 behaviour: 'deferral'
  vs 'stall'* (Summary): The behaviour is the "structured deferral" in Summary/
  Requirements/AC but the "structured stall" in Technical Notes; the research
  uses both. A reader cannot be certain the two phrases name the same behaviour.
  Suggestion: pick one term and use it consistently, or state once that they are
  synonymous.
- 🔵 **suggestion** (medium) — *Dense domain phrases used without a definition
  or link* (Requirements): "tolerant backfill leaves absent",
  "hard-fail-on-tolerated-state", "DIVERGE", "ambiguous-band linkage
  transformation" appear without an in-item gloss. Suggestion: add a one-line
  gloss for "tolerated state"/"DIVERGE" or explicitly defer to the research-issue
  link for these terms.

### Completeness

**Summary**: This task work item is structurally complete and well-populated for
its kind: every expected section is present and substantive, frontmatter
integrity is sound, and the Summary, Requirements, and Acceptance Criteria all
clearly define the two-part work. Context, Dependencies, Assumptions, Open
Questions, and Technical Notes all carry real content drawn from the source
research. The only minor gap is that one of the three acceptance criteria
restates a requirement rather than adding a distinct done-condition.

**Strengths**:
- All expected sections for a task are present and substantively populated.
- Frontmatter is complete and coherent (kind, status, priority, parent,
  relates_to all present and valid).
- Context explains *why* the work is needed and traces each gap to a concrete
  prior failure, rather than restating the Summary.
- Requirements and Technical Notes give concrete file/line starting points.
- Dependencies and Assumptions carry genuinely relevant content.

**Findings**:
- 🔵 **suggestion** (medium) — *Third acceptance criterion partly restates
  Requirements/Assumptions without adding a distinct done-condition* (Acceptance
  Criteria): The third AC re-expresses content already stated in Requirements and
  Assumptions. Suggestion: sharpen it into a distinct done-condition (e.g. both
  tests wired into a named existing suite and run on the standard CI matrix) or
  fold it into the first two.

### Dependency

**Summary**: 0120 captures its two genuine upstream blockers (0116 for the
structured deferral, 0118 for the backfill/validator invariant), and these are
reciprocally consistent and well-sequenced — 0118 independently names 0120 in
its Blocks field, and the ordering rationale is stated in Dependencies, Technical
Notes, and Drafting Notes. The coupling map is essentially complete with no
hidden upstream blocker. The only gaps are interpretive: a non-reciprocal Blocks
entry on the 0116 side and the absence of 0117 from relates_to.

**Strengths**:
- Both real upstream blockers (0116, 0118) are explicitly named with rationale
  for what each delivers.
- The 0118 coupling is reciprocally consistent (0118 lists "Blocks: 0120").
- Ordering constraints captured redundantly and correctly across three sections.
- "Blocks: none" is accurate — this is leaf prevention work.

**Findings**:
- 🔵 **minor** (medium) — *Upstream blocker 0116 does not reciprocally name 0120
  as a downstream consumer* (Dependencies): 0116's own Dependencies records
  "Blocks: none" and does not name 0120, unlike 0118. Closing 0116 will not
  visibly signal 0120 is unblocked. Suggestion: add 0120 to 0116's Blocks, or
  downgrade 0120's dependency on 0116 to "Relates to" if it is ordering-only.
- 🔵 **suggestion** (low) — *0117 (invoker-contract sibling) absent from the
  relates_to coupling set* (Frontmatter: relates_to): The other two prevention
  items map to sibling 0117, which 0120's relates_to does not name. Traceability
  gap, not a scheduling blocker. Suggestion: add work-item:0117 to relates_to and
  a one-line "Relates to: 0117" note.

### Scope

**Summary**: 0120 is a well-bounded prevention task that gathers the two
test-coverage gaps the research identifies. Both items share a single unifying
purpose (regression prevention for this failure class) and the task is correctly
sequenced after the behaviour-introducing siblings (0116, 0118). The two test
concerns are arguably independently deliverable, but their cohesion as "the
prevention work of 0115" and the M sizing keep this within reasonable bounds for
a single task.

**Strengths**:
- Clear, stated scope boundary that explicitly excludes the behaviour changes
  themselves (deferred to 0116 and 0118).
- Coherent unifying purpose — both requirements trace to the research's
  "Prevention" section.
- Sizing rationale made explicit (M — two new test classes); kind (task, child of
  epic 0115) is appropriate.
- Requirements actively guard against scope creep (excludes the
  protocol-via-decisions-file path).

**Findings**:
- 🔵 **suggestion** (medium) — *Two independently-deliverable test additions
  bundled in one task* (Requirements): The two tests exercise unrelated
  subsystems with disjoint blocking dependencies, so either could ship as its
  dependency lands. Acceptable as-is given both are small; consider splitting
  into two children of 0115 or noting the intentional bundling in Drafting Notes.
- 🔵 **suggestion** (low) — *Open Question's 'standalone lint' option could widen
  scope beyond M* (Open Questions): The generalised-lint option would affect
  every migration, exceeding the stated sizing. Suggestion: resolve before
  implementation and, if the lint route is preferred, carve it out as a separate
  work item.

### Testability

**Summary**: This task's acceptance criteria are reasonably concrete because the
deliverable is itself a pair of tests, and they correctly name the discriminating
condition (no TTY, no decisions file, structured deferral rather than
decisions-file completion). However, the central observable — what counts as a
"structured deferral" — is never defined to a pass/fail threshold here, and the
second criterion's "a check prevents it" is verb-vague about what artefact a
verifier inspects. A verifier would also lack a defined corpus/input fixture to
exercise the cross-validation against.

**Strengths**:
- AC1 specifies the discriminating precondition precisely (no TTY, no decisions
  file) and explicitly excludes the false-positive path.
- AC3 ties verifiability to a concrete environment constraint checkable in CI.
- Assumptions names a concrete mechanism for the no-input precondition (close
  fd 0 / redirect from /dev/null, decisions file unset).
- Technical Notes anchor verification to specific seams and line references.

**Findings**:
- 🟡 **major** (high) — *'Structured deferral' has no defined observable, so
  pass/fail is undecidable here* (Acceptance Criteria): Neither AC1 nor
  Requirements define what the deferral concretely looks like (exit code,
  deferral marker, listed pending keys, resume command). A test checking only
  "did not complete via decisions file" would satisfy the wording while proving
  nothing. Suggestion: name the concrete observable, referencing the 0116
  contract for exact tokens.
- 🟡 **major** (high) — *AC2 'a check prevents it' does not specify the
  verifiable artefact or its inputs* (Acceptance Criteria): AC2 states neither
  what the check is nor what input it runs against; the Open Question leaves its
  location undecided. The check could pass vacuously. Suggestion: rephrase around
  a concrete fixture and observable (tracker-key-named PR file whose pr_number
  cannot be derived → 0007 completes without a MISSING-EXTRA hard-fail).
- 🔵 **minor** (medium) — *AC3 ('without requiring a real TTY') risks being
  argued as always met* (Acceptance Criteria): A green CI run satisfies it
  regardless of whether the no-TTY condition was exercised. Suggestion: tie to a
  positive, inspectable signal (explicitly closes fd 0 / redirects from
  /dev/null, suite passes headless).
- 🔵 **minor** (medium) — *Cross-validation requirement lacks a defined input
  that triggers the failure* (Requirements): The requirement describes the
  relationship to check but not the triggering input. Suggestion: state the
  minimal fixture (a corpus document carrying a required extra with no derivable
  default) drawn from the H2 description in the research.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-06-20

**Verdict:** REVISE

The pass-1 edits cleanly resolved both original blocking majors and every
clarity/completeness/scope/dependency finding from pass 1. Completeness now
reports zero findings. The verdict remains REVISE because three *new* majors
surfaced — two are a natural consequence of the acceptance criteria now being
concrete enough to scrutinise at finer grain (one residual underspecified clause
in AC1, one missing baseline in AC3), and one is a genuine discovery pass 1
missed: a non-reciprocal dependency with sibling 0119. None reopen the resolved
issues; they are a tighter, smaller-surface set than pass 1.

### Previously Identified Issues

- 🟡 **Testability**: "Structured deferral" has no defined observable — **Resolved**. AC1 now asserts non-zero exit, the pending key, the three resume-command properties, and the absence of `failed to obtain decision`.
- 🟡 **Testability**: AC2 "a check prevents it" unspecified — **Resolved**. AC2 now names the tracker-key fixture, the `unknown` sentinel, and the two validator gates.
- 🔵 **Testability**: AC3 "always met" — **Partially resolved**. Now ties to closing fd 0 / `/dev/null`, but still lacks a baseline pass-count (see new finding).
- 🔵 **Testability**: Cross-validation requirement lacks a triggering input — **Resolved**. Fixture (`<TRACKER>-NNNN-description.md`) now named in Requirements + AC2.
- 🔵 **Clarity**: deferral/stall label inconsistency — **Resolved**. Standardised on "structured stall" with an inline gloss.
- 🔵 **Clarity**: dense domain phrases — **Partially resolved**. "tolerated state" glossed; "required type-extra"/"derivable" still defined only by example (see new finding).
- 🔵 **Completeness**: third AC restates earlier content — **Resolved**. AC3 is now a distinct headless-CI / no-input-precondition condition.
- 🔵 **Dependency**: 0116 does not reciprocally name 0120 — **Resolved**. 0116 now records `Blocks: 0120`; 0120's note updated to reflect the mirrored edge.
- 🔵 **Dependency**: 0117 absent from relates_to — **Resolved**. Added to frontmatter + Dependencies as a traceability link.
- 🔵 **Scope**: two tests bundled — **Resolved**. Drafting Notes now records the intentional bundling.
- 🔵 **Scope**: Open Question's standalone-lint option could widen scope — **Resolved**. Open Question resolved to the 0007 suite; lint option explicitly rejected/deferred.

### New Issues Introduced

- 🟡 **Dependency** (high): *Non-reciprocal dependency with 0119.* 0119 declares "Blocks: 0120's guarded-resume coverage" and lists 0120 in `relates_to`, but 0120 records no edge to 0119 and its scope (two tests) never mentions guarded resume. The two items disagree on whether 0120 owns guarded-resume test coverage. **Verified against 0119:129.** Needs a scope decision: either 0120 stays at two tests and 0119's "Blocks" is corrected to a relates-to, or 0120 expands to a third guarded-resume test blocked by 0119.
- 🟡 **Testability** (high): *AC1 clause (a) lacks a concrete match target.* "Names the current pending decision key" provides no fixture key/format to assert against, so a test could assert merely that *some* token is present.
- 🟡 **Testability** (medium): *AC3 has no baseline for "the suites pass".* No expected test count / named suites, so a silently-shrunk or skipped suite could be argued to satisfy it (the migrate suites are count-floored).
- 🔵 **Testability** (medium): AC2's elided `FAIL: … MISSING-EXTRA` and "completes its stages" lack an exact observable (exit code / regex).
- 🔵 **Testability** (medium): The no-input Assumption isn't positively asserted — a test could pass without traversing `read_decision`'s bare fd-0 branch (the stall signal from AC1 largely covers this).
- 🔵 **Dependency** (medium): The resume-command shape AC1 hard-asserts is owned by the pre-flight hint format (per 0116); a change via 0119 would silently stale the assertion, and 0120 doesn't note the coupling.
- 🔵 **Clarity** (medium): Heavy bare-ticket shorthand ("0116's stall", "0118 writes", "the 0007 suite") makes the item not fully self-interpretable without the siblings in hand.
- 🔵 **Clarity** (medium): AC1's three nested lettered sub-clauses are dense enough to admit more than one parse.
- 🔵 **Clarity** (low): `EMPTY-PLACEHOLDER` introduced in AC2 without a gloss; research's `pending` vs 0118's `unknown` not reconciled in-item.
- 🔵 **Scope** (minor): With disjoint blockers, completion is gated by the later of 0116/0118; consider noting the two ACs can merge incrementally as each blocker clears.

### Assessment

The work item improved substantially: the two criteria that made it un-verifiable
are now concrete, and the cross-document dependency hygiene is largely fixed. It
is close to ready. Remaining before APPROVE: (1) a scope decision reconciling the
0119 edge, (2) pinning AC1 clause (a) to the fixture's known key, and (3) giving
AC3 a baseline pass condition. The clarity/testability minors are polish that can
ride along. These are convergent refinements, not structural rework.

## Re-Review (Pass 3) — 2026-06-20

**Verdict:** COMMENT

The Pass 2 edits resolved all three majors. The verdict moves from REVISE to
COMMENT: only **one** major finding remains (below the configured REVISE
threshold of 2), and it is a precision judgment call, not a defect. The trend
across passes is cleanly convergent — REVISE (2 major) → REVISE (3 major) →
COMMENT (1 major). The work item is acceptable for implementation as-is; the
remaining items are polish.

### Previously Identified Issues (Pass 2)

- 🟡 **Dependency**: Non-reciprocal dependency with 0119 — **Resolved**. 0119's `Blocks: 0120` corrected to a relates-to; 0120 adds 0119 to `relates_to` with an out-of-scope note; the dependency lens confirms both blocking edges (0116, 0118) are reciprocal.
- 🟡 **Testability**: AC1 clause (a) lacked a concrete match target — **Resolved**. Now asserts the fixture's known decision key as a literal substring.
- 🟡 **Testability**: AC3 had no baseline for "the suites pass" — **Partially resolved**. AC3 now names both suites and requires a count "at or above its current floor", but the floor is not numerically pinned (see new finding).
- 🔵 **Testability**: AC2 elided observable — **Resolved**. Now "exits 0, no line matching `FAIL:.*MISSING-EXTRA`".
- 🔵 **Testability**: no-input branch not positively asserted — **Resolved** (but the new wording drew a minor inference-vs-observable finding; see below).
- 🔵 **Dependency**: resume-shape ownership coupling — **Resolved**. Now noted in 0120's Dependencies.
- 🔵 **Clarity**: dense domain phrases — **Resolved** ("required type-extra"/"derivable" now defined; though the added gloss drew a new "long parenthetical" minor).
- 🔵 **Clarity**: `EMPTY-PLACEHOLDER` gloss — **Resolved**. AC2 now explains why the sentinel clears both gates.

### New Issues Introduced

- 🟡 **Testability** (high): *AC3's test-count floor is referenced but never pinned.* "At or above its current floor" has no concrete integer or stated read-at-runtime mechanism, so the anti-shrink guard is itself indeterminate. Fix is cheap: either state per-suite minimums, or delegate explicitly to the floor the suite already asserts in CI.
- 🔵 **Testability** (medium): AC1's "positive evidence the bare fd-0 branch was traversed" conflates an external observable with an internal control-flow fact — reframe as a consequence, or add a branch-distinguishing marker.
- 🔵 **Testability** (medium): AC1(b) verifies the resume command's *shape* (substrings) but not that it is *operative*; clarify whether functional resumability is in scope or delegated.
- 🔵 **Clarity** (medium): the requirement's ~70-word inline definition parenthetical buries the X-vs-Y main clause; consider hoisting to a glossary/Technical Notes.
- 🔵 **Clarity** (medium): AC2's "the value the validator sees" leaves the producer (0118's backfill) implicit — risks faking the sentinel in the fixture.
- 🔵 **Clarity** (low): AC3's "each/both" mixes the two suites without scoping the no-input precondition to the interactive test only.
- 🔵 **Clarity** (low): `<TRACKER>` placeholder not explicitly tied to the prose term "external tracker key".
- 🔵 **Dependency** (minor): blocking edges (0116/0118) live only in prose; frontmatter encodes them undifferentiated in `relates_to` (schema limitation — no `blocked_by` field).
- 🔵 **Dependency** (minor): 0118 reciprocity not stated in-text the way 0116's is (both are in fact reciprocal); 0117 relates-to is one-directional in frontmatter (benign).
- 🔵 **Scope** (suggestion): the disjoint-blocker bundling persists; the lens explicitly says no change is required given both tests are small and single-purpose.

### Assessment

The work item is ready for implementation. The single remaining major (pin or
delegate AC3's test-count floor) is worth a one-line fix but is a precision
choice rather than a blocker — pinning a magic integer in a work item is itself
debatable, so delegating to the suite's existing CI-asserted floor is the
cleaner option. The clarity/testability minors are polish, several of them
introduced by the very edits that resolved the larger Pass 2 findings (denser
prose is the cost of the added precision). No structural rework remains.

## Approval — 2026-06-20

**Verdict:** APPROVE

The single remaining Pass 3 major (AC3's unpinned test-count floor) was resolved
after the re-review: AC3 now delegates the count comparison to the floor each
suite already asserts in CI — deterministic at run time, with no hard-coded
integer to drift. With that closed there are no critical or major findings
across any lens; only 🔵 clarity/testability polish remains, none of it blocking.
The work item is approved for implementation, and its status has been
transitioned draft → ready.
