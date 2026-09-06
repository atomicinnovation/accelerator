---
type: "plan-review"
id: "2026-09-06-0269-remove-bash-vocabulary-and-redesign-exit-code-classification-review-1"
title: "Plan Review: Remove Bash Vocabulary And Redesign Exit-Code Classification In Jira And Linear Clients"
date: "2026-09-06T16:21:14+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-06-0269-remove-bash-vocabulary-and-redesign-exit-code-classification"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "correctness", "code-quality", "test-coverage", "compatibility", "standards"]
review_number: 1
review_pass: 2
tags: ["jira", "linear", "cleanup", "refactor", "exit-codes", "classification"]
last_updated: "2026-09-06T19:21:44+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Remove Bash Vocabulary And Redesign Exit-Code Classification In Jira And Linear Clients

**Verdict:** REVISE

The plan is unusually well-grounded: the correctness lens traced every reachable
`Outcome` through both crates and confirmed the proposed match arms reproduce
every emitted exit-code value and every retry-class verdict exactly — including
Linear's load-bearing code-34 create/update divergence — and verified each named
constant holds its pinned value. The core redesign (move the granular map to the
CLI, re-express the retry collapse on `(Outcome, Operation)`, drop the numeric
intermediary and the `({code})` detail embed) is sound and cleanly phased. It
needs revision before implementation on five points: Phase 4 deletes
`AdfError::code()` without accounting for its four live test consumers (a
compile break and a dropped contract oracle); the relocated granular map is
tested more loosely than the assertions it replaces; and the scrub, the naming,
and the exhaustive-match discipline all stop short of what the plan claims for
them.

### Cross-Cutting Themes

- **Exhaustive-match discipline is overstated and under-delivered** (flagged by:
  architecture, correctness, code-quality, test-coverage) — the plan promises
  "every mapping function stays an exhaustive `match` with no wildcard so a new
  variant is a compile error." Three places break the promise: jira `classify`
  uses a non-exhaustive `matches!` (loses the compile-time guard the current
  code has); the jira map necessarily keeps a `Status(_)` catch-all (variant-
  exhaustive, not wildcard-free); and exhaustiveness constrains the domain, not
  the codomain, so the deleted "reserved codes" property is not guarded by the
  match at all.
- **Coverage of the relocated granular map is looser than what it replaces**
  (flagged by: test-coverage, correctness) — the linear inline test is specced
  as one assertion per code value, not per `GraphQlError` variant, so a variant
  drifting between OR-pattern arms would ship silently; `SuccessWithErrors(RateLimited)`
  (mapped to 34 by the rewrite) has no row in the retained TABLE; and dropping
  the row-count guards removes both the completeness guard and the client
  crates' only cross-check against the shared bridge fixture.
- **The scrub and the rename stop at the client-crate boundary** (flagged by:
  standards, compatibility) — the CLI `exit_codes.rs` that now *owns* the map
  keeps "bash code" / "bash `jira-request.sh`" doc comments (verified: jira
  lines 3, 5, 16, 26, 139, 194; linear 5, 16, 25, 173), and both CLI module docs
  cite `tests/fixtures/bash-exit-codes.txt`, which the plan renames — leaving a
  dangling reference. These sit outside the client-scoped grep AC, so they
  survive silently unless deliberately in or out of scope.
- **Naming does not finish disambiguating exit code from wire status** (flagged
  by: code-quality, standards) — `code_for_outcome` is renamed to
  `exit_code_for_outcome`, but the sibling `code_for_status(status: u16)` is
  retained, needing a prose note to explain the surviving `u16`. The exact
  ambiguity the rename targets survives in the module.
- **Mandated justification comments conflict with the repo's comment
  convention** (flagged by: code-quality, standards) — the shebang justification
  comment and the "restate it as a comment" fallback both add what-comments the
  project bans, and the shebang comment guards nothing the ACs check (the `.sh`
  is in no grep path).

### Tradeoff Analysis

- **Cleanup hygiene vs regression guard**: dropping the bridge-fixture coupling
  from the client crates is good hygiene (Option A from the research; the
  transcription-only rows guard retired code) and correctness confirms it costs
  no *runtime* coverage. But it also removes the only in-crate cross-check of
  live retry verdicts against the shared contract fixture, leaving them pinned
  only by hand-authored table columns. Recommendation: take Option A, but retain
  one minimal assertion feeding the *reachable* `(Outcome, Operation)` rows
  through `classify` against the fixture's live rows.

### Findings

#### Critical

None.

#### Major

