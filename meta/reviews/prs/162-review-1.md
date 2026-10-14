---
type: "pr-review"
id: "162-review-1"
title: "[0334] Hold tree leases on the cold path and honour unknown liveness"
date: "2026-10-13T16:45:00+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:162"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "correctness", "code-quality", "test-coverage", "safety", "security", "portability", "standards"]
review_number: 1
pr_number: 162
tags: []
last_updated: "2026-10-13T16:45:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #162 - [0334] Hold tree leases on the cold path and honour unknown liveness

**Verdict:** COMMENT

The PR closes all three ways a live daemon's trees could be reclaimed under it. Raising lease descriptors to fd 10 or above (the descriptor floor), leasing the trees `cache ensure` returns on the cold path, and treating `Unknown` liveness conservatively are each sound. The reclaim policy is a pure, fully tested `const fn` core behind an injected `LivenessProbe`. No lens found a blocking defect. The main structural gap is that the extraction is half-done: the probe that tells whether a lease is held (the held probe) and the `.lease` naming rule still live in more than one crate. The remaining findings concentrate on filesystems where `flock` is missing or emulated, which the code accommodates but nothing verifies.

### Cross-Cutting Themes

- **Lease observation is implemented twice** (flagged by: architecture, code-quality, standards) — `tree_lease::observed_held` and the launcher's `probe_liveness` run the same flock probe but classify errors differently. The tests assert through one and the reclaimers decide through the other. Rebuild `probe_liveness` over `observed_held`, or move the three-way `Liveness` result into `tree-lease`.
- **The `.lease` sibling convention has no single owner** (flagged by: architecture, code-quality) — `lease_beside`, `TreePaths::lease`/`LEASE_SUFFIX` and three test `lease_of` helpers each encode the same rule.
- **The sibling check is relational, not containment** (flagged by: correctness, security) — `tree.path` arrives on stdout unchecked. The doc comment's claim that "no other file is created or removed" overstates what is enforced.
- **Filesystems where `flock` is missing or emulated (NFS) are accommodated but unverified** (flagged by: correctness, safety, portability, code-quality) — POSIX-lock emulation may not carry the lock to the forked daemon. The one-hour backstop can expire under a long-running daemon. `utimensat` and sidecar-removal failures are discarded silently. Every verification ran on local filesystems.
- **Fixture descriptor-closing code is duplicated and `/dev/fd`-dependent** (flagged by: code-quality, portability) — two byte-identical copies of `unsafe` helpers, which need `/proc` on Linux.

### Tradeoff Analysis

- **Safety vs simplicity at the `Defer` disposition**: safety wants a failed `utimensat` to fall back to `Keep`, or to renew the backstop while the daemon runs. Both cost extra state and I/O on the reap path. Recommendation: make the failure visible (a `tracing` diagnostic) now. Record in 0334 that daemon lifetime is not bounded by `AGE_BACKSTOP` on filesystems without `flock`, rather than adding renewal machinery.
- **Defence in depth vs coupling for the lease-path guard**: true containment means the executor knows the launcher's trees root. Moving `lease_beside` into `tree-lease` gives one owner of the convention without that coupling. Recommendation: do the latter and narrow the doc comment to what is enforced.

### Strengths

