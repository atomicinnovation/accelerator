---
type: "plan-review"
id: "2026-10-19-0218-bound-cache-root-growth-review-1"
title: "Plan Review: Bound a Relocated Cache Root's Growth Implementation Plan"
date: "2026-10-19T18:54:53+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-10-19-0218-bound-cache-root-growth"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "safety", "security", "compatibility", "performance"]
review_number: 1
review_pass: 7
tags: ["cli", "launcher", "cache", "tree", "prune", "claims", "retention"]
last_updated: "2026-10-21T19:37:54+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Bound a Relocated Cache Root's Growth Implementation Plan

**Verdict:** REVISE

The plan is disciplined. Retention rules are pure functions backed by decision
tables, seams follow the `StepObserver` precedent, the default-root warm path
is frozen by a tests-only commit, every acceptance criterion maps to a named
test, and destructive behaviour ships behind the verb before any cold path
triggers it. The verdict rests on one critical and a cluster of major issues.
All of them sit where automatic prune turns content under the root into
deletion targets. Pointer contents reach `remove_dir_all` unvalidated. The
orphan reap and the claim sweeps recognise names too loosely or follow
symlinks. The running version has no self-protection. Several structural
choices will also make this safety-critical code drift: two tree-prune
implementations, a scattered `CacheRootKind` regime, and a `retention` ↔
`tree` cycle.

### Cross-Cutting Themes

- **Prune deletes beyond the allowlist** (flagged by: security, safety). The
  plan's "every other name is kept" principle holds for top-level single-file
  names but not elsewhere:
  - pointer contents are joined into a path unvalidated (`resolver.rs:843-848`
    → `layout.rs:153`);
  - the orphan reap recognises a generation by the substring `-1-`
    (`reap.rs:281-285`);
  - `trees/claims/` and `claims/` sweeps have no name grammar;
  - `trees/`, `trees/claims/` and `.accelerator-lock-<p>` are followed through
    symlinks.

  Automatic cold-path prune turns each of these into an unattended deletion.
- **The running version is not self-protected** (flagged by: safety,
  correctness, compatibility, security). Tree prune spares the running
  launcher's own digest; single-file prune relies on a best-effort claim.
  `refresh`'s fast path skips the write for a young but foreign-owned claim
  that `list` then rejects, and `--older-than` below `CLAIM_WINDOW/10` makes
  the verb evict its own version. Built-in commands never refresh the claim at
  all.
- **A cold-path reap probes the process's own held lock** (flagged by:
  architecture, correctness, performance). `TreePrune` runs the root-wide
  `Reaper::reap_orphans` per artifact while the materialise still holds its
  own tree lock. `reap.rs:17-20` forbids exactly this on POSIX-emulated
  `flock` (NFS), where closing the probe releases the held lock. Running it
  per artifact is also O(A²) lock probes.
- **The retention regime is scattered and tree prune duplicated** (flagged by:
  architecture, code-quality). `CacheRootKind` is matched at five sites, with
  `RootPrune::run` also no-op-guarding `PluginBin`. `TreePrune` and
  `TreeResolver::prune`/`reclaim_unclaimed` implement the same keep rules and
  each receive the same layout gate and no-new-lock patch.
- **`retention` and `tree` depend on each other** (flagged by: architecture,
  code-quality). `SingleFlightLocks` lives in `retention` but returns `tree`
  types, and `tree` imports `retention`'s constants, patience and report.
- **Shim-copy removal leaves a lasting integrity notice** (flagged by: safety,
  compatibility). The work item accepts "fails that one bootstrap". But
  `fail_integrity` appends to `.accelerator-unverified.log`, and
  `hooks/launcher-link-refresh.sh:34-37` reports it on every later
  SessionStart. That durable consequence was not priced in.
- **Unbuildable or ineffective tests** (flagged by: test-coverage,
  architecture, compatibility). The `RemovalObserver` guard test has no
  observer on `TreePrune`. `test_a_build_without_seams_passes_no_feature_flag`
  breaks in Phase 6. The paused-prune store tests pass through `AGE_BACKSTOP`
  alone. The production `re_resolve` wiring is protected only by a defaulted
  `None`.
- **Silent failure and fail-open reads** (flagged by: safety, code-quality,
  compatibility). `claimed_within` returns `false` on any `read_dir` error, so
  an unreadable `trees/claims/` reclaims claimed trees. Claim-write failures,
  `Unreadable` causes and removal errors are all dropped, which leaves
  repeated eviction undiagnosable.
- **Root identity by path string** (flagged by: correctness, compatibility,
  performance). Comparing `realpath` strings misclassifies bind mounts and
  macOS firmlinks. Comparing `(st_dev, st_ino)` is cheaper and correct.
- **Bootstrap lock sweep** (flagged by: safety, correctness, compatibility,
  security). The sweep does not trim the `"$$\n"` pid. It does not reject
  pid ≤ 0. It does not re-check before unlinking, so it races the shell's own
  reclaim. It does not lstat the directory.

### Tradeoff Analysis

- **Bounded growth vs cold-path latency**: performance wants whole-tree
  `remove_dir_all` deferred or capped, because a first cold store after an
  upgrade can delete several 100 MB+ trees before `exec`. The work item
  accepts per-tree latency "about once per tree". Recommendation: keep tree
  deletion on the cold path, but rename a reclaimed generation to a reapable
  `.tmp-` trash name under its lock (O(1)) and let the residue reap remove it.
  That bounds pre-`exec` cost without slowing reclamation.
- **Simplicity vs operational safety**: safety suggests an environment opt-out
  for automatic prune and a `tracing` event per removal. The opt-out adds
  surface; the per-removal trace is cheap. Recommendation: add the trace and
  defer the opt-out.
- **Work-item decision vs durable alarm**: the work item rejected protecting
  shim copies. Keeping only the copy matching the running plugin's shim digest
  costs one hash on the cold path. Recommendation: keep the matching copy, or
  record the durable unverified-log consequence as a named accepted risk.

### Findings

#### Critical

- 🔴 **Security**: Pointer contents reach `remove_dir_all` unvalidated; `parse_generation` is looser than `validated_generation`
  **Location**: Phase 5 §2 step 1 and §3 (`reclaim_unclaimed`)
  A `.ref` holding `/home/user` or `../x` names a directory outside the root, and both prune paths would `remove_dir_all` it. `locate` uses the strict `validated_generation`; prune does not.

#### Major

- 🟡 **Safety**: Orphan reap's `-1-` substring test can recursively delete a foreign directory under `trees/`
  **Location**: Phase 5 §2 step 2
  A missing `.lease` probes `Free`, so any aged, unreferenced directory containing `-1-` goes. The orphan reap now runs on every cold path.
- 🟡 **Security**: Stale tree-claim sweep deletes old regular files wherever `trees/` or `trees/claims/` points
  **Location**: Phase 5 §2 (trees/claims sweep)
  There is no name grammar and no lstat/ownership check, so a symlinked `trees/claims` turns the sweep against any directory.
- 🟡 **Security**: Bootstrap-lock sweep follows a symlinked lock directory
  **Location**: Phase 4 §4
  `remove_file(pid)` follows `.accelerator-lock-<p>` if it is a symlink. `bin/accelerator:380` refuses that case.
- 🟡 **Security**: Tree-claim refresh still truncates through a planted symlink
  **Location**: Phase 3 §3 / Phase 5
  Only version claims gain `O_NOFOLLOW`. `tree/claims.rs` keeps `create(true).truncate(true)`.
- 🟡 **Security**: Release byte-scan likely cannot see a `#[no_mangle]` symbol in a stripped, LTO'd launcher
  **Location**: Phase 6 §3
  The release profile sets `strip = true` and `lto = "thin"`. `compile_error!` keys on `debug_assertions`, which flags can force on.
- 🟡 **Safety / Correctness / Compatibility**: The running version has no self-protection in single-file prune, and `--older-than` evicts versions in active use
  **Location**: Phase 3 §3, Phase 4 §2-3, §7
  Mirror tree prune's own-digest exemption, ownership-check `refresh`'s fast path, and refresh before the verb prunes.
- 🟡 **Safety**: Tree reclaim fails open when `trees/claims/` cannot be read
  **Location**: Phase 5 §2 step 1
  `claimed_within` returns `false` on any error (`claims.rs:61-63`). Give tree claims a `Read`/`Unreadable` listing.
- 🟡 **Safety / Compatibility**: Always removing shim copies yields a durable integrity notice
  **Location**: Phase 4 §2
  A race between glob/hash and use calls `fail_integrity`, and the SessionStart hook then reports the log indefinitely.