- 🟡 **Compatibility**: Phase 4 deletes `AdfError::code()` but leaves four live
  test consumers
  **Location**: Phase 4, Change 1 and Success Criteria
  `adf_differential.rs:134,138`, `adf.rs:84`, `adf_inventory.rs:173` all call
  `error.code()` (verified). The last is the ADF exit-code differential oracle
  against the frozen contract. The phase's "no test parses it" claim covers only
  the fixture `bash_exit=` line, not these call sites — the change will fail to
  compile, and deleting the assertions rather than relocating them silently
  drops the 40/41/42 contract guard.

- 🟡 **Code Quality / Standards**: retained `code_for_status` perpetuates the
  exit-code/wire-status ambiguity the redesign targets
  **Location**: Phase 2 Change 3, Phase 3 Change 3
  `exit_code_for_outcome` is renamed for disambiguation, yet `code_for_status(status: u16)`
  is kept unchanged — "code" is the returned exit code, "status" the incoming
  wire status, the exact conflation the rename kills. That the plan needs a prose
  note to explain the surviving `u16` signals the name isn't carrying its
  meaning. Rename to `exit_code_for_status` or inline the one-line wrapper.

- 🟡 **Code Quality / Test Coverage**: the reserved-codes guard is dropped, its
  replacement enforces nothing
  **Location**: Phase 3 Change 5
  The plan replaces `linear_emits_no_403_404_410_or_429_...` with "the exhaustive
  match enforces it" and "drop the test or restate it as a comment."
  Exhaustiveness guarantees every variant is *handled*, not that the *codomain*
  excludes Jira-only codes — a future arm mapping to a literal `13` stays
  exhaustive. A comment enforces nothing and violates the comment convention.
  Keep a small structural test asserting the Linear map never yields
  `{12,13,14,15,17,19}`.

- 🟡 **Test Coverage**: linear granular-map unit test under-specified for
  OR-pattern groupings
  **Location**: Phase 3 Change 4
  "Assert every Outcome arm maps to its pinned constant (11/36/35/34/16/21/20)"
  is seven codes, but `exit_code_for_outcome` bundles multiple `GraphQlError`
  variants per arm. One representative per code won't catch a variant moving
  between arms (e.g. `BadRequest(Complexity)` drifting 36→34, or
  `BadRequest(RateLimited)` 35→34). Assert every constructible
  `(SuccessWithErrors|BadRequest) × GraphQlError` combination, ideally via an
  in-test exhaustive `match`.

- 🟡 **Test Coverage**: the rewrite introduces an outcome the retained TABLE
  never exercises
  **Location**: Phase 3 Change 5 and Change 1
  The re-expressed `classify` has arm `SuccessWithErrors(RateLimited | BadRequest)`
  (retryable-create / terminal-update, mapped to 34), but the current linear
  TABLE covers only `BadRequest(RateLimited)`. `SuccessWithErrors(RateLimited)`
  is constructible (a 200 body classified rate-limited) and is asserted by
  neither the retry-class TABLE nor the granular test. Add its row with explicit
  create/update expectations.

#### Minor

- 🔵 **Architecture / Correctness**: jira `classify` trades an exhaustive `match`
  for a non-exhaustive `matches!`
  **Location**: Phase 2 Change 1
  `matches!(outcome, Outcome::Status(400|401|403|404|410|429))` is behaviourally
  identical today but non-exhaustive — a future `Outcome` variant is silently
  terminal-for-a-mutation with no compile error, diverging from the linear
  rewrite (which keeps a full `match`) and from the plan's own stated invariant.
  Mirror linear: use an exhaustive `match` for `provably_unapplied`.

- 🔵 **Test Coverage / Correctness**: dropping the row-count guards removes the
  completeness guard and the shared-fixture cross-check
  **Location**: Phase 2 Change 5, Phase 3 Change 5
  Deleting the 43-row/31-row guards stops the client crates reading
  `bridge-exit-code-tables.txt`, so live verdicts and the fixture can drift, and
  no guard forces a new outcome to gain an assertion. Replace with an in-test
  exhaustive `match` over `Outcome` plus a minimal live-row fixture cross-check.

