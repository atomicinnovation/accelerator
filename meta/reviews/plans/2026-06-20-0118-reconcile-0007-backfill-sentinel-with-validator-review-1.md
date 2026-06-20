---
type: plan-review
id: "2026-06-20-0118-reconcile-0007-backfill-sentinel-with-validator-review-1"
title: "Plan Review: Reconcile 0007 Backfill Sentinel With Its Validator"
date: "2026-06-20T18:32:43+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-06-20-0118-reconcile-0007-backfill-sentinel-with-validator"
target: "plan:2026-06-20-0118-reconcile-0007-backfill-sentinel-with-validator"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [correctness, test-coverage, code-quality, architecture, safety, portability]
review_number: 1
review_pass: 3
tags: [migrate, migration-0007, corpus-validator, backfill, sentinel]
last_updated: "2026-06-20T21:41:24+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Reconcile 0007 Backfill Sentinel With Its Validator

**Verdict:** REVISE

The plan's actual fix — returning the `unknown` sentinel for an underivable
`pr_number` in `extra_default()` — is correct, idempotent, bash-3.2-safe, and
proven against both validator gates; the test wiring (`RUN_RC`/`DIRECT_RC`/
`DIRECT_ERR`, the red→green transition through `self_validate_structural`, the
date-prefixed fixture) all check out, and the production change reuses the
existing `verdict`/`lenses` sentinel idiom cleanly. The plan is held to REVISE by
two high-confidence, multi-lens defects: (1) its load-bearing claim that "only
`pr_number` reaches the empty branch in the current schema" is factually wrong —
several other required extras reach the identical abort, so the migration's
backfill-vs-validator contradiction is only partially resolved while the plan
asserts it is fully resolved; and (2) the plan's test-reconciliation edit list
misses an existing P4BC assertion (`:1249-1250`) that the fix turns red, so
following the plan verbatim leaves the suite failing. A safety concern (the
sentinel is sticky and silently discards a real-but-underivable PR number) rounds
out the changes needed.

### Cross-Cutting Themes

- **False "only `pr_number`" premise** (flagged by: correctness, architecture) —
  The plan repeatedly asserts `pr_number` is the only required extra that reaches
  `extra_default`'s empty branch, and elevates this to a Manual Verification gate.
  Both lenses independently verified this is false against the current schema:
  `result` (plan-validation), `current_inventory`/`target_inventory` (design-gap),
  the `source`/`source_kind`/`source_location`/`crawler`/`sequence`/
  `screenshots_incomplete` bundle (design-inventory), `review_pass` (reviews), and
  `priority` (work-item) are all required, in-scope extras with neither a
  derivation nor a sentinel — each hits the catch-all `*) printf ''`, the
  `missing-extra-no-default` tolerant branch, and the same `MISSING-EXTRA` abort
  this plan exists to fix. The fix is correct for `pr_number`, but the
  justification, the "unreachable backstop" framing, and the completeness claim
  are all built on a false premise.

- **Missed P4BC assertion the fix turns red** (flagged by: test-coverage,
  code-quality) — Both lenses independently found that the P4BC fixture carries a
  second assertion at `test-migrate-0007.sh:1249-1250` —
  `assert_not_contains "...has no fabricated pr_number" "$(cat …)" "pr_number:"` —
  that the plan does not mention. After the fix the file contains
  `pr_number: unknown`, so this assertion goes red, and the stale block comment at
  `:1218-1220` describing the old masking behaviour is also left untouched.
  Following the plan verbatim leaves the 0007 suite failing, contradicting the
  Phase 2 success criterion.

### Tradeoff Analysis

- **Scope discipline vs. structural completeness** (architecture vs. the plan's
  stated narrow scope): The plan deliberately fixes only `pr_number` and defers
  the general "every required extra is derivable-or-sentinel" invariant to
  0114/0120. That scope choice is defensible — but it is in tension with the
  plan's own claim that Option A "fully resolves the live contradiction." The
  resolution is not to widen scope; it is to make the plan tell the truth about
  what remains unresolved (the residual abort path for the other required extras)
  and explicitly hand it to 0120 rather than asserting closure.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Correctness + Architecture**: False premise — multiple current required
  extras besides `pr_number` reach the empty branch and the same abort
  **Location**: Current State Analysis "Key Discoveries" (lines 84-90); Phase 2
  Manual Verification (lines 332-338); "What We're NOT Doing" (line 90)
  `extra_default` only derives `topic`/`pr_number`/`review_number`/`verdict`/
  `lenses`; `result`, `current_inventory`/`target_inventory`, the design-inventory
  bundle, `review_pass`, and `priority` are all required (not in
  `FM_OPTIONAL_EXTRAS`), their types are in the default doc-type allowlist, and
  none has a derivation or sentinel — so each hits the catch-all and reproduces
  the exact `MISSING-EXTRA` abort. The central correctness/architecture
  justification is wrong and a Manual Verification step directs the implementer to
  confirm a false invariant.

