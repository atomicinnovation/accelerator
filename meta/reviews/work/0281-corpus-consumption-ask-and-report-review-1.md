---
type: "work-item-review"
id: "0281-corpus-consumption-ask-and-report-review-1"
title: "Work Item Review: Corpus Consumption: Ask and Report"
date: "2026-10-23T07:11:15+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:0121"
target: "work-item:0281"
work_item_id: "0281"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 6
tags: ["research", "consumption"]
last_updated: "2026-10-23T13:42:14+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Corpus Consumption: Ask and Report

**Verdict:** REVISE

0281 is well structured. Every section is filled in, the gates are named
precisely, and each decision is traced through the Drafting Notes. Its weak
points are the transitions between steps: the status-gate recovery pointer
and the rejected-`report` flow are stated inconsistently or not at all. The
criteria for the coverage branches and the untrusted-data contract cannot be
checked against known inputs, and the 0347 hand-off leaves out the
completeness gate. The 5 major findings exceed the 2-major threshold, so the
verdict is REVISE, but no finding is critical.

### Cross-Cutting Themes

- **Rejecting an answer after `report` has written it** (flagged by:
  clarity, testability) — rejection is treated as `unanswered`, which writes
  no report. But `report` writes the file and the `report_count` edit before
  the user sees the answer, and nothing says whether those writes survive.
- **Status-gate recovery pointer** (flagged by: clarity, testability) — the
  Requirements point to `synthesise SLUG`, while the acceptance criterion says
  "the recovery path for the set's state". That criterion also omits "nothing
  is written".
- **Contract deliverables untested** (flagged by: completeness, testability)
  — the template, catalogue entry, injection, optional `report_count` and
  `argument-hint` have no acceptance criterion, including the
  backward-compatibility guarantee for existing manifests.
- **0347 boundary** (flagged by: scope, dependency) — the "whichever lands
  second" hand-off leaves 0281's scope dependent on landing order, and it
  covers only the proposal, not the completeness gate.

### Findings

#### Major

- 🟡 **Clarity / Testability**: Status-gate recovery path contradicts between
  Requirements and AC
  **Location**: Requirements: Shared — Status gate / Acceptance Criteria
  The Requirements point every pre-synthesis refusal to `synthesise SLUG`,
  but the acceptance criterion says it points to "the recovery path for the
  set's state". That criterion also does not assert that nothing is written.
- 🟡 **Clarity / Testability**: Rejecting an answer after `report` has
  written it is undefined
  **Location**: Requirements: Coverage / Acceptance Criteria
  A rejected answer is treated as `unanswered` ("writes no report"), yet the
  report and its `report_count` edit may already be on disk. The work item
  does not say whether they are kept or reverted, or when rejection can
  happen.
- 🟡 **Testability**: Coverage-dependent criteria have no defined fixture
  corpus
  **Location**: Acceptance Criteria
  "The corpus answers it", "partly answers" and "does not answer" are
  themselves model judgements. Without reference scenarios that fix the
  expected judgement, a wrong judgement looks the same as a correctly handled
  branch.
- 🟡 **Testability**: Untrusted-data contract has no acceptance criterion
  **Location**: Requirements: Shared
  The only security-relevant Shared requirement is unverified. An
  implementation that follows an instruction embedded in a finding would
  still pass every criterion.
- 🟡 **Dependency**: 0347 hand-off covers proposals but not the completeness
  gate
  **Location**: Dependencies
  0347 adds pending `redo`/`revisit`/`deepen` items to `outline.md`. The
  completeness gate checks only `(focus area, profile)` pairs, so a pending
  re-research item would pass the gate and break the "no round is pending"
  invariant.

#### Minor

- 🔵 **Completeness / Testability**: Contract and template wiring
  requirements have no criteria
  **Location**: Requirements: Contract / Acceptance Criteria
  Only the `coverage` vocabulary check has a criterion. Nothing verifies that
  an existing manifest without `report_count` still validates.
- 🔵 **Scope**: Story bundles a conversational verb with an artefact-writing
  verb and its contract work
  **Location**: Requirements
  `ask` with the shared gates and the proposal could ship independently, and
  `report` with its contract is a natural follow-on.
- 🔵 **Scope**: Scope boundary with 0347 depends on landing order
  **Location**: Dependencies
  The `revisit`/`deepen` proposal extension has no requirement or criterion
  here, and it risks being dropped by both items.
- 🔵 **Dependency**: Uncaptured coupling to 0306's cross-finding tier-merge
  rule
  **Location**: Requirements: report
  0306's `tier-3 (also tier-2)` merge rule would apply to report Sources, but
  neither item references the other.
- 🔵 **Dependency**: 0280 and 0282 are prerequisites but recorded as Related
  or not at all
  **Location**: Dependencies
  Per-profile findings, the tier suffixes and `None found.` come from 0280,
  and the breadth ceiling comes from 0282.
