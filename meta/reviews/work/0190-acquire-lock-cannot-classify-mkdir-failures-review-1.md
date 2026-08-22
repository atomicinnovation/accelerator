---
type: work-item-review
id: "0190-acquire-lock-cannot-classify-mkdir-failures-review-1"
title: "Work Item Review: acquire_lock cannot classify an unusable lock directory"
date: "2026-08-20T23:09:33+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0190"
work_item_id: "0190"
reviewer: Toby Clemson
verdict: COMMENT
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-08-21T14:47:24+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: acquire_lock cannot classify an unusable lock directory

**Verdict:** REVISE

0190 is a disciplined, tightly-scoped bug item: every section describes the
same two-part fix (classify a failed `mkdir`, bound the dead-owner reclaim
arm), the frontmatter is complete, and an explicit Scope note holds the
redesign out. The REVISE verdict rests entirely on the Acceptance Criteria,
where three major testability gaps mean the criteria as written do not reliably
verify the behaviour they target — AC1's diagnostic is not pinned to an
assertable substring, AC1/AC2's `chmod`-manufactured preconditions omit the
root-guard and filesystem caveats the item's own sibling 0186 required, and
AC2's ~30 s budget guard is slow and flake-prone with its fast seam left
optional. Clarity, completeness, dependency, and scope raise only minor and
suggestion-level polish.

### Cross-Cutting Themes

- **AC2's timeout mechanism is both ambiguous and operationally risky** (flagged
  by: testability, clarity) — the criterion overloads "timeout" for the loop's
  internal ~30 s budget and the harness subprocess `timeout=`, and a correct
  implementation runs ~30 s of real sleeps every time, so the guard can flake
  under CI load. The env-injectable ceiling that would make it fast and
  deterministic is left "optional" in Technical Notes.
- **The higher-severity unbounded arm is under-anchored** (flagged by: scope,
  completeness, testability) — the dead-owner spin is the more severe defect
  (an unbounded hang, not a wrong diagnostic after a bounded budget), yet its
  broken behaviour is reasoned rather than observed, its severity is not
  distinguished from the milder arm, and AC2 proves only termination, not the
  terminating outcome.
- **0186's test discipline is leaned on for context but not inherited by these
  criteria** (flagged by: testability, dependency) — the item reuses 0186's
  masked-instance framing and cites its precedents, but AC1/AC2 do not adopt
  0186's root-guard and permission-ignoring-filesystem rules for the very
  `chmod`-based preconditions they share.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: AC1 diagnostic "naming the cause" is not pinned to an assertable substring
  **Location**: Acceptance Criteria
  AC1 requires the cold run to fail "with a diagnostic naming the cause" but never states the exact substring a test must assert; the concrete properties (lock path plus a "permission or I/O" clause) live only in Technical Notes. Sibling 0186 set the precedent of pinning the analogous diagnostic to an exact substring, so two verifiers could disagree on whether a message "names the cause".

- 🟡 **Testability**: AC1/AC2 permission preconditions omit the root-guard and filesystem caveats 0186 required
  **Location**: Acceptance Criteria
  Both preconditions are manufactured by `chmod`, exactly the setup for which 0186 devoted a whole preamble: assert `id -u ≠ 0`, hard-fail rather than skip under root, and handle permission-ignoring filesystems via a recorded privilege check. Under root the `mkdir`/`rmdir` succeed regardless of the fix, so both criteria pass vacuously without the fix present.

- 🟡 **Testability**: AC2's ~30 s budget-timeout guard is slow and flake-prone, with the fast seam left optional
  **Location**: Acceptance Criteria
  AC2 pins the bound at the literal 300 × `sleep 0.1` ≈ 30 s ceiling and sets the harness `timeout=` "above ~30 s", so a correct implementation runs ~30 s of real sleeps plus 300 process spawns every time. On a loaded CI host the loop can exceed a just-above-30 s timeout and a correct-but-slow run flakes — a known pattern for this repo's timeout-based shell suites.

#### Minor

- 🔵 **Clarity**: "neither gate" implies two gates but only one is named
  **Location**: Context
  The Context names a single probe gate, then says the dead-owner arm is one "which neither gate prevents". "Neither" presupposes exactly two gates, but only the probe gate is ever named — a reader cannot tell whether a second safeguard exists or whether this is a slip.

