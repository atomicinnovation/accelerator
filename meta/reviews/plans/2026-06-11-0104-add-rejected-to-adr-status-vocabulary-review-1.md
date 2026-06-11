---
type: plan-review
id: "2026-06-11-0104-add-rejected-to-adr-status-vocabulary-review-1"
title: "Plan Review: Add rejected to the ADR Status Vocabulary"
date: "2026-06-11T13:24:38+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
target: "plan:2026-06-11-0104-add-rejected-to-adr-status-vocabulary"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [correctness, test-coverage, architecture, code-quality, standards, documentation]
review_number: 1
review_pass: 2
tags: [frontmatter, schema, adr, status, validator]
last_updated: "2026-06-11T13:42:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Add rejected to the ADR Status Vocabulary

**Verdict:** COMMENT

This is a small, exceptionally well-grounded plan: every claim about the schema
source, the data-driven read-sites, the verbatim-coupled template comment, the
`emit_valid` first-token rule, and the unconditional `skip_test` was verified
against the actual source and holds up. The change is sound, additive-only, and
correctly split along a clean green-to-green seam. No critical or blocking
issues were found. The findings cluster around two improvable areas: the Phase 2
corpus fixture is constructed more fragilely than the surrounding suite idiom
warrants, and the `skip_test` removal leaves more stale `deferred to 0104`
comments behind than the plan currently accounts for. Both are easy to fold in
before implementation.

### Cross-Cutting Themes

- **Phase 2 fixture: full-vocab-then-`sed` is fragile and misleading** (flagged
  by: correctness, test-coverage, code-quality, standards) — All four lenses
  independently flagged the same construction. The plan's accept-fixture passes
  the full pipe-delimited vocab (`"proposed | accepted | rejected | superseded | deprecated"`)
  to `emit_valid` and then overwrites the status via `sed -i.bak 's/^status: .*/status: rejected/'`.
  Because `emit_valid` only ever uses the *first* token (`frontmatter-fixtures.sh:40`),
  the vocab argument is inert — it resolves to `status: proposed`, which the
  `sed` discards. The sibling conformance loop already does the right thing by
  passing the single token `"$tgt"`. The strong convergence here makes this the
  top recommended change.

- **Stale `deferred to 0104` comments survive the `skip_test` removal** (flagged
  by: documentation, standards, code-quality) — The deferral framing is
  re-encoded in *three* places in the conformance suite: the file-header
  paragraph (`:20-24`), an inline comment above the loop (`:339-340`,
  "EXCEPT `rejected` — a known schema-source divergence deferred to 0104"), and
  the `skip_test` reason string (`:349`). The plan's Phase 2 names the header
  paragraph and the skip branch but never mentions the inline comment at
  `:339-340`, which will directly contradict the now-uniform loop once the branch
  is gone.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Documentation**: Third `deferred to 0104` re-encoding at `:339-340` not accounted for
  **Location**: Phase 2, Section 2 (flip the conformance `skip_test`)
  The conformance suite carries three re-encodings of the deferral framing; the
  plan accounts for two (header `:20-24`, skip branch `:346-351`) but overlooks
  the inline comment at `:339-340` ("EXCEPT `rejected` — … deferred to 0104").
  Once `rejected` falls through to the uniform loop, that comment contradicts the
  code directly below it — exactly the comment-drift this suite otherwise polices
  verbatim.

#### Minor

- 🔵 **Correctness · Test Coverage · Code Quality · Standards**: Phase 2 corpus fixture passes full vocab then `sed`-overrides it
  **Location**: Phase 2, Section 1 (corpus-validator accept-fixture)
  The full-vocab argument to `emit_valid` is inert (first-token rule), so the
  `sed` does all the real work. Two silent-failure modes: if the `sed` pattern
  ever drifts the fixture would assert acceptance of `status: proposed` (a
  true-but-meaningless PASS), and it needlessly couples the fixture to the exact
  vocab spelling. Replace with the single-token idiom
  `emit_valid adr no decision_makers "rejected" "$TMP/ok-adr-rejected.md"` and
  drop the `sed`, matching the conformance loop (`:354`).