- 🔵 **Dependency**: Reverse edges on 0278 and 0279 disagree with 0281's
  record
  **Location**: Frontmatter: blocked_by
  0278's `blocks` still lists 0281, and 0279's `blocks` omits it.
- 🔵 **Clarity**: `reports/<slug>.md` reads as the topic `SLUG`
  **Location**: Summary
  The report's slug is derived from the question, which only becomes clear
  later in the Requirements.
- 🔵 **Clarity**: `conduct` / `synthesise SLUG` recovery pointer is ambiguous
  **Location**: Requirements: Shared — Freshness gate
  It does not say whether both commands are always named, or one per type of
  mismatch.
- 🔵 **Clarity**: A "synthesised set with a pending round" seems to
  contradict the reopen lifecycle
  **Location**: Acceptance Criteria
  Appending a round reopens the set, so it is unclear how a `synthesised` set
  comes to have a pending round.
- 🔵 **Clarity**: Coverage judgement for a `None found.` profile is unstated
  **Location**: Acceptance Criteria
  When a question depends on scholarly evidence that turned out not to exist,
  it is unclear whether the result is `answered` or `partial`.
- 🔵 **Clarity**: Which question a rewritten report answers is unclear
  **Location**: Requirements: report
  It could be the stored original question or the new wording.
- 🔵 **Clarity**: Report body sections differ from the parent epic
  **Location**: Technical Notes
  0121 lists "Question, Report, Sources" with no `coverage` extra, and no
  Drafting Note records the change.
- 🔵 **Testability**: "Both inform the answer" is not observable
  **Location**: Acceptance Criteria
  It needs an observable marker, such as Sources from each profile's finding.
- 🔵 **Testability**: Proposal constraints and next-step naming have no
  criteria
  **Location**: Requirements: Coverage
  No criterion checks that proposed profiles come from `source_profiles`,
  that the proposal stays within the breadth ceiling, or that the next steps
  are named.
- 🔵 **Testability**: `complete` happy path and reopen-from-`synthesised` are
  not exercised
  **Location**: Acceptance Criteria
  Every answer criterion uses a `synthesised` set, and the reopen criterion
  covers only `complete`.

#### Suggestions

- 🔵 **Completeness**: Decision deferred to planning is not recorded as an
  open question
  **Location**: Technical Notes
  Whether to use an existing subagent or a custom one is left to planning but
  sits only in Technical Notes.
- 🔵 **Scope**: Generic per-extra vocabulary validator mechanism is a
  separable infrastructure seam
  **Location**: Technical Notes
  Keep it to the minimum `report` needs, or carve it into an enabling task.
- 🔵 **Testability**: Ordering and `.levels/` criteria need a stated
  observation method
  **Location**: Acceptance Criteria
  Final disk state alone cannot show the ordering or prove that no level note
  was read.
- 🔵 **Testability**: Slug derivation and numeric suffix format are
  unspecified
  **Location**: Requirements: report
  The collision criterion cannot assert the filename or that `report_count`
  becomes 2.
- 🔵 **Clarity**: Several project terms used without definition or link
  **Location**: Context / Requirements / Dependencies
  Undefined terms include "reference systems", "stragglers", "Shared
  Preamble", "breadth ceiling" and "answer-not-found proposal".
- 🔵 **Clarity**: Report `status: complete` shares its name with set status
  `complete`
  **Location**: Requirements: Contract
  The work item should state that a report's status is independent of the
  set's lifecycle.

### Strengths

- ✅ Each gate in the shared precondition is named (Status, Freshness,
  Completeness) and has its own refusal message and recovery verb. Most
  refusal criteria also assert that nothing is written.
- ✅ The coverage vocabulary is consistent across the Requirements, the
  Acceptance Criteria and the validator rule `coverage ∈ {answered, partial}`.
- ✅ Requirements are grouped by concern (Shared, ask, report, Coverage,
  Contract) and name the actor for each action.
- ✅ The report-writing criterion pins exact values (`rounds_covered: 3`,
  `report_count` 1, `status` and `primary` unchanged).
- ✅ Boundaries are drawn deliberately. Visualiser rendering moved to 0284,
  finding re-research split into 0347, and the `.levels/` and no-`conduct`
  exclusions are stated outright.
- ✅ Blockers (0277, 0279) are named with what each provides and are both
  done. The 0284 Block entry states the interim consequence.
- ✅ The Drafting Notes trace every departure from the earlier draft and the
  reasoning behind the append-only and no-empty-report decisions.

### Recommended Changes

