---
type: "plan-review"
id: "2026-10-16-0339-tree-reap-cross-artifact-lock-probe-review-1"
title: "Plan Review: Tree Reap Cross-Artifact Lock Probe Implementation Plan"
date: "2026-10-16T16:17:51+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-10-16-0339-tree-reap-cross-artifact-lock-probe"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "safety", "compatibility"]
review_number: 1
review_pass: 6
tags: ["cli", "launcher", "cache", "tree", "reap", "concurrency"]
last_updated: "2026-10-17T12:26:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Tree Reap Cross-Artifact Lock Probe Implementation Plan

**Verdict:** REVISE

The core design is sound. Reaps probe each other artifact's lock without
blocking and without creating the file, the reap's own lock is
short-circuited, the guard is held across that owner's removals, and the
disposition rule is a pure `const fn`. Each of the three phases leaves
production safe. There are 7 major findings, which is over the threshold of
3. Three of them are concrete defects: an existing integration test
contradicts AC11; `Artifact::owning` misattributes platform aliases that
extend a known one; and sparing every pinned partial archive removes the
only automatic recovery from a corrupt partial. The rest tighten the TDD
claims and the type-level guarantees around the guard and the caller's own
lock.

### Cross-Cutting Themes

- **`cache_prune_reclaims_orphan_residue` contradicts AC11** (flagged by: compatibility, test-coverage, correctness): `tests/tree_resolution.rs:880-893` asserts that `.tmp-orphan` is removed. After Phase 2 it is kept, so Phase 2's "full suite passes" cannot go green as written.
- **Platform-prefix parsing in `Artifact::owning`** (flagged by: compatibility, safety, architecture): splitting at the earliest `-<known-platform>-` misattributes a newer launcher's `linux-x64-musl` entries to `linux-x64`. The reap then probes the wrong lock. The same unguarded assumption covers artifact names that contain a platform.
- **Guard paired with its state by convention only** (flagged by: code-quality, architecture): `(TreeLock, Option<SingleFlight>)` and the near-duplicate `TreeLock`/`SingleFlightProbe` enums leave the hold-across-removal invariant enforced by nothing but a binding name.
- **The own-artifact bypass trusts the caller** (flagged by: architecture, safety): `own: &Artifact` is not tied to a held guard, so a future 0218/0222 caller could reap an unheld artifact without probing.
- **Silent keep-forever paths** (flagged by: code-quality, safety): probe errors and unowned entries are dropped with no trace, so growth under `trees/` cannot be diagnosed.
- **`Artifact` alongside string builders** (flagged by: code-quality, architecture): `TreePaths` still takes `(name, platform)` strings, so the grammar has a builder and a parser that can drift.
- **Non-unix stub surface** (flagged by: compatibility, correctness): `stubs` also needs `SingleFlightProbe`, not only the function.

### Tradeoff Analysis

- **Safety vs. simplicity of the keep rule**: sparing every artifact's pinned partial (AC3) is what makes resumption safe across artifacts. It also removes the accidental cross-artifact cleanup that clears a corrupt partial today. Recommendation: keep AC3 and delete the temp archive on verification failure inside `materialise_fresh`, so a surviving pinned partial is always a valid prefix.
- **Silence vs. diagnosability**: the plan deliberately leaves logging to 0218. One `tracing::warn!` per unprobeable group would change no behaviour and follows the existing `restart_backstop` pattern. Recommendation: add it.

### Findings

#### Critical

None.

#### Major

- 🟡 **Compatibility / Test Coverage / Correctness**: Existing `cache_prune_reclaims_orphan_residue` contradicts AC11 and is not addressed
  **Location**: Phase 2: Per-Artifact Resumable Rule
  The test plants `trees/.tmp-orphan` and asserts that prune removes it. After Phase 2 that entry has no owner and is kept, so Phase 2's suite criterion fails. Migration Notes also do not mention that `cache prune` stops removing unowned residue.
- 🟡 **Compatibility / Safety**: A platform alias that extends a known one is misattributed, so its entries go without their lock
  **Location**: Phase 1, Section 3: `Artifact::owning`
  `.tmp-browser-linux-x64-musl-<digest>.archive` parses as `(browser, linux-x64)`. The reap probes the wrong lock and can delete a newer launcher's in-progress residue. Anchor the match on the grammar after the platform (a well-formed digest), and add a test.
- 🟡 **Safety**: Sparing every pinned partial removes the only automatic recovery from a corrupt partial
  **Location**: Phase 2, Section 2: Pin lookup
  `materialise_fresh` leaves `temp_archive` in place on a verification failure, and `stream_archive` resumes from it. Today, `cache prune` for the other artifact clears it by accident. After Phase 2 nothing does, and `repair --force` uses the same path.
- 🟡 **Safety**: Safe removal on an absent lock depends on an invariant 0218 can break
  **Location**: Desired End State (`Absent` arm); What We're NOT Doing
  The flock-plus-unlink race lets a materialiser hold a lock on an unlinked inode. Every reap then sees `Absent`. Correctness adds that a deterministic partial-archive name can make a concurrent fresh materialise fail outright, not only lose its resume. Record the invariant for 0218.
- 🟡 **Test Coverage**: Red-first claims overstated: AC5 and AC9 (and AC7, AC12) are not behaviourally red
  **Location**: Phase 3: Success Criteria
  AC5 and AC9 expect removal, which Phase 2 already does, so they are red only through a compile error. AC12's "before item 4" is really "before item 3". Restate them as over-correction guards, or derive a genuine red.
- 🟡 **Code Quality**: The lock state and guard are returned as an unenforced tuple, and `TreeLock` mirrors `SingleFlightProbe`
  **Location**: Phase 3, Section 3: `tree_lock_of`
  Renaming `_held_across_removal` to `_` would drop the guard before any removal, and the types would allow it. Embed the guard in the state, for example `TreeLock::Taken(SingleFlight)`.