- 🔵 **Documentation · Standards · Code Quality**: Header-comment update (`:20-24`) under-specified
  **Location**: Phase 2, Section 2 (header-comment paragraph instruction)
  The plan says only to "drop the 'deferred to work item 0104' framing", but the
  paragraph also describes the *mechanism* ("represented here as an explicit
  `skip_test` keyed to that id"). After Phase 2 there is no `skip_test`, so that
  sentence is false too. Specify a wholesale rewrite of `:20-24` describing the
  current behaviour, not a partial trim.

- 🔵 **Architecture**: Four re-encodings of the vocab, no cross-check guard for two of them
  **Location**: Current State Analysis — "Independent re-encodings (verify only)"
  Only the template comment has an automated guard tying it to the TSV.
  `status-variant.ts:7` and `adr-read-status.sh:51` are independently maintained
  and nothing cross-checks them against `templates-schema.tsv`. The plan treats
  them as one-time "verify only"; it should name this as an accepted standing
  drift hazard and note a candidate follow-up (a conformance assertion parsing
  the TSV vocab and checking each token is present in both re-encodings).

- 🔵 **Correctness**: Success-criterion `grep -E "rejected"` does not assert the negative
  **Location**: Phase 2 Success Criteria (Automated Verification)
  A bare `grep -E "rejected"` succeeds on *any* matching line, so a residual
  `SKIP:` line could coexist with the PASS lines and still satisfy a casual read.
  Split into two checks: assert the two `PASS:` lines are present, and separately
  `! grep -q 'SKIP:.*rejected'` (exit-code-gated negative).

- 🔵 **Test Coverage**: No negative companion fixture for the adr type
  **Location**: Phase 2, Section 1 (corpus-validator accept-fixture)
  The plan adds a positive accept-fixture but no adr-specific negative. After
  widening, nothing asserts that a *non-member* adr status (a near-miss like
  `reject`) still fails with `BAD-STATUS`. A too-permissive regression on the adr
  status check could slip through. Add a one-line negative mirroring the existing
  `bad-status` case but for the adr type.

- 🔵 **Test Coverage**: TDD reversal checks are manual-only — no standing regression net
  **Location**: Phase 1 & 2 Manual Verification
  The mutation checks that prove the new assertions are non-vacuous live entirely
  under Manual Verification; nothing in CI re-runs them. Consider extending the
  corpus suite's existing tampered-helper pattern (`:140-166`) into a standing
  in-suite negative assertion rather than a one-time human ritual.

- 🔵 **Correctness**: Phase ordering dependency is unguarded
  **Location**: Implementation Approach (independent mergeability)
  Phase 2 applied without Phase 1 present would correctly red on
  `BAD-STATUS`/non-member, but the plan describes Phase 2 as "independently
  mergeable once Phase 1 has landed" without stating the hard ordering
  precondition. State explicitly that Phase 2 must not merge unless Phase 1's TSV
  change is already on the integration branch.

- 🔵 **Architecture**: Verbatim guard is substring-containment, not equality
  **Location**: Phase 1, Change 2 (coupled template comment)
  `test-template-frontmatter.sh:237` uses `grep -qF`, so the comment must
  *contain* the cell as a substring — it would still pass if the comment carried
  *extra* trailing tokens. The substring direction is the one that matters for
  this additive change, but a one-line note that the guard is not a bidirectional
  equality check would stop reviewers over-trusting it.

- 🔵 **Standards**: Quoting style of the new fixture diverges from neighbours
  **Location**: Phase 2, Section 1 (corpus-validator accept-fixture)
  The proposed call quotes positional words (`"decision_makers"`) whereas
  adjacent fixtures pass them unquoted. Functionally identical; cosmetic
  consistency only. (Moot if the single-token idiom recommendation above is
  adopted.)

#### Suggestions

- 🔵 **Architecture**: Phase 1 alone leaves a now-inaccurate skip reason
  **Location**: Implementation Approach (Phase 1 independent mergeability)
  Landing Phase 1 alone makes the vocab accept `rejected` while the `skip_test`
  still claims a divergence that no longer exists. Either keep the phases as one
  mergeable unit, or have Phase 1 acknowledge the transient inaccuracy as a
  conscious, time-boxed tradeoff Phase 2 promptly retires.

- 🔵 **Test Coverage**: Note that the accept-fixture is the *sole* corpus coverage for `rejected`
  **Location**: Testing Strategy
  The generic per-type loop cannot reach `rejected` (first-token rule), so the
  dedicated fixture is the only corpus-validator assertion for it. A brief inline
  comment saying so would stop future cleanup treating it as redundant.

- 🔵 **Documentation**: Template comment does not hint at `rejected_reason`
  **Location**: Phase 1, Section 2 (coupled template comment)
  An author setting `status: rejected` from the template would not learn from it
  that `rejected_reason` is the companion field review-adr persists. Likely out of
  scope, but a one-line note in "What We're NOT Doing" would close the question.

### Strengths

- ✅ Exceptional codebase grounding — every claim about the TSV cell, the
  validator's pipe-split-and-trim `BAD-STATUS` path, the `emit_valid` first-token
  rule, the unconditional `rejected` `skip_test`, and the producer-prose oracle
  was verified against source and is accurate.
- ✅ Correctly diagnoses the central coverage insight: the generic per-type loop
  pins status to the first vocab token, so widening a vocab never auto-tests the
  new value — a dedicated explicit-status fixture is required.
- ✅ Correctly recognises the `skip_test` is unconditional on `tgt = rejected` and
  must be hand-removed; adding the value to the TSV does not auto-convert it to a
  live assertion.
- ✅ Token order (`proposed | accepted | rejected | superseded | deprecated`)
  matches ADR-0031's canonical lifecycle enumeration, the review-adr producer
  table, and the lifecycle-order convention the other TSV rows already use.
- ✅ The Phase 1/Phase 2 seam is well-chosen and the dependency direction is
  honest; the widen-only invariant (existing ADRs stay valid) is correctly
  reasoned, including the real-corpus sanity check staying green.
- ✅ Respects the immutability boundary — accepted ADRs are left alone, with the
  TSV correctly identified as the editable schema source.
- ✅ The independent re-encodings (`status-variant.ts`, `adr-read-status.sh`) were
  confirmed to already carry `rejected`, validating the verify-only
  classification.

### Recommended Changes

1. **Adopt the single-token fixture idiom in Phase 2, Section 1** (addresses:
   the four-lens fixture-construction theme). Replace
   `emit_valid adr no "decision_makers" "proposed | accepted | rejected | superseded | deprecated" "$TMP/ok-adr-rejected.md"`
   + `sed` with `emit_valid adr no decision_makers "rejected" "$TMP/ok-adr-rejected.md"`
   then `assert_accepts`. This removes the inert vocab argument, the fragile `sed`
   step, and the quoting-style divergence in one stroke.

2. **Add the inline comment at `:339-340` to Phase 2's edit list** (addresses:
   the major documentation finding). Instruct the implementer to rewrite the
   "EXCEPT `rejected` … deferred to 0104" clause so it matches the now-uniform
   loop, alongside the header `:20-24` and the skip-branch removal.