- 🔵 **Clarity**: "timeout budget" and "explicit timeout=" name two different timeouts
  **Location**: Acceptance Criteria
  AC2 uses "timeout" for two distinct mechanisms — the loop's internal ~30 s budget and the pytest harness subprocess `timeout=`. Read on its own the criterion invites setting the harness timeout to the loop budget, which would race a correctly-bounded loop against the harness and flake.

- 🔵 **Testability**: AC2 asserts termination but not the terminating outcome
  **Location**: Acceptance Criteria
  AC2 verifies the subprocess terminates before the harness `timeout=` fires but does not require asserting *how* it terminated. A regression that exits for an unrelated reason inside the budget would satisfy the criterion.

- 🔵 **Dependency**: Sibling follow-up 0191 also edits `bin/accelerator` but is not cross-referenced
  **Location**: Dependencies
  0191, raised alongside 0190 in 0186, also edits `bin/accelerator` (the staging block). Neither Dependencies nor `relates_to` cross-references it, so the implied coupling between two concurrently-editable changes to the same file is invisible in the record.

#### Suggestions

- 🔵 **Completeness**: Bug reproduction elements are woven into prose rather than consolidated as steps
  **Location**: Context
  All four reproduction elements are present but distributed across Context and Acceptance Criteria; the first arm carries a measured actual outcome while the more severe unbounded arm's behaviour is stated analytically. A short Reproduction subsection listing each arm's setup/action/expected/actual would let a verifier confirm the fix addresses both.

- 🔵 **Clarity**: "else arm" is not identified among the listed post-fix arms
  **Location**: Technical Notes
  The post-fix arm-order list enumerates six arrow-labelled arms, then refers to "The `else` arm" without indicating which listed arm that is — an inference needed to follow the claim that the reclaim arm shares the else arm's 300-iteration cap.

- 🔵 **Dependency**: No downstream Blocks edge for the 0186 masking gate this fix may make redundant
  **Location**: Context
  Once 0190 lands a general `mkdir`-failure classification, 0186's cold-branch probe gate may become redundant defence, yet no downstream item records that a future "remove the gate" cleanup depends on this fix. A one-line note either recording the follow-up or confirming the gate is retained as defence-in-depth closes the question.

- 🔵 **Scope**: Two bundled defects carry different severities and are separately deliverable
  **Location**: Requirements
  The item bundles a wrong-diagnostic fix (bounded ~30 s budget) with an infinite-hang fix (unbounded spin). Keeping them together is right — both are small and share the root theme — but the unbounded arm is arguably higher severity than the item's medium priority, worth noting so the liveness fix is not deprioritised alongside the milder one.

- 🔵 **Testability**: No criterion maps to the preserved empty/unreadable-pid and genuine-race arms
  **Location**: Technical Notes
  The post-fix control flow enumerates six arms, but the criteria pin only the two new arms and the live-owner/reclaim arms. The "empty/unreadable pid → advance budget" arm and the `else` race arm are asserted preserved with no explicitly named guard — AC3's concurrency test is assumed to cover the race but the mapping is unstated.

### Strengths

- ✅ Summary, Context, Requirements, the explicit Scope note, Acceptance Criteria, and Technical Notes all describe the identical narrow two-arm fix, with no drift between the stated problem and the proposed solution.
- ✅ Both defects live in the same `acquire_lock` loop in one file, serving one unified purpose, so the item is a single indivisible unit of delivery — correctly sized as a `bug` rather than inflated or over-decomposed.
- ✅ Requirements close with an explicit in/out-of-scope statement, and Technical Notes actively resist creep by flagging the env-injectable ceiling as "a testability seam, not a redesign of the scheme".
- ✅ AC2 engineers against a hung suite by pinning the bound with an explicit harness `timeout=`, and AC3 names two existing tests as concrete preservation guards for the live-owner and dead-owner arms.
- ✅ Upstream provenance is captured with a reason per reference — 0164 (introduced the lock), 0186 (masked the reachable instance, recorded the unbounded arm), parent epic 0136 — and the 0186 relationship is described structurally rather than by volatile line numbers.
- ✅ Frontmatter is complete and internally consistent; domain terms (EEXIST, `kill -0`, the bash 3.2 floor) are standard for the audience or glossed in context.

### Recommended Changes

1. **Pin AC1's diagnostic to an exact assertable substring** (addresses: AC1 diagnostic "naming the cause" is not pinned)
   State in AC1 the concrete tokens a test must assert — the lock directory path plus a "permission or I/O" cause clause — mirroring how 0186 pinned its `no writable, exec-capable cache directory` substring, rather than leaving those properties in Technical Notes.

