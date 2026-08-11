---
type: plan-review
id: "2026-08-10-0185-converge-corpus-adapters-on-library-backed-vcs-review-1"
title: "Plan Review: Converge corpus-adapters on the Library-Backed VCS Adapter Implementation Plan"
date: "2026-08-10T16:06:48+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-08-10-0185-converge-corpus-adapters-on-library-backed-vcs"
target: "plan:2026-08-10-0185-converge-corpus-adapters-on-library-backed-vcs"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, security, safety, compatibility]
review_number: 1
review_pass: 4
tags: []
last_updated: "2026-08-10T16:52:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Converge corpus-adapters on the Library-Backed VCS Adapter Implementation Plan

**Verdict:** REVISE

The plan is carefully sequenced and its Current State Analysis is unusually well-verified against the live codebase — it corrects several stale claims from the source work item rather than propagating them. Its core wiring change (repointing `vcs_adapters::facts` at `InProcessProbe`) is logically sound and its boundary-preservation claims mostly hold up. However, Phase 3's `CommandProbe` deletion is scoped incompletely in a way that will break the build, and the plan's three Phase 1 policy decisions (containment bound, sha256 handling, snapshot-on-read) are each less complete than their doc comments claim once checked against the wider consumer set and the specific call sites this switch newly exposes.

### Cross-Cutting Themes

- **`tests/library.rs` is missing from Phase 3's file inventory** (flagged by: code-quality, test-coverage, correctness, compatibility) — four independent lenses found the same certain compile break: `cli/vcs-adapters/tests/library.rs` imports and constructs `CommandProbe` directly, including in the one test that pins the plan's own documented snapshot-on-read divergence. This is the single most corroborated finding in the review.

- **The containment-bound decision is under-analysed relative to what's actually changing** (flagged by: architecture, security, safety) — three lenses converge on the same underlying gap from different angles: the plan frames the choice as "subprocess isolation vs. nothing" without evaluating a cheap in-process middle path (Security); it removes `CommandProbe`'s existing 10-second cap from a call site that now runs while a work-item creation lock is held, with no reclaim path for a hung-but-alive holder (Safety); and the "revisit if that changes" language in the doc comment has no concrete, tracked trigger tying it to 0168's server-integration work (Architecture, Security).

- **The snapshot-on-read "no persisted-data impact" claim is scoped too narrowly** (flagged by: safety, compatibility) — both lenses independently found that Phase 1's confirmation only audits corpus-adapters' own Rust write paths, missing the dozens of `SKILL.md` workflows that copy `corpus metadata derive`'s printed `Current Revision:` line verbatim into committed `meta/` document frontmatter. The plan's own frontmatter is itself an example of this pattern.

- **Fixture-binary placement/gating deviates from the established sibling-crate convention** (flagged by: architecture, code-quality, test-coverage) — three lenses independently noticed the new `corpus-adapters-fixture` binary sits in `src/bin/` rather than following `vcs-adapters-fixture`'s deliberate `tests/fixtures/` placement, with no note explaining the divergence.

### Tradeoff Analysis

- **Scope discipline vs. containment robustness**: the plan's "What We're NOT Doing" section explicitly declines to build any bounded-execution primitive, framing that as appropriately out of scope for a wiring-and-deletion task. Security's critical finding pushes the other way — arguing a narrow, call-site-specific bound (a timeout thread plus panic-catching around just the widened calls) is cheap enough that "out of scope" undersells the risk, particularly at the lock-holding `work create` call site Safety separately identified. This is a real judgement call: the plan's scope boundary is reasonable in isolation, but the two lenses together suggest the boundary was drawn without weighing the specific new exposure this switch introduces, rather than as a considered tradeoff.

### Findings

#### Critical

- 🔴 **Code Quality, Test Coverage, Correctness, Compatibility**: `CommandProbe` deletion omits `tests/library.rs`, which will fail to compile
  **Location**: Phase 3: Delete `CommandProbe` and collapse the dual-adapter comparison
  `cli/vcs-adapters/tests/library.rs` imports `CommandProbe` and uses it in `assert_parity` (several tests) and in `an_unsnapshotted_edit_is_the_one_documented_divergence` — the sole automated proof of the plan's own documented behavioural exception. Phase 3's Changes Required lists only `subprocess.rs` and `tests/detection.rs`; as scoped, `cargo test -p vcs-adapters --features bash-parity` and the `grep -r CommandProbe` check both fail.