- 🟡 **Code Quality**: `.tmp-` classification is spread across `classify`, `temp_entries_by_owner` and `Artifact::owning`
  **Location**: Phase 2, Section 2 / Phase 3, Section 3
  The plan does not say what happens to `Residue::Temp` and the `.lock` check once temp entries bypass `classify`. Make `Artifact::owning` the only place that classifies them.

#### Minor

- 🔵 **Architecture / Safety**: The invariant that the caller holds its own artifact's lock is a convention, not a type
  **Location**: Phase 3: Grouped, lock-gated reap
  Pass a value that can only be built from a held `SingleFlight`, so `own` cannot be named without its lock.
- 🔵 **Architecture / Code Quality**: `Artifact` sits alongside the stringly-typed `TreePaths` builders instead of replacing them
  **Location**: Phase 1, Section 3
  Add `single_flight_lock_of(&Artifact)` (and `temp_archive_of`) so each name has its builder and parser in one place.
- 🔵 **Architecture**: Nothing checks the assumption that artifact names never contain a known platform
  **Location**: Phase 1: Known platforms
  Add a `pins.rs` test that no artifact name contains `-<platform>-` or ends with `-<platform>`.
- 🔵 **Architecture**: Restructuring into two passes may move when `referenced` is computed
  **Location**: Phase 3: `reap_generations`
  Today, temp `.ref` pointers protect their generation. State that `referenced_generations` is computed before the temp groups are processed.
- 🔵 **Compatibility / Correctness**: The non-unix stubs need the `SingleFlightProbe` type as well as the function
  **Location**: Phase 1, Section 4
- 🔵 **Code Quality / Safety**: Probe errors are discarded with no trace
  **Location**: Phase 3, Section 3; What We're NOT Doing
  Emit one `tracing::warn!` per unprobeable group.
- 🔵 **Code Quality**: `RemovalObserver` claims to see every removal, but generation sidecars bypass it
  **Location**: Phase 3, Section 2
  `reap.rs:81` calls `remove_file(sidecar)` directly.
- 🔵 **Correctness**: The Phase 2 verification grep also matches `is_reclaimable_generation`
  **Location**: Phase 2: Success Criteria
  `reap.rs:159` takes `&BTreeSet<String>`. Narrow the check to `keep_digests`.
- 🔵 **Test Coverage / Correctness**: The bystander's pinned digest must differ from the harness digest for AC3 to be red
  **Location**: Phase 2, Section 4
  Specify a distinct digest, for example `"b".repeat(64)`.
- 🔵 **Test Coverage**: Minisign-gated integration tests pass vacuously in the red step
  **Location**: Phases 2 and 3: Success Criteria
- 🔵 **Test Coverage**: AC3 does not say whether "X's lock free" means `Taken` or `Absent`
  **Location**: Phase 2: Pin lookup
- 🔵 **Test Coverage**: AC2's observer uses the production parser as its oracle, and how it shares results is unspecified
  **Location**: Phase 3: AC2

#### Suggestions

- 🔵 **Code Quality**: Some planned doc comments repeat what the names already say (`UnobservedRemovals`, `SingleFlightProbe`)
  **Location**: Phase 1, Section 4; Phase 3, Section 2
- 🔵 **Code Quality**: Phase 2 adds `is_orphan_temp_entry`, which Phase 3 immediately folds away
  **Location**: Phases 2 and 3
- 🔵 **Compatibility**: Add a rule that no `KNOWN_PLATFORMS` alias is a `-`-delimited prefix or suffix of another
  **Location**: Phase 1, Section 1
- 🔵 **Architecture**: The `reap_orphans` parameter list keeps widening as seams are added
  **Location**: Phase 3
- 🔵 **Safety**: Holding another artifact's lock while a large tree is removed can make its materialise time out
  **Location**: Phase 3, Section 3
- 🔵 **Test Coverage**: The probe unit tests on a held lock have no time bound
  **Location**: Phase 1, Section 4
- 🔵 **Test Coverage**: The soundness of the bound harness is checked only by hand
  **Location**: Phase 3: Manual Verification

### Strengths

- ✅ Probes never block and the reap holds at most one lock besides its own, so deadlock is ruled out by construction. The plan also states the tradeoff that both reaps may keep each other's residue.
- ✅ Short-circuiting the reap's own artifact avoids a real self-conflict: `flock` is per open file description. AC7 under `reap_for_y` proves this.
- ✅ The probe fails safe. A lock held elsewhere, a probe error and an unparseable name all keep the entry, and only `Taken` and `Absent` permit removal.
- ✅ `probe_single_flight` never creates a file, and `reap_for_y` compares the lock-file set on every test. Together they guarantee that no reap creates or removes a lock.
- ✅ `PinnedDigests` keeps dependencies flowing from the resolver to the reaper, and per-owner pins cover every platform on a shared root.
- ✅ `temp_residue_is_reclaimable` follows the 0334 functional-core pattern, with one test per cell.
- ✅ `reap_for_y` and the detached-thread bound turn a blocking probe into a failure rather than a hang. The A and Z bystanders on either side of X catch an early return.
- ✅ All `.tmp-` names stay byte-identical, so old and new launchers on one root agree on names and lock paths.
- ✅ Each phase is independently safe to merge, and out-of-scope obligations (0218, 0222) are recorded rather than half-built.

### Recommended Changes

1. **Rewrite `cache_prune_reclaims_orphan_residue` in Phase 2** (addresses: existing test contradicts AC11)
   Plant an owned orphan with `temp_generation` and assert that it goes. Assert that `.tmp-orphan` beside it survives. Add a Migration Notes line saying unowned residue is now left to 0218.
2. **Anchor `Artifact::owning` on the grammar after the platform** (addresses: platform-alias misattribution; unguarded name assumption; `KNOWN_PLATFORMS` rule)
   Accept a match only when a well-formed 64-hex digest follows. Add tests that `linux-x64-musl` entries have no owner, that no pinned artifact name contains a platform, and that no alias is a `-`-delimited prefix or suffix of another.