2. **Govern AC1/AC2's permission preconditions with 0186's root-guard rule** (addresses: AC1/AC2 permission preconditions omit the root-guard and filesystem caveats)
   Require a non-root runner, assert `id -u ≠ 0` with a hard-fail-not-skip under root, and record the privilege/filesystem check — or add one line making 0186's Acceptance Criteria preamble govern these two criteria.

3. **Make the env-injectable ceiling mandatory and disambiguate the two timeouts** (addresses: AC2's ~30 s guard is slow and flake-prone; "timeout budget" and "timeout=" name two different timeouts)
   Move the env-injectable ceiling from "optional" into the fix so AC2 runs under a small budget (sub-second wall time) with a correspondingly small harness `timeout=`, and reword the criterion to distinguish the loop's internal budget from the harness subprocess timeout.

4. **Extend AC2 to assert the terminating outcome, not just termination** (addresses: AC2 asserts termination but not the terminating outcome)
   Require the terminating exit to be non-zero and its output to contain the existing lock-timeout message substring (which Scope keeps unchanged), so the criterion pins the intended bounded-failure path rather than merely "did not hang".

5. **Fix the "neither gate" referent** (addresses: "neither gate" implies two gates but only one is named)
   Either name both gates explicitly (the staging gate and the cold-branch gate) so the count of existing safeguards is unambiguous, or reword to "the probe gate does not prevent".

6. **Cross-reference sibling 0191 in `relates_to`/Dependencies** (addresses: 0191 also edits `bin/accelerator` but is not cross-referenced)
   Add 0191 (and, if relevant, 0189) with a one-line note that they touch different regions of `bin/accelerator`, so the merge order is stated as a non-issue rather than discovered at rebase time.

7. **Apply the remaining polish** (addresses: reproduction consolidation; "else arm" label; downstream 0186-gate edge; bundled-defect severity; empty-pid/race arm mapping)
   Consolidate the two arms' reproduction steps, label which listed arm is the `else` arm, record whether the 0186 gate becomes a follow-up cleanup or stays as defence-in-depth, note the unbounded arm's higher severity, and state which criterion or existing test guards the empty-pid and genuine-race arms.

## Per-Lens Results

### Clarity

**Summary**: 0190 is internally consistent and unusually disciplined: the Summary, Context, Requirements, Scope note, Acceptance Criteria and Technical Notes all describe the same narrow two-part fix, and its outcomes are concrete and observable. The clarity weaknesses are localised: one genuine referent mismatch ("neither gate" presupposes two gates while only one is named), a subtle overloading of the word "timeout" across two different mechanisms, and an unlabelled "else arm" in the post-fix arm-order list.

**Strengths**:
- Strong cross-section consistency across Summary, Context, Requirements, the explicit Scope note, Acceptance Criteria and Technical Notes, with no contradiction between the stated problem and the proposed solution.
- Outcomes are stated as observable system states rather than vague properties ("fails within a second with a diagnostic naming the cause", named tests stay green, "mise run exits 0 end-to-end").
- Pronouns and subjects generally resolve cleanly despite the passive framing typical of a bug write-up.
- Domain terms (EEXIST, `mkdir`/`rmdir`, `kill -0`, the bash 3.2 floor) are either standard for the audience or explained in context.

**Findings**:
- 🔵 Minor (high confidence) — Context: "neither gate" implies two gates but only one is named. "Neither" presupposes exactly two gates, but only the probe gate is ever named; a reader cannot tell whether a second safeguard exists or whether this is a slip, which undercuts the paragraph's central claim. Suggestion: name both gates explicitly, or reword to "the probe gate does not prevent".
- 🔵 Minor (medium confidence) — Acceptance Criteria: "timeout budget" and "explicit timeout=" name two different timeouts. AC2 uses "timeout" for both the loop's internal ~30 s budget and the harness subprocess `timeout=`; without cross-referencing Technical Notes a reader could set the harness timeout to the loop budget, making the pinning test flake. Suggestion: "a harness `timeout=` set above the loop's ~30 s budget".
- 🔵 Suggestion (low confidence) — Technical Notes: "else arm" not identified among the listed post-fix arms. The list enumerates six arrow-labelled arms then refers to "The `else` arm" without indicating which one it is — an inference needed to follow the shared-cap claim. Suggestion: label the relevant list item as the else arm.

### Completeness

**Summary**: A thorough, well-populated bug work item: the Summary names the defect precisely, the Context explains both failure arms and their forces, Requirements are specific and explicitly scoped, and five concrete Acceptance Criteria define done. Frontmatter is complete and valid (kind: bug, status: draft). The only completeness observation is that the bug's reproduction elements, while all present, are woven into Context prose rather than presented as explicit reproduce-and-observe steps.

**Strengths**:
- The Summary is a single, unambiguous statement of the defect.
- The Context explains why the work is needed and captures both failure arms in depth, including a measured actual outcome for the first arm and the conditions that trigger each.
- Requirements are specific enough to start work and carry an explicit Scope/"Not" boundary that prevents scope drift.
- Acceptance Criteria give five concrete, self-standing conditions including named regression-guard tests, well above the two-criterion bar.
- Frontmatter is complete and internally consistent — kind: bug and status: draft both present, recognised, and appropriate.

**Findings**:
- 🔵 Suggestion (medium confidence) — Context: the four reproduction elements are all present but distributed across Context prose and Acceptance Criteria rather than consolidated as explicit reproduce-and-observe steps; the first arm carries a measured actual outcome ("TIMEOUT after 31 iters, 3s") while the more severe unbounded-spin arm's behaviour is stated analytically rather than as an observed reproduction. Impact: an implementer or reviewer must reconstruct the two scenarios from narrative. Suggestion: a short Reproduction subsection listing each arm's setup/action/expected/actual, noting where the second arm's failure is reasoned rather than observed.

### Dependency

**Summary**: 0190 is a self-contained shell bug fix with no external systems, vendors, or cross-team actions, and its upstream couplings are well captured: Dependencies names 0164, 0186, and parent epic 0136, each with a one-line rationale. Both upstream items are complete, so the "Relates to" framing correctly signals no live blocker. The only gaps are interpretive and minor: sibling follow-up 0191 (which also edits `bin/accelerator`) is not cross-referenced, and no downstream "Blocks" edge records whether landing this fix enables removal of 0186's now-potentially-redundant masking gate.

**Strengths**:
- Upstream provenance is explicitly captured — 0164 as the origin of the lock, 0186 as the item that masked the reachable instance and recorded the unbounded arm, 0136 as the parent epic — each with a reason.
- The relationship to 0186 is described structurally (the probe gate, the cold branch, the masked instance) rather than by volatile line numbers, so the coupling survives the concurrent rework of `bin/accelerator`.
- No external, third-party, or cross-team coupling is claimed where none exists — the absence is correct rather than a gap.

**Findings**:
- 🔵 Minor (low confidence) — Dependencies: sibling follow-up 0191 also edits `bin/accelerator` (the staging block) but is not cross-referenced in Dependencies or `relates_to`. Impact: whoever picks up the second follow-up discovers the shared-file coupling at rebase time rather than at planning time. Suggestion: add 0191 (and, if relevant, 0189) with a one-line note that they touch different regions of the file.
- 🔵 Suggestion (low confidence) — Context: no downstream Blocks edge for the 0186 masking gate this fix may make redundant. Once 0190 lands a general classification, that gate may become redundant defence, yet no downstream item records the "this enables Y" edge. Suggestion: record a follow-up if a cleanup is intended, or a single line stating the gate is retained as defence-in-depth.

### Scope

**Summary**: Work item 0190 is a well-scoped, coherent bug fix: both defects it addresses live in the same `acquire_lock` loop in a single file and serve one unified purpose — making `acquire_lock` handle an unusable lock directory correctly. It carries an explicit in/out-of-scope statement, its Summary, Requirements, and Acceptance Criteria describe the same work, and the `bug` kind fits the small, indivisible scope. The only scope-adjacent observation is that the two arms carry materially different severities and are technically separable, a low-confidence note rather than a delivery risk.

**Strengths**:
- The Requirements section closes with an explicit Scope statement giving clear in-scope and out-of-scope boundaries.
- Summary, Context, Requirements, and Acceptance Criteria all describe the same two-arm fix; each acceptance criterion maps to a stated requirement.
- Both fixes target a single function in one file with no cross-service or multi-team span.
- The `bug` kind and medium priority fit the scope — neither inflated to a story nor over-decomposed into separate tickets.
- Scope creep is actively resisted — the env-injectable ceiling is explicitly flagged as an optional testability seam.

**Findings**:
- 🔵 Suggestion (low confidence) — Requirements: the item bundles the mkdir-misclassification (burns the bounded ~30 s budget then fails with the wrong diagnostic) with the dead-owner reclaim arm (spins unbounded with no timeout at all). The fixes touch different arms and are independently completable, and the unbounded-hang defect is arguably higher severity than the item's medium priority. Impact: the more urgent liveness fix cannot ship on its own timeline if that becomes desirable. Suggestion: keep them bundled — splitting would be over-decomposition — but note the unbounded-spin arm's higher severity so it is not deprioritised alongside the milder fix.

### Testability

**Summary**: As a bug, both failure modes are well characterised — each has a specified trigger, a measured broken outcome, and a clear expected outcome — and the criteria sensibly reuse two named existing tests as preservation guards while engineering AC2 against a hung suite. However, AC1's "diagnostic naming the cause" is not pinned to a concrete assertable substring (unlike 0186's exact-substring precedent), and AC1/AC2 both manufacture their preconditions by directory-permission manipulation whose reliability caveats — root bypass, permission-ignoring filesystems — were documented at length in the referenced 0186 but are entirely absent here. AC2's ~30 s budget-timeout mechanism is also verifiable but slow and flake-prone, with its fast-path seam left optional.

