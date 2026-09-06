---
type: "work-item-review"
id: "0257-sync-specific-work-items-review-1"
title: "Work Item Review: Sync Specific Work Items"
date: "2026-09-06T10:00:27+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:0146"
target: "work-item:0257"
work_item_id: "0257"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-09-06T14:51:05+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Sync Specific Work Items

**Verdict:** REVISE

This is a well-structured, unusually well-populated story: every section
carries substantive, kind-appropriate content, the scope is a single coherent
capability (targeted reconciliation over named work items), and most acceptance
criteria are observable Given/When/Then pairs with strong negative assertions.
The verdict is REVISE only on the major-count threshold — three major findings,
no critical — clustered around two gaps: acceptance criteria that do not cover
every behaviour the Requirements promise, and dependency couplings recorded as
flat, direction-less relations that may hide an upstream blocker. Resolving the
unsettled precedence Open Question would close the largest cross-cutting risk.

### Cross-Cutting Themes

- **Dual-match / precedence undecided** (flagged by: testability, clarity) —
  Open Question 2 leaves the outcome undefined when a token is both a valid
  local-id shape and some item's `external_id`, while Requirement 8 treats
  "ambiguous" as a settled abort condition. Testability cannot pin a pass/fail
  for the most error-prone resolution case; clarity flags the Requirement and
  the Open Question as contradictory.
- **Requirements outrun the acceptance criteria** (flagged by: testability,
  completeness) — the Requirements promise full-sync parity (conflict dossiers,
  blast-radius bounds, baseline resumability, `--resolve`, max-pull/push caps)
  and a target surface, but several of those behaviours have no verifying
  criterion and the interface surface is deferred to an Open Question while the
  item sits at `ready`.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: `behaves as today` criterion has no defined baseline
  **Location**: Acceptance Criteria (AC10)
  AC10 asserts the no-target run "behaves as today" but captures no baseline —
  expected outputs, report shape, or write set — so a verifier cannot produce a
  definitive pass/fail against the very whole-set path this change must not
  break.

- 🟡 **Testability**: Several Requirements behaviours have no verifying criterion
  **Location**: Requirements
  Requirements promise per-item parity with full sync (conflict dossiers,
  blast-radius bounds, baseline-last resumability) and composition with
  `--resolve` and `--max-pulls`/`--max-pushes`, yet the criteria verify only the
  dirty-overwrite guard, `--preview`, and `--push-only`/`--pull-only`. The rest
  could regress silently with no acceptance check.

- 🟡 **Dependency**: 0213 (id-targeted resolve) may be an upstream blocker
  **Location**: Dependencies
  This story's core mechanism is id-based target resolution, yet 0213
  "id-targeted resolve" is recorded as a symmetric "Relates to" under "Blocked
  by: none". If 0213 delivers the resolution capability this story consumes, it
  is an upstream blocker, and scheduling this first would surface a hidden
  blocker mid-sprint.

#### Minor

- 🔵 **Completeness**: Target-surface shape left undecided in a ready item
  **Location**: Requirements
  Requirements say the skill "accepts one or more targets" but defer the
  invocation surface (positional vs repeatable `--target`/`--only`) to an Open
  Question, while `status: ready`. An implementer must resolve this interface
  decision before starting.

- 🔵 **Testability**: Expected outcome for dual-match token is undefined
  **Location**: Open Questions
  With precedence unresolved for a token that matches both a local id and an
  `external_id`, the expected result is undefined and AC6 does not say whether
  such a token counts as ambiguous — a verification blind spot in the most
  error-prone case.

- 🔵 **Dependency**: Remote tracker external system not named as a coupling
  **Location**: Requirements
  Reading, matching (`external_id`, e.g. `PP-787`), creating remote issues, and
  detecting remote-absent items are all operations against an external tracker
  never named in Dependencies, so its availability/SLA implications are
  invisible to planning.

- 🔵 **Dependency**: Ordering with 0255 (chunk merge) and 0229 (pull scope) left
  undirected
  **Location**: Dependencies
  This story reuses full-sync conflict handling and suppresses discovery,
  overlapping 0255 (conflict) and 0229 (pull/discovery), yet both are undirected
  "Relates to" entries, so any ordering constraint is unexpressed.

- 🔵 **Clarity**: The word `resolve` carries three distinct meanings
  **Location**: Requirements
  "Resolve" denotes mapping a target to a local file, the `accelerator work
  resolve` command, and the `--resolve` flag (whose purpose is never stated and
  most naturally reads as conflict resolution). A reader cannot tell what
  `--resolve` controls.