- 🔴 **Security**: Containment-bound decision treats subprocess-isolation-vs-nothing as the only two options
  **Location**: Phase 1, item 2: Containment-bound decision / "What We're NOT Doing"
  `CommandProbe` actively provides a 10-second cap, kill-on-timeout, and scrubbed environment for the `facts()` composition root today — this plan removes that protection from this call site rather than merely declining to add a new one, without evaluating a cheap in-process middle path (timeout thread + `catch_unwind`) that wouldn't require reintroducing a subprocess boundary.

- 🔴 **Compatibility**: Phase 4's MPL-2.0 re-check is scoped to `accelerator-visualiser` only, missing sub-binaries that already call `InProcessProbe` directly
  **Location**: Phase 4: Re-run the MPL-2.0 licence check
  `cli/vcs-cli` (`accelerator-vcs`) and `cli/collaboration-cli` (`accelerator-collaboration`) both construct `InProcessProbe` directly and call it unconditionally from `main()` — dead-code elimination cannot remove directly-called code, so the `gix`/`jj-lib` closure (including `uluru`) is very likely already linked into and reachable from these two already-shipped, dispatched sub-binaries, independent of this plan. After this switch, `accelerator-corpus` and `accelerator-work` join them. Re-checking only `accelerator-visualiser` leaves the `deny.toml` comment's "actual finding" claim incomplete.

#### Major

- 🟡 **Safety, Compatibility**: Snapshot-on-read confirmation audits only Rust write paths, missing skill-layer consumers of the printed revision
  **Location**: Phase 1, item 3: Snapshot-on-read dependency confirmation / Migration Notes
  Multiple `SKILL.md` workflows (`create-plan`, `research-issue`, `create-note`, and others) invoke `corpus metadata derive` and persist its `Current Revision:` output into committed frontmatter. The Migration Notes' claim of "no persisted-data impact" doesn't hold at this broader scope — an artefact authored with unsnapshotted edits present will silently record a stale revision after the switch.

- 🟡 **Safety**: Dropping the timeout removes a hang guard specifically at a lock-holding call site
  **Location**: Phase 1, item 2 (containment-bound decision)
  `work-cli`'s `create` acquires a work-item-creation lockdir guard and then calls `derive_at`, which after this switch reaches an unbounded `InProcessProbe` with no cap. The lockdir's reclaim mechanism only reclaims a *dead* holder — a hung-but-alive process is never reclaimed, so a pathological repository state could block every subsequent `work create` indefinitely.

- 🟡 **Architecture, Security**: No tracked trigger to revisit the containment-bound decision when server reachability changes
  **Location**: Phase 1, item 2 (containment-bound decision) / Phase 4
  The doc comment's "revisit if that changes" has no concrete follow-up mechanism, and Phase 4 requires filing a follow-up work item only for the licensing consequence of increased reachability, not for containment — even though both are triggered by the same event (a server call site into `facts`).

- 🟡 **Security**: No adversarial-input test proves `InProcessProbe` fails gracefully rather than hanging or panicking
  **Location**: Phase 2: Extend zero-spawn coverage, then repoint `facts` / Testing Strategy section
  Existing unit tests cover permission-denial and absent repositories, but none exercise structurally malformed input (a truncated `.jj` checkout protobuf, a corrupted git object graph) — exactly the class of input a now-wider caller set is more likely to encounter.

- 🟡 **Test Coverage**: The sha256 revision-folding policy is documented as "already covered" by tests that don't actually exercise `revision()`
  **Location**: Phase 1, item 1: sha256 revision-handling policy
  `an_unsupported_object_format_fails_rather_than_misreads` and its `classify.rs` counterpart assert other queries fail on a sha256 fixture, but neither calls `probe.revision(...)`. No test in the suite exercises `revision()` against a sha256 repository, so the doc comment's specific behavioural claim is unproven.

- 🟡 **Test Coverage**: The new zero-spawn test's fixture scope may not exercise both the git and jj composition paths
  **Location**: Phase 2: Failing-first zero-spawn assertion
  The new test is described against a single, unspecified-kind fixture. Since `facts` dispatches differently per `VcsKind`, a single-kind test may not prove the zero-spawn property holds for both dispatch paths through the new composition.

