---
type: "work-item-review"
id: "0300-sync-pulls-of-tracked-work-items-fail-silently-review-1"
title: "Work Item Review: Sync Pulls of Tracked Work Items Fail Silently"
date: "2026-10-10T22:05:08+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:0146"
target: "work-item:0300"
work_item_id: "0300"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 5
tags: []
last_updated: "2026-10-10T23:22:46+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Sync Pulls of Tracked Work Items Fail Silently

**Verdict:** REVISE

The bug report is strong on diagnosis: a precise causal chain, an exact
reproduction with observable expected and actual states, and observed
evidence from a real run. It falls short where the fix meets its
surroundings. The acceptance criteria are narrower than the requirements
and can pass for the wrong reason, the message channel is described two
ways, and the item omits that making pulls succeed activates 0296's
title-line corruption.

### Cross-Cutting Themes

- **Classification scope mismatch** (flagged by: testability, scope,
  clarity) — the requirement classifies "every apply failure", but AC3,
  the Summary and the Open Question cover only a pull's local write.
- **Message channel ambiguity** (flagged by: clarity, testability) — the
  requirement allows stderr "or the report detail", while AC3 demands
  stderr and reserves "detail" for the class.
- **Pull reconstruction is assumed correct** (flagged by: dependency,
  clarity) — the Assumption contradicts 0296 Defect 2 and understates
  the reporting-contract change.
- **Dossier pruning left open in-item** (flagged by: scope, dependency) —
  unrelated to this defect; should be split out and linked to 0257.

### Findings

#### Major

- 🟡 **Dependency**: Ordering with 0296 (projected title line in pulls)
  not captured
  **Location**: Dependencies
  0296 Defect 2 reconstructs pulled content from Linear's projected body,
  putting a title line above the H1 that the next push sends back. Today
  every tracked pull fails before writing; once 0300 lands, that
  corruption becomes live for every remote edit.
- 🟡 **Testability / Scope / Clarity**: Requirement to classify every
  apply failure is tested only for local pull writes
  **Location**: Acceptance Criteria
  A fix that classifies only `write-local` passes every criterion while
  baseline, marker and other I/O failures still render `-` and exit 0.
- 🟡 **Testability**: Non-zero exit check can pass because conflicts are
  pending
  **Location**: Acceptance Criteria
  The original bug hid behind exit 4 from pending conflicts; AC3's
  "non-zero" would pass the same way unless the failure is the run's
  only problem.
- 🟡 **Clarity / Testability**: "Report detail" means two things; AC3 and
  the requirement name different channels
  **Location**: Requirements
  An implementation putting the message in the detail column meets the
  requirement and fails AC3.

#### Minor

- 🔵 **Testability**: Containment criterion has no defined input or
  observable result, and checks only one direction
  **Location**: Acceptance Criteria
  AC4 does not say how an out-of-bounds pull arises or what "refused"
  looks like; no criterion holds baseline writes inside the integration
  state directory, which a widened store could breach.
- 🔵 **Testability / Clarity**: Resolve-to-remote outcome is loosely
  stated and worded differently from AC1
  **Location**: Acceptance Criteria
  AC2 says "overwritten from remote" and "reports the pull as applied";
  it omits the baseline, the dossier and the frontmatter rule AC1 states.
- 🔵 **Clarity**: Report row columns named loosely and inconsistently
  **Location**: Requirements
  "The row reads `pull`", "the row's action" and "reports the pull as
  applied" refer to unnamed columns.
- 🔵 **Clarity**: "Work directory" and "working copy" undefined
  **Location**: Requirements
  The containment boundary depends on which root is meant; a repo-root
  reading reproduces the over-wide root the Technical Notes criticise.
- 🔵 **Clarity**: "Only the writer wiring is wrong" conflicts with the
  reporting defect
  **Location**: Assumptions
  Two of three requirements address the separate reporting contract.
- 🔵 **Completeness**: `--resolve <id>=remote` symptom has no reproduction
  **Location**: Requirements
  The Summary claims it and AC2 covers it, but no steps or observed
  output are given.
- 🔵 **Dependency**: Linear tracker not named as an external dependency
  **Location**: Dependencies
  End-to-end verification needs a reachable tracker and a remotely
  edited item.