3. **Broaden the header-comment instruction to a wholesale rewrite of `:20-24`**
   (addresses: the header-comment under-specification theme). Specify that both
   the "deferred to 0104" framing *and* the "explicit `skip_test`" mechanism
   description be removed/rewritten to describe current behaviour.

4. **Tighten the Phase 2 grep success criterion** (addresses: correctness
   permissive-grep finding). Split into a positive PASS-line check and an
   exit-code-gated `! grep -q 'SKIP:.*rejected'` negative.

5. **Add an adr-type negative fixture** (addresses: test-coverage negative-bound
   finding). One line mirroring the existing `bad-status` case but emitting an
   adr fixture with a non-member status, asserting `BAD-STATUS`.

6. **State the Phase 2-after-Phase-1 ordering as a hard precondition** (addresses:
   correctness ordering finding) and **name the four-re-encoding drift as an
   accepted hazard with a follow-up candidate** (addresses: architecture
   cross-check finding).

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Correctness

**Summary**: The plan is logically sound and its data-flow reasoning is accurate:
the TSV cell, the validator's pipe-split-and-trim `BAD-STATUS` path, the
`emit_valid` first-token rule, the unconditional `rejected` `skip_test`, and the
`extract_review_adr_targets` prose oracle were all verified. The single-token-vocab
idiom the plan relies on is correct, and the widen-only change preserves the
corpus-validity invariant. The only risks are in the Phase 2 accept-fixture
(more fragile than surrounding patterns) and one permissive success-criterion
grep.