#### Minor

- 🔵 **Architecture, Code Quality, Test Coverage**: New fixture binary's placement and gating diverge from the sibling crate's established convention
  **Location**: Phase 2, item 1: New reference binary for the metadata-read path
  `vcs-adapters-fixture` is deliberately placed under `tests/fixtures/` (with a `Cargo.toml` comment explaining why) rather than `src/bin/`. The new `corpus-adapters-fixture` binary is proposed at `src/bin/`, gated by `required-features` instead — a different mechanism for the same category of artefact, undiscussed.

- 🔵 **Correctness**: `RepoRoot` canonicalisation semantics differ between the two implementations, unaddressed by the plan
  **Location**: Phase 2: Extend zero-spawn coverage, then repoint `facts` / Current State Analysis
  `InProcessProbe::discover` canonicalises (resolves symlinks); `MarkerWalkRoot::discover` does not. Every existing/planned test pre-canonicalises its fixture root, structurally masking this divergence, so the "invisible to callers" claim holds only for the tests, not necessarily for a caller passing a relative or symlink-laden path.

- 🔵 **Security**: Environment-scrubbing asymmetry between `CommandProbe` and `InProcessProbe` is not addressed
  **Location**: Phase 1: Record the pending policy decisions
  `CommandProbe` scrubs `GIT_DIR`/`GIT_CONFIG`/`JJ_CONFIG` and related variables before spawning; `InProcessProbe`'s `gix`/jj-lib calls have no equivalent scrubbing. Not surfaced as a policy decision alongside the other three.

- 🔵 **Safety**: sha256-repository failure folds to a silent "no VCS here" with only debug-gated logging
  **Location**: Phase 1, item 1 (sha256 revision-handling policy)
  The failure is `warn!`-logged, but this codebase gates logging behind `ACCELERATOR_LOG`, so a typical run shows no signal distinguishing "unsupported format" from "no repository at all."

- 🔵 **Compatibility**: No automated guard ties `deny.toml`'s reachability rationale to the actual call graph
  **Location**: Phase 4: Re-run the MPL-2.0 licence check
  The exception rationale is a point-in-time prose comment with no CI check re-verifying it when a new call site is added — the scope gap found above illustrates how easily this drifts.

#### Suggestions

- 🔵 **Architecture**: Reachability verification method is a byte-literal proxy sensitive to dependency wording changes
  **Location**: Phase 4: Re-run the MPL-2.0 licence check
  Grepping for `extensions.objectFormat` / `There is no Jujutsu repo` is carried over unchanged from 0188; worth a note that it should be re-validated, not just re-run, whenever `gix`/`jj-lib` are upgraded.

### Strengths

- ✅ The four phases are ordered so policy decisions precede the behaviour change that depends on them, and deletions are deferred until nothing else references what's being removed — each phase stays independently mergeable and low-risk to review in isolation.
- ✅ Phase 2 requires proving the new zero-spawn test fails against the current `CommandProbe` wiring before flipping the composition root — genuine red/green discipline, not an assertion added alongside code that already satisfies it.
- ✅ The Current State Analysis is unusually well-verified: it traces actual current line numbers and call sites, corrects the work item's stale `work_item_pattern_parity.rs` reference, and independently confirms (rather than assumes) that the visualiser server has no current call site into `facts`.
- ✅ The plan is disciplined about scope: it keeps `scrub_environment`/`run_capped` (shared with 0198) and `MarkerWalkRoot`/`markers.rs` untouched, having confirmed actual callers before proposing any deletion.
- ✅ Recording the sha256, containment-bound, and snapshot-on-read decisions as doc comments in the code (rather than leaving them implicit) is good practice for auditable, security-relevant tradeoffs — the content gaps found above are about completeness, not about the practice itself.

### Recommended Changes

1. **Add `cli/vcs-adapters/tests/library.rs` to Phase 3's Changes Required** (addresses: the four-lens critical finding). Decide explicitly whether `an_unsnapshotted_edit_is_the_one_documented_divergence` is rewritten to invoke the real `jj` binary directly (mirroring the file's own `jj_revision_oracle` helper) or deleted with the loss of coverage acknowledged; update or remove the remaining `assert_parity`-based tests since parity with a deleted type is meaningless.