- 🔵 **Dependency**: `sync-work-items` skill not listed as a consumer of
  the changed failure-class and exit-code contract
  **Location**: Requirements
  The skill reads sync reports and exit codes; 0285 had to update it for
  a similar change.

#### Suggestions

- 🔵 **Scope / Dependency**: Move the dossier-pruning question out
  **Location**: Open Questions
  Record it as its own item related to 0257, or state it is out of scope.
- 🔵 **Scope**: Two separable fixes bundled
  **Location**: Requirements
  Acceptable together; split the reporting contract if the
  retryable-versus-terminal decision stalls the pull fix.
- 🔵 **Testability**: AC5 names a store type the fix may replace
  **Location**: Acceptance Criteria
  Phrase it as the same port wiring `work sync` builds, with real
  writers rooted as in production.
- 🔵 **Clarity**: Sync-domain terms used without definition
  **Location**: Context
  Gloss "baseline", "failure class" and "conflict dossier", or point to
  the 0194 documents.
- 🔵 **Completeness**: No environment or version details
  **Location**: Requirements
  Add the accelerator version or commit the bug was observed on.

### Strengths

- ✅ The Context gives a precise causal chain — writer port, mis-rooted
  store, containment refusal, unclassified `Io` error, `-` detail,
  exit 0 — each link naming its component.
- ✅ The reproduction states precondition, exact command, and expected
  and actual outcomes as observable states (row text, file and baseline,
  stderr, exit code), and rules out `--preview` and `--per-item-reads`.
- ✅ Observed evidence (ten named items against Linear, the exit-4
  explanation) grounds the report.
- ✅ AC1 is concrete, and AC5 targets the exact reason the bug escaped
  tests.
- ✅ Scope is deliberate: dossier pruning is parked with a reason, and
  the writer design choice is left to the implementer.
- ✅ The Technical Notes name the contracts that change
  (`ApplyError::class`, `render_report`, `exit_code_for_report`).

### Recommended Changes

1. **Capture the 0296 ordering and revise the Assumption** (addresses:
   Ordering with 0296; "Only the writer wiring is wrong")
   Make 0300 blocked by 0296 Defect 2, or require they ship together.
   Narrow the Assumption to the write path and acknowledge 0296's
   reconstruction defect.
2. **Settle the classification scope and widen AC3 to match** (addresses:
   classify every apply failure)
   Decide whether every `ApplyError` variant or only I/O failures are in
   scope; express it in both the requirement and the criteria.
3. **Pin AC3's exit code** (addresses: Non-zero exit check)
   Resolve the retryable-versus-terminal Open Question, require the write
   failure to be the run's only problem, and assert the specific code.
4. **Use one message channel and one meaning of "detail"** (addresses:
   "Report detail" means two things)
   E.g. detail column carries the class; stderr carries a line with item
   id, operation and cause.
5. **Name the row columns and restate AC1/AC2 against them** (addresses:
   row columns; resolve-to-remote outcome; resolve reproduction)
   Define id/action/state/detail once; give AC2 the same file, baseline
   and dossier outcomes as AC1, plus its own reproduction or a note that
   it fails through the same path.
6. **Make containment testable in both directions** (addresses:
   Containment criterion; "work directory" undefined; AC5 store type)
   Define the work directory as the configured work-items directory; add
   paired refusal criteria for the work-item and baseline writers; phrase
   AC5 by wiring rather than type.
7. **Complete Dependencies** (addresses: Linear tracker;
   `sync-work-items` consumer; dossier pruning)
   Add the tracker as external, the skill as a consumer, and split the
   dossier question into an item related to 0257.

## Per-Lens Results

### Clarity

**Summary**: Clear overall with a precise mechanism and reproduction;
residual ambiguity is terminological — "detail" carries two meanings,
pull outcomes and row columns are described inconsistently, and several
sync-domain terms are undefined.

**Strengths**:
- Precise causal chain naming each responsible component.
- Expected and actual behaviour stated as observable states.
- Exit 4 in the observed run is explained, avoiding a contradiction.
- Drafting Notes make implementer latitude and scope exclusions explicit.