- 🟡 **Test Coverage + Code Quality**: Plan misses a second P4BC assertion
  (`:1249-1250`) that the fix turns red, plus a stale block comment (`:1218-1220`)
  **Location**: Phase 2, Changes Required #1(b) — Direct-run reconciliation
  The plan reconciles only the breadcrumb assertion at `:1243-1244` but the same
  fixture asserts `pr_number:` is absent at `:1249-1250`; after the fix the file
  carries `pr_number: unknown`, so that assertion fails. The header comment at
  `:1218-1220` documenting the old behaviour is also left stale. Following the
  plan verbatim leaves the suite red.

- 🟡 **Architecture**: Open-closed not satisfied — catch-all `*)` still returns
  empty, so the next required extra re-introduces the identical bug
  **Location**: Phase 2, Section 2 (the production change); "What We're NOT Doing"
  The sentinel is keyed to the named `pr_number` case rather than the structural
  property "a required extra with no derivable default." Adding a future required
  extra without its own `extra_default` case re-creates the hard-fail. Either
  return the sentinel from the catch-all (or the loop branch keyed on an empty
  default for a required extra), or explicitly mark the catch-all-stays-empty as
  an accepted, owned-elsewhere risk.

- 🟡 **Architecture**: Tolerant `missing-extra-no-default` branch left intact has
  ambiguous post-fix semantics (benign diagnostic vs. latent abort)
  **Location**: Implementation Approach; Phase 2, Section 2
  After the fix the branch's only trigger is a required extra hitting the
  catch-all — which is still reachable. The plan treats the branch as both a
  "hypothetical future" backstop and (in Manual Verification) a relied-upon
  surfacing mechanism. A future maintainer cannot tell whether reaching it is a
  warning or a migration-stopping bug. Clarify the intended contract and pin which
  of 0114/0120 owns converging the branch with the self-validation gate.