- 🔵 **Clarity**: Multiple names used for the same whole-set behaviour
  **Location**: Context
  The whole-set behaviour appears as "a full run", "full-set sync", "a full
  sync", "whole-set reconcile", and "full-set sync semantics", forcing the
  reader to unify five labels for one concept.

- 🔵 **Clarity**: Requirement treats `ambiguous` as settled while an Open
  Question reopens it
  **Location**: Open Questions
  Requirement 8 aborts on an "ambiguous" target, but Open Question 2 asks
  whether the both-match case is ambiguity or resolves by precedence, leaving
  the two sections contradictory.

#### Suggestions

- 🔵 **Scope**: Remote-id (`external_id`) resolution is the one net-new mechanism
  worth confirming as part of this unit
  **Location**: Requirements
  Most work reuses existing machinery, but corpus-wide `external_id` lookup is
  genuinely new; if it proves substantial (indexing, precedence), consider
  splitting it under epic 0146.

- 🔵 **Clarity**: Full-sync concept terms used without a definition or link
  **Location**: Requirements
  "Conflict dossiers", "dirty-overwrite guard", "blast-radius bounds",
  "baseline-last resumability", `pushed-unsynced`, and `remote-absent` are used
  as known terms without a gloss or link to where full-sync behaviour is
  defined.

- 🔵 **Clarity**: "The sync skill" accepts targets, but Technical Notes locate
  the logic in the engine
  **Location**: Requirements
  Requirement 1 attributes target acceptance to the skill, while Technical Notes
  place the target set in the `accelerator work sync` engine with the skill
  confined to parsing/gating/rendering.

### Strengths

- ✅ Summary follows complete user-story form (role, want, benefit) and names
  the beneficiary unambiguously; Context motivates rather than restates.
- ✅ Every expected section is genuinely populated — Open Questions,
  Dependencies, Assumptions, Technical Notes are substantive, not placeholders —
  and frontmatter is valid (kind=story, status=ready, parent/relates_to
  linkage).
- ✅ Scope is a single coherent capability with explicit in/out boundaries
  (whole-set preserved when no target; discovery deliberately out of scope) and
  is bounded by reusing full-sync semantics rather than re-specifying them.