2. **Broaden Phase 4's MPL-2.0 re-check to every dispatched sub-binary that reaches `vcs_adapters`** (addresses: compatibility critical), not just `accelerator-visualiser` — at minimum `accelerator-vcs`, `accelerator-collaboration`, and after this switch `accelerator-corpus`/`accelerator-work`. If the closure is already reachable in the first two independent of this plan, that's a pre-existing finding worth its own urgency, not one to fold silently into a comment scoped to the visualiser alone.

3. **Re-scope the containment-bound decision to reflect what's actually changing** (addresses: the three-lens containment theme, both major findings on this topic). At minimum, record that a protection is being removed from the `facts()` call site (not just declined-to-add), weigh the lock-holding blast radius at `work create` explicitly, and either accept a narrow call-site-specific bound or record why even that's rejected — with the acceptance tied to a concrete trigger (e.g. a cross-reference to 0168) rather than an unenforced "revisit if that changes."

4. **Broaden the snapshot-on-read confirmation to cover skill-layer consumers** (addresses: safety + compatibility major). Either extend the doc comment and Migration Notes to explicitly address the `SKILL.md` workflows that persist `Current Revision:` into frontmatter and accept the staleness with that scope in view, or flag it as a genuine known regression rather than a closed question.

5. **Back the sha256 doc comment's claim with an actual test of `revision()`** (addresses: test-coverage major) — extend or add a sibling to `an_unsupported_object_format_fails_rather_than_misreads` that calls `InProcessProbe.revision(...)` against the sha256 fixture and asserts `None`.

6. **Add at least one malformed-input test for `InProcessProbe`'s parsers** (addresses: security major) — a truncated checkout protobuf and/or corrupted git object graph, asserting `Err`/`None` rather than a panic or hang, before this path's reach widens.

7. **Confirm and state the new zero-spawn test's coverage across both git and jj composition paths** (addresses: test-coverage major) — either loop the test over both kinds or state explicitly why one is sufficient.

8. **Align the new fixture binary with the established `tests/fixtures/` convention, or note why it diverges** (addresses: the three-lens minor finding).

## Per-Lens Results

### Architecture

**Summary**: The plan is a well-bounded, cleanly-sequenced convergence of two probe implementations onto one, correctly reasons about crate dependency direction and about which claims in the work item are stale versus still true, and takes care not to overreach into 0198's neighbouring subprocess path. Its main architectural weight sits in the resilience/containment decision it defers: collapsing the last isolation boundary between subprocess and in-process VCS reading onto a single unbounded implementation, mitigated only by a doc comment rather than any tracked trigger. A secondary, lower-stakes concern is a build-surface inconsistency between the new test-support binary's placement and the sibling crate's established convention for the equivalent artefact.

**Strengths**:
- Phase 2 correctly reasons about dependency direction — `vcs-adapters` cannot depend on `corpus-adapters`, so the existing `vcs-adapters-fixture` binary cannot be extended, and a new `corpus-adapters`-local binary is needed instead.
- Phase 4 declines to accept the work item's framing that the switch "first makes the server reachable" at face value; the Current State Analysis independently verifies the server's actual call graph.
- The four phases sequence every deletion after the thing that would break from it has already collapsed, avoiding a half-migrated intermediate state landing in the tree.
- The plan is disciplined about scope, confirming via file inspection which helpers are `CommandProbe`-exclusive before proposing their deletion.

**Findings**:
- 🟡 Major — Containment-bound decision has no tracked revisit trigger (Phase 1, item 2). See merged cross-cutting finding above.
- 🔵 Minor — Fixture binary placement diverges from sibling-crate convention (Phase 2, item 1). See merged finding above.
- 🔵 Suggestion — MPL check method is a byte-literal proxy sensitive to dependency wording changes (Phase 4).

### Code Quality

**Summary**: The plan itself is well-structured — phased so policy decisions precede behaviour changes and deletions are preceded by collapsing their last references, with TDD (red/green) called out explicitly for the zero-spawn extension. However, it has one significant gap that will break the build as scoped: Phase 3's `CommandProbe` deletion does not account for `tests/library.rs`. There is also an unexplained deviation from this codebase's consistent fixture-binary convention in Phase 2's new binary.