1. **Define the rejected-`report` flow** (addresses: Rejecting an answer
   after `report` has written it is undefined)
   State when the user can reject an answer and whether the written report
   and its `report_count` edit are kept, reverted or rewritten as partial.
   Split the acceptance criterion by verb and state the expected disk state
   for each.
2. **Reconcile the status-gate recovery pointer** (addresses: Status-gate
   recovery path contradicts between Requirements and AC; `conduct` /
   `synthesise SLUG` recovery pointer is ambiguous)
   Use one rule in both the Requirements and the criterion: a fixed
   `synthesise SLUG`, or a mapping from each state to its recovery command.
   Add "nothing is written" to the criterion. Apply the same treatment to the
   freshness gate's two mismatch types.
3. **Widen the 0347 hand-off and give it one owner** (addresses: 0347
   hand-off covers proposals but not the completeness gate; Scope boundary
   with 0347 depends on landing order)
   Give one owner the job of extending both the proposal and the completeness
   gate to cover pending operation items. That owner should be 0347 or a
   small follow-up item. Then mirror the change in 0347.
4. **Pin the coverage branches to reference scenarios** (addresses: Coverage-
   dependent criteria have no defined fixture corpus; Coverage judgement for
   a `None found.` profile is unstated)
   Describe a fixture corpus with one answered question, one unanswered and
   one partially answered, together with the expected judgement for each.
   Include the `None found.` case.
5. **Add the missing acceptance criteria** (addresses: Untrusted-data
   contract has no acceptance criterion; Contract and template wiring
   requirements have no criteria; Proposal constraints and next-step naming
   have no criteria; `complete` happy path and reopen-from-`synthesised` are
   not exercised; "Both inform the answer" is not observable)
   Cover the following:
   - an injected instruction in a finding is not acted on
   - an existing manifest without `report_count` validates
   - a user-tier override of the report template is used
   - `argument-hint` lists both verbs
   - proposed profiles come from `source_profiles`, stay within the breadth
     ceiling, and the next steps are named
   - a `complete` set is answered, and a `synthesised` set is reopened
   - Sources draw on each profile's finding
6. **Tidy the dependency record** (addresses: 0280 and 0282 are
   prerequisites but recorded as Related or not at all; Reverse edges on 0278
   and 0279 disagree with 0281's record; Uncaptured coupling to 0306's
   cross-finding tier-merge rule)
   Move 0280 to Blocked by and add 0282, both marked as done. Correct the
   `blocks` lists on 0278 and 0279. Add 0306 as Related, with an owner for
   extending the merge rule to `report`.
7. **Resolve the smaller clarity points** (addresses: `reports/<slug>.md`
   reads as the topic `SLUG`; A "synthesised set with a pending round" seems
   to contradict the reopen lifecycle; Which question a rewritten report
   answers is unclear; Report body sections differ from the parent epic;
   Several project terms used without definition or link)
   - Rename the report filename placeholder.
   - Limit the pending-round case to stragglers, or explain how a
     `synthesised` set can have a pending round.
   - State which question a rewrite answers.
   - Record the change of body layout in the Drafting Notes.
   - Define or link the undefined terms.
8. **Optionally, record the split decision** (addresses: Story bundles a
   conversational verb with an artefact-writing verb and its contract work;
   Generic per-extra vocabulary validator mechanism is a separable
   infrastructure seam)
   Either split `ask` from `report`, or add a Drafting Note on why they ship
   together as one epic slice.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is mostly precise: it names its gates, verbs,
files and fields exactly, and the Drafting Notes explain the reasoning behind
each decision. The main clarity problems are where sections disagree or
leave a gap: the status-gate recovery path, what happens to a `report` the
user rejects after it is written, and the use of `<slug>` next to `SLUG`.
A handful of project terms are used without a definition or a link.

**Strengths**:
- Each gate is named and has its own refusal message and recovery path.
- The coverage vocabulary is consistent across Requirements, Acceptance
  Criteria and the validator rule.
- Requirements use active voice and name the actor throughout.
- The Drafting Notes record terminology changes from the earlier draft.

**Findings**:
- 🟡 major (high) — Requirements: Shared — Status gate / Acceptance
  Criteria — Status-gate recovery path contradicts between Requirements and
  AC.
- 🟡 major (medium) — Requirements: Coverage — Rejecting an answer after
  `report` has written it is undefined.
- 🔵 minor (high) — Summary — `reports/<slug>.md` reads as the topic `SLUG`.
- 🔵 minor (medium) — Requirements: Shared — Freshness gate — `conduct` /
  `synthesise SLUG` recovery pointer is ambiguous.
- 🔵 minor (medium) — Acceptance Criteria — A "synthesised set with a
  pending round" seems to contradict the reopen lifecycle.
- 🔵 minor (medium) — Acceptance Criteria — Coverage judgement for a
  `None found.` profile is unstated.