- 🔵 **Code Quality**: "no wildcard" is stated more absolutely than the jira map
  can honour
  **Location**: Implementation Approach, Phase 2 Change 3
  The jira `exit_code_for_outcome` must keep a `Status(_)` catch-all (`Status(u16)`
  can't be matched exhaustively). Reword to "exhaustive over enum variants with
  no variant-level wildcard" and note the deliberate `Status(_)` coercion to
  `SERVER_ERROR`.

- 🔵 **Standards**: CLI-side doc comments still speak "bash"
  **Location**: Phase 2 Change 3, Phase 3 Change 3
  `jira-cli`/`linear-cli` `exit_codes.rs` keep "granular bash code", "bash
  `jira-request.sh`", and module-doc bash references (verified). Outside the
  client grep AC, so they survive silently. Reword them as part of Phases 2/3 or
  add an explicit "NOT doing" note scoping CLI prose out.

- 🔵 **Compatibility**: renamed fixture leaves a dangling reference in the CLI
  module docs
  **Location**: Phase 2/3 Change 7
  Both CLI `exit_codes.rs` module docs (line 5) name
  `tests/fixtures/bash-exit-codes.txt` as the authoritative contract; after the
  rename they point at a nonexistent path. Update these references as part of the
  rename.

- 🔵 **Compatibility**: narrow per-phase verification can miss a stale consumer
  of a removed public symbol
  **Location**: Implementation Approach, Phases 2–4
  The client crates aren't surface-pinned, so `public-api:check` won't flag the
  removals — the sole backstop is compile-time breakage (exactly the AdfError
  gap). Single-target test commands can pass while the crate doesn't fully
  compile. Add `cargo test -p <crate> --no-run` (or clippy `--all-targets`) per
  phase.

- 🔵 **Test Coverage**: the divergence success criterion overstates `flow_errors`
  coverage
  **Location**: Phase 3 Success Criteria, Testing Strategy
  `linear-cli` emits code 34 for both create and update; the retryable-create /
  terminal-update divergence is a 70/71 distinction only visible in `work-cli`.
  `flow_errors` never contrasts create vs update. Attribute the divergence
  coverage to the retained `classify` TABLE, not `flow_errors`.

- 🔵 **Architecture**: the split `Outcome` taxonomy has no coherence guard for
  the code-34 divergence
  **Location**: Phase 3 Changes 1 & 3
  Retry-class grouping (client) and numeric grouping (CLI) now live in different
  crates as independent matches; nothing asserts the set the CLI labels 34 is
  exactly the set whose verdict flips by operation. A future regroup on one side
  desynchronises silently. Add a cross-boundary guard tying the two views.

- 🔵 **Standards / Code Quality**: mandated justification comments conflict with
  the comment convention and guard nothing checked
  **Location**: Phase 2 Change 7 / Change 6 (and Phase 3 equivalents)
  The shebang justification comment is a what-comment under the repo's very-low
  comment tolerance, and the `.sh` is in no grep AC path, so it appeases no
  check. Drop the shebang comment; for `transport.rs` prefer the plan's own
  "reword to drop bash" option.

- 🔵 **Standards**: the divergence from the work item's file list is undocumented
  **Location**: Phase 1
  The work item lists `mutation.rs`, `error.rs`, `auth.rs`, `upload.rs`; the plan
  omits them. A grep of the tree confirms none carries "bash" — so the plan is
  correct — but the deviation is untraceable. Add a one-line note that the work
  item's list is stale at this revision.

#### Suggestions

- 🔵 **Standards**: the Phase 2/3 grep-AC caveat describes a match its command
  can't produce
  **Location**: Phase 2 Success Criteria (and Phase 3 equivalent)
  The parenthetical says "any residual match is the capture-script shebang", but
  the grep argument list omits the `.sh` file. Either drop the caveat or add the
  path — consistently across both phases.

- 🔵 **Code Quality**: reworded `failure.rs` docs risk narrating cross-crate
  behaviour
  **Location**: Phase 2 Change 2, Phase 3 Change 2
  The exit-code mapping now lives in the CLI crate; keep these docs on the
  invariant the failure type itself guarantees (it carries the outcome/operation
  discriminant so no consumer parses a detail string), and let the CLI own any
  statement about the numeric mapping.

- 🔵 **Test Coverage**: sequence the moved unit test before deleting client
  assertions
  **Location**: Implementation Approach, Phase 2/3 change ordering
  State explicitly that `exit_code_for_outcome` and its inline unit test are
  written first (green), and the client-crate `bash_code` assertions deleted only
  after — so no interim commit leaves the granular map unasserted.

- 🔵 **Architecture**: record the intentionally-retained `TrackerError` collapse
  in Desired End State
  **Location**: Current State Analysis / What We're NOT Doing
  The client crate keeps a second output path (`TrackerError` for `work-cli`)
  coupled to the `tracker` crate. State this explicitly so reviewers of 0271
  inherit an accurate boundary map.

### Strengths

- ✅ Behaviour preservation is verified, not asserted: every reachable `Outcome`
  was traced through both crates' current `classify` paths, and each proposed
  match arm reproduces the emitted exit-code value and retry-class verdict
  exactly, including Linear's code-34 create/update divergence and Read's
  always-retryable path.
- ✅ Every named constant the maps target holds its pinned numeric value
  (verified against both `exit_codes.rs`): jira 400→34, 401→11, 403→12, 404→13,
  410→14, 429→19, NonJsonBody→16, Transport→21, `Status(_)`→20; linear Auth→11,
  Complexity→36, RateLimited→35, BadRequest→34, plus 16/21/20; ADF 40/41/42.
- ✅ Moving the granular map to the CLI is a genuine separation-of-concerns win:
  the overloaded `bash_code`/`classify_bash_code`/`build` triple is decomposed,
  the map returns `u8` directly (dissolving the `u8::try_from(...).unwrap_or(...)`
  narrowing), and the new mappers are pure `const fn` at the shell boundary.
- ✅ Phased delivery is genuinely independently mergeable and green (doc-only →
  per-crate → jira-only AdfError), with the deferred `grep -i bash = 0`
  criterion honestly acknowledged.
- ✅ The plan correctly distinguishes the two fixtures and two oracles: it
  renames only the per-CLI parity fixture and deliberately leaves
  `bridge-exit-code-tables.txt` and its independent `tracker-support` self-test
  untouched.
- ✅ The live retry-class oracle is explicitly retained (the outcome-keyed
  `create_retryable`/`update_retryable` columns), and dropping the
  transcription-only rows and dead `18|23|25|27|29|110..=114` arms is correctly
  justified as guarding no live path.

### Recommended Changes

1. **Complete Phase 4's `AdfError::code()` removal** (addresses: Phase 4 deletes
   `AdfError::code()` but leaves four live test consumers)
   Add explicit steps to relocate the `error.code()` assertions in `adf.rs`,
   `adf_inventory.rs`, and — critically — the `adf_differential.rs` oracle onto
   the new `for_adf` mapping in `jira-cli` (or a variant-keyed helper), and
   change the phase verification to full `cargo test -p jira-client -p jira-cli`.

