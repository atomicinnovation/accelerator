---
type: plan-review
id: "2026-06-15-0105-close-corpus-validator-provenance-and-linkage-blind-spots-review-1"
title: "Plan Review: Close the Corpus Validator Provenance and Linkage Blind Spots"
date: "2026-06-15T20:21:42+00:00"
author: Toby Clemson
producer: review-plan
status: complete
target: "plan:2026-06-15-0105-close-corpus-validator-provenance-and-linkage-blind-spots"
parent: "plan:2026-06-15-0105-close-corpus-validator-provenance-and-linkage-blind-spots"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [correctness, code-quality, test-coverage, architecture, portability, standards]
review_number: 1
review_pass: 3
tags: [frontmatter, schema, validator, provenance, linkage]
last_updated: "2026-06-15T21:18:25+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Close the Corpus Validator Provenance and Linkage Blind Spots

**Verdict:** REVISE

This is a high-quality, deeply-researched plan: every line reference checks
out against source, the phase decomposition (1 & 2 independent, 3 depending on
both) is correct, the single-oracle consolidation is architecturally sound, and
the new rules stay genuinely data-driven. The reject-side test coverage is
strong and the TDD discipline is exemplary. The verdict is REVISE rather than
APPROVE because five major issues would either fail the plan's own
`scripts:check`/specificity success criteria or leave the riskiest new code path
(the comma-splitting tokenizer) unguarded: 80-column overflow on the new message
lines, a diagnostic-code superstring that defeats `assert_rejects` specificity,
an unquoted-`$rest` glob-expansion hazard, a missing multi-element accept
fixture, and the deletion of liveness self-tests without a full automated
replacement. None is structural — all are addressable with small, local edits to
the plan before implementation.

### Cross-Cutting Themes

- **Unquoted `$rest` / `$elem` glob expansion** (flagged by: correctness,
  portability) — the rewritten tokenizer iterates `for elem in $rest` under
  `IFS=','` with `$rest` unquoted and no `set -f`, so word-splitting also
  performs pathname (glob) expansion. Latent today (corpus ids are
  `[A-Za-z0-9.-]` only) but a real, cwd-dependent nondeterminism hazard the
  current quoted-regex `while` loop does not have. Both lenses independently
  recommend wrapping the split in `set -f`/`set +f`, mirroring the `oldifs`
  save/restore.