- 🔵 minor (medium) — Requirements: report — Which question a rewritten
  report answers is unclear.
- 🔵 minor (medium) — Technical Notes — Report body sections differ from the
  parent epic.
- 🔵 suggestion (medium) — Context / Requirements / Dependencies — Several
  project terms used without definition or link.
- 🔵 suggestion (low) — Requirements: Contract — Report `status: complete`
  shares its name with set status `complete`.

### Completeness

**Summary**: A well-populated story. Every expected section is present and
substantive, and the frontmatter is complete. The remaining gaps are that
the Contract requirements have no acceptance criteria, and there is no Open
Questions section for the choice deferred to planning.

**Strengths**:
- The Summary names the user, the capability and the benefit, then
  describes each verb concretely.
- Context explains why the work is needed and why the verbs differ in
  durability.
- The 17 Given/When/Then criteria cover the gates, coverage outcomes,
  rewriting, collisions, tiers, `.levels/` and ordering.
- The frontmatter is complete and consistent with the body header.
- Assumptions and Drafting Notes record the deliberate trade-offs.

**Findings**:
- 🔵 minor (medium) — Acceptance Criteria — Contract requirements have no
  acceptance criteria.
- 🔵 suggestion (medium) — Technical Notes — Decision deferred to planning is
  not recorded as an open question.

### Dependency

**Summary**: The dependency map is strong. Upstream blockers and `finalise`'s
freshness gate are named and done, and 0284 is recorded with its interim
consequence. The gaps are at the edges: 0280 and 0282 are under-recorded,
the 0347 hand-off misses the completeness gate, 0306's tier-merge rule is not
linked, and some reverse edges on sibling items disagree.

**Strengths**:
- Each blocker entry states what it provides and that it is done.
- The 0284 Block entry states the interim visibility consequence, and the
  removal of the 0278 blocker is explained.
- The 0347 coupling is captured as a symmetric "whichever lands second"
  obligation.
- Reuse of `finalise`'s freshness gate is recorded as a dependency.

**Findings**:
- 🟡 major (medium) — Dependencies — 0347 hand-off covers proposals but not
  the completeness gate.
- 🔵 minor (medium) — Requirements — Uncaptured coupling to 0306's
  cross-finding tier-merge rule.
- 🔵 minor (high) — Dependencies — 0280 and 0282 are prerequisites but
  recorded as Related or not at all.
- 🔵 minor (high) — Frontmatter: blocked_by — Reverse edges on 0278 and 0279
  disagree with 0281's dependency record.

### Scope

**Summary**: One coherent capability that maps directly onto 0121's Slice 4,
with its boundaries to 0284, 0283 and 0347 mostly well drawn. The concerns
are its size as a single story and a 0347 boundary that changes with landing
order.

**Strengths**:
- Visualiser rendering and finding re-research were deliberately split off,
  and the Drafting Notes record why.
- The Summary, Requirements and Acceptance Criteria describe the same scope.
- Both blockers are done.
- The out-of-scope items are stated explicitly.

**Findings**:
- 🔵 minor (medium) — Requirements — Story bundles a conversational verb
  with an artefact-writing verb and its contract work.
- 🔵 minor (high) — Dependencies — Scope boundary with 0347 depends on
  landing order.
- 🔵 suggestion (medium) — Technical Notes — Generic per-extra vocabulary
  validator mechanism is a separable infrastructure seam.

### Testability

**Summary**: Strongly testable. Most criteria pin concrete preconditions and
observable outcomes. The weaknesses are:
- the coverage-dependent criteria have no fixture corpus;
- the expected disk state after a rejected `report` is undefined;
- several requirements have no criterion, among them the untrusted-data
  contract, the proposal constraints, the contract wiring and the `complete`
  happy path.

**Strengths**:
- Refusal criteria state the trigger, the message content and the recovery
  verb, and most assert that nothing is written.
- The report-writing criterion pins exact frontmatter values and fields that
  must not change.
- Negative and boundary cases are explicit.
- The ordering criterion makes the sequencing invariant explicit.

**Findings**:
- 🟡 major (medium) — Acceptance Criteria — Coverage-dependent criteria have
  no defined fixture corpus.
- 🟡 major (high) — Acceptance Criteria — Rejected-answer criterion leaves
  the expected report file state undefined.
- 🟡 major (high) — Requirements: Shared — Untrusted-data contract has no
  acceptance criterion.
- 🔵 minor (high) — Acceptance Criteria — "Both inform the answer" is not
  observable.
- 🔵 minor (high) — Requirements: Coverage — Proposal constraints and
  next-step naming have no criteria.
- 🔵 minor (high) — Requirements: Contract — Contract and template wiring
  requirements have no criteria.
- 🔵 minor (medium) — Acceptance Criteria — Status-gate criterion's recovery
  path is not stated, and it omits the no-write assertion.