**Strengths**:
- Correctly identifies that `emit_valid` pins status to the FIRST pipe token
  (`frontmatter-fixtures.sh:40`); the central correctness insight, verified.
- Correctly recognises the `rejected` `skip_test` is unconditional (`:346-351`)
  and must be hand-removed.
- The Phase 2 conformance loop fix is correct: `$tgt` is the single token
  `rejected`, so the first-token rule yields `status: rejected` directly.
- The widen-only invariant is sound: the validator only rejects values absent
  from the vocab, so adding a member cannot invalidate any existing ADR.
- Verbatim-substring coupling between `templates/adr.md:8` and the TSV cell is
  correctly characterised.

**Findings**:
- 🔵 minor / high — Phase 2, Change 1: corpus fixture passes full vocab then
  `sed`s status. Inert vocab arg + fragile `sed`; two silent-failure modes. Use
  single-token idiom and drop the `sed`.
- 🔵 minor / medium — Phase 2 Success Criteria: `grep -E "rejected"` is permissive
  and does not assert the negative (a residual `SKIP:` could coexist). Split into
  positive PASS check + exit-code-gated negative.
- 🔵 minor / medium — Phase ordering: Phase 2 applied without Phase 1 would red on
  `BAD-STATUS`; the plan does not state the ordering as a hard precondition.

### Test Coverage

**Summary**: An exemplary test-coverage plan: it correctly diagnoses that the
generic per-type loop cannot exercise `rejected` (first-token rule), adds a
dedicated accept-fixture, and converts an unconditional `skip_test` into a live
two-assertion axis. The TDD reversal checks are genuinely load-bearing. Residual
gaps are minor: no negative assertion after widening, manual-only reversal
checks, and a slightly fragile fixture construction.

**Strengths**:
- Correctly identifies the per-type-loop blind spot (first-token rule) — the
  single most important coverage insight, verified against source.
- Correctly recognises the `skip_test` branch is unconditional and must be
  hand-removed, avoiding a silent-no-op trap.
- The Phase 2 manual TDD reversal check is a real mutation test.
- Test reachability is explicitly verified (16-suite floor + named-presence
  guard, `tasks/test/integration.py`).
- The flipped axis runs both `assert_check` and `assert_accepts`, identical to
  every other adr target.

**Findings**:
- 🔵 minor / high — Phase 2, Section 1: full-vocab arg is inert; `sed` does the
  real work; a maintainer could remove the `sed` and silently revert to
  `proposed`. Use single-token idiom.
- 🔵 minor / medium — Manual Verification: load-bearing reversal checks are
  manual-only with no CI regression net; consider extending the tampered-helper
  pattern (`:140-166`) into a standing negative assertion.
- 🔵 minor / medium — Phase 2, Section 1: no negative companion fixture; nothing
  asserts a non-member adr status still rejects after widening.
- 🔵 minor / high — Testing Strategy: the accept-fixture is the sole
  corpus-validator coverage for `rejected`; add an inline comment so cleanup
  doesn't treat it as redundant with the loop.

### Architecture

**Summary**: Architecturally sound: correctly identifies the TSV cell as the
single source of truth, distinguishes auto-propagating read-sites from coupled
re-encodings, and respects the immutability boundary. The Phase 1/Phase 2 seam
cleanly separates the source-of-truth fix from coverage lock-in. The main
weakness is unaddressed: the vocab is re-encoded in four places with no
cross-checking guard for two of them — a latent drift hazard the plan treats as
one-time "verify only".

**Strengths**:
- Correctly models one authoritative source with a well-mapped fan-out; only the
  source and its verbatim-checked re-encoding are edited.
- Respects the immutability boundary (ADRs left alone; TSV identified as the
  lever).
- Phase 1/Phase 2 seam is well-chosen with an honest dependency direction.
- Purely additive widening preserves the open-closed posture.

**Findings**:
- 🔵 minor / high — Current State Analysis: four re-encodings, only the template
  comment guarded; `status-variant.ts` and `adr-read-status.sh` are unguarded
  drift hazards. Name as accepted latent coupling + follow-up candidate.
- 🔵 minor / medium — Phase 1, Change 2: the `grep -qF` guard is
  substring-containment, not equality; would not catch the comment drifting
  *ahead*. Note this so reviewers don't over-trust it.
