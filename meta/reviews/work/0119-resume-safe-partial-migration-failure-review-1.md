---
type: work-item-review
id: "0119-resume-safe-partial-migration-failure-review-1"
title: "Work Item Review: Resume-Safe Partial Migration Failure"
date: "2026-06-19T23:55:54+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
parent: "work-item:0115"
target: "work-item:0119"
work_item_id: "0119"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 3
tags: [migrate, interactive-migration, agent-invocation]
last_updated: "2026-06-20T00:36:16+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Resume-Safe Partial Migration Failure

**Verdict:** REVISE

This is a well-structured, atomic task that maps cleanly to fix E of the source
research, with every expected section present and an explicit out-of-scope
boundary against a full staging/transaction layer. The blocking issue is not
structure but definition: the central concept the whole work item pivots on —
"this run's own prior-migration output" / the "ownership set" — is left
operationally undefined behind an open design question, which ripples into
contradictory acceptance criteria, untestable fixtures, and an uncaptured
planning prerequisite. Five major findings (no critical) cross three lenses,
all tracing back to that one unresolved concept plus an ambiguous third
acceptance criterion.

### Cross-Cutting Themes

- **Undefined "ownership set" / "this run's own prior-migration output"**
  (flagged by: clarity, completeness, dependency, scope, testability — all five)
  — Every lens independently flagged that the term anchoring all Requirements
  and Acceptance Criteria has no settled definition. Its identification source
  is openly listed as an unresolved Open Question (session log vs. ledger vs. a
  new path manifest). This makes the Requirements read as more definite than
  they are (clarity), leaves the item not yet actionable for planning
  (completeness), is an uncaptured upstream constraint masquerading as "Blocked
  by: none" (dependency), risks doubling the scope if a new manifest is needed
  (scope), and blocks fixture construction so the criteria cannot be turned into
  pass/fail tests (testability).

