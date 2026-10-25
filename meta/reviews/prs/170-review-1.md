---
type: "pr-review"
id: "170-review-1"
title: "[0218] Bound a relocated cache root's growth with claim-gated pruning"
date: "2026-10-25T08:39:18+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:170"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "safety", "security", "performance", "compatibility"]
review_number: 1
pr_number: 170
tags: []
last_updated: "2026-10-25T08:39:18+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #170 - [0218] Bound a relocated cache root's growth with claim-gated pruning

**Verdict:** COMMENT

A carefully reasoned change. `CacheRootKind` is matched once, `SafePruneRoot`/`PluginBinRoot` make "no removal in an unvetted root" a type-level guarantee, pointers are grammar-validated before they become paths, and race windows are tested deterministically with positive controls. No lens found a critical defect. The main risks are in edge paths: both claim readers fail open when a single claim's `lstat` fails; a second plugin version sharing the root loses its verifier shim copy (and, for pre-claim launchers, much more); the macOS ACL check misses `writesecurity`/`chown`; and the `CacheFilesystem` port, the PR's central test seam, is bypassed by tree claims, reap and lease code.

### Cross-Cutting Themes

- **Claim readers fail open on a per-claim `lstat` error** (flagged by: safety, test-coverage, code-quality): `filesystem.entry(&path).ok()?` in both `version_claims::list` and `tree::claims::list` drops a claim whose `lstat` fails for any reason, while the listing still reports `Read`. The PR names only the tree half. Both readers are near-identical copies that already disagree on how they report an untrusted directory, and neither residual has a test.
- **Two plugin versions sharing one relocated root** (flagged by: correctness, safety, compatibility): the scenario this feature exists for has three gaps. Another version's staged `accelerator-verify-*` copy is unprotected after an hour, which reopens the `fail_integrity` race that `RunningShims` documents. Pre-claim (1.23.x) launchers are evicted on every claim-aware prune, cannot re-resolve, and offline they leave durable integrity records. The `<version>.<launcher-id>` claim name is now a frozen cross-version format with no pinning test.
- **`CacheFilesystem` port applied only in part** (flagged by: architecture, code-quality, test-coverage): `tree::claims::refresh`, `Reaper::referenced_generations` and `TreePrune::remove_tree`'s exclusive lease go around the injected ports. So `an_unset_override_performs_exactly_the_fixed_calls` and the recording seams cannot see every call, and each refresh builds a fresh `ProcessOwnership` with an empty `OnceLock`.
- **A not-yet-existing override is identified as `PluginBin`** (flagged by: architecture, compatibility): the first process into a fresh `ACCELERATOR_CACHE_DIR` gets `PluginBinRetention`. It writes no claim and reaps under `RootTrust::PluginBin` without the `SafePruneRoot` walk, so `PluginBinRoot` no longer means what its name says.
- **Unbounded synchronous prune before `exec`** (flagged by: performance, safety): `after_store` and `after_materialise` remove every unclaimed, aged tree inline. On a root that has never been pruned, or on slow or NFS storage, the first SessionStart hook after the upgrade can stall long enough to hit a hook timeout.
- **Prune failure reporting loses fidelity** (flagged by: code-quality, test-coverage): a reap error is reported as `could not remove …/trees: other error`, and a listing failure is filed as a `RemovalFailure`. No test shows the prune continuing and reporting every failure after the first removal failure, which is a declared breaking change.

### Tradeoff Analysis

- **A/B/C criterion vs cold-path latency**: the work item requires every unclaimed tree gone after one dispatch, while performance and safety want a budget on the hook path. Recommendation: keep "everything in one run" for `accelerator cache prune` and give the cold path a time or item budget, leaving the rest for later cold paths. Alternatively, keep the criterion and record the worst case explicitly.
- **Bounded growth vs mixed-version tolerance**: evicting claim-less installs keeps the root bounded immediately. A grace period of one `CLAIM_WINDOW` after `claims/` is first created would protect 1.23.x sessions during rollout, at the cost of 14 days of extra footprint. The safety and compatibility lenses both recommend the grace period. At minimum, expand the CHANGELOG Migrations note.
- **Fail-closed safety vs spurious refusals**: refusing on any macOS `getfacl` error, or when `getent` lives outside `/usr/bin` and `/bin` (NixOS), is the safe default. The refusal should name the cause, so users are not left with an unexplained non-zero exit.

### Strengths

