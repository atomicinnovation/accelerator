---
type: "work-item-review"
id: "0201-in-process-section-diff-review-1"
title: "Work Item Review: In-Process Section Diff"
date: "2026-08-30T01:08:22+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0201"
work_item_id: "0201"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-08-30T07:47:51+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: In-Process Section Diff

**Verdict:** COMMENT

This is a strong, well-scoped task: every section is present and substantively
populated, the single purpose (swap a subprocess `diff -u` for an in-process
Rust implementation and clean up the artefacts that exist only because of it)
holds across all six requirements, and dependencies are correctly mapped as
none-blocking. The one theme worth attention is the diff-body output contract:
because byte-for-byte GNU parity is deliberately waived and only the header
framing is frozen, the body's actual shape is left self-referential — testable
only against whatever the implementation emits. The work item is acceptable
as-is; the findings below would sharpen it before implementation but none block.

### Cross-Cutting Themes

- **Underspecified diff-body output contract** (flagged by: testability,
  dependency) — Testability flags that the body has no concrete expected text
  and "unified-diff-style" has no positive spec, so the core behaviour is
  self-referential. Dependency flags the same gap from the consumer side: the
  assumption that the body is an internal contract rests on no enumerated list
  of callers that parse or grep it. Both point at one fix — pin the body's
  shape with a worked example and confirm who consumes it.

### Findings

#### Critical

None.

#### Major

- 🟡 **Testability**: Diff body output has no concrete expected text; contract is self-referential
  **Location**: Acceptance Criteria
  AC1 requires "a unified-diff-style body" and AC4 asks tests to "assert the
  new implementation's own output contract against fixed expected text", but no
  concrete expected diff body is specified anywhere. With GNU parity waived and
  only the header framing frozen, whatever the implementation emits becomes the
  contract — the criterion can always be claimed met.

#### Minor

- 🔵 **Clarity**: Component under change is named inconsistently across sections
  **Location**: Acceptance Criteria
  The one component is called "the section-diff renderer" (AC1), "the
  in-process implementation" (AC2), "the renderer" (AC3), and named via
  `render`/`render_with`/`run_capped` (Technical Notes). A reader must infer
  they all denote the same thing.

- 🔵 **Clarity**: Summary states pup.ron rule removal more definitively than later sections
  **Location**: Summary
  The Summary lists removing the quarantine rule as a definite outcome, whereas
  Context, Requirements, Acceptance Criteria, and Open Question 2 all treat
  removal-versus-narrowing as unresolved.

- 🔵 **Dependency**: Downstream consumers of the diff-body format are assumed-away rather than enumerated
  **Location**: Assumptions
  The item rests body-format freedom on an Assumption that the body is an
  internal contract, but names no specific consumers of `accelerator work diff`
  output beyond `section_diff.rs`. The coupling surfaces as unplanned scope
  widening if a caller does parse it.

- 🔵 **Testability**: "unified-diff-style body" lacks a defined positive specification
  **Location**: Acceptance Criteria
  AC1's "unified-diff-style body" sets no measurable threshold; with hunk
  formatting explicitly not required to match GNU diffutils, what the body must
  positively contain (`-`/`+` prefixes, `@@` headers, or neither) is never
  pinned down.

- 🔵 **Testability**: DiffUnavailable / render signature resolution has no verifying criterion
  **Location**: Requirements
  The Requirements ask the implementer to decide whether `render` becomes
  infallible or keeps a narrower error type, but no Acceptance Criterion states
  the expected outcome or how to confirm the spawn/timeout failure mode is gone.

#### Suggestions

- 🔵 **Scope**: Tentative parent linkage to epic 0174
  **Location**: Frontmatter: parent
  The item declares `parent: work-item:0174`, but Drafting Notes concede 0174
  "doesn't literally scope this item today". If 0174's decomposition does not
  enumerate this migration, the linkage inflates the epic's child set with work
  its scope statement does not cover.

### Strengths

- ✅ Every expected section is present and substantively populated — Summary,
  Context (with the *why*), specific Requirements, six Given/When/Then
  Acceptance Criteria, and genuinely populated Open Questions, Assumptions, and
  Technical Notes.
- ✅ Single coherent purpose: all six requirements are consequences of the one
  change, not independent concerns bundled together; boundaries stay confined
  to `work-adapters` plus one `cli/pup.ron` rule.
- ✅ Kind selection (`task` over `story`) is explicitly justified in Drafting
  Notes for an internal swap with no user-visible behaviour change.
- ✅ Dependencies are well-mapped: the one upstream prerequisite (the 0170
  port) is named and correctly identified as complete; the new `similar` crate
  and its cargo-deny interaction are captured for implementation-time checks.
- ✅ Several criteria bind to mechanically verifiable outcomes — no
  `std::process::Command`, no `diff` on PATH, `pup:check` and the full CI
  mirror passing — and the header contract is quoted literally.
