---
type: plan-review
id: "2026-06-23-0120-prevention-tests-agent-invocation-path-review-1"
title: "Plan Review: Prevention Tests for the Agent-Invocation Path"
date: "2026-06-23T09:03:23+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-06-23-0120-prevention-tests-agent-invocation-path"
target: "plan:2026-06-23-0120-prevention-tests-agent-invocation-path"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [test-coverage, correctness, code-quality, standards, portability, architecture]
review_number: 1
review_pass: 2
tags: [migrate, interactive-migration, agent-invocation, testing, 0007]
last_updated: "2026-06-23T09:17:20+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Prevention Tests for the Agent-Invocation Path

**Verdict:** REVISE

This is a tightly-scoped, well-grounded regression-test plan that correctly
recognises AC1 is satisfied-by-0116 and concentrates real value in the AC2
incident-shaped `pr-description`/tracker-key cross-check; its analysis of the
vacuous `FAIL:.*MISSING-EXTRA` regex is verified-correct and the
meaningful-equivalent substitute is sound. However, one verified **critical**
defect blocks it: the plan asserts quoted `"unknown"` sentinels on `pr_url` and
`merge_commit`, but both keys are in `FM_OPTIONAL_EXTRAS`, so the 0007 backfill
skips them entirely — those two assertions will fail and Phase 1 cannot go
green as written. A second, **cross-cutting** issue (flagged by all six lenses)
is the internal fixture-name inconsistency (`ENG-1234` in the code vs `PP-142`
in the manual-verification step). Both are mechanical to fix; the plan's overall
structure and reasoning are otherwise strong.

### Cross-Cutting Themes

- **Fixture-name inconsistency `ENG-1234` vs `PP-142`** (flagged by: test-coverage,
  correctness, code-quality, standards, architecture) — the Phase 1 heredoc
  creates `meta/prs/ENG-1234-description.md`, but the Manual Verification step
  (and several narrative sections: Current State, Key Discoveries, Testing
  Strategy) refer to `meta/prs/PP-142-description.md` (the work item's real
  `external_id`). An implementer following the manual check looks for a file
  that is never created. Both stems happen to derive an empty `pr_number`, so
  the technical conclusion is unaffected — this is a traceability/consistency
  defect, not a logic error.
- **Hyphenated pinned-key spelling left contingent** (flagged by: test-coverage,
  code-quality, portability) — Phase 2's distinctive key
  `agent-invocation-pending-key` is load-bearing for the AC1(a) assertion, yet
  the plan leaves an `agentInvocationPendingKey` fallback open "if any consumer
  chokes on the hyphens." The hyphen is in fact inert (`fields[0]` split on `|`,
  fixed-string `grep -qF`), so the form is safe — pre-resolve to one spelling
  so comments, banner, and seeded key cannot diverge.
- **Same reasoning re-stated in three places** (flagged by: test-coverage,
  code-quality) — the underivable→sentinel / vacuous-regex rationale appears in
  the fixture comment, the assertion comment, and the implementer notes; these
  will drift independently.

### Tradeoff Analysis

- **Targeted assertions vs validator-gate reuse**: Code Quality flags the
  incident fixture being validated three overlapping ways (corpus-wide
  `assert_validates`, a hand-rolled `INCIDENT_VOUT` capture, and a per-file
  `assert_validates`). The hand-rolled capture duplicates what the
  `assert_validates` helper exists to encapsulate. Recommendation: keep the
  per-file `assert_validates "$INCIDENT"` plus the `pr_number` sentinel-form
  assertion, and drop the open-coded `INCIDENT_VOUT` capture unless the explicit
  no-`MISSING-EXTRA`-token check is judged load-bearing — in which case wrap it
  in a small helper rather than open-coding it.

### Findings

#### Critical