- ✅ `retention::for_root` is the only place that matches on `CacheRootKind`. Retention is chosen once at start-up as a polymorphic `RootRetention`, not through scattered `if relocated` branches.
- ✅ `SafePruneRoot`, `PluginBinRoot` and `ReapPermit` are proof-carrying types: a `WholeTree` prune or a relocated reap cannot be built without a passed safety check.
- ✅ The safety walk fails closed at every step (unreadable ACL, unknown volume ownership state, symlinked ancestor, malformed `ACCELERATOR_CACHE_TRUSTED_OWNERS`), and it refuses the overflow uid and uid 99 even when they are listed.
- ✅ `getent` runs only from fixed absolute paths, with a cleared environment, its own process group killed on timeout, a 64 KiB output cap, and an exact match required on the answer.
- ✅ Pointer contents are validated against the full generation grammar before any join, so `..`, absolute paths and other artifacts' generations are never followed.
- ✅ Clock skew can only keep entries: an unknown or future mtime counts as fresh or young everywhere.
- ✅ Entries come only from a listing taken before claims are read, and stale claims are re-checked just before unlink. A version's last claim goes only after all of its entries.
- ✅ The root lock is released before any tree lock is taken, and cold paths only try locks, so there is no lock-order cycle with `cache prune`.
- ✅ The re-resolve of a vanished entry is bounded to one attempt on ENOENT and declines override paths and present-but-unrunnable files.
- ✅ Race windows are tested deterministically through `PruneSteps` pauses and nth-call scripted injections, each with a positive control. The trigger matrix runs the same invariants through the verb, a cold store and a materialise.
- ✅ The default-root warm path is pinned by exact call-list assertions, and warm-dispatch-8's `cache::find` and `reverify` terms match run 7.
- ✅ The release guard is layered: `compile_error!`, a `black_box`'d marker, a byte scan at staging, and real stripped release builds in CI. `test_bootstrap_name_coherence.py` ties the bash name templates to the Rust grammar.
- ✅ Forward-compatible grammar: names the plugin does not fully recognise are always kept, and the tree-lock path is byte-identical across versions.

### General Findings

- 🟡 **Safety / Compatibility**: Pre-claim launchers sharing a relocated root are evicted on every claim-aware prune. They lack `re_resolve` and `execfail` handling, so they can fail dispatches outright and, offline, write durable unverified-launcher records. Consider a one-window grace period, and expand the Migrations note (`CHANGELOG.md:756`).
- 🟡 **Architecture**: ADR-0072 still says a multi-user root "is not protected". The PR builds a whole trust model (ancestor walk, ACLs, private group, `ACCELERATOR_CACHE_TRUSTED_OWNERS`, refusing unsafe roots), and that decision lives only in the plan and work items. Amend ADR-0072 or add a follow-up ADR.
- 🟡 **Performance**: warm-dispatch-8's median G-fast rose by ~5.8 ms over run 7 (44.0 → 49.8 ms) on a quieter host. Almost all of the rise is in the shim's minisign verification of the launcher (7.0 → 13.7 ms), outside the launcher's code paths. Re-measure base and head on one host and compare launcher binary sizes (new `exacl`, `libc` and the retention modules) before claiming the end-to-end warm path is unchanged.
- 🔵 **Architecture**: `TreeResolver`'s materialise path still uses `std::fs` directly. `write_private` now exists in three places.
- 🔵 **Code Quality**: `TreePrune` has two removal mechanisms. `Reaper` removes through `std::fs` and discards per-path failures, which weakens "reports every failure" for orphan residue.
- 🔵 **Test Coverage**: the default-root invariance test only sees calls made through the port. Add `RecordingLocks` and consider banning direct `std::fs` in `retention/` and `start_up`.
- 🔵 **Test Coverage**: the cold-path diagnostics (`warn_of`, and the log-once unsafe-root refusal) are untested, and they are the operator's only signal that pruning is off.
- 🔵 **Safety**: the root-level sub-binary grammar is broad if the override names a directory the plugin shares with other files, such as `~/.local/bin`. Consider a `.accelerator-cache-root` marker.
- 🔵 **Security**: every debug launcher built through the dev-build tasks honours the test release key for a `127.0.0.1` URL. Pointed at a real shared cache root, it could seed a test-signed entry that a release launcher later dispatches warm.

### Additional Findings