- **Acceptance Criterion 3's mixed-path semantics are ambiguous**
  (flagged by: clarity, testability) — AC3 ("does not bypass the dirty-tree
  protection for paths outside this run's ownership set") presumes a resume can
  proceed while non-owned paths exist, which AC2 says must be refused. Clarity
  reads this as an internal contradiction between the criteria; testability
  reads it as a negative/absence assertion with no defined observation
  procedure. Both point to the same fix: pin down exactly what tree state a
  guarded resume runs against.

### Findings

#### Critical

_None._

#### Major

- 🟡 **clarity**: Third acceptance criterion's 'ownership set' boundary contradicts the first criterion's enabling condition
  **Location**: Acceptance Criteria
  AC1 enables a guarded resume only when dirty paths are *exclusively* owned, but
  AC3 presumes a resume can proceed while paths *outside* the ownership set
  exist — which AC2 says must still be refused. The three criteria together
  leave it ambiguous whether a guarded resume can ever run with any non-owned
  dirty path present.

- 🟡 **dependency**: Sibling fix items from the same research (0116, 0118, 0117, 0120) are uncaptured as related/ordering couplings
  **Location**: Dependencies
  This is fix E of a coordinated fix set (B/C/D/E) decomposed from 0115; sibling
  items authored concurrently touch the same scripts (0116 → `run-migrations.sh`
  `read_decision`; 0118 → `0007`/its validator). Dependencies lists only 0115
  and 0069, so two items could land conflicting edits to the same pre-flight/apply
  region.

- 🟡 **dependency**: Unresolved ownership-identification source is a stated planning prerequisite but not captured as a blocking constraint
  **Location**: Open Questions
  The work item itself states the ownership-source question "should be resolved
  before planning," yet Dependencies records "Blocked by: none." A reader
  scanning Dependencies sees no blocker and may schedule the task, only to stall
  on an unresolved design decision.

- 🟡 **testability**: Pivotal term 'this run's own prior-migration output' is operationally undefined, blocking fixture construction
  **Location**: Acceptance Criteria
  All three criteria turn on whether dirty paths are "owned," but the work item
  never defines how that set is determined. A verifier cannot construct the
  precondition fixtures needed to distinguish a passing AC1 case from a passing
  AC2 case.

- 🟡 **testability**: AC3 negative guarantee lacks an observable detection procedure
  **Location**: Acceptance Criteria (AC3)
  AC3 asserts the *absence* of a behaviour but does not state how a verifier
  would observe that the guard remained active for non-owned paths during a
  resume — no defined input and no expected signal, so it can be argued met
  regardless of implementation.

#### Minor

- 🔵 **clarity**: Central phrase 'this run's own prior-migration output' is used as a settled term but its referent is undefined
  **Location**: Requirements
  The phrase anchors every Requirement and Acceptance Criterion, yet its meaning
  is deferred to Open Questions, which lists three competing identification
  sources. The term is used throughout as if defined while its definition is
  openly unresolved.

- 🔵 **clarity**: Opening sentence's nested clauses are hard to parse on first read
  **Location**: Summary
  The first sentence stacks two relative clauses plus a trailing em-dash aside,
  so the referent of "which disables the single guard" (it refers to
  `ACCELERATOR_MIGRATE_FORCE=1`, not the resume) is only resolvable on a careful
  re-read.

- 🔵 **completeness**: Unresolved design question on ownership-identification source remains open
  **Location**: Open Questions
  The single Open Question is unresolved, and both Technical Notes and Drafting
  Notes state it should be resolved before planning. The item is not yet fully
  actionable for its stated next step.

- 🔵 **dependency**: Research-stated sequencing of E behind C/D not reflected as an ordering constraint
  **Location**: Summary
  The research characterises E as "secondary to C/D." Without that ordering
  recorded, this hygiene task could be prioritised ahead of the root-cause fixes
  or have its value re-litigated mid-sprint.

- 🔵 **testability**: AC1 'guarded resume is offered' does not specify the observable form of the offer
  **Location**: Acceptance Criteria (AC1)
  "Offered" is undefined as an observable output (a message? a non-error exit
  that proceeds? a prompt? an emitted resume command?), so a tester cannot assert
  on a concrete signal.

#### Suggestions

- 🔵 **clarity**: Passive 'a guarded resume is offered' leaves the offering actor unnamed
  **Location**: Acceptance Criteria
  The resume is described passively without naming what does the offering — the
  pre-flight, the runner, or an interactive prompt — three materially different
  behaviours.

- 🔵 **scope**: Unresolved ownership-source question could pull a new manifest into scope
  **Location**: Open Questions
  Assumptions concede "a lightweight manifest may be needed." A new path-manifest
  artifact is materially larger than "one guarded-resume branch"; if required,
  the task's scope roughly doubles and may warrant its own item.

- 🔵 **scope**: Sizing depends on an unresolved design decision
  **Location**: Technical Notes
  The M size is qualified by the open question. Because the chosen source
  determines whether the work is a single branch or a new artifact, the estimate
  is provisional rather than settled.

### Strengths

- ✅ Summary, Context, Requirements, and Acceptance Criteria tell one coherent
  story with no contradiction between the stated problem and the proposed fix.
- ✅ Maps cleanly to a single fix option (E) from the source research, with no
  bleed into sibling fixes A/B/C/D.
- ✅ Scope boundaries are stated explicitly and consistently — "a full
  staging/transaction boundary is out of scope" appears in both Summary and
  Requirements, pre-empting creep into the transaction-boundary alternative.
- ✅ All expected sections for a task are present and substantively populated
  (no placeholders); frontmatter is complete with a recognised kind/status,
  parent linkage (0115), and relations (0069).
- ✅ Three Given/When/Then acceptance criteria cover the positive resume path,
  the preserved-refusal path, and the no-weakening guarantee — beyond the
  two-criterion minimum.
- ✅ Technical Notes pin exact source locations (`run-migrations.sh:67-141`,
  `:90-132`, `:252-297`), giving an implementer/verifier the concrete seam.

### Recommended Changes

1. **Resolve the ownership-identification source, or commit to a leading
   candidate with a fallback** (addresses: all five "undefined ownership set"
   findings — the dominant cross-cutting theme)
   The Assumptions section already names session-log/ledger as the likely
   source; promote that to a decision in Requirements (e.g. "a path is 'owned'
   if it appears in this run's session log / per-migration ledger"). If a new
   manifest turns out to be required, split it into its own work item so this
   task stays bounded. This single change unblocks clarity (settled referent),
   completeness/dependency (actionable, real prerequisite resolved), scope
   (sizing settled), and testability (fixtures constructable).

2. **Reconcile AC3 with AC1/AC2 and make it a concrete positive observation**
   (addresses: clarity AC3-contradiction, testability AC3 negative-guarantee)
   Decide whether a guarded resume only ever runs on a fully-owned dirty tree
   (in which case fold AC3 into AC1/AC2 or remove its mixed-path framing) or
   must filter owned vs. non-owned paths at resume time (state that in
   Requirements). Reframe AC3 with a definite input and observable, e.g. "Given
   a non-owned dirty path is introduced after the ownership set is computed, when
   resume proceeds, then it refuses/aborts on that path."

3. **Specify the observable form of the resume offer** (addresses: clarity
   "offering actor unnamed", testability AC1 "offered" form)
   Name the component that offers the resume and what the offer looks like (exit
   0 proceeding into the apply loop without FORCE / an emitted resume-command
   message), so AC1 has a signal to assert against.

4. **Capture sibling and sequencing couplings in Dependencies** (addresses:
   dependency sibling-items, dependency E-behind-C/D sequencing, dependency
   ownership-source-as-blocker)
   Add the concurrent sibling items that share files (at minimum 0116 and 0118)
   to Relates-to, note this task is sequenced after / secondary to C/D per the
   research, and record the unresolved ownership-source decision as an explicit
   pre-planning gate / Blocked-by rather than "Blocked by: none."

5. **Tighten the Summary's opening sentence** (addresses: clarity "hard to
   parse")
   Split the sentence so the antecedent of "which disables the single guard"
   sits immediately next to `ACCELERATOR_MIGRATE_FORCE=1`.

## Per-Lens Results

### Clarity

**Summary**: The work item communicates its single intent — a guarded resume for
partial migration failures — with strong internal consistency between Summary,
Context, Requirements, and Acceptance Criteria, and it correctly traces back to
fix E of the source research. Clarity is mostly good, but a few referents and
quantifiers are underspecified: the recurring phrase "this run's own
prior-migration output" is doing heavy lifting without a settled definition
(acknowledged as an Open Question), and one acceptance criterion contains a
subtle scope mismatch against the Requirements regarding mixed dirty-path sets.

**Strengths**:
- Summary, Context, and Requirements tell one coherent story: the FORCE bypass
  disables the sole corpus guard, and the fix is a narrower
  path-ownership-gated resume — no contradiction between problem and solution.
- Scope boundaries are stated explicitly and consistently ("a full
  staging/transaction boundary is out of scope" appears in both Summary and
  Requirements).
- `ACCELERATOR_MIGRATE_FORCE` and "corpus guard" are introduced with enough
  surrounding explanation that a new reader can follow the change.

**Findings**:
- 🟡 major (high) — Acceptance Criteria — Third acceptance criterion's
  'ownership set' boundary contradicts the first criterion's enabling condition.
  AC1 enables resume only when dirty paths are exclusively owned; AC3 presumes a
  resume can proceed while non-owned paths exist (which AC2 refuses). An
  implementer cannot tell whether the guarded resume operates on a fully-owned
  tree or a mixed tree.
- 🔵 minor (medium) — Requirements — Central phrase "this run's own
  prior-migration output" is used as a settled term but its referent is
  undefined; its meaning is deferred to Open Questions, which lists three
  competing identification sources.
- 🔵 minor (medium) — Summary — Opening sentence's nested relative clauses plus
  em-dash aside make the antecedent of "which disables the single guard"
  resolvable only on a careful re-read.
- 🔵 suggestion (low) — Acceptance Criteria — Passive "a guarded resume is
  offered" leaves the offering actor unnamed (pre-flight? runner? interactive
  prompt?), three materially different behaviours.

### Completeness

**Summary**: Work item 0119 is a well-structured task with every expected
section present and substantively populated: Summary, Context, Requirements,
Acceptance Criteria, Open Questions, Dependencies, Assumptions, Technical Notes,
Drafting Notes, and References. The work to be done is clearly defined for a task
kind, frontmatter is complete with a recognised kind/status, and the acceptance
criteria meaningfully define done. The only notable completeness consideration
is that the work item carries an unresolved open design question that the item
itself flags must be settled before planning.

**Strengths**:
- All expected sections for a task are present and substantively populated — no
  placeholders.
- Frontmatter is complete and well-formed: kind (task), status (draft), priority
  (high), parent (0115), relations (0069), tags.
- Acceptance Criteria contains three specific given/when/then criteria covering
  happy path, preserved-refusal path, and no-bypass guarantee.
- Context explains motivation (no transaction boundary; only the corpus-wide
  FORCE bypass) rather than restating the Summary.
- Requirements are concrete and implementation-ready, including an explicit
  out-of-scope boundary.

**Findings**:
- 🔵 minor (medium) — Open Questions — Unresolved design question on
  ownership-identification source remains open; both Technical Notes and Drafting
  Notes state it should be resolved before planning, so the item is not yet fully
  actionable for that next step. The Assumptions section already names
  session-log/ledger as the likely source, which could be promoted to a decision.

### Dependency

**Summary**: As fix E decomposed from epic 0115, this task names its parent
(0115) and a related runner-side resumability item (0069), and correctly bounds
the relationship to 0069. However, the work item under-captures couplings to its
sibling fix items (notably the 0116 structured-stall and 0118/0007
backfill-sentinel work) that the research frames as related and partly sequenced
ahead of E, and it leaves a design-source open question unresolved that the
Drafting Notes themselves flag as a planning prerequisite. No hard upstream
blocker prevents starting, but the sequencing relative to siblings and the
shared touch on run-migrations.sh are uncaptured.

**Strengths**:
- The parent epic (0115) and the runner-side resumability item it extends (0069)
  are both named in Dependencies and frontmatter.
- The relationship to 0069 is correctly scoped as "extends" rather than a
  blocker, and Blocked-by/Blocks are stated explicitly rather than left to
  inference.

**Findings**:
- 🟡 major (medium) — Dependencies — Sibling fix items from the same research
  (0116, 0118, 0117, 0120) are uncaptured as related/ordering couplings; several
  touch the same scripts (0116 → `run-migrations.sh`; 0118 → `0007`/validator),
  risking conflicting edits to the same pre-flight/apply region.
- 🟡 major (medium) — Open Questions — Unresolved ownership-identification source
  is a stated planning prerequisite but recorded as "Blocked by: none," so the
  prerequisite is invisible in the dependency record.
- 🔵 minor (medium) — Summary — Research-stated sequencing of E behind C/D not
  reflected as an ordering constraint; this hygiene task could be prioritised
  ahead of the root-cause fixes.

### Scope

**Summary**: Work item 0119 is a well-scoped, atomic task: it carves out exactly
one of the five fix options (E) from the research issue and draws an explicit
out-of-scope boundary against a full staging/transaction layer. The Summary,
Requirements, and Acceptance Criteria all describe the same single concern —
guarded resume via path-ownership detection — with no bundling of independent
work. The only scope-adjacent concern is an unresolved design question (the
ownership-identification source) flagged in the work item itself as something to
resolve before planning, which is a sizing/readiness nuance rather than a
decomposition defect.

**Strengths**:
- Maps cleanly to a single fix option (E); siblings A/B/C/D are decomposed into
  separate work items, giving crisp boundaries.
- Explicitly states what is out of scope in both Summary and Requirements,
  pre-empting creep into the transaction-boundary alternative.
- Summary, Requirements, and Acceptance Criteria are mutually consistent — no
  "and also" bundling of a second capability.
- The "task" kind and M sizing are appropriate for the described footprint.

**Findings**:
- 🔵 suggestion (medium) — Open Questions — Unresolved ownership-source question
  could pull a new manifest into scope; Assumptions concede "a lightweight
  manifest may be needed," which is materially larger than one guarded-resume
  branch and may warrant its own work item.
- 🔵 suggestion (low) — Technical Notes — Sizing depends on an unresolved design
  decision; the M estimate is conditional on whether the source is an existing
  artifact or a new manifest.

### Testability

**Summary**: The three Acceptance Criteria are framed as Given/When/Then triples
with clear preconditions and observable outcomes, and they map cleanly to the
three Requirements. The principal testability gap is that the central concept
the criteria pivot on — "this run's own prior-migration output" / the "ownership
set" — is never operationally defined, and the work item explicitly leaves its
identification source as an unresolved Open Question, so a verifier cannot
construct the input fixtures needed to exercise the criteria. A secondary gap is
AC3's "does not bypass the dirty-tree protection for paths outside this run's
ownership set", whose negative-observable nature has no specified procedure for
detection.

**Strengths**:
- All three criteria use an explicit precondition/action/expected-outcome
  structure, naming trigger and observable outcome rather than implementation
  steps.
- The criteria collectively cover the positive resume path, the preserved-refusal
  path, and the no-weakening guarantee.
- AC2 specifies a concrete negative case with a definite expected outcome (the
  pre-flight still refuses), giving a clear pass/fail boundary.
- Technical Notes pin exact source locations, giving a verifier the concrete seam
  to instrument.

**Findings**:
- 🟡 major (high) — Acceptance Criteria — Pivotal term "this run's own
  prior-migration output" is operationally undefined, blocking fixture
  construction; a verifier cannot distinguish a passing AC1 case from a passing
  AC2 case.
- 🟡 major (medium) — Acceptance Criteria (AC3) — AC3 negative guarantee lacks an
  observable detection procedure; it asserts an absence with no defined input or
  expected signal and overlaps ambiguously with AC2.
- 🔵 minor (medium) — Acceptance Criteria (AC1) — AC1 "guarded resume is offered"
  does not specify the observable form of the offer (message? non-error exit?
  prompt? emitted command?), leaving the pass/fail line subjective.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-06-20T00:29:05+00:00

**Verdict:** REVISE

All five lenses were re-run against the revised work item. Every finding from
Pass 1 is resolved — the abstract defects (undefined ownership concept, AC3
contradiction, missing dependency couplings) are gone. Resolving them by
committing to a concrete mechanism (the per-run path manifest) exposed a fresh
layer of second-order findings: the manifest's observable surface and
failure-mode behaviour are not yet specified, and the C/D sequencing — now
captured as prose — wants promotion to a tracked ordering constraint with a
work-item id for fix D. Four new majors (no critical), all concrete and cheaply
addressable.

### Previously Identified Issues

- 🟡 **clarity**: AC3 ownership-set boundary contradicts AC1 — **Resolved** (AC3 folded into the refusal criterion; fully-owned-only semantics stated).
- 🟡 **dependency**: Sibling fix items (0116/0118/0117/0120) uncaptured — **Resolved** (captured in Dependencies + frontmatter `relates_to`).
- 🟡 **dependency**: Ownership-source prerequisite not captured as blocking — **Resolved** (design question resolved in Open Questions; "Blocked by: none" now justified inline).
- 🟡 **testability**: Pivotal "ownership" term operationally undefined — **Resolved** (defined: a path is owned iff it appears in the per-run manifest).
- 🟡 **testability**: AC3 negative guarantee unobservable — **Resolved** (folded into a concrete refusal criterion).
- 🔵 **clarity**: Central phrase used as settled but undefined — **Resolved**.
- 🔵 **clarity**: Summary opening sentence hard to parse — **Resolved** (sentence split).
- 🔵 **completeness**: Open design question unresolved — **Resolved** (completeness returned zero findings this pass).
- 🔵 **dependency**: E-behind-C/D sequencing not reflected — **Partially resolved** (now present as a prose Sequencing note; dependency lens wants it promoted to a tracked ordering entry — see new issues).
- 🔵 **testability**: AC1 "offered" form unspecified — **Resolved** (now exit 0 + resume-affordance message).
- 🔵 **clarity**: Offering actor unnamed — **Resolved** (pre-flight named throughout).
- 🔵 **scope**: Manifest could pull scope wider / sizing provisional — **Resolved** (decision recorded; size set to M–L; split considered and rejected with rationale).

### New Issues Introduced

- 🟡 **testability**: AC1 manifest-recording outcome lacks an observable verification surface — the manifest's location/format isn't specified, so "recorded in the manifest" has no concrete artefact to assert against. (Location: Acceptance Criteria)
- 🟡 **testability**: No criterion covers a missing/stale/partially-written manifest — an implementation could treat an absent manifest as "everything owned" and defeat the corpus guard while still passing all current criteria. A fail-closed criterion is needed. (Location: Acceptance Criteria)
- 🟡 **dependency**: C/D-before-E sequencing recorded as a soft prose note rather than a tracked ordering constraint. (Location: Dependencies)
- 🟡 **dependency**: Fix D (the 0007-split) is referenced by letter only, with no work-item id, so the "schedule after D" ordering is unenforceable. (Location: Dependencies)
- 🔵 **clarity**: Fix letter codes (C/D/E) used without an in-document mapping to work-item ids. (Location: Dependencies)
- 🔵 **clarity**: "mutates" vs "completes" leaves recorded-path timing ambiguous for a migration that fails mid-way. (Location: Requirements)
- 🔵 **dependency**: Shared-surface note pairs 0007 only with 0118 and run-migrations.sh only with 0116; this task touches both, so the conflict surface is split imprecisely. (Location: Dependencies)
- 🔵 **dependency**: Downstream coupling to 0120 stated as "relates to" rather than a tracked Blocks. (Location: Dependencies)
- 🔵 **testability**: Resume-affordance message has no checkable content threshold (which paths, which stream). (Location: Acceptance Criteria)
- 🔵 **testability**: AC1 precondition ("applies one or more migrations and then fails") not anchored to a reproducible failure fixture. (Location: Acceptance Criteria)
- 🔵 **scope**: M–L is the largest fix in the five-fix set; optional internal phase boundary (manifest-write then resume-read) suggested, not a defect.

### Assessment

The revision was a clear success against its target: every Pass-1 finding is
resolved and completeness is now clean. The work item moved from "abstractly
underspecified" to "concretely specified with edge cases to pin down." The
remaining REVISE is driven by the manifest's testability surface (where it
lives, and the fail-closed behaviour when it is missing/stale) and tightening
the C/D sequencing into a tracked constraint with an id for fix D. These are
narrow, concrete edits — a third pass should clear them. The two
manifest-testability majors are the priority: the missing fail-closed criterion
is a genuine guard-regression risk, not just documentation polish.