**Strengths**:
- AC2 explicitly engineers against a hung suite by pinning the bound with an explicit harness `timeout=`, so an unbounded regression surfaces as a pytest failure rather than a hang.
- AC3 names two specific existing tests as concrete preservation guards for the live-owner-extends-budget and dead-owner-reclaim arms.
- The Context supplies a measured broken outcome and AC1 pins a one-second wall-clock threshold that cleanly separates the new fail-fast path from the old ~30 s timeout.
- The Technical Notes surface an env-injectable ceiling as an explicit testability seam and correctly identify directory-presence as the only portable EEXIST discriminator.

**Findings**:
- 🟡 Major (medium confidence) — Acceptance Criteria: AC1 requires the run to fail "with a diagnostic naming the cause" but never states the exact substring a test must assert; the concrete properties live only in Technical Notes, and 0186 set the precedent of pinning the analogous diagnostic to an exact substring. Impact: two verifiers could disagree on whether an arbitrary message "names the cause", so the criterion does not yield a single deterministic assertion. Suggestion: state the exact assertable substring in AC1 (lock path plus a "permission or I/O" cause token).
- 🟡 Major (medium confidence) — Acceptance Criteria: AC1's and AC2's preconditions are both manufactured by `chmod`, exactly the setup for which 0186 required asserting `id -u ≠ 0`, hard-failing rather than skipping under root, and handling permission-ignoring filesystems via a recorded privilege check. Under root the `mkdir`/`rmdir` succeed regardless of the fix, so both criteria pass vacuously. Impact: on a privileged lane, a local root run, or a permission-ignoring filesystem, AC1/AC2 report pass without the fix present. Suggestion: reuse 0186's rule, or reference its preamble as governing these two criteria.
- 🟡 Major (medium confidence) — Acceptance Criteria: AC2 pins the bound at the literal 300 × `sleep 0.1` ≈ 30 s ceiling and requires the harness `timeout=` "above ~30 s", so a correct implementation runs ~30 s of real sleeps plus 300 process spawns every time, and the env-injectable ceiling that would shrink this is left optional. Impact: on a loaded CI host the loop can exceed a just-above-30 s timeout and a correct-but-slow run flakes — a known pattern for this repo's timeout-based shell suites. Suggestion: make the ceiling env-injectable a mandatory part of the fix so AC2 runs under a small budget with a small harness `timeout=`.
- 🔵 Minor (medium confidence) — Acceptance Criteria: AC2 verifies the subprocess terminates before the harness `timeout=` fires but does not require asserting how it terminated. A regression that exits for an unrelated reason inside the budget would satisfy the criterion. Suggestion: also assert the terminating exit is non-zero and its output contains the existing lock-timeout message substring.
- 🔵 Suggestion (low confidence) — Technical Notes: the post-fix control flow enumerates six arms, but the criteria pin only the two new arms and the live-owner/reclaim arms; the "empty/unreadable pid → advance budget" arm and the `else` race arm are asserted preserved with no explicitly named guard. Impact: a verifier cannot confirm the classification branch left these arms intact. Suggestion: state which criterion or existing test exercises the empty-pid and genuine-race arms, or note that AC3's concurrency case is the intended guard.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-08-21