**Findings**:
- 🟡 major / high — Requirements — "Report detail" means two things.
- 🔵 minor / medium — Acceptance Criteria — Sections describe a pull's
  result on the local file differently.
- 🔵 minor / medium — Requirements — Report row columns named loosely and
  inconsistently.
- 🔵 minor / medium — Requirements — "Work directory" and "working copy"
  left undefined.
- 🔵 minor / medium — Requirements — Classification requirement is broader
  than the pull-focused criteria.
- 🔵 minor / medium — Assumptions — "Only the writer wiring is wrong"
  conflicts with the reporting defect.
- 🔵 suggestion / medium — Context — Sync-domain terms used without
  definition or link.

### Completeness

**Summary**: Well-populated bug report; all sections substantive, the
reproduction complete. Minor gaps: no environment/version and no
reproduction for the `--resolve` symptom.

**Strengths**:
- Complete frontmatter.
- Four-part reproduction with exact row, stderr and exit code.
- Root-cause context backed by observed evidence.
- Requirements map to criteria, including a regression test.
- Open Questions, Assumptions and Drafting Notes substantive.

**Findings**:
- 🔵 minor / medium — Requirements — `--resolve <id>=remote` symptom has
  no reproduction steps.
- 🔵 suggestion / low — Requirements — No environment or version details.

### Dependency

**Summary**: The internal cause is articulated, but no dependencies are
recorded. The strongest uncaptured coupling is 0296, whose pull
reconstruction defect becomes live once pulls succeed; the tracker and
the `sync-work-items` skill are also absent.

**Strengths**:
- Explicit "Blocked by" and "Blocks" entries.
- Related items listed in frontmatter and References.
- Dossier pruning surfaced rather than silently absorbed.
- Technical Notes name the changing contracts.

**Findings**:
- 🟡 major / high — Dependencies — Ordering with 0296 not captured.
- 🔵 minor / high — Dependencies — Linear tracker not named as an
  external dependency.
- 🔵 minor / medium — Requirements — `sync-work-items` skill not listed
  as a consumer of the changed contract.
- 🔵 suggestion / medium — Open Questions — Dossier-pruning follow-on has
  no captured link to 0257.

### Scope

**Summary**: Well-bounded: one defect plus the reporting gap that hid
it, sharing the "fails silently" framing. Wrinkles: the classification
requirement exceeds its criterion, and the dossier question sits in-item.

**Strengths**:
- Deliberate scope boundary with rationale.
- Summary, Requirements and Criteria cover the same two concerns.
- Kind `bug` fits.
- Design choice left to the implementer.

**Findings**:
- 🔵 minor / medium — Requirements — Failure-classification requirement
  broader than its acceptance criterion.
- 🔵 suggestion / medium — Open Questions — Dossier-pruning question
  should be split out.
- 🔵 suggestion / low — Requirements — Two separable fixes bundled.

### Testability

**Summary**: The reproduction is fully verifiable and most criteria are
precondition/action/outcome checks, but AC3 covers less than the
requirement and its exit check can pass for the wrong reason, and the
containment criteria lack inputs and miss the baseline direction.

**Strengths**:
- Complete reproduction including ruled-out flags.
- AC1 has crisp pass/fail checks.
- AC5 targets the reason the bug escaped.

**Findings**:
- 🟡 major / high — Acceptance Criteria — Requirement to classify every
  apply failure tested only for local pull writes.
- 🟡 major / medium — Acceptance Criteria — Non-zero exit check can pass
  because conflicts are pending.
- 🔵 minor / medium — Acceptance Criteria — Containment criterion lacks
  input and observable, and checks only one direction.
- 🔵 minor / medium — Acceptance Criteria — AC3 requires stderr while the
  requirement allows the report detail.
- 🔵 minor / medium — Acceptance Criteria — Resolve-to-remote outcome
  loosely stated.
- 🔵 suggestion / low — Acceptance Criteria — Regression-test criterion
  names a store type the fix may replace.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-10-10T22:33:10+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Dependency**: Ordering with 0296 not captured — Partially resolved
  (recorded as "ships with", but targets one defect of a three-defect item
  and is not mirrored in 0296)