## Re-Review (Pass 3) — 2026-06-20T00:36:16+00:00

**Verdict:** APPROVE

Pass 2's four new majors are all resolved (manifest format/location named,
fail-closed criterion added, fix-letter map added, C/D sequencing promoted to a
tracked "Ordered after" entry, 0120 recorded as a downstream block). Completeness
remained clean and was not re-run. The remaining findings are of a notably
different character from passes 1–2: they are either (a) external to this work
item (fix D does not yet exist as a work item — a sibling-decomposition action,
already documented here as a known gap), or (b) test-assertion-level precision
(exact stderr marker token, refusal exit code, AC4 condition-splitting, manifest
dedup/ordering) that conventionally belongs in the implementation plan, not the
work item. This is the diminishing-returns tail of the review.

### Previously Identified Issues (Pass 2 new majors)

- 🟡 **testability**: AC1 manifest-recording lacked an observable surface — **Resolved** (manifest now specified as a plain-text file, one repo-relative path per line, co-located with the ledger; AC1 asserts against it).
- 🟡 **testability**: No criterion for a missing/stale manifest — **Resolved** (AC4 added: fail closed → refuse; "never treated as everything owned").
- 🟡 **dependency**: C/D sequencing was a soft prose note — **Resolved** (promoted to an "Ordered after: 0118 / fix D" entry with rationale).
- 🟡 **dependency**: Fix D referenced without a work-item id — **Resolved as far as 0119 can** (fix-letter map added; D explicitly flagged as undecomposed and the ordering noted as unenforceable until D exists — see persisting issue below).
- 🔵 Pass-2 minors (letter mapping, mutates-vs-completes timing, affordance content, AC1 fixture anchor, 0118/0116 surface precision, 0120 coupling) — **Resolved** in the revision.