2. **Assert the relocated granular maps exhaustively** (addresses: linear
   granular-map test under-specified; rewrite introduces an untested outcome;
   dropping the row-count guards)
   Specify the inline unit tests to assert every constructible `Outcome`
   (per-variant, not per-code) via an in-test exhaustive `match`, add the
   `SuccessWithErrors(RateLimited)` row to the retained TABLE, and retain a
   minimal live-row cross-check against `bridge-exit-code-tables.txt`.

3. **Keep the reserved-codes property as a test, not a comment** (addresses: the
   reserved-codes guard is dropped, its replacement enforces nothing)
   Replace, don't demote: keep a structural test that `exit_code_for_outcome`
   over all Linear outcomes never yields `{12,13,14,15,17,19}`. Remove the
   "restate as a comment" option.

4. **Finish the naming and the scrub at the CLI boundary** (addresses:
   `code_for_status` ambiguity; CLI-side bash prose; dangling fixture reference)
   Rename `code_for_status` → `exit_code_for_status` (or inline it); reword the
   CLI `exit_codes.rs` module and helper doc comments; update the module-doc
   fixture reference to the renamed path — or add an explicit "NOT doing" note
   scoping CLI-side prose out.

5. **Tighten the exhaustive-match wording and the jira `classify` rewrite**
   (addresses: `matches!` non-exhaustiveness; "no wildcard" overstated)
   Use an exhaustive `match` for jira `provably_unapplied`, and reword the
   discipline to "exhaustive over enum variants with no variant-level wildcard",
   noting the deliberate jira `Status(_)` coercion.

6. **Correct the verification and comment instructions** (addresses: per-phase
   verification gaps; justification comments; overstated `flow_errors`; stale
   file list; grep-AC caveat)
   Add `--no-run`/`--all-targets` compile checks per phase; drop the shebang
   comment; re-attribute the divergence coverage to the `classify` TABLE; note
   the work item's stale file list; fix the grep-AC caveat.

---

## Per-Lens Results

### Architecture

**Summary**: A structurally sound refactor that improves separation of concerns
— it pulls the granular exit-code map out of the client crates (where it was a
premature presentation concern) into each CLI boundary, leaving the clients to
own only the retry-class domain decision. The structured errors already carried
the needed context, so the change removes coupling rather than adding it, and the
phased decomposition keeps each step independently mergeable. Two
evolutionary-fitness concerns stand out: the jira `classify` rewrite silently
drops the exhaustive-match safety net the plan promises, and the `Outcome`
taxonomy is now split across a crate boundary with no guard keeping the two
groupings coherent for the code-34 divergence.