- 🟡 **Testability / Scope / Clarity**: Classification tested only for
  pull writes — Resolved (stderr check still pull-only; see new
  suggestion)
- 🟡 **Testability**: Non-zero exit can pass via pending conflicts —
  Resolved
- 🟡 **Clarity / Testability**: "Report detail" means two things —
  Resolved
- 🔵 **Testability**: Containment criterion one-directional — Resolved
- 🔵 **Testability / Clarity**: Resolve-to-remote outcome loosely stated —
  Partially resolved (AC2 fixed; Reproduction B's expected omits the
  dossier removal)
- 🔵 **Clarity**: Row columns named loosely — Resolved
- 🔵 **Clarity**: "Work directory" undefined — Resolved
- 🔵 **Clarity**: "Only the writer wiring is wrong" — Resolved
- 🔵 **Completeness**: No `--resolve` reproduction — Resolved
- 🔵 **Dependency**: Linear not named — Resolved
- 🔵 **Dependency**: `sync-work-items` consumer not listed — Partially
  resolved (listed, but no acceptance criterion)
- 🔵 **Scope / Dependency**: Dossier question left open — Resolved
  (brought into scope)
- 🔵 **Testability**: AC5 named a store type — Resolved
- 🔵 **Clarity**: Sync-domain terms undefined — Resolved
- 🔵 **Completeness**: No version details — Still present
- 🔵 **Scope**: Two separable fixes bundled — Still present (now three;
  accepted by the author)

### New Issues Introduced

- 🟡 **Clarity**: "Widened store" option contradicts the paired
  containment criteria — one store cannot refuse outside both roots.
- 🟡 **Dependency**: "Ships with 0296 (Defect 2)" points at part of a
  three-defect item that does not mirror the coupling.
- 🟡 **Testability / Completeness**: Skill-consistency requirement has no
  acceptance criterion.
- 🔵 **Testability**: Retry-safety claim unchecked — AC3 does not assert
  the baseline is unchanged or that a re-run succeeds.
- 🔵 **Testability**: AC3 does not name how the write failure is induced.
- 🔵 **Testability**: Orphaned dossiers (items a full run no longer
  classifies) have no defined outcome.
- 🔵 **Testability**: AC9 ambiguous about stubbing the tracker and what
  it asserts.
- 🔵 **Clarity**: "Classified" carries both failure-class and sync-state
  meanings; the deletion trigger lives only in the criteria.
- 🔵 **Clarity**: Exit codes 4, unconfigured/rejected codes,
  `--per-item-reads`, "apply mode" and 0296 Defect 2 undefined.
- 🔵 **Clarity**: Exit-code precedence for an I/O failure alongside
  pending conflicts unstated.
- 🔵 **Dependency**: Verification of 0296's pull criteria depends on this
  fix; not recorded.
- 🔵 **Dependency**: 0296's open list-marker question becomes live once
  pulls succeed.
- 🔵 Suggestions: dossier fix could be split so 0296 does not gate it
  (scope); note the all-failures exit-code blast radius (scope); dossier
  work overlaps 0296 Defect 3 (dependency); AC1/AC2 content check
  depends on 0296 (testability); stderr check only for `write-local`
  (testability); "when its class is taken" phrasing (clarity).

### Assessment

The four pass-1 majors are resolved or narrowed, and the item is
substantially stronger. Three new majors remain: the writer-design
latitude contradicts its own criteria, the 0296 coupling is one-sided and
imprecise, and the skill update has no criterion. All three are small
textual fixes, but the 0296 coupling needs a decision on whether to split
Defect 2 or couple to the whole of 0296.

## Re-Review (Pass 3) — 2026-10-10T22:42:08+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: "Widened store" contradicts paired containment — Resolved
- 🟡 **Dependency**: "Ships with 0296 (Defect 2)" one-sided and partial —
  Resolved (whole of 0296, mirrored in 0296)
- 🟡 **Testability / Completeness**: Skill requirement has no criterion —
  Partially resolved (criterion covers exit 70 only, not a retryable row
  under exit 4)
- 🔵 **Testability**: Retry safety unchecked — Resolved
- 🔵 **Testability**: Write-failure induction unspecified — Partially
  resolved (the read-only file may not fail an atomic temp-and-rename
  write)