- 🟡 **Architecture / Correctness**: Per-artifact `reap_orphans` under a reused held lock probes the process's own tree lock
  **Location**: Phase 5 §2 step 2, Phase 6 §2
  On emulated `flock` this releases the materialise's lock. Run the reap once, under `held`.
- 🟡 **Correctness**: Orphan-lock removal ignores temp residue, so a lockless partial archive is never reclaimed
  **Location**: Phase 5 §1-2 step 4
  Require that no `Artifact::owning` residue remains, or discover residue in `of_tree_entry`.
- 🟡 **Architecture / Code Quality**: The retention regime is selected by `CacheRootKind` conditionals at five sites
  **Location**: Phases 3, 4 §7, 6 §1-2
  Choose a `RootRetention` strategy once at composition.
- 🟡 **Architecture / Code Quality**: Two tree-prune implementations will drift
  **Location**: Phase 5 §2-3
  Parameterise `TreePrune` by `ArtifactScope::{PinnedOnHost, Discovered}` and delete `reclaim_unclaimed`.
- 🟡 **Architecture**: Bidirectional dependency between `retention` and `tree`
  **Location**: Phase 4 §5, Phase 5 §2
  Move the lock seam to `tree/` and give `TreePrune` its own report.
- 🟡 **Code Quality**: Prescribed doc comments embed work-item and plan references
  **Location**: Phase 2 §1, Phase 3 §2, Phase 4 §3, Phase 5 §2, Phase 6 §3
  The comments in question are "the 0339 contract", "the Accepted Risk", the Cargo.toml comment, and the module-pointer doc.
- 🟡 **Code Quality**: Seam ownership and injection style is inconsistent
  **Location**: Phase 1 §1, Phase 3 §3, Phase 6 §1
  Ports arrive through boxed builders, `ResolverConfig` and borrowed `RetentionPorts`. A recorder swapped in may miss prune calls.
- 🟡 **Performance**: Unbounded recursive tree removal runs synchronously before `exec`
  **Location**: Phase 6 §1, Performance Considerations
  Several 100 MB+ trees may go on the first post-upgrade hook dispatch.
- 🟡 **Test Coverage**: Paused-prune store tests pass through `AGE_BACKSTOP` alone
  **Location**: Phase 4 §8
  Advance the clock past the backstop, and add a no-claim control.
- 🟡 **Test Coverage**: Recorded warm-path tests check the test's own composition
  **Location**: Phase 3 §4
  `main.rs` wires `identify(…, &SystemFilesystem)`. Extract the dispatch composition and record through it.
- 🟡 **Test Coverage**: Production `re_resolve` wiring is protected only by a defaulted `None`
  **Location**: Phase 2 §2
  Drop the default or add a `main.rs` test.
- 🟡 **Test Coverage / Compatibility**: Phase 6 breaks `test_a_build_without_seams_passes_no_feature_flag`; the new Python test is redundant
  **Location**: Phase 6 §3
- 🟡 **Test Coverage / Architecture**: The lock-removal guard test has no observer to record through
  **Location**: Phase 5 §4
- 🟡 **Test Coverage**: "Both roots" safety tests run before root kinds exist and never re-run with prune active
  **Location**: Phase 2 §3
- 🟡 **Test Coverage**: Claim re-check, last-claim guard and removal-error branches are untested
  **Location**: Phase 4 §3 step 5, Phase 5 §2
- 🟡 **Compatibility**: Claim refresh relies on `O_TRUNC` of an empty file advancing mtime
  **Location**: Phase 3 §3
  Set the mtime explicitly and test it on both OS legs.
- 🟡 **Compatibility**: No CHANGELOG entry for user-visible behaviour changes
  **Location**: Phase 7

#### Minor

- 🔵 **Security**: The key override and http relaxation are independent; tie both to a parsed loopback host — Phase 6 §3
- 🔵 **Security**: `refresh` trusts a foreign-owned claim on its fast path — Phase 3 §3
- 🔵 **Safety**: Stale-claim removal is not restricted to the claim-name grammar — Phase 4 §3, Phase 5 §2
- 🔵 **Safety**: The fail-safe meaning of `modified_seconds: None` is unspecified — Phase 1 §1, Phase 4 §2
- 🔵 **Safety**: An `ENOENT` from an existing but unrunnable binary re-downloads and re-prunes on every dispatch — Phase 2 §1
- 🔵 **Safety / Correctness / Compatibility**: The bootstrap-lock sweep races the shell's reclaim; there is no pid trim and no pid ≤ 0 rule — Phase 4 §4
- 🔵 **Correctness**: A crash between the two unlinks leaves residue the classifier keeps forever — Phase 4 §3 step 4
- 🔵 **Correctness / Compatibility**: Built-in commands never refresh the version claim; wget may set an old mtime — Phase 3 §3
- 🔵 **Compatibility**: Pre-claim versions are evicted after every claim-aware prune, not once — Migration Notes
- 🔵 **Compatibility**: The verb's output count and exit contract are undefined for removal failures — Phase 4 §7
- 🔵 **Compatibility**: A launcher entry removed before `exec` breaks `--fail-safe` (no `execfail`) — Phase 6
- 🔵 **Architecture / Code Quality**: Public `HeldTreeLock::from_guard` weakens the proof type — Phase 5 §2
- 🔵 **Architecture / Code Quality**: The filesystem port is defined in full before tests demand it — Phase 1 §1
- 🔵 **Architecture**: `ResolverConfig` starts carrying behavioural collaborators — Phase 3 §3, Phase 6 §1
- 🔵 **Code Quality**: Best-effort paths fail silently — Phase 3 §3, Phase 6 §1
- 🔵 **Code Quality**: `TreePrune::run` packs a five-row lock table and five steps into one method — Phase 5 §2
- 🔵 **Performance**: Tree prune repeats root-wide passes per artifact — Phase 5 §2
- 🔵 **Performance**: The relocated warm-path cost is understated; `identify` runs more than once per dispatch — Phase 3 §2
- 🔵 **Performance**: `launcher_id` is computed on every dispatch, outside the recorded port — Phase 3 §3
- 🔵 **Performance**: Materialise holds its own tree lock across the whole root prune — Phase 6 §2
- 🔵 **Performance**: The measurement gates nothing; the only timing assertion is a 10 s guard — Phase 7 §3
- 🔵 **Test Coverage**: Several lock-patience table rows have no test — Phase 5 §2
- 🔵 **Test Coverage**: `refresh`'s directory-safety checks and parsing negatives have no test — Phase 3 §4
- 🔵 **Test Coverage**: No exactly-at-threshold boundary tests — Testing Strategy
- 🔵 **Test Coverage**: The unparseable/missing pid branches and the real `KillZeroProbe` have no test — Phase 4 §4
- 🔵 **Test Coverage**: Cold-path variants of the foreign/kept-names criteria are not named — Phase 6 §4
- 🔵 **Test Coverage**: The trust-relaxing seams and `assert_hermetic` widening have no negative tests — Phase 6 §3

#### Suggestions

- 🔵 **Correctness / Compatibility**: Compare root identity by `(st_dev, st_ino)` — Phase 3 §2
- 🔵 **Correctness**: Map `wait_existing`'s `KeptBeingReplaced` to `Err` — Phase 4 §5
- 🔵 **Architecture**: Make the root-lock/tree-lock ordering rule explicit and structural — Phase 4 §5, Phase 5 §2
- 🔵 **Safety**: Opt-out and audit trail for automatic deletion — Phase 6
- 🔵 **Compatibility**: Coherence test pinning bash-produced names against the classifier — Phase 4 §1
- 🔵 **Code Quality**: Rename `RootEntry::Kept`; replace `disposition`'s boolean pair with enums — Phase 4 §1-2
- 🔵 **Code Quality**: Disambiguate `ProcessLiveness` from `LivenessProbe` — Phase 4 §4
- 🔵 **Test Coverage**: The end-to-end scenario should assert exit status, stub output and a single fetch — Phase 6 §4

### Strengths