- 🔴 **Test Coverage**: AC2 assertions on `pr_url`/`merge_commit` will fail — they are optional extras, never sentinel-stamped
  **Location**: Phase 1, Changes Required #3: Assertions (lines 256–259)
  The plan asserts `pr_url: "unknown"` and `merge_commit: "unknown"` on the
  incident fixture, but both keys are in `FM_OPTIONAL_EXTRAS`
  (`frontmatter-emission-rules.sh:74`) and the 0007 backfill loop skips optional
  extras (`0007:510 case " $FM_OPTIONAL_EXTRAS " in *" $ex "*) continue`). Only
  `pr_number` (genuinely required) receives the bare `unknown` sentinel; the
  other two stay absent (validator-clean, since MISSING-EXTRA also skips optional
  extras). **Verified directly against source during aggregation.** Those two
  `assert_contains` calls will fail, so Phase 1 cannot reach green, and the TDD
  sanity-revert would go red for the wrong reason — masking whether the genuine
  `pr_number` guard bites.

#### Major

- 🟡 **Test Coverage / Correctness / Code Quality / Standards / Architecture**: Manual-verification step references a fixture filename that does not exist
  **Location**: Phase 1, Manual Verification (lines 312–313) vs Changes Required #1 (line 194)
  The heredoc creates `ENG-1234-description.md`; the manual-verification step,
  and the Current State (:89), Key Discoveries (:126), and Testing Strategy
  (:428) sections, reason in terms of `PP-142-description`. Pick one stem and use
  it everywhere. (Merged from five lenses; highest contributing severity used.)

#### Minor

- 🔵 **Architecture**: AC1 stall test couples to a resume-command format owned by 0119
  **Location**: Phase 2: AC1 — harden + relabel the no-input stall test
  The test asserts the literal resume-command output (`--decisions-file`,
  `ACCELERATOR_MIGRATE_DECISIONS_FILE=`, `run-migrations.sh`, migration id). The
  work item warns 0119 may change this format. The plan cites the dependency in
  prose but does not encode it where the coupling lives — add a comment at the
  assertion block naming 0119 / `emit_no_input_stall` as the owner.

- 🔵 **Architecture**: New fixtures spliced into the shared Phase 4 corpus block
  **Location**: Phase 1, Changes 1–3 (the shared `run_0007 "$P4"` block)
  The new fixtures join the corpus-wide `assert_validates` and idempotency
  diff-check, growing the block's blast radius. This is the same tradeoff
  NODEFAULT/WIDENING already make (acceptable for reuse), but the idempotency
  note (:286–287) should be verified explicitly: the bare-`unknown` `pr_number`
  must survive a second `run_0007` unchanged.

- 🔵 **Code Quality**: Redundant triple validator assertion on the incident fixture
  **Location**: Phase 1, Changes Required #3 (the `INCIDENT_VOUT` block, ~lines 260–270)
  The incident fixture is validated three overlapping ways via two mechanisms;
  the hand-rolled `INCIDENT_VOUT=$("$VALIDATOR" … ) || true` re-implements what
  `assert_validates` encapsulates. Consolidate (see Tradeoff Analysis).

- 🔵 **Correctness**: Counter-fixture `assert_not_contains 'unknown'` passes vacuously if `fm_line` is empty
  **Location**: Phase 1, Changes Required #3: Counter-fixture assertions (lines 276–277)
  If `pr_number` were dropped entirely (not sentinel-replaced), the negative
  assertion passes on an empty string. The paired positive `pr_number: 42`
  assertion covers the derive case, so the bundle is sound, but the negative
  contributes no independent signal — consider `assert_eq 'pr_number: 42'`
  (mirroring the existing PR430 pattern at `:1304`).

- 🔵 **Test Coverage**: TDD sanity-revert validates only one coarse mutation
  **Location**: Implementation Approach / Phase 1 TDD sanity check (lines 158–163, 292–297)
  Commenting out the sentinel branch is a single mutation; combined with the
  critical assertion defect, the revert would go red for the wrong reason. After
  fixing the optional-extras assertions, confirm specifically that
  `pr_number: unknown`, no-`MISSING-EXTRA`, and per-file `assert_validates` each
  flip red, while the counter-fixture's `pr_number: 42` stays green.