- 🔵 **Testability**: Orphaned dossiers undefined — Resolved
- 🔵 **Testability**: AC9 tracker stubbing and assertions — Resolved
- 🔵 **Clarity**: "Classified" double meaning — Resolved
- 🔵 **Clarity**: Exit codes, apply mode, `--per-item-reads` undefined —
  Partially resolved ("error" in the precedence list is undefined)
- 🔵 **Clarity**: Exit-code precedence unstated — Resolved
- 🔵 **Clarity / Completeness**: Reproduction B omits dossier removal —
  Resolved
- 🔵 **Dependency**: 0296 verification ordering — Resolved
- 🔵 **Dependency**: 0296 list-marker question — Resolved
- 🔵 Suggestions: all-failures blast radius (resolved), 0296 Defect 3
  overlap (resolved), AC1/AC2 content dependency (resolved), stderr only
  for `write-local` (resolved), "when its class is taken" (resolved),
  version details (still present), dossier fix bundled (still present;
  now gated on 0296)

### New Issues Introduced

- 🟡 **Clarity**: Retryable rationale ("baseline is written last") covers
  only a failed `write-local`; a failed baseline write after the local
  write succeeded is not obviously safe to re-run.
- 🟡 **Testability**: Skill criterion misses a retryable row in a run that
  exits 4 because conflicts are pending — the observed 2026-10-05 shape.
- 🟡 **Testability**: A read-only target file may not make an atomic
  temp-and-rename write fail, or root ignores the mode; AC3/AC4 may be
  unsatisfiable as written.
- 🔵 **Testability**: Per-variant criterion accepts any class mapping;
  expected classes are unstated.
- 🔵 **Testability / Clarity**: A still-conflicted target's own dossier
  has no stated outcome; "leave every other dossier untouched" reads as
  forbidding its rewrite.
- 🔵 **Testability**: Containment criteria name no out-of-bounds input;
  the cross-root case may go unexercised.
- 🔵 **Clarity**: Which writer handles dossiers is unstated; "baseline
  writer" is named by one file rather than its root.
- 🔵 **Scope**: Coupling to all of 0296 makes the two one delivery unit.
- 🔵 **Dependency**: Other callers of `work sync` exit codes not ruled
  out; the consumer entry omits the stderr line and dossier lifecycle.
- 🔵 Suggestions: release gate on 0296 has no owner (dependency); skill
  criterion has no verification method (testability); "existing
  frontmatter" strictness (testability); unglossed terms — local-author
  port, sync state, `ApplyError` variants, 0296 Defect 3 (clarity);
  version details (completeness); dossier fix gated on 0296 (scope).

### Assessment

All three pass-2 majors are resolved or narrowed, and completeness is
clean. The three new majors sit at a finer grain: the retry-safety
rationale for post-write failures, the skill's handling of a masked
retryable row, and a fault-injection method that may not fail an atomic
write. The first needs a classification decision; the other two are
textual.

## Re-Review (Pass 4) — 2026-10-10T22:48:28+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: Retryable rationale covers only `write-local` — Partially
  resolved (pull path now split, but the two phrasings of the rule diverge
  for push and create paths)
- 🟡 **Testability**: Skill misses a retryable row under exit 4 — Resolved
- 🟡 **Testability**: Read-only file may not fail an atomic write —
  Resolved (non-writable directory)
- 🔵 **Testability**: Per-variant criterion accepts any mapping — Partially
  resolved (tracker-originated classes still implicit)
- 🔵 **Testability / Clarity**: Still-conflicted target dossier — Partially
  resolved (first-time conflicts on a targeted run unaddressed)
- 🔵 **Testability**: Containment inputs unspecified — Resolved
- 🔵 **Clarity**: Writer for dossiers unstated — Resolved
- 🔵 **Clarity**: "Error" undefined — Resolved
- 🔵 **Dependency**: Other `work sync` callers; consumer surfaces —
  Partially resolved (in-repo covered; external callers not)
- 🔵 **Scope**: Coupling makes one delivery unit — Still present
- 🔵 Suggestions: version details, dossier fix gated on 0296, release gate
  owner — Still present

### New Issues Introduced