- 🔵 minor (medium) — Acceptance Criteria — `complete` happy path and
  reopen-from-`synthesised` are not exercised.
- 🔵 suggestion (medium) — Acceptance Criteria — Ordering and `.levels/`
  criteria need a stated observation method.
- 🔵 suggestion (medium) — Requirements: report — Slug derivation and
  numeric suffix format are unspecified.

## Re-Review (Pass 2) — 2026-10-23T12:56:29+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity / Testability**: Status-gate recovery path contradicts between
  Requirements and AC — Resolved. Both now point to `synthesise SLUG` and
  assert no writes. Clarity still notes that `briefed`/`outlined` sets are
  not recovered by `synthesise`; this was accepted deliberately to mirror
  `finalise`.
- 🟡 **Clarity / Testability**: Rejecting an answer after `report` has
  written it is undefined — Resolved
- 🟡 **Testability**: Coverage-dependent criteria have no defined fixture
  corpus — Partially resolved. The reference corpus exists in Technical
  Notes, but the criteria do not cite it.
- 🟡 **Testability**: Untrusted-data contract has no acceptance criterion —
  Resolved
- 🟡 **Dependency**: 0347 hand-off covers proposals but not the completeness
  gate — Resolved
- 🔵 **Completeness / Testability**: Contract and template wiring
  requirements have no criteria — Resolved
- 🔵 **Scope**: Story bundles a conversational verb with an artefact-writing
  verb — Resolved by decision. One unit, recorded in the Drafting Notes.
- 🔵 **Scope**: Scope boundary with 0347 depends on landing order — Resolved
- 🔵 **Dependency**: Uncaptured coupling to 0306 — Partially resolved. It is
  now linked, but see the new major findings below.
- 🔵 **Dependency**: 0280 and 0282 recorded as Related or not at all —
  Resolved
- 🔵 **Dependency**: Reverse edges on 0278 and 0279 — Partially resolved.
  The frontmatter is fixed, but the prose in 0278's Dependencies still says
  it blocks 0281.
- 🔵 **Clarity**: `reports/<slug>.md` reads as the topic `SLUG` — Resolved
- 🔵 **Clarity**: Freshness recovery pointer ambiguous — Resolved
- 🔵 **Clarity**: Synthesised set with a pending round — Resolved
- 🔵 **Clarity**: Coverage judgement for `None found.` — Resolved, by the
  tier-based coverage rule
- 🔵 **Clarity**: Which question a rewritten report answers — Partially
  resolved. It is now unclear whether the answer the user accepted is the
  one written.
- 🔵 **Clarity**: Report body sections differ from the parent epic —
  Resolved
- 🔵 **Testability**: "Both inform the answer" not observable — Resolved
- 🔵 **Testability**: Proposal constraints and next-step naming — Resolved
- 🔵 **Testability**: `complete` happy path and reopen from `synthesised` —
  Resolved
- 🔵 **Completeness**: Open question for the subagent choice — Still present
- 🔵 **Scope**: Per-extra vocabulary mechanism seam — Resolved. It is
  bounded to `coverage`.
- 🔵 **Testability**: Ordering and `.levels/` observation method — Still
  present
- 🔵 **Testability**: Slug derivation and suffix format — Still present.
  `report_count` after a collision is also not asserted.
- 🔵 **Clarity**: Undefined project terms — Partially resolved. "Reference
  systems" and stragglers are now defined; `outstanding`, Shared Preamble
  and 3-tier override are still undefined.
- 🔵 **Clarity**: Report `status` vs set `complete` — Resolved

### New Issues Introduced

- 🟡 **Clarity / Dependency / Scope / Testability**: Merged-tier rule
  depends on draft 0306 and contradicts "tiers carry through unchanged" —
  0306 is only Related. Until it lands, `synthesise` has no merge rule to
  inherit. The Sources requirement and criterion say tiers are unchanged,
  and the item does not say whether `report` merges itself or reads
  `synthesis.md`. The direction of "lowest" is not stated, and no criterion
  covers a source cited at conflicting tiers.
- 🟡 **Testability**: Equivalent-report criterion is conditioned on the
  model's own judgement — there is no fixture of equivalent and
  non-equivalent questions, so the overwrite safeguard cannot fail.
- 🔵 **Clarity**: A confirmed proposal's writes are described
  inconsistently — `ask` "writes nothing except a confirmed proposal", yet
  the reopen edits `manifest.md`. "Final manifest edit" is also ambiguous.
- 🔵 **Testability**: Refusal when several gates fail at once is unspecified.
- 🔵 **Dependency**: 0347's proposal extension is an ordering constraint
  recorded only as `relates_to`.
- 🔵 **Scope / Dependency**: Whether the visualiser's `template-tier.ts`
  entry for the report template is in scope is unclear.