3. **Delete the temp archive on verification failure in `materialise_fresh`** (addresses: corrupt pinned partial wedges materialisation)
   Write a test first: a corrupt partial at the pinned digest does not make materialisation fail permanently. This could be a small Phase 2 item or a separate phase.
4. **Record the unlink-only-while-held invariant** (addresses: the absent-lock invariant that 0218 can break; the absent-lock failure mode)
   Put it in `probe_single_flight`'s docs, the rewritten module doc and the 0218 dependency note. State the real consequence: a transient materialise failure.
5. **Make the guard part of the lock state** (addresses: unenforced tuple; duplicate enums)
   Use `TreeLock::Taken(SingleFlight)` with the rule over `&TreeLock`, or one `From<Result<SingleFlightProbe, _>>`.
6. **Consolidate `.tmp-` classification** (addresses: classification spread; churn from the short-lived helper)
   Say in Phase 3 what happens to `classify` and `Residue::Temp`, and let `Artifact::owning` alone exclude locks. Consider introducing `temp_entries_by_owner` in Phase 2 so Phase 3 only adds the probe.
7. **Restate the red-first claims** (addresses: overstated red claims; bystander digest; minisign)
   Split AC5 and AC9 out as over-correction guards, change AC12's "before item 4" to "before item 3", pin `BYSTANDER` to a distinct digest, and add minisign or `CI=1` as a precondition.
8. **Smaller tidy-ups** (addresses: minor findings)
   - Add the stub `SingleFlightProbe`.
   - Narrow the `rg` check to `keep_digests`.
   - Route sidecars through the observer, or narrow its doc.
   - Compute `referenced` before the temp groups.
   - Emit one `tracing::warn!` per unprobeable group.
   - Make AC3's lock-file state explicit.
   - Give the AC2 observer an exact-path oracle and an `Arc<Mutex<_>>` record.
   - Drop the doc comments that only restate names.

## Per-Lens Results

### Architecture

**Summary**: The plan is structurally sound. Each entry has an explicit owning `Artifact`, the disposition rule is pure, and `PinnedDigests` is inverted so the reaper gains no dependency on the resolver. Non-blocking probes rule out deadlock, and each phase can merge on its own. Remaining concerns are about how the design evolves: the own-lock invariant is a convention, `Artifact` sits beside the string builders, the signature keeps widening, and the two-pass restructure could change when `referenced` is computed.

**Strengths**: `PinnedDigests` is declared in `reap` and implemented by the resolver. The `const fn` disposition rule matches the 0334 rules. `SingleFlightProbe::Taken` returns a real guard that 0218 can reuse. Probes never block. The `BTreeMap` grouping gives a deterministic order. Each phase is independently safe. `KNOWN_PLATFORMS` is tied to the pins. Out-of-scope work is recorded.

**Findings**:
- minor / medium — Phase 3: Grouped, lock-gated reap — The invariant that the caller holds its own lock is a convention, not a type. Suggest `&HeldTreeLock { artifact, _guard }`.
- minor / medium — Phase 1: `Artifact` value type — `Artifact` sits alongside the stringly-typed path builders instead of replacing them. Add `single_flight_lock_of(&Artifact)`.
- minor / medium — Phase 1: Known platforms — The assumption that artifact names never contain a known platform has no test.
- minor / low — Phase 3: `reap_generations` — Restructuring into two passes may move when `referenced` is computed, so removed temp pointers would stop protecting their generation.
- suggestion / medium — Phase 3: `tree_lock_of` — Whether a guard is present depends on a convention in the tuple.
- suggestion / low — Phase 3: Callers — The `reap_orphans` parameter list keeps growing. Consider a public `ReapContext`.

### Code Quality

**Summary**: The domain concepts are well named, the disposition rule is small and tested, and the substring keep set is replaced by a per-artifact lookup. Phase 3 still loses readability in three places: the guard is carried in a tuple, `.tmp-` classification is spread across several functions, and strings are still passed where an `Artifact` would fit. Probe errors are dropped without any trace.

**Strengths**: The `const fn` rule has one test per cell. `PinnedDigests` follows the existing seam pattern. Phase 1 removes the last inline `.tmp-` format string, and an `rg` check guards it. The tests stop `KNOWN_PLATFORMS` drifting. The reap is decomposed into small functions.

**Findings**:
- major / high — Phase 3, Section 3 — The lock state and guard are returned as an unenforced tuple, and `TreeLock` mirrors `SingleFlightProbe`. Put the guard inside the state.
- major / medium — Phase 2, Section 2 / Phase 3, Section 3 — `.tmp-` classification is spread across `classify`, `temp_entries_by_owner` and `Artifact::owning`, and `Residue::Temp` may be left unused.
- minor / medium — Phases 1–3 — `Artifact` is introduced, but callers still split it back into name and platform strings.
- minor / high — Phase 3, Section 2 — `RemovalObserver` claims to see every removal, but generation sidecars bypass it (`reap.rs:80-82`).
- minor / medium — Phase 3, Section 3 — Probe errors are discarded with no trace. Keep the error in `Unprobeable`, or log it.
- suggestion / medium — Phases 1 and 3 — Some planned doc comments only restate names.
- suggestion / medium — Phases 2 and 3 — `is_orphan_temp_entry` is added in Phase 2 and folded away in Phase 3.

### Test Coverage

**Summary**: Coverage is strong overall. Every AC maps to a named test, `reap_for_y` puts three preconditions in one place, and the detached-thread bound is sound. There are two real gaps: an existing integration test contradicts AC11, and several red-first claims overstate what genuinely fails before the change. Smaller issues: some fixtures are underspecified, and minisign-gated tests pass vacuously.

**Strengths**: `reap_for_y` holds the lock and compares the lock set on every test. Scoped threads are correctly avoided. AC7 doubles as a guard against the reap probing its own lock. AC2 asserts an exact removal count. The A and Z bystanders catch an early return. AC4, AC8, AC10 and AC11 are genuinely red. The EISDIR fixture gives a probe error without a special filesystem. The disposition rule has one test per cell.