- 🔵 **Test Coverage**: Fixture comment misstates which `extra_default` arm is exercised
  **Location**: Phase 1, Changes Required #1: Incident fixture comment (lines 188–192)
  The comment's "date-prefixed-exclusion is moot" wording is loosely worded;
  the leading-numeric fallback returns empty because the stem is non-numeric-
  leading, and the date-prefix `case` is never reached. Documentation precision
  only — the end-to-end sentinel assertion still catches a contract break.

#### Suggestions

- 🔵 **Code Quality**: Comment duplication across fixture / assertion / implementer notes
  **Location**: Phase 1, Changes Required #1 (lines 189–192) and #3 (lines 251–265)
  Let the fixture comment state *what* it is; concentrate the *why* (vacuous
  regex, bare-vs-quoted sentinel) in one place at the assertion site.

- 🔵 **Code Quality**: Counter-fixture overlaps the existing PR430 boundary check
  **Location**: Phase 1, Changes Required #2: Counter-fixture
  PR430 (`:1264–1298`) already proves a `pr-` stem derives and is not sentinel-
  replaced for `pr-review`; the new counter is the `pr-description` analogue. A
  one-line comment noting the type-specific distinction prevents a future
  "consolidation" mistake.

- 🔵 **Code Quality / Test Coverage / Portability**: Pre-resolve the hyphenated pinned-key spelling
  **Location**: Phase 2, Notes for the implementer (lines 374–378)
  The hyphen is inert; commit to a single spelling everywhere and drop the
  conditional fallback so the seeded key, comments, and banner cannot diverge.

- 🔵 **Standards**: A few new lines exceed the documented 80-column standard
  **Location**: Phase 1, Changes Required #3 (assertion block) and comments
  Unenforced for shell (shfmt/ShellCheck have no length rule; the suites already
  carry many over-80 lines), so CI stays green — but be consistent within the
  new block.

- 🔵 **Portability**: `interactive` suite lacks the `export LC_ALL=C` the 0007 suite pins
  **Location**: Phase 2
  No concrete failure (all Phase 2 assertions are ASCII), noted only as a
  cross-suite asymmetry. Optional housekeeping, separate item.

### Strengths

- ✅ Correctly recognises AC1 is satisfied-by-0116 and chooses to harden+relabel
  the existing no-input stall test rather than author a near-duplicate — DRY,
  while still pinning a distinctive key to make AC1(a) unambiguous.
- ✅ The "vacuous regex" analysis is verified accurate (violation lines are
  `<file>: CODE — <msg>` with no `FAIL:` prefix; the only `FAIL:` line is the
  codeless summary), and the meaningful-equivalent substitute is the right call,
  documented with an explanatory comment.
- ✅ The counter-fixture (`pr-42-description` → `pr_number: 42`, not sentinel-
  replaced) is the correct boundary guard, proving the sentinel fires only where
  the value is genuinely underivable.
- ✅ Fixture placement directly under `meta/prs/` (longest-dir-wins →
  `pr-description`) is load-bearing and correctly identified; the
  derivable-vs-underivable boundary for `extra_default`'s `pr_number` arm is
  accurately analysed (verified: `ENG-1234`/`PP-142` derive empty,
  `pr-42-description` derives `42`).
- ✅ Rides the existing `run_0007 "$P4"` corpus path and corpus-wide
  `assert_validates` gate rather than a parallel harness; matches the work
  item's resolved decision (cross-check in the 0007 suite, not a standalone
  lint).
- ✅ Two-phase decomposition is genuinely decoupled (one file each, independently
  mergeable), matching the disjoint-blocker structure (AC2→0118, AC1→0116).
- ✅ All proposed shell is within the bash 3.2 floor and locale-safe (asserts the
  ASCII `MISSING-EXTRA` token, never the non-ASCII em-dash); fixtures use fixed
  timestamps and `git_init` identity, so they are environment-independent.

