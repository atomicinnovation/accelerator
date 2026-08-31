---
type: "plan-review"
id: "2026-08-31-0272-relocate-insecure-local-override-marker-review-1"
title: "Plan Review: Relocate Insecure-Local Override Marker to .accelerator"
date: "2026-08-31T21:43:17+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
parent: "plan:2026-08-31-0272-relocate-insecure-local-override-marker"
target: "plan:2026-08-31-0272-relocate-insecure-local-override-marker"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["correctness", "security", "test-coverage", "documentation", "code-quality", "architecture"]
review_number: 1
review_pass: 3
tags: ["security", "config", "cleanup"]
last_updated: "2026-08-31T22:01:10+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Relocate Insecure-Local Override Marker to .accelerator

**Verdict:** REVISE

The plan's actual edits are sound: the resolver seam stays byte-for-byte
identical, the security semantics are preserved, and the documentation diffs
were verified line-accurate against the live files. What needs revision is the
plan's *verification* — the acceptance greps match the marker basename but never
pin the `.accelerator/` directory that is the whole point of the move, the
production builders that construct the path have no standing test, and the
Testing Strategy claims a symlink test that does not exist. These are cheap
plan-text and acceptance-criteria fixes, not code rework.

### Cross-Cutting Themes

- **Verification proves the basename moved, not the directory** (flagged by:
  correctness, test-coverage, security) — every automated criterion checks that
  `insecure-local-ok` is gone and `allow-insecure-local` is present. A typo like
  `root.join(".claude/allow-insecure-local")` passes both, silently defeating
  the relocation. The production seam is guarded only by a one-time grep, never
  by a CI test.
- **The plan's stated existing coverage is partly fictional** (flagged by:
  test-coverage) — the Testing Strategy asserts existing tests exercise "a
  symlinked personal config"; grep confirms no symlink test exists anywhere in
  `cli/tracker-support/tests/credentials.rs`. The symlink gates the plan leans on
  for acceptance criterion #3 are exercised only by manual steps.
- **The marker literal is duplicated across eight hand-copied sites** (flagged
  by: code-quality, architecture) — the rename is the canonical moment a shared
  constant would pay off; the per-builder grep criteria are themselves a symptom
  of there being no single source of truth. The plan scopes this out explicitly,
  a defensible but debatable call.

### Tradeoff Analysis

- **De-duplication now vs. minimal-diff scope**: Code Quality argues the rename
  is exactly when to extract a shared `.accelerator/allow-insecure-local`
  constant; Architecture agrees the coupling exists but rates leaving it a
  reasonable scope decision, and the work item and research both explicitly
  scope the refactor out. Recommendation: keep the refactor out of *this* plan,
  but the shared-constant extraction would also close the production-seam test
  gap (a constant is trivially unit-testable) — consider it the cheapest way to
  satisfy the test-coverage finding rather than a separate refactor.

### Findings

#### Major