- ✅ Deferred decisions (module rename, infallible-vs-narrower error, delete-vs
  -narrow the pup rule) are signposted as intentional implementation-time
  judgment, not accidental ambiguity.

### Recommended Changes

1. **Pin the diff-body output with a worked example** (addresses: Diff body
   output has no concrete expected text; "unified-diff-style body" lacks a
   positive specification) — Add one small two-section input to the Acceptance
   Criteria with the exact expected rendered output (header plus hunk lines),
   and state the minimum structural elements the body must contain (e.g. per-
   line `-`/`+` prefixes and at least one `@@`-style hunk header). This turns
   the self-referential contract into a concrete golden target.

2. **Make the consumer scan explicit** (addresses: Downstream consumers of the
   diff-body format are assumed-away) — Either enumerate the known consumers of
   the section-diff body in Dependencies/Assumptions, or state that a consumer
   scan is a required first implementation step, so the coupling is visible
   before work starts rather than surfacing as scope widening.

3. **Add a criterion for the DiffUnavailable resolution** (addresses:
   DiffUnavailable / render signature resolution has no verifying criterion) —
   Add an Acceptance Criterion asserting the observable outcome, e.g. "the
   `render` signature no longer returns a `DiffUnavailable` variant" or "the
   only remaining error variant is X", so the failure-mode removal is
   confirmable.

4. **Soften the Summary's pup.ron claim and unify the component name**
   (addresses: Summary states pup.ron rule removal more definitively;
   Component under change is named inconsistently) — Change the Summary to the
   hedged "remove or simplify" language used elsewhere, and pick one noun for
   the component (e.g. "the section-diff renderer") used across all Acceptance
   Criteria, mapped once to `render` in Technical Notes.

5. **Confirm or drop the 0174 parent linkage** (addresses: Tentative parent
   linkage to epic 0174) — Either confirm 0174's decomposition enumerates this
   migration (and adjust the epic), or drop `parent` and keep the connection as
   a `relates_to` architectural-precedent link, as already done for
   0170/0188/0198.

## Per-Lens Results

### Clarity

**Summary**: The work item is unusually clear and internally consistent: an
imperative Requirements list with a named implementer-actor, Given/When/Then
Acceptance Criteria whose subjects are explicit, and domain terms that are
either standard or defined in context. The only clarity frictions are minor:
the component being changed is named several different ways across sections,
and the Summary states the pup.ron rule's removal more definitively than the
hedged "remove or simplify" language everywhere else. Neither forces a genuine
misinterpretation for a reader who knows the domain.

**Strengths**:
- Every acronym or domain term (DiffUnavailable, bash-parity feature, GNU
  diffutils, Myers/Patience/Histogram diff algorithms, cargo-deny allow-list,
  pup.ron rule name) is either defined at first use in Context or is standard
  vocabulary within the project's known Rust/toolchain domain.
- Requirements are written in the imperative with a single implied actor and
  concrete, observable outcomes, leaving no passive-voice ambiguity about who
  does what.
- Deferred decisions are explicitly signposted as implementation-time judgment
  in Open Questions and Requirements, so their openness is stated intent.

**Findings**:
- 🔵 minor / medium — **Component under change is named inconsistently across
  sections** (Location: Acceptance Criteria). The single component is referred
  to as "the section-diff renderer" (AC1), "the in-process implementation"
  (AC2), "the renderer" (AC3), and via `render`/`render_with`/`run_capped`
  (Technical Notes). Impact: the varied naming forces a small resolution step
  and could momentarily read as if AC2's artefact is distinct from AC1's.
  Suggestion: pick one consistent noun and use it across all Acceptance
  Criteria, mapping it once to the `render` entry point.