**Findings**:
- major / high — Phase 2 — `cache_prune_reclaims_orphan_residue` (`tree_resolution.rs:881-893`) contradicts AC11 and is not addressed.
- major / high — Phase 3: Success Criteria — AC5 and AC9 are red only through a compile error. AC7 in Phase 2 is the same, and AC12 is really "before item 3".
- minor / high — Phases 2 and 3 — Minisign-gated tests pass vacuously in the red step.
- minor / medium — Phase 2, Section 4 — The bystander's digest must differ from `harness.archive_sha`.
- minor / medium — Phase 2: AC3 — "Free lock" could mean `Taken` or `Absent`, and nothing combines `Taken` with a resumable archive.
- minor / medium — Phase 3: AC2 — The observer uses the production parser as its oracle, and how it shares results is unspecified.
- suggestion / medium — Phase 1, Section 4 — The held-lock probe tests have no time bound.
- suggestion / low — Phase 3: Manual Verification — The bound harness is checked only by hand.

### Correctness

**Summary**: The concurrency design is sound. Probes never block, the guard lives across that owner's removals, and short-circuiting the reap's own artifact avoids a self-conflict. `Artifact::owning` parses every real name correctly. The snippets borrow-check. The main gap is the contradicting integration test, plus smaller points about verification commands, the stubs and the absent-lock consequence.

**Strengths**: The `_held_across_removal` binding keeps the guard alive. `LOCK_NB` rules out deadlock. The trailing hyphen in `-<platform>-` excludes lock files, and `at > 0` excludes entries with no name. The own-artifact short-circuit is correct under per-open-file-description `flock`. Taking the snapshot first and then probing is safe for `Taken`. The probe's error mapping matches `try_single_flight`. The generation pass re-evaluates each generation when it decides.

**Findings**:
- major / high — Phase 2: Success Criteria — The existing `cache_prune_reclaims_orphan_residue` contradicts the rule that unparseable entries are kept.
- minor / high — Phase 2: Success Criteria — The `rg` check also matches `is_reclaimable_generation` (`reap.rs:159`).
- minor / medium — Phase 1, Section 4 — The non-unix stubs also need `SingleFlightProbe`.
- suggestion / medium — Phase 3, Section 3 — The absent-lock window can make a fresh materialise fail, not just lose its resume, because the partial archive name is deterministic. Document this, and flag it for 0218.
- suggestion / medium — Phase 2, Section 4 — AC3 is red only if the bystander's digest differs from `driver`'s.

### Safety

**Summary**: The plan closes the cross-artifact race, and it fails safe on any outcome other than `Taken` or `Absent`. Three gaps remain: aliases matched by prefix probe the wrong lock; sparing every pinned partial removes the only recovery from a corrupt one; and "absent lock means safe" relies on an unlink invariant the plan never states for 0218. Residue that is kept forever is invisible.

**Strengths**: The fail-safe outcomes are tested by AC1, AC6 and AC11. The guard is named and AC2 verifies it. The probe never creates a file, and the lock set is compared. Non-blocking probes and the 10s bound turn a blocking probe into a failure. The phases are ordered so each is safe on its own.

**Findings**:
- major / medium — Phase 1, Section 3 — Future platform aliases that extend a known alias are checked against the wrong lock.
- major / medium — Phase 2, Section 2 — Sparing every pinned partial removes the only automatic recovery from a corrupt partial (`resolver.rs:262`, `download.rs:46-47`).
- major / medium — Desired End State (`Absent`); What We're NOT Doing — Safe removal on an absent lock depends on an invariant 0218 can break: the flock-plus-unlink race.
- minor / medium — Phase 3, Section 3 — The own-artifact bypass trusts an unenforced claim that the caller holds the lock.
- minor / medium — What We're NOT Doing — Residue kept forever (no owner, or an unprobeable lock) is invisible. A kept `.tmp-<gen>.ref` also pins its generation.
- suggestion / low — Phase 3, Section 3 — Holding X's lock during a large `remove_dir_all` can make X's bounded waiter time out.

### Compatibility

**Summary**: The cross-version contracts hold. Names, lock paths and `flock` semantics are byte-identical, the probe never creates a file, and every probe is non-blocking. There are two gaps: a newer launcher's alias that extends a known one is misparsed and deleted without its lock, and Phase 2 changes what `cache prune` visibly does, which an existing test asserts.

**Strengths**: All `.tmp-` names are unchanged. The probe sees an old launcher's in-progress materialise as `Held`. Non-blocking probes cannot deadlock with old launchers. Unknown platforms are kept. Per-owner pins cover every platform. The drift tests tie `KNOWN_PLATFORMS` to the pins. No external consumer of `reap_orphans` exists.