- 🟡 `cli/launcher/src/launch/outbound/resolve/mod.rs:223`: the synchronous root prune before `exec` has no work budget (performance, safety)
- 🔵 `cli/launcher/src/launch/outbound/resolve/retention/version_claims.rs:77`: claim refresh checks trust by path, then writes through the path (TOCTOU) (security)
- 🔵 `cli/launcher/src/launch/outbound/resolve/ownership/mod.rs:114`: the private-group test ignores accounts whose primary gid is this group (security)
- 🔵 `cli/launcher/src/launch/outbound/resolve/retention/single_file.rs:373`: removing `.minisig` before the entry can strand an unsigned binary for good (correctness)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/prune.rs:561`: tree claims on a removed digest are unlinked without a per-claim re-check (correctness)
- 🔵 `cli/launcher/src/launch/outbound/resolve/retention/root_retention.rs:187`: an `--older-than` below the ~1.4-day refresh interval treats active versions as stale (correctness)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/prune.rs:416`: other-layout pointers and their generations are never reclaimed (correctness)
- 🔵 `bin/accelerator:460`: a launcher removed between verify and `exec` is a hard failure with no re-fetch retry (safety)
- 🔵 `cli/launcher/src/launch/outbound/resolve/retention/root_prune.rs:142`: a listing failure is reported as a removal failure (code-quality)
- 🔵 `cli/launcher/src/launch/outbound/resolve/retention/root_prune.rs:26`: three overlapping report shapes need hand-written `absorb` glue (code-quality)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/prune.rs:121`: `TreePrune::run` mixes dispatch, a reap-once flag and a fallback reap in a single 60-line loop (code-quality)
- 🔵 `cli/launcher/src/launch/outbound/resolve/retention/single_file.rs:315`: boolean flag arguments sit beside the module's rich enums (code-quality)
- 🔵 `cli/launcher/src/launch/outbound/resolve/retention/single_file.rs:440`: `is_still_stale` re-parses a claim it already holds, giving a third copy of the validity rule (code-quality)
- 🔵 `cli/launcher/src/main.rs:206`: the crypto provider, release keys and fetcher are set up in three entry points (code-quality)
- 🔵 `cli/launcher/src/launch/start_up.rs:66`: the identification error is flattened to `String` and rebuilt as `CacheRootUnavailable` (code-quality)
- 🔵 `cli/launcher/src/launch/outbound/resolve/ownership/mod.rs:123`: the owner/private-group/ACL trust rule is encoded twice with different mode-bit vocabularies (code-quality)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:303`: the reaper names the concrete `SystemFilesystem` adapter (architecture)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/prune.rs:542`: the exclusive lease in `TreePrune` bypasses the lock ports (architecture)
- 🔵 `cli/launcher/src/launch/outbound/resolve/retention/ports.rs:18`: `CachePorts` is a nine-port bundle passed whole to narrow collaborators (architecture)
- 🔵 `cli/launcher/src/launch/outbound/resolve/filesystem.rs:1`: `CacheFilesystem`'s scope has outgrown its doc and mixes I/O with trust inspection (architecture)
- 🔵 `cli/launcher/src/launch/cache.rs:17`: the prune use case's port and outcome live in the outbound adapter tree (architecture)
- 🔵 `cli/launcher/src/launch/outbound/resolve/retention/root_retention.rs:150`: the safety walk and tree survey are repeated within one cold materialise (performance)
- 🔵 `cli/launcher/src/launch/outbound/resolve/ownership/accounts.rs:141`: `getent` supervision polls every 10 ms, with a worst-case block of 4 s (performance)
- 🔵 `cli/launcher/src/main.rs:929`: the `re_resolve` unit tests run `start_up` against the developer's real cache root (test-coverage)
- 🔵 `cli/launcher/tests/root_prune.rs:442`: permission-based tests are inconsistently guarded against running as root and do not restore modes on panic (test-coverage)
- 🔵 `cli/launcher/tests/root_prune.rs:646`: the claim-order assertion also pins the order of signature and entry removal (test-coverage)
- 🔵 `tests/integration/release_guard/test_release_guard.py:63`: the primary `compile_error!` guard is untested (test-coverage)
- 🔵 `CHANGELOG.md:161`: the `bin/accelerator` exit-code break is hidden inside the `cache prune` bullet (compatibility)
- 🔵 `CHANGELOG.md:756`: the Migrations note understates the effect on claim-less installs (compatibility)
- 🔵 `cli/launcher/src/launch/outbound/resolve/retention/version_claims.rs:197`: the claim-file name format is now a frozen cross-version contract (compatibility)
- 🔵 `cli/launcher/src/launch/outbound/resolve/filesystem.rs:292`: the macOS ACL read treats every error as unexaminable, unlike Linux's handling of ENOTSUP (compatibility)
- 🔵 `docs-site/package-lock.json:2735`: the lock bump includes astro 7.4.1 and a Node ≥22.22.0 floor, which the description does not mention (compatibility)
- 🔵 `.github/workflows/main.yml:54`: `apt-get install acl` runs without a preceding `apt-get update` (compatibility)
- 🔵 `cli/launcher/src/launch/outbound/resolve/ownership/accounts.rs:30`: the fixed `getent` paths miss NixOS (compatibility)

---
*Review generated by /review-pr*

## Inline Comments

### `cli/launcher/src/launch/outbound/resolve/tree/claims.rs:48-57` — Tree claim refresh bypasses the injected ports (architecture, code-quality)
**Severity**: major | **Confidence**: high | **Lens**: architecture, code-quality

🟡 **Architecture**

`claims::refresh` calls `SystemFilesystem.create_private_dir` and `is_trusted_directory(&claims, &SystemFilesystem, &ProcessOwnership::default())` directly, and writes the claim with its own `write_private`. Every other consumer gets `CacheFilesystem` and `Ownership` through `CachePorts`. This code runs on every tree resolve, including the warm `acquire` path. The recording filesystem cannot see these calls, and each refresh builds a fresh `ProcessOwnership` whose empty `OnceLock` does not share the primary-group lookup, including the bounded `getent` spawn on Linux.

🔵 **Code Quality**

`version_claims::refresh` takes `filesystem` and `ownership` as parameters, so the two claim kinds now have different shapes for the same operation. `retention/ports.rs` promises that "a single recorder sees every call", and this path breaks that promise.

**Suggestion**: pass `&dyn CacheFilesystem` and `&dyn Ownership` into `refresh`, give `TreeResolver` both ports from `CachePorts` in `StartUp::tree_resolver`, and use `CacheFilesystem::write_private`.

---

### `cli/launcher/src/launch/outbound/resolve/tree/prune.rs:394-399` — Reap error discarded and reported as a generic removal failure on `trees/`
**Severity**: major | **Confidence**: high | **Lens**: code-quality

🟡 **Code Quality**

`Err(_)` pushes `RemovalFailure { path: <trees root>, kind: ErrorKind::Other }` and drops the `TreeError`, which knows which pointer, generation or lock failed and why. Both `cache prune` and the cold-path warnings then print `could not remove …/trees: other error`, which names the wrong path and gives no cause.

**Suggestion**: carry the error through, for example as `ReapFailure { root, detail: error.to_string() }` rendered as `could not reap orphans under {root}: {detail}`. At minimum, `tracing::warn!(%error, …)` before dropping it.

---

### `cli/launcher/src/launch/outbound/resolve/retention/version_claims.rs:106-149` — Version-claim and tree-claim listing are duplicated and model "untrusted" differently
**Severity**: major | **Confidence**: high | **Lens**: code-quality

🟡 **Code Quality**

`version_claims::list` and `tree::claims::list` have the same structure: NotFound becomes empty, then the trust check, list, parse and `entry()` per claim. `is_valid` and `is_valid_claim_entry` are identical, and the 16-hex launcher-id check is written out twice. The two have already drifted: an untrusted version-claims directory is reported as `Unreadable(PermissionDenied)` ("claims unreadable (permission denied)"), while tree claims report a distinct `Untrusted` ("another user could write to it"). The fail-open `lstat` bug flagged at line 138 and at `tree/claims.rs:183` is the same bug in both copies.

**Suggestion**: extract a shared `ClaimsDirectory::list(dir, fs, ownership)` that returns `Read`/`Unreadable`/`Untrusted`. Move the validity rule and the launcher-id grammar next to `retention::launcher_id()`, and let each claim kind parse only its own `<subject>.<launcher-id>` name.

---

### `cli/launcher/tests/root_prune.rs:971-997` — No test proves the prune continues after a removal failure
**Severity**: major | **Confidence**: high | **Lens**: test-coverage

🟡 **Test Coverage**

The PR lists as breaking that `cache prune` "reports every failure instead of stopping at the first … exits non-zero after pruning everything else". These tests, and `a_tree_removal_failure_is_reported`, each plant a single prunable item and inject a single failure. If `Removals::tally` or `SingleFilePrune` returned early on the first `Failed`, every test would still pass.

**Suggestion**: plant two unclaimed versions and an unclaimed tree, then fail the first version's entry removal and the tree's `RemoveTree`. Assert that the second version's entries are gone, that `err` names both failed paths, and that the verb exits non-zero.

---

### `cli/launcher/src/launch/outbound/resolve/ownership/safe_root.rs:202-211` — The safety gate's fail-closed error branches are untested
**Severity**: major | **Confidence**: high | **Lens**: test-coverage

🟡 **Test Coverage**

`SafePruneRoot::check` treats a `statfs` error as ownership ignored (`unwrap_or(true)`) and refuses with `Unexaminable` when `entry` or `access_acl` fails. `ReportingFilesystem` can only override with successful values, so no test reaches these branches. Mutating `unwrap_or(true)` to `unwrap_or(false)`, or `map_err(...)?` to `.unwrap_or(AccessAcl::None)`, would make the only defence against pruning through another user's directory fail open, and every test would stay green.

**Suggestion**: give `ReportingFilesystem` `failing_entry`/`failing_acl`/`failing_ownership_check(path, ErrorKind)` overrides, and add table cases for the root and for an ancestor.

---

### `cli/launcher/src/launch/outbound/resolve/retention/version_claims.rs:138` — Version claims whose `lstat` fails are dropped (fails open)
**Severity**: major | **Confidence**: medium | **Lens**: safety

🟡 **Safety**

`filesystem.entry(&path).ok()?` drops a claim on any `lstat` error (EACCES, EIO, ESTALE), and the listing is still `ClaimsListing::Read`. For a `claims/` directory that can be read but not searched (e.g. `0600`, which passes `is_trusted_directory_entry`), every `lstat` fails. `FreshVersions` then holds only the running version, and every other version's aged entries are removed without any report. This is the version-claims twin of the `tree/claims.rs:183` residual. The PR description names only the tree one.

**Suggestion**: map any non-NotFound error to `ClaimsListing::Unreadable(kind)`, or keep the claim as protecting with `modified_seconds: None` so that, like an unknown mtime, it can only keep things.

---

### `cli/launcher/src/launch/outbound/resolve/tree/claims.rs:183` — Tree claims whose `lstat` fails are dropped (safety, test-coverage)
**Severity**: major | **Confidence**: medium | **Lens**: safety, test-coverage

🟡 **Safety**

The same `.ok()?` lets a `trees/claims/` that can be read but not searched list as `Read` with no claims. `WholeTree` then reclaims every aged, unleased generation of every artifact and platform, each up to ~294 MB. The re-read before `remove_tree` goes through the same path, so it does not catch this. Mapping non-NotFound errors to `TreeClaimsListing::Unreadable(kind)` reuses the existing "reclaim nothing" branch.

🔵 **Test Coverage**

This residual, and the bootstrap lock directory holding more than `pid`, has no test. The existing unreadable-claims tests use `0o000`, which fails at `list` and so fails closed. Add a `0o444` case, or mark it `#[ignore = "…"]` if the fix is deferred.