- ✅ Retention rules are pure, exhaustively tested functions with decision tables mirroring `reap.rs`.
- ✅ The default-root warm path is frozen by a tests-only commit against today's resolver.
- ✅ Every acceptance criterion maps to a named test and phase.
- ✅ Phase ordering puts protections (call list, re-resolve, claims) before destruction, and the verb before cold paths.
- ✅ List-then-read-claims, remove-only-listed and claim-before-find give a sound concurrent-store argument.
- ✅ Lock patience is deadlock-free: cold paths only try, and the verb waits while holding no tree lock.
- ✅ `try_single_flight`'s `Ok(None)`/`Err` split is reused instead of adding a new error taxonomy.
- ✅ `re_resolve` is a defaulted method, bounded to one retry, and skips `find`.
- ✅ An unresolvable override fails safe as `PluginBin`, and an `Unreadable` version-claims directory blocks eviction.
- ✅ Unknown names, platforms and layouts are kept, and name grammars are pinned across versions.
- ✅ Seams are zero-sized trait objects in the `StepObserver` style, so they cost nothing on the hot path.
- ✅ All `#[cfg(feature)]` arms are confined to `main.rs`, following the jira/linear guard precedent.

### Recommended Changes

1. **Validate every pointer before joining** (addresses: Pointer contents reach `remove_dir_all` unvalidated)
   In Phase 5, route every pointer-to-generation step in both root kinds through `validated_generation` with the pointer's own artifact, platform and digest, trying each layout for the other-layout rule. Add both-root tests for an absolute path, `../`, and another artifact's generation name.
2. **Make every tree-side recognition strict and non-following** (addresses: orphan reap `-1-`, tree-claim sweep, bootstrap-lock symlink, tree-claim truncation, claim grammar)
   - Make the orphan reap use `parse_generation`'s strict grammar.
   - Require `trees/`, `trees/claims/`, `claims/` and `.accelerator-lock-<p>` to lstat as euid-owned directories.
   - Remove only names that parse as claims.
   - Route tree-claim writes through the `O_NOFOLLOW` `write_private`.
   - Extend `foreign_tree_names_survive` and `foreign_names_survive_the_verb` with each case.
3. **Self-protect the running version** (addresses: running version not protected, `--older-than`, `refresh` fast path, built-ins never claim)
   - Treat the running version as always fresh in `SingleFilePrune`.
   - Ownership-check `refresh`'s fast path.
   - Refresh the claim at launcher start for every command in a relocated root.
   - Decide whether to clamp `--older-than` for version claims.
4. **Fail closed on unreadable tree claims** (addresses: tree reclaim fails open)
   Give tree claims a `Read`/`Unreadable` listing. On `Unreadable`, skip unclaimed-tree reclaim and the stale-claim sweep.
5. **Run the orphan reap once, under the held lock** (addresses: self-probe, O(A²) probes)
   Index `trees/` and `trees/claims/` once per prune. Add a guard test asserting that no held lock is ever probed.
6. **Unify tree prune and choose the regime once** (addresses: two implementations, scattered `CacheRootKind`, `retention` ↔ `tree` cycle, `from_guard`, injection inconsistency)
   - Introduce a `RootRetention` port with `PluginBinRetention` and `RelocatedRetention`, chosen in `main.rs`.
   - Parameterise `TreePrune` by scope and delete `reclaim_unclaimed`.
   - Move `SingleFlightLocks` into `tree/`, returning `HeldTreeLock` directly.
   - Keep `ResolverConfig` value-only.
7. **Handle orphan-lock residue** (addresses: lockless partial archive)
   In step 4, require that no temp residue owned by the artifact remains.
8. **Resolve the shim-copy decision** (addresses: durable integrity notice)
   Keep the copy matching the running plugin's shim digest, or record the unverified-log consequence as a named accepted risk.
9. **Fix the tests that cannot work** (addresses: guard test observer, dev_builds break, paused-prune backstop, recorded composition, `re_resolve` wiring, both-roots timing, untested branches)
   - Give `TreePrune` a removal observer.
   - Rework the no-seam dev_builds test to use `accelerator-vcs`.
   - Advance the clock past `AGE_BACKSTOP` in the paused-prune tests.
   - Record through an extracted dispatch composition.
   - Drop `re_resolve`'s default.
   - Parameterise the safety tests by kind from Phase 3.
   - Add a scriptable filesystem for the failure and refresh branches.
10. **Harden the release guard** (addresses: byte-scan blind to stripped symbol, independent relaxations)
    Use a marker that survives strip and LTO, add a build test that forces debug-assertions in release, and gate both relaxations on one parsed loopback host.
11. **Bound pre-`exec` deletion cost** (addresses: unbounded removal, materialise lock hold time)
    Rename a reclaimed generation to `.tmp-` trash under its lock and let the reap remove it. Release the held lock before pruning other artifacts.
12. **Pin the residue-sweep contract** (addresses: pid format, sweep race)
    Trim the pid, reject pid ≤ 0, re-read before unlinking, and seed fixtures with bash's exact bytes.
13. **Remove referencing comments and the speculative port surface** (addresses: comments, filesystem port YAGNI)
    Strip "0339"/"Accepted Risk"/pointer comments, and grow `CacheFilesystem` per phase.
14. **Document honestly** (addresses: CHANGELOG, migration wording, verb contract, `mtime` refresh)
    - Add `[Unreleased]` entries.
    - Correct "re-fetches them once".
    - Define the verb's count and its removal-failure exit status.
    - Set the claim's mtime explicitly with a cross-OS test.
15. **Smaller items**: compare `(st_dev, st_ino)`; identify once per process; compute `launcher_id` lazily; map `KeptBeingReplaced` to `Err`; add per-removal `tracing`; re-resolve only when the path is truly missing; add `execfail` in the bootstrap.

## Per-Lens Results

### Architecture

**Summary**: Follows the launcher's hexagonal shape with trait-object seams, pure tables and a defaulted port method. Its structural weaknesses are a `retention` ↔ `tree` cycle, regime checks scattered across five sites, two tree-prune implementations, and a per-artifact root-wide `Reaper` that breaks reap.rs's never-probe-your-own-lock invariant.

**Strengths**: trait-object seams with production adapters; functional core and imperative shell; defaulted `re_resolve`; reuse of `try_single_flight`'s contention split; test trust confined to `main.rs`; protections ordered before destruction.

**Findings**:
- 🟡 major / high — **Bidirectional dependency between `retention` and `tree`** (Phase 4 §5, Phase 5 §2, Phase 6 §2). Each module imports the other's types. Pick one direction: move the lock seam and constants to `tree/` or a neutral module, and give `TreePrune` its own report.
- 🟡 major / high — **Retention regime decided by scattered kind checks** (Phases 3, 4 §7, 6 §1-2). Five sites match `CacheRootKind`, and both resolvers carry `RetentionPorts`. Use a `RootRetention` port chosen in `main.rs`.
- 🟡 major / medium — **Root-wide Reaper run per artifact while another tree lock is held** (Phase 5 §2 step 2, Phase 6 §2). The Reaper probes M's held lock, which `reap.rs:17-20` forbids. Run it once under `held`, or pass the full held set.
- 🟡 major / high — **Two tree-prune implementations** (Phase 5 §2-3). Parameterise `TreePrune` with `ArtifactScope` and delete `reclaim_unclaimed`.
- 🔵 minor / medium — **Public `from_guard` weakens `HeldTreeLock`** (Phase 5 §2). Return `HeldTreeLock` from `take_tree`, or keep `from_guard` crate-private and derive the path.
- 🔵 minor / medium — **`TreePrune` has no removal seam for the guard test** (Phase 5 §4).
- 🔵 minor / medium — **Filesystem port defined up front, contradicting the plan's own seam rule; tree side stays on `std::fs`** (Phase 1 §1).
- 🔵 minor / medium — **`ResolverConfig` carries behavioural collaborators** (Phase 3 §3, Phase 6 §1).
- 🔵 suggestion / medium — **Root-lock/tree-lock ordering invariant is implicit** (Phase 4 §5, Phase 5 §2). Scope `HeldRootLock` to the single-file block.

### Code Quality

**Summary**: Well-structured, with domain vocabulary, pure rules and precedent-following seams. The risks are structural: regime branching across call sites, two parallel tree prunes, inconsistent seam ownership, doc comments that reference work items, an overbuilt Phase 1 port, a weakened proof type, and silent best-effort failures.

**Strengths**: pure rules with tables; domain vocabulary table; `LockPatience` reuse; small `run_external` change; trait-object seams; cfg arms confined to `main.rs`.