**Findings**:
- major / medium — Phase 1, Section 3 — A newer launcher's alias that extends a known one is misattributed, and its entries are removed without its lock. Anchor the split on a well-formed digest, and prefer the longest match on ties.
- major / high — Phase 2 / Migration Notes — `cache prune` stops removing unowned residue, and `cache_prune_reclaims_orphan_residue` still asserts that it does.
- minor / high — Phase 1, Section 4 — The non-unix stubs need the `SingleFlightProbe` type as well as the function.
- suggestion / medium — Phase 1, Section 1 — Make it a forward-compatibility rule that no `KNOWN_PLATFORMS` alias extends another.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-10-16T16:46:15+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Compatibility / Test Coverage / Correctness**: `cache_prune_reclaims_orphan_residue` contradicts AC11 — Resolved
- 🟡 **Compatibility / Safety**: Platform alias extending a known one is misattributed — Resolved (digest-anchored parse)
- 🟡 **Safety**: Sparing pinned partials removes recovery from a corrupt partial — Partially resolved (a corrupt prefix is discarded; a full-length partial still wedges, see below)
- 🟡 **Safety**: Absent-lock safety depends on a 0218 invariant — Resolved in the docs; how it holds across launcher versions is a new finding
- 🟡 **Test Coverage**: Red-first claims overstated — Partially resolved (AC5/AC9/AC7/AC12 fixed; the unit ACs are now red only by compilation)
- 🟡 **Code Quality**: Unenforced `(TreeLock, Option<SingleFlight>)` tuple — Resolved (`OwnerLock::Taken(SingleFlight)`)
- 🟡 **Code Quality**: `.tmp-` classification spread across three places — Resolved
- 🔵 **Architecture / Safety**: Own-lock invariant is a convention — Resolved (`HeldTreeLock`)
- 🔵 **Architecture / Code Quality**: `Artifact` alongside the string builders — Resolved (the rest is deferred explicitly)
- 🔵 **Architecture**: Artifact names containing a platform go unchecked — Resolved (the assumption is removed)
- 🔵 **Architecture**: Timing of the `referenced` computation — Resolved
- 🔵 **Compatibility / Correctness**: Non-unix stubs need `SingleFlightProbe` — Resolved
- 🔵 **Code Quality / Safety**: Probe errors discarded silently — Resolved
- 🔵 **Code Quality**: Observer contract and sidecars — Resolved
- 🔵 **Correctness**: The `rg` check also matches `is_reclaimable_generation` — Resolved
- 🔵 **Test Coverage**: Bystander digest, minisign gating, AC3 lock state, AC2 oracle, unbounded probes, harness soundness — Resolved
- 🔵 Suggestions (redundant doc comments, short-lived helper, alias rule, widening signature, holding a lock during a large removal) — Resolved or accepted explicitly

### New Issues Introduced

- 🟡 **Safety / Correctness**: A full-length pinned partial wedges materialisation permanently, and the window is wider than a crash. Any failure after streaming leaves it, including a failed attestation fetch. A real server then answers 416 to the resume, the fetch reports `Unreachable`, and after Phase 2 no reap or `repair --force` clears the file. The mock answers 200, which hides this (`tests/common/mod.rs:188-189`).
- 🟡 **Compatibility**: The inode re-check handed to 0218 is unsatisfiable on a mixed-version root. Shipped launchers and this plan's `HeldTreeLock` do not re-check. 0218's Assumptions (`:524-526`) still accept the old-launcher race.
- 🟡 **Test Coverage**: The unit ACs depend on `Reaper` / `reap_orphans(&held)`, which arrive in the same item as the behaviour, so their claimed behavioural red cannot be observed.
- 🔵 **Compatibility**: `is_generation` must keep rejecting `TEMP_PREFIX` names. Otherwise the generation arm age-reaps `-1-` temp trees that the temp arm keeps (newer-platform residue, or a lock held elsewhere).
- 🔵 **Correctness**: Non-overlapping `match_indices` misses the split for a name that ends with its own platform (`foo-linux-x64`), so that artifact's residue leaks.
- 🔵 **Architecture / Code Quality / Test Coverage**: The Key Discovery is wrong. `reap.rs` already has the `logged()` tracing capture (`:606-649`), so `TracingReapObserver` is an unneeded seam. Warn directly and test with `logged()`.
- 🔵 **Architecture**: `HeldTreeLock` proves the artifact but not the cache root the `Reaper` scans.
- 🔵 **Architecture / Code Quality**: `repair` still takes the raw lock (`resolver.rs:678-680`), and `HeldTreeLock` sits in the flock-primitives module.
- 🔵 **Code Quality**: Adjacent `owner`/`own` parameters are easy to swap. A failed discard of a rejected partial is swallowed silently.
- 🔵 **Code Quality / Test Coverage**: The generation tests' path to the new signature is unspecified, since `reap_for_y` hard-codes `present()`.
- 🔵 **Test Coverage**: The integration AC3/AC12 tests skip the lock-set invariant and use an absent lock. The `HeldTreeLock` tests are unbounded and skip the free `try_take` path.
- 🔵 **Correctness**: `lock_path` and the `try_single_flight` import go dead in `materialise`, which `-D warnings` rejects.
- 🔵 Suggestions: limit the discard to `UnexpectedDigest` or document its scope; pin `.tmp-` names as literal cross-version contracts; drop the duplicated `generation_prefix` doc note; round-trip every pinned pair; test that a failed discard does not mask the error; check `jj` history for older shipped `.tmp-` naming.

### Assessment

The revision resolved 21 of the 24 prior findings and partially resolved the other 3. It introduced 3 new majors, which meets the threshold. Two of them are scoping errors in the revision itself: the full-length-partial gap was deferred when Phase 2 is what makes it permanent, and the 0218 invariant cannot hold across launcher versions. The third is a sequencing fix in TDD order: add an API-only refactor step before each behaviour item. The `is_generation` and `match_indices` minors are real latent defects and cheap to fix. With those addressed, the plan should reach COMMENT or APPROVE.