**Strengths**:
- The four phases are ordered so policy decisions land before the behaviour change that depends on them, and deletions are deferred until nothing else references what's being removed.
- Phase 2 explicitly requires proving the new zero-spawn test fails against the current `CommandProbe` wiring before flipping the composition root.
- The plan traced actual current line numbers and call sites rather than trusting the stale work item, and explicitly corrected a stale reference instead of propagating it.

**Findings**:
- 🔴 Critical — `CommandProbe` deletion omits `tests/library.rs`, which will fail to compile (Phase 3). See merged cross-cutting finding above.
- 🔵 Minor — New fixture binary deviates from the repo's established fixture-binary convention without explanation (Phase 2, item 1). See merged finding above.

### Test Coverage

**Summary**: The plan follows solid TDD discipline for its central deliverable (the failing-first zero-spawn test in Phase 2) and correctly reuses the crate's established fixture/stub test patterns. However, it has a significant blind spot around `tests/library.rs`, and it overstates existing coverage for the sha256 revision-folding policy being recorded in Phase 1. A secondary gap is that the new zero-spawn test's fixture scope may not exercise both the jj and git dispatch paths.

**Strengths**:
- Phase 2 explicitly requires running the new zero-spawn test red before flipping the composition root, and calls out why that step must not be skipped.
- The new zero-spawn test is designed to mirror the existing `zero_spawn.rs` test's dual-run pattern and two-part assertion, correctly reusing a proven anti-flakiness structure.
- The plan is explicit and specific about which existing suites must keep passing unchanged, and corrects the work item's stale reference to a test file that doesn't exist.

**Findings**:
- 🔴 Critical — `tests/library.rs`'s dependency on `CommandProbe`, including the sole regression test for the plan's documented behavioural divergence, is not addressed (Phase 3). See merged cross-cutting finding above.
- 🟡 Major — The sha256 revision-folding policy is documented as already covered by tests that don't actually exercise `revision()` (Phase 1, item 1).
- 🟡 Major — The new zero-spawn test's fixture scope may not exercise both the git and jj composition paths (Phase 2).
- 🔵 Minor — New fixture binary's feature-gating approach diverges from the crate's established pattern (Phase 2, item 1). See merged finding above.

### Correctness

**Summary**: The plan's core wiring change is logically sound and its claims about the snapshot-on-read side effect and dead-code-elimination reachability check out against the actual source. However, Phase 3's deletion of `CommandProbe` is scoped incompletely — it omits `tests/library.rs` — and there's an unaddressed semantic divergence between the old and new `RepoRoot` implementations' path canonicalisation.

**Strengths**:
- The plan traces the two actual production write paths and correctly verifies, against the real source, that neither reads `.revision`/`.repository_name` from the derived metadata.
- The plan correctly identifies that `scrub_environment`/`run_capped` are shared with 0198's still-active path and must survive, while `run_checked`/`wait_capped_checked` are `CommandProbe`-only and safe to delete.
- Sequencing the new zero-spawn test as failing-first before the composition-root flip is sound practice.

**Findings**:
- 🔴 Critical — `CommandProbe` deletion omits `tests/library.rs`, which still depends on it (Phase 3). See merged cross-cutting finding above.
- 🔵 Minor — `RepoRoot` canonicalisation semantics differ between `MarkerWalkRoot` and `InProcessProbe`, unaddressed by the plan (Phase 2).

### Security

**Summary**: The plan is careful to document the isolation-boundary tradeoff explicitly rather than deciding it silently, and correctly re-verifies rather than assumes the visualiser server's current reachability into the `gix`/`jj-lib` closure. However, its containment-bound decision treats the choice as binary without evaluating a cheap middle path for the specific call site being widened, and it deletes `CommandProbe`'s existing protections without a replacement or adversarial-input test proving the replacement degrades gracefully. The forward-looking risk is tracked for MPL-2.0 licensing but not for containment.

**Strengths**:
- Phase 1 records the containment-bound, sha256-handling, and snapshot-on-read decisions as auditable doc comments rather than leaving them implicit.
- Phase 4 empirically re-runs the MPL-2.0/reachability check against a real unstripped build rather than assuming either outcome.
- The plan preserves `scrub_environment`/`run_capped`, which remain load-bearing for 0198's still-active path.
- Phase 2's red/green sequencing gives genuine evidence the subprocess call is actually removed, rather than relying on code inspection alone.