---

### `cli/launcher/src/launch/outbound/resolve/filesystem.rs:294-300` — macOS ACL write check misses `writesecurity` and `chown` (security, test-coverage)
**Severity**: major | **Confidence**: medium | **Lens**: security, test-coverage

🟡 **Security**

`access_acl_of` treats a directory as writable by others only on an allow ACE for `WRITE`, `APPEND` or `DELETE_CHILD`. An ACE granting another principal `WRITE_SECURITY` or `CHOWN` reads as `AccessAcl::None`, yet that principal can rewrite the ACL or take ownership and then rename names in the root or any ancestor. **Suggestion**: count `WRITE | APPEND | DELETE_CHILD | WRITE_SECURITY | CHOWN`, or fail closed on any non-deny ACE that is not read-only.

🔵 **Test Coverage**

Only `add_file` is tested. Removing `APPEND` or `DELETE_CHILD`, or dropping `system.nfs4_acl`/`richacl` on Linux, would not fail any test. Loop over every write-granting permission, including the two above, and assert `AccessAcl::Other`.

---

### `cli/launcher/src/launch/outbound/resolve/retention/single_file.rs:57-62` — Another plugin version's shim copy is unprotected (correctness, safety, compatibility)
**Severity**: major | **Confidence**: medium | **Lens**: correctness, safety, compatibility