- 🔵 **Clarity**: The equivalent-report pick is undefined for an
  `unanswered` judgement (suggestion).
- 🔵 **Clarity**: "Part of the question" is undefined (suggestion).
- 🔵 **Dependency**: 0305 and 0311 affect tier accuracy, and so coverage
  (suggestion).
- 🔵 **Completeness**: References list omits 0306 and the blockers
  (suggestion).
- 🔵 **Testability**: Template-override and report-body criteria lack an
  observable marker (suggestion).

### Assessment

All 5 major findings from pass 1 are resolved or narrowed, and the
structural problems (the rejected-report flow, the untrusted-data contract,
the 0347 gate, the contract criteria) are closed. The verdict stays REVISE
on two new majors. Both are narrower. The first is the 0306 cluster, which
four lenses flagged: the coverage rule now depends on a draft item's
merge rule and contradicts the "tiers carry through unchanged" wording. The
second is the untestable equivalence safeguard. Resolving the 0306 ownership,
either by making 0306 a blocker or by having 0281 state the rule itself, and
pinning an equivalence fixture would bring the item to COMMENT.

## Re-Review (Pass 3) — 2026-10-23T13:16:15+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity / Dependency / Scope / Testability**: Merged-tier rule
  depends on draft 0306 and contradicts "tiers carry through unchanged" —
  Resolved. 0306 is now a blocker, `report` applies the rule itself,
  "lowest" means `tier-3`, and a criterion covers a source cited at
  conflicting tiers.
- 🟡 **Testability**: Equivalent-report criterion is conditioned on the
  model's own judgement — Partially resolved. A fixture now exists, but one
  of its cases contradicts "equivalent" (see new issues).
- 🔵 **Clarity**: Confirmed proposal's writes described inconsistently —
  Resolved
- 🔵 **Testability**: Multiple gate failures unspecified — Resolved. Gates
  are checked in order and refusal happens at the first that fails. Only
  status-over-freshness is exercised by a criterion.
- 🔵 **Dependency**: 0347 ordering constraint — Resolved
- 🔵 **Scope / Dependency**: `template-tier.ts` ownership — Resolved (0281
  owns the entry). Scope suggests a Drafting Note on the departure from
  0121's "no visualiser work".
- 🔵 **Clarity**: Equivalent-report pick for `unanswered` — Resolved
- 🔵 **Clarity**: "Part of the question" undefined — Resolved
- 🔵 **Dependency**: 0305/0311 — Partially resolved. They are in the prose
  but not in `relates_to`.
- 🔵 **Completeness**: References list — Resolved
- 🔵 **Testability**: Template-override marker — Resolved
- 🔵 **Testability**: Slug derivation — Partially resolved. Punctuation and
  length are still unspecified.
- 🔵 **Completeness**: Open question for the subagent choice — Resolved
- 🔵 **Testability**: Ordering and `.levels/` observation — Resolved

### New Issues Introduced

- 🟡 **Clarity**: "Equivalent" report matching contradicts the reference
  expectations — the fixture expects an overlapping question to list both A
  and B.
- 🟡 **Clarity**: Unclear which question a rewritten report's answer
  answers — the accepted answer was produced for the new wording.
- 🟡 **Clarity**: The "no `openalex` finding" criterion conflicts with the
  completeness gate.
- 🔵 **Completeness**: Report contract rules (body layout, fixed `status`,
  typed linkage) live only in Technical Notes.
- 🔵 **Completeness / Testability**: The coverage reference corpus is not
  named, located or made a deliverable, and its questions are not tied to
  the criteria.
- 🔵 **Testability**: No repeatability standard for criteria that rest on
  model judgement.
- 🔵 **Testability**: `report_count` expectations conflict with a reference
  corpus that already holds reports A and B.
- 🔵 **Clarity**: Pick flow for answers with no candidate reports is
  unclear; "offers nothing" and "pending item" are undefined; "complete" is
  overloaded.
- 🔵 **Dependency**: Source of the freshness gate untraced (0279's item
  describes a different gate); the handover to 0284 omits `coverage`, Gaps
  and the `also` suffix.
- 🔵 Suggestions: the breadth-ceiling criterion can pass trivially; the
  freshness-over-completeness order is not exercised; report status
  invariants are untested; the injection scenario is not pinned; 0306's
  suffix wording may change; a Drafting Note is needed on visualiser work;
  an Open Questions section is missing; the rejected-`ask` "no file
  changes" wording is ambiguous.

### Assessment

The pass-2 majors on 0306 ownership are closed, and scope, completeness,
dependency and testability report no majors. The verdict stays REVISE on
three clarity majors. Two of them come from the equivalence fixture added in
pass 2, which treats an overlapping question as equivalent, and from the
rewrite wording that relies on equivalence. The third is a criterion that
contradicts the completeness gate. Fixing the matching semantics and
rewording that criterion would leave only minors and suggestions.