**Strengths**:
- Moving the granular map to the CLI decomposes the overloaded
  `bash_code`/`classify_bash_code`/`build` triple into a single-responsibility
  client `classify` and a CLI-side map.
- The new CLI mappers are pure `const fn` at the imperative-shell boundary —
  trivially unit-testable, a clean functional-core placement.
- Phased delivery is well-decomposed, each phase leaving `cli:check` green, with
  the deferred grep-zero criterion honestly acknowledged.
- Removing the `u16` intermediary, the `({code})` embed, and linear's dead arms
  shrinks the classification surface and removes misleading coupling.
- The external contract is protected by retaining `exit_codes_parity` and
  `flow_errors` while dropping only transcription-only rows.

**Findings**:
- **minor / high** — jira `classify` rewrite drops the exhaustive-match safety
  net (Phase 2 Change 1). The Implementation Approach promises exhaustive
  no-wildcard matches, but the proposed jira `classify` uses `matches!`, which
  has an implicit catch-all; a future `Outcome` variant is silently
  not-provably-unapplied. Also inconsistent with the linear phase, which keeps a
  full `match`. Use an explicit exhaustive `match`.
- **minor / medium** — `Outcome` taxonomy split across a crate boundary with no
  coherence guard for the code-34 divergence (Phase 3 Changes 1 & 3). Retry-class
  grouping (client) and numeric grouping (CLI) are now two independent matches;
  no test asserts the set the CLI labels 34 is exactly the set whose verdict
  flips by operation. A regroup on one side desyncs silently. Add a
  cross-boundary guard.