🟡 **Correctness**

`RunningShims::runs` protects a shim copy only when its digest matches the *running* plugin's `accelerator-verify-<platform>`. In a root shared by two installed versions, the other version's copy is `Unprotected` and `Aged` an hour after staging, because `bin/accelerator` stages it once and never refreshes its mtime. Each cold path then removes it. If the removal lands between staging and `verify_launcher`/`fetch_and_verify`, that bootstrap falls into `fail_integrity`, which is exactly the race this type's doc comment says it prevents. Version claims protect that version's launcher and sub-binaries, but not its shim.

🔵 **Safety / Compatibility**

The result is an occasional failed dispatch, silent under `--fail-safe`, plus a durable, misleading unverified-launcher record, caused by a routine prune between two supported, claim-writing versions.

**Suggestion**: have the bootstrap touch the copy on every use and age `ShimCopy` against the claim window, or keep every copy on a platform while another version holds a fresh claim. Alternatively, have `bin/accelerator` re-stage and retry once when the shim is missing at exec time.

---

### `cli/launcher/src/launch/outbound/resolve/cache_root.rs:134-141` — An unresolvable override is labelled `PluginBin` (architecture, compatibility)
**Severity**: major | **Confidence**: medium | **Lens**: architecture, compatibility

🟡 **Architecture**

When `ACCELERATOR_CACHE_DIR` does not exist yet, `identify` returns `CacheRootKind::PluginBin` with the override path. `start_up` identifies once, before the cold path's `create_dir_all`, so the first process into a fresh override runs `PluginBinRetention`. It writes no claim and reaps under `RootTrust::PluginBin`, skipping the `SafePruneRoot` walk. That breaks "a materialise in an unsafe relocated root no longer reaps" for this process, and it lets `PluginBinRoot`, documented as something "no bare path can stand in for", be a bare relocated path.

🔵 **Compatibility**

The first invocation into a new root behaves differently from every later one, which is hard to reason about.

**Suggestion**: add a `Pending`/`Unresolved` kind whose retention does nothing (no claim, no reap, no prune), or re-identify after `create_dir_all`. Never borrow `PluginBin`.

## Per-Lens Results

### Correctness

**Summary**: The claim-gated prune is carefully reasoned. The listing is taken before claims are read, version claims are re-checked before unlink, pointers are validated before any join, the root lock is released before any tree lock, and re-resolve is bounded to one attempt. The main gap is shim-copy protection, which covers only the running plugin's digest. Smaller issues: tree claims are unlinked without a per-claim re-check, signature-first removal can strand an unsigned binary, and an `--older-than` below the refresh interval misreads active versions as stale.

**Strengths**: the pre-claim listing makes concurrent stores safe by construction; stale claims are re-read before unlink, and a version's last claim goes only after every entry; `PointerTarget::of` validates before joining; lock ordering is sound, with `after_materialise` dropping the tree lock first; re-resolve happens once and only on ENOENT; unknown or future mtimes only keep; `PruneReport` separates "kept" from failures and collects every failure.