**Verdict:** COMMENT

All three major findings from review 1 are resolved, and every round-1 minor
and suggestion is addressed. The revised criteria pin an assertable diagnostic,
adopt 0186's root-guard rule, and run the bounded arm sub-second under an
env-injected ceiling. The re-review surfaced one new major — a crossed
test/behaviour pairing in the reworked AC3, introduced by the review-1 edit —
now fixed in this same pass, leaving no findings above minor. The residual
minors and suggestions are optional polish; the work item is ready for
implementation.

### Previously Identified Issues

- 🟡 **Testability**: AC1 diagnostic not pinned to an assertable substring — Resolved (AC1 now asserts a fixed cause substring verbatim, run under the injected ceiling).
- 🟡 **Testability**: AC1/AC2 permission preconditions omit 0186's root-guard/filesystem caveats — Resolved (Acceptance Criteria preamble adopts 0186's `id -u ≠ 0` hard-fail rule and privilege check).
- 🟡 **Testability**: AC2's ~30 s budget guard is slow and flake-prone — Resolved (env-injectable ceiling is now a Requirement; AC2 runs sub-second with a small harness timeout).
- 🔵 **Testability**: AC2 asserts termination but not the terminating outcome — Resolved (AC2 asserts non-zero exit + the existing lock-timeout message).
- 🔵 **Clarity**: "neither gate" implies two gates — Resolved (reworded to "the probe gate does not prevent").
- 🔵 **Clarity**: "timeout budget"/"timeout=" name two different timeouts — Resolved (AC2 + Technical Notes mark the loop budget and harness `timeout=` as distinct).
- 🔵 **Clarity**: "else arm" not labelled — Resolved (the empty/unreadable-pid arm is now labelled the `else` arm).
- 🔵 **Dependency**: sibling 0191 not cross-referenced — Resolved (added to `relates_to` + Dependencies with a merge-order note).
- 🔵 **Dependency**: no downstream Blocks edge for the 0186 gate — Resolved (recorded as retained defence-in-depth; nothing downstream waits).
- 🔵 **Scope**: two bundled defects carry different severities — Resolved (reclaim Requirement flags it as the more severe, unbounded-hang defect).
- 🔵 **Completeness**: reproduction elements woven into prose — Resolved (completeness re-review is clean; actual-vs-expected is now contrasted in Requirements and the criteria).
- 🔵 **Testability**: empty-pid/genuine-race arms not mapped to a guard — Resolved (a deterministic empty/unreadable-pid criterion now pins the `else` arm directly).