- **suggestion / medium** — asymmetric split leaves retry-class domain logic in
  the client while numeric mapping leaves (Current State / What We're NOT Doing).
  The client retains a second output path (`TrackerError` for `work-cli`) coupled
  to the `tracker` crate; the modularity gain is partial (deferred to 0271).
  State this explicitly in Desired End State so 0271's reviewers inherit an
  accurate boundary map.

### Correctness

**Summary**: The exit-code redesign is logically sound. Every reachable `Outcome`
was traced through both crates' current `classify` paths and the proposed match
arms reproduce every emitted granular value and every retry-class verdict
exactly, including Linear's code-34 create/update divergence. All named constants
hold the pinned values, the maps are exhaustive over every `Outcome`/`GraphQlError`
variant, and the dropped fixture-count guards cover only transcription-only rows
no `Outcome` reaches. The one correctness observation is a minor loss of a
compile-time exhaustiveness guarantee in the jira rewrite.

**Strengths**:
- The jira rewrite (`matches!(...)` + `provably_unapplied || !mutates()`) is
  behaviourally identical to the current classify→build path; jira never routes
  through `classify_bash_code` at runtime, so the retired 15/17/22 arms are
  correctly irrelevant.
- The linear rewrite reproduces every reachable verdict exactly: {11,35,36}
  retryable on both, {16,20,21} terminal on both, code-34 retryable-create /
  terminal-update, Read always retryable.
- Every named constant holds its pinned value (verified against both
  `exit_codes.rs`).
- Both maps and the linear `classify` match are exhaustive over all variants with
  no wildcard, preserving compile-error-on-new-variant at the mapping sites.
- Phase 4's inlined `for_adf` reproduces `AdfError::code()` exactly; the dropped
  count-guards demonstrably cover only unreachable rows.

**Findings**:
- **minor / high** — jira `classify` trades an exhaustive `match` for a
  non-exhaustive `matches!`, losing the compile-time guarantee the current code
  has (Phase 2 Change 1). Behaviourally identical today; a future variant could
  be misclassified (wrong 70/71 dispatch) with no compile signal. Keep an
  exhaustive `match`.
- **suggestion / medium** — after the change, neither client crate verifies its
  live retry-class against the bridge oracle; parity rests on hand-authored
  columns (Phase 2/3 Change 5). A mis-transcribed column would pass. Consider
  retaining a single fixture-backed assertion feeding the reachable
  `(Outcome, Operation)` rows through `classify` (research Option A).

### Code Quality

**Summary**: Unusually rigorous for a refactor: grounded in exhaustive matches,
preserves the frozen contract, names the successor `exit_code_for_outcome` well,
and explicitly reasons about the surviving wire-status `u16`. The main
maintainability concerns are residual vocabulary inconsistency at the CLI
boundary (`code_for_status` keeps the ambiguous "code" term), one instruction
that would introduce a banned comment, and a slightly imprecise "no wildcard"
statement.

**Strengths**:
- `exit_code_for_outcome(Outcome) -> u8` is domain-expressive and self-documenting.
- Returning `u8` directly dissolves the `u8::try_from(...).unwrap_or(...)`
  fallbacks and their divergent defaults.
- The exit-code/wire-status `u16` distinction is explicitly reasoned about rather
  than left implicit.
- Phase 1 carries a manual checkpoint that each reworded comment still names an
  invariant, aligned with the comment philosophy.
- The exhaustive-match, no-variant-wildcard discipline is stated as a first-class
  design intent.

**Findings**:
- **major / medium** — retained `code_for_status` perpetuates the exit-code/wire-
  status ambiguity the redesign targets (Phase 2/3 Change 3). "code" is the
  return, "status" the input — the exact conflation the rename kills; the plan
  even needs prose to explain it. Rename to `exit_code_for_status` or inline the
  one-line wrapper.
- **major / high** — "restate it as a comment on the linear-cli map" would add a
  banned redundant comment (Phase 3 Change 5). The property is compiler-enforced-
  for-handling; a comment restating it is the what-comment the convention bans.
  Delete the numeric-iterating test and rely on the match (paired with the
  structural test the test-coverage lens asks for); add no comment.
- **minor / medium** — shebang justification comments risk being tooling-
  appeasement rather than genuine why-comments (Phase 2/3 Change 7). If kept,
  word it as the external provenance-capture constraint, not an apology for the
  word "bash"; better, exclude the fixtures dir from the grep gate.
- **minor / high** — "no wildcard" is stated more absolutely than the jira map
  can honour (Implementation Approach, Phase 2 Change 3). The jira map keeps a
  `Status(_)` catch-all; the invariant is variant-exhaustiveness. Reword and note
  the deliberate coercion.
- **suggestion / low** — reworded `failure.rs` docs risk describing code and now
  reference cross-crate behaviour (Phase 2/3 Change 2). Keep them on the
  invariant the failure type guarantees; let the CLI own the mapping statement.

### Test Coverage

**Summary**: The plan preserves the live retry-class oracle well (the
outcome-keyed table with create/update columns is retained, exhaustive matches
stay) and correctly reasons the deleted rows guard no live behaviour. The main
risk is the relocated granular map: the new inline tests — especially linear's —
are specified loosely enough to assert one representative per code rather than
every variant, dropping mutation coverage of the OR-pattern groupings. Secondary
concerns: no mechanism forces a new variant to gain an assertion, the reserved-
codes property is not enforced by the exhaustive match, and the client crates
stop cross-checking against the shared bridge fixture.

**Strengths**:
- The live retry-class oracle is explicitly preserved, including the code-34
  divergence.
- Every mapping stays an exhaustive no-wildcard match (compile-time handling
  guarantee).
- Coverage responsibilities are cleanly separated (discriminant tests, CLI map
  unit test, flow_errors).
- The plan correctly identifies transcription-only rows and dead arms as guarding
  no live path.

**Findings**:
- **major / medium** — linear granular-map unit test under-specified for
  OR-pattern groupings (Phase 3 Change 4). "Every arm (11/36/35/34/16/21/20)" is
  seven codes, but arms bundle multiple `GraphQlError` variants; one
  representative per code misses a variant drifting between arms. Assert every
  constructible `(SuccessWithErrors|BadRequest) × GraphQlError` combination plus
  the standalone outcomes, via an in-test exhaustive `match`.
- **major / medium** — the re-expressed classify introduces `SuccessWithErrors(RateLimited)`,
  which the retained TABLE never exercises (Phase 3 Change 5 & 1). It maps to 34
  with a divergent retry class but is asserted by neither the TABLE nor the
  granular test. Add its row.
- **minor / high** — the reserved-codes property is not enforced by the
  exhaustive match (Phase 3 Change 5). Exhaustiveness constrains the domain, not
  the codomain; a future arm could map to a Jira-only value. Keep a structural
  test that the Linear map never yields {12,13,14,15,17,19}.
- **minor / medium** — dropping the row-count guards leaves no completeness guard
  and drops the shared-fixture cross-check (Phase 2/3 Change 5). Replace with an
  in-test exhaustive `match` over `Outcome` and a lightweight live-row fixture
  cross-check.
- **minor / high** — `flow_errors` does not exercise the code-34 create/update
  divergence (Phase 3 Success Criteria, Testing Strategy). The divergence is a
  70/71 distinction only visible in `work-cli`; `linear-cli` emits 34 for both.
  Attribute the coverage to the `classify` TABLE.
- **suggestion / medium** — sequence the moved unit test before deleting the
  client assertions to avoid a coverage-gap window (Implementation Approach).
  Write `exit_code_for_outcome` and its test first (green), delete client
  assertions after.

### Compatibility

**Summary**: A structural/vocabulary refactor that preserves the frozen exit-code
contract for the classification path exactly — CLI constants equal the old map
value-for-value, the removed `u16→u8` narrowing was never lossy, the Linear
divergence is faithfully reproduced, and the `From<Failure> for TrackerError` →
`work-cli` 70/71 contract is preserved. The significant gap is Phase 4: deleting
`AdfError::code()` does not account for three in-crate test consumers, one of
which is the ADF exit-code differential oracle. Because the client crates aren't
surface-pinned, no API guard fires on the removals — the only backstop is
compile-time breakage, which the plan's narrow per-phase commands can let slip.

**Strengths**:
- The granular relocation preserves every emitted value; the removed
  `u8::try_from(bash_code(..))` never truncated (all codes ≤ 36).
- The Linear divergence is correctly re-expressed on `(Outcome, Operation)`.
- The `From<Failure> for TrackerError` contract `work-cli` depends on is
  preserved (Wire arm still delegates to `classify`; other arms unchanged).
- The plan correctly renames only the per-CLI parity fixture and leaves the
  bridge fixture and its independent reader untouched.

**Findings**:
- **major / high** — Phase 4 deletes `AdfError::code()` but leaves four live
  consumers: `adf.rs:84`, `adf_inventory.rs:173`, `adf_differential.rs:134,138`
  (the differential oracle). "No test parses it" covers only the fixture line.
  Compile break plus a dropped contract guard. Relocate the assertions onto
  `for_adf` and widen the phase verification.
- **minor / medium** — the removed symbols are public but the crates aren't
  surface-pinned, so `public-api:check` won't flag them; narrow single-target
  test commands can pass while the crate doesn't fully compile. Add
  `cargo test -p <crate> --no-run` / clippy `--all-targets` per phase.
- **minor / low** — the renamed parity fixture leaves a dangling reference in
  both CLI `exit_codes.rs` module docs (line 5), and the `detail` string drops
  the `({code})` fragment (no in-repo consumer, parity suite already forbids
  parsing detail). Update the module-doc references as part of the rename.

### Standards

**Summary**: Well-structured, and the client-crate file-touch coverage matches
the actual current "bash" inventory (verified by grep), so the client-scoped
grep=0 AC is achievable and each phase is independently mergeable. The renamed
`capture-*.sh` correctly falls outside `SURVIVING_SHELL_SOURCES`, triggering no
shfmt/ShellCheck/bashisms obligation. The main concerns are naming consistency in
the CLI helpers, residual bash vocabulary in CLI-side doc comments, an
undocumented divergence from the work item's file list, and mild tension between
the mandated justification comments and the comment convention.

**Strengths**:
- Phased delivery is genuinely independently mergeable; the case-insensitive
  grep=0 AC met only after all four is correctly acknowledged.
- The renamed capture script creates no shell-lint obligation — the shell tasks
  enumerate exactly two allowlisted files.
- The production-src touch list matches the live bash inventory exactly.
- Successor names are descriptive, consistent, and preserve discoverability.

**Findings**:
- **minor / high** — `exit_code_for_outcome` vs retained `code_for_status`
  leaves two differently-prefixed names for the same helper family (Phase 2/3
  Change 3). Pick one prefix.
- **minor / medium** — CLI-side `exit_codes.rs` keeps bash-vocabulary doc
  comments (`for_failure` line 139, `for_credential` line 194, module docs)
  outside the client grep AC (Phase 2 Change 3). Reword them or add an explicit
  "NOT doing" note.
- **minor / high** — the work item's file list (`mutation.rs`, `error.rs`,
  `auth.rs`, `upload.rs`) diverges from the plan with no reconciliation; a grep
  confirms the plan is correct but the deviation is untraceable (Phase 1). Add a
  one-line note that the list is stale at this revision.