### Recommended Changes

1. **Remove the `pr_url`/`merge_commit` quoted-sentinel assertions** (addresses:
   critical AC2 assertions will fail; redundant triple validator assertion)
   Delete the two `assert_contains … 'pr_url: "unknown"'` /
   `'merge_commit: "unknown"'` lines. Both are optional extras the backfill
   skips, so they are absent post-migration (validator-clean). Keep only the
   `pr_number: unknown` bare-sentinel assertion plus the per-file
   `assert_validates "$INCIDENT"`. Optionally add
   `assert_not_contains … "$(fm_line "$INCIDENT" pr_url)" 'pr_url'` to positively
   document that optional extras stay absent. Update the "stamps all three"
   claim in Key Discoveries (:132) and the bare/quoted reasoning in the
   assertion comment accordingly.

2. **Make the fixture stem consistent** (addresses: major cross-cutting naming
   inconsistency)
   Pick one neutral prefix (the plan argues for `ENG-1234`) and use it in the
   heredoc, `id`/title, assertion descriptions, Manual Verification, Current
   State (:89), Key Discoveries (:126), and Testing Strategy (:428).

3. **Pre-resolve the Phase 2 key spelling and drop the fallback** (addresses:
   hyphenated-key contingency)
   Commit to `agent-invocation-pending-key` (hyphen verified inert) everywhere;
   remove the `agentInvocationPendingKey` conditional from the implementer notes.

4. **Consolidate the incident-fixture validation** (addresses: redundant triple
   validator assertion)
   Keep the per-file `assert_validates "$INCIDENT"`; drop the open-coded
   `INCIDENT_VOUT` capture, or wrap it in a small helper if the explicit
   no-`MISSING-EXTRA` check is kept.

5. **Encode the 0119 resume-format coupling as a comment** (addresses: AC1
   coupling to externally-owned format)
   Add a comment at the Phase 2 assertion block naming 0119 /
   `emit_no_input_stall` (interactive-lib.sh:313–346) as the owner of the
   resume-command strings these assertions track.

6. **Re-run the TDD sanity-revert after the fix** (addresses: revert validates
   one mutation)
   Confirm `pr_number: unknown`, no-`MISSING-EXTRA`, and per-file
   `assert_validates` each flip red under the sentinel-branch revert, while the
   counter's `pr_number: 42` stays green.

## Per-Lens Results

### Test Coverage

**Summary**: Well-targeted regression-test plan: correctly identifies AC1 as
satisfied-by-0116 and concentrates value in the AC2 incident-shaped
`pr-description`/tracker-key cross-check; the vacuous-regex analysis is
verified-correct with a sound meaningful-equivalent substitute. **A verified
defect**: `pr_url`/`merge_commit` are in `FM_OPTIONAL_EXTRAS`, so the backfill
never stamps them and the plan's `pr_url: "unknown"` / `merge_commit: "unknown"`
assertions will fail, undermining both the green criterion and the sanity-revert.
Remaining edge-case coverage (bare-vs-quoted `pr_number`, derivable counter,
idempotency) is appropriate.

**Strengths**:
- Recognises AC1 satisfied-by-0116; harden+relabel over near-duplicate.
- "Vacuous regex" analysis verified; meaningful equivalent is the right call.
- Counter-fixture is the correct underivable-boundary guard.
- Incident fixture correctly placed under `meta/prs/` (longest-dir-wins).
- Explicitly calls out the idempotency constraint the new fixtures must preserve.

**Findings**:
- **critical / high** — Phase 1 §3 Assertions (lines 256–259): `pr_url`/
  `merge_commit` assertions will fail; they are optional extras
  (`frontmatter-emission-rules.sh:74`) the backfill skips (`0007:510`). Only
  `pr_number` gets the bare sentinel. Drop the two assertions or assert their
  absence.
- **major / high** — Phase 1 Manual Verification (lines 312–313): references
  `PP-142-description.md` but the fixture is `ENG-1234-description.md`.