- 🟡 **Correctness / Test-Coverage**: Acceptance greps verify the basename but
  never pin the `.accelerator/` directory — the relocation objective is
  unverified.
  **Location**: Phase 1, Success Criteria: Automated Verification (#1, #4)
  A builder written as `root.join(".claude/allow-insecure-local")` satisfies
  criterion #1 (no `insecure-local-ok`) and #4 (`allow-insecure-local` present)
  while completely defeating the move to `.accelerator/`. The verification can
  pass green with the marker in the wrong directory. This gap was already
  flagged in the work-item review.

- 🟡 **Test-Coverage / Security**: Production marker-path construction has no
  standing test; acceptance criterion #2 is guarded only by a one-time grep.
  **Location**: Testing Strategy (Unit Tests); Implementation Approach
  The three builders (`jira-cli/src/context.rs:165`, `linear-cli/src/context.rs:145`,
  `work-cli/src/tracker_registry.rs:124`) are exercised by no test — every
  credential test injects its own marker. Nothing in CI pins them to the new
  path, so a later edit could repoint a builder or add a fallback with zero
  regression signal on a security-sensitive gate.

- 🟡 **Test-Coverage**: The plan overstates existing coverage — the claimed
  symlink test does not exist. **CONFIRMED** by grep.
  **Location**: Testing Strategy (Unit Tests)
  The Testing Strategy asserts existing tests exercise "a symlinked personal
  config," but `cli/tracker-support/tests/credentials.rs` contains no symlink
  test at all (no `symlink` string, no such test function), and no test covers
  the marker's `is_file()` symlink gate at `credentials.rs:426` that acceptance
  criterion #3 relies on. The plan uses this inaccurate claim to justify adding
  no new tests.

- 🟡 **Code-Quality / Architecture**: The rename is the moment to extract a
  shared marker constant; grep acceptance criteria substitute for
  de-duplication.
  **Location**: What We're NOT Doing — "No refactor of the duplication"; Phase 1 §1
  The literal is hand-copied at three production sites plus five fixtures. A
  `const` in `tracker-support` (where `CredentialContext` lives) referenced by
  all three callers would give a compiler-enforced single source of truth and
  retire the per-builder greps. Grep catches a *missing* update, not a builder
  silently diverging to a different path.

#### Minor

- 🔵 **Architecture**: Marker trackability is now implicitly coupled to
  `.accelerator/.gitignore`.
  **Location**: Desired End State; Phase 1 Success Criteria
  The VCS-tracked gate is the security hinge. Today `.accelerator/.gitignore`
  ignores only `config.local.md`, so the marker is trackable — but a future
  broad rule (`.accelerator/*`) would render it untrackable and silently disable
  the override (fail-closed refuse), with no failing test to catch it.

- 🔵 **Code-Quality**: Two divergent literal forms of the same conceptual marker
  path.
  **Location**: Phase 1 §§3–4: flat vs contract-harness fixtures
  Bare `allow-insecure-local` in flat fixtures, full
  `.accelerator/allow-insecure-local` in contract harnesses. A maintainer must
  re-derive the bare-vs-full distinction; a one-line note or a shared constant
  would remove the cognitive load.

- 🔵 **Documentation**: The `credentials.rs:139` citation is attached to the
  wrong diff hunk.
  **Location**: Phase 1, Changes Required §3
  Line 139 is the `Workspace::marker` helper body
  (`self.root.path().join("insecure-local-ok")`), which is §3's *second* hunk —
  the first hunk's `insecure_marker: root.join("insecure-local-ok")` context does
  not appear there. Purely a citation placement nit; the change is fully
  specified.

- 🔵 **Correctness**: No CI-executed test exercises the nested `.accelerator/`
  marker path used in production.
  **Location**: Phase 1, Changes §3
  Flat fixtures keep the bare basename at the temp root; the contract harnesses
  that use the nested path are gated on live tenant credentials and excluded from
  the default run. Benign today (the resolver is path-agnostic), latent if marker
  placement ever becomes directory-sensitive.

#### Suggestions

- 🔵 **Architecture**: The hard-cutover premise assumes the override gate is
  intended to ship.
  **Location**: Migration Notes / Overview
  The research flagged that the 0197 port plan recorded "no bypass gate" while
  the live resolver implements one — possible plan-vs-code drift over whether the
  override is deliberate. If the override were later judged unintended, this
  ten-site rename is churn on a soon-deleted seam. Note the contingency and
  confirm intent before landing.

- 🔵 **Architecture**: Test fixtures diverge from the production path shape.
  **Location**: Phase 1 §3 vs §4
  A reasonable pragmatic tradeoff given the path-agnostic resolver; a shared
  fixture helper creating the `.accelerator/` parent would let all fixtures adopt
  the production shape if consistency is later wanted.

### Strengths

- ✅ The resolver seam is correctly identified and left untouched:
  `refuse_insecure_personal_config` and `insecure_override_allowed` consume
  `insecure_marker` abstractly, so the four-gate check is provably preserved
  (verified by correctness, security, architecture).
- ✅ The new path `.accelerator/allow-insecure-local` is genuinely trackable —
  neither `.accelerator/.gitignore` nor the root `.gitignore` matches it — so the
  VCS-tracked gate stays satisfiable (verified against both ignore files).
- ✅ The `warns` → `refuses` doc correction matches the code (the resolver
  returns `Err(LocalPermsInsecure)`, never merely warns), removing a dangerous
  understatement and making Linear's "mirroring the Jira integration" claim
  truthful for the first time.
- ✅ Every user-facing diff hunk (`SKILL.md` :707-709, :808-811; docstring
  :375-377) matches the live file text verbatim, line numbers included.
- ✅ The single write-site reasoning holds: keeping the flat fixture's basename
  bare keeps `fs::write(&marker, "")` targeting the temp root, so no
  `create_dir_all` is needed.
- ✅ Honest TDD reasoning: the plan correctly declines to fabricate a ceremonial
  failing test for a fixture-path move under an existing behavioural net.

### Recommended Changes

1. **Pin the full path in the acceptance criteria** (addresses: "greps verify
   basename not directory"; "no CI test at the production seam"). Replace the
   basename greps with full-path ones —
   `rg -n '\.accelerator/allow-insecure-local' cli/jira-cli/src/context.rs
   cli/linear-cli/src/context.rs cli/work-cli/src/tracker_registry.rs` matches
   all three — and add a negative guard
   `rg -n '\.claude/allow-insecure-local' cli/ skills/` returns no matches.

2. **Correct the Testing Strategy's coverage claim** (addresses: "plan
   overstates existing coverage"). Remove the assertion that a symlinked personal
   config is already tested. Either add a symlinked-marker /
   symlinked-personal-config test asserting `LocalPermsInsecure`, or explicitly
   state these branches are covered only by manual verification and are out of
   automated scope.

3. **Add a standing regression guard at the production seam** (addresses:
   "production marker-path construction has no test"). The lowest-cost option
   doubles as the de-duplication fix: extract a
   `const INSECURE_MARKER_RELATIVE: &str = ".accelerator/allow-insecure-local"`
   in `tracker-support`, have all three builders reference it, and pin it with
   one unit test — converting the throwaway grep into standing coverage.

4. **Fix the `credentials.rs:139` citation** (addresses: "citation attached to
   wrong hunk"). Move the `:139` reference from §3's first hunk to the
   `Workspace::marker` helper hunk that actually matches that line.

5. **Note the trackability invariant and the override-intent contingency**
   (addresses: ".gitignore coupling"; "hard-cutover premise"). Record that
   `.accelerator/allow-insecure-local` must remain unignored, and that the rename
   is contingent on the override being an intended, shipping feature (per the
   research open question).

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Correctness

**Summary**: A pure, mechanical path-literal rename with the resolver logic left
byte-for-byte untouched, so there are no logical, arithmetic, boundary, or
concurrency risks in the changed code. Every cited line number, both fixture
literal forms, and the single write-site's no-`create_dir_all` reasoning were
verified and hold. The one substantive concern is that the automated acceptance
criteria verify the new basename appears but never pin the `.accelerator/`
directory, so a wrong-directory literal would pass every grep while defeating the
work item's entire purpose.

**Strengths**:
- The resolver seam (`insecure_marker: PathBuf` consumed path-agnostically at
  `credentials.rs:424`) is correctly identified; behaviour is provably preserved.
- The write-site analysis is correct — the marker is written to a bare temp-root
  sibling, so renaming the basename needs no `create_dir_all`.
- Both distinct fixture literal forms are accurately catalogued; the contract
  harnesses run under `NothingTracked` provenance so the swap is inert there.
- The hard-cutover reasoning is sound — builders align with the
  `.accelerator/config.local.md` sibling; the tracked check is path-agnostic.

**Findings**:
- 🟡 major (medium): Acceptance greps verify the basename but never pin the
  `.accelerator/` directory — the relocation objective is unverified. A builder
  written `root.join(".claude/allow-insecure-local")` satisfies criteria #1 and
  #4 while defeating the move. Add a full-path positive grep and a
  `.claude/allow-insecure-local` negative grep.
- 🔵 minor (low): No CI-executed test constructs or reads the marker at a nested
  subdirectory; flat fixtures keep the bare basename, contract harnesses are
  credential-gated. Benign today; latent if placement becomes directory-sensitive.

### Security

**Summary**: A path-literal relocation of a credential-permission override
marker, and sound from a security standpoint. The resolver reads the marker
abstractly through `context.insecure_marker`, so all four gates — env `"1"`,
regular-file (symlink-rejecting `is_file()`), VCS-tracked, and the underlying
`0600` permission gate — are preserved byte-for-byte; only the caller-supplied
`PathBuf` moves. The new path is not matched by `.accelerator/.gitignore` nor the
root `.gitignore`, so it stays trackable. The `warns` → `refuses` correction
accurately describes the real control, and every risky edge fails closed toward
refusing credentials.

**Strengths**:
- The override remains path-agnostic at the resolver; the byte-for-byte claim is
  verified.
- The new marker path is trackable — the VCS-tracked gate stays satisfiable and
  is not inadvertently ignored.
- The doc correction is a real security-accuracy improvement — the code returns
  `Err(LocalPermsInsecure)`, so the prior "warns" wording materially understated
  the control.
- The hard cutover fails closed: a repo that committed the old marker has its
  override silently stop firing (credentials become more protected, never less).
- Manual verification covers the security-negative cases (legacy path not
  honoured; symlinked/untracked markers still refuse).

**Findings**:
- 🔵 suggestion (medium): The "old path no longer honoured" property is enforced
  only by a one-shot grep and a manual step, not a standing test. Add a unit test
  per production builder asserting its `insecure_marker` equals
  `.accelerator/allow-insecure-local`, turning the no-fallback property into a
  durable CI guard.

### Test Coverage

**Summary**: The plan leans entirely on the claim that existing path-agnostic
resolver tests suffice and that acceptance criterion #2 is covered by a one-time
grep. That reasoning is partly sound — the resolver genuinely is path-agnostic —
but it leaves a durable gap: the production builders that construct the marker
path have no automated coverage, the verification greps match only the basename,
and the plan overstates existing coverage by claiming a symlink case is tested
when none exists.

**Strengths**:
- Correctly identifies the resolver as path-agnostic, so existing behavioural
  assertions survive the fixture-path move without rewrites.
- Reasons carefully about the single write-site test — keeping the basename bare
  avoids a `create_dir_all` hazard that was spotted and avoided.
- Correctly determines the public-API snapshot and `E_LOCAL_PERMS_INSECURE`
  string are unaffected.

**Findings**:
- 🟡 major (high): Production marker-path construction has no ongoing test;
  acceptance criterion #2 is guarded only by a one-time grep not part of
  `mise run check`. A future edit could reintroduce a stale path and the whole
  resolver suite still passes green.
- 🟡 major (high): The plan overstates existing coverage — the claimed symlinked
  personal config test does not exist (grep for `symlink` returns nothing), and
  no test covers the marker's `is_file()` symlink gate at `credentials.rs:426`.
  **Confirmed by the orchestrator's own grep.**
- 🔵 minor (high): Verification greps match the basename, not the full path — a
  wrong-directory typo (`.claude/allow-insecure-local`) passes both the positive
  and negative checks. Anchor the builder greps on the full path literal.

### Documentation

**Summary**: Fundamentally a documentation-accuracy exercise wrapped around a
path rename, and it handles the documentation dimension well. Every user-facing
diff hunk — `SKILL.md` :707-709, :808-811 and the `credentials.rs` :375-377
docstring — was verified against the live files and the "before" text matches
exactly, line numbers included. The `warns` → `refuses` correction is accurate
against the resolver code, the symmetric parity fix genuinely makes the two
`SKILL.md` passages mirror each other, and the grep scope covers every shipped
documentation surface — no README or docs-site content references the marker.

**Strengths**:
- The user-facing diff contexts are accurate to the line; an implementer will
  find clean matches.
- The `warns` → `refuses` correction is verified against the code
  (`refuse_insecure_personal_config` returns an error, never warns).
- The symmetric fix genuinely restores Jira/Linear parity, making Linear's
  cross-reference truthful for the first time.
- The acceptance greps are scoped correctly; `rg -c 'allow-insecure-local'
  SKILL.md` returning 2 is right, and no other shipped doc surface is missed.
- The `insecure_marker` struct field is documented abstractly, consistent with
  the "path is a caller concern" framing; `:375-377` is correctly the sole
  docstring naming the path.

**Findings**:
- 🔵 minor (high): The `cli/tracker-support/tests/credentials.rs (:139)`
  citation in §3 is attached to the wrong hunk — line 139 is the
  `Workspace::marker` helper body, which is §3's second hunk, not the first
  hunk's `insecure_marker: root.join(...)` context. Move the citation to the
  helper hunk.

### Code Quality

**Summary**: A clean, well-scoped path-literal rename that correctly leaves the
path-agnostic resolver seam untouched and honestly reasons about why TDD's
red-green loop does not apply. Its main weakness is declining to de-duplicate the
marker literal at exactly the moment the duplication bites — the rename must
touch eight hand-copied sites and leans on grep acceptance criteria as a
substitute for a compiler-enforced single source of truth. A secondary smell is
keeping two divergent fixture forms of the same conceptual path.

**Strengths**:
- Honest, accurate reasoning about TDD applicability — no fabricated ceremonial
  test.
- Corrects the docstring and the inaccurate Jira prose alongside the code change.
- Introduces no comments and no dead code, adhering to the low-comment
  convention.
- Respects the existing design seam — the path-agnostic gate logic is left alone.

**Findings**:
- 🟡 major (medium): The rename is the moment to extract a shared marker
  constant; grep ACs substitute for de-duplication. A `const` in `tracker-support`
  referenced by all three builders would give a single source of truth and retire
  the per-builder greps. Grep catches a missing update, not a silent divergence.
- 🔵 minor (medium): Two divergent literal forms of the same conceptual marker
  path (bare basename vs full path). If a shared constant is introduced the flat
  fixtures can reference the same basename; otherwise a one-line note would remove
  the cognitive load.

### Architecture

**Summary**: The plan correctly honours the central boundary — the
`insecure_marker: PathBuf` seam keeps the resolver path-agnostic, so relocating
the marker is confined to caller-side edits plus a docstring. The
single-atomic-phase and hard-cutover decisions are sound and their tradeoffs are
explicitly acknowledged given the unreleased status. The main observation is that
the rename fans out across three independently hand-built `CredentialContext`
literals, and that moving the marker into `.accelerator/` introduces a new
implicit dependency on that directory's `.gitignore` for the security-critical
VCS-tracked gate.

**Strengths**:
- Respects the resolver seam; no logic crosses the boundary.
- The single-atomic-phase decision is well-reasoned — the three builders are
  genuinely independent with no dependency ordering to exploit.
- The hard-cutover tradeoff is explicitly acknowledged and time-bounded.
- Aligning the marker as a sibling of `.accelerator/config.local.md` improves
  local cohesion and makes a future shared-helper extraction more visible.

**Findings**:
- 🔵 minor (high): Marker path duplicated across three hand-built
  `CredentialContext` literals; the per-builder grep criteria are a symptom of
  shotgun-surgery shape. Accept for this rename, but capture a follow-up to
  extract a shared layout builder.
- 🔵 minor (medium): Marker trackability is now implicitly coupled to
  `.accelerator/.gitignore`. A future broad ignore rule (`.accelerator/*`) would
  render the marker untrackable and silently disable the override, with no
  failing test. Add an assertion (or ADR/comment) that the path is not ignored.
- 🔵 suggestion (medium): Test fixtures diverge from the production path shape
  (bare basename vs nested). A reasonable pragmatic tradeoff; a shared fixture
  helper creating the `.accelerator/` parent would restore uniformity if wanted.
- 🔵 suggestion (low): The hard-cutover premise assumes the override gate is
  intended to ship. The research flagged possible plan-vs-code drift (0197 port
  plan recorded "no bypass gate"); note the contingency and confirm intent before
  landing.

## Re-Review (Pass 2) — 2026-08-31T22:01:10+00:00

**Verdict:** COMMENT

The revision resolves every finding from Pass 1. All four majors are closed: the
shared `INSECURE_MARKER_RELATIVE` constant referenced by all five callers turns
acceptance criterion #2 into a compile-time property (de-dup + seam-test in one),
the full-path and negative greps close the wrong-directory gap, and the false
symlink-coverage claim is retracted with two real characterisation tests added.
The re-review surfaced one new major — the added symlink tests were
under-specified and would have passed for the wrong reason — plus a sharp
correctness catch on the `git check-ignore` criterion's exit-code and scope. Both
were fixed in this pass; nothing critical remains, so the verdict moves REVISE →
COMMENT. The plan is ready to implement.

### Previously Identified Issues

- 🟡 **Correctness/Test-Coverage**: Greps verify basename, not `.accelerator/`
  directory — **Resolved**. Full-path positive grep + negative
  `.claude/allow-insecure-local` grep + seam-pin test.
- 🟡 **Test-Coverage/Security**: Production seam untested; AC#2 grep-only —
  **Resolved**. Shared constant + `the_marker_path_lives_under_accelerator` pin;
  compile-time references at all five sites.
- 🟡 **Test-Coverage**: Plan overstated coverage — claimed symlink test —
  **Resolved**. Claim retracted; two Unix-gated symlink tests added; TDD framing
  corrected to characterisation.
- 🟡 **Code-Quality/Architecture**: Rename is the moment to extract a shared
  constant — **Resolved**. `pub const INSECURE_MARKER_RELATIVE` in
  `tracker-support`, re-exported at the crate root per existing convention.
- 🔵 **Architecture**: Trackability coupled to `.accelerator/.gitignore` —
  **Resolved** (with refinement below). `git check-ignore` acceptance criterion
  added.
- 🔵 **Code-Quality**: Two divergent fixture literal forms — **Resolved**. Bare
  basename retained with plan-level rationale (not a code comment).
- 🔵 **Documentation**: `credentials.rs:139` citation on wrong hunk —
  **Resolved**. Section split; helper carries a `:138` citation, verified against
  the live file.
- 🔵 **Correctness**: No CI test at the nested path — **Partially resolved**
  (accepted). Contract harnesses use the constant but are credential-gated; the
  resolver is path-agnostic, so the residual gap carries no logic divergence.
- 🔵 **Architecture** (×2), 🔵 **Security**: framing + standing-guard
  suggestions — **Resolved / accepted**. Override-intent precondition added to
  Migration Notes; asymmetry noted.

### New Issues Introduced (all fixed this pass)

- 🟡 **Test-Coverage**: The two new symlink tests were under-specified and
  over-determined — each would pass even if the gate it guards were deleted
  (`symlink_metadata` reads the link's own 0o777 mode, so the config-symlink test
  refuses via the mode path regardless of the `:388` gate; the marker-symlink
  test refuses at the env-var short-circuit before `:426`). **Confirmed** by
  tracing the gate order. Fixed: §6 now pins every other condition so only the
  target gate flips the outcome.
- 🔵 **Correctness**: The `git check-ignore` criterion treated exit 128 (error)
  as a pass and its wording implied it guarded the end-user repo's ignore rules,
  which it cannot. **Confirmed** — the check inspects the accelerator repo's own
  `.gitignore` (exit 1 today). Fixed: criterion now asserts exit exactly 1 and
  scopes its claim to this repo.
- 🔵 **Test-Coverage**: The seam-pin test's "red" state is a compile error, not
  an assertion failure. Fixed: reframed as a change-detector that does not compile
  until the constant exists.
- 🔵 **Documentation**: Stale `(see §6)` cross-reference after renumbering
  pointed at the new-tests section, not the write-site rationale. Fixed: repointed
  to the Testing Strategy write-site bullet.
- 🔵 **Documentation**: The new constant lacked the one-line rustdoc every
  sibling `pub` item carries. Fixed: `///` added, documenting public API (within
  the comment convention).
- 🔵 **Code-Quality**: The sibling `personal_config` path stays duplicated while
  the marker is centralised — asymmetric single-source-of-truth. Addressed by
  tightening the scope note to state the symmetric `Level::Personal.filename()`
  option was considered and deferred to keep the change scoped to the marker.

### Assessment

The plan is in good shape and ready to implement. The verdict is COMMENT rather
than APPROVE only because the plan carries an acknowledged open precondition —
confirm the insecure-local override is an intended, shipping feature (the
research's unresolved 0197 plan-vs-code-drift question) before landing the hard
cutover. Two residual items are accepted tradeoffs, not blockers: the nested-path
coverage gap (benign under a path-agnostic resolver) and the `personal_config`
de-duplication (deliberately out of 0272's scope).

## Precondition Resolution (Pass 3) — 2026-08-31T22:01:10+00:00

**Verdict:** APPROVE

The sole blocker to APPROVE — confirming the insecure-local override is an
intended, shipping feature — is **resolved in favour of intended**. The research's
"plan-vs-code drift" was a misreading. The 0197 plan's "no bypass gate" line
(`2026-08-08-0197-…:265-266`) is scoped to the config-*write* path
(`WriteConfigLevel::write` clamps a personal write to 0600, so `config set` needs
no bypass), not the read-side resolver override that handles a hand-created,
loose-permission `config.local.md`. The two are different code paths and do not
conflict.

Three facts confirm intent: the override was designed deliberately in the origin
plan (`2026-04-29-jira-integration-phase-1-foundation:976-996`, the opt-out with
its six-case matrix); it ships wired into the resolver (`insecure_override_allowed`,
`credentials.rs:418-428`); and it carries a matching named test
(`the_insecure_override_needs_both_the_variable_and_a_tracked_marker`), which
asserts the tracked-marker path resolves and the untracked path refuses. The hard
cutover rests on a verified premise. The plan's Migration Notes were updated to
record this resolution.

With the precondition cleared and all Pass-1 and Pass-2 findings resolved, the
plan is approved for implementation. The two accepted tradeoffs (nested-path
coverage, `personal_config` de-duplication) remain non-blocking.

---
*Re-review generated by /accelerator:review-plan*