**Findings**:
- 🟡 major / high — **Regime selected by `CacheRootKind` conditionals at many sites** (Phase 3 §3, Phase 4 §7, Phase 6 §1-2). Inject a `RetentionRegime` with `claim_version` and `after_cold_path`.
- 🟡 major / high — **Two parallel tree-prune implementations will drift** (Phase 5 §2-3).
- 🟡 major / high — **Prescribed doc comments embed work-item/plan references or restate code** (Phase 2 §1, Phase 3 §2, Phase 4 §3, Phase 5 §2). Keep only reference-free *why*.
- 🟡 major / medium — **Seam ownership and injection style inconsistent** (Phase 1 §1, Phase 3 §3, Phase 6 §1). Use one filesystem and ownership instance per resolver.
- 🔵 minor / medium — **Filesystem port defined in full before tests demand it** (Phase 1 §1).
- 🔵 minor / medium — **Public `from_guard` weakens the proof type** (Phase 5 §2).
- 🔵 minor / medium — **Mutual module dependency `retention` ↔ `tree`** (Phase 4 §5, Phase 5 §2).
- 🔵 minor / medium — **Best-effort paths fail silently** (Phase 3 §3, Phase 6 §1). Log at debug/warn level, and carry the cause in `Unreadable`.
- 🔵 minor / medium — **`TreePrune::run` packs the lock table and five steps into one method** (Phase 5 §2). Model `ArtifactLock::{Reused, Taken, LockFree, Skipped, Failed}`.
- 🔵 suggestion / medium — **`RootEntry::Kept` names a disposition; boolean pair in `disposition`** (Phase 4 §1-2).
- 🔵 suggestion / low — **`ProcessLiveness` vs `LivenessProbe` naming** (Phase 4 §4, §7).

### Test Coverage

**Summary**: Very test-forward, with a full criterion map, exhaustive table tests and seams for every non-deterministic input. Several safety-critical tests could pass against broken code, and a few cannot work as written.

**Strengths**: the criterion map; the fixed list frozen first; seams for uid, pid, lease, clock, lock and pause; pure rules tested row by row; ordering asserted on recorded calls; `within` bounds; the end-to-end test through the real bootstrap.

**Findings**:
- 🟡 major / high — **Paused-prune store tests pass through the age backstop alone** (Phase 4 §8). Advance the clock past `AGE_BACKSTOP` inside `CLAIM_WINDOW`, and add a no-claim control.
- 🟡 major / high — **Recorded warm-path tests check the test's own composition** (Phase 3 §4). Extract the dispatch composition, add a no-direct-`std::fs` guard, and define how out-of-root paths are recorded.
- 🟡 major / high — **Production `re_resolve` wiring protected only by a defaulted `None`** (Phase 2 §2).
- 🟡 major / high — **Existing dev_builds test will break; the new Python test is redundant** (Phase 6 §3). Also add a `_assert_no_test_loopback` rejection test.
- 🟡 major / high — **Lock-removal guard test has no observer** (Phase 5 §4). Add an observer that probes from a separate open file description, as `ProbingLockOf` does.
- 🟡 major / medium — **"Both roots" tests scheduled before root kinds exist and never re-run with prune** (Phase 2 §3).
- 🟡 major / medium — **Claim-removal safety branches and filesystem error paths untested** (Phase 4 §3 step 5, Phase 5 §2). Add a scriptable filesystem decorator.
- 🔵 minor / medium — **Lock-patience rows untested; `of_tree_entry`/`parse_generation` not in the exhaustive list** (Phase 5 §2).
- 🔵 minor / medium — **`refresh` directory-safety and parsing negatives untested** (Phase 3 §4).
- 🔵 minor / medium — **No exactly-at-threshold tests** (Testing Strategy).
- 🔵 minor / medium — **Missing/unparseable pid and real `KillZeroProbe` untested** (Phase 4 §4).
- 🔵 minor / medium — **Cold-path variants of foreign/kept names not named** (Phase 6 §4).
- 🔵 minor / medium — **No negative tests for trust-relaxing seams or `assert_hermetic`** (Phase 6 §3).
- 🔵 suggestion / low — **End-to-end asserts only the end state** (Phase 6 §4).

### Correctness

**Summary**: Ordering is mostly sound and lock patience is deadlock-free. The tree-prune phases introduce a self-probe of a held lock and lockless residue that leaks permanently, and single-file prune lacks the running-version exemption.

**Strengths**: the list-then-claims argument; deadlock-free patience; bounded `find`-skipping re-resolve; fail-safe identity; claim re-check and `Unreadable`; the layout gate.

**Findings**:
- 🟡 major / medium — **Tree prune under a reused held lock makes `reap_orphans` probe the process's own tree lock** (Phase 5 §2 step 2, Phase 6 §2).
- 🟡 major / medium — **Orphan-lock removal ignores temp residue; a lockless partial archive is never reclaimed** (Phase 5 §1-2 step 4).
- 🟡 major / medium — **No running-version exemption in single-file prune** (Phase 4 §2-3).
- 🔵 minor / medium — **A crash between the two unlinks leaves residue kept forever** (Phase 4 §1, §3 step 4).
- 🔵 minor / medium — **Built-ins never claim; wget's `Last-Modified` mtime can defeat the pre-claim grace** (Phase 3 §3, Phase 4 §2).
- 🔵 minor / low — **The bootstrap-lock sweep can remove a re-created live lock** (Phase 4 §4).
- 🔵 suggestion / low — **`wait_existing`'s `KeptBeingReplaced` mapping unspecified** (Phase 4 §5).
- 🔵 suggestion / low — **Canonical-path comparison misclassifies bind mounts** (Phase 3 §2).

### Safety

**Summary**: Careful about deletion: allowlist, `Unreadable` fails safe, staged rollout, re-resolve recovery. Four gaps remain: the loose orphan reap, always deleting the in-use shim copy, no self-protection when the claim fails, and fail-open tree claims.

**Strengths**: strict top-level allowlist; `Unreadable` blocks eviction; the concurrent-store ordering; an unresolvable override is `PluginBin`; cold paths only try; `.minisig` removed first and rename-over replacement; the verb ships before cold paths; release guard on test-loopback.

**Findings**:
- 🟡 major / high — **Orphan reap's loose generation test can recursively delete a foreign directory** (Phase 5 §2 step 2).
- 🟡 major / high — **Always deleting shim copies removes the in-use verifier and causes false integrity alarms** (Phase 4 §2).
- 🟡 major / medium — **Running version not self-protected when its claim silently fails** (Phase 3 §3, Phase 4 §3). Consider flooring `--older-than` at `AGE_BACKSTOP`.
- 🟡 major / high — **Tree reclaim fails open when `trees/claims/` cannot be read** (Phase 5 §2 step 1).
- 🔵 minor / medium — **Stale-claim removal not restricted to the claim-name grammar** (Phase 4 §3, Phase 5 §2).
- 🔵 minor / medium — **Fail-safe meaning of an unavailable mtime unspecified** (Phase 1 §1, Phase 4 §2). `None` should mean not aged and still fresh; a future mtime counts as age zero.
- 🔵 minor / medium — **`ENOENT` re-resolve repeats for an existing but unrunnable binary** (Phase 2 §1). Re-resolve only when `!exists(program)`.
- 🔵 minor / low — **Bootstrap-lock sweep races the shell's reclaim** (Phase 4 §4).
- 🔵 suggestion / medium — **No opt-out or audit trail for automatic deletion** (Phase 6, Migration Notes).

### Security

**Summary**: Deliberate on claims, the allowlist and `O_NOFOLLOW`. Three gaps let attacker-writable content under the root become deletion targets: unvalidated pointer joins, an unchecked `trees/claims/` sweep, and symlink-following lock and tree-claim handling. Automatic prune makes each one triggerable without user action. The release byte-scan is probably blind to a stripped symbol.

**Strengths**: ownership- and symlink-checked version claims and fail-safe `Unreadable`; the allowlist plus listed-only removal; `O_NOFOLLOW`/0600 writes; EPERM treated as alive; orphan locks unlinked only while held; the test-loopback guard pattern.

**Findings**:
- 🔴 critical / high — **Pointer contents reach `remove_dir_all` unvalidated; `parse_generation` looser than `validated_generation`** (Phase 5 §2 step 1, §3). Verified against `resolver.rs:786-852` and `layout.rs:153`.
- 🟡 major / high — **The stale tree-claim sweep deletes any old regular file wherever `trees/` or `trees/claims/` points** (Phase 5 §2). Use lstat and ownership checks, the name grammar, and `openat`/`unlinkat`.
- 🟡 major / medium — **The bootstrap-lock sweep follows a symlinked lock directory** (Phase 4 §4).
- 🟡 major / high — **Tree-claim refresh truncates through a planted symlink** (Phase 3 §3, Phase 5).
- 🟡 major / medium — **The byte-scan backstop likely cannot see the marker in a stripped, LTO'd launcher** (Phase 6 §3). Use a surviving string or `#[used]`, and add a forced-debug-assertions release build test.
- 🔵 minor / medium — **Tie the key override and the http relaxation to one parsed loopback host** (Phase 6 §3).
- 🔵 minor / medium — **`refresh`'s fast path trusts a foreign-owned claim** (Phase 3 §3). This enables a targeted repeated-eviction DoS.