- ✅ Most acceptance criteria are observable Given/When/Then pairs with strong
  negative assertions ("no other local item is written", "aborts before any
  write"); AC5 pins verification to a machine-observable report line.
- ✅ The term "target" is defined inline with its three accepted forms, and
  Assumptions/Drafting Notes surface the author's interpretations, reducing
  reader guesswork.

### Recommended Changes

1. **Decide the dual-match precedence rule and reconcile it across sections**
   (addresses: dual-match token undefined; `ambiguous` settled vs reopened)
   Resolve Open Question 2, then make Requirement 8 explicit about whether a
   both-match token aborts as ambiguous or resolves by precedence, and add an
   acceptance criterion pinning the expected outcome.

2. **Add acceptance criteria for the unverified full-sync parity behaviours**
   (addresses: Requirements behaviours have no verifying criterion)
   Add criteria for conflict dossiers, blast-radius bounds / max-pull-push caps,
   baseline advance/resumability, and `--resolve` on a named target.

3. **Anchor AC10 to an observable baseline** (addresses: `behaves as today` has
   no defined baseline)
   Reframe as "report/write set is identical to a captured golden run for the
   same fixture", or reference an existing full-sync acceptance test as the
   baseline.

4. **Clarify the dependency direction on 0213 and name the remote tracker**
   (addresses: 0213 may be an upstream blocker; remote tracker not named;
   undirected ordering with 0255/0229)
   Confirm whether 0213 provides resolution capability this story consumes and
   move it to "Blocked by" if so; name the external tracker as a coupling; note
   the direction (blocks / blocked-by / concurrent) for 0255 and 0229.

5. **Pin the target surface before `ready`, or mark it implementer discretion**
   (addresses: target-surface shape undecided)
   Decide positional vs repeatable-flag in Requirements, or state explicitly
   that the surface is an implementation detail with no pending external answer.

6. **Tidy terminology** (addresses: `resolve` overloaded; multiple whole-set
   names; full-sync terms undefined; skill vs engine actor)
   Adopt one canonical name for the whole-set behaviour, disambiguate "resolve",
   gloss or link the full-sync concept terms on first use, and align Requirement
   1's actor with the skill/engine split in Technical Notes.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is generally clear: it defines "target" inline
(local id, remote tracker id with an example, or path), consistently uses "sync"
as the acting subject, and its Requirements, Acceptance Criteria, and
Assumptions align on scope. The main clarity risks are terminology overloading —
"resolve" carries three distinct meanings — and a cluster of full-sync concept
terms used without a link to where they are defined. Several different names are
also used for the same whole-set behaviour.

**Strengths**:
- "Target" is defined explicitly with its three accepted forms, and "remote
  tracker id" is pinned to `external_id` with an example (`PP-787`).
- The acting subject "sync" is used consistently, and Drafting Notes resolve who
  "the user" is.
- Assumptions and Drafting Notes surface the author's interpretations, reducing
  guesswork.

**Findings**:
- 🔵 minor (confidence: medium) — **The word `resolve` carries three distinct
  meanings** (Requirements): "resolve" denotes mapping a target to a local file,
  the `accelerator work resolve` command, and the `--resolve` flag (purpose
  never stated, reads as conflict resolution). A reader cannot tell what
  `--resolve` controls. Suggestion: state what `--resolve` does, or rename one
  concept so each use has a single meaning.
- 🔵 suggestion (confidence: medium) — **Full-sync concept terms used without a
  definition or link** (Requirements): "conflict dossiers", "dirty-overwrite
  guard", "blast-radius bounds", "baseline-last resumability", `pushed-unsynced`,
  `remote-absent` appear as known terms. Suggestion: link to where full-sync
  behaviour is defined, or gloss the least self-evident on first use.
- 🔵 minor (confidence: medium) — **Multiple names used for the same whole-set
  behaviour** (Context): five labels for one concept. Suggestion: adopt one
  canonical term.
- 🔵 suggestion (confidence: low) — **"The sync skill" accepts targets, but
  Technical Notes locate the logic in the engine** (Requirements): Requirement 1
  attributes acceptance to the skill; the target set lives in the engine.
  Suggestion: align the Requirement's actor with the Technical Notes.
- 🔵 minor (confidence: low) — **Requirement treats `ambiguous` as settled while
  an Open Question reopens it** (Open Questions): Requirement 8 aborts on
  ambiguity; Open Question 2 reopens the both-match case. Suggestion: reconcile —
  define precedence now or make the deferral explicit.

### Completeness

**Summary**: The story is structurally complete and unusually well-populated:
every expected section contains substantive, kind-appropriate content, the
frontmatter is valid with a recognised kind and status, the user-story format is
properly applied, and the ten acceptance criteria are specific and mapped to the
requirements. The only completeness concern is that a fundamental interface
decision remains an Open Question while the item is marked ready.

**Strengths**:
- Summary follows complete user-story form and names the beneficiary
  unambiguously.
- Context motivates rather than restating the summary.
- Requirements enumerate resolution forms, per-item semantics, flag composition,
  and abort behaviour; acceptance criteria are numerous and specific.
- Optional sections are genuinely populated, not placeholders.
- Frontmatter is complete and valid.

**Findings**:
- 🔵 minor (confidence: medium) — **Target-surface shape left undecided in a
  ready item** (Requirements): the invocation surface (positional vs
  `--target`/`--only`) is captured only as an Open Question, yet
  `status: ready`. An implementer must resolve it before starting. Suggestion:
  pin the surface in Requirements before `ready`, or note it is implementation
  discretion.

### Dependency

**Summary**: The item extends the existing `accelerator work sync` engine, reuses
`accelerator work resolve`, and captures a Dependencies section naming a parent
epic and four related items. However, couplings are recorded as flat,
direction-less "relates to" entries, which hides at least one plausible upstream
ordering constraint (0213 overlaps this story's target resolution), and the
remote tracker whose API this story reads from and writes to is never named as
an external coupling.

**Strengths**:
- A Dependencies section is present and states "Blocked by: none" explicitly.
- Related items are enumerated with short descriptors.
- The reuse of `accelerator work resolve` and full-sync per-item semantics is
  stated explicitly, and the new remote-id lookup is flagged as new.

**Findings**:
- 🟡 major (confidence: medium) — **0213 (id-targeted resolve) may be an upstream
  blocker, not merely a relate** (Dependencies): if 0213 delivers the resolution
  capability this story consumes, it is a blocker, not a peer. Suggestion:
  confirm and move to "Blocked by" if so.
- 🔵 minor (confidence: medium) — **Remote tracker external system not named as a
  coupling** (Requirements): every push/pull criterion assumes an external
  tracker (the `PP-` prefix implies a vendor) that Dependencies never names.
  Suggestion: name the tracker and any rate-limit/availability constraints.
- 🔵 minor (confidence: low) — **Ordering with 0255 (chunk merge) and 0229 (pull
  scope) left undirected** (Dependencies): overlapping items are captured only as
  undirected "Relates to". Suggestion: note the direction of coupling or confirm
  no ordering constraint.

### Scope

**Summary**: A well-scoped story describing one coherent unit of work — narrowing
`/sync-work-items` to named targets. Every requirement serves that single
purpose, boundaries are explicit (full-set preserved when no target; discovery
out of scope), and sizing fits a story because the net-new work sits atop reused
engine semantics rather than reimplementing them.

**Strengths**:
- Summary, Requirements, and Acceptance Criteria describe the same scope with no
  drift.
- Boundaries are stated explicitly (in and out of scope).
- Scope is bounded by reusing full-sync per-item semantics, keeping the increment
  atomic.
- The multi-target framing remains a single coherent capability.

**Findings**:
- 🔵 suggestion (confidence: low) — **Remote-id (`external_id`) resolution is the
  one net-new mechanism worth confirming as part of this unit** (Requirements):
  corpus-wide `external_id` lookup is the one place scope grows beyond reuse.
  Suggestion: confirm it is small enough; if substantial (scanning, indexing,
  precedence), split it under epic 0146.

### Testability

**Summary**: The acceptance criteria are largely strong — most are clear
Given/When/Then pairs with observable, negative-assertable outcomes. The main
gaps are a reference-based regression criterion ("behaves as today") with no
concrete baseline, several Requirements-level behaviours that no criterion
verifies, and an undefined expected outcome for the dual-match resolution case
still open in Open Questions.

**Strengths**:
- Most criteria use unambiguous precondition/action/outcome framing with strong
  negative assertions.
- AC5 pins verification to a machine-observable artefact — the report's discovery
  line reading "skipped".
- AC6 makes abort-before-side-effect testable via zero writes and named offending
  targets.

**Findings**:
- 🟡 major (confidence: medium) — **`behaves as today` criterion has no defined
  baseline** (Acceptance Criteria): AC10 has no captured baseline, so a verifier
  cannot produce a definitive pass/fail against the whole-set path. Suggestion:
  anchor to a captured golden run or an existing full-sync acceptance test.
- 🟡 major (confidence: medium) — **Several Requirements behaviours have no
  verifying criterion** (Requirements): conflict dossiers, blast-radius bounds,
  baseline-last resumability, `--resolve`, and max-pull/push caps have no
  criterion. Suggestion: add criteria for conflict dossiers, max-push caps, and
  baseline advance/resumability.
- 🔵 minor (confidence: medium) — **Expected outcome for dual-match token is
  undefined** (Open Questions): the most error-prone resolution scenario has no
  specified pass/fail, and AC6 does not say whether such a token is ambiguous.
  Suggestion: once precedence is decided, add a criterion pinning the outcome.

## Re-Review (Pass 2) — 2026-09-06

**Verdict:** REVISE

Every finding from pass 1 is resolved. The verdict stays REVISE only on the
major-count threshold — exactly two new majors surfaced, both a direct
consequence of the pass-1 edits: the added parity criteria define their outcome
as "as in a full sync" without pinning that to a concrete artefact, and the
newly explicit engine-reuse framing makes epic 0146 (which owns the full-sync
engine) an implicit prerequisite that the Dependencies section never reasons
about the way it now reasons about 0213. Both are cheap to close.

### Previously Identified Issues

- 🟡 **Testability**: `behaves as today` criterion has no defined baseline —
  Resolved (AC anchored to a captured golden run / existing full-sync test).
- 🟡 **Testability**: Several Requirements behaviours have no verifying
  criterion — Resolved (criteria added for conflict dossier, max-pushes,
  `--resolve`, baseline/resume), though see new "as in a full sync" major.
- 🟡 **Dependency**: 0213 may be an upstream blocker — Resolved (reasoned
  explicitly as a peer, not a blocker).
- 🔵 **Completeness**: Target-surface shape undecided — Resolved (repeatable
  `--target` flag pinned in Requirements).
- 🔵 **Testability**: Dual-match token outcome undefined — Resolved (dedicated
  local-wins criterion).
- 🔵 **Dependency**: Remote tracker not named — Resolved (Linear named with
  inherited SLA/rate-limit implications).
- 🔵 **Dependency**: Ordering with 0255/0229 undirected — Partially resolved
  (marked concurrent; new minor asks to confirm the shared pull/discovery path).
- 🔵 **Clarity**: `resolve` carries three meanings — Resolved (explicit
  disambiguation of the `--resolve` flag).
- 🔵 **Clarity**: Multiple names for the whole-set behaviour — Resolved (canonical
  "full sync" / "targeted sync" throughout).
- 🔵 **Clarity**: Requirement 8 vs Open Question 2 on "ambiguous" — Resolved
  (both-match is not a failure; local wins).
- 🔵 **Clarity**: Skill-vs-engine actor — Resolved (Requirement 1 splits skill
  parsing from engine-owned target set).
- 🔵 **Clarity**: Full-sync terms undefined — Partially resolved (cross-ref to
  0146 added; still a suggestion to gloss "baseline-last resumability").
- 🔵 **Scope**: Confirm `external_id` lookup stays small — Resolved (kept
  self-contained as a conscious, justified choice).

### New Issues Introduced

- 🟡 **Testability**: Per-item criteria defer expected outcome to unspecified "as
  in a full sync" behaviour, with no captured artefact to compare against (unlike
  the no-target golden-run criterion).
- 🟡 **Dependency**: Epic 0146 owns the full-sync engine this story reuses
  unchanged, yet it is filed as a concurrent peer rather than reasoned about as a
  prerequisite; if the engine is not shipped, the story is silently blocked.
- 🔵 **Testability**: `--max-pulls` has no verifying criterion (only
  `--max-pushes` does).
- 🔵 **Testability**: Baseline-advance and interrupted-resume bundled into one
  criterion with different setups and failure signatures.
- 🔵 **Dependency**: 0229 (pull scope) and 0255 (chunk merge) touch the same
  engine pull/discovery path as this story's discovery suppression; strict
  concurrency may hide an integration-ordering coupling.

### Assessment

The work item is materially stronger and all pass-1 concerns are addressed. The
two remaining majors are narrow and mechanical: pin the "as in a full sync"
criteria to the same byte-identical golden-run comparison already used for the
no-target case, and add one line confirming the 0146 full-sync engine is shipped
and reused. With those closed, the item clears to APPROVE.

## Re-Review (Pass 3) — 2026-09-06

**Verdict:** COMMENT

Both pass-2 majors and all three pass-2 minors are resolved, and the one open
product question (a path target outside the work directory) is now decided —
reject and abort — with a matching Requirement and criterion. The verdict
crosses from REVISE to COMMENT: one major remains, below the two-major
threshold. That major and its related minor are cheap to close and would clear
the item to APPROVE.

### Previously Identified Issues

- 🟡 **Testability**: Per-item criteria defer to unspecified "as in a full sync"
  — Resolved (AC preamble pins a byte-identical report + write-set comparison
  over the same fixture).
- 🟡 **Dependency**: 0146 not reasoned as prerequisite — Resolved (Dependencies
  now states the full-sync engine is shipped under 0146 and reused; a completed
  prerequisite, not an open blocker).
- 🔵 **Testability**: `--max-pulls` unverified — Resolved (mirror criterion
  added).
- 🔵 **Testability**: baseline-advance and interrupted-resume bundled — Resolved
  (split into two criteria).
- 🔵 **Dependency**: 0229/0255 shared pull path undirected — Resolved
  (integration watch-out added).
- 🔵 **Clarity**: "work corpus" undefined — Resolved (now "all local work items
  in the work directory").
- ❓ **Open Question**: outside-work-dir path — Resolved (reject/abort; Requirement
  and criterion added; Open Questions now "None").

### New Issues Introduced

- 🟡 **Testability**: The unlinked-both-sides-exist and remote-absent cases are
  named in Requirements/Assumptions as "handled as in a full sync" but have no
  verifying criterion; the unsynced/`pushed-unsynced` case is covered, these two
  are not.
- 🔵 **Testability**: The byte-identical preamble may conflict with the
  discovery-line "skipped" criterion — a full sync that found nothing would not
  emit "skipped", so the comparison scope (per-item vs run-level lines) needs
  stating.
- 🔵 **Clarity**: The Summary's "pull a specific few" could be read as pulling
  remote-only issues with no local counterpart, which the design forbids;
  clarify that targeted pull reaches only locally-tracked items.

### Assessment

The work item is implementation-ready in substance: complete sections, mapped
dependencies, single coherent scope, and eighteen concrete criteria with a
mechanical pass condition. The remaining major is narrow — two missing criteria
for cases the Requirements already specify — and closing it plus the two minors
(comparison-scope wording, Summary "pull" clarification) would take the item to
APPROVE. All remaining items are low-severity polish; none blocks `ready`.

## Approval — 2026-09-06

**Verdict:** APPROVE

After the pass-3 edits, the last major (missing unlinked and remote-absent
criteria) and both minors (per-item comparison scope, Summary "pull"
clarification) are closed. Approved by verdict override without a fourth lens
pass; the remaining six suggestions are planning-time polish and none blocks
implementation.