- **The `${rest%%#*}` inline-comment strip** (flagged by: correctness,
  code-quality, architecture, portability) — strips from the *first* `#` across
  the whole value before splitting. The inline rationale ("refs contain no
  `#`") states the wrong invariant, the behaviour diverges subtly from the old
  regex, and it re-encodes a grammar fact that lives in `FM_TYPED_REF_RE`.
- **Diagnostic specificity & message wording** (flagged by: correctness,
  code-quality, standards) — `FORBIDDEN-PROVENANCE-NONANCHORED` is a superstring
  of `FORBIDDEN-PROVENANCE` (defeats the `grep -qF` substring match in
  `assert_rejects`), and the reused `BAD-LINKAGE-SHAPE` code now carries two
  divergently-worded messages.
- **Phase 3 coverage retirement** (flagged by: code-quality, test-coverage,
  architecture) — deleting the blind-spot liveness self-tests (`:404-422`)
  removes both negative-wiring proofs and clean-control assertions; the fold-in
  replaces the reject intent but the accept-side controls and an automated
  vacuity guard are only partly re-established.

### Tradeoff Analysis

- **One code per violation class vs. triage clarity**: standards endorses
  reusing `BAD-LINKAGE-SHAPE` (consistent with the validator's documented
  convention), while code-quality notes the two divergent message strings under
  one code are a minor readability cost. Recommendation: keep the single code
  (correct per convention) but align the two messages on a shared tail so the
  unquoted case reads as a qualifier, not a separate rule.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Standards**: New violation message lines exceed the project's 80-column limit
  **Location**: Phase 1 §2 (The rule) and Phase 2 §2 (The rewritten tokenizer)
  The Phase 1 `FORBIDDEN-PROVENANCE-NONANCHORED` violation line is ~110 columns
  and the Phase 2 unquoted-arm line ~105; shfmt (run by `scripts:check`) does not
  wrap string-argument lines and shell has no autofixer, so these will fail the
  gate each phase's success criteria depend on. Pre-plan the wrap (hoist the
  message to a `local`, or split with the file's `cmd ||` continuation style).

- 🟡 **Correctness**: `FORBIDDEN-PROVENANCE-NONANCHORED` is a superstring of `FORBIDDEN-PROVENANCE`, defeating `assert_rejects` specificity
  **Location**: Phase 1 §2 (The rule); Key Discoveries
  `assert_rejects` matches with `grep -qF -- "$code"` (`frontmatter-fixtures.sh:79`),
  so any test asserting `FORBIDDEN-PROVENANCE` is also satisfied by output
  emitting only the NONANCHORED variant — silently weakening AC3 ("specific
  diagnostic, not merely non-zero exit") on the provenance axis. Prefer a
  non-superstring code (e.g. `PROVENANCE-ON-NONANCHORED`).

- 🟡 **Correctness / Portability**: Unquoted `$rest` word-split runs with glob (pathname) expansion enabled
  **Location**: Phase 2 §2 (The rewritten tokenizer, the `for elem in $rest` loop)
  Under `IFS=','` an unquoted expansion word-splits *and* glob-expands; the
  validator runs `set -euo pipefail` but never `set -f`. Latent (no corpus value
  currently survives the single bracket strip with a glob char) but a real
  cwd-dependent nondeterminism the old quoted-regex loop avoided. Wrap the split
  in `set -f` / `set +f` (saved/restored like `oldifs`).

- 🟡 **Test Coverage**: No accept-side fixture for a well-formed *multi-element* quoted list
  **Location**: Phase 2 §1 (Failing fixtures); Testing Strategy edge case 5
  Comma-splitting is the exact new behaviour, yet the only checked-in
  bracketed-list accept fixture is single-element (`ok-dotted-linkage`,
  `test-validate-corpus-frontmatter.sh:80`); the multi-element accept case
  appears only as a *manual* step. Add an `assert_accepts` for
  `relates_to: ["adr:0001", "adr:0002"]` (ideally with irregular inter-element
  spacing) as a green-side regression guard.

- 🟡 **Test Coverage**: Deleting the liveness self-tests removes negative-wiring proofs and clean-control accept assertions
  **Location**: Phase 3 §1 (Remove the helpers and their wiring)
  `:404-422` includes both "this check *can* fail" proofs and accept controls
  (anchored provenance allowed `:413-414`; quoted linkage accepted `:421-422`).
  The fold-in re-adds reject assertions but not a paired accept control for
  every new axis (notably non-anchored-provenance-*absent*) nor an automated
  vacuity guard. Confirm the conditional-axis accept cases (`:356,:358,:365,:367`)
  remain and extend the `assert_axis_mutation` block to the two new axes.

#### Minor

- 🔵 **Correctness**: Whole-value `${rest%%#*}` comment strip can drop a malformed bad tail
  **Location**: Phase 2 §2 (comment-strip guard)
  Applied to the whole value before splitting, so an element after an in-list
  `#` is discarded rather than flagged — a subtle weakening of the very
  blind-spot the phase closes. Either document as intentional or move the `#`
  strip to a per-element step on the final element.

- 🔵 **Portability**: `${rest%%#*}` can corrupt a value containing a literal `#`, yielding a truncated diagnostic
  **Location**: Phase 2 §2 (line 319)
  `parent: a#b` is truncated to `a` before the shape check; still rejected, but
  the message shows a truncated token. Optionally only strip when preceded by
  whitespace (`${rest%% #*}`), matching YAML's inline-comment rule.

- 🔵 **Code Quality**: Inline-comment-strip comment states the wrong invariant
  **Location**: Phase 2 §2 (comment-strip guard + rationale bullet)
  `%%#*` strips from the first `#`; the safety claim that matters is "no value
  byte before the comment is `#`", not "refs contain no `#`". The rationale
  bullet also describes the old regex's tolerance via a different mechanism.
  Reword to the actual invariant.

- 🔵 **Code Quality**: Tokenizer re-implements quote-stripping instead of reusing `fm_inner`
  **Location**: Phase 2 §2 (the `'"'*'"'` case arm)
  The file already has `fm_inner()` (`:127-140`) used everywhere else a quoted
  value is unwrapped; the new manual strip creates a second dequoting idiom.
  Either add a one-line note that the same-quote-pair strip is deliberate, or
  route through `fm_inner`.

- 🔵 **Architecture**: Embedded empty quoted element in a list is silently accepted
  **Location**: Phase 2 §2 (the empty-`inner` `continue`)
  `relates_to: ["plan:0001", ""]` is neither the literal `[]`/`""` that
  `EMPTY-PLACEHOLDER` matches nor flagged by the tokenizer — a narrow new
  acceptance gap. Acknowledge in "What We're NOT Doing" or emit
  `EMPTY-PLACEHOLDER` for an embedded empty element.

- 🔵 **Architecture**: Tokenizer hard-codes YAML/grammar assumptions, softly re-encoding the contract
  **Location**: Phase 2 §2 (comment/bracket strip)
  "Refs contain no `#`" and "ids cannot contain `[`/`]`/`,`" are facts living in
  `FM_TYPED_REF_RE`; encoding them procedurally a second time risks silent drift
  if the charset widens. Cross-reference `FM_TYPED_REF_RE` in a comment.

- 🔵 **Architecture**: Template/skill text-inspection coverage is retired, not just deleted
  **Location**: Phase 3 §2 (Fold intent into the conditional-axis section)
  The helpers inspected the *templates/skills themselves*; the fold-in asserts
  the *validator rejects synthesised fixtures*. Make explicit that the section-5
  real-corpus sanity check is what now backstops live artifacts, so the tradeoff
  is acknowledged.

- 🔵 **Test Coverage**: The no-double-flag invariant for empty values is unverified
  **Location**: Testing Strategy edge case 6 / Phase 2 rationale
  `bad-empty.md` (`:70-71`) only asserts `EMPTY-PLACEHOLDER` is present and would
  still pass if a spurious `BAD-LINKAGE-SHAPE` were also emitted. Add a negative
  assertion that empty values do NOT also emit `BAD-LINKAGE-SHAPE`.

- 🔵 **Test Coverage**: Documented trailing-comment / trailing-comma tolerances are untested
  **Location**: Phase 2 §2 (comment-strip + empty-element skip)
  These are explicitly-intended new behaviours with no regression fixture. Add an
  `assert_accepts` for a quoted ref with a trailing inline comment (and, if the
  corpus can produce one, a trailing-comma list).

- 🔵 **Standards**: Two `BAD-LINKAGE-SHAPE` message variants for one code
  **Location**: Phase 2 §2 (message wording)
  Intended triage mechanism (one code, specifics in the message) and
  convention-consistent, but the two phrasings ("is not a typed" vs "is not a
  quoted … reference") read as independent strings. Align on a shared tail.

- 🔵 **Standards**: Leaving the `:295` iff comment unchanged understates the now-bidirectional rule
  **Location**: Phase 1 §2 (comment at `:295`)
  The work item itself flags this comment as overstating enforcement; once both
  directions are real the bare "iff" obscures that this was a deliberate fix.
  Make the `(both directions enforced)` wording the default, not optional.

- 🔵 **Code Quality**: Provenance comment correction left as an either/or
  **Location**: Phase 1 §2 (comment correction) — *merges with the standards comment finding above*
  The plan offers "leave it, or tighten"; leaving it perpetuates the inaccuracy
  the change closes. Commit to the explicit wording.

#### Suggestions

- 🔵 **Code Quality**: Deleting liveness self-tests removes the automated guard against the folded assertions becoming vacuous
  **Location**: Phase 3 §2
  Extend the existing `assert_axis_mutation` block (`:380-402`) to cover the
  provenance-over-emission and unquoted-linkage axes so the wiring proof stays
  automated rather than relying on the manual revert step.

- 🔵 **Correctness**: Two-field non-anchored provenance emits two violations
  **Location**: Phase 1 §2
  Cosmetic — both fixtures pass via `grep -qF`. Noted only so a future
  count-based assertion expects N violations for N present provenance fields.

- 🔵 **Standards**: `tok` → `elem`/`inner` naming
  **Location**: Phase 2 §2 (local variable naming)
  `inner` aligns with `fm_inner`; `elem` is new. Pick deliberately for a
  consistent "one split element" vocabulary across the comma/pipe-split loops.

### Strengths

- ✅ Exceptional research grounding: every line reference in the plan was
  verified against current source (provenance block `:295-305`, linkage loop
  `:334-357`, `emit_valid`/`assert_rejects`, conformance helpers `:176-209`,
  `FM_TYPED_REF_RE`) and all check out.
- ✅ The single-oracle consolidation reinforces the codebase's established
  "one authority, sourced not re-encoded" pattern; both rules read the shared
  contract constants so future provenance fields / source-types are picked up
  automatically (open-closed).
- ✅ The provenance `if/else` is a clean symmetric mirror of the existing
  forward branch and the legacy-forbid loop directly below it.
- ✅ The bracket-strip-then-comma-split insight makes scalars and flow-lists
  tokenize identically and is materially more readable than the prior regex.
- ✅ The `parent: ""` / `relates_to: []` no-double-flag claim was hand-verified
  correct, as were the dotted-stem, note-source, and tamper-guard regressions.
- ✅ Strict per-phase TDD framing with explicit "these FAIL before the rule"
  steps; reject-side coverage pins the exact diagnostic, satisfying AC3 with no
  new tooling.
- ✅ Phase dependency graph (1 & 2 independent; 3 depends on both) is correctly
  modelled, and the claim that the 0104 contract-file merge concern does not
  apply is verified (neither contract file is edited).
- ✅ Honours the bash-3.2 floor throughout — reuses proven trim and case-glob
  idioms, introduces no associative arrays / `${var,,}`.

### Recommended Changes

1. **Pre-plan the 80-column wrap of both new violation lines** (addresses:
   "New violation message lines exceed the project's 80-column limit"). Hoist the
   message into a `local msg=...` or split with the file's `cmd ||` continuation
   style so `scripts:check`/shfmt passes; note this in Phase 1 §2 and Phase 2 §2.

2. **Rename the new provenance diagnostic to a non-superstring code** (addresses:
   "`FORBIDDEN-PROVENANCE-NONANCHORED` is a superstring … defeating
   `assert_rejects` specificity"). e.g. `PROVENANCE-ON-NONANCHORED`. Update the
   Phase 1 fixtures, Desired End State, and `What We're NOT Doing` references.

3. **Disable globbing around the comma-split** (addresses: "Unquoted `$rest`
   word-split runs with glob expansion"). Add `set -f`/`set +f` around the
   per-key body in Phase 2 §2, saved/restored like `oldifs`, and add a rationale
   bullet.

4. **Add a multi-element quoted-list accept fixture** (addresses: "No accept-side
   fixture for a well-formed multi-element quoted list"). Add `assert_accepts`
   for `relates_to: ["adr:0001", "adr:0002"]` to Phase 2 §1, plus a negative
   assertion that empty values don't also emit `BAD-LINKAGE-SHAPE`.

5. **Preserve the accept-side controls and add an automated vacuity guard in
   Phase 3** (addresses: "Deleting the liveness self-tests removes … accept
   assertions"). Explicitly retain the conditional-axis accept cases and extend
   the `assert_axis_mutation` block to the two new axes so the wiring proof stays
   automated rather than manual-only.

6. **Fix the inline-comment-strip rationale and grammar coupling** (addresses the
   `${rest%%#*}` cluster). State the real invariant in the inline comment,
   cross-reference `FM_TYPED_REF_RE`, and decide whether whole-value vs
   per-element comment stripping is intended.

7. **Make the `:295` comment correction non-optional and align the two
   `BAD-LINKAGE-SHAPE` messages** (addresses: comment-accuracy + message-variant
   findings). Commit to `(both directions enforced)` and give the two messages a
   shared tail.

8. **Acknowledge the embedded-empty-element edge** (addresses: "Embedded empty
   quoted element … silently accepted") in `What We're NOT Doing`, or emit
   `EMPTY-PLACEHOLDER` for it.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Correctness

**Summary**: The plan is logically sound on its central claims: the new
comma-split tokenizer was hand-traced against every fixture (bare scalar, bare
path, bracketed-unquoted, mixed list, dotted id, note source, empty `""`/`[]`)
and the accept/reject outcomes are correct, including the no-double-flag claim
for `parent: ""` and `relates_to: []`. The Phase 1 provenance if/else is correct
and bidirectional. Two correctness concerns warrant attention: the unquoted
`$rest`/`$elem` word-split runs with pathname (glob) expansion enabled (latent
fragility under `set -euo pipefail` with no `set -f`), and the new diagnostic
code `FORBIDDEN-PROVENANCE-NONANCHORED` is a superstring of the existing
`FORBIDDEN-PROVENANCE`, defeating the substring-based `assert_rejects`
specificity guarantee.

**Strengths**:
- The `parent: ""` / `relates_to: []` no-double-flag claim is correct: the
  empty-quoted element hits the `'"'*'"'` case arm, `inner` strips to empty, and
  the `[ -n "$inner" ] || continue` skips it, while `[]` bracket-strips to an
  empty `$rest` that yields zero loop iterations — so only EMPTY-PLACEHOLDER
  fires.
- The single-leading-`[`/single-trailing-`]` strip is lossless for all real
  linkage values because typed-ref ids are constrained to `[A-Za-z0-9.-]`.
- Phase 1's forward/else split is logically complete with the legacy
  git_commit/branch loop preserved unchanged.
- All existing accept fixtures and the tamper guard at line 163 re-trace green;
  the real-corpus sanity check is a sound regression backstop.
- The TDD ordering and the Phase 3 dependency reasoning are correct.

**Findings**:
- 🟡 major / high — **New diagnostic FORBIDDEN-PROVENANCE-NONANCHORED is a
  superstring of FORBIDDEN-PROVENANCE, defeating assert_rejects specificity**
  (Key Discoveries / Phase 1 §2). `assert_rejects` matches `grep -qF -- "$code"`;
  a test asserting `FORBIDDEN-PROVENANCE` is satisfied by output emitting only
  the NONANCHORED variant. Choose a non-superstring code (e.g.
  `PROVENANCE-ON-NONANCHORED`) or match the full diagnostic line.
- 🟡 major / high — **Unquoted `$rest` / `$elem` word-split runs with glob
  expansion enabled** (Phase 2 §2). No `set -f`; a future value with a glob char
  surviving the bracket strip would expand against CWD. Wrap the split in
  `set -f`/`set +f`.
- 🔵 minor / medium — **Unconditional inline-comment strip changes behaviour for
  values whose token precedes non-ref trailing content** (Phase 2 §2). Whole-value
  `%%#*` discards an element after an in-list `#`. Document or move to per-element.
- 🔵 suggestion / high — **Two-field non-anchored provenance emits two
  violations** (Phase 1 §2). Cosmetic; both fixtures pass via `grep -qF`.

### Code Quality

**Summary**: The plan is unusually high-quality from a maintainability
standpoint: it mirrors the file's established idioms (the `violation()` helper,
IFS save/restore, whitespace-trim parameter expansions, `case`-glob quoting
checks), keeps complexity flat, and reduces the overall surface by collapsing
three authorities into one. The provenance if/else restructure is a clean,
symmetric mirror of the existing forward branch. The linkage-tokenizer rewrite
trades an opaque regex `while` for a more readable strip-then-split-then-case
pipeline, but its per-guard comments carry two subtly inaccurate justifications,
and the rewrite passes up an opportunity to reuse the existing `fm_inner`
quote-stripping helper.

**Strengths**:
- The provenance if/else is a symmetric mirror of the existing forward branch
  and the legacy-forbid loop below it.
- The linkage rewrite replaces an opaque regex `while` with a readable
  strip → split → `case` pipeline; the bracket-strip-then-split insight lowers
  cognitive load.
- Strong idiom fidelity: reuses IFS save/restore, the trim expansions, the
  `case`-glob checks, and the single `violation()` helper; no bash-4 constructs.
- Phase 3 is a net maintainability win — removing a parallel re-derivation of the
  contract.

**Findings**:
- 🔵 minor / high — **Inline-comment-strip comment overstates safety with an
  inaccurate justification** (Phase 2 §2). The real invariant is "no value byte
  before the comment is `#`", not "refs contain no `#`". Reword.
- 🔵 minor / medium — **Tokenizer re-implements quote-stripping instead of
  reusing the existing `fm_inner` helper** (Phase 2 §2). Two dequoting idioms in
  one file invite drift.
- 🔵 minor / medium — **Two distinct BAD-LINKAGE-SHAPE messages under one code**
  (Phase 2 §2). Confirm intent and add a clarifying comment.
- 🔵 suggestion / high — **Provenance comment correction left as an either/or**
  (Phase 1 §2). Commit to the explicit `(both directions)` wording.
- 🔵 suggestion / medium — **Deleting liveness self-tests removes a guard against
  the folded assertions becoming vacuous** (Phase 3 §2). Extend the
  `assert_axis_mutation` block to the two new axes.

### Test Coverage

**Summary**: The plan is TDD-disciplined and reject-side coverage is strong: six
new failure-mode fixtures each pin a specific diagnostic, and the real-corpus
sanity check plus the tamper guard form a solid regression backstop. The
principal gap is ACCEPT-side regression coverage of the rewritten comma-splitting
tokenizer — no automated fixture asserts a well-formed multi-element quoted list
still passes. There is also a wiring-proof gap in Phase 3 where deleting the
bespoke liveness self-tests removes negative coverage that the conditional-axis
fold-in only partly replaces.

**Strengths**:
- Strict per-phase TDD framing with explicit "FAIL before the rule" assertions.
- Reject-side coverage is specific (exact diagnostic code), satisfying AC3 with
  no new tooling.
- Linkage reject cases enumerate the real blind-spot variants.
- The real-corpus sanity check and existing accept fixtures are correctly
  identified as the tokenizer-rewrite regression guard.
- Phase 3's dependency on both 1 and 2 is correctly modelled.

**Findings**:
- 🟡 major / high — **No automated fixture asserts a well-formed multi-element
  quoted list still ACCEPTS** (Phase 2 §1 / Testing Strategy edge case 5). The
  only bracketed-list accept fixture is single-element. Add an `assert_accepts`
  for `relates_to: ["adr:0001", "adr:0002"]`.
- 🟡 major / medium — **Deleting the blind-spot liveness self-tests removes
  negative-wiring proofs and clean-control accept assertions** (Phase 3 §1).
  Confirm the conditional-axis accept cases remain and add any missing paired
  accept assertion.
- 🔵 minor / medium — **The no-double-flag invariant is unverified for empty
  values** (Testing Strategy edge case 6). `bad-empty.md` only asserts
  EMPTY-PLACEHOLDER present. Add a negative assertion.
- 🔵 minor / medium — **Documented trailing-inline-comment / trailing-comma
  tolerances have no regression fixture** (Phase 2 §2).
- 🔵 minor / low — **Provenance accept side is implicit** (Phase 1 §1). Optionally
  add an explicit `assert_accepts` for a non-anchored type with no provenance.

### Architecture

**Summary**: The single-oracle consolidation is architecturally sound and
directly reinforces the codebase's established "one authority, sourced not
re-encoded" pattern. The new rules stay genuinely data-driven, so future
provenance-bundle or source-type additions are picked up without touching the
validator, and deleting the bespoke conformance helpers removes a duplicated
authority rather than creating one. The phase decomposition is correct, and the
claim that the 0104 contract-file merge concern does not apply is verified.

**Strengths**:
- Collapses a three-authority temporary state back to a single oracle, restoring
  the intended invariant; the section-4 tamper guard already encodes this value.
- Both rules read shared contract constants rather than re-encoding them
  (open-closed holds).
- Deleting the bespoke helpers removes a validator-bypassing parallel enforcement
  path and its drift risk.
- Phase dependency graph correctly identified and justified.
- The provenance rule reuses the exact shape of the existing forbid loop.

**Findings**:
- 🔵 minor / medium — **Embedded empty quoted element in a list silently
  accepted** (Phase 2 §2). `relates_to: ["plan:0001", ""]` falls through both
  the tokenizer and EMPTY-PLACEHOLDER. Acknowledge or emit EMPTY-PLACEHOLDER.
- 🔵 minor / medium — **Tokenizer hard-codes YAML/grammar assumptions, softly
  re-encoding the contract** (Phase 2 §2). Cross-reference `FM_TYPED_REF_RE`.
- 🔵 minor / high — **Template/skill text-inspection coverage is retired, not just
  deleted** (Phase 3 §2). Make explicit that the section-5 real-corpus sanity
  check now backstops live artifacts.

### Portability

**Summary**: The plan is strongly grounded in the file's existing bash-3.2-safe
idioms and explicitly reuses the proven whitespace-trim and case-glob building
blocks; every new parameter expansion is POSIX/bash-3.2-compatible and none would
be flagged by `lint-bashisms.sh`. The one genuine portability hazard is that the
new tokenizer introduces unquoted word-splitting on an author-controlled value
with no glob suppression, exposing pathname expansion the current quoted-regex
loop does not. Tooling portability is unchanged.

**Strengths**:
- Explicitly anchors to the bash-3.2 floor and names the exact proven idioms it
  reuses.
- All new parameter expansions are POSIX-portable and pass the bashisms denylist.
- Phase 1's if/else reuses the existing array-iteration pattern verbatim.
- IFS is saved to `oldifs` and restored inside the per-key body.
- No contract-file or tooling changes; LC_ALL=C byte-stability discipline
  untouched.

**Findings**:
- 🟡 major / high — **Unquoted `$rest` under IFS=',' is exposed to pathname
  (glob) expansion** (Phase 2 §2, lines 323-325). The validator runs without
  `set -f`; a value with glob metacharacters surviving the strip would expand
  against CWD, producing nondeterministic diagnostics. Wrap in `set -f`/`set +f`.
- 🔵 minor / medium — **`${rest%%#*}` inline-comment strip can corrupt a value
  containing a literal `#`** (Phase 2 §2, line 319), yielding a truncated
  diagnostic. Optionally only strip when preceded by whitespace.

### Standards

**Summary**: The plan adheres closely to the validator's established conventions:
the new diagnostic follows SCREAMING-KEBAB-CASE and slots into the
FORBIDDEN-PROVENANCE prefix family, and the decision to reuse BAD-LINKAGE-SHAPE
is well-reasoned and consistent with the validator's "one code per violation
class, specifics in the message" convention. The main convention risk is the
80-column line-width rule: several proposed new code lines exceed 80 columns and
would fail `scripts:check`.

**Strengths**:
- `FORBIDDEN-PROVENANCE-NONANCHORED` follows the file's diagnostic-code naming and
  extends the FORBIDDEN-PROVENANCE prefix family.
- Reusing BAD-LINKAGE-SHAPE is convention-consistent (one code per violation
  class; specifics in the message); a `BARE-LINKAGE-VALUE` split would be the only
  code fragmenting a class on a syntactic axis.
- New messages mirror sibling phrasing and structure; unchanged arms kept verbatim.
- The `[0105]` tag-removal is an explicit, checkable step; stale docblock item 4
  and banner removed.
- Respects the bash-3.2 floor and the data-driven contract pattern.

**Findings**:
- 🟡 major / high — **New violation message lines exceed the project's 80-column
  limit** (Phase 1 §2 and Phase 2 §2). ~110 and ~105 columns; shfmt won't wrap
  string args and shell has no autofixer. Pre-plan the wrap (hoist to a `local`
  or use the `cmd ||` continuation style).
- 🔵 minor / high — **Leaving the `:295` iff comment unchanged understates the
  now-bidirectional rule** (Phase 1 §2). Make `(both directions enforced)` the
  default wording.
- 🔵 minor / medium — **Two BAD-LINKAGE-SHAPE message variants for one code**
  (Phase 2 §2). Align on a shared tail so the variant reads as a qualifier.
- 🔵 suggestion / medium — **`tok` renamed to `elem`/`inner`** (Phase 2 §2). Pick
  deliberately for a consistent "one split element" vocabulary across the
  comma/pipe-split loops.

## Re-Review (Pass 2) — 2026-06-15

**Verdict:** REVISE

Re-ran all six lenses against the revised plan. **Every pass-1 finding is
resolved.** The re-review surfaced one *newly-introduced* critical defect in the
Phase 3 edit (a BSD/macOS-sed incompatibility in the `assert_axis_mutation`
snippets added to address pass-1's wiring-proof gap), plus a handful of minor
follow-ups. The critical defect and the worthwhile minors were corrected in a
follow-up edit immediately after this pass (see Assessment); a pass-3
confirmation run is recommended to certify green.

### Previously Identified Issues

- 🟡 **Standards**: 80-column message lines fail `scripts:check` — **Resolved
  (finding was invalid)**. Empirically confirmed by the standards re-review: no
  line-length gate exists in the shell check chain (ShellCheck has no width rule,
  shfmt never wraps, bashisms denylist is construct-only), and existing committed
  message lines already run 92–101 columns and pass CI. Plan now documents this.
- 🟡 **Correctness**: `FORBIDDEN-PROVENANCE-NONANCHORED` superstring defeats
  `assert_rejects` — **Resolved**. Renamed to `PROVENANCE-ON-NONANCHORED`
  (plan + work-item ACs reconciled, with a Drafting Note recording why).
- 🟡 **Correctness / Portability**: unquoted `$rest` glob expansion — **Resolved**.
  `set -f`/`set +f` brackets the comma-split; both lenses re-confirmed it is
  bash-3.2-safe, not flagged by the bashisms linter, and the unconditional
  restore is correct (no global noglob).
- 🟡 **Test Coverage**: no multi-element quoted-list accept fixture — **Resolved**.
  Added multi-element, irregular-spacing, and trailing-comment accept fixtures.
- 🟡 **Test Coverage**: Phase 3 liveness deletion removes coverage — **Partially
  resolved → re-corrected**. Accept-side controls preserved (good), but the new
  `assert_axis_mutation` wiring-proof was the source of the new critical defect
  below; replaced with `emit_valid`-built reject fixtures as the proof.
- 🔵 All pass-1 minors (comment-strip rationale, `:295` comment non-optional,
  shared `BAD-LINKAGE-SHAPE` message tail, `fm_inner` annotation, embedded-empty
  scoped out, grammar-coupling cross-reference, text-inspection acknowledgement)
  — **Resolved**.

### New Issues Introduced

- 🔴 **Correctness / Portability / Test Coverage**: Phase 3 `assert_axis_mutation`
  sed snippets use `\n` in the replacement — **introduced by the pass-1 fix,
  now corrected**. BSD/macOS sed treats `\n` in the `s///` RHS as a literal `n`,
  so the injected provenance/linkage lines would be a single mangled line and the
  wiring proof would be a macOS-only failure. Fixed by dropping the
  `assert_axis_mutation` extension and using the `emit_valid`-built
  `prov-overemit`/`link-bare` reject fixtures (C-style `$'…\n…'`, bash-expanded)
  as the portable wiring proof, with an explicit "do NOT use sed `\n` here" note.
- 🔵 **Correctness / Test Coverage**: no-double-flag assertion covered only
  `parent: ""`, not `relates_to: []` — **corrected**. Added a `relates_to: []`
  fixture and a reusable `assert_absent` helper (replacing the inline grep
  snippet, per the code-quality suggestion) covering both empty branches.
- 🔵 **Test Coverage**: the `set -f` glob success criterion used a *quoted*
  `"plan:*"` that the shape regex catches regardless of `set -f` — **corrected**
  to an unquoted `parent: plan-*` run from a glob-matching directory.
- 🔵 **Architecture**: confirm template-surface coverage survives the helper
  deletion — **addressed** with a Phase 3 note that the per-emitter
  composed-acceptance loop (`:306-310`) already validates each template's
  composed emission through the real validator.
- 🔵 **Standards**: minor message-wording drift (`well-formed` vs `quoted` vs the
  existing `typed`) — **deferred** as cosmetic (no consumer greps message text);
  noted for optional alignment during implementation.
- 🔵 **Code Quality**: the seven plan-prose rationale bullets have no in-tree home
  — **deferred**; the load-bearing invariants are already captured in the block
  comment, the rest is plan-level explanation.

### Assessment

The plan is materially stronger and all original concerns are closed. The single
serious item the re-review found was a defect the pass-1 revision itself
introduced (the BSD-sed `\n` injection), which has been fixed by routing the
Phase 3 wiring proof through `emit_valid` rather than `sed` — the portable,
already-proven idiom. With that and the no-double-flag/`assert_absent`/glob-
criterion corrections applied, the plan is in good shape for implementation. A
pass-3 confirmation run would certify the post-fix state green; the remaining two
deferred items are cosmetic.

## Re-Review (Pass 3) — 2026-06-15

**Verdict:** APPROVE

Confirmation run across all six lenses against the post-pass-2-fix plan.
**Correctness and Portability returned zero findings**; Standards confirmed the
deferrals are sound; Code Quality and Test Coverage returned only minor polish.
The pass-2 critical (BSD-sed `\n`) and all other prior findings are verified
closed. The pass surfaced **one major** — a factual error in the pass-2
template-surface-coverage note I had added — which has been corrected, along
with the two actionable minors. With those edits the plan is ready for
implementation.

### Previously Identified Issues

- 🔴 **Correctness/Portability/Test-Coverage**: Phase 3 BSD-sed `\n` injection —
  **Verified resolved**. Correctness re-traced the `emit_valid`-built
  `prov-overemit`/`link-bare` fixtures (C-style `$'…\n…'`, host-expanded) and
  confirmed they are portable non-vacuous wiring proofs; portability confirmed no
  sed `\n`-in-replacement remains anywhere.
- 🟡 **Test Coverage**: multi-element accept + no-double-flag — **Verified
  resolved** (both empty branches now covered via `assert_absent`).
- 🟡 **Correctness/Portability**: glob expansion — **Verified resolved** (`set -f`
  bracket re-confirmed correct and bash-3.2-safe; leak-safe across keys).
- 🔵 All other pass-1/pass-2 minors — **Verified resolved**.

### New Issues Introduced (and addressed in this pass)

- 🟡 **Architecture (major, high confidence)**: the pass-2 note "Template-surface
  coverage is preserved, not lost" was **factually wrong** — `emit_valid`
  (`:309`, 5-arg) synthesises a minimal fixture that never carries template
  content (provenance from the schema `anchored` flag; linkage keys omitted), and
  the completeness check (`:288-304`) inspects key *names* only, so the
  composed-acceptance loop does **not** catch a template over-emitting provenance
  or carrying bare linkage; section-5 walks `meta/` artifacts, not `templates/`.
  **Corrected**: the note now states honestly that template-*source* inspection
  of these two axes is intentionally dropped (per the work item's "delete the
  helpers outright" decision), justified by templates being reviewed source whose
  defects surface downstream when the validator rejects the emitted artifact, and
  flags a possible follow-up `templates/*.md` lint. I verified the `emit_valid`
  behaviour against source before rewriting.
- 🔵 **Code Quality (minor)**: `assert_absent`'s `shellcheck disable=SC2001`
  dropped the rationale trailer its siblings carry — **fixed** (trailer added;
  also documented the rc-agnostic-by-design intent).
- 🔵 **Test Coverage (minor)**: the `set -f` glob criterion was prose-only —
  **fixed** by promoting it to a checked-in `bad-glob-linkage` fixture (unquoted
  `parent: plan-*` validated from a directory seeded with matching files,
  asserting the literal token in the diagnostic), so it goes red if `set -f` is
  dropped.
- 🔵 **Standards / Code Quality (suggestions)**: message-wording drift and the
  prose-rationale-home point — confirmed cosmetic and **deferred** (no consumer
  greps message text; load-bearing invariants are already in the block comment).

### Assessment

The plan is in good shape for implementation. Three of six lenses are clean, the
one major was a self-inflicted documentation inaccuracy now corrected against
verified source, and the test suite is now mutation-proof on the previously
prose-only glob guard. No design-level concerns remain; the two deferred items
are cosmetic and can be folded in opportunistically during implementation.