## Re-Review (Pass 4) — 2026-10-23T13:21:57+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: "Equivalent" report matching contradicts the reference
  expectations — Resolved. Matching is strict equivalence, the fixture has
  an overlap-only negative case, and the decision is recorded in the
  Drafting Notes.
- 🟡 **Clarity**: Unclear which question a rewritten report's answer
  answers — Resolved
- 🟡 **Clarity**: "No `openalex` finding" criterion conflicts with the
  gate — Resolved. The criterion now covers profiles that exclude
  `openalex`.
- 🔵 **Completeness**: Report contract only in Technical Notes — Resolved
- 🔵 **Completeness / Testability**: Reference corpus not located or named
  — Partially resolved. C1–C4 are named and the corpus is a deliverable,
  but it does not yet hold every scenario the criteria need.
- 🔵 **Testability**: Repeatability standard — Resolved (three runs).
  Clarity asks which criteria count as judgement criteria.
- 🔵 **Testability**: `report_count` vs fixture reports — Resolved
- 🔵 **Clarity**: Pick flow, "offers", "pending item", "complete"
  overload — Resolved
- 🔵 **Clarity / Testability**: Slug punctuation and length — Resolved.
  There is still no truncation criterion.
- 🔵 **Dependency**: Freshness-gate source, 0284 handover, 0305/0311
  linkage — Resolved or narrowed. The gate's origin is still untraced to a
  work item.
- 🔵 Suggestions from pass 3 (breadth criterion, freshness-over-
  completeness order, report status invariants, pinned injection scenario,
  0306 wording, Open Questions, Drafting Note on visualiser work, rejected
  `ask` wording) — Resolved

### New Issues Introduced

- 🟡 **Scope**: `ask` and `report` could ship in sequence — Resolved by
  decision. The user chose a single unit, recorded in the Drafting Notes;
  this does not count towards the verdict.
- 🟡 **Clarity / Testability**: Report's `parent`/`relates_to` targets are
  unstated and no criterion covers them.
- 🟡 **Clarity**: Sources membership is undefined (are gap sources
  included?), and the merged-tier criterion can read as `unanswered`.
- 🟡 **Testability / Completeness**: Sources, mixed-tier, retracted and
  injection criteria rely on fixture content the reference corpus does not
  define.
- 🔵 **Testability**: Rewrite does not assert the updated `coverage`/Gaps;
  no slug-truncation criterion; the `complete`-set `ask` criterion is near
  unfalsifiable and `report` on a `complete` set is untested; injection is
  tested in findings only, not `synthesis.md`; the preserved `round_count`
  and `finding_count` and the quarantined or invalid finding cases are
  unchecked.
- 🔵 **Clarity**: Equivalence criterion mixes "picks one or none" with "no
  pick offered"; "stale" clashes with 0279's sense; the manifest template's
  `reports/` entry is described both as unconditional and as conditional;
  "judgement criterion" is undefined.
- 🔵 **Dependency**: Epic 0121's report contract is superseded with no
  owner to update it; the freshness gate's origin is untraced.
- 🔵 **Scope**: Per-extra vocabulary is validator infrastructure inside a
  skill story.
- 🔵 Suggestions: the 0347 ordering is in prose only; the reference corpus
  may need live services unless hand-authored; the corpus could be a
  reusable deliverable; borrowed terms need links; one noun should be used
  for the set; subagent visibility in transcripts.

### Assessment

All pass-3 majors are resolved. The new majors are narrower and
specification-level: report linkage targets, Sources membership and fixture
content. Each lens is now finding finer detail as earlier gaps close, which
suggests the item is converging. The report linkage can be settled from the
skill's existing rule, and the fixture content follows from the existing
criteria. Sources membership is the one remaining behavioural choice.

## Re-Review (Pass 5) — 2026-10-23T13:28:44+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity / Testability**: Report linkage targets unstated — Resolved.
  Reports follow the skill's linkage rule, and the C1 report criterion
  checks `parent`.
- 🟡 **Clarity**: Sources membership undefined — Resolved. Every source
  drawn on is listed, gap evidence included.
- 🟡 **Testability / Completeness**: Criteria rely on undefined fixture
  content — Partially resolved. C5, the retracted source, the injection
  fixture and the `.levels/` canary are defined, but some criteria still
  need corpus states that are not described (see new issues).
- 🔵 Pass-4 minors (rewrite coverage, truncation criterion, `complete`
  path, `synthesis.md` injection, preserved counts, quarantined and invalid
  findings, "stale", manifest template line, judgement criteria, epic
  update, freshness-gate origin, 0347 ordering) — Resolved