- 🔵 minor / medium — **Summary states pup.ron rule removal more definitively
  than later sections** (Location: Summary). The Summary lists removing the
  quarantine rule as a definite outcome, whereas Context ("removed or
  simplified"), Requirements ("Remove or simplify"), Acceptance Criteria
  ("removed or updated accordingly"), and Open Question 2 all treat
  removal-versus-narrowing as unresolved. Impact: a reader taking the Summary
  at face value may believe outright removal is committed. Suggestion: soften
  the Summary to match the hedged language used elsewhere.

### Completeness

**Summary**: This task is exceptionally complete for its kind: every expected
section is present and substantively populated, with a clear Summary, a Context
that fully explains the motivation, specific Requirements, six concrete
Acceptance Criteria, and populated Open Questions, Assumptions, and Technical
Notes. The frontmatter is intact with a recognised `kind: task`, a valid
`status: draft`, and priority set. No completeness gaps rise to the level of a
finding.

**Strengths**:
- Summary states the work as a single unambiguous action and names the concrete
  payoffs (removing the external process dependency, the spawn/timeout failure
  mode, and the quarantine rule).
- Context thoroughly explains why the work is needed and identifies exactly
  what contract must be preserved versus what can change.
- Requirements are specific and implementer-ready, enumerating each concrete
  change.
- Acceptance Criteria contains six specific criteria in given/when/then form,
  well above the minimum expected for a task.
- Kind-appropriate for a task; Drafting Notes explicitly justify `task` over
  `story`.
- Open Questions and Assumptions are genuinely populated rather than empty
  placeholders.

**Findings**: None.

### Dependency

**Summary**: This task is well dependency-mapped: it explicitly declares no
blocking or blocked work items, correctly notes that its one upstream
prerequisite (the 0170 port) is already complete, and captures the new
third-party crate dependency plus its cargo-deny verification in Assumptions and
Technical Notes. The only residual coupling is downstream — the set of consumers
that read the diff body format — which is acknowledged as an assumption to
verify rather than enumerated.

**Strengths**:
- The single upstream prerequisite (the 0170 diff port) is explicitly named and
  correctly identified as already complete.
- The new external crate dependency (`similar`, with `imara-diff`/`diffy`
  alternatives) is captured in Technical Notes, and its cargo-deny interaction
  is flagged in Assumptions.
- The internal consumer `cli/work/src/section_diff.rs` (and the `accelerator
  work diff` surface) is named, and removal of the pup.ron isolation rule is
  tied to confirming no other subprocess remains.

**Findings**:
- 🔵 minor / medium — **Downstream consumers of the diff-body format are
  assumed-away rather than enumerated** (Location: Assumptions). The item
  changes the internal diff-body formatting and rests this on an Assumption that
  the body is an internal contract, not a byte-for-byte frozen format some
  downstream skill parses or greps. No specific consumers beyond
  `section_diff.rs` are named. Impact: if a caller does parse the body, the
  coupling surfaces at implementation time as unplanned scope widening.
  Suggestion: enumerate the known consumers of the section-diff body, or state
  that a consumer scan is a required first step.

### Scope

**Summary**: This is a well-scoped, atomic task: every requirement serves the
single unified purpose of replacing one subprocess-based section-diff with an
in-process implementation and cleaning up the artefacts (parity feature,
subprocess-specific tests, quarantine rule) that exist only because of that
subprocess. The declared kind `task` fits an internal implementation swap with
no user-visible behaviour change, and the boundaries are crisp — one crate plus
its architecture-enforcement rule, with no cross-service spread. The only
scope-relevant wrinkle is the tentative parent linkage to epic 0174.

**Strengths**:
- Single coherent purpose: all six requirements are consequences of the one
  change, not independent concerns bundled together.
- Kind selection is justified explicitly in Drafting Notes and matches the
  scope described.
- Clear in/out-of-scope boundaries: byte-for-byte GNU parity is deliberately
  out of scope while the header framing contract is in scope.
- No cross-service or cross-team spread — work confined to `work-adapters` and
  a single `cli/pup.ron` rule, with Dependencies correctly recording
  blocked-by/blocks as none.

**Findings**:
- 🔵 suggestion / low — **Tentative parent linkage to epic 0174** (Location:
  Frontmatter: parent). The item declares `parent: work-item:0174`, but Drafting
  Notes concede 0174 "doesn't literally scope this item today" and asks to
  confirm or drop the linkage. Impact: a child the parent epic does not actually
  scope blurs the epic's decomposition boundary. Suggestion: either confirm
  0174's scope enumerates this migration, or drop `parent` and keep it as a
  `relates_to` link.

### Testability

**Summary**: Most acceptance criteria are framed as concrete Given/When/Then
pairs anchored to CI-verifiable or grep-able outcomes (no
`std::process::Command`, empty diff for identical sections, no diff binary on
PATH, pup:check and mise run passing), which gives a verifier clear pass/fail
procedures. The main weakness is that the central behaviour — the diff body's
actual output — has no concrete expected text: it is deferred to "the new
implementation's own output contract", making the core criterion partly
self-referential. A secondary gap is that the requirement to resolve the
DiffUnavailable/render-signature question has no acceptance criterion verifying
its outcome.

**Strengths**:
- AC3 (identical sections yield an empty diff body) is a fully specified,
  unambiguous pass/fail behaviour.
- Several criteria bind to mechanically verifiable outcomes — absence of
  `std::process::Command`, no diff binary required on PATH, mise run pup:check
  and the full CI mirror passing.
- The header/framing contract to preserve is quoted literally, giving a tester
  an exact string to assert against.
- Criteria are consistently expressed as observable behaviours rather than
  implementation instructions, appropriate for a task-kind item.

**Findings**:
- 🟡 major / medium — **Diff body output has no concrete expected text;
  contract is self-referential** (Location: Acceptance Criteria). AC1 requires
  "a unified-diff-style body" and AC4 asks tests to assert "the new
  implementation's own output contract against fixed expected text", but no
  concrete expected diff body is specified. Because GNU parity is waived and
  only the header line is fixed, a verifier has no independent expected value —
  whatever the implementation emits becomes the contract. Impact: the single
  most behaviour-defining criterion cannot be tested against a pre-agreed
  expectation. Suggestion: add at least one worked example — a small two-section
  input with the exact expected rendered output.
- 🔵 minor / medium — **"unified-diff-style body" lacks a defined positive
  specification** (Location: Acceptance Criteria). AC1's phrase sets no
  measurable threshold; with hunk formatting explicitly not required to match
  GNU diffutils, what the body must positively contain is never pinned down.
  Impact: a tester cannot definitively decide whether a body is
  "unified-diff-style enough". Suggestion: state the minimum structural elements
  the body must contain.
- 🔵 minor / medium — **DiffUnavailable / render signature resolution has no
  verifying criterion** (Location: Requirements). The Requirements ask the
  implementer to decide whether `render` becomes infallible or keeps a narrower
  error type, but no Acceptance Criterion states the expected outcome or how to
  verify the spawn/timeout failure mode is gone. Impact: "done" for this
  requirement is unverifiable. Suggestion: add a criterion asserting the
  observable outcome.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-08-30

**Verdict:** COMMENT

Every finding from the initial review is resolved. The re-run of the four
lenses that had findings (clarity, dependency, scope, testability) surfaced no
critical or major issues — a handful of minor/suggestion observations remain,
most of them pre-existing latent wording, plus two byproducts of the fixes
(proving-a-negative phrasing on the new consumer-scan gate and the fallible
branch of the new DiffUnavailable criterion). The work item is ready for
planning; the residual observations are optional polish.

### Previously Identified Issues

- 🟡 **Testability**: Diff body output has no concrete expected text — Resolved. A golden-example criterion now pins the full rendered output to exact expected text plus required structural markers.
- 🔵 **Clarity**: Component named inconsistently — Resolved. Unified to "section-diff renderer" across the Acceptance Criteria.
- 🔵 **Clarity**: Summary overstates pup.ron rule removal — Resolved. Summary now reads "removed or simplified", matching the hedged language elsewhere.
- 🔵 **Dependency**: Downstream consumers assumed-away — Resolved. A consumer scan is now an explicit first requirement with a widen-scope fallback.
- 🔵 **Testability**: "unified-diff-style body" lacks a positive spec — Resolved. The golden criterion now names the required `-`/`+` prefixes and `@@`-style hunk header.
- 🔵 **Testability**: DiffUnavailable resolution has no verifying criterion — Resolved (with residual). A criterion asserting `render` no longer surfaces `DiffUnavailable` was added; testability notes its fallible branch could still be tightened (see below).
- 🔵 **Scope**: Tentative 0174 parent linkage — Resolved. `parent` dropped; 0174 moved to `relates_to`, Drafting Note rewritten.

### New Issues Introduced

- 🔵 **Testability** (minor): AC4's alternate branch ("only remaining error variant reflects a distinct failure mode") has no verifiable success condition — the negative half (DiffUnavailable removed) is checkable, the fallible branch is not. Byproduct of the added criterion.
- 🔵 **Testability** (suggestion): The consumer-scan requirement's "confirm none depend" is a proving-a-negative with no recorded deliverable. Byproduct of the added requirement.
- 🔵 **Scope** (suggestion): The consumer-scan requirement's "widen scope to accommodate them" leaves the unit of work conditionally open-ended; consider splitting any accommodation into a follow-up item. Byproduct of the added requirement.

### Pre-Existing Observations (newly surfaced, not introduced by the edits)

- 🔵 **Clarity** (minor): Summary calls the module "the crate's one external process dependency" as settled fact, while Open Question 2 leaves the sole-subprocess claim unconfirmed.
- 🔵 **Dependency** (suggestion): The cargo-deny allow-list gate for the new diff crate lives in Assumptions but the Dependencies section still reads "Blocked by: none."
- 🔵 **Testability** (suggestion): AC8 ("no `diff` binary required anywhere") proves a negative without a stated procedure — unlike AC6, which pins to `cargo test -p work-adapters`.
- 🔵 **Clarity** (suggestion): Context describes the vcs shell-out pattern in present tense while Drafting Notes cite 0188/0198 as later migrations of it.

### Assessment

The work item is ready for implementation planning. The initial review's one
material weakness — an untestable, self-referential diff-body contract — is
fully closed by the golden-example criterion. The remaining observations are
minor wording and verification-procedure refinements; none blocks planning, and
several (the cargo-deny visibility, the AC8 procedure, tightening AC4's
fallible branch) can be folded in during planning or left as implementation
judgment.