## Re-Review (Pass 3) — 2026-10-17T00:09:04+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Safety / Correctness**: A full-length pinned partial wedges materialisation — Resolved (hash and skip the fetch; an over-long file is refetched whole; discard on `UnexpectedDigest`; the mock answers 416)
- 🟡 **Compatibility**: The 0218 inode re-check could not hold across versions — Resolved (the check is built into the single-flight locks; 0218 names the risk from launchers before 0339)
- 🟡 **Test Coverage**: Behavioural red could not be observed — Resolved (each test ran against its phase's API and fails for its stated reason)
- 🔵 **Compatibility**: `is_generation` swept `.tmp-` names — Resolved
- 🔵 **Correctness**: The `match_indices` overlap — Resolved
- 🔵 **Architecture / Code Quality / Test Coverage**: The false tracing claim and its observer seam — Partially resolved (the seam is gone; AC6′'s capture does not reach the bounded thread, see below)
- 🔵 **Architecture**: `HeldTreeLock` root binding; `repair`; module placement — Resolved
- 🔵 **Code Quality**: owner/own swap; swallowed discard failure; generation-test migration; duplicated doc; dead `lock_path` — Resolved
- 🔵 **Test Coverage**: Integration lock set; `HeldTreeLock` bounds and the free `try_take` path; round-trips; discard masking — Resolved

### New Issues Introduced

- 🟡 **Architecture / Code Quality / Test Coverage**: AC6′ wraps a reap that runs on a different thread in a thread-local `logged()` capture. `with_default` does not reach `within`'s spawned thread, so AC6′ can never go green. Have `within` carry the caller's dispatcher into the spawned thread, and add a `testing.rs` test for that.
- 🟡 **Test Coverage**: No failing test drives the inode retry loops in `take_single_flight`, `try_single_flight` and `probe_single_flight`; only `is_still_at` is tested. Extract one `lock_confirmed` loop with an injectable opener, and test the replaced and unlinked cases through it.
- 🔵 **Architecture / Safety / Code Quality / Compatibility / Correctness**: The inode retry loop has no bound, and it treats any stat error as a replaced file, so it can spin forever. Use a three-way check (confirmed, replaced or unlinked, error), retry only when the file was replaced or unlinked, and cap the retries.
- 🔵 **Architecture / Compatibility / Correctness**: Skipping the fetch relies on `archive_size` being the exact length, but its doc says only that it bounds the download. State the exact length in the manifest doc and schema, or treat a 416 whose `Content-Range` length matches the file on disk as complete.
- 🔵 **Safety**: A complete partial that is kept can wedge the free-space precheck. Subtract the bytes already on disk from the requirement.
- 🔵 **Safety**: The 0218 wording names only the `Absent` arm. An inode orphaned by a launcher before 0339 also exposes the own-artifact and `Taken` arms, and lets concurrent appends corrupt the partial.
- 🔵 **Compatibility**: A complete partial that is kept stalls launchers before 0339, which spend their retry deadline. Record this in Migration Notes, or discard the partial when the attestation fetch fails after a complete stream.
- 🔵 **Code Quality**: The open, flock, confirm and retry loop is written out three times. The `Absent` invariant is documented three times, in a version-relative form ("before this one"). `Reaper`'s Phase 3 fields are only implied, so a stale `paths` field could survive alongside `held`.
- 🔵 **Test Coverage**: The name-only mutation is caught by AC8, not AC9. A full-length corrupt partial, and a signature failure that keeps the file, have no tests.
- 🔵 Suggestions: the observer doc overstates what it sees (it fires before removal; the test should assert a subset); compare identity with `MetadataExt` `dev()`/`ino()`, as `directory_identity` does.

### Assessment

Every major from pass 2 is resolved, and the core design is now stable across all six lenses. Two majors remain. Both concern how a test is wired rather than the design: the AC6′ subscriber does not reach the bounded thread, and the inode loop has no driving test. That puts the plan under the REVISE threshold of 3. The retry loop with no bound or error handling, flagged by five lenses, is the most important minor to fix before implementation, together with the exact `archive_size` contract. The plan is acceptable to implement once these are folded in.

## Re-Review (Pass 4) — 2026-10-17T09:49:49+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Architecture / Code Quality / Test Coverage**: AC6′ capture does not reach the bounded thread — Resolved (`within` carries the dispatcher; a test covers it)
- 🟡 **Test Coverage**: No failing test drives the inode loops — Partially resolved (`lock_confirmed` is tested directly; moving `take`/`try` onto it is undriven)
- 🔵 **Five lenses**: Unbounded retry loop that treats a stat error as replaced — Resolved (three-way `placement`, `MAX_LOCK_ATTEMPTS`)
- 🔵 **Architecture / Compatibility / Correctness**: Exact-`archive_size` reliance — Partially resolved (a 416 whose total matches the range start is complete; a 416 below the start, or one with no `Content-Range`, still wedges)
- 🔵 **Safety**: Free-space precheck wedge — Partially resolved (`space_needed` is tested; its wiring into step 3 is undriven)
- 🔵 **Safety**: 0218 wording — Resolved
- 🔵 **Compatibility**: Pre-0339 stall noted — Resolved, but the note is inaccurate (see below)
- 🔵 **Code Quality**: Loop and invariant written in three places; `Reaper` fields implied — Resolved
- 🔵 **Test Coverage**: AC8/AC9 mutation pairing — Resolved; the corrupt complete partial — Resolved; signature-kept — Partially resolved (see below)
- 🔵 Suggestions (observer doc, `MetadataExt`) — Resolved

### New Issues Introduced

- 🟡 **Correctness / Safety / Compatibility**: Under a loose bound, a 416 whose total is below the range start, or a 416 with no `Content-Range`, stays retryable, is never discarded, and wedges materialise permanently. Make every 416 on a ranged request terminal. Treat a total equal to the start as `AlreadyComplete`, and anything else as a range rejection that refetches whole (truncate) or discards. Test an over-long partial under the loose-bound harness.
- 🟡 **Code Quality**: `OwnerLock::Taken(SingleFlight)` is never read. The `dead_code` lint fires on unread tuple fields since 1.77, and `warnings = "deny"` makes that fail `cli:check`. Use `Taken { _guard: SingleFlight }`, matching `SingleFlight { _file }` and `HeldTreeLock { _guard }`.
- 🟡 **Test Coverage**: `a_partial_whose_signature_fails_is_kept_for_the_next_run` can pass without testing anything. Re-routing the manifest fails the manifest's own minisign check at step 2, which is the same `Attestation` variant, so step 5 is never reached. The harness keeps neither the secret key nor `minisign`. It needs to keep them so it can re-sign the substituted manifest, and the test should assert the archive-signature detail.
- 🔵 **Architecture / Code Quality / Correctness**: The mapping of `Locked` arms that `take` and `try` can never receive is unspecified, which invites `unreachable!` or a silent `Ok(None)`. Map them to `TreeError::Lease`, or split the opener's return type so a creating opener yields `File`.
- 🔵 **Architecture / Code Quality**: `StreamedBody { sha256, continuation }` carries a meaningless digest under `AlreadyComplete`, and `open_sink` still takes a `bool`. Carry the data in the variants instead.
- 🔵 **Architecture / Code Quality**: The none/partial/complete/over-long classification is encoded in both `stream_archive` and `space_needed`. Give it one name, for example `PartialArchive::assess`.
- 🔵 **Correctness**: `within` reports a panic in the closure as "blocked", so the bound test can pass for the wrong reason. Match `Timeout` against `Disconnected` and re-raise the thread's panic.
- 🔵 **Compatibility / Correctness**: `fstat` and `stat` can disagree on some overlayfs and FUSE mounts, so every lock would fail. Either compare like with like (re-open and `fstat`), or record the stable-inode assumption beside ADR-0072's `flock` assumption and name it in the exhausted-attempts error.
- 🔵 **Compatibility / Safety**: The Migration Note says an old launcher's own reap clears the stalled partial. It does not: its own keep set spares it. Reword it as the loss of accidental clearing.
- 🔵 **Safety**: The `Absent` arm removes a pre-existing partial without a guard, racing a launcher from another version that reopens it. Spare partials in `Absent`, or record the race.
- 🔵 **Test Coverage**: Wiring `space_needed` into step 3 is observable through `DiskShortfall { needed }` but has no test. Moving `take`/`try` onto `lock_confirmed` has no public-level red test. The give-up and identity-error tests need open counts to tell them apart. One fetcher test that is claimed red is green today, and the other is red only because it does not compile.
- 🔵 Suggestions: when the cap is exhausted in the waiter, treat it as `Held` rather than a sticky `Lease`; pass `path` into the opener; `#[allow(clippy::panic)]` on `within` and rename `REAP_BOUND`; test a 416 with no `Content-Range`.

### Assessment

The core design has held for two passes. Every finding in this pass is in the newest layer, the partial-recovery and lock-confirmation hardening. Three majors meet the threshold:

- The 416 recovery is still incomplete. Three lenses converge on the fix: make every 416 on a ranged request terminal.
- There is a `dead_code` build break, fixed by one rename.
- One test can pass without testing anything, because the harness needs to re-sign the manifest.

All three are small, mechanical fixes. The minor findings are mostly about tightening tests and modelling.

## Re-Review (Pass 5) — 2026-10-17T11:17:05+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Correctness / Safety / Compatibility**: A 416 below the range start, or with no `Content-Range`, wedges — Resolved (every ranged 416 is terminal; `RangeRejected` triggers one whole refetch)
- 🟡 **Code Quality**: `OwnerLock::Taken` tuple field trips `dead_code` — Resolved
- 🟡 **Test Coverage**: The signature-kept test can pass without reaching step 5 — Resolved (`serve_manifest` re-signs the manifest; the test asserts the detail)
- 🔵 Unspecified mapping of the `Locked` arms — Resolved (explicit table)
- 🔵 `StreamedBody` meaningless digest — Resolved (`Streamed`, `SinkMode`)
- 🔵 Classification duplicated — Resolved (`PartialArchive::assess`)
- 🔵 `within` reported a panic as blocked — Resolved
- 🔵 `fstat` vs `stat` on overlayfs/FUSE — Resolved, but the fix introduces the NFS regression below
- 🔵 Migration Note accuracy — Resolved
- 🔵 `Absent`-arm race — Resolved (recorded)
- 🔵 Test-coverage minors (space wiring, counted opens, red claims, a 416 with no header) — Resolved
- 🔵 Public `take` red test — Partially resolved (relies on a sleep; `try` argued from the shared loop)

### New Issues Introduced

- 🟡 **Compatibility** (verified against `cli/lease/src/lib.rs:137-139`, `:202-203`): `placement` reopens the lock path and closes that second descriptor after `flock`. Where `flock` is emulated with POSIX record locks, which includes Linux NFS without `local_lock=flock` and macOS NFS, closing any descriptor drops the process's lock. Tree locks and probe-taken locks would then be released silently on NFS cache roots, which work today. Either keep the confirming descriptor alive inside `SingleFlight` until release, or compare `fstat(held)` with a path `stat` and record the stable-inode assumption. Add a test that the guard still excludes a fresh descriptor after `placement` runs.
- 🔵 **Correctness**: An existing bug: `get_streaming` retries a ranged attempt with the same `range_from`, and the append sink appends after the failed attempt's bytes, which duplicates and corrupts the file. The new discard turns this into loss of all resume progress. Make `SinkMode::Append` carry the offset and `set_len(offset)` before appending, and test a mid-body failure followed by a successful retry.
- 🔵 **Correctness**: `space_needed` charges an over-long partial for the whole archive, although the truncating refetch frees that partial first, so the precheck can wedge on a nearly full disk. Charge `uncompressed_size + archive_size.saturating_sub(len)`.
- 🔵 **Safety**: An unreadable pinned partial is never discarded or reaped (the `digest_file` and reopen errors are `Extraction`, `verifies_stream` read errors are `Attestation`). Discard on re-read failure, or have `repair` discard its own temp archive first.
- 🔵 **Safety**: "Fails that run" understates the impact: every cause except "in progress" is sticky for the session TTL, and several are classed as Refusal (tampering). Reword in the module doc, in 0218 and in Migration Notes. Optionally map a `NotFound` on the loser's own temp archive to `Unreachable`.
- 🔵 **Architecture / Safety**: `KeptBeingReplaced` is silent, so a persistent environment fault looks like contention, and the waiter can stall up to 20s per invocation. Emit `tracing::warn!` with the lock path, and test it with `logged`.
- 🔵 **Architecture / Code Quality / Correctness**: An unranged fetch, and the refetch after `RangeRejected`, still return the four-variant `Streamed`. Split the fetcher into a whole-fetch entry point that returns `{ bytes, sha256 }` and a ranged one.
- 🔵 **Architecture**: The partial is read and assessed twice, and `space_needed` re-derives the bytes still to fetch. Assess once, or add `PartialArchive::bytes_to_fetch`.
- 🔵 **Code Quality / Test Coverage**: The `bytes` fields are read only by tests, and step 1a constructs neither `AlreadyComplete` nor `RangeRejected`. Under `warnings = "deny"`, both break the build unless the `#[allow(dead_code)]` is kept, or `bytes` is dropped.
- 🔵 **Code Quality**: The `reap.rs` module doc names 0218 and carries a meta-instruction. Remove both.
- 🔵 **Test Coverage**: The loose-bound complete test can't tell `AlreadyComplete` from a whole refetch; assert one archive hit. The public `take` test's 100ms sleep can let it pass vacuously. The per-caller `Locked` mappings, notably probe `KeptBeingReplaced` → `Held`, are untested; extract the mappings as pure functions and give each cell a test.
- 🔵 Suggestions: `const fn` on trivial accessors and constructors (`missing_const_for_fn`); a neutral message for the impossible `Absent`; avoid the `held` shadowing in `placement`, and rename `PartialArchive::None`; get_streaming outcomes when no range was requested; boundary tests for a `Content-Range` total above the start and for a step-2 discard.

### Assessment

All three majors from pass 4 are resolved. One new major remains, a real regression that I verified: the like-with-like reopen releases the lock on NFS. That puts the plan below the REVISE threshold. The minor findings cluster around partial recovery (append on retry, over-long space, unreadable partial) and around making the fetcher and `Locked` mapping types more precise. Fix the NFS issue and the append-on-retry bug before implementation; the rest can be folded in as each step is built.

## Re-Review (Pass 6) — 2026-10-17T12:00:22+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Compatibility**: The like-with-like reopen releases NFS-emulated locks. Resolved: the confirming descriptor now lives in `SingleFlight`. Correctness and compatibility each confirmed that the descriptors dropped on the displaced path name files this single-threaded process does not lock.
- 🔵 **Correctness**: Append-on-retry. Resolved (`SinkMode::Append { from }` with `set_len`). A residual zero-extend case is listed below.
- 🔵 **Correctness**: The over-long space charge. Resolved (`additional_bytes`).
- 🔵 **Safety**: An unreadable partial is never discarded. Partially resolved: only the Complete arm; a short partial that cannot be read or written still wedges.
- 🔵 **Safety**: The sticky wording. Partially resolved: it says "rest of the session" where the marker is bounded by a TTL, and it omits that a Refusal stops a `--fail-safe` crawl.
- 🔵 `KeptBeingReplaced` silent; the four-variant fetch outcome; the double assessment; dead fields; the module doc naming 0218; the untested `Locked` mappings; the loose-bound hit count. All resolved.
- 🔵 The public `take` test sleep. Accepted and documented.

### New Issues Introduced

- 🟡 **Test Coverage**: Once the mock answers 416, nothing covers a server that answers a range request with 200. The `Resumed::Whole` and `Truncate` cell is untested, and a mutation that appends on a 200 passes. Add a fetcher test where `resume` meets a 200 and returns `Whole` with `Truncate`.
- 🔵 **Correctness / Safety / Test Coverage**: A short partial that can be stat'd but not opened for writing or reading fails `open_sink` as `Retryable`, ends `Unreachable` and is never discarded. Make a sink-open failure terminal, then discard and fetch whole. Add a test for a short unreadable or unwritable partial, and assert the error variant.
- 🔵 **Correctness / Safety**: `set_len(from)` silently zero-extends a file shorter than `from`, which a Truncate attempt that failed mid-body or a reap from an older launcher can leave behind. A 206 that starts at an offset other than `from` is also appended. Open the Append sink without `create`, fail it into a whole fetch when the file is shorter than `from`, and treat a mismatched `Content-Range` start as `RangeRejected`.
- 🔵 **Correctness / Safety**: `digest_partial` discards on any error, including `Interrupted`, EMFILE and EIO, and reports `Extraction`, which is Refusal-class. Hash with `io::copy`. Discard only when the error belongs to the file itself.
- 🔵 **Safety**: Reword the sticky statements as "sticky for the marker TTL; a Refusal-class cause also stops a `--fail-safe` crawl".
- 🔵 **Compatibility**: An `ESTALE` from `fstat` after another NFS client unlinks the file should count as `Displaced`, not a `Lease` error.
- 🔵 **Architecture**: The confirmed-lock invariant is assembled in three mappers. Have `lock_confirmed` return `Taken(SingleFlight)` from one constructor.
- 🔵 **Architecture / Code Quality**: The discard policy is split between `download.rs` and the resolver, where it is keyed on `UnexpectedDigest` meaning different things at steps 2 and 5. The helper's name fits only one case.
- 🔵 **Code Quality**: `'_` inside the `FnMut` return type of `get_whole` and `resume` is E0106. Keep the named `<'sink>` shape.
- 🔵 **Code Quality**: `within_bound` duplicates `within` without its panic handling. Share one implementation.
- 🔵 **Test Coverage**: `FailFirstRange` must promise the full suffix in `Content-Length` and close without the read-to-end guard. The step-2 refusal guard must plant its partial at the expected digest (`"0"×64`).
- 🔵 Suggestions: the never-probe-your-own-held-lock rule on NFS is worth a doc line; `stream_archive` takes a path and an assessment that must agree; a directory at the partial path still wedges; skip the 0o000 test on readability, not euid; `OverLong` does not need a payload; describe the confirming-descriptor test as a structural check.

### Assessment

Two lenses confirmed the NFS fix independently. The one remaining major is a test gap, not a design flaw. Every finding now sits at the outer edge of partial-archive recovery, or concerns precise wording and signatures. The core design has held for four passes, and the remaining items are implementation-level refinements. The plan is ready to implement once the 200-on-range test, the `'_` signature fix and the unwritable or short partial handling are folded in.

## Approval — 2026-10-17T12:26:43+00:00

**Verdict:** APPROVE

The plan was revised after pass 6, and every finding from that pass was addressed. Approved for implementation without a seventh pass. The design has held for four passes, and any edge case that remains is expected to surface in the red-green steps.