**Comments**:
- `retention/single_file.rs:57-62` (RIGHT), major/medium: the shim copies of a concurrently active plugin version are unprotected, which reopens the `fail_integrity` race. *(Merged inline.)*
- `retention/single_file.rs:373-383` (RIGHT), minor/medium: removing the signature before the entry can strand an unsigned binary that no later prune recognises. Remove the entry first, or classify an aged, unsigned, digest-suffixed stem as residue.
- `tree/prune.rs:561-568` (RIGHT), minor/medium: tree claims on a removed digest are unlinked without a per-claim re-check, so a claim refreshed after the re-read is deleted and the freshly materialised tree becomes reclaimable. Re-stat each claim first and skip it if it now protects the digest (`protects_within`).
- `retention/root_retention.rs:187-196` (RIGHT), minor/medium: `--older-than` below `CLAIM_WINDOW / REFRESH_FRACTION` (~1.4 days) treats recently used versions as unclaimed. Clamp the window to the refresh interval or document the behaviour.
- `tree/prune.rs:416-422` (RIGHT), minor/low: `OtherLayout` pointers and their generations are never reclaimed, so retired layouts grow without bound.

**General Findings**: none.

### Safety

**Summary**: The prune is mostly fail-closed: `SafePruneRoot` gating, `Unreadable` kept separate from empty, the running version always protected, re-checks before unlink, and a separate retention for the plugin bin. The one gap is that both claim readers drop claims whose `lstat` fails, and the PR acknowledges only half of it. Smaller risks are mixed or pre-claim versions sharing a root and an unbounded synchronous cold-path prune. Everything deleted can be re-fetched, so the worst case is lost availability, not permanent data loss.

**Strengths**: fail-closed `SafePruneRoot::check`; `Unreadable` listings skip all versioned and tree removals; the running version is unioned in, and a future mtime counts as fresh; claims and locks are re-read before unlink; the name grammar keeps half-stored entries and lone `.minisig` files; the cold-path prune only tries locks and only logs problems; the pointer is unlinked before the generation, which is removed only under an exclusive lease; `PluginBinRetention` writes no claims.

**Comments**:
- `retention/version_claims.rs:138` (RIGHT), major/medium: version claims whose `lstat` fails are silently dropped. *(Inline.)*
- `tree/claims.rs:183` (RIGHT), major/medium: tree claims whose `lstat` fails are dropped, so live installs' trees can be reclaimed. *(Merged inline.)*
- `retention/single_file.rs:42-62` (RIGHT), minor/medium: another installation's shim copy can be removed under its live bootstrap. *(Merged into the correctness inline.)*
- `bin/accelerator:460-465` (RIGHT), minor/medium: a launcher removed between verify and `exec` is now a hard failure. Loop once through `acquire_lock` and `fetch_and_verify` before calling `fail`.

**General Findings**:
- major: pre-claim launchers sharing a relocated root are evicted on every claim-aware prune, cannot re-resolve, and fail offline. Consider a one-window grace period keyed on when `claims/` was first created, or at least a one-time warning.
- minor: the synchronous cold-path prune has no time bound. Give it a budget, or run the tree half in a detached child.
- suggestion: the root-level sub-binary grammar is broad if the override names a shared directory. Require a `.accelerator-cache-root` marker.

### Security

**Summary**: The deletion trust model is careful and mostly sound: an sshd-StrictModes-style `lstat` walk, grammar-validated pointers, symlink-safe `remove_dir_all`, and a three-layer `test-loopback` guard. The main gap is that the macOS ACL check misses `writesecurity` and `chown`. Smaller gaps are the claim refresh TOCTOU and the private-group test ignoring accounts whose primary gid is the group.

**Strengths**: `SafePruneRoot` is the only constructor for a removal regime; the root may never be sticky-and-world-writable; trusted-owner parsing is strict and refuses the overflow uids; `getent` is run with a fixed path, cleared environment, timeout, output cap and exact-match check; POSIX ACLs are treated as the mask, and NFSv4 and richacl as unknown; strict pointer validation; the release guard has defence in depth.

**Comments**:
- `filesystem.rs:294-300` (RIGHT), major/medium: the macOS ACL write check misses `writesecurity` and `chown` grants. *(Merged inline.)*
- `retention/version_claims.rs:77-84` (RIGHT), minor/medium: claim refresh checks trust by path, then writes through the path (TOCTOU). Use `openat` with `O_DIRECTORY|O_NOFOLLOW`, `fstat` and `futimens`, or skip refresh in roots that fail the ownership check.
- `ownership/mod.rs:114-121` (RIGHT), minor/low: the private-group test ignores accounts whose primary gid is this group. Document the assumption, or enumerate passwd under a bound.

**General Findings**:
- suggestion: every debug launcher carries the test-key trust seam. Consider refusing a relocated root under `test-loopback`, or recording the trust source, and note in `tasks/README.md` that dev launchers must not share a cache root with installed releases.