### New / Persisting Issues

- 🟡 **dependency**: "Ordered after fix D" still points at a non-existent work item. *Not fixable within 0119* — the resolution is to decompose fix D (the 0007-split) into its own item or accept E as independent hygiene on epic 0115. 0119 already documents the gap honestly.
- 🟡 **testability**: AC2's resume-affordance message has no exact content contract (marker token / phrasing). Borderline over-specification for a work item; the stable observable (message present + lists owned paths on stderr) is stated — exact token belongs in the plan.
- 🟡 **testability**: AC3/AC4 refusal outcome lacks an explicit observable signal (exit code, affordance-message absence, FORCE-hint present). **Genuinely worth adding** — it makes the safety-critical guard-not-relaxed guarantee verifiable rather than narrative.
- 🔵 **clarity**: "corpus"/"corpus guard", "ledger", "state area" used without first-use glosses; "run identity" stated only as an example. Readability polish.
- 🔵 **testability**: AC4 bundles four fail-closed conditions in one checkbox; AC1 leaves dedup/ordering of the manifest implicit; per-mutation timing only verified indirectly via the post-abort end state.
- 🔵 **scope**: New persisted artefact pushes this to the upper edge of a `task` kind — a planning-time flag to watch, not a defect.

### Assessment

The work item is now in strong, plannable shape — three rounds have driven it
from abstractly underspecified to concretely specified with edge cases covered.
The one remaining edit worth making to the work item itself is giving the
refusal criteria (AC3/AC4) an explicit observable signal, since that is the
safety-critical guarantee the whole task exists to protect. Everything else is
either an epic-level sibling action (decompose fix D) or assertion-grade detail
best fixed when the implementation plan is written. Recommendation: make the
refusal-observability edit, treat the rest as plan-level, and consider the work
item ready rather than chasing further lens passes.

**Resolution:** The refusal-observability edit was applied — AC3 and AC4 now
assert an explicit observable refusal (non-zero exit, no resume-affordance
message, dirty-tree FORCE-hint present), so the safety-critical guard-not-relaxed
guarantee is verifiable. With that in place the work item is approved for
planning. Two items are deliberately carried out of the work item: (1) decompose
fix D (the 0007-split) into its own work item, or accept E as standalone hygiene
— an epic-0115 scheduling decision; (2) assertion-grade detail (exact stderr
marker token, splitting AC4's four fail-closed conditions, manifest
dedup/ordering semantics) to be settled in `/create-plan`.