### Compatibility

**Summary**: Cross-version contracts are mostly well handled: unknown names, platforms and layouts are kept, grammars are pinned, the warm path is frozen, and the trait method is defaulted. The weak points sit where new Rust behaviour meets the unchanged bash bootstrap and the existing CLI contract.

**Strengths**: additive `re_resolve`; frozen default-root list; keep-unrecognised; grammar pin test; reuse of 0339's probe; test-loopback copies the jira/linear guard; ENOLCK degrades to skip.

**Findings**:
- 🟡 major / medium — **Removing every shim copy breaks the unchanged bash bootstrap of every live version** (Phase 4 §2). The notice is durable, through `hooks/launcher-link-refresh.sh`.
- 🟡 major / low — **Claim refresh relies on `O_TRUNC` of an empty file advancing mtime** (Phase 3 §3). Set it with `File::set_modified`/`futimens` and test on both OS legs.
- 🟡 major / medium — **Widened `--older-than` evicts versions in active use, including the verb's own** (Phase 4 §7).
- 🟡 major / high — **No CHANGELOG entry for user-visible behaviour changes** (Phase 7).
- 🔵 minor / high — **Pre-claim versions are evicted after every claim-aware prune, not once** (Migration Notes, Accepted Risks).
- 🔵 minor / medium — **Built-in commands never refresh the version claim** (Phase 3 §3).
- 🔵 minor / medium — **Bash pid format `"$$\n"` not specified for the parser; pid ≤ 0** (Phase 4 §4).
- 🔵 minor / low — **Lock-directory removal races the bootstrap's reclaim-and-reacquire** (Phase 4 §4).
- 🔵 minor / medium — **Making the launcher loopback-seamed breaks an existing build-system test** (Phase 6 §3). Also update `tasks/README.md:254`.
- 🔵 minor / medium — **The verb's output and exit contract changes without a defined count or error reporting** (Phase 4 §7).
- 🔵 minor / medium — **A launcher entry removed before `exec` breaks `--fail-safe`** (Phase 6). Add `shopt -s execfail` before the final `exec`.
- 🔵 suggestion / medium — **Compare root identity by `(st_dev, st_ino)`** (Phase 3 §2). Covers bind mounts and macOS firmlinks.
- 🔵 suggestion / medium — **Coherence test for bash-produced names** (Phase 4 §1).

### Performance

**Summary**: The default-root warm path stays essentially free, and the seams cost nothing. The real cost is cold-path work before `exec`: unbounded synchronous tree removal and redundant per-artifact passes over `trees/`. Neither is measured, because `measure:warm-dispatch` refuses `ACCELERATOR_*` overrides.

**Strengths**: no canonicalisation when the override is unset; throttled claim refresh; `Try` avoids a thundering herd; one listing shared by single-file prune and the sweep; zero-sized adapters.

**Findings**:
- 🟡 major / medium — **Unbounded recursive tree removal runs synchronously before `exec`** (Phase 6 §1). Cap removals, rename to trash, or limit tree deletion to materialise; time it manually.
- 🔵 minor / high — **Tree prune repeats root-wide passes per artifact** (Phase 5 §2). Index once per prune.
- 🔵 minor / medium — **The relocated warm-path cost is understated** (Phase 3 §2). `identify` runs up to twice per dispatch, `realpath` walks every component, and the `(dev, ino)` comparison is cheaper.
- 🔵 minor / medium — **`launcher_id` computed on every dispatch** (Phase 3 §3). Compute it lazily, only for `Relocated`.
- 🔵 minor / medium — **Materialise holds its own tree lock across the whole root prune** (Phase 6 §2). Release it before pruning other artifacts.
- 🔵 minor / high — **The measurement gates nothing; the only timing assertion is a 10 s guard** (Phase 7 §3).

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-10-20

**Verdict:** REVISE

The plan was revised in 15 agreed steps. All eight lenses were re-run. There are no critical findings. Nine distinct new or remaining majors cross the three-major threshold. Most are local: missing wiring detail, tests that would not compile or would pass vacuously, and one keep rule that misses roots shared across platforms. None of the first review's structural problems remains.

### Previously Identified Issues

- ✅ **Security**: Pointer contents reach `remove_dir_all` unvalidated (critical) — Resolved
- ✅ **Security**: Tree-claim sweep follows a symlinked `trees/claims` — Resolved
- ✅ **Security**: Bootstrap-lock sweep follows a symlinked lock directory — Resolved
- ✅ **Security**: Tree-claim refresh truncates through a symlink — Resolved
- ✅ **Security**: Byte scan blind to a stripped marker — Resolved (proven on host target; cross-target is a suggestion)
- ✅ **Security**: Test key and http relaxation independent — Resolved
- ✅ **Safety**: Orphan reap's loose generation test — Resolved
- 🟡 **Safety / Compatibility**: Shim-copy removal causes durable integrity notice — Partially resolved (host platform only)
- ✅ **Safety / Correctness / Compatibility**: Running version not self-protected; `--older-than` evicts it — Resolved
- ✅ **Safety**: Tree reclaim fails open on unreadable `trees/claims/` — Resolved
- ✅ **Architecture / Correctness**: Reaper probes the process's own held lock — Resolved
- ✅ **Correctness**: Orphan-lock removal strands partial archives — Resolved
- 🟡 **Architecture / Code Quality**: `retention` ↔ `tree` cycle — Partially resolved (`PruneSteps`, `Ownership`, `is_trusted_directory`, failure types still point the wrong way)
- ✅ **Architecture / Code Quality**: Regime scattered across `CacheRootKind` checks — Resolved
- ✅ **Architecture / Code Quality**: Two tree-prune implementations — Resolved
- ✅ **Architecture / Code Quality**: Public `from_guard` — Resolved
- 🟡 **Architecture / Code Quality**: Inconsistent injection; tree side off the ports — Partially resolved
- ✅ **Code Quality**: Comments citing work items — Resolved
- ✅ **Performance**: Pre-`exec` tree removal — Resolved as an accepted, documented tradeoff
- ✅ **Performance**: Per-artifact root-wide passes; duplicate `identify`; eager `launcher_id`; lock hold time — Resolved
- 🟡 **Test Coverage**: Recorded tests check a test-assembled composition — Still present
- ✅ **Test Coverage**: Paused-prune backstop; `re_resolve` default; dev_builds break; guard observer — Resolved
- 🟡 **Test Coverage**: Defensive branches untested — Partially resolved (`ScriptedFilesystem` trigger wrong)
- ✅ **Compatibility**: O_TRUNC mtime; CHANGELOG; built-ins claim; pid format; verb contract; `execfail`; `(dev, ino)` — Resolved

### New Issues Introduced