### Architecture

**Summary**: A substantial structural improvement: a single identify → `for_root` step, and shared leaf modules that make dependencies run one way only (retention → tree → ownership → filesystem). The weaknesses are a port applied only in part, an `identify` fallback that breaks the `PluginBinRoot` invariant, and an ADR-0072 that no longer describes what was built.

**Strengths**: `CacheRootKind` is matched once; proof-carrying root types; `prune_report` and `retention_windows` avoid a retention ↔ tree cycle; one `TreePrune` with two regimes, plus a `TreeLocks` facade with a documented lock ordering; `ReResolution` keeps the bounded retry in the core; `UnidentifiedRetention` keeps built-ins working when identification fails.

**Comments**:
- `cache_root.rs:134-141` (RIGHT), major/medium: an unresolvable override is labelled `PluginBin`. *(Merged inline.)*
- `tree/claims.rs:48-57` (RIGHT), major/high: tree claim refresh hard-wires `SystemFilesystem` and `ProcessOwnership`. *(Merged inline.)*
- `tree/reap.rs:303` (RIGHT), minor/high: the reaper names the concrete `SystemFilesystem` adapter.
- `tree/prune.rs:542` (RIGHT), minor/medium: the exclusive lease in `TreePrune` bypasses the lock ports.
- `retention/ports.rs:18-28` (RIGHT), minor/medium: `CachePorts` is a nine-port bundle passed whole to narrow collaborators. Move it to the composition root.
- `filesystem.rs:1-2` (RIGHT), suggestion/medium: `CacheFilesystem`'s scope has outgrown its doc. Consider splitting out the trust inspection.
- `launch/cache.rs:17-20` (RIGHT), suggestion/low: `RootRetention` and `PruneOutcome` live in the outbound adapter tree.

**General Findings**:
- major: ADR-0072 no longer describes the multi-user safety design that was built. Amend it or add an ADR.
- minor: `TreeResolver`'s materialise path still uses `std::fs` directly, and `write_private` is triplicated.

### Code Quality

**Summary**: Explicit domain types and pure decision functions are kept apart from the effectful loops. The risks are in error handling (reap errors discarded, listing failures filed as removal failures) and duplication between the two claim kinds. A few paths hard-wire the system adapters. No comment-policy violations were found.

**Strengths**: pure, exhaustively matched decision functions; `SafePruneRoot` and `TreePruneRegime` enforce the safety check by type; one `for_root` choice; `UnsafeRoot`'s message names the path and its remedy; `Removals::tally` separates `AlreadyGone` from real failures; `main.rs` no longer builds `TreeResolver` twice.

**Comments**:
- `tree/prune.rs:394-399` (RIGHT), major/high: the reap error is discarded. *(Inline.)*
- `retention/version_claims.rs:106-149` (RIGHT), major/high: duplicated claim listing, with diverging handling of untrusted directories. *(Inline.)*
- `retention/root_prune.rs:142-155` (RIGHT), minor/high: a listing failure is filed as a `RemovalFailure`. Add an `UnlistableDirectory` category.
- `tree/claims.rs:48-59` (RIGHT), minor/high: hard-wired ports. *(Merged into the architecture inline.)*
- `retention/root_prune.rs:26-33` (RIGHT), minor/medium: three overlapping report shapes need `absorb` glue. Use one `PruneReport`.
- `tree/prune.rs:121-180` (RIGHT), minor/medium: `TreePrune::run` mixes dispatch, a reap-once flag and a fallback reap. Extract `prune_artifact_under` and an `OrphanReap` value.
- `retention/single_file.rs:315-330` (RIGHT), minor/medium: boolean flag arguments. Use `Option<&FreshVersions>` and small enums.
- `retention/single_file.rs:440-459` (RIGHT), suggestion/medium: `is_still_stale` re-parses a claim it already holds.
- `main.rs:206-222` (RIGHT), suggestion/high: provider, keys and fetcher setup repeated three times. Extract `release_access()`.
- `start_up.rs:66-71` (RIGHT), suggestion/medium: the identification error is flattened to `String`.
- `ownership/mod.rs:123-136` (RIGHT), suggestion/medium: two encodings of the trust rule.

**General Findings**:
- minor: two removal mechanisms in one tree prune. Move `Reaper` onto `CacheFilesystem` and `Removals`.

### Test Coverage

**Summary**: Testing is unusually strong and well suited to the risk: every acceptance criterion is mapped, race windows are deterministic with positive controls, and the default-root invariance is pinned exactly. The gaps: no test shows the prune continuing after a removal failure; the `SafePruneRoot` fail-closed branches cannot be reached through the seam; and some permission tests depend on the host or fail as root.

**Strengths**: deterministic race tests that assert the script fired; positive controls; exact call-list warm-path contracts; the trigger matrix; a table-driven `SafePruneRoot` suite; real-stub `getent` supervision tests; the end-to-end A/B/C scenario and the real-release guard; bash and Rust name coherence.