- ✅ The descriptor floor and the compile-time `IDENTITY_FD < LOWEST_LEASE_DESCRIPTOR` assertion sit beside the descriptor layout they protect. Closing the low descriptor before `flock` allows for POSIX-lock emulation.
- ✅ `Unknown` now defers to the age backstop instead of counting as free. The default is fail-safe, and no live tree is removed just because the filesystem cannot answer.
- ✅ The three reclaim rules are exhaustive pure `const fn` matches, with one unit test per cell and both sides of the `AGE_BACKSTOP` boundary.
- ✅ `LivenessProbe` follows the existing `Clock` injection pattern and makes `Unknown` testable without exotic filesystems.
- ✅ `TreeLeases::hold` takes the lease first and then re-checks the tree. It separates gone trees from unleasable ones, and `remove_unheld_lease` deletes only on an affirmative `Ok(false)`.
- ✅ `ensure_and_pin` makes at most two attempts, drops the first attempt's leases before retrying, and writes no sticky marker for a reclaimed tree.
- ✅ `tree-lease` follows every point of the library-crate registration checklist (role, `pup.ron` rule with probe pair, `_EXEMPT_MEMBERS`).
- ✅ The real-process cold and warm tests drive the real `DaemonSpawner` and `ExecClient`. The warm test was confirmed red before the descriptor floor.
- ✅ `LeaseError` carries the action, the path and the `io::Error` as `source()`, which is better than the old string-formatted detail.

### General Findings