- 🟡 **Testability**: AC5's non-writable integration-state directory may
  fail a write other than `baseline-set`, and AC5 does not assert the
  rewritten-file, unchanged-baseline state that defines "terminal".
- 🔵 **Clarity / Testability**: "Holds the remote body" contradicts the
  Assumption that 0296 verifies the exact body.
- 🔵 **Clarity**: Relationship of the local-author port to the two bounded
  writers unstated.
- 🔵 **Testability**: Skill criterion has no `terminal` (exit 71) case.
- 🔵 **Testability**: A targeted run whose target stopped conflicting
  without `--resolve` is unchecked.
- 🔵 **Dependency**: "Blocked by: none" understates the release gate; no
  owner or mechanism.
- 🔵 Suggestions: "including its open questions" ambiguous (clarity);
  0285's relationship unexplained (dependency); permission-based fault
  injection ineffective as root (testability).

### Assessment

Two majors remain, both on the retryable/terminal boundary: the rule needs
a single statement covering push and create paths, and the terminal-path
criterion needs a fault injection that targets `baseline-set` and asserts
the state that justifies the class. Completeness and scope carry only
minor items. The item is converging.

## Re-Review (Pass 5) — 2026-10-10T23:05:06+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Clarity / Testability**: Classification rule diverges off the pull
  path — Resolved (single rule plus per-operation table)
- 🟡 **Testability**: AC5 fault injection may miss `baseline-set` —
  Resolved (wrapped writer; asserts the terminal state)
- 🔵 **Clarity / Testability**: "Holds the remote body" vs the 0296
  Assumption — Resolved
- 🔵 **Clarity**: Local-author port relationship — Resolved
- 🔵 **Testability**: No `terminal` skill case — Resolved
- 🔵 **Testability**: Target stopped conflicting without `--resolve` —
  Resolved
- 🔵 **Clarity**: First-time conflicts on a targeted run — Resolved
- 🔵 **Dependency**: "Blocked by: none" understates the gate — Resolved
- 🔵 **Dependency**: External exit-code consumers — Resolved
- 🔵 **Dependency**: 0285 relationship — Resolved
- 🔵 **Testability**: Tracker-originated classes implicit — Resolved
- 🔵 **Testability**: Root ignores permissions — Partially resolved
  (noted in Drafting Notes, not as an environment dependency)
- 🔵 **Scope**: Dossier fix gated on 0296; one delivery unit — Still
  present (accepted trade-off)
- 🔵 **Completeness**: Version details — Still present

### New Issues Introduced

- 🔵 **Clarity**: "The same apply" could be read run-wide rather than
  per item.
- 🔵 **Clarity**: "Reports as conflicted" names no row column; a failed
  resolve pull's dossier fate is undefined.
- 🔵 **Clarity**: Dossier and baseline writes attributed to the applier,
  though the CLI and `BaselineStore` perform them; dossier-write failures
  not placed in or out of the classification.
- 🔵 **Clarity**: Operation label for tracker-originated failures
  undefined.
- 🔵 **Testability**: AC9 passes if the target's dossier is never
  rewritten.
- 🔵 **Testability**: Exit code asserted only for `write-local` and
  `baseline-set`; mixed-outcome precedence unchecked.
- 🔵 **Testability**: Totality of classification not asserted.
- 🔵 **Dependency**: Non-root runner not captured as an environment
  dependency.
- 🔵 Suggestions: table vs rule precedence and `author-local` scope
  (clarity); "combined change" vs "same release" wording (clarity);
  glossary entries for pending-push markers and local-author port
  (clarity); release-notes owner (dependency); verify the baseline by a
  no-op second run (testability); title understates the exit-code reach
  (scope).

### Assessment

No major findings remain; every pass-4 major is resolved. The open items
are precision refinements — scoping "apply" per item, pinning dossier
outcomes to a row column, attributing each write to its component, and
tightening exit-code assertions — none of which blocks planning. The item
is acceptable as-is and would benefit from the minor fixes.

## Verdict Change — 2026-10-10T23:22:46+00:00

**Verdict:** APPROVE

Changed from COMMENT by the author after the pass-5 minor findings were
addressed in the work item without a further review pass.