**Comments**:
- `tests/root_prune.rs:971-997` (RIGHT), major/high: no test that the prune continues after a removal failure. *(Inline.)*
- `ownership/safe_root.rs:202-211` (RIGHT), major/high: the fail-closed error branches are untested. *(Inline.)*
- `main.rs:929-958` (RIGHT), minor/high: the `re_resolve` unit tests run `start_up` against the developer's real cache root, and a poisoned mutex drops the serialisation.
- `tests/root_prune.rs:442-458` (RIGHT), minor/medium: the root guard on permission tests is inconsistent, and modes are not restored on panic. Use a `ModeGuard`.
- `filesystem.rs:294` (RIGHT), minor/high: only one of the three macOS write permissions is tested. *(Merged into the security inline.)*
- `tree/claims.rs:183` (RIGHT), minor/medium: the known fail-open residuals have no tests. *(Merged into the safety inline.)*
- `tests/root_prune.rs:646-666` (RIGHT), suggestion/low: the claim-order assertion also pins the order of signature and entry removal.
- `tests/integration/release_guard/test_release_guard.py:63` (RIGHT), suggestion/medium: the primary `compile_error!` guard is untested. Add a `cargo check --release` failure test.

**General Findings**:
- minor: the default-root invariance test only sees calls made through the port. Add `RecordingLocks` and a ban on direct `std::fs`.
- minor: the cold-path diagnostics (`warn_of` and the log-once refusal) are untested.

### Performance

**Summary**: The launcher's default-root warm path is unchanged. In a relocated root a warm start adds a few syscalls, and the claim write is throttled to about once per 1.4 days. The real risks are the unbounded synchronous prune before `exec` and an unexplained +5.8 ms end-to-end rise in warm-dispatch-8, located in the shim's verification of the launcher.

**Strengths**: `identify` returns early and the plugin-bin claim does nothing; the claim refresh is throttled; the cold path uses `LockPatience::Try` and holds the root lock briefly; the primary-group lookup is memoised; scans are linear over small directories, and one listing is reused.

**Comments**:
- `resolve/mod.rs:223` (RIGHT), major/medium: the synchronous root prune before `exec` has no work budget. *(Deferred to Additional Findings because of the inline cap; see Tradeoff Analysis.)*
- `retention/root_retention.rs:150-167` (RIGHT), minor/medium: the safety walk and tree survey are repeated within one cold materialise. Memoise `SafePruneRoot` and pass the `TreesIndex` along.
- `ownership/accounts.rs:141-150` (RIGHT), minor/medium: `getent` polling every 10 ms, with a worst case of 4 s. Back off the poll interval and use one overall deadline.

**General Findings**:
- major: warm-dispatch-8 shows a +5.8 ms rise in median G-fast, almost all in the shim's minisign verification of the launcher. Re-measure base and head on the same host and compare binary sizes.

### Compatibility

**Summary**: Cross-version care is evident: tree-lock paths are byte-identical, unknown names are kept, and the bash change is 3.2-safe. The gaps are in sharing one root across mixed versions and in the accuracy of the CHANGELOG and PR description.

**Strengths**: an unchanged tree single-flight lock path; a forward-compatible grammar; the bootstrap name-coherence test; `exacl` only on macOS, with Linux reading xattrs; `execfail` with a 3.2-safe `[[ ! -e ]]` check; dependency hygiene (tokio lock-only bump, `exacl` licence, dated zod ignore); all breaking changes in the CHANGELOG.

**Comments**:
- `CHANGELOG.md:161-168` (RIGHT), minor/high: the `bin/accelerator` exit-code break is hidden inside the `cache prune` bullet. Split it into three Breaking bullets.
- `CHANGELOG.md:756-758` (RIGHT), minor/medium: the Migrations note understates the effect on claim-less installs (no re-resolve, 126/127 under `--fail-safe`, offline integrity records).
- `retention/single_file.rs:99-101` (RIGHT), minor/medium: version claims do not protect another version's staged verifier copy. *(Merged into the correctness inline.)*
- `retention/version_claims.rs:197-204` (RIGHT), suggestion/medium: the claim-file name format is now a frozen cross-version contract. Pin it with a test.
- `filesystem.rs:292-306` (RIGHT), minor/low: the macOS ACL read treats every error as unexaminable. Map ENOTSUP to `None`, or name the cause.
- `docs-site/package-lock.json:2735` (RIGHT), minor/high: the lock bump includes astro 7.4.1, a Node ≥22.22.0 floor and transitive major bumps that the description does not mention.
- `.github/workflows/main.yml:54-56` (RIGHT), minor/low: `apt-get install acl` runs without `apt-get update`.
- `ownership/accounts.rs:30` (RIGHT), suggestion/low: the fixed `getent` paths miss NixOS's `/run/current-system/sw/bin/getent`.

**General Findings**:
- minor: an override that does not exist yet is identified as the plugin bin for that whole process. *(Merged into the architecture inline.)*

---
*Review generated by /accelerator:review-pr*