**Findings**:
- 🔴 Critical — Containment-bound decision treats subprocess-isolation-vs-nothing as the only two options (Phase 1, item 2). See merged cross-cutting finding above.
- 🟡 Major — No adversarial-input test proves `InProcessProbe` fails gracefully rather than hanging or panicking (Phase 2).
- 🟡 Major — Forward-looking server reachability is tracked for licensing but not for containment (Phase 4). See merged cross-cutting finding above.
- 🔵 Minor — Environment-scrubbing asymmetry between `CommandProbe` and `InProcessProbe` is not addressed (Phase 1).

### Safety

**Summary**: The plan is careful and well-evidenced on the two deletions it foregrounds and correctly establishes that the hook path is unaffected, since it already runs `InProcessProbe` today. However, its safety analysis of the two behavioural changes it does introduce is narrower than it appears: the snapshot-on-read confirmation misses the skill layer that persists `Current Revision:` into frontmatter, and the containment-bound decision doesn't account for the lock-holding call site at `work create`.

**Strengths**:
- The plan independently verifies that the one Rust write path persisting frontmatter reads only `datetime_utc` and never `.revision`/`.repository_name`.
- The plan correctly identifies that `InProcessProbe` already runs unbounded on the hook path today, so this switch does not newly expose the hook path itself.
- `CommandProbe`'s `OriginRemote` implementation and its dedicated helpers are already unused in production, so Phase 3's deletion doesn't orphan a still-needed capability.
- The failing-first structure for the new zero-spawn test avoids a window where the guarantee is claimed but unproven.