- 🔵 suggestion / high — Implementation Approach: Phase 1 alone leaves a
  now-inaccurate skip reason (claims a divergence that no longer exists); make the
  transient inconsistency a conscious tradeoff.

### Code Quality

**Summary**: A small, well-scoped data-edit plan with unusually thorough
codebase grounding and a clean green-to-green seam. The edits match existing
suite conventions almost exactly. The main concern is that the proposed
accept-fixture passes a full multi-token vocab to `emit_valid` then immediately
overwrites status via `sed`, which is more indirect than required and
misrepresents how the fixture's status is set.

**Strengths**:
- The `sed`-based override is consistent with the established pattern in the same
  suite.
- The plan documents `emit_valid`'s first-token rule and why the per-type loop
  never exercises the new value.
- The `skip_test` removal is framed as a full clean-up (inline branch + header
  comment + `0104` grep guard).
- Phase boundaries are genuinely independently mergeable and green-to-green.

**Findings**:
- 🔵 minor / high — Phase 2, Section 1: full-vocab arg to `emit_valid` is inert,
  discarded by the next `sed`; obscures that the fixture's purpose is `rejected`.
  Pass a single honest token and drop the `sed`.
- 🔵 suggestion / medium — Phase 2, Section 2: header-comment update scope
  under-specified; the paragraph also describes the `skip_test` mechanism that
  will be gone. Specify removing the `rejected`-specific sentences wholesale.

### Standards

**Summary**: Adheres closely to project schema and test conventions. The TSV
edit preserves tab delimiters and ` | ` separators verbatim, the new token is in
the position matching both lifecycle ordering and ADR-0031's enumeration, and the
fixture follows the suite's `emit_valid` + `sed` + `assert_accepts` idiom. No
violations; observations are minor consistency notes.

**Strengths**:
- Token order matches ADR-0031's canonical lifecycle enumeration and the
  review-adr producer table, and follows the TSV's lifecycle-order convention.
- TSV instruction explicitly preserves tab + ` | ` separators (correct, given the
  `grep -qF` fixed-string match).
- The Phase 2 fixture mirrors the suite's existing idiom.
- Flipping the `skip_test` to fall through to the identical assertion block keeps
  the loop uniform.
- Independent re-encodings correctly scoped as verify-only.

**Findings**:
- 🔵 minor / high — Phase 2, Change 1: quotes positional args (`"decision_makers"`)
  where adjacent fixtures pass them unquoted; cosmetic consistency only.
- 🔵 minor / medium — Phase 2, Change 2: success criteria verify code-level
  outcomes but not that the prose header comment was updated.
- 🔵 suggestion / medium — References: flags ADR-0033 as a possible stray
  reference. **Reviewer note: investigated and dismissed** — ADR-0033 is
  `unified-base-frontmatter-schema`, a legitimate schema ADR the plan rightly
  disclaims editing; not a typo for 0042.

### Documentation

**Summary**: Unusually rigorous documentation audit: it inventories the schema
source, the data-driven read-sites, the coupled template comment, and the
independent re-encodings (verified to already carry `rejected`). The one real gap
is comment drift inside the conformance suite — a third inline comment at
`:339-340` re-encoding the `deferred to 0104` framing that the plan overlooks.
The header-comment update is also under-specified.

**Strengths**:
- The Current State Analysis classifies every re-encoding by edit-disposition.
- Verified accurate: `status-variant.ts:7` (+ tests) and `adr-read-status.sh:51`
  already carry `rejected`.
- Correctly identifies the verbatim TSV↔template-comment coupling and specifies
  preserving separators byte-identically.
- Leaves immutable ADRs untouched, matching the "oversight not exclusion" claim.

**Findings**:
- 🟡 major / high — Phase 2, Section 2: a third `deferred to 0104` re-encoding at
  `:339-340` ("EXCEPT `rejected` …") is not in the plan's edit list; it will
  contradict the now-uniform loop once the skip branch is removed.
- 🔵 minor / high — Phase 2, header-comment instruction: also describes the
  `skip_test` mechanism (`:24`) which will be false post-Phase-2; broaden to a
  wholesale rewrite of `:20-24`.
- 🔵 suggestion / medium — Phase 1, Section 2: the template comment does not hint
  at `rejected_reason`; optionally note in "What We're NOT Doing".