- **minor / medium** — mandated inline justification comments conflict with the
  very-low comment tolerance and the `.sh` is in no grep AC path, so they guard
  nothing (Phase 2 Change 6/7). Drop the shebang comment; reword `transport.rs`.
- **suggestion / medium** — the Phase 2/3 grep-AC caveat describes a match its
  command can't produce (the `.sh` is not in the grep list). Remove the caveat or
  add the path, consistently.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-09-06T19:21:44+00:00

**Verdict:** APPROVE

All six lenses were re-run against the revised plan. Every finding from Pass 1
is resolved. The re-review surfaced four new issues introduced by the revision
edits; all four were fixed in the same session, so the plan is now approved.

### Previously Identified Issues

Pass 1 raised 5 majors, 10 minors, and 4 suggestions across the six lenses.
Re-review status:

- 🟡 **Compatibility**: Phase 4 `AdfError::code()` consumers — Resolved. New
  Phase 4 change 3 re-homes all four call sites (`adf.rs`, `adf_inventory.rs`
  drop the numeric assertions to the `for_adf` unit test; `adf_differential.rs`
  gains a test-local `expected_adf_exit` avoiding the cargo cycle). Verification
  widened to whole-crate runs and a `.code()` grep.
- 🟡 **Code Quality / Standards**: `code_for_status` ambiguity — Resolved.
  Renamed to `exit_code_for_status`, one `exit_code_*` prefix family.