- **minor / medium** — Phase 1 §1 fixture comment (lines 188–190): misstates
  which `extra_default` arm is exercised.
- **minor / medium** — TDD sanity check (lines 158–163, 292–297): revert
  validates one coarse mutation; isolate per-assertion bite after the fix.
- **minor / low** — Phase 2 (lines 356, 373–378): distinctive hyphenated key
  carries an unverified pass-through assumption (mitigated by the sanity-revert).

### Correctness

**Summary**: The plan's load-bearing claims are overwhelmingly correct: verified
that `ENG-1234`/`PP-142` derive empty `pr_number` from both `extra_default` arms
while `pr-42-description` derives `42`; that the `FAIL:.*MISSING-EXTRA` regex is
genuinely vacuous; that longest-dir-wins yields `pr-description` under
`meta/prs/`; and that the hyphenated key passes through `seed_predicate_sandbox`
and the `IFS='|'` split unchanged. The main issue is the `ENG-1234`/`PP-142`
inconsistency.

> **Aggregator note:** this lens additionally claimed `pr_url`/`merge_commit`
> route through `fm_normalise_value` and get quoted `"unknown"`. That claim was
> **checked directly during aggregation and found incorrect** — the backfill
> loop skips optional extras before any emission/normalisation, so those keys
> are never stamped. The Test Coverage lens's critical finding stands; this
> lens's contradicting claim is discarded.

**Strengths**:
- Both `pr_number` derivation claims provably correct (arm 1 segment + arm 2
  leading-numeric fallback).
- Bare-vs-quoted sentinel distinction correct *for required extras*.
- Vacuous-regex analysis exactly right.
- Type inference correctly reasoned (longest-dir-wins).
- Hyphenated key pass-through holds.

**Findings**:
- **minor / high** — Manual Verification (:312) and narrative (:89, :126, :428):
  `ENG-1234` vs `PP-142` inconsistency.
- **minor / medium** — Phase 1 §3 counter assertions (:276–277):
  `assert_not_contains 'unknown'` vacuous on empty `fm_line`; prefer exact-line
  positive form.
- **suggestion / high** — Phase 1 §1 comment (:189): tighten to match
  `extra_default` branch order.

### Code Quality

**Summary**: Well-scoped; proposed additions closely match the host suites'
idioms (heredoc fixtures, `fm_line` + assert helpers, `Phase 4 X:` naming, heavy
comments matching house style). The harden+relabel AC1 decision is the cleaner
choice. Main concerns: a redundant triple validator assertion (hand-rolled
`INCIDENT_VOUT` duplicating `assert_validates`) and the `ENG-1234`/`PP-142`
inconsistency.

**Strengths**:
- Harden-and-relabel over near-duplicate is DRY and justified.
- Fixtures mirror existing NODEFAULT/WIDENING/HYBRID heredoc pattern.
- Reuses existing helpers rather than new assertion machinery.
- Magic strings well-chosen (`ENG-1234`, `pr-42-description`,
  `agent-invocation-pending-key`).
- Heavy comments proportional to the host suite's existing density.

**Findings**:
- **minor / high** — Phase 1 §3 (`INCIDENT_VOUT` block, ~260–270): triple
  validator assertion; consolidate to the helper.
- **minor / high** — Manual Verification (~312–314) vs fixture (line 194):
  `ENG-1234` vs `PP-142`.
- **suggestion / medium** — Phase 1 §1 (189–192): comment duplication across
  fixture/assertion/notes.
- **suggestion / medium** — Phase 1 §2: counter-fixture overlaps PR430; add a
  type-distinction comment.
- **suggestion / low** — Phase 2 notes (374–378): pre-resolve the hyphenated key
  spelling.

### Standards

**Summary**: Strongly convention-aligned: fixtures, heredoc shapes, assertion
idioms, `Phase 4 X:` naming, and em-dash comment style mirror the existing
suites; the bash 3.2 floor (ADR-0016, lint-bashisms.sh) is respected; the
`meta/prs/` placement targets `pr-description` correctly; edits-only scope means
the exec-bit invariant doesn't trigger. Only issues: the `ENG-1234`/`PP-142`
inconsistency and a documented-but-unenforced 80-col over-run.