### New Issues Introduced

- 🟡 **Clarity** (Acceptance Criteria): crossed test/behaviour pairing and an ambiguous "The latter" in the reworked AC3 — introduced by the review-1 edit; **fixed in this pass**: the tests are now paired inline with their behaviours and the `else`-arm guard is a separate deterministic criterion, so "The latter" is gone.
- 🔵 **Testability** (Acceptance Criteria): the fail-fast criterion ran against the default budget, so a regression yielded a ~30 s test — **fixed in this pass**: AC1 now runs under the injected ceiling and discriminates on the diagnostic.
- 🔵 **Dependency**: the 0191 coupling note covers only `bin/accelerator`, not the shared entrypoint test file (minor); 0164/0186 sit under "Relates to" rather than a "completed dependencies" category (suggestion). Not applied — the record is already thorough and both are label-precision nits.
- 🔵 **Scope**: the title/summary lead with the less-severe defect (suggestion). Partly addressed — the Summary now names the unbounded-hang branch self-containedly; the title is left unchanged, deferred to the author.
- 🔵 **Clarity**: `EEXIST` used before a gloss; "arm" used in the Summary ahead of its definition (both low). "arm" addressed in the Summary ("one branch of the retry loop"); the `EEXIST` gloss is left — it already carries an inline parenthetical gloss on first use.
- 🔵 **Testability**: the "default stays 300" Requirement has no verifying criterion (low). Not applied — verifying a constant has limited behavioural value; left intentionally ungated.

### Assessment

The work item is ready for implementation. The three review-1 majors and the one major introduced during revision are all resolved; what remains is optional, low-severity polish (a slightly more descriptive title, a couple of term glosses, minor dependency-label precision) that does not block planning.

## Author Acceptance — 2026-08-21

**Accepted by:** Toby Clemson (author)

Accepted as-is. Both review passes are closed: the three round-1 testability majors and the one major introduced during revision are resolved, and the residual items are optional low-severity polish. No further revisions requested — the work item is cleared for planning.