- 🔵 **Code Quality / Safety**: best-effort operations (`restart_backstop`'s `utimensat`, sidecar removal, `remove_unheld_lease`) discard their errors silently. On NFS, a failed `utimensat` leaves a deferred generation with no grace and no trace. Emit a `tracing` diagnostic.
- 🔵 **Architecture**: the descriptor floor protects leases only from children that never use a fixed fd ≥ 10. Record it as an invariant of the daemon-spawn contract.
- 🔵 **Security**: lease descriptors deliberately survive `exec` and propagate to every daemon descendant. That is harmless today (`lsof` showed Chromium closes them), but a future runtime could keep a generation pinned indefinitely.
- 🔵 **Portability**: the NFS and network-filesystem claims in the comments are inferred, not tested. Consider documenting the supported filesystem classes for `ACCELERATOR_CACHE_DIR`, or doing a one-off NFS check.

### Additional Findings

- 🔵 `cli/tree-lease/src/lib.rs:137` — the error branches of `observed_held` are untested (test-coverage)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/lease.rs:194` — `HeldLease` and `From<LeaseError>` impls are duplicated across the `unix` and `stubs` modules (code-quality)
- 🔵 `cli/design-cli/src/executor.rs:382` — a re-ensure after a reclaimed tree leaves no diagnostic trail (code-quality)
- 🔵 `cli/design-adapters/src/ensure.rs:111` — a second crate now creates and removes files in the launcher-owned cache root (architecture)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:712` — deferral silently depends on `utimensat` succeeding (safety)
- 🔵 `cli/tree-lease/src/lib.rs:145` — probing a lease the calling process itself holds is unsafe under POSIX-lock emulation (portability)
- 🔵 `cli/design-adapters/src/lib.rs:22` — partial root re-exports of `ensure` types leave callers with mixed import paths (standards)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:219` — generation mtime now carries two meanings (architecture)
- 🔵 `cli/design-adapters/src/ensure.rs:85` — a catch-all `_` arm hides what "tree still present" means (code-quality)
- 🔵 `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:706` — "remove a generation with its sidecars" is spelled out in two reclaimers (code-quality)
- 🔵 `cli/launcher/tests/tree_resolution.rs:1282` — the unclaimed table is asserted twice, as unit and integration tests (test-coverage)
- 🔵 `cli/tree-lease/src/lib.rs:157` — the lease open follows symlinks (security)
- 🔵 `cli/design-cli/src/executor.rs:111` — the `PinnedTrees` doc comment restates its fields (standards)
- 🔵 `cli/tree-lease/src/lib.rs:131` — `observed_held` breaks the verb-led naming and liveness vocabulary (standards)
- 🔵 `cli/tree-lease/src/lib.rs:12` — `LOWEST_LEASE_DESCRIPTOR` is typed `i32` rather than `RawFd` (standards)

---
*Review generated by /review-pr*

## Inline Comments

### `cli/tree-lease/src/lib.rs:131-155` — Lease observation implemented twice with divergent semantics (architecture, code-quality)
**Severity**: major | **Confidence**: high | **Lens**: architecture, code-quality

🟡 **Architecture**

`tree-lease` takes on the holding half of the lease protocol and adds `observed_held`. The launcher's production reclaimers still go through their own `probe_liveness` in `launcher/.../tree/lease.rs` via `FlockProbe`. That probe does the same thing but maps errors differently: open errors other than `NotFound`, and `ENOLCK`, become `Unknown`. The doc here calls `observed_held` the one observation every reclaimer and test agrees on, but no reclaimer uses it.

**Impact**: whether a live daemon's tree may be deleted is a safety-critical decision, and its observation side now has two implementations in two crates. A change to how leases are held could update one and silently leave the other stale.

🟡 **Code Quality**

The tests assert liveness through `observed_held`, while production decides through `probe_liveness`. They can drift apart without any test noticing.

**Suggestion**: move `Liveness` and the three-way probe into `tree-lease`, and derive `observed_held` from it (or rebuild `probe_liveness` as `Ok(true) => Held, Ok(false) => Free, Err(_) => Unknown` over `observed_held`). `FlockProbe` then becomes a thin adapter.

---

### `cli/design-adapters/src/ensure.rs:79-82` — Sibling check does not confine the lease to the cache layout (correctness, security)
**Severity**: minor | **Confidence**: medium | **Lens**: correctness, security

🔵 **Correctness**

`lease_beside` only checks that `tree.lease == tree.path + ".lease"`. `tree.path` also comes from stdout and is never confined to the trees root. Suppose a malformed or other-layout-version `cache ensure` names `/home/u/notes` and that path does not exist. `hold_shared_lease` creates `/home/u/notes.lease`, and `remove_unheld_lease` then deletes it. If `/home/u/notes.lease` already exists as an unrelated, unlocked file, it is deleted.

🔵 **Security**

The practical risk is low, because the launcher is the product's own binary and its stdout already chooses which runtime runs. The guarantee in the doc comment and the PR description is still narrower than it reads.

**Suggestion**: require the tree path to sit directly under the expected trees root, or its file name to match the generation grammar, before creating or removing anything. Otherwise, narrow the doc comment on `lease_beside` to the relational guarantee actually enforced.

---

### `cli/tree-lease/src/lib.rs:107-110` — Under POSIX-lock emulation a forked daemon may not hold the lease at all (correctness)
**Severity**: minor | **Confidence**: low | **Lens**: correctness

🔵 **Correctness**

The comment accommodates `flock` emulated with POSIX locks, as on the Linux NFS client. POSIX locks belong to the process and are not inherited across `fork`. If the emulation keeps those semantics, the detached daemon inherits the descriptor but not the lock. Once the client process exits, the probe would then read `Free`, not `Unknown`, so the new `Unknown` rules never engage. `reclaim_unclaimed` and `reap_superseded` would reclaim the live tree at once. Whether Linux's NFS emulation tracks the owner per process or per open file description has not been verified.

**Suggestion**: verify on an NFS mount. If confirmed, either document NFS cache roots as unprotected or have the daemon re-take its own leases at start-up.

---

### `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:207-208` — Unknown backstop can expire under a still-running daemon (safety)
**Severity**: minor | **Confidence**: medium | **Lens**: safety

🔵 **Safety**

`orphan_is_reclaimable` reclaims an `Unknown` generation after `AGE_BACKSTOP` (1h). `repair` restarts the backstop once, when it defers, and never again. A daemon kept busy, so never reaching its 10-minute idle exit, can outlive the backstop on a filesystem without `flock`. The next `materialise` or `prune` then deletes the tree under it. The failure is recoverable by re-ensuring, but it surfaces as confusing errors mid-session.

**Suggestion**: either renew the backstop from the cold path's unleasable branch, or record in 0334 that daemon lifetime is not bounded by `AGE_BACKSTOP`.

---

### `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:776-779` — `reclaim_unclaimed`'s Unknown gate counts from extraction (safety)
**Severity**: minor | **Confidence**: medium | **Lens**: safety

🔵 **Safety**

For `Unknown`, `generation_is_aged` measures from the extraction mtime, so a pointed-at generation extracted long ago is already aged. Once its claim lapses, nothing protects it. Example: `cache prune --older-than 0` from another launcher on a shared root without `flock` deletes a tree that a cold-path daemon is running unleased.

**Suggestion**: have the executor's unleasable branch refresh the claim or the mtime, so the age gate measures time since last use. Otherwise, add this case to 0334's accepted risks.

---

### `cli/design-adapters/src/ensure.rs:103-109` — `.lease` sibling convention duplicated outside the launcher layout (architecture, code-quality)
**Severity**: minor | **Confidence**: high | **Lens**: architecture, code-quality

🔵 **Architecture**

`lease_beside` hard-codes the rule the launcher owns as `TreePaths::lease` / `LEASE_SUFFIX`. If the launcher's sidecar layout changes, every cold-path tree classifies as `OutsideTreeLayout` and runs unleased. The only symptom is a warning line.

🔵 **Code Quality**

The same rule also appears in three test `lease_of` helpers (`tree_leases.rs`, `executor_runtime.rs`, `tree_resolution.rs`).

**Suggestion**: let `tree-lease` own the convention, as `pub fn lease_beside(tree: &Path) -> PathBuf` or a suffix constant, and derive `TreePaths::lease`, `TreeLeases::hold` and the test helpers from it.

---

### `cli/design-cli/src/executor.rs:178-181` — Lease lifetime rests on a `_leases` binding that looks removable (code-quality)
**Severity**: minor | **Confidence**: medium | **Lens**: code-quality

🔵 **Code Quality**

The leases survive the daemon spawn and client `exec` only because the destructuring binds `leases: _leases`. Tidying it to `leases: _` or `..` drops every lease at once and silently brings back the cold-path bug. Clippy will not flag it, and only the slower `executor_runtime` tests would catch it.

**Suggestion**: keep `PinnedResolution` whole (`let pinned = …; match &pinned.resolution { … }`), or bind it to a name that states its job, such as `_pinned_until_exec`.

---

### `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:80-82` — No test checks that a spared generation keeps its sidecars (test-coverage)
**Severity**: minor | **Confidence**: medium | **Lens**: test-coverage

🔵 **Test Coverage**

`an_aged_orphan_is_reclaimed_with_its_sidecars` proves that sidecars go with a reclaimed generation. Nothing proves that a held or young orphan, or a deferred superseded generation, keeps them. The held and young tests plant no sidecars, and `a_superseded_unknown_generation_is_kept` checks only `old.path`. If the sidecar loop moved outside the reclaim branch, every test would still pass. Deleting a live generation's `.lease` breaks liveness detection: the next probe opens a fresh inode and reads it as free.

**Suggestion**: plant all three sidecars in one held and one young orphan test. In the superseded `Unknown` test, assert that `paths.lease(&generation)` and both attestation files survive.

---

### `cli/launcher/tests/fixtures/launcher_fixture_lease_holder.rs:59-94` — Descriptor-closing helpers duplicated and `/dev/fd`-dependent (code-quality, portability)
**Severity**: minor | **Confidence**: medium | **Lens**: code-quality, portability

🔵 **Code Quality**

`close_inherited_descriptors` and `descriptors_above_stdio` are byte-for-byte copies of the same functions in `design_cli_fixture_warm_handover.rs`, including the `unsafe` block, its SAFETY argument and the `Dir::new` vs `Dir::read_from` subtlety.

🔵 **Portability**

Both copies list `/dev/fd`. On Linux that is `/proc/self/fd`, which is missing in chroots, Nix or bubblewrap sandboxes, and containers without `/proc`. There the fixtures exit with code 70, which reads like a lease regression.

**Suggestion**: move the pair into a shared test-support crate. Fall back to `/proc/self/fd`, or use `close_range(3, ~0)` where available.

---

### `cli/design-cli/Cargo.toml:20-28` — Test fixture binary adds production dependency edges (architecture)
**Severity**: minor | **Confidence**: medium | **Lens**: architecture

🔵 **Architecture**

`design-cli-fixture-warm-handover` is a `[[bin]]` of the shipped package, so `tree-lease` and `rustix` become normal dependencies of `design-cli`. The manifest comment says the fixture is their only user. The crate-dependency register and cargo-deny see this as a production edge, and an accidental `use tree_lease` in `design-cli/src` would not be flagged.

**Suggestion**: host the fixture in a crate where those dependencies are legitimate, such as `tree-lease` or a `*-test-support` crate. If the launcher's fixture-bin pattern is the accepted convention, record that.

---

## Per-Lens Results

### Architecture

**Summary**: The `tree-lease` extraction is sound and guarded by `pup` and a compile-time assertion, and the reclaim policy is a clean functional core. The extraction is incomplete: `probe_liveness`/`Liveness` stay in the launcher, duplicated by `observed_held`, and the `.lease` naming convention is duplicated across the process boundary. A fixture binary adds production dependency edges to `design-cli`.

**Strengths**: one dependency-light crate enforced by boundary tooling; the descriptor-floor assertion beside the layout; pure `const fn` rules behind injected `LivenessProbe` and `Clock`; lease lifetime modelled explicitly (`PinnedResolution`, `PinnedTrees`, `TreeLeases`); lease paths from stdout treated as untrusted; tradeoffs stated openly.

**Comments**:
- `cli/tree-lease/src/lib.rs:123-138` (major, high) — lease observation split between `tree-lease` and the launcher, with divergent semantics.
- `cli/design-adapters/src/ensure.rs:103-109` (minor, high) — the `.lease` sidecar naming convention is duplicated.
- `cli/design-adapters/src/ensure.rs:111-115` (minor, medium) — a second crate now creates and removes files in the launcher-owned cache root; document the set of writers, or leave cleanup to the launcher's reaper.
- `cli/design-cli/Cargo.toml:20-36` (minor, medium) — a test fixture binary adds production dependency edges (re-anchored to 20-28: the range spanned two hunks).
- `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:219-225` (suggestion, low) — generation mtime now carries two meanings, extraction and deferral.

**General Findings**:
- (suggestion) The descriptor floor relies on children never using a fixed fd ≥ 10; record it as a spawn-contract invariant.

### Correctness

**Summary**: The core logic is sound: the descriptor floor, closing the low descriptor before locking, clearing CLOEXEC after it, the re-check after leasing, dropping leases before the retry, and lease lifetime through spawn and exec. The rules are total over `Held`/`Free`/`Unknown`. Two gaps remain: the sibling check does not confine paths to the layout, and NFS POSIX-lock emulation may not carry the lock to a forked daemon.

**Strengths**: `opened_above_floor` drops the low descriptor before `flock`, and no failure path leaves a lock without an owner; an unleasable tree is kept separate from a gone one; `ensure_and_pin` drops the failed attempt's leases before re-ensuring, with a bounded loop; exhaustive `const fn` matches; `checked_sub` makes a future mtime read as young; `_leases` binds rather than discards.

**Comments**:
- `cli/design-adapters/src/ensure.rs:103-113` (minor, medium) — the sibling check does not confine the lease path to the cache layout (merged with security, re-anchored to 79-82).
- `cli/tree-lease/src/lib.rs:107-110` (minor, medium; lowered to low confidence in aggregation, since NFS emulation's lock-owner semantics are unverified) — under POSIX-lock emulation a forked daemon never holds the lease.

**General Findings**: none.

### Code Quality

**Summary**: The change is well structured, with a focused crate, a structured `LeaseError`, readable pure rules and a clean `LivenessProbe` seam. The main concern is duplication: the flock observation and the `.lease` convention each live in several places. Smaller implicit spots: cfg-duplicated impls, copied fixture helpers, a silent retry, and a drop-sensitive `_leases` binding.

**Strengths**: pure `const fn` rules; `LivenessProbe` follows the `Clock` injection pattern; `LeaseError` with `source()`; `SupersededDisposition` names `repair`'s outcomes; the compile-time descriptor assertion; the `EnsureFailure` enum.

**Comments**:
- `cli/tree-lease/src/lib.rs:131-155` (major, high) — two independent implementations of the same flock liveness probe.
- `cli/design-adapters/src/ensure.rs:104-110` (minor, medium) — the `.lease` sibling convention is re-derived in several places.
- `cli/design-adapters/src/ensure.rs:85-98` (suggestion, medium) — a catch-all `_` arm hides what "tree still present" means; extract `has_gone`.
- `cli/design-cli/src/executor.rs:178-181` (minor, medium) — lease lifetime depends on a `_leases` binding.
- `cli/design-cli/src/executor.rs:382-384` (minor, medium) — the re-ensure after a reclaimed tree leaves no diagnostic trail; emit a `warning:`.
- `cli/launcher/src/launch/outbound/resolve/tree/lease.rs:194-202` (minor, high) — `HeldLease` and `From<LeaseError>` impls are duplicated across the unix and stub modules; hoist them to the top level.
- `cli/launcher/tests/fixtures/launcher_fixture_lease_holder.rs:59-94` (minor, medium) — descriptor-closing helpers copied verbatim between fixtures.
- `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:706-711` (suggestion, medium) — extract `reap::reclaim_generation`.

**General Findings**:
- (suggestion) Best-effort filesystem operations fail silently; emit `tracing` diagnostics.

### Test Coverage

**Summary**: Coverage is unusually strong and proportionate to the risk: every rule cell is unit-tested, the wiring is proven at the entry points, real processes check that leases survive the client's exit, and the fixtures are isolated against leaked descriptors. The gaps: no test checks that spared generations keep their sidecars, the error branches of `observed_held` are untested, and the unclaimed table is asserted twice.

**Strengths**: per-cell `const fn` tests including the `AGE_BACKSTOP` boundary; the `LivenessProbe` seam avoids flakiness from real locks; tests in both directions for the backstop restart; black-box executor tests counting `cache ensure` calls; a red-then-green warm-path test plus a unit check of the descriptor floor; both halves of the outside-the-layout rule; deliberate isolation of lease-holding tests; the `pup` probe pair.

**Comments**:
- `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:80-82` (minor, medium) — no test checks that a spared generation keeps its sidecars.
- `cli/tree-lease/src/lib.rs:137-153` (minor, medium) — the error branches of `observed_held` are untested; observe an unopenable path and assert `is_err()`.
- `cli/launcher/tests/tree_resolution.rs:1282-1310` (suggestion, medium) — the five unclaimed integration tests duplicate the unit table; reduce them to two wiring cases.

**General Findings**: none.

### Safety

**Summary**: The PR closes three paths to deleting a live daemon's tree and fails safe throughout. The residual risk is narrow and documented: on filesystems without `flock`, the one-hour backstop is the only protection, and it can expire under a running daemon.

**Strengths**: `Unknown` defers or keeps, and never deletes at once; the sibling check; `remove_unheld_lease` deletes only on `Ok(false)`; a non-`NotFound` stat error counts as present; no sticky marker after a reclaimed tree, with retries bounded; the descriptor floor with its compile-time check; `restart_backstop` is best-effort.

**Comments**:
- `cli/launcher/src/launch/outbound/resolve/tree/reap.rs:207-208` (minor, medium) — the `Unknown` backstop can expire under a still-running daemon.
- `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:776-779` (minor, medium) — `reclaim_unclaimed`'s `Unknown` gate counts from extraction.
- `cli/launcher/src/launch/outbound/resolve/tree/resolver.rs:712-713` (suggestion, low) — deferral silently depends on `utimensat`; fall back to `Keep` or emit a diagnostic.

**General Findings**: none.

### Security

**Summary**: The change is local locking for a single-user cache, with no network or auth surface. The one trust boundary is lease paths from `cache ensure`'s stdout, guarded relationally. No exploitable vulnerability; the findings are hardening suggestions.

**Strengths**: subprocess-supplied paths treated as untrusted and tested; sidecars created with mode 0600 and no truncation; `remove_unheld_lease` fails closed; the descriptor floor; a fail-safe `Unknown`.

**Comments**:
- `cli/design-adapters/src/ensure.rs:156-168` (suggestion, medium) — the lease-path trust check is relational, not containment-based (out of diff range; merged with correctness at `ensure.rs:79-82`).
- `cli/tree-lease/src/lib.rs:3140-3151` (suggestion, low) — the lease open follows symlinks; add `O_NOFOLLOW` (the agent reported a diff-relative line; re-anchored to `open_lease_file` at line 157 and deferred).

**General Findings**:
- (suggestion) Inheritable lease descriptors propagate to every daemon descendant; consider re-setting `FD_CLOEXEC` in the daemon before it launches the browser.

### Portability

**Summary**: The PR makes the cache safer on filesystems where `flock` is missing or emulated: `Unknown` defers, the cold path warns and continues, and non-Unix stubs return typed errors rather than panicking. The release targets are darwin and linux-musl, so the Unix-only code fits. What remains: the fixtures need `/dev/fd`, and lease probing is not safe under emulation.

**Strengths**: `Unknown` falls back to the backstop; the daemon runs unleased rather than failing; non-Unix stubs return `ErrorKind::Unsupported` instead of `unimplemented!()`; the low descriptor is closed before locking; `utimensat` is best-effort and cfg-gated; the unstattable-tree test skips under root.

**Comments**:
- `cli/launcher/tests/fixtures/launcher_fixture_lease_holder.rs:73-77` (minor, medium) — the fixtures assume `/dev/fd` exists (merged into the code-quality comment at 59-94).
- `cli/design-cli/tests/fixtures/design_cli_fixture_warm_handover.rs:101-105` (suggestion, medium) — the duplicated `/dev/fd` enumeration (merged).
- `cli/tree-lease/src/lib.rs:145-148` (minor, low) — probing a lease the calling process itself holds is unsafe under POSIX-lock emulation; document the precondition or converge on `tree_lease`.

**General Findings**:
- (suggestion) The NFS and network-filesystem behaviour is asserted but verified only on local filesystems.

### Standards

**Summary**: The `tree-lease` registration follows the library-crate checklist point for point, and the fixture naming, test naming, stderr prefixes and error impls all match existing conventions. The findings are small: export paths, naming, and one doc comment that adds nothing.

**Strengths**: the full checklist compliance; fixture `<crate>-fixture[-variant]` naming; a `warning:` prefix; the `LeaseError` pattern matches `LockError`/`JsonError`; behaviour-describing test names; comments that explain why, with no references to work items or ADRs.

**Comments**:
- `cli/design-adapters/src/lib.rs:22-24` (minor, medium) — only some `ensure` types are re-exported at the crate root.
- `cli/design-cli/src/executor.rs:111` (suggestion, medium) — the `PinnedTrees` doc comment restates its fields.
- `cli/tree-lease/src/lib.rs:131` (suggestion, low) — `observed_held` naming; prefer `probe_held` or `is_held`.
- `cli/tree-lease/src/lib.rs:12` (suggestion, low) — `LOWEST_LEASE_DESCRIPTOR` should be `RawFd`.

**General Findings**: none.

---
*Review generated by /accelerator:review-pr*