**Strengths**:
- Assertion idioms match the suite exactly (`grep -qF` semantics).
- Test names follow `Phase 4 <TAG>:` convention.
- Heredoc frontmatter mirrors existing P4 heredocs; correct `pr-description`
  inference.
- Bash 3.2 floor respected (no denylisted constructs).
- Em-dash usage consistent with existing convention.
- Edits-only scope correctly identified (no exec-bit/SHELL_LIBRARIES impact).

**Findings**:
- **minor / high** — Phase 1 §1 (~194/250) vs Manual Verification (312–313):
  `ENG-1234` vs `PP-142`.
- **suggestion / medium** — Phase 1 §3 (248–278) and comments (187–192,
  251–265): some new lines exceed 80 cols (unenforced for shell; CI stays green,
  consistent with the suite's existing reality).

### Portability

**Summary**: Test-only, low-risk. Every construct (quoted heredocs, `</dev/null`,
`mkdir -p`, the `assert_*`/`grep -qF` helpers) is bash-3.2 safe and already
proven across Linux CI and macOS local. Notably locale-aware: asserts the ASCII
`MISSING-EXTRA` token and `pr_number: unknown` rather than the non-ASCII em-dash;
the 0007 suite pins `export LC_ALL=C`.

**Strengths**:
- `</dev/null` no-input idiom reuses the shipped pattern; identical CI/local
  reproduction.
- Locale-safe assertion design (ASCII tokens, never the em-dash byte sequence).
- All new constructs within the bash 3.2 floor.
- Fixtures environment-independent (fixed timestamps, `git_init` identity).
- Pre-empts the hyphen concern with a documented fallback.

**Findings**:
- **suggestion / high** — Phase 2 §1: interactive suite lacks `export LC_ALL=C`
  that the 0007 suite pins; no concrete failure (all-ASCII), noted as a
  cross-suite asymmetry — optional separate housekeeping.
- **suggestion / medium** — Phase 2 notes: hyphenated pinned key is portable
  as-is; keep the documented fallback, no change needed.

### Architecture

**Summary**: Tightly-scoped and respects the existing test architecture: rides
`run_0007 "$P4"` + `assert_validates` rather than a parallel harness, and honours
the resolved work-item decision (cross-check in the 0007 suite). The two-phase
split is genuinely decoupled and the structural constraints (fixture placement,
bare-vs-quoted sentinel) are correctly identified. Main concern: the AC1 test's
coupling to a resume-command format 0119 may change.

**Strengths**:
- Hosting AC2 in the existing 0007 corpus block is the sound choice over a
  parallel harness or standalone lint; reuses the single validator gate.
- Fixture-placement constraint (`meta/prs/` → `pr-description`) correctly
  identified as load-bearing.
- Two-phase decomposition genuinely decoupled and independently mergeable.
- Incident + counter pairing guards both sides of the backfill boundary.
- Recognises and substitutes the vacuous AC regex.

**Findings**:
- **minor / high** — Phase 2: AC1 test couples to a resume format the work item
  warns 0119 may change; encode the ownership as a comment.
- **minor / medium** — Phase 1 §§1–3: new fixtures join the shared corpus
  block's gates (blast radius); verify the idempotency note explicitly.
- **suggestion / high** — Phase 1 §1 (:194) vs Manual Verification (:312):
  `ENG-1234` vs `PP-142` inconsistency.

---

## Re-Review (Pass 2) — 2026-06-23

**Verdict:** APPROVE

Re-ran the four lenses that carried actionable findings (test-coverage,
correctness, code-quality, architecture) against the edited plan. **All prior
findings — the critical, the major, and every minor — are resolved.** The
critical was independently re-verified: the plan now asserts only the bare
`pr_number: unknown` sentinel plus the genuine absence of the optional
`pr_url`/`merge_commit` extras (confirmed against `FM_OPTIONAL_EXTRAS`
:74, the backfill optional-skip `0007:510`, and the validator carve-out
`validate-corpus-frontmatter.sh:344`). The earlier Correctness lens's false
"quoted `\"unknown\"`" claim is fully purged from the plan. No new critical or
major issues were introduced; the residual minor/suggestion items were polish,
and the material ones have now been applied (see "Edits applied this pass").

### Previously Identified Issues

- 🔴 **Test Coverage**: `pr_url`/`merge_commit` quoted-sentinel assertions will
  fail (optional extras) — **Resolved**. Replaced with whole-file absence
  assertions; narrative contract corrected throughout.
- 🟡 **Test Coverage / Correctness / Code Quality / Standards / Architecture**:
  `ENG-1234` vs `PP-142` fixture-name inconsistency — **Resolved**. `ENG-1234`
  used uniformly; no `PP-142` remains in the plan.
- 🔵 **Architecture**: AC1 test couples to a resume format 0119 may change —
  **Resolved**. In-code comment added naming `emit_no_input_stall` / 0119 as the
  owner.
- 🔵 **Architecture**: shared-corpus-block idempotency note — **Resolved**, and
  further strengthened this pass to name the mechanism (present non-empty value
  short-circuits the backfill branch).
- 🔵 **Code Quality**: redundant triple validator assertion — **Resolved**.
  Per-file `assert_validates` dropped; `INCIDENT_VOUT` kept as the AC-named
  no-`MISSING-EXTRA` check.
- 🔵 **Correctness**: counter-fixture vacuous `assert_not_contains` —
  **Resolved**. Positive assertion now exact-equality (`assert_eq 'pr_number:
  42'`, verified correct argument order).
- 🔵 **Test Coverage**: TDD sanity-revert validated one coarse mutation —
  **Resolved**, then corrected this pass for mechanism accuracy (see below).
- 🔵 **Code Quality**: counter-fixture/PR430 overlap, hyphenated-key fallback,
  comment duplication — **Resolved** (type-distinction comment added; key
  committed single-spelling, fallback removed; per-file assertion removed).

### New Issues Introduced

None of critical/major severity. Minor/suggestion observations from this pass,
all now addressed in the plan:

- 🔵 **Correctness** (minor): the TDD sanity-revert narration was mechanically
  imprecise — removing the sentinel makes 0007's `self_validate_structural` gate
  abort the migration under `set -e`, so `Phase 4 corpus exits 0` fails *first*,
  rather than a clean migrate with failing per-fixture assertions. **Fixed** —
  the note now describes the abort mechanism.
- 🔵 **Test Coverage / Code Quality** (minor/suggestion, convergent): the
  optional-extra absence assertions used `fm_line` on an absent key, passing
  vacuously on an empty string. **Fixed** — switched to whole-file
  `assert_not_contains "$(cat "$INCIDENT")" 'pr_url:'` so a stray stamping
  anywhere is caught.
- 🔵 **Test Coverage** (suggestion): the fixture-placement line anchors had
  drifted from the host suite. **Fixed** — re-anchored to structural landmarks
  (alongside NODEFAULT/WIDENING heredocs, before `git_init`/`run_0007`) with an
  explicit "create before the run" caveat.

### Edits applied this pass

- Optional-extra absence assertions now inspect the whole migrated file.
- TDD sanity-revert note rewritten to describe the `self_validate_structural`
  abort mechanism accurately.
- Idempotency note extended to name the short-circuit mechanism.
- Fixture-placement instruction re-anchored to structural landmarks.

### Assessment

The plan is in good shape and ready to implement. The one blocking defect (the
optional-extra assertions that could never have gone green) is fixed and
re-verified against source; the cross-cutting naming inconsistency is gone; and
the remaining edits are accuracy/robustness polish. No further review pass is
needed before implementation.

---
*Review generated by /accelerator:review-plan*