- 🟡 **Compatibility / Test Coverage**: Phase 5's "pass unchanged" safety net calls the deleted `TreeResolver::prune` (17 call sites). `cache_prune_reclaims_owned_orphans_and_keeps_unowned_ones` expects a reap that the lock-free arm no longer does. Verified against `tests/tree_resolution.rs`.
- 🟡 **Compatibility** (minors from Safety and Correctness): Other platforms' current shim copies are removed in cross-platform roots. The accepted-risk wording is inaccurate.
- 🟡 **Test Coverage**: The end-to-end "exactly the item's list" omits the shim copy that prune now keeps.
- 🟡 **Test Coverage** (minor from Code Quality): `ScriptedFilesystem` fires on the first `entry`, so the re-check tests pass vacuously or never pass. It cannot reach tree prune.
- 🟡 **Test Coverage**: No test catches reading claims before listing the root.
- 🟡 **Architecture / Code Quality**: `tree/` still imports `retention/` types.
- 🟡 **Architecture / Code Quality**: Tree prune bypasses `CacheFilesystem`/`Ownership`. `RetentionPorts` lacks `liveness`, `pins` and `removals`. `PluginBinRetention` is a unit struct but needs tree collaborators.
- 🟡 **Performance**: The release-guard test adds uncached release builds to `test:integration:tasks` on both OS legs. Use one profile environment, its own leaf, and its own `CARGO_TARGET_DIR` (also a Safety suggestion).
- 🔵 **Correctness**: The lock-only table row yields a bare `SingleFlight`, but steps 2 and 4 need `HeldTreeLock`.
- 🔵 **Correctness**: `a_retired_partial_archive_goes_and_then_its_lock` contradicts the step order, because one prune removes both.
- 🔵 **Correctness**: The tree-claims index is read once per run, which widens the claim-to-removal gap.
- 🔵 **Safety / Correctness**: `.tmp-launcher-$$` aged by wget's `Last-Modified` can be swept mid-fetch. Gate it on pid liveness.
- 🔵 **Security**: The relocated root itself is not ownership-checked. `prune_held` does not state the `trees/` gate.
- 🔵 **Security**: Pointer files are read unbounded; a FIFO or device symlink can stall cold paths.
- 🔵 **Compatibility**: The CHANGELOG Breaking entry misdescribes the exit-code change and omits `execfail`, `ENOENT` re-resolve and fail-closed default-root changes.
- 🔵 **Code Quality / Architecture**: `ArtifactScope` and `Bookkeeping` always co-vary. The orphan reap is listed as a per-artifact step. `for_root` uses `RetentionPorts` a phase early. `is_protected` returns `bool`.
- 🔵 **Architecture**: The default root's post-materialise reap moved out of `tree/` and now depends on wiring. `SingleFlightLocks` keeps a path route to tree locks.
- 🔵 **Test Coverage**: Keep-only tests lack a positive control. Phase 7 re-runs are not scheduled. There is no `require_https` negative test and no version-claims-directory refusal test.
- 🔵 **Performance**: The accepted removal cost usually lands in a SessionStart hook. Time that path.
- 🔵 Suggestions: cross-target marker proof; `execfail` makes every exec failure silent under `--fail-safe`; `--older-than` above 14 days must widen; the `identify` error selects `PluginBinRetention`; a `ReResolution` enum; the "only `main.rs` matches on kind" rule should name `for_root`; split the near-miss shim-copy test cases.

### Assessment

The plan has moved from structurally unsafe to structurally sound. Pointer containment, fail-closed reads, self-protection, single-lock discipline and the single tree-prune implementation all hold. What remains is concentrated in three areas:
- **Wiring precision.** Where tree-side collaborators and shared types live, and making tree prune use the same filesystem port.
- **Test buildability and validity.** Migrating the default-root regression net, the `ScriptedFilesystem` trigger, the end-to-end expected set, and the claims-before-listing ordering test.
- **One keep rule.** Shim copies across platforms.

A third pass should need only the architecture, code-quality and test-coverage lenses once these are addressed.

## Re-Review (Pass 3) — 2026-10-21

**Verdict:** REVISE

Pass 2's structural and test fixes were applied directly, and six design decisions were taken one at a time. All eight lenses were re-run. There are no critical findings. Ten distinct majors remain, so the verdict stays REVISE. Most sit where two pass-2 decisions meet real environments: the root-safety gate and the bootstrap `execfail` tail. The rest are wiring gaps left by the regime and port refactor.

### Previously Identified Issues