- 🟡 **Safety**: Sentinel is sticky and never re-derived — silently discards a
  real-but-underivable `pr_number` with no recovery path
  **Location**: Phase 2, Section 2; Migration Notes
  `extra_default` derives `pr_number` only from the filename stem; a review that
  genuinely belongs to a real PR but whose stem doesn't encode the number is
  stamped `pr_number: unknown` permanently (the `fm_is_empty_val` guard skips it on
  every re-run, so even a future migration that could derive it won't). The loss
  is silent — nothing flags which files were degraded. Recommend a counted DIVERGE
  breadcrumb on the sentinel-write path mirroring the removed
  `missing-extra-no-default` breadcrumb.

#### Minor

- 🔵 **Correctness**: Tolerant branch characterised as an "unreachable/
  hypothetical-future" backstop, but it is reachable today for several document
  types
  **Location**: "What We're NOT Doing" (line 90); Phase 2 Manual Verification
  (lines 336-338)
  An implementer trusting this framing may conclude the post-fix migration cannot
  abort on any current document type, when it still can for `result`/
  `current_inventory`/etc.

- 🔵 **Test Coverage**: Tracker-keyed and bare-leading-number stems are claimed
  in the edge-case table but not exercised by any new fixture
  **Location**: Testing Strategy — Key edge cases
  The only new no-default fixtures are date-prefixed (the
  `2026-06-20-tracker-keyed-review.md` name is misleading — its stem is
  date-prefixed, not tracker-keyed). A genuinely tracker-keyed stem (e.g.
  `JIRA-1234-foo`) and the bare-leading-number derivation are asserted only in
  prose. Either add the fixtures or trim the table to what is tested.

- 🔵 **Code Quality**: Stale block comment at `test-migrate-0007.sh:1218-1220`
  documents the pre-fix behaviour
  **Location**: Phase 2, Changes Required #1(b)
  (Captured in the cross-cutting P4BC theme; listed here for completeness — add
  the comment rewrite to the Phase 2(b) edit list.)

- 🔵 **Architecture**: Backfill-vs-validator contract enforced only by a
  per-extra guard test, not a shared structural property
  **Location**: "What We're NOT Doing"; Phase 1 Overview
  Required-ness is single-sourced via the schema TSV, but defaultability lives
  solely in `extra_default`. The durable fix is a single cross-check asserting
  every schema-required extra is derivable-or-sentinel-defaulted. 0120's resolved
  Open Question reportedly rejected the generalised lint, so the plan should
  acknowledge the general invariant is intentionally left unguarded.

- 🔵 **Architecture**: Concurrent-edit coupling note with 0114 is slightly
  over-broad
  **Location**: Migration Notes
  0118's only production change is in `extra_default()`'s `pr_number` case, not the
  backfill loop (`0007:502-512`) the note points at. Tighten so a future 0114
  reconciler looks at the derivation function.

- 🔵 **Safety**: Weakening `self_validate_structural` converts a loud "required
  value missing" failure into a quiet accepted sentinel
  **Location**: Current State Analysis; "What We're NOT Doing"
  Acceptable for parity with the existing verdict/lenses sentinels, but pair it
  with the DIVERGE breadcrumb so the weakened gate still leaves an audit trail.

- 🔵 **Portability**: New production comment sits at the 80-column boundary with
  multi-byte glyphs (`→`/`—` are 3 bytes, 1 column)
  **Location**: Phase 2, Section 2 (proposed comment lines 300-302)
  There is no shell autofixer and shfmt does not reflow comments, so an
  over-width line is a hand-fix that blocks CI. Measure rendered column width (not
  bytes) when implementing; consider ASCII `->` to remove the ambiguity.

#### Suggestions

- 🔵 **Test Coverage**: Phase 1 guard test is genuinely useful but its protection
  is purely forward-looking (a characterization test, not a TDD red) — optionally
  strengthen by asserting the fixture's `pr_number: unknown` line is bare/unquoted.

- 🔵 **Test Coverage**: Idempotency is test-backed (via the shared second-run
  empty-diff over the new P4 fixture), not merely claimed — optionally add a
  one-line targeted re-run assertion (exactly one `pr_number: unknown`, no
  duplicate key) so a future re-append regression is localised.

- 🔵 **Code Quality**: `pr_number: unknown` is a slightly surprising magic string
  for a numeric-looking field — optionally retain a `backfilled-sentinel`-style
  breadcrumb so the provenance is discoverable from logs (dovetails with the
  safety breadcrumb suggestion).

- 🔵 **Code Quality**: The now-effectively-unreachable tolerant branch
  (`0007:507-509`) reads as accidental dead code — a one-line comment marking it a
  backstop for future no-default extras (cross-referencing 0120) would help.

- 🔵 **Safety**: Optionally add a one-line note in 0007's catch-all comment that
  an absent required extra here will hard-abort the migration, so the next person
  adding an extra sees the coupling before tripping it.

- 🔵 **Portability**: Keep `sed -i.bak` exactly as written (the `.bak` suffix is
  required for BSD/GNU cross-compatibility; do not drop it). The stray
  `$TMP/ok-pr-unknown.md.bak` is harmless because the fixture is validated by
  explicit path, not a directory glob — don't "fix" it by switching to bare
  `sed -i` or by glob-validating `$TMP`.

### Strengths

- ✅ The production change is a minimal, co-located one-line guard placed in the
  pure derivation function (the functional core), reusing the existing
  `verdict`/`lenses` `unknown` sentinel verbatim — KISS, DRY, and testable via the
  unit seam without orchestration.
- ✅ The sentinel-clears-both-gates reasoning is verified correct against the
  validator source: `MISSING-EXTRA` is key-presence only and `EMPTY-PLACEHOLDER`
  rejects only `""`/`[]`, so a bare `pr_number: unknown` trips neither.
- ✅ Idempotency holds and is genuinely test-backed: the `fm_is_empty_val` guard
  skips a file already carrying the sentinel, and the existing Phase 4 second-run
  empty-diff assertion now covers the sentinel-bearing file.
- ✅ The plan correctly diagnoses the masking bug in the existing P4BC fixture (it
  never asserts an exit code) and adds the missing exit-code gate plus a real
  end-to-end red→green transition (`RUN_RC == 0` + `assert_validates`).
- ✅ All assumed test-harness references are real and accurately used
  (`DIRECT_RC`/`DIRECT_ERR`/`RUN_RC`/`PR430F`/`fm_line`/`assert_validates`/
  `emit_valid` emitting required extras as `name: "x"`).
- ✅ Every proposed shell construct is bash-3.2-safe and reuses idioms already
  proven portable in the exact files touched (`grep -oE`, POSIX `case` globs,
  `[ -n ] ||`, `sed -i.bak` anchored substitution — byte-compatible BSD/GNU).
- ✅ Recovery is sound: 0007 is fully VCS-recoverable and the change fails forward
  to completion rather than aborting mid-pipeline.

### Recommended Changes

1. **Correct the "only `pr_number`" claim and drop the false Manual Verification
   gate** (addresses: the false-premise major; the "unreachable backstop" minor).
   Rewrite Current State Analysis lines 84-90 to acknowledge that `result`,
   `current_inventory`/`target_inventory`, the design-inventory bundle,
   `review_pass`, and `priority` also reach the catch-all/empty branch today, and
   explicitly scope them out as a known residual owned by 0114/0120. Remove or
   rewrite the Phase 2 Manual Verification step (lines 332-338) so it does not
   instruct the implementer to confirm a false invariant.

2. **Add the missing P4BC test reconciliation to the Phase 2(b) edit list**
   (addresses: the missed-`:1249-1250` major; the stale-comment minor). Explicitly
   call out deleting or inverting the `:1249-1250` "no fabricated pr_number"
   assertion (it is superseded by the new `pr_number: unknown` assertion) and
   rewriting the stale block comment at `:1218-1220` to match the reconciled
   behaviour. Re-confirm the Phase 2 "0007 suite passes" criterion against this.

3. **Decide and document the seam / open-closed posture** (addresses: the
   open-closed major; the tolerant-branch-semantics major; the contract-enforcement
   minor). Either move the sentinel to the catch-all `*)` (or the loop branch keyed
   on an empty default for a required extra) so the invariant holds for all required
   extras, or — keeping the narrow `pr_number` placement — state explicitly that the
   catch-all remaining empty is a deliberate accepted risk, clarify whether the
   retained tolerant branch is a benign diagnostic or a hard guard post-fix, and
   pin the general invariant to 0120 (noting 0120 reportedly rejected the
   generalised lint).

4. **Add a sentinel-write breadcrumb and document the sticky-loss tradeoff**
   (addresses: the safety sticky-sentinel major; the gate-weakening minor; the
   provenance suggestion). Emit a counted DIVERGE breadcrumb on the sentinel-write
   path (e.g. `0007-DIVERGE[pr_number-sentinel]: <file> — no PR number derivable;
   stamped unknown`) so operators can see which files were degraded, and note in
   Migration Notes that the sentinel is sticky-by-design with a manual
   reconciliation path.

5. **Tighten test edge-case coverage to match the documented table** (addresses:
   the tracker-keyed/bare-number minor). Either add a genuinely tracker-keyed
   fixture (non-date stem, no leading digit → `unknown`) and rename the misleadingly
   named date-prefixed fixture, or trim the edge-case table to the stem shapes
   actually exercised.

6. **Fix-up nits during implementation** (addresses: portability comment width;
   coupling-note over-breadth). Measure the new comment's rendered column width
   (use ASCII `->`), and tighten the 0114 coupling note to point at
   `extra_default()` rather than the backfill loop.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Correctness

**Summary**: The core fix is logically sound: returning the `unknown` sentinel
for an underivable `pr_number` correctly clears both validator gates
(MISSING-EXTRA is key-presence only; EMPTY-PLACEHOLDER rejects only `""`/`[]`),
is idempotent via the `fm_is_empty_val` guard, is bash-3.2-safe, and the
date-prefixed/tracker-keyed fixtures genuinely reach the no-fallback branch. The
test wiring is accurate — `DIRECT_RC`/`DIRECT_ERR`/`RUN_RC`/`PR430F` all exist,
and the red/green transition through `self_validate_structural` is correctly
reasoned. However, the plan's load-bearing claim that "only `pr_number` reaches
the empty branch in the current schema" is factually wrong: `result`
(plan-validation), `current_inventory`/`target_inventory` (design-gap), and the
`source`/`source_kind`/`source_location`/`crawler`/`sequence`/
`screenshots_incomplete` bundle (design-inventory) are all required, in-scope
extras with neither a derivation nor a sentinel, so they hit the identical abort
the plan is fixing.

**Strengths**:
- The sentinel-clears-both-gates reasoning is verified correct against the
  validator source: MISSING-EXTRA uses `bk_present` (key-presence only, :345) and
  EMPTY-PLACEHOLDER matches only `""`/`[]` (:354-356).
- Idempotency claim holds: the loop's `fm_is_empty_val` guard (0007:504) treats
  only `''`/`'""'`/`'[]'` as empty, so a re-run skips a file already carrying
  `pr_number: unknown`.
- The Phase 4 end-to-end fixture stem `2026-06-20-tracker-keyed-review` is
  genuinely date-prefixed (matches the no-fallback glob) and pr-token-less, so it
  truly reaches the empty branch.
- Test-harness references are all real: `DIRECT_RC`/`DIRECT_ERR` (run_0007_direct),
  `RUN_RC` (run_0007), `PR430F`/`fm_line`/`assert_validates`.
- The red/green transition is correctly reasoned: `self_validate_structural`
  (0007:771) runs inside the `{ } >&2` set -e block before `harness_run` (:788).

**Findings**:
- **major / high** — *False premise: multiple current required extras besides
  pr_number reach the empty branch* — Location: Current State Analysis "Key
  Discoveries" (84-90), Phase 2 Manual Verification (332-338). The plan asserts
  `pr_number` is the only required extra reaching the empty branch and uses it to
  claim Option A "fully resolves the live contradiction." `result`,
  `current_inventory`/`target_inventory`, and the design-inventory bundle are all
  required (none in `FM_OPTIONAL_EXTRAS`), all in the default doc-type allowlist,
  none with a derivation or sentinel — each hits `*) printf ''`, the
  `missing-extra-no-default` branch, and the same `self_validate_structural`
  MISSING-EXTRA abort. The Manual Verification step instructs confirming a false
  statement.
- **minor / high** — *Tolerant branch characterised as an unreachable/
  hypothetical-future backstop, but it is reachable today* — Location: line 90,
  lines 336-338. Given the above, the branch is reachable now for several current
  document types, not just hypothetically.

### Test Coverage

**Summary**: The plan is unusually rigorous for its size: it correctly identifies
that the existing P4BC fixture is green-while-masking-the-bug (never asserts an
exit code), and adds a true end-to-end red gate (RUN_RC==0 + assert_validates)
plus derivable-path no-regression coverage. However, it under-specifies the P4BC
reconciliation: it overlooks an existing `assert_not_contains "pr_number:"`
assertion at :1249-1250 that the fix will turn red, so the plan as written would
not leave the suite green. Edge-case coverage is asserted in prose (tracker-keyed,
date-prefixed, bare-leading-number) but only one stem shape is actually exercised
by a fixture.

**Strengths**:
- Correctly diagnoses that the P4BC fixture is green while masking the abort
  (never asserts DIRECT_RC) and plans to add the missing exit-code assertion.
- Phase 2(a) creates a genuine red-then-green gate; RUN_RC, DIRECT_RC, DIRECT_ERR,
  and fm_line all verified to exist with the assumed semantics.
- Adds an explicit no-regression guard on the derivable path (assert_not_contains
  'unknown' on PR430).
- All assumed helper signatures are accurate (assert_accepts, assert_absent,
  assert_contains/not_contains, emit_valid emitting required extras as `name: "x"`
  with pr_url/merge_commit skipped as FM_OPTIONAL_EXTRAS).

**Findings**:
- **major / high** — *Plan misses a second P4BC assertion that the fix will turn
  red* — Location: Phase 2 §1(b). The fixture has a second assertion at
  :1249-1250 (`assert_not_contains "...has no fabricated pr_number" … "pr_number:"`)
  that goes red after the fix; the plan does not mention removing/inverting it, so
  the suite fails if followed verbatim.
- **minor / high** — *Tracker-keyed and bare-leading-number stems are claimed but
  not fixture-covered* — Location: Testing Strategy. The only new no-default
  fixtures are date-prefixed (the `tracker-keyed-review` name is misleading). A
  genuinely tracker-keyed stem and bare-leading-number derivation are asserted only
  in prose.
- **minor / medium** — *Phase 1 guard test is genuinely useful but its
  red-protection is only forward-looking* — Location: Phase 1. It is a
  characterization/contract test, not a TDD red (honestly stated). Optionally
  strengthen by asserting the bare/unquoted `pr_number: unknown` line.
- **suggestion / high** — *Idempotency is asserted, not merely claimed — but only
  via the shared second-run diff* — Location: Testing Strategy. Consider a
  targeted re-run assertion (one `pr_number: unknown`, no duplicate key).

### Code Quality

**Summary**: The plan is unusually well-grounded: a single one-line change to
extra_default() reusing the existing verdict/lenses 'unknown' sentinel idiom
verbatim, with an accurate, proportionate comment, and test additions that follow
the suite's emit_valid/assert_*/fm_line idioms exactly. The main maintainability
risks are not in the production change but in test-reconciliation completeness —
two existing assertions/comments in the P4BC block describe pre-fix behaviour and
will be left stale or broken but are not addressed in the plan's edit list.

**Strengths**:
- Minimal co-located one-line guard, no new abstraction; KISS/DRY served by
  reusing the verdict/lenses sentinel.
- The inline comment is accurate and proportionate (trigger, precedent, why).
- Test additions faithfully mirror existing idioms.
- Correctly identifies the sentinel is emitted as a bare YAML scalar; idempotency
  via fm_is_empty_val.

**Findings**:
- **major / high** — *Existing P4BC assertion at :1249-1250 contradicts post-fix
  state but is not in the edit list* — Location: Phase 2 §1(b). (Same defect as the
  test-coverage major.) Leaving it produces a confusing red.
- **minor / high** — *Block header comment at :1218-1220 documents old behaviour
  and is not updated* — Location: Phase 2 §1(b). Stale leading comment that
  contradicts the assertions below it.
- **suggestion / medium** — *`pr_number: unknown` is a magic string; only the code
  comment documents semantics* — Location: Desired End State / Migration Notes.
  Consider retaining a `backfilled-sentinel`-style breadcrumb for provenance.
- **suggestion / medium** — *Now-unreachable tolerant branch reads as accidental
  dead code* — Location: Phase 2 §2. A one-line backstop comment cross-referencing
  0120 would help.

### Architecture

**Summary**: A narrowly-scoped, structurally-clean fix: co-locating the sentinel
in `extra_default()`'s `pr_number` case reuses the existing convention and places
the fix at the data-derivation seam (functional core) rather than the imperative
loop; the scope boundary with 0114/0120 is explicit and the validator-vs-backfill
contract is locked by a Phase-1 guard test. The main weakness is the plan's
central justification — that the tolerant "left absent" branch is an "unreachable
backstop" — which does not hold: several other required extras still reach the
catch-all `*)` and reproduce the same hard-fail, leaving the contradiction
unresolved at the architectural level and deferred to 0120 as a recurring gap.

**Strengths**:
- Sentinel placed in the pure derivation function preserves testability via the
  unit seam.
- Reuses the established `unknown` convention rather than a new token/mechanism.
- Phase-1 guard test pins the backfill-vs-validator contract.
- Scope boundaries with 0114/0120 explicitly stated; concurrent-edit coupling
  called out.
- Idempotent by construction.

**Findings**:
- **major / high** — *False premise: the tolerant branch is reachable today for
  multiple required extras* — Location: Current State, "only pr_number…". `result`,
  `review_pass`, the design-inventory bundle, `current_inventory`/
  `target_inventory`, and `priority` (no REFUSE like `kind`) hit the catch-all →
  empty → tolerant branch → MISSING-EXTRA abort. The structural defect is
  mis-classified as dead-code debt when it is a still-reachable single point of
  failure; the residual gap is silently deferred.
- **major / high** — *Open-closed not satisfied: catch-all `*)` still returns
  empty* — Location: "What We're NOT Doing" / Phase 2 §2. Adding a future required
  extra without its own case re-introduces the bug. Consider returning the sentinel
  from the catch-all (or the loop branch keyed on an empty default for a required
  extra), or explicitly mark the catch-all-empty as an accepted owned-elsewhere
  risk.
- **major / medium** — *Tolerant branch left intact has ambiguous post-fix
  semantics* — Location: Implementation Approach / Phase 2 §2. The plan treats it
  as both a hypothetical-future backstop and a relied-upon surfacing mechanism;
  reaching it is either a benign warning or a migration-stopping bug depending on
  the extra. Clarify the contract; pin ownership to 0114/0120.
- **minor / medium** — *Contract enforced only by a per-extra guard test, not a
  shared structural property* — Location: "What We're NOT Doing" / Phase 1.
  Required-ness is single-sourced via the schema TSV; defaultability lives solely
  in `extra_default`. The durable fix is a single cross-check over every required
  extra (0120 reportedly rejected the generalised lint — acknowledge the invariant
  is intentionally unguarded).
- **minor / high** — *Concurrent-edit coupling note with 0114 is over-broad* —
  Location: Migration Notes. 0118's only production change is `extra_default()`,
  not the loop (0007:502-512); tighten the note.

### Safety

**Summary**: A destructive in-place corpus migration mutating real meta/ files;
the change is narrowly scoped, idempotent, and well-reasoned about blast radius —
the VCS-revert recovery path is explicit and the fm_is_empty_val guard protects
files that already carry a real pr_number. The primary concern is data-quality,
not data-loss: writing a sticky, never-re-derived `unknown` sentinel into files
that genuinely belong to a real PR can silently lose the connection with no later
recovery, and nothing surfaces which files were stamped. Letting
self_validate_structural pass on a previously-aborted state is defensible parity
but does weaken a gate.

**Strengths**:
- Recovery path is sound and explicit (fully VCS-recoverable).
- Idempotency correctly preserved; second-run empty-diff covers the sentinel file.
- Blast radius tightly bounded; derivable path provably untouched (PR430
  no-regression assertion).
- Fails forward to completion rather than aborting mid-pipeline.

**Findings**:
- **major / high** — *Sentinel is sticky and never re-derived — silently loses a
  real-but-underivable pr_number with no recovery* — Location: Phase 2 §2 /
  Migration Notes. `extra_default` derives `pr_number` only from the stem; a real
  PR's review with a non-encoding stem is stamped `unknown` permanently
  (fm_is_empty_val skips it on re-run forever). The loss is silent. Recommend a
  counted DIVERGE breadcrumb on the sentinel-write path and documenting the
  sticky-by-design behaviour + manual reconciliation path.
- **minor / medium** — *Weakening self_validate_structural masks a genuine
  data-quality signal* — Location: Current State / "What We're NOT Doing".
  Acceptable for parity, but pair with the DIVERGE breadcrumb for an audit trail.
- **suggestion / medium** — *No safeguard if a future required extra reaches the
  catch-all and stays absent* — Location: Phase 2 Manual Verification. Forward-
  looking; deferral to 0120 is reasonable. Optionally add a one-line note in the
  catch-all comment.

### Portability

**Summary**: Solid from a portability standpoint: every proposed shell construct
is bash-3.2-safe and reuses idioms already proven portable in the exact files
touched. The production one-liner (`[ -n "$n" ] || n='unknown'`, `grep -oE`, POSIX
`case` globs) introduces no bash-4 features, GNU-only flags, or locale/path
assumptions, and `sed -i.bak 's/^X$/Y/'` behaves identically on BSD (macOS) and
GNU (Linux) sed. The only risks are non-blocking: new comments at the 80-column
boundary (no autofixer), and a `.bak` temp file left by the new fixture rewrite
(confirmed harmless).

**Strengths**:
- Reuses constructs already present in the same file (`grep -oE '^[0-9]+'`, POSIX
  bracket-glob `case`).
- `[ -n "$n" ] || n='unknown'` is pure POSIX; passes the bashisms linter.
- `sed -i.bak` is the one spelling byte-compatible between BSD and GNU sed (15+
  existing uses in the suite).
- Bare-scalar `unknown` avoids cross-tool encoding/quoting divergence.
- `export LC_ALL=C` already set in the validator suite.

**Findings**:
- **minor / medium** — *New production comment sits at the 80-column boundary with
  multi-byte glyphs* — Location: Phase 2 §2 (lines 300-302). `→`/`—` are 3 bytes /
  1 column; no shell autofixer and shfmt does not reflow comments. Measure rendered
  column width when implementing; consider ASCII `->`.
- **suggestion / high** — *Keep `sed -i.bak` exactly as written* — Location: Phase
  1 §1 (line 174). The `.bak` suffix is required for BSD/GNU compatibility; the
  stray `$TMP/ok-pr-unknown.md.bak` is harmless because the fixture is validated by
  explicit path, not a directory glob. Do not switch to bare `sed -i` or
  glob-validate `$TMP`.

## Re-Review (Pass 2) — 2026-06-20

**Verdict:** REVISE

The plan was rewritten between passes: the fix was **widened** from a
`pr_number`-only edit in `extra_default` to the backfill loop's generic
no-derivable-default branch (covering every required extra), an audit breadcrumb
was added, and the false premise + test-reconciliation gaps were corrected. The
re-review confirms **every pass-1 major and minor is resolved** — but the
widening introduced one **critical** correctness defect (a quoted-vs-bare
emission error in the new test) and one **major** safety consequence
(string-typing of integer/boolean fields), so the plan remains REVISE.

### Previously Identified Issues
- 🟡 **Correctness + Architecture**: False "only `pr_number`" premise — **Resolved.**
  Current State now enumerates `result`/`current_inventory`/etc.; the widened
  loop-branch fix actually covers them.
- 🟡 **Test Coverage + Code Quality**: Missed P4BC `:1249-1250` assertion + stale
  `:1218-1220` comment — **Resolved.** Phase 2(c) now explicitly removes the
  assertion and rewrites the comment. (Code-quality flags a residual: the `:1247`
  assertion *label* is still stale — new minor below.)
- 🟡 **Architecture**: Open-closed / catch-all stays empty — **Resolved.** The
  loop branch keys the sentinel on the structural property, so a future required
  extra cannot silently re-introduce the abort.
- 🟡 **Architecture**: Tolerant-branch ambiguous semantics — **Resolved.** The
  branch now unconditionally writes + breadcrumbs; no latent-abort ambiguity.
- 🟡 **Safety**: Sticky sentinel silent loss — **Resolved (mechanism), widened
  (scope).** The per-file `backfill-sentinel` breadcrumb addresses the audit gap;
  but widening enlarges the blast radius (new major below).
- 🔵 Minors (tracker-keyed fixture, contract-enforcement note, coupling-note
  over-breadth, comment width, idempotency assertion) — **Resolved or carried
  forward** as noted below; the coupling note is correctly retargeted to the loop
  branch.

### New Issues Introduced
- 🔴 **Critical / Correctness**: The sentinel emits **quoted** for all
  `awk:223`-routed scalar extras (`result: "unknown"`, not bare `result:
  unknown`) because `fm_normalise_value` wraps bare values in quotes; only
  `pr_number`/`review_number` (the dedicated `awk:222` branch) emit bare. The
  WIDENING test asserts the bare form (`'result: unknown'`) via fixed-string
  match, so it would **fail post-fix**; the "Not quoting the sentinel" scope claim
  and the "bare unquoted scalars" Manual Verification item are wrong for every
  non-`pr_number` extra. (Verified against the existing `verdict: "unknown"` test
  assertion.) — Location: Phase 2 §1(b), "What We're NOT Doing", Manual Verification.
- 🔴 **Major / Safety**: Widening stamps `unknown` (a YAML string) onto extras
  whose natural type is integer (`sequence`, `review_pass`) or boolean
  (`screenshots_incomplete`), and an invalid enum value onto `priority`. The
  visualiser parses frontmatter into typed JSON (`Bool`→`J::Bool`, `Number`→
  `J::Number`), so this is a real, sticky type change propagated to consumers —
  affecting more fields/types than the pre-widening `pr_number`-only fix. —
  Location: Phase 2 §2, Migration Notes.
- 🟡 **Minor / Architecture**: The widened branch sentinels typed extras
  (`priority`, `sequence`) and, being sticky against *future* migrations (not just
  re-runs), can pre-empt/mask the proper-derivation work nominally owned by 0114.
- 🔵 **Minor / Correctness**: Edit-site citation `0007:507-510` should explicitly
  note the pack step it falls through to at `0007:511` must remain untouched.
- 🔵 **Minor / Code Quality**: The new breadcrumb uses ASCII `--` where every
  existing `0007-DIVERGE` breadcrumb (and the awk emitter) uses the em-dash
  idiom; the `:1247` assertion label still describes the old "mid-rewrite abort"
  behaviour.
- 🔵 **Minor / Test Coverage**: The `backfill-sentinel` breadcrumb is asserted by
  presence only (not by file-name/count), so the per-file audit-trail contract
  the Migration Notes rely on is not actually verified; idempotency for the
  sentinel files is covered only by the aggregate second-run diff.
- 🔵 **Suggestion / Safety**: No structural guard prevents a *future* list-valued
  required extra from being emitted as a malformed scalar `unknown`; consider an
  end-of-run summary count of stamped files.

### Assessment
The rewrite is a clear improvement — the structural objections from pass 1 are
all genuinely resolved, and the widening + breadcrumb make the fix more complete
and auditable. Two issues block approval: (1) the **quoting defect** is a
clear-cut factual fix (correct the WIDENING assertion to `result: "unknown"` and
the bare-scalar claims), and (2) the **type-coercion blast radius** from widening
is a genuine tradeoff that needs a decision — accept-and-document (string-typing
typed fields is tolerable for this dev-tooling corpus, guarded by the breadcrumb)
versus re-scoping the sentinel to string-typed extras only. Once those are
addressed the plan should reach APPROVE; the remaining items are minors.

---
*Re-review generated by /accelerator:review-plan*

## Re-Review (Pass 3) — 2026-06-20

**Verdict:** COMMENT

Focused pass over the new HYBRID shape (typed defaults for the numeric/boolean
extras + the awk bare-emission change, which no prior pass had reviewed against).
The pass-2 blockers are confirmed resolved, and the hybrid is **verified correct**
against the source: the typed defaults divert `review_pass`/`sequence`/
`screenshots_incomplete` away from the loop sentinel; the bare-print branch emits
them unquoted (type preserved) while `fm_normalise_value` quotes the string
extras; the validator is provably extra-agnostic (no numeric/bool/enum shape
gate); and the HYBRID design-inventory fixture validates clean (0007 backfills the
anchored provenance bundle). No critical, two majors (both clear-cut, now fixed),
so the verdict lands at COMMENT — the plan is implementation-ready.

### Previously Identified Issues (pass 2)
- 🔴 **Critical / Correctness**: quoted-vs-bare emission — **Resolved** (pass 2
  fixes verified: `result: "unknown"` quoted, `pr_number: unknown` bare, against
  `fm_normalise_value`).
- 🔴 **Major / Safety**: type coercion of numeric/boolean fields — **Resolved.**
  Pass-3 safety verified the *complete* set of numeric/bool required extras is
  exactly `review_number` (pre-existing) + the three now given typed defaults;
  none is missed, and the visualiser's YAML→JSON parser confirms types are
  preserved. `screenshots_incomplete: true` is an advisory flag (cannot
  hide/drop content), so the conservative default is fail-safe.

### New Issues (pass 3) — all now addressed
- 🔴 **Major / Portability + Correctness**: the proposed awk `else if` used
  backslash line-continuation; the file's idiom (and the one-true-awk-safe form)
  is breaking *after* trailing `||` with no backslash (`awk:62-64`). **Fixed** —
  snippet rewritten to the trailing-`||` form with rationale.
- 🟡 **Major / Test Coverage**: the third typed default `review_pass` was
  exercised by no fixture (a regression emitting quoted `"1"` would pass).
  **Fixed** — added a plan-review-missing-`review_pass` fixture asserting bare
  `review_pass: 1`.
- 🔵 **Minor / Test Coverage**: bare typed-default assertions used substring
  `assert_contains`. **Fixed** — switched to exact `assert_eq`; also added
  `source_kind`/`source_location`/`crawler` sentinel assertions and a
  breadcrumb-names-the-extra assertion.
- 🔵 **Minor / Safety**: `sequence: 1` fabricates a colliding value in the
  design-gap resolver's primary tiebreaker, written without a breadcrumb.
  **Addressed** — Migration Notes caveat added (resolver degrades to mtime/date;
  operators treat backfilled `sequence: 1` as unauthoritative).
- 🔵 **Minor / Safety**: `priority: unknown` (and other enum sentinels) is an
  out-of-vocabulary enum value. **Addressed** — Migration Notes note added
  (validator is enum-agnostic; visualiser passes strings through; future
  enum-switching consumers must degrade gracefully).
- 🔵 **Suggestion**: dedicated tracker-keyed fixture — left optional (the same
  branch is already covered behaviourally).

### Assessment
The plan is sound and implementation-ready. The core mechanism is verified
correct at the source, the type-coercion hazard is fully eliminated by the
hybrid, the awk change now matches the codebase's portable idiom, and all three
typed defaults plus the string/enum sentinel routing are test-covered with exact
assertions. Remaining items are documentation caveats and an optional fixture.

### Approval — 2026-06-20

**Verdict: APPROVE.** All pass-1/2/3 critical and major findings are resolved in
the plan; the pass-3 majors (awk idiom, `review_pass` coverage) and the cheap
minors are applied. Plan status set to `ready`; cleared for
`/accelerator:implement-plan`.

---
*Re-review generated by /accelerator:review-plan*