- 🟡 **Code Quality / Test Coverage**: reserved-codes guard — Resolved. Kept as a
  structural test, "restate as comment" removed.
- 🟡 **Test Coverage**: linear map test under-specified — Resolved. Now asserts
  every constructible `Outcome` via an in-test exhaustive `match`.
- 🟡 **Test Coverage**: untested `SuccessWithErrors(RateLimited)` — Resolved. Row
  added to the retained `TABLE`.
- 🔵 **Architecture / Correctness**: jira `classify` `matches!` — Resolved. Now an
  exhaustive `match`.
- 🔵 **Standards / Compatibility**: CLI-side "bash" prose + dangling fixture
  reference — Resolved. Folded into Phase 2/3 change 3.
- 🔵 **Code Quality**: "no wildcard" overstated — Resolved. Reworded to
  variant-exhaustive, with the jira `Status(_)` exception noted.
- 🔵 **Test Coverage / Correctness**: completeness guard + bridge cross-check —
  Resolved. In-test exhaustive-match guards added; the bridge drop recorded as
  the accepted Option A trade-off.
- 🔵 **Test Coverage**: `flow_errors` divergence overstatement — Resolved.
  Re-attributed to the `classify` `TABLE`.
- 🔵 **Compatibility**: narrow per-phase verification — Resolved. `--all-targets
  --no-run` and whole-crate runs added.
- 🔵 **Standards**: stale work-item file list — Resolved. Documented in Phase 1.
- 🔵 **Architecture**: split-taxonomy coherence — Resolved. Coherence check added
  to Phase 3 manual verification.
- 🔵 **Code Quality / Standards**: comment mandates — Resolved. Shebang and
  transport.rs now reword-not-comment.
- 🔵 **Standards**: grep-AC shebang caveat — Resolved. Corrected.
- 🔵 **Code Quality**: failure.rs cross-crate docs — Resolved. Scoped to the
  type's own invariant.

### New Issues Introduced (all fixed this session)

- 🟡 **Correctness / Test Coverage**: the reserved-codes structural test was
  placed in `linear-client/tests/classify.rs` but calls `exit_code_for_outcome`,
  which now lives in `linear-cli` — the plan's own Phase 4 cargo-cycle rule
  forbids that dependency. **Fixed**: the test is relocated to `linear-cli`'s
  inline `exit_codes.rs` module (change 4), with Phase 3 change 5 and the Testing
  Strategy updated to match.
- 🔵 **Standards**: the Phase 2 grep AC omitted `cli/jira-client/tests/transport.rs`
  though change 6 sweeps it (asymmetric with Phase 3). **Fixed**: added to the
  grep argument list.
- 🔵 **Test Coverage**: `expected_adf_exit` (test-local) is not cross-checked
  against production `for_adf`, so the manual-verification wording overstated the
  guard. **Fixed**: the seam is documented — each side is independently anchored
  to a frozen artefact — and the manual-verification bullets reworded.
- 🔵 **Architecture**: Phase 4 lacked the coherence note Phase 3 gained.
  **Fixed**: a symmetric coherence bullet added to Phase 4 manual verification.

### Assessment

The plan is sound and ready for implementation. The core redesign was verified
behaviour-preserving in Pass 1 (every exit-code value and retry verdict traced
against source); Pass 2 confirms the revision closed every finding and the
edit-induced issues have been corrected. The one substantive Pass-2 finding was
a self-contained test-placement contradiction, now resolved. No open issues
remain.