### New Issues Introduced

- 🟡 **Testability / Clarity**: Fixture states conflict with the criteria.
  Report A asks exactly C1's question, so `report SLUG C1` criteria lack a
  defined pick. The rewrite criterion needs a C2 state that contradicts
  C2's fixed judgement, and no four-gap question exists for the breadth
  criterion.
- 🟡 **Dependency**: 0284 does not record the `coverage`, Gaps and `also`
  rendering that 0281 assigns to it.
- 🔵 **Clarity**: The slug truncation rule differs between Requirements and
  the criterion; the injection text and C4 state are given as "or";
  "writing nothing otherwise" reads two ways; "item" is undefined in the
  gate; "orientation only" conflicts with findings being the evidence;
  duplicate sources at the same tier are unaddressed.
- 🔵 **Testability**: "Every source drawn on" has no fixture oracle; gap
  naming, the Sources tier form and `.levels/` avoidance are outside the
  three-run rule.
- 🔵 **Dependency**: 0306's parser question is not reflected; 0306 is
  medium priority on a high-priority item's critical path; the gate's
  reliance on `outstanding` is unrecorded.
- 🔵 **Scope**: Size is accepted only implicitly.
- 🔵 Suggestions: References omit the 0279 plan; Open Questions should
  track 0306; the epic update has no criterion; the config-tier override,
  `relates_to` and "framed by the question" have no criteria; the Summary's
  "synthesised" could be read as excluding `complete`.

### Assessment

The pass-4 majors are closed. The new majors come from fixture
consistency, now that the fixtures are concrete enough to cross-check, and
from 0284 not being updated for the report contract. Both can be fixed
mechanically: name each fixture state per criterion, and record the
rendering obligations on 0284.

## Re-Review (Pass 6) — 2026-10-23T13:35:03+00:00

**Verdict:** REVISE

Lenses re-run: clarity, testability.

### Previously Identified Issues

- 🟡 **Testability / Clarity**: Fixture states conflict with criteria —
  Resolved. The base, equivalence and rewrite states are named per
  criterion, and C6, C7, S1, R1 and R2 are defined.
- 🟡 **Dependency**: 0284 missing the report rendering obligations —
  Resolved. 0284 gained requirements and criteria for `coverage`, Gaps and
  the `also` suffix (not re-reviewed this pass).
- 🔵 Pass-5 clarity and testability minors — Resolved

### New Issues Introduced

- 🟡 **Clarity / Testability**: Template-override criterion — "adding
  `## Scope` as well" contradicts "not `## Method`"; the tier precedence
  is unstated.
- 🟡 **Testability**: The equivalence questions have no fixed coverage; the
  vector-store question would likely be judged `unanswered`, which
  contradicts the accept/reject flow.
- 🟡 **Testability**: Rewriting under the original `question` is never
  exercised with differently worded input.
- 🔵 **Clarity**: Source of `relates_to` values; Sources handling of an
  existing `also` suffix; next steps after a declined proposal; the status
  gate's "recovery path" for `briefed`/`outlined`; disk-derived
  `round_count` gloss; slug character class.
- 🔵 **Testability**: Brief profiles and breadth unset in the fixture; no
  operation-item case; compound gate triggers; trivial `parent` check and
  no `relates_to` check; "no pick for `unanswered`" unchecked; slug,
  collision and 0121 fixtures partly specified.

### Fixes applied after this pass (not yet re-reviewed)

All three majors and every minor above were addressed:
- The config-tier override now adds `## Scope` without `## Method`, and the
  tiers replace in order.
- The equivalence questions are each `answered`, with the vector-store
  question replaced by an in-domain one.
- A rewrite-under-different-wording criterion picks A.
- Report U backs an unanswered-equivalent case.
- The fixture manifest has `parent: work-item:0900`, and the brief is
  limited to `web` and `openalex`.
- Breadth is set through `research.topic.breadth`.
- Each gate trigger is a separate case.
- C8 pins the no-hyphen slug.
- The 0121 fields are listed.

0281 now has 33 criteria.

### Assessment

The majors on each pass are now inconsistencies in the fixture added the
pass before, not design gaps. The design has been stable since pass 4.

## Verdict Change — 2026-10-23T13:40:53+00:00

**Verdict:** COMMENT (changed from REVISE by the reviewer)

The design has been stable since pass 4. Passes 5 and 6 found only
consistency problems in fixture detail added in the previous pass, and the
pass-6 fixes were applied. The remaining risk is fixture detail, which
planning and the reference corpus's construction will exercise. The
work item is acceptable for planning.

## Verdict Change — 2026-10-23T13:42:14+00:00

**Verdict:** APPROVE (changed from COMMENT by the reviewer)

The reviewer approved the work item for implementation, and its status
moved to `ready`.