- ✅ **Architecture / Code Quality**: `tree/` imports `retention/` types — Resolved (leaf modules; tree-owned ports and report types)
- 🟡 **Architecture / Code Quality**: Tree side off the ports; `RetentionPorts` incomplete — Partially resolved (`TreePrune` is on the port; tree-claims `list`, `read_pointer`, `TreeResolver` and `Reaper` are not)
- ✅ **Compatibility / Safety / Correctness**: Cross-platform shim copies removed — Resolved (per-platform `RunningShims`)
- 🟡 **Compatibility / Test Coverage**: Phase 5 regression net calls the deleted API — Partially resolved (two-commit helper planned; its signature cannot carry the seams, and one test blocks under `Wait`)
- ✅ **Test Coverage**: End-to-end omits the kept shim copy — Resolved
- ✅ **Test Coverage / Code Quality**: `ScriptedFilesystem` trigger — Resolved (nth call plus `fired()`; one timing gap below)
- ✅ **Test Coverage**: Claims-before-listing ordering — Resolved
- 🟡 **Test Coverage**: Recorded composition — Partially resolved (`start_up` extracted; resolver wiring still duplicated in the tests)
- 🟡 **Performance / Safety**: Release-guard CI cost and isolation — Partially resolved (own leaf and target dir, but that dir is never cached)
- ✅ **Correctness**: Lock-only row yields a bare guard — Resolved (`TreeLocks`)
- 🟡 **Correctness**: Retired-archive test contradicts the step order — Partially resolved (the reap stage's position relative to step 3 is unspecified)
- ✅ **Correctness**: Tree-claims index read once — Resolved (re-read before removal)
- ✅ **Safety / Correctness**: wget-stamped staging swept mid-fetch — Resolved on a single host (pid liveness)
- ✅ **Security**: Root ownership unchecked; unbounded pointer reads — Resolved in intent (see the root-gate gaps and the port gap below)
- ✅ **Compatibility**: `execfail` silences every failure; `--older-than` widening; CHANGELOG gaps — Resolved in design (see the bash 3.2 finding below)
- ✅ **Code Quality**: Co-varying enums; reap as a per-artifact step; `is_protected` bool; `ReResolution` — Resolved

### New Issues Introduced

- 🟡 **Compatibility**: Bash 3.2 returns 126, not 127, for an `exec` of a missing file, so the decision-6 tail never reaches `fail` on macOS. Verified on `/bin/bash` 3.2.57. The fix is to branch on `[[ ! -e "${launcher}" ]]`.
- 🟡 **Safety / Compatibility** (plus a Correctness major on symlinks): `is_safe_prune_root` rejects 0775 roots, which are common under user-private-group umask 002 on Ubuntu and Fedora. It also rejects a symlinked root, and the verb then misreports it as "writable by other users". On those hosts pruning never runs.
- 🟡 **Correctness** (Security minor): The sticky branch trusts a root owned by another uid, and that owner can still rename every entry.
- 🟡 **Compatibility** (Correctness minor): In a sticky root shared by several users, other users' entries fail with `EPERM` and are recorded as `RemovalFailure`, so the verb always exits non-zero.
- 🟡 **Compatibility / Test Coverage**: The Phase 5 regression helper `prune_plugin_bin(&harness, older_than)` cannot carry the clocks, liveness and pins the 17 call sites inject. The bystander-held test blocks under `Wait`.
- 🟡 **Architecture**: The tree-lock primitive never reaches `TreeResolver` or its `Reaper`. `materialise` still uses the static `HeldTreeLock::try_take`, so the materialise leg of the self-probe guard is blind.
- 🟡 **Architecture** (Code Quality minor): `PluginBinRetention` cannot get the collaborators its `prune` needs. `for_root` runs at start-up, the tree resolver is built lazily in `run_cache`, and holding it would form a cycle with `AfterMaterialise`.
- 🟡 **Code Quality** (Security, Architecture and Test Coverage minors): The tree-claims `list(paths)` and `read_pointer(path)` take no port, which contradicts "all through `ports.filesystem`". An implementer could fall back to the unbounded `read_to_string`.
- 🟡 **Test Coverage** (Security, Safety and Correctness minors): The unsafe-root gate before `prune_held` is untested, and `materialise`'s own reap runs ungated. That contradicts "no prune path in an unsafe root removes anything".
- 🟡 **Performance**: The release-guard leaf's per-session target dir is never cached. Each CI leg and each local `mise run` does a cold LTO build and leaves 1–2 GB behind.
- 🔵 **Security**: A planted or symlinked `.accelerator-prune.lock` in a sticky shared root can block pruning, or create files elsewhere on macOS. Ancestors of the root are not checked.
- 🔵 **Correctness**: A tree can be removed while a dispatch holds its lease, between the probe and `remove_tree`, so the accepted risk overstates what the lease protects. An `Unreadable` re-read is unspecified. The lock table leaves the absent-lock outcomes and empty pinned artifacts unmapped.
- 🔵 **Safety / Correctness**: Pid liveness is host-local; record that limit. A non-running `ShimCopy` has no age gate. An untrusted `trees/` is not reported.
- 🔵 **Architecture**: A cold-store prune can probe leases its own process holds. `start_up` stops before the resolver composition. `is_safe_prune_root` is enforced at call sites rather than structurally.
- 🔵 **Code Quality**: Three overlapping tree-name classifiers; `RetentionPorts` growing into a catch-all bag of ports; the stale-claim rule written inline; a duplicated trust predicate; root-level vocabulary placed under `tree/`; a `PruneOutcome` enum suggested; small signature smells.
- 🔵 **Compatibility**: The CHANGELOG omits the unsafe-root refusal. The default-root verb no longer always runs the root-wide reap, and §3 contradicts the lock table.
- 🔵 **Test Coverage**: `prune_held`'s effect cannot be told apart from the following `run`. `fired()` does not prove the event came after the decision. The paused-store margin is one second. Some keep-only tests have no positive control. Foreign `trees/` names are not tested from the cold path. Default-root lock-failure behaviour is not pinned. The Phase 6 criterion wording is wrong. No test covers the exit-1 vanished-launcher case. The end-to-end root depends on the umask.
- 🔵 **Performance**: Step 3 re-lists `trees/` for every artifact.

### Assessment

The core design is settled and the lenses agree it holds: containment, fail-closed reads, self-protection, single-lock discipline, one tree prune and the per-platform shim rule. The remaining majors cluster in three places:
- **The root-safety rule** (decision 5) needs refining for umask 002, symlinked roots, foreign-owned sticky roots and multi-user sticky roots. Those are design choices.
- **The bootstrap tail** (decision 6) must test for the file rather than the exit status.
- **Wiring left by the refactor**: tree-lock injection into `TreeResolver` and `Reaper`, `PluginBinRetention`'s collaborators, the port signatures, the Phase 5 helper seams, gate coverage of the materialise path, and caching of the release-guard target dir.

The wiring items are mechanical. Pass 4 should need only architecture, compatibility and test-coverage, plus security if the root-safety rule changes materially.

## Re-Review (Pass 4) — 2026-10-21

**Verdict:** REVISE

Lenses: architecture, test-coverage, compatibility, security. Pass 3's structural fixes and decisions A, C and D were applied. There are no critical findings. Nine majors remain. Seven are precision gaps: signatures, test seams, phase isolation. Two are design questions about the root-safety rule.

### Previously Identified Issues

- ✅ **Compatibility**: Bash 3.2 `exec` status — Resolved (tests the file)
- 🟡 **Safety / Compatibility**: User-private-group umask roots — Partially resolved (the rule exists; musl has no NSS, see below)
- ✅ **Correctness / Security**: Sticky root owned by another uid — Resolved (the root must be owned by the euid)
- ✅ **Compatibility**: Multi-user sticky roots — Resolved (only the owner prunes)
- 🟡 **Compatibility / Test Coverage**: Phase 5 regression helper — Partially resolved (seams carried; commit 1 widens its scope)
- ✅ **Architecture**: Tree-lock primitive into `TreeResolver`/`Reaper` — Resolved (`repair` unnamed)
- ✅ **Architecture**: `PluginBinRetention` collaborators — Resolved (`prune_pinned_on_host`)
- ✅ **Code Quality / Security**: Tree-side reads off the port — Resolved
- ✅ **Test Coverage / Security**: Unsafe-root gate on materialise — Resolved (`ReapPermit`); `reap_superseded` still outside it
- ✅ **Performance**: Release-guard cache — Resolved (not re-reviewed)
- ✅ **Test Coverage**: Positive controls; foreign `trees/` names from cold paths; default-root lock failure; exit 1; end-to-end umask; recorded composition; Phase 6 wording — Resolved
- 🟡 **Test Coverage**: `prune_held` ordering; `fired()` ordering — Partially resolved (no shared event timeline)
- 🟡 **Architecture**: Proof types at call sites — Partially resolved (`RootPrune` and `WholeTree` do not carry `SafePruneRoot`)

### New Issues Introduced

- 🟡 **Compatibility / Test Coverage** (Security minor): The ancestor walk `lstat`s each component and rejects a symlinked intermediate ancestor. That breaks every macOS `/var` and `/tmp` path, including CI tempdirs, and Linux `/home -> var/home`. Verified that `/tmp` and `/var` are symlinks here. The fix is to canonicalise the whole override once in `start_up` and walk the canonical path, as sshd does.
- 🟡 **Compatibility**: `primary_group_is_private` uses `getpwuid_r`/`getgrgid_r`, but the shipped Linux launchers are static musl, which has no NSS (verified in `tasks/shared/targets.py`). SSSD, LDAP and FreeIPA users are never resolved.
- 🟡 **Security**: The sticky exception admits a root other users can still create names in. The bootstrap's probe, staging and temp writes follow planted names, and macOS has no `protected_symlinks`.
- 🟡 **Architecture**: `SafePruneRoot::check(root, &CachePorts)` makes the `ownership` leaf depend on `retention`.
- 🟡 **Architecture**: Phase 4 merged alone stops the relocated-root verb pruning trees, because `RootPrune` has no tree step until Phase 5.
- 🟡 **Architecture**: `held_leases` has no path from the acquired leases to `TreePrune`, and no `Generation` type exists.
- 🟡 **Test Coverage**: A single effective-uid seam cannot make one file foreign-owned, so the foreign-claim criterion and the owner-rule tests can't fail for the right reason. A per-path metadata overlay is needed.
- 🟡 **Test Coverage**: The Phase 5 commit-1 helper loops over every compiled-in artifact rather than `ARTIFACT`, and blocks the bystander test.
- 🟡 **Test Coverage**: The real user-private-group detection has no test; it needs a pure-rule table and a smoke test.
- 🔵 **Architecture**: Signatures for `RootPrune`/`PruneOutcome` disagree. The `locks` leaf imports `tree/lease.rs`. `repair` is missing from the lock migration. `StartUp::tree_resolver` lifetimes are unclear. `CachePorts` is declared twice. Suggested: an `AfterStore` port, and avoiding the `TreeEntry`/`EntryKind` name collisions.
- 🔵 **Test Coverage**: The warm-path exact list omits the symlink check. There is no shared event timeline for ordering assertions. The squatted-lock test is in the wrong phase. The end-to-end test cites `cli/launcher/pins.toml`, but the file is the repo-root `pins.toml` (verified).
- 🔵 **Security**: Squatted-lock replacement can unlink a peer's live lock. `is_trusted_entry` drops the write-mode test. `reap_superseded` is ungated.
- 🔵 **Compatibility**: `protected_regular` blocks the squatted-lock open. CHANGELOG shared-root and unsafe-root entries are inaccurate. Three default-root divergences. WSL, rootless containers and NFS squash never prune. The `deny.toml`/pup sentence is unnecessary.

### Assessment

Pass 4 moved the remaining issues from design to precision. The two design points left, sticky roots and NSS-free private-group detection, both concern which shared or managed environments the root-safety rule admits. Everything else is mechanical: canonical-path walking, proof types in signatures, phase-4 isolation, held-lease wiring, a per-path metadata overlay for tests, and documentation accuracy. A pass 5 with test-coverage and security should be enough if the design points are settled without new mechanisms.

## Re-Review (Pass 5) — 2026-10-21

**Verdict:** REVISE

Lenses: test-coverage, security, compatibility. Pass 4's structural fixes and decisions E and F were applied, and work items 0340–0342 were raised for shared multi-user roots. There are no critical findings. Five majors remain: four test-buildability gaps and one compatibility gap in the ancestor-ownership rule. Security found no major issues.

### Previously Identified Issues

- ✅ **Compatibility / Security / Test Coverage**: Intermediate symlink ancestors — Resolved (canonical path walked)
- ✅ **Compatibility**: musl without NSS — Resolved (`getent` fallback; edge cases below)
- ✅ **Security**: Sticky exception lets others create names; squatted-lock race — Resolved (decision E)
- ✅ **Security**: `is_trusted_entry` write-mode test; proof types in signatures — Resolved
- 🟡 **Security**: `reap_superseded` ungated — Partially resolved (`ReapPermit::plugin_bin` is constructible anywhere; `repair`'s permit source is unspecified)
- 🟡 **Test Coverage**: Single uid seam — Partially resolved (Phase 4 overlay; Phase 3 and four annotations still use the uid seam)
- ✅ **Test Coverage**: Phase 5 commit-1 widening; ancestor walk on host temp paths; event timeline; squatted-lock phase; `pins.toml` path — Resolved
- 🟡 **Test Coverage**: User-private-group detection untested — Partially resolved
- 🟡 **Compatibility**: CHANGELOG accuracy; WSL and container layouts — Partially resolved

### New Issues Introduced

- 🟡 **Test Coverage**: Phase 3's foreign-claim unit tests still use a whole-process uid seam, which also makes `claims/` foreign. They need a per-path metadata fake available in `src/`.
- 🟡 **Test Coverage** (Compatibility minor): The recorded call lists don't canonicalise the relocated root. On macOS the `tempfile` paths `/var/folders/…` become `/private/var/…`, so the recorded relocated tests fail on that leg.
- 🟡 **Test Coverage**: The private-group branches beyond the root's own mode are untested: the `group == egid` conjunct, ancestors, and the `private_gid` widening of `is_trusted_entry`. If that widening breaks, tree prune silently stops for umask-002 users.
- 🟡 **Test Coverage**: Held leases are tested only on the cold store. Nothing covers `run`'s `HeldLeases::of` wiring or a materialise that prunes while the process holds another artifact's lease.
- 🟡 **Compatibility**: Requiring every ancestor to be owned by the euid or root refuses common container and CI layouts (root over a host-owned bind mount, GitHub Actions `/__w`, macOS "ignore ownership" volumes reporting uid 99). The docs and CHANGELOG don't mention this, and the verb changes from exit 0 to non-zero there.
- 🔵 **Security**: The private-group exception reads Linux POSIX ACL mask bits as the group's permission, and macOS ACLs aren't visible in mode bits.
- 🔵 **Security**: `ReapPermit::plugin_bin` and an unbound `SafePruneRoot` leave the relocated reap gate up to callers.
- 🔵 **Security / Compatibility** (suggestion): `getent` output validation and child handling (kill and reap on timeout), and the NixOS and Guix paths.
- 🔵 **Test Coverage**: Stale uid-seam annotations. A stale, inverted criterion-map name (`a_user_owned_sticky_root_is_pruned_…`). An unused `patience` field fails clippy in commit 1. `another_artifacts_pinned_archive_survives_a_prune` loses its meaning. `RootHarness` hides `start_up`'s canonicalisation and depends on the umask.
- 🔵 **Compatibility**: The CHANGELOG under-describes the end of the materialise reap in unsafe roots and the refusal of unresolvable private groups. The `/tmp/accelerator` example is wrong. The docs success criterion says five entries where there are seven.

### Assessment

The safety design is settled. Security found no major issues and confirmed that every removal path sits behind `SafePruneRoot` or a trusted-directory check. What remains is test seams and fixtures that can't fail for the right reason, plus one design question: whether the ancestor-ownership rule should admit container and CI layouts where the ancestor's owner is the host user rather than the euid or root. After that is decided and the test fixes are applied, a final test-coverage pass should suffice.

## Re-Review (Pass 6) — 2026-10-21

**Verdict:** REVISE

Lenses: test-coverage, security. Pass 5's fixes and decision G (`ACCELERATOR_CACHE_TRUSTED_OWNERS`) were applied. There are no critical findings. Security found no majors. Test-coverage found five majors, all about whether tests can be built or can fail for the right reason. Every finding is structural or tightens a rule, so none needs a design decision.

### Previously Identified Issues

- 🟡 **Test Coverage**: Phase 3 uid seam — Partially resolved (the per-path overlay is right; delivery through `cfg(test)` cannot reach integration tests)
- ✅ **Test Coverage**: Recorder canonicalisation — Resolved for in-root paths (the anchor still needs both spellings)
- 🟡 **Test Coverage**: Private-group branches — Partially resolved
- 🔴 **Test Coverage**: Held leases through `run` or materialise — Still present (the materialise scenario cannot occur in production, see below)
- ✅ **Test Coverage**: Stale annotations; criterion map; unused `patience` field — Resolved
- 🟡 **Test Coverage**: `RootHarness` — Partially resolved (seeded subdirectories still depend on the umask)
- 🟡 **Security**: POSIX/macOS ACLs; `ReapPermit` plugin-bin arm; `getent` child — Partially resolved

### New Issues Introduced

- 🟡 **Test Coverage**: A `cfg(test)` `ReportingFilesystem` cannot reach `tests/`, because integration tests link the library without `cfg(test)`. It needs an always-compiled `#[doc(hidden)] pub mod testing`.
- 🟡 **Test Coverage**: The held-lease materialise test targets a path production never takes. Verified: `acquire` returns `None` for an absent tree (`resolver.rs:557-558`), and materialise runs in a separate `cache ensure` process. `run`'s `HeldLeases::of` wiring stays unpinned because `run` is in the binary.
- 🟡 **Test Coverage**: `is_trusted_entry`'s write rule is untested, and its inputs (`mode`, `group`, gid methods) arrive only in Phase 4. The `trees/` widening test cannot fail in Phase 4, and `ensure_trees_dir` chmods `trees/` to `0700`; the realistic umask-002 case is the bootstrap lock directory.
- 🟡 **Test Coverage**: The `getent` oracle doesn't exist on macOS (`dscacheutil`/`dscl` instead).
- 🟡 **Test Coverage**: The `getent` fallback needs an injectable lookup seam with executable paths and a timeout.
- 🔵 **Security**: `command_runner`'s `Supervision` is private and `pup.ron` forbids the launcher from naming `BashCommandRunner` (verified), so the `getent` child needs a launcher-local supervisor.
- 🔵 **Security**: A listed trusted owner can redirect prune's removals. The refusal invites catch-all uids (65534, 99), and ignore-ownership volumes enforce no permissions.
- 🔵 **Security**: ACL detection fails open on rich-ACL filesystems (NFSv4, SMB, ZFS nfsv4); `lgetxattr` is needed; the macOS ACL scope (ancestors, deny-only ACEs) is unspecified.
- 🔵 **Security**: `RootTrust::PluginBin` accepts any `TreePaths`; it needs a proof only `identify` can mint.
- 🔵 **Test Coverage**: `another_artifacts_pinned_archive_survives_a_prune` still swaps its arm. The `ReapPermit` refusal tests can pass vacuously. Real-filesystem ACL adapter tests are missing. Fixtures depend on the umask and the group database. `TrustedOwners` parsing and `start_up`'s read are unpinned. The anchor needs both spellings.

### Assessment

The design is stable: two consecutive passes have raised no new design questions, and security has found no majors for three passes. The remaining work is precision in test seams and in where test code lives. These are mechanical and can be applied without further decisions. A final test-coverage pass then confirms the tests can be built and can fail.

## Re-Review (Pass 7) — 2026-10-21

**Verdict:** REVISE

Lens: test-coverage. Pass 6's fixes were applied. There are no critical findings. Three majors remain, each a test that cannot fail or cannot be built as described. Every acceptance criterion still maps to a named test.

### Previously Identified Issues

- ✅ **Test Coverage**: `cfg(test)` overlay delivery; `is_trusted_entry` phasing; `getent` oracle on macOS; `getent` seam; pinned-archive arm; fixture modes; anchor spellings — Resolved
- 🟡 **Test Coverage**: Held-lease wiring — Partially resolved (the materialise scenario is gone; the new test cannot fail)
- 🟡 **Test Coverage**: `ReapPermit` refusal vacuity; real-filesystem ACL tests; `TrustedOwners` read — Partially resolved

### New Issues Introduced

- 🟡 **Test Coverage**: `acquired_trees_are_never_probed_by_the_cold_store_prune` cannot reach a lease probe. Acquired digests are pinned and freshly claimed, so step 1 keeps them before probing, and the test passes with `HeldLeases::none()`. It needs unpinned, stale-claim seeding and a `none()` control that fires.
- 🟡 **Test Coverage**: No fixture can mint `PluginBinRoot`. The 17 `prune_plugin_bin` call sites and `Harness::resolver_pinning` (struct literal) break in Phase 7, and Phase 5's payload-free `PinnedOnHost` contradicts Phase 7.
- 🟡 **Test Coverage**: The macOS allow-write versus deny-only ACE classification is tested only through the overlay. An adapter that treats any ACE as disqualifying would refuse every root under `$HOME`.
- 🔵 **Test Coverage**: The Linux `setfacl` arm skips silently on CI. The `PATH`-planting `getent` test is tautological and mutates global `PATH`. A process-static account cache would couple tests. The `TrustedOwners` value isn't shown reaching `SafePruneRoot::check`. The untrusted-`trees/` materialise test lacks a trusted control. The always-compiled `testing` module is held to production clippy lints. The Phase 3 table says 0755 fails where the rule passes it.

### Assessment

The issues have narrowed to individual tests: whether three can fail, and how fixtures obtain the proof types. These are mechanical fixes with no design impact.

## Approval — 2026-10-21

**Verdict:** APPROVE

The fixes for pass 7's three majors were applied, with no further pass:
- the held-lease tests now seed unpinned digests with stale claims and have a `HeldLeases::none()` control;
- `PluginBinRoot` is introduced in Phase 5 with a harness that obtains it through `identify`;
- the macOS ACL classification has real-filesystem cases.

The plan was approved on the strength of the trend. Over seven passes the findings moved from structural and safety design to the construction of individual tests. The last two passes raised no design questions, and security found no majors for three passes. Any test-construction issue still left will surface in the red-first loop during implementation.

