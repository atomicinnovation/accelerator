---
type: plan-review
id: "2026-06-21-0119-resume-safe-partial-migration-failure-review-1"
title: "Plan Review: Resume-Safe Partial Migration Failure"
date: "2026-06-21T08:35:52+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-06-21-0119-resume-safe-partial-migration-failure"
target: "plan:2026-06-21-0119-resume-safe-partial-migration-failure"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [correctness, safety, portability, test-coverage, architecture, code-quality]
review_number: 1
review_pass: 4
tags: [migrate, interactive-migration, manifest, tooling, plan-review]
last_updated: "2026-06-22T12:35:23+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Resume-Safe Partial Migration Failure

**Verdict:** REVISE

The plan is structurally strong — a clean three-phase decomposition, a single
enumeration helper as the source of truth, recording on both success and failure
paths to capture partial writes (AC1's hardest requirement), and an honest
Limitations section. But three independent lenses converged on the same defect:
the `migrations-run.id` sidecar — the plan's entire answer to the work item's
"fail closed when the manifest is from a different run (stale)" requirement — is
written but **never compared**, so staleness-by-identity is not actually
detected. Two further critical issues compound it: the baseline computation
silently drops a guarded resume's own prior partial output from the continuing
manifest (a resume-that-fails-again becomes unresumable), and the AC1 test model
(Test 4) uses a fake `.git` that cannot exercise the chosen diff-based recorder.
These are design-level corrections, not polish, hence REVISE.

### Cross-Cutting Themes

- **Run-id sidecar provides no staleness signal** (flagged by: correctness 🔴,
  safety 🟡, test-coverage 🟡) — `dirty_tree_fully_owned` only checks the run-id
  file is non-empty/readable (`[ -s ]`/`[ -r ]`); it never compares the recorded
  id to the current run, and there is no current run-id at pre-flight time (the
  id is minted *after* the pre-flight). Staleness protection therefore rests
  entirely on the separate truncate-on-fresh-start invariant, which has its own
  holes (below). AC4's "different run identity" clause is narrated, not
  implemented — and not tested.

- **The jj-vs-git untracked asymmetry survives the single enumeration source**
  (flagged by: portability 🟡, correctness 🟡, safety 🔵, architecture 🔵) — the
  plan claims one helper makes jj and git "behave consistently," but it only
  makes the *recording* and *owned-check* halves agree with each other; the
  underlying divergence (git's `grep -v '^??'` excludes untracked files, jj
  includes them) remains. Migrations routinely *create* files; under git those
  new paths are recorded by neither the manifest nor the dirty enumeration, so
  AC1's "exactly those paths" assertion and the AC2/AC3 guarantees are
  VCS-conditional. The existing `test-migrate-interactive.sh:199` has to `git add`
  a file precisely because `grep -v '^??'` would otherwise hide it.

- **New helpers reach into enclosing-scope globals** (flagged by: code-quality
  🟡/🔵, correctness 🟡, architecture 🔵) — `manifest_record_delta` reads an
  ambient `$vcs` instead of taking it as a parameter (inconsistent with
  `enumerate_scoped_dirty`, which is correctly parameterised), and the `RESUME`
  flag is initialised in Phase 3, set in Phase 3, but read by Phase 2's
  fresh-run guard. The `RESUME=0` default must be hoisted above the FORCE gate or
  the FORCE path risks an unbound-variable abort under `set -u`.

### Tradeoff Analysis

- **Path-only ownership vs. operational safety**: The plan accepts that ownership
  is by path, not content, so an operator who commits a failed run, edits the
  same paths, and re-runs can resume over those edits. The plan frames this as
  exotic; safety reads the commit-then-edit-same-paths path as plausible for an
  operator iterating on a failed migration. Recommendation: keep the limitation,
  but tie it to a *working* staleness gate (so a committed-and-resumed manifest
  cannot be re-honoured indefinitely) rather than leaving the run-id inert.

- **Runner-side diffing vs. migration self-reporting**: The chosen diffing
  approach correctly avoids a migration-author contract change and captures
  partial writes — a sound call. The cost (architecture 🔵): the manifest becomes
  a *derived view* of `enumerate_scoped_dirty`, inheriting every quirk (the
  untracked asymmetry, the scope filter, porcelain normalization). This is
  acceptable but should be stated: "ownership = the scoped-dirty delta," not
  "paths the migration wrote."

### Findings

#### Critical

- 🔴 **Correctness + Safety**: Run-id sidecar is never compared — a different
  run's manifest is not detectable as stale
  **Location**: Phase 3 §1 (`dirty_tree_fully_owned`); Phase 2 §2 (mint run-id)
  The work item's AC4 demands fail-closed when the manifest "carries a different
  run's identity (stale)." The plan adds `migrations-run.id` for this but only
  tests its existence/non-emptiness, never comparing it to the current run —
  which doesn't exist at pre-flight time anyway. The sidecar is dead weight; the
  "different run" case is never rejected. A manifest minted by run A can
  authorise a guarded resume in run B whenever its paths match the dirty tree.

- 🔴 **Correctness**: Baseline captures the prior partial output on resume, so it
  is dropped from the continuing manifest
  **Location**: Implementation Approach (baseline = enumerate_scoped_dirty);
  Phase 2 §2 (BASELINE_FILE capture on resume)
  On a guarded resume, `BASELINE_FILE` is captured unconditionally as the current
  dirty tree — which *is* the prior run's partial output. Since `delta =
  enumerate_scoped_dirty - baseline`, every prior partial path is excluded from
  all subsequent deltas. If the resumed run fails again mid-way, the manifest no
  longer covers the original partial paths (still dirty), so the next re-run
  fail-closes and refuses — trapping the operator back at FORCE-only, the exact
  situation 0119 exists to prevent. Fix: seed the baseline empty (or from the
  existing manifest) when `RESUME=1`.

- 🔴 **Test Coverage**: AC1 stub model (Test 4) uses a fake `.git` and cannot
  exercise the diff-based recorder
  **Location**: Phase 2 Success Criteria / Testing Strategy (AC1)
  Test 4 (`test-migrate.sh:138-154`) sets up its repo with `mkdir -p
  "$REPO/.git"` — a fake directory, not a real repo. The chosen recorder runs
  `git status --porcelain`/`jj diff`, which yield nothing against a fake `.git`,
  so the manifest stays empty and the AC1 `grep -cFx` assertion fails (or gets
  silently rewritten to seed the manifest directly, verifying nothing). The AC1
  test must use a real initialised repo (Test 14's `git init -q` + commit, or the
  jj equivalent).

#### Major

- 🟡 **Safety**: Stale manifest survives across refusing runs — truncation only
  fires after the pre-flight passes
  **Location**: Phase 2 §2; Implementation Approach ("Fail-closed staleness")
  The truncation that resets a leftover manifest is placed *after* the pre-flight,
  but the pre-flight `exit 1`s on refusal before reaching it. A manifest from a
  prior run that is never cleanly re-run (only resumed or refused) is never
  truncated, so it can later authorise a resume over coincidentally-matching
  dirty paths. The "a prior run's manifest cannot survive a clean start" claim is
  true only for the clean-start path, not the refuse/resume paths.

- 🟡 **Safety**: Fail-closed return codes risk colliding with `set -euo pipefail`
  **Location**: Phase 3 §1 (`dirty_tree_fully_owned` return paths)
  The runner runs under `set -euo pipefail`. The fail-closed design depends on
  helpers returning 1 and callers treating that as "refuse." Any unguarded
  non-zero from an intermediate command (a `grep` no-match outside a conditional,
  a `mktemp`/`enumerate_scoped_dirty` failure) could abort the script *before* the
  refusal message prints — turning an intended refusal into a silent abort and
  breaking the AC3/AC4 observable-refusal criteria. The plan should state the
  `set -e` contract per helper and assert the refusal *message* (not just exit
  code) in every branch.

- 🟡 **Correctness**: `RESUME` flag read in Phase 2 but set in Phase 3 inside the
  FORCE-guarded block; unbound risk under `set -u`
  **Location**: Phase 2 §2 (RESUME guard); Phase 3 §2 (RESUME assignment)
  `RESUME=0` must be initialised unconditionally at the top of the script
  (alongside the hoisted `vcs` detection), not "before the pre-flight" (which
  starts at the FORCE gate). On the FORCE path the entire pre-flight is skipped,
  so an in-block initialisation leaves `RESUME` unset; `${RESUME:-0}` saves it
  only by luck and a future edit dropping the default breaks the FORCE path.

- 🟡 **Correctness**: git rename (`R old -> new`) and quoted/special-char paths
  break string-equality ownership
  **Location**: Phase 1 §1 (normalization); Phase 3 §1 (`grep -Fxq`)
  The git branch does `git status --porcelain | sed 's/^[A-Z?][[:space:]]*//'`.
  Renames emit `R  old -> new` (one line, both paths) and special-char paths are
  C-escaped/quoted. The sed only strips the status code, so a rename yields a
  bogus `old -> new` token that never string-matches a recorded path — and
  rename-heavy migrations (e.g. the 0003 relocate that `mv`s files) are exactly
  the ones that motivate resume. Use `git status --porcelain -z` (or `git diff
  --name-only`) for NUL-separated, unquoted, single-path entries.

- 🟡 **Portability + Correctness**: git-excludes-untracked vs jj-includes-untracked
  asymmetry survives the single enumeration source
  **Location**: Phase 1 §1 (`enumerate_scoped_dirty`); Phase 3 Manual
  Verification (git bullet)
  See the cross-cutting theme. Under git, migration-*created* files are recorded
  by neither half; a `mv` of a tracked file records only the `D` deletion, not the
  new `??` path, so AC1's manifest is wrong under git while correct under jj.
  Either include untracked in the git enumeration
  (`--untracked-files=all`, normalising both `??` and status prefixes) so git
  matches jj, or explicitly scope the ACs to jj and document the divergence.

- 🟡 **Test Coverage**: AC4 omits the genuinely-stale (different run-id) manifest
  case the work item requires
  **Location**: Phase 3 Success Criteria / Testing Strategy (AC4)
  AC4 parameterises over {manifest absent, empty, run-id absent, run-id empty} —
  all caught by the `[ -s ]` guards — but never a *populated* manifest paired with
  a run-id carrying a *different* identity, which is the one case the identity
  gate exists for. (This intersects the run-id-never-compared critical: the test
  would expose that the gate does nothing.)

- 🟡 **Test Coverage**: jj code path is only manually verified; automated tests
  are not parameterised over both VCSes
  **Location**: Testing Strategy / Manual Testing Steps
  The automated AC2-AC4 tests use the git-based Test 14 model; the suite has zero
  jj coverage. jj is the project's primary VCS and the higher-risk branch
  (untracked semantics differ). Parameterise the AC2-AC4 tests over both git and
  jj (guarded by `command -v jj`), or add at least one jj-backed guarded-resume
  test — the single-enumeration-source claim is a testable hypothesis.

- 🟡 **Test Coverage**: Phase 1's byte-identical-refusal claim is only manually
  verified
  **Location**: Phase 1 Success Criteria (Manual Verification)
  Phase 1 deliberately changes git's emitted output (strips porcelain prefixes),
  yet its only guard that refusal/session-log stderr stays byte-identical is a
  manual diff. Existing Test 14 asserts only the loose substring `dirty`. Add an
  automated assertion (golden-string or several `assert_stderr_contains` on
  distinctive lines) so the claim is enforced by CI.

- 🟡 **Architecture**: Phase 2 is not behaviour-neutral — it hoists VCS detection
  and writes self-enumerating state
  **Location**: Phase 2 Overview ("mergeable on its own, harmless")
  The manifest/run-id files live under `.accelerator/state/`, which the dirty
  enumeration matches (`^\.accelerator/`). Without Phase 3's implicit-ownership
  case, a Phase-2-only partial failure leaves those files as dirty paths that the
  *unmodified* pre-flight refuses over — with no resume affordance. Either land
  the bookkeeping-file recognition in Phase 2, or restate the mergeability claim
  as "safe only when followed by Phase 3."

- 🟡 **Architecture**: Two resume axes share one manifest with under-specified
  interaction
  **Location**: Phase 2 §3 (interactive recording); Phase 3 §2
  Phase 2 records deltas after *interactive* migrations too, so an interactive
  partial failure writes the session-log path into the manifest. A mixed
  interactive+mechanical run (interactive succeeds, later mechanical fails) leaves
  a manifest containing the session-log path; on re-run the 0069 session-log
  branch fires first and blocks even though the run is otherwise fully owned. The
  boundary between the 0069 and 0119 axes is defined only by source ordering.
  State the invariant that the manifest excludes session-log paths, and document
  the mixed-run case.

- 🟡 **Code Quality**: `BASELINE_FILE` temp file leaked on every `exit 1` failure
  path
  **Location**: Phase 2 §2 (capture) & §4 (cleanup)
  `BASELINE_FILE=$(mktemp)` is removed only on full success. Every failure path —
  mechanical (`:304`), interactive (`:291`), the new `manifest_record_delta`
  failure branch — exits without `rm -f`, so the leak fires on the *exact*
  partial-failure path the feature targets. Install an `EXIT` trap (mirroring
  `atomic-common.sh:28`/`:202`) covering `BASELINE_FILE` and the existing
  `STDOUT_FILE`.

- 🟡 **Code Quality**: Dirty-tree refusal message duplicated verbatim, will drift
  **Location**: Phase 3 §2 (`else` branch) vs `run-migrations.sh:162-166`
  Phase 3's `else` re-emits the refusal message word-for-word, leaving two copies.
  Tests assert only the `ACCELERATOR_MIGRATE_FORCE` substring, so drift passes
  undetected. Factor a single `refuse_dirty_tree()` helper, or restructure so the
  not-owned case falls through to the existing `:162-166` emission (deleting the
  duplicated `else` body).

- 🟡 **Code Quality**: `manifest_record_delta` reads global `$vcs` instead of
  taking it as a parameter
  **Location**: Phase 2 §1
  Inconsistent with the adjacent `enumerate_scoped_dirty <vcs>`, which is
  correctly parameterised. Reading an ambient global makes the helper
  non-self-contained and risks a `set -u` fatal far from the cause. Give it a
  `<vcs>` parameter.

#### Minor

- 🔵 **Safety**: Bookkeeping allowlist could whitelist a path a migration
  legitimately wrote
  **Location**: Phase 3 §1 (case allowlist)
  The four-path `case` carve-out trusts those `.accelerator/state/` paths
  unconditionally. A future state-rewriting migration (cf. 0003 relocating state)
  would have its genuine output waved through, and a foreign edit to those exact
  files too. Prefer recording bookkeeping files into the manifest like any other
  write, or assert the invariant "no migration mutates these files."

- 🔵 **Safety**: Manifest deletion on success is non-atomic relative to the
  mutations it tracks
  **Location**: Phase 2 §4
  An interrupt (Ctrl-C/SIGKILL) between the final migration's success and the `rm`
  leaves a stale manifest describing a fully-applied run; the next dirty re-run
  offers a misleading resume affordance (the ledger prevents re-application, so
  benign). Note the window in Limitations or guard the resume on pending-set
  non-emptiness.

- 🔵 **Correctness**: Interactive session-log path is recorded but the
  session-log branch exits before guarded resume sees it
  **Location**: Phase 2 §3
  Dead recording work, and a maintainer-misleading manifest that lists paths the
  guarded resume can never act on. Skip manifest recording on the interactive
  path, or document that interactive entries are never consumed.

- 🔵 **Correctness**: Manifest deletion also fires on the "no pending migrations"
  early-exit gap
  **Location**: Phase 2 §4
  The early `exit 0` for "no pending migrations" happens *before* the cleanup, so
  a stale manifest is not cleaned when nothing is pending — compounding the
  run-id-never-compared finding. Move cleanup to also run on the no-pending path.

- 🔵 **Correctness**: TOCTOU — the dirty set is enumerated three times at resume
  **Location**: Phase 3 §1/§2
  `dirty=`, the affordance loop, and inside `dirty_tree_fully_owned`. A tree
  mutation between reads could make the listed paths disagree with the validated
  ones. Low real-world risk (single operator), but pass the captured `$dirty`
  into both the listing and the ownership check; don't re-enumerate inside the
  helper.

- 🔵 **Architecture**: Manifest correctness is coupled to the dirty-enumeration
  definition by design
  **Location**: Key Discoveries / Implementation Approach
  Ownership is "the scoped-dirty delta," not "paths the migration wrote." A future
  scope change to the enumeration changes resume correctness as a side effect.
  State this explicitly.

- 🔵 **Architecture**: Hoisting VCS detection widens its scope; helpers now read
  an ambient `$vcs`
  **Location**: Phase 2 §2 (see also Code Quality `$vcs` finding)

- 🔵 **Architecture**: Phase 3-without-Phase-2 identity claim leaves a dangling
  `RESUME` contract; phases are sequenced, not commutative
  **Location**: Phase 3 Overview
  State the intended merge order (1 → 2 → 3).

- 🔵 **Architecture / Code Quality**: Bookkeeping allowlist hard-codes path
  strings that already exist as variables
  **Location**: Phase 3 §1
  `case` arms duplicate `$STATE_FILE`/`$SKIP_FILE`/`$RUN_PATHS_FILE`/`$RUN_ID_FILE`
  as repo-relative literals, kept in sync by hand. Derive from the `*_FILE`
  variables (strip `$PROJECT_ROOT/` as `0003:120` does).

- 🔵 **Code Quality**: `RESUME` global is implicit cross-phase coupling with no
  single owner
  **Location**: Phase 2 §2 / Phase 3 §2 (see also the Correctness major)

- 🔵 **Code Quality**: Unguarded `mktemp` / `atomic_write` failures lack a clear
  diagnostic
  **Location**: Phase 2 §2
  Guard `mktemp` with an explicit error message matching the runner's
  `[id] failed`-style diagnostics.

- 🔵 **Code Quality**: Resume-affordance marker embedded inline, asserted by
  substring — risks test/message drift
  **Location**: Phase 3 §2
  Add a comment at the echo site marking the test-asserted substring, or hoist to
  a named constant shared with the test.

- 🔵 **Test Coverage**: No test for a guarded resume that itself fails again
  **Location**: Phase 3 / Implementation Approach (RESUME)
  Realistic operator path (fix one cause, hit the next). Intersects the
  baseline-drops-prior-output critical. Add a test: partial failure → resume →
  second failure → assert the manifest unions both failures' paths and a third
  re-run still resumes.

- 🔵 **Test Coverage**: No test pins the "runner bookkeeping files are implicitly
  owned" carve-out
  **Location**: Phase 3 §1
  Assert a dirty tree of owned paths + the bookkeeping sidecars resumes, and that
  a *foreign* `.accelerator/state/` path (not in the carve-out) still refuses
  (exact-path, not prefix, match).

- 🔵 **Test Coverage**: AC1 asserts content but not dedup/ordering across two
  recording points
  **Location**: Testing Strategy (AC1)
  Extend the fixture so one path is mutated across two recording points
  (success + failure) to lock in `atomic_append_unique`'s idempotency.

#### Suggestions

- 🔵 **Portability**: New helper hardcodes `&>` while the rest of the repo uses
  `>/dev/null 2>&1`
  **Location**: Phase 1 §1
  `command -v jj &>/dev/null` is bash-3.2-safe and linter-clean, but every other
  `command -v jj` in the repo (vcs-common.sh, the 0007/0004 migrations) uses the
  POSIX form. Use `>/dev/null 2>&1` in the new helper for consistency — free, no
  behaviour change.

- 🔵 **Code Quality**: Per-path `atomic_append_unique` rewrites the whole manifest
  O(n²) per migration
  **Location**: Performance Considerations
  Negligible for realistic batches (the plan's YAGNI rationale is correct); noted
  only so a future optimiser knows the ledger-append and bulk-recording needs are
  conflated. If revisited: accumulate + single `sort -u` + `atomic_write`.

### Strengths

- ✅ Recording the delta on **both** the success and failure branches before any
  `exit 1` correctly captures a mid-failing migration's partial writes — AC1's
  hardest requirement, sound across correctness and safety.
- ✅ A single `enumerate_scoped_dirty` helper as the one definition of the scoped
  dirty set: recorded paths string-match resume-time owned-checks, and the
  jj/git asymmetry has exactly one home — strong cohesion and the right seam.
- ✅ The recording-mechanism tradeoff (runner-side diffing) is explicitly chosen
  and justified: it avoids a migration-author contract change (open-closed for
  the corpus) and makes the failure path itself a recording point.
- ✅ Phasing is safety-aware in intent: Phase 2 (write-only) and Phase 3 (read)
  are each meant to be inert alone, so a partial rollout cannot relax the guard.
- ✅ Replacing the all-or-nothing `ACCELERATOR_MIGRATE_FORCE=1` escape with a
  path-scoped guarded resume is a genuine reduction in blast radius for the
  operator.
- ✅ Accepted limitations (path-only ownership, no transaction boundary, no
  complete-but-uncommitted resume) are documented honestly rather than hidden,
  with VCS revert correctly identified as the backstop.
- ✅ Fail-closed staleness is modelled on an established precedent
  (`launcher-helpers.sh:157`), and the guard-clause idiom matches the surrounding
  code — the *intent* is right even though the run-id mechanism needs teeth.

### Recommended Changes

1. **Make staleness real, or drop the pretence** (addresses: run-id never
   compared; stale manifest survives refusing runs; AC4 omits stale case;
   path-only ownership tradeoff). Either (a) stamp the run-id into the manifest
   and bind ownership to a freshly-derivable identity so the AC4 "different run"
   clause is actually enforced *and* testable, or (b) remove the `migrations-run.id`
   sidecar entirely and narrow AC4 in the plan to the cases truncation genuinely
   covers — but do not keep an inert sidecar that claims a guarantee it doesn't
   deliver. Then add the missing AC4 test case (populated manifest + foreign id).

2. **Fix the resume baseline** (addresses: baseline drops prior partial output;
   no test for resume-that-fails-again). When `RESUME=1`, seed the baseline empty
   (or from the existing manifest) so the prior partial paths stay owned across
   successive failures. Add the resume → second-failure → third-re-run test.

3. **Specify a real repo for the AC1 test** (addresses: Test 4 fake-`.git`).
   State in Phase 2's success criteria that the AC1 (and recording) tests use an
   initialised git/jj repo (Test 14 model), not Test 4's fake `.git`.

4. **Resolve the jj/git untracked divergence** (addresses: asymmetry survives;
   git rename breaks ownership; jj only manually tested). Include untracked files
   in the git enumeration (and switch to `--porcelain -z` / `git diff
   --name-only` so renames and special-char paths normalise correctly), making
   git match jj — or explicitly scope the ACs to jj and document the divergence.
   Parameterise the automated tests over both VCSes.

5. **Harden the shell mechanics** (addresses: BASELINE_FILE leak; set -e collision;
   RESUME unbound; refusal-message duplication; `$vcs` global; mktemp diagnostics).
   Add an `EXIT` trap for the temp files; initialise `RESUME=0` above the FORCE
   gate; factor a single `refuse_dirty_tree()` helper instead of duplicating the
   message; parameterise `manifest_record_delta` with `<vcs>`; guard `mktemp`; and
   state the `set -e` contract for each new helper, asserting the refusal *message*
   (not just exit code) in the AC3/AC4 tests.

6. **Tighten the phase/axis contracts** (addresses: Phase 2 not behaviour-neutral;
   two resume axes; merge-order). Land bookkeeping-file recognition in Phase 2 (or
   restate its mergeability), state that the manifest excludes session-log paths
   and document the mixed interactive+mechanical run, and declare the intended
   merge order (1 → 2 → 3).

## Per-Lens Results

### Correctness

**Summary**: Logically careful about baseline/delta recording (the failure path
is a recording point, so AC1's partial-write capture is sound) and the
fail-closed existence checks are reasonable. But the staleness story is
incorrect: the run-id sidecar is minted but never compared, so a manifest with a
different run's identity is honoured whenever its paths match. Two real
state-transition bugs: the baseline silently drops a guarded resume's own prior
partial output, and a resume that fails again can corrupt ownership accounting.

**Strengths**:
- Delta recorded on both success and failure before any `exit 1` (AC1's hardest part).
- Single enumeration source makes recorded paths string-match dirty paths.
- Bookkeeping allowlist correctly anticipates the runner's own state files.
- Phases have behaviour-preserving intermediate states (in intent).

**Findings**:
- 🔴 high — Run-id sidecar never compared; different-run manifest not detectable as stale (Phase 3 §1 / Phase 2 §2).
- 🔴 high — Baseline captures prior partial output on resume → dropped from continuing manifest; resume-that-fails-again becomes unresumable (Implementation Approach / Phase 2 §2).
- 🟡 high — `RESUME` read in Phase 2 but set in Phase 3 inside the FORCE block; unbound risk under `set -u` (Phase 2 §2).
- 🟡 medium — git rename (`R old -> new`) and quoted/special-char paths break string-equality ownership (Phase 3 §1 / Phase 1 §1).
- 🟡 medium — jj-vs-git untracked asymmetry makes the owned-check VCS-dependent (Phase 3 §1).
- 🔵 medium — Interactive session-log path recorded but session-log branch exits before guarded resume sees it (Phase 2 §3).
- 🔵 medium — Empty-dirty here-string iteration / triple re-enumeration TOCTOU (Phase 3 §1).
- 🔵 high — Manifest deletion not run on the "no pending migrations" early-exit (Phase 2 §4).

### Safety

**Summary**: The core intent — narrowing the all-or-nothing FORCE escape to a
path-scoped guarded resume — is itself a net safety improvement, and the
fail-closed posture is the stated goal. But the staleness mechanism doesn't match
the work item: the run-id sidecar is written but never compared, so "different
run" staleness is detected only indirectly via a fresh-start truncation invariant
with at least two holes. Combined with path-only ownership, there are narrow but
real windows where the resume proceeds over paths it doesn't own; recovery still
depends entirely on VCS revert against a corpus with no transaction boundary.

**Strengths**:
- Replacing all-or-nothing FORCE with a path-scoped resume genuinely reduces blast radius.
- Recording on the failure path captures partial writes (the AC1 case that matters for safety).
- Phasing is safety-aware: each phase inert alone, so partial rollout can't relax the guard (in intent).
- Default-in-absence-of-manifest is the existing refusal; FORCE deliberately mints a fresh identity.
- Accepted limitations documented; VCS revert correctly identified as the backstop.

**Findings**:
- 🟡 high — Run-id sidecar never compared; "different-run staleness" not detected (Phase 3 §1).
- 🟡 medium — Stale manifest survives across refusing runs; truncation fires only after the pre-flight passes (Phase 2 §2).
- 🟡 medium — Fail-closed return codes risk colliding with `set -e`/`pipefail`; a bare abort could skip the refusal message (Phase 3 §1).
- 🔵 medium — Bookkeeping allowlist could whitelist a path a migration legitimately wrote (Phase 3 §1).
- 🔵 high — Manifest deletion on success non-atomic vs the mutations it tracks (interrupt window) (Phase 2 §4).
- 🔵 medium — jj-includes-untracked vs git-excludes-untracked makes resume behave differently per VCS (Phase 1/3).

### Portability

**Summary**: The plan reuses established bash-3.2-safe idioms (here-strings,
process substitution, `mktemp` without template, `date -u +…Z`) that are all
permitted by the bashisms linter and portable across macOS/Linux, so the
shell-construct surface is clean. The one genuine concern is environmental, not
syntactic: the "single enumeration source makes jj and git consistent" claim is
only half-true — it unifies the two internal uses but the git-excludes-untracked
vs jj-includes-untracked asymmetry remains, so observable behaviour still diverges
by VCS for migration-created files.

**Strengths**:
- `command -v jj &>/dev/null` is bash-3.2-safe and linter-clean (linter only bans `&>>`).
- Here-strings and process substitution are bash-3.2-safe and already used by the runner.
- `date -u +%Y-%m-%dT%H:%M:%SZ` and template-less `mktemp` are BSD/GNU portable and match existing usage.
- Set membership via `grep -Fxq` against a file, honouring the bash-3.2 floor.

**Findings**:
- 🟡 high — git-excludes-untracked vs jj-includes-untracked asymmetry survives the single source; AC1 manifest wrong under git for created/renamed files (Phase 1 §1 / Phase 3 Manual Verification).
- 🔵 high — `BASELINE_FILE` temp leaks on the mid-run failure exit path; more visible on macOS `$TMPDIR` (Phase 2 §2/§4).
- 🔵 medium (suggestion) — New helper hardcodes `&>` against the repo's prevailing `>/dev/null 2>&1` convention (Phase 1 §1).

### Test Coverage

**Summary**: Each of the four ACs maps to a concrete asserting test, and lifecycle
tests are folded in — solid. But the AC1 stub model (Test 4) uses a fake `.git`
incompatible with the diff-based recorder, so it would not exercise it. The most
serious gap is AC4: the work item requires a genuinely-stale (different-run)
manifest case the plan omits, and the entire jj path is asserted only manually
despite jj being the primary VCS.

**Strengths**:
- Each AC maps to a named test with specific assertions, not narration.
- Refusal-message assertions are string-accurate (`own partial migration output`, `ACCELERATOR_MIGRATE_FORCE`).
- Lifecycle behaviours (delete-on-success, truncate-on-fresh-start) explicitly tested.
- AC1 recording-on-failure exercised by a write-then-`exit 1` stub.

**Findings**:
- 🔴 high — AC1 stub model (Test 4) uses a fake `.git` and cannot exercise the diff-based recorder (Phase 2 / Testing Strategy).
- 🟡 high — AC4 omits the genuinely-stale (different run-id) manifest case (Phase 3 / Testing Strategy).
- 🟡 high — jj code path only manually verified; automated tests not parameterised over both VCSes (Testing Strategy / Manual Steps).
- 🟡 medium — Phase 1's byte-identical-refusal claim only manually verified (Phase 1 Success Criteria).
- 🔵 medium — No test for a guarded resume that itself fails again (Phase 3).
- 🔵 medium — No test pins the bookkeeping-files-implicitly-owned carve-out (Phase 3 §1).
- 🔵 low — AC1 asserts content but not dedup/ordering across two recording points (Testing Strategy).

### Architecture

**Summary**: Structurally sound and well-anchored: a single enumeration helper as
source of truth, cohesive recording/reading halves, and explicit tradeoffs. The
central risk is that runner-side diffing couples manifest correctness to the
dirty-enumeration definition (defensible, but makes the manifest a derived view
of the VCS asymmetry). The concrete concerns are the "independently mergeable"
claim for Phase 2 (it hoists VCS detection and writes self-enumerating state, so
it isn't behaviour-neutral) and the under-specified interplay between the new
run-identity axis and the existing 0069 session-log axis.

**Strengths**:
- Single enumeration helper as the one definition — strong cohesion, clean seam.
- Recording-mechanism tradeoff explicitly chosen and justified (open-closed for the corpus).
- Fail-closed staleness modelled on an established precedent.
- Functional boundary respected: relaxes the guard only for provably self-produced paths.
- Correctly identifies runner-managed bookkeeping files as implicitly owned.

**Findings**:
- 🟡 high — Phase 2 not behaviour-neutral: hoists VCS detection and writes self-enumerating state (Phase 2 Overview).
- 🟡 medium — Two resume axes share one manifest with under-specified interaction (Phase 2 §3 / Phase 3 §2).
- 🔵 high — Manifest correctness coupled to the dirty-enumeration definition by design (Key Discoveries).
- 🔵 medium — Hoisting VCS detection widens its scope; helpers read an ambient `$vcs` (Phase 2 §2).
- 🔵 medium — Phase 3-without-Phase-2 leaves a dangling `RESUME` contract; phases are sequenced, not commutative (Phase 3 Overview).
- 🔵 low — Implicit-ownership bookkeeping list hard-coded rather than derived (Phase 3 §1).

### Code Quality

**Summary**: Well-structured into three mergeable phases with a clean
single-source helper and a fail-closed gate mirroring precedent. But several
maintainability concerns weaken it: new helpers reach into enclosing-scope globals
(`vcs`, `RESUME`) instead of taking parameters; `BASELINE_FILE` is leaked on every
`exit 1` path; the bookkeeping allowlist re-hardcodes path strings that already
exist as variables; and the refusal message is duplicated where it can drift. None
are correctness blockers but each is a real future-maintenance hazard.

**Strengths**:
- Phase 1 cleanly extracts a single-source enumeration helper — strong DRY/cohesion.
- Three-phase decomposition keeps each diff small and reviewable.
- Fail-closed gate modelled on an existing precedent; guard-clause style matches the idiom.
- Limitations / "What We're NOT Doing" honestly record accepted shortcuts.
- `enumerate_scoped_dirty` correctly takes `vcs` as a parameter (the pattern others should follow).

**Findings**:
- 🟡 high — `BASELINE_FILE` temp file leaked on every `exit 1` failure path (Phase 2 §2/§4).
- 🟡 high — Dirty-tree refusal message duplicated verbatim, will drift (Phase 3 §2 vs `:162-166`).
- 🟡 high — `manifest_record_delta` reads global `$vcs` instead of a parameter (Phase 2 §1).
- 🔵 high — Bookkeeping allowlist hard-codes four path strings that already exist as variables (Phase 3 §1).
- 🔵 medium — `RESUME` global is implicit cross-phase coupling with no single owner (Phase 2 §2 / Phase 3 §2).
- 🔵 medium — Unguarded `mktemp`/`atomic_write` failures lack a clear diagnostic (Phase 2 §2).
- 🔵 medium — Resume-affordance marker embedded inline, asserted by substring — test/message drift risk (Phase 3 §2).
- 🔵 low — Per-path `atomic_append_unique` rewrites the whole manifest O(n²) per migration (Performance Considerations).

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-06-22

**Verdict:** COMMENT

The revision resolved **all three criticals** and the substantive majors from
Pass 1. The six lenses re-ran against the edited plan; the only high-severity new
finding (the `BASELINE_FILE` EXIT trap) has since been fixed in this pass by
switching to explicit inline cleanup. What remains is one scope decision (the
mixed interactive+mechanical run) and a handful of minors the implementer should
heed. The plan is implementable.

### Previously Identified Issues

- 🔴 **Correctness/Safety**: Run-id never compared (staleness undetectable) —
  **Resolved**. Base-revision gate (`current_base_revision`, jj `change_id`/git
  `HEAD`) now compares recorded vs current and fails closed on mismatch; safety
  confirmed it closes the "different run honoured" hole.
- 🔴 **Correctness**: Baseline drops prior partial output on resume —
  **Resolved**. Empty-baseline-on-resume makes the manifest self-healing;
  correctness confirmed the reasoning and the new resume-fails-again test locks it.
- 🔴 **Test Coverage**: AC1 stub fake `.git` can't exercise the recorder —
  **Resolved**. Real initialised repo (Test 14 model) mandated.
- 🟡 **Safety**: Stale manifest survives refusing runs — **Resolved** (revision
  gate handles the committed-since case; residual documented).
- 🟡 **Correctness**: `RESUME` unbound under `set -u` — **Resolved** (hoisted).
- 🟡 **Safety**: `set -e` collision could skip refusal — **Resolved** (contract
  stated; AC4 asserts the message text, not just exit code).
- 🟡 **Correctness**: git rename breaks ownership — **Resolved** (`s/^.* -> //`),
  with a residual minor on greedy `.* -> ` matching (see new issues).
- 🟡 **Portability/Correctness**: jj/git untracked asymmetry — **Resolved**
  (reframed as benign — same exclusion governs recorder and guard under git — and
  documented; portability confirmed).
- 🟡 **Test Coverage**: AC4 omits stale case / jj only manual / Phase 1 manual —
  **Resolved** (stale-revision case added; both-VCS parameterisation; Phase 1
  assertion now names the specific lines).
- 🟡 **Architecture**: Phase 2 not behaviour-neutral — **Resolved** (fixed merge
  order 1→2→3 declared; carve-out placement noted, see new issues).
- 🟡 **Code Quality**: refusal message duplicated / `manifest_record_delta` global
  `vcs` / bookkeeping literals — **Resolved** (`refuse_dirty_tree()` helper;
  `<vcs>` parameter; derived from path variables).
- 🟡 **Code Quality/Portability**: `BASELINE_FILE` leak — **Resolved this pass**.
  The Pass-1 edit added an `EXIT` trap; the re-review found `atomic_write` does
  `trap - EXIT` (atomic-common.sh:31). Verified: that `trap -` is subshell-local
  (atomic_write is always piped, atomic-common.sh:55-60), so the trap *happened*
  to survive — but the correctness depended on that invariant. **Now switched to
  explicit inline `rm -f "$BASELINE_FILE"`** at every exit (mid-loop fail,
  interactive fail, success, and the no-pending early-exit needs none as the
  baseline isn't yet created), matching the existing `STDOUT_FILE` convention and
  removing the fragility entirely.

### New Issues Introduced (by the Pass-1 edits)

- 🔴→resolved **Correctness/Portability/Code Quality**: `BASELINE_FILE` EXIT trap
  vs `atomic_write`'s `trap - EXIT` — see above; two lenses rated it critical on
  the (incorrect) assumption the trap was cleared, one lens correctly noted the
  subshell nuance. Net: fragility, now removed via explicit cleanup. (Real-world
  impact was a temp-file leak, never a guard regression.)
- 🟡 **Architecture**: Mixed interactive+mechanical run is unresumable **by
  construction** — the session-log branch runs first and `exit 1`s whenever a
  session log is dirty, so guarded resume is unreachable for any batch mixing the
  two. Documented as a known boundary, but flagged as broader than an edge.
  **Open scope decision** (see Recommended Changes) — not auto-resolved.
- 🔵 **Correctness**: greedy `s/^.* -> //` could mis-fire on a path legitimately
  containing ` -> ` (vanishingly rare in the kebab-case corpus; fail-closed
  direction). Documented edge; gate on the `R`/`C` status letter if desired.
- 🔵 **Architecture**: the bookkeeping carve-out still lives in Phase 3 but is what
  makes Phase 2's own output non-blocking — the plan offers moving it into Phase 2
  conditionally rather than deciding. Acceptable given the fixed merge order.
- 🔵 **Code Quality**: resume-affordance marker is a literal coupled to a substring
  assertion (drift risk); `current_base_revision`'s `|| true` swallows VCS-tool
  failures (now mitigated by the empty-at-mint note); the four `rel_*` locals are a
  mild data-clump. All optional.

### Assessment

In good shape. No unresolved criticals; the lone remaining major is a deliberate
scope question (mixed interactive+mechanical resumability) for you / epic 0115 to
settle, not a plan defect. The minors are implementer-time refinements. Recommend
deciding the mixed-run scope item, then proceeding to implementation.

## Re-Review (Pass 3) — 2026-06-22 — Phase 4 only

**Verdict:** REVISE

The mixed-run scope question was resolved by adding **Phase 4** (reconcile the
interactive session-log axis). A focused re-review (correctness, safety,
architecture, test-coverage) of Phase 4 found a **verified critical** and several
majors — and multiple lenses independently questioned whether Phase 4 belongs in
0119 versus a follow-up. Phases 1–3 remain in their Pass-2 state (COMMENT-grade).

### Critical (Phase 4)

- 🔴 **Correctness (high)**: **Pure in-flight interactive resume is unreachable.**
  `dirty_tree_fully_owned` fail-closes on `[ -s "$RUN_PATHS_FILE" ]` (non-empty
  manifest), but an interactive migration interrupted before any *mechanical*
  migration wrote a delta leaves the manifest **empty** (Phase 2 truncates it; the
  interactive path is deliberately not recorded). So Phase 4's headline case
  (Success Criteria test 2) refuses instead of resuming — the glob arm is never
  even consulted. **Fix:** gate manifest *usability* on `[ -r ]` (exists), not
  `[ -s ]` (non-empty), and let the per-path loop be the sole ownership authority
  (an empty manifest + a dirty mechanical path still refuses, because the
  mechanical path isn't in the empty manifest; an empty manifest + only owned
  session-log/bookkeeping paths now correctly resumes). AC4's "manifest empty →
  refuse" wording must narrow to "empty manifest + a mechanical dirty path".

### Major (Phase 4)

- 🟡 **Correctness/Architecture**: **A failed interactive migration leaves
  sibling `.accelerator/state/` artifacts** (`migrations-<id>-stderr.log`; the
  failure return at `interactive-lib.sh:638-639` removes resume-state but not
  stderr). The owned-check whitelists only `-session.jsonl`, so under jj (which
  tracks created files) a dirty stderr.log defeats guarded resume — breaking the
  same in-flight case. **Fix:** clean the full `migrations-<id>-*` family on
  failure, or whitelist the family in the owned-check.
- 🟡 **Correctness/Safety**: **The owned glob `migrations-*-session.jsonl` is
  broader than the detection regex** (`migrations-[0-9a-z-]+-session\.jsonl`) and
  owns session logs of migrations **outside** this run's pending/applied set, and
  **custom `migration_session_log_path`** logs escape both. **Fix:** share one
  anchored pattern between the detector and the owned-check; constrain ownership
  to ids in pending ∪ applied (the id is in the filename), or document the
  widened residual explicitly.
- 🟡 **Safety**: **Loss of the structured discard affordance.** Today's branch
  prints `To discard: rm <path> (loses N decisions)` + a status hint; Phase 4's
  auto-resume replaces it with a vague pointer, and the "Ctrl-C" framing is wrong
  for a *completed* session log (no process to interrupt). **Fix:** make the
  owned-resume affordance reproduce the resolved `rm` + decision count verbatim;
  drop Ctrl-C framing for completed logs; assert it.
- 🟡 **Architecture**: **0069 replay-on-entry is now a load-bearing contract**
  with no fail-closed guard (asserted in prose only). **Fix:** record it in
  Limitations alongside "migrations must not commit", and add the CI assertion
  (protocol-log: decided transformations not re-prompted) that fails if replay
  regresses.
- 🟡 **Test Coverage**: **No existing helper composes Phase 4's test shape** —
  real repo + **no** FORCE + fixture-dir interactive migration + pre-seeded
  manifest/run-id. Every existing interactive replay test uses FORCE + fake `.git`
  (bypassing the pre-flight). **Fix:** add a success-criterion for a new harness
  helper; otherwise the cited models reproduce the fake-`.git` trap.
- 🟡 **Test Coverage**: **The existing session-log block test's continued passing
  is load-bearing** (no run-id ⇒ fail-closed ⇒ block) but unasserted. **Fix:** add
  complementary positive (matching run-id ⇒ resume) and negative cases.

### Minor (Phase 4)

- 🔵 In-flight resume without a decision channel re-stalls (doesn't complete) —
  affordance wording should set that expectation (point at `--decisions-file`).
- 🔵 The reordered else-arm must preserve the session-log branch's internal
  `exit 1`, or a stale in-flight log gets *both* the scaffold and the generic
  refusal.
- 🔵 Stale-session-log test trigger under-specified — prefer overwriting
  `migrations-run.id` with a sentinel revision to isolate the mismatch branch.
- 🔵 §3 affordance wording and the single-run-capture constraint aren't asserted /
  restated for Phase 4.
- 🔵 `dirty_tree_fully_owned` now carries three responsibilities; the session-log
  pattern is duplicated (glob vs regex) — factor one shared definition.
- 🔵 Build-in-Phase-3 / reorder-in-Phase-4 churns the same block — justify by
  independent mergeability or collapse into Phase 3.

### Cross-lens signal: keep Phase 4 in 0119, or split it out?

Three lenses independently noted Phase 4 grows 0119's blast radius from "mechanical
manifest in `run-migrations.sh`" to "two resume mechanisms across `run-migrations.sh`
**and** the interactive harness", adds the only cross-work-item runtime coupling
(0069 replay), the only behavioural `interactive-lib.sh`/0116 change, and needs its
own test harness. The mechanical axis (AC1–AC4) is the work item's stated core and
is COMMENT-grade ready. **Open decision:** fix Phase 4's critical+majors in place,
or split Phase 4 into a follow-up work item that builds on a shipped 0119.

### Assessment (Pass 3)

Phases 1–3 remain implementable (Pass-2 COMMENT). Phase 4 needs the critical fixed
regardless of where it lives — the empty-manifest reachability bug means the
interactive resume it promises does not currently fire. The split-vs-keep decision
should be made before further Phase 4 edits, since splitting would move Phase 4 out
of this plan entirely.

### Resolution (post-Pass-3 edits — fix in place)

Decision: **keep Phase 4 in 0119, fix in place.** All Pass-3 findings applied to the plan:

- 🔴 **Empty-manifest reachability** — `dirty_tree_fully_owned` now gates the
  manifest on `[ -r ]` (exists), not `[ -s ]` (non-empty); the per-path loop is the
  sole ownership authority. AC4 wording narrowed to "empty manifest + a dirty
  *mechanical* path → refuse". In-flight test added for the empty-manifest path.
- 🟡 **Session-artifact family** — new shared `is_session_artifact` predicate covers
  `-session.jsonl`, `-stderr.log`, `-resume-state.tmp` (the preserved-on-failure
  family), anchored to the detection regex's id class, used by **both** the detector
  and the owned-check (no glob/regex drift).
- 🟡 **Discard affordance** — §3 is now a hard requirement to reproduce the
  `To discard: rm … (loses N decisions)` line + status hint; Ctrl-C framing dropped
  for completed logs; asserted in tests.
- 🟡 **0069 replay contract** — recorded in Limitations with the protocol-log
  assertion as the fail-closed CI guard.
- 🟡 **Test harness gap** — new helper (real repo + no FORCE + interactive fixture +
  seeded manifest/run-id) is a success criterion; single-run-capture restated.
- 🟡 **Block-test load-bearing** — explicit block-vs-resume pivot test added.
- Minors (else-arm `exit 1` preserved, stale-trigger via sentinel run-id,
  near-miss-filename not owned, custom session-log path constrained, residual
  documented) all folded in.

Status: findings addressed in the plan; **pending Pass-4 verification** (verdict
stays REVISE until a re-review confirms the fixes hold and introduce nothing new —
prior passes showed fixes can introduce follow-on issues).

## Re-Review (Pass 4) — 2026-06-22 — Phase 4 fix verification

**Verdict:** COMMENT

Focused verification (correctness, safety, architecture, test-coverage) of the
Pass-3 Phase 4 fixes. **The Pass-3 critical and all majors are confirmed resolved**;
the relaxations hold and opened no proceed-when-should-refuse hole. One new major was
caught (introduced by the Pass-3 over-unification) and has been fixed this pass.

### Previously Identified Issues (Pass-3) — verification

- 🔴 **Empty-manifest reachability** — **Resolved & verified.** Correctness traced
  all cases: empty manifest + dirty mechanical path still refuses (path ∉ empty
  manifest, per-path loop is the authority); empty manifest + only a session
  artifact + valid run-id resumes; absent manifest fails closed. No mechanical AC4
  case regresses; AC4 wording accurately narrowed.
- 🟡 **Session-artifact family / glob-vs-regex drift** — **Resolved** (see new
  finding for the over-correction).
- 🟡 **Discard affordance** — **Resolved & verified**; now a hard requirement,
  stitched into the §2 resume-branch code block with the exact-count assertion.
- 🟡 **0069 replay contract** — **Resolved**; in Limitations with the protocol-log
  CI guard.
- 🟡 **Test harness gap / block-test load-bearing** — **Resolved**; new helper and
  block-vs-resume pivot specified concretely against the existing harness.
- 🟡 **Run-id staleness gate** — **Verified untouched** (still `[ -s ]` + base-rev
  equality); only the manifest gate relaxed to `[ -r ]`.

### New Issue Introduced (by the Pass-3 fix) — now fixed

- 🟡→resolved **Correctness/Safety/Architecture**: the Pass-3 instruction to reuse
  the **family-wide** `is_session_artifact` in the session-log *detector* would
  mislabel a dirty `stderr.log`/`resume-state.tmp` as a decisions log with a bogus
  `(loses N decisions)` count. **Fixed this pass:** split into two predicates sharing
  one id-class — `is_session_log` (canonical log only; detector + affordance) and
  `is_session_artifact` (full family; owned-check). Resolves the id-class drift too.

### Minors (this pass) — addressed

- Affordance now asserts the **exact** decision count (`loses $(wc -l <LOG)
  decisions`), not just the substring.
- `stderr.log`-is-owned (jj-guarded) and custom-`migration_session_log_path`-rejected
  tests added; near-miss test reframed as the *sole* non-owned path in an
  otherwise-owned tree (so it actually probes the predicate boundary).
- FIFO siblings (`-r2m.fifo`/`-m2r.fifo`) documented as deliberately excluded from
  the family (VCS doesn't track named pipes, so they never appear in the dirty set).

### Residuals (accepted, documented — not defects)

- Interactive ownership is by base-revision + path-pattern, wider than the
  mechanical manifest (a prior interactive run at the same *uncommitted* base
  revision); the affordance names id + decision count, advisory-only on the
  non-interactive `--decisions-file` path. VCS revert is the backstop.

### Assessment (Pass 4)

Converged. Across four passes the findings shrank critical → major → a clean
predicate split, with no new critical and no safety hole in Phase 4. All four phases
are implementable; remaining items are documented residuals and implementer-time
detail. The two-predicate split (this pass) is low-risk (suffix-scoped split of an
existing predicate). Recommend proceeding to implementation; a further verification
pass would likely be low-yield.

## Approved — 2026-06-22

**Verdict: APPROVE.** Reviewer accepted the plan after four review passes (initial
+ three re-reviews). All criticals and majors across the six lenses are resolved;
remaining items are documented, accepted residuals (path-only/base-revision
ownership, VCS-revert backstop) and implementer-time detail. Plan marked **ready**
for implementation.