## Re-Review (Pass 2) — 2026-06-11T13:42:43+00:00

**Verdict:** APPROVE

The same six lenses re-reviewed the edited plan. The one **major** finding is
resolved, every prior finding is resolved, and the two small new issues the
re-review surfaced were fixed in the same pass. The plan is ready for
implementation.

### Previously Identified Issues

- 🟡 **Documentation**: Third `deferred to 0104` re-encoding at `:339-340` not
  accounted for — **Resolved**. Phase 2 Section 2 now enumerates all the surviving
  comment sites and prescribes a wholesale header rewrite.
- 🔵 **Correctness · Test Coverage · Code Quality · Standards**: Phase 2 fixture
  full-vocab-then-`sed` — **Resolved** (accept-fixture now uses the single-token
  idiom with no `sed`). _Note: the fix introduced the same pattern in the new
  reject-fixture — see New Issues; now also fixed._
- 🔵 **Documentation · Standards · Code Quality**: header-comment update
  under-specified — **Resolved**. Wholesale rewrite specified, covering both the
  deferral framing and the now-false `skip_test`-mechanism sentence.
- 🔵 **Architecture**: four re-encodings, no cross-check guard — **Resolved**. Named
  as accepted latent coupling in "What We're NOT Doing" with a concrete follow-up
  candidate.
- 🔵 **Correctness**: permissive `grep -E "rejected"` — **Resolved**. Split into a
  positive PASS-line check and an exit-code-gated `! … grep -q "SKIP:.*rejected"`.
- 🔵 **Test Coverage**: no negative companion fixture — **Resolved**. A non-member
  `reject` reject-fixture now bounds the widened vocab; verified the validator
  uses exact token equality so `reject` is a true non-member.
- 🔵 **Test Coverage**: TDD reversal checks manual-only — **Resolved**. The accept
  and reject fixtures plus the live conformance axis are standing CI assertions.
- 🔵 **Correctness**: unguarded phase ordering — **Resolved**. Phase 2 now states a
  hard ordering precondition and recommends landing both phases together.
- 🔵 **Architecture**: `grep -qF` substring-vs-equality — **Resolved**. The plan now
  notes the guard is substring-containment, not equality.
- 🔵 **Architecture**: Phase 1 leaves an inaccurate skip reason — **Resolved**. The
  transient inconsistency is now a conscious, documented tradeoff.
- 🔵 **Test Coverage**: accept-fixture is sole corpus coverage — **Resolved**. Inline
  NOTE comment added so cleanup doesn't treat it as redundant.
- 🔵 **Standards**: quoting-style divergence — **Resolved**. The rewritten fixture
  passes the extras arg unquoted (matching neighbours); the quoted single-token
  vocab arg matches the suite's vocab-arg convention.
- 🔵 **Standards**: stray ADR-0033 reference — **Resolved/dismissed** (ADR-0033 is a
  legitimate schema ADR the plan rightly disclaims editing).
- 🔵 **Documentation**: template comment doesn't hint at `rejected_reason` —
  **Accepted as-is**. Out of scope per the plan's deliberate deferral of
  rejected-ADR surfacing; re-review confirmed this disposition is reasonable
  (`rejected_reason` is a producer-persisted field orthogonal to the vocab edit).

### New Issues Introduced

- 🔵 **Correctness · Test Coverage · Code Quality** (now fixed): the reject-fixture
  initially reused the inert-vocab + `sed` two-step (`emit_valid … "rejected"`
  then `sed` to `reject`) — the same anti-pattern just removed from the
  accept-fixture. **Fixed**: rewritten to `emit_valid adr no decision_makers
  "reject"` with no `sed`, matching the accept-fixture idiom.
- 🔵 **Documentation** (now fixed): the "three comment sites" framing missed a
  fourth `0104` reference (the in-branch comment at `:348`). **Fixed**: Phase 2
  Section 2 now frames the edit as deleting the entire `:346-351` block, with the
  `grep -c "0104"` criterion as backstop.

### Assessment

The plan is in good shape and ready for implementation. The core change remains
a sound, additive, well-grounded data edit; the two-phase seam, the bounded
fixtures (accept + non-member reject), the comprehensive comment cleanup, and
the explicit ordering precondition all hold up under re-review. No outstanding
critical, major, or unaddressed minor findings remain.