**Findings**:
- 🔴 Major (elevated from the lens's own critical rating in cross-cutting synthesis; retained at major here to avoid double-counting the merged cross-cutting critical) — Snapshot-on-read loss is confirmed against too narrow a consumer set (Phase 1, item 3). See merged cross-cutting finding above.
- 🟡 Major — Dropping the timeout removes a hang guard specifically at a lock-holding call site (Phase 1, item 2). See merged finding above.
- 🔵 Minor — sha256-repository failure folds to a silent "no VCS here" with only debug-gated logging (Phase 1, item 1).

### Compatibility

**Summary**: The plan is careful about preserving the `vcs_adapters::facts` boundary contract and correctly scopes `CommandProbe`'s deletion to a workspace-internal, unpublished crate with no external consumers. However it has a concrete, compile-breaking gap in Phase 3's file inventory, and its Phase 4 MPL-2.0 re-verification is scoped too narrowly against the actual set of distributed sub-binaries that already reach the `gix`/`jj-lib` closure.

**Strengths**:
- `CommandProbe` is referenced nowhere outside `cli/vcs-adapters` itself, and the crate carries `publish = false`, so its deletion carries no external semver risk.
- The `vcs_adapters::facts` boundary contract is explicitly test-locked before and after the switch across all seven checkout-shape assertions.
- The red/green ordering in Phase 2 is a sound way to prove the compatibility guarantee is real rather than assumed.
- Recording the sha256, containment-bound, and snapshot-on-read decisions at the port-contract level is a good choice for forward compatibility.

**Findings**:
- 🔴 Critical — Phase 3's file inventory omits `tests/library.rs` (Phase 3). See merged cross-cutting finding above.
- 🔴 Critical — Phase 4 only re-verifies `accelerator-visualiser`, missing sub-binaries that already call `InProcessProbe` directly (Phase 4). See merged cross-cutting finding above.
- 🟡 Major — The snapshot-on-read consumer audit covers only Rust write paths, missing `SKILL.md` workflows (Phase 1, item 3). See merged finding above.
- 🔵 Minor — No automated guard ties `deny.toml`'s reachability rationale to the actual call graph (Phase 4).

## Re-Review (Pass 4) — 2026-08-10

**Verdict:** APPROVE

Three iterative fix-and-re-review rounds followed the initial REVISE verdict, each re-running the affected lenses against the updated plan. The pattern across all three: fixes for one round's findings were themselves source-verified and precise, but each round's edits introduced a small number of new, narrowly-scoped gaps of the same character (an incomplete file inventory, an orphaned import, a doc comment that outran what the surrounding code actually proved) — caught and closed by the next round. By pass 4, no lens found any new issue; the plan converges to a state every lens has independently checked against the live codebase.

### Previously Identified Issues

- 🔴 **Code Quality, Test Coverage, Correctness, Compatibility**: `tests/library.rs` omission from Phase 3 — Resolved (Phase 3 now reworks the file explicitly; further hardened across passes 2–3 to also delete `assert_parity` outright, rename tests describing the deleted comparison, and add a `git_revision_oracle` closing a git-side coverage gap the rework itself would otherwise have introduced)
- 🔴 **Security**: Containment-bound decision framed as binary (subprocess vs. nothing) — Resolved via the user's explicit "document only" scope decision; the doc comment now names the protection removed, the specific lock-holding blast radius, and a structural (not dead-ticket) revisit trigger
- 🔴 **Compatibility**: Phase 4 MPL-2.0 re-check scoped to the visualiser alone — Resolved; broadened across passes 2–3 to all six `DISPATCHED_SUBBINARIES` tokens after two intermediate omissions (`accelerator-migrate`, then a work/corpus attribution error) were each caught by independent lenses and corrected
- 🟡 Major findings (snapshot-on-read scope, lock-holding hang risk, sha256 test gap, zero-spawn git/jj coverage, adversarial-input test, containment revisit tracking) — all Resolved; see per-round detail below
- 🔵 Minor findings (fixture binary convention, symlink canonicalisation, env-scrubbing asymmetry, sha256 silent failure, deny.toml drift guard) — fixture binary convention Resolved; symlink canonicalisation documented (accepted as narrow, no new test infrastructure); env-scrubbing asymmetry, sha256 silent failure, and the deny.toml drift guard remain accepted open items, unchanged from the original review — none were part of the recommended-changes list and none blocked approval

### New Issues Introduced (across passes 2–3, all now Resolved)

- 🔴/🟡 `subprocess.rs`'s `CommandProbe`/`MarkerWalkRoot` deletion left dangling top-level imports, an orphaned `origin_repo()` test helper, and (caught a round later) a second, deeper-scoped orphaned import inside the test module itself (`PathBuf`, `CommandProbe` in a `use super::{...}` list) — fully enumerated and fixed by pass 3, independently confirmed by both correctness and test-coverage
- 🟡 `detection.rs`'s `facts_via` helper and its `RepoRoot`/`VcsProbe` imports were left orphaned by the "calls `vcs::facts` directly" phrasing — fixed
- 🟡 `lib.rs`'s crate-level module doc comment went stale after the repoint/deletion with no scheduled update — fixed, split correctly across Phase 2 (the sentence that goes false first) and Phase 3 (the sentence that goes false once `MarkerWalkRoot` is gone), verified to leave a truthful intermediate state at each independently-mergeable phase boundary
- 🟡 The containment-bound doc comment's revisit trigger named work item 0168, which is closed and — per the plan's own Current State Analysis — never performs the triggering action; replaced with a structural condition (any code under `cli/visualiser/server` calling into `facts`) that holds regardless of which future item does it
- 🔵 Several `tests/library.rs` doc/naming loose ends (stale section header, ambiguous `assert_parity` disposition, three test names still describing the deleted comparison, a retitled module doc comment that first overstated then under-differentiated its own test groups, a renamed test colliding with an existing name in `detection.rs`) — all fixed
- 🔵 The new fixture binary's described `required-features` gating contradicted the actual sibling convention it claimed to match — fixed
- 🔵 `run_capped`'s own doc comment carried a dangling intra-doc link to the deleted `CommandProbe::revision`, and two deletion ranges started one comment-block short of the doc comment actually preceding the deleted item — fixed

### Assessment

The plan is in good shape for implementation. Its core wiring change is sound, its policy decisions are recorded with the specific blast radii they carry (not just the abstract exposure), and its test-coverage claims are now backed by tests that actually exercise what they claim to. The remaining open items (env-scrubbing asymmetry, sha256 silent-failure UX, no automated guard on `deny.toml` drift) are legitimate but narrowly-scoped follow-up candidates, not blockers — none were part of any round's recommended-changes list, and each was independently re-confirmed as intentionally out of scope rather than overlooked.

---
*Review generated by /accelerator:review-plan*
