---
type: "plan-review"
id: "2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness-review-1"
title: "Plan Review: Tree Leases on the Cold Path and Under Unknown Liveness Implementation Plan"
date: "2026-10-12T22:22:04+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-10-12-0334-tree-leases-cold-path-and-unknown-liveness"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "safety", "portability", "standards"]
review_number: 1
review_pass: 5
tags: ["runtime", "cache", "lease", "prune", "tree-lease"]
last_updated: "2026-10-12T23:16:07+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Tree Leases on the Cold Path and Under Unknown Liveness Implementation Plan

**Verdict:** REVISE

The plan's diagnosis is right. All seven lenses accept the fd 3 clobber analysis, the Held/Free/Unknown × age policy and the two-strand phasing, and almost every cited line number checks out against the source. It is not ready to implement as written. The descriptor floor calls `rustix::io::fcntl_dupfd`, which rustix 1.1.4 compiles only for `espidf`, so Phase 2 does not build on any supported target; this was verified in `rustix-1.1.4/src/io/fcntl.rs:138`. Most of the other findings sit in the test harness: lease descriptors without `CLOEXEC` leak between parallel tests, the exit waits cannot see zombies, and several "red first" claims are false.

### Cross-Cutting Themes

- **`fcntl_dupfd` is unavailable, and the lock order is fragile** (flagged by: portability, correctness): `fcntl_dupfd` is gated to `espidf`, so the floor has to use `fcntl_dupfd_cloexec` plus the existing `clear_cloexec`. Both lenses also recommend duplicating before `flock`, so the lock is taken only on the surviving descriptor. That order holds whether the filesystem gives `flock` its own semantics or emulates it with POSIX locks (NFS).
- **Inheritable test leases leak into concurrent children** (flagged by: safety, test-coverage, correctness): `spawn_lease_holder` takes leases without `CLOEXEC` inside the test process, which runs tests on parallel threads. A `sleep 300` spawned by another test, or a `minisign` child, inherits them and pins generations it does not own. The result is intermittent failures of the "reclaims" assertions.
- **Exit waits cannot see zombies** (flagged by: safety, test-coverage, correctness, portability): `kill(pid, 0)` succeeds on a zombie. Phase 6's holder is a direct child that nothing reaps, and the "freed after `killpg`" assertions race the process exiting. Both can be fixed by polling `observed_held` or calling `Child::wait`.
- **The cold path skips lease-then-recheck** (flagged by: architecture, safety, correctness): `TreeLeases::hold` opens with `create(true)` and never checks that the tree still exists. A prune that runs in the gap leaves behind a `.lease` that is never reclaimed and a daemon spawned on a missing tree.
- **False "red first" claims** (flagged by: test-coverage, correctness): `an_unleasable_tree_still_starts_the_daemon` and `an_unclaimed_unknown_generation_is_reclaimed_once_aged` already pass. The Phase 4 seam tests fail to compile rather than fail on behaviour. Before Phase 3 the cold-path red fails on a missing lease file, not on "not held".
- **The liveness policy is split three ways** (flagged by: architecture, code-quality): the three reclaimers apply three different rules across two modules. The plan calls the predicate "shared" though only one reclaimer uses it, and the predicate does I/O.
- **Skipped leases leave no trace** (flagged by: code-quality, safety): `filter_map(.ok())` discards every `LeaseError` and records nothing.
- **design-cli depends on `tree-lease` only for a fixture** (flagged by: architecture, code-quality, standards): the dependency is declared in both `[dependencies]` and `[dev-dependencies]`.

### Tradeoff Analysis

- **Test fidelity vs. isolation**: the stand-in daemon should mimic the executor's inheritance faithfully, which is why the plan takes leases in-process and leaves them inheritable. To be safe under parallel tests, that inheritance has to be confined to the holder. A small fixture binary that takes its own leases, as Phase 2's warm fixture already does, keeps the fidelity and removes the leak. Recommended.
- **Exempting `tree-lease` from the public-API pin vs. pinning it**: the `process-probe` precedent supports exempting it. However, `LOWEST_LEASE_DESCRIPTOR` is now a constant that `design-adapters` relies on implicitly. A `const` assert beside `IDENTITY_FD` would enforce that link more strongly than a snapshot, so keeping the exemption is defensible once the assert exists.

### Findings

#### Critical

- 🔴 **Portability + Correctness**: `rustix::io::fcntl_dupfd` does not exist on macOS or Linux in rustix 1.1.4
  **Location**: Phase 2 §4: The descriptor floor; Key Discoveries
  In rustix 1.1.4 the function is gated `#[cfg(target_os = "espidf")]`, and only `fcntl_dupfd_cloexec` is exported for Unix targets. `tree-lease` will not compile on any release target, and the Key Discovery that `F_DUPFD` clears `CLOEXEC` for free does not apply.

#### Major

- 🟡 **Safety + Test Coverage + Correctness**: Inheritable in-process leases leak into concurrent tests' long-lived holders
  **Location**: Phase 6 §2: Stand-in daemon
  The lease descriptors stay open without `CLOEXEC` in the test process while other threads spawn children. One test's `sleep 300` can then pin another test's generation, so the "reclaims" tests fail intermittently.
- 🟡 **Code Quality**: `TreeLeases.held` is never read, so the build fails under `warnings = "deny"`
  **Location**: Phase 3 §2
  Rustc reports `dead_code`, and `cli/Cargo.toml:231` denies warnings. The snippet needs an underscore field name or a tuple struct with a targeted allow.
- 🟡 **Safety**: `killpg` targets a pid parsed from `server-info.json` without validating it
  **Location**: Phase 2 §2: `DaemonGroup`
  `killpg(0)` signals the test runner's own group, and `killpg(1)` signals every process the user owns. The fix is to refuse `pid <= 1`, confirm `getpgid(pid) == pid`, and build the guard before the first assertion.
- 🟡 **Test Coverage + Correctness**: `an_unleasable_tree_still_starts_the_daemon` passes before the change and asserts nothing about leases
  **Location**: Phase 3 §1
  Today the executor ignores leases entirely, so the test is green from the start. It also does not prove that the other tree is still held. Separately, the fake launcher never records the arguments it was called with, although `a_driver_exported_without_its_browser_…` relies on them.
- 🟡 **Test Coverage**: The cold-path red run fails on a missing lease file, not on "not held"
  **Location**: Phase 2 §2: `observed_held`
  `observed_held` opens the lease file, and the fake launcher never creates one. It also returns `false` on any `flock` error other than `EWOULDBLOCK`, which lets a broken probe satisfy the "freed" assertions.
- 🟡 **Test Coverage**: Post-`killpg` "free" assertions race the daemon's exit
  **Location**: Phase 2 §3 / Phase 3 §1
  `killpg` is asynchronous, and the plan waits for exit only inside `Drop`. The fix is an explicit bounded `terminate()`, or polling `observed_held` until it reads free.
- 🟡 **Portability**: The warm-path red run assumes the test runner leaks no inheritable descriptors
  **Location**: Phase 2 §1, §3
  An fd 3 inherited from mise, cargo, llvm-cov or an IDE shifts the driver lease to fd 4 or above, so the red result depends on the host. The fixture should close descriptors 3–9, or assert that fd 3 is free, before it takes the leases.
- 🟡 **Code Quality**: The liveness policy has three variants across two modules under a misleading predicate name
  **Location**: Phase 5 §2
  The plan calls the predicate "shared by both reclaimers", but only `reclaim_unclaimed` uses it. Its name, `reclaimable_without_a_holder`, also misdescribes `Unknown`. The three rules should be named side by side in `reap.rs`. Architecture rated the same issue minor and adds that the predicate should be pure.
- 🟡 **Standards**: The plan edits ADR-0069, which is accepted and therefore immutable
  **Location**: Phase 1 §2
  ADR-0069 has `status: "accepted"`, and line 79 does enumerate the crates, so the conditional edit would fire. The edit should be dropped.

#### Minor

- 🔵 **Portability + Correctness**: Lock-then-dup-then-close loses the lock where `flock` is emulated with POSIX locks
  **Location**: Phase 2 §4
  The fix is to reorder the steps: open, duplicate onto the floor, drop the low descriptor, then `flock` the raised one.
- 🔵 **Architecture + Safety + Correctness**: The cold-path lease is taken without lease-then-recheck
  **Location**: Phase 3 §2
  This leaves an unreclaimable `.lease` and a daemon running on a missing tree. The work item's accepted-gap assumption also rests on "`cache ensure` refreshed the claim", but the hit path (`resolver.rs:584-590`) does not call `write_claim`.
- 🔵 **Correctness**: Deferring `Unknown` superseded generations to `reap_orphans` orphans their sidecars
  **Location**: Phase 5 §2
  `reap_orphans` removes only the directory (`reap.rs:68-78`), so `.sealed`, `.sealed.sig` and `.lease` remain forever.
- 🔵 **Correctness**: The `rg 'probe_liveness\('` criterion cannot pass
  **Location**: Phase 4 Success Criteria
  The unit tests left in `lease.rs` (`:208-254`) and the non-unix stub (`:283`) also match.
- 🔵 **Test Coverage**: The Phase 4 seam tests fail to compile rather than fail on behaviour, and `materialise_with_waiter`'s `reap_orphans` call site is untested
  **Location**: Phase 4 §3
- 🔵 **Test Coverage**: The aged-`Unknown` row already passes, and the reap-once-aged test needs an intermediate "still exists" assertion
  **Location**: Phase 5 §1
- 🔵 **Test Coverage**: The backstop boundary is untested, and `fresh()` depends on `checked_sub` underflow
  **Location**: Phase 5 §1
- 🔵 **Test Coverage**: `minisign_or_skip!` can turn a red run into a silent pass outside mise
  **Location**: Phases 4–6
- 🔵 **Test Coverage**: The cross-launcher tests should pin `second` to `&FlockProbe` and assert the holder's inheritance before pruning
  **Location**: Phase 6 §1, §3
- 🔵 **Safety / Portability**: Waits on `kill(pid, 0)` never end for zombies
  **Location**: Phase 2 §2 and Phase 6 §2
  The fix is to keep the `Child` and `wait()` on it, or to poll `observed_held`. The wait must never panic in `Drop`.
- 🔵 **Portability**: Executor runtime tests fail rather than skip on musl and NixOS hosts
  **Location**: Phase 2–3 harness
- 🔵 **Portability**: A crate-level `LOWEST_LEASE_DESCRIPTOR: RawFd` breaks the non-unix stub
  **Location**: Phase 2 §4
- 🔵 **Architecture**: The descriptor floor is coupled to `IDENTITY_FD` only through a doc comment
  **Location**: Phase 2 §4
  The fix is `const _: () = assert!(IDENTITY_FD < tree_lease::LOWEST_LEASE_DESCRIPTOR);` in `design-adapters`.
- 🔵 **Code Quality**: Leases travel through a `RefCell` side channel and return as an anonymous tuple
  **Location**: Phase 3 §3
- 🔵 **Code Quality**: `pub fn lease_failure` and repeated `map_err` calls instead of `From`; `LeaseError` drops its source error
  **Location**: Phase 1 §3
- 🔵 **Code Quality / Standards**: design-cli gains a production dependency only for the fixture, and it is declared twice
  **Location**: Phase 2 §1–2
- 🔵 **Standards**: The fixture binary's name departs from the `<crate>-fixture` pattern, and the cited precedent (`accelerator-fixture`) contradicts the rule
  **Location**: Phase 2 §1; Key Discoveries
- 🔵 **Standards**: `libc` is added to the launcher, which uses only `rustix`
  **Location**: Phase 6 §2
- 🔵 **Standards**: The plan calls `LivenessProbe` "the port" but places it in an outbound module
  **Location**: Phase 4 §1

#### Suggestions

- 🔵 **Code Quality / Safety**: Log each skipped cold-path lease as a `warning:` line or in the bootstrap log
  **Location**: Phase 3 §2
- 🔵 **Code Quality**: The Phase 4 seam tests duplicate rows of the Phase 5 table, and `aged()` should use `BACKSTOP_SECONDS`
  **Location**: Phases 4–5
- 🔵 **Architecture / Test Coverage**: Three hand-written "observed held" helpers; consider one helper exported by `tree-lease`
  **Location**: Phases 1–3
- 🔵 **Standards**: The release-staging question is already answered by `_CLI_RELEASE_BINARIES` (`tasks/build.py:39-64`)
  **Location**: Phase 2 §1
- 🔵 **Standards**: Reconsider the public-API exemption now that the crate exports a constant other crates rely on
  **Location**: Phase 1 §2
- 🔵 **Standards**: The test name `holding_a_resolved_tree_s_lease_pins_it` contains an awkward possessive
  **Location**: Phase 3 §2
- 🔵 **Portability**: `lsof` is missing on minimal Linux installs; give `/proc/<pid>/fd` as an alternative
  **Location**: Manual steps
- 🔵 **Safety**: The manual `prune --older-than 0` step should use a scratch cache root
  **Location**: Manual Testing Steps
- 🔵 **Test Coverage**: The floor unit test may pass before the change by chance
  **Location**: Phase 2 §4
- 🔵 **Test Coverage**: Document `executor_runtime.rs`'s scope against `executor_preflight`'s lane convention
  **Location**: Phase 2 §2

### Strengths

- ✅ The fd 3 clobber diagnosis is correct, and the fix lives inside the primitive, so warm and cold paths both benefit. The Phase 2 stop-and-re-plan gate guards against fixing the wrong cause.
- ✅ `tree-lease` is a proper leaf technical library with its own `LeaseError`. `HeldLease` is implemented on the foreign type, so `launch::core` stays untouched, and the six-item crate checklist is followed fully.
- ✅ The `LivenessProbe` seam mirrors the existing `Clock` injection and makes `Unknown` testable without NFS.
- ✅ `reclaimable_without_a_holder` encodes the work item's table exactly. Keeping `reap_orphans`' own rule correctly preserves the young-orphan backstop, and every uncertain signal resolves to keeping the tree.
- ✅ The end-to-end harness uses the real executor, `DaemonSpawner` and `ExecClient`, with only `node` and the launcher stubbed, and it tests the work item's definition of "observed held".
- ✅ The phasing has two independent strands, each phase mergeable with `mise run` green, and an explicit map from acceptance criteria to tests.
- ✅ Cited line numbers and current-state claims are almost all accurate.

### Recommended Changes

1. **Use `fcntl_dupfd_cloexec` and dup before locking** (addresses: `fcntl_dupfd` missing; lock-then-dup ordering)
   Rewrite `hold_shared_lease` in this order: open, `fcntl_dupfd_cloexec(.., LOWEST_LEASE_DESCRIPTOR)`, drop the low descriptor, `flock` the raised one, then `clear_cloexec`. Correct the Key Discovery, and put the constant inside the `cfg(unix)` module.
2. **Give the Phase 6 holder its own leases** (addresses: inheritable lease leak)
   Use a small launcher fixture binary that takes `hold_shared_lease` on its arguments and then sleeps, so the test process never holds an inheritable lease while it spawns. Add a note to Flake discipline.
3. **Harden the process lifecycle in the harness** (addresses: unvalidated `killpg` pid; zombie waits; post-`killpg` race)
   `DaemonGroup::new` refuses `pid <= 1` and checks `getpgid(pid) == pid`, and is built before any assertion. Add an explicit `terminate()` that polls `observed_held` until it reads free, with a 60s bound, and call it before the "freed" assertions. `LeaseHolder` keeps its `Child` and `wait()`s on it. `Drop` never panics.
4. **Give `observed_held` the full contract** (addresses: cold-path red cause; three helpers)
   An absent file reads as not held, `EWOULDBLOCK` as held, success as free, and any other error fails the test. Write it once and reuse it.
5. **Correct the red-first claims** (addresses: unleasable test; aged-`Unknown` row; seam tests; fake launcher arguments)
   The unleasable test prints one leasable and one unleasable tree and asserts the leasable one is held. The fake launcher records its arguments. The Phase 5 criterion names only the genuinely red rows. Split Phase 4 item 2 so the field is threaded first and consulted afterwards. Add the intermediate "still exists" assertion to the reap-once-aged test.
6. **Close the warm fixture's descriptor precondition** (addresses: inherited fd 3)
   Close descriptors 3–9 in a loop, or assert that fd 3 is free, before taking the leases.
7. **Fix the `TreeLeases` shape** (addresses: `dead_code`; silent skip; `RefCell` tuple; lease-then-recheck)
   Use a field form that compiles under `warnings = "deny"`. Report skipped leases as `warning:` lines. Return a named type, or name the binding for its role. Re-check `tree.path` after leasing, or cite the accepted gap, and correct the work item's claim-refresh reasoning.
8. **Name the three liveness rules together in `reap.rs`** (addresses: three variants of the policy)
   Write one pure predicate per reclaimer, taking `Liveness` plus an already-computed age flag. Keep `older_than_backstop` private.
9. **Drop the ADR-0069 edit** (addresses: accepted-ADR immutability)
10. **Tidy dependencies and naming** (addresses: double declaration; `libc` in the launcher; fixture name; "port" wording; probe grep)
    Declare `tree-lease` only once in design-cli. Use `rustix::process` in the launcher. Rename the fixture to follow `<crate>-fixture` and cite `tasks/build.py`. Call `LivenessProbe` a seam rather than a port, or move it to core. Restrict the `rg` check to production call sites.
11. **Address sidecar residue under deferred `Unknown`** (addresses: orphaned sidecars)
    Make `reap_orphans` remove `sidecars_of` and assert it does, or record the residue as an accepted limitation.
12. **Smaller test hardening** (addresses: backstop boundary; `minisign` skip; cross-launcher pinning; musl skip)
    Add rows at `mtime + BACKSTOP - 1` and `mtime + BACKSTOP`. Run the success criteria via `mise exec --`. Pin `second` to `&FlockProbe` and assert the holder holds before pruning. Add a platform skip guard to `executor_runtime.rs`.
13. **Add the cross-crate floor assert** (addresses: implicit `IDENTITY_FD` coupling)
    Put `const _: () = assert!(IDENTITY_FD < tree_lease::LOWEST_LEASE_DESCRIPTOR);` in `design-adapters`.

## Per-Lens Results

### Architecture

**Summary**: The plan is structurally sound. It extracts a leaf library following the `process-probe` precedent and places the liveness seam in the outbound adapter, leaving `launch::core` untouched. Its weaknesses are protocol and policy ones: the cold path skips lease-then-recheck, the descriptor floor is coupled to the spawner only implicitly, and the reclaim policy is encoded three ways.

**Strengths**: Dependency direction is correct, with `LeaseError` mapped into `TreeError`. The fd 3 fix lives in one place and the scope deviation is recorded. The seam is wired at the composition root, and `pup:check` guards core. `TreeLeases` stays in adapters. Unleasable trees degrade gracefully. The two strands are independent.

**Findings**:
- 🔵 minor/high: The cold-path lease bypasses lease-then-recheck (Phase 3 §2). `TreeLeases::hold` lacks `acquire`'s identity re-check. A racing prune leaves an unreclaimable `.lease` and a daemon running on a missing tree. Re-check `tree.path`, or open without `create`, and cite the accepted gap.
- 🔵 minor/high: The descriptor floor is coupled to `IDENTITY_FD` only implicitly (Phase 2 §4). Add a `const` assert in `design-adapters`, and keep the `tree-lease` doc generic.
- 🔵 minor/medium: The reclaim policy is encoded three ways with I/O inside the predicate (Phase 5 §2). Put named pure predicates side by side in `reap.rs`.
- 🔵 suggestion/medium: The lease holder and the lease observer live in different crates (Phase 1 §1; Phase 2 §2). Consider exporting an observation helper from `tree-lease`.
- 🔵 suggestion/low: Lease ownership rides on the `Ensured` `RefCell` (Phase 3 §3). Use a named `PinnedResolution`.
- 🔵 suggestion/low: The fixture adds a production dependency edge (Phase 2 §1). Note it, or route through `design_adapters`.

### Code Quality

**Summary**: The plan is well decomposed: a small crate, a seam that mirrors `Clock`, and red tests first. The risks are a policy scattered across modules under an inaccurate name, and `TreeLeases` code that will not compile under `warnings = "deny"` and drops failures silently.

**Strengths**: The `LeaseError` decoupling. A seam backed by an `rg` guard. The floor fixed in one place, with a comment that explains why. A stale comment removed. `Drop` guards on daemons.

**Findings**:
- 🟡 major/high: `TreeLeases.held` is never read, which fails the build under `warnings = "deny"` (Phase 3 §2). Use a tuple struct with an allow, following `RunLockGuard` (`cli/migrate/src/ports.rs:346`), or a `_pinned` field.
- 🟡 major/medium: Three variants of the liveness policy across two modules under a misleading name (Phase 5 §2). Name one rule per reclaimer in `reap.rs`, and keep `older_than_backstop` private.
- 🔵 minor/medium: A skipped cold-path lease leaves no trace (Phase 3 §2). Have the executor print `warning:` lines.
- 🔵 minor/medium: The `RefCell` side channel and anonymous tuple (Phase 3 §3).
- 🔵 minor/medium: `pub fn lease_failure` and repeated `map_err` instead of `From`, and `LeaseError` drops its source (Phase 1 §3).
- 🔵 minor/low: design-cli gains a production dependency on `tree-lease` used only by the fixture (Phase 2 §1).
- 🔵 suggestion/medium: The Phase 4 seam tests duplicate Phase 5 rows, and `aged()` should use `BACKSTOP_SECONDS`.

### Test Coverage

**Summary**: The plan is strongly test-led, with an end-to-end harness that has the right fidelity and a complete `Unknown` table. Several red-first claims are false or fail for the wrong reason, and parallel-test leaks, `killpg` races and `minisign` skips threaten reliability.

**Strengths**: A faithful warm red test through a fresh-process fixture. A real executor, spawner and client. Full table coverage that catches the obvious mutations. A seam that reaches all paths through public entry points. The existing backstop unit test guards `reap_orphans`. Explicit flake discipline.

**Findings**:
- 🟡 major/high: `an_unleasable_tree_still_starts_the_daemon` passes before the change and asserts nothing about leases (Phase 3 §1).
- 🟡 major/medium: The cold-path red fails on a missing lease file, and `observed_held` returns false on any error (Phase 2 §2).
- 🟡 major/medium: Inheritable leases leak into concurrent tests' children (Phase 6 §2).
- 🟡 major/medium: Post-`killpg` "free" assertions race the exit (Phase 2 §3 / Phase 3 §1).
- 🔵 minor/high: The seam tests cannot be behaviourally red, and the `materialise_with_waiter` call site is untested (Phase 4 §3).
- 🔵 minor/high: The aged-`Unknown` row already passes, and the reap-once-aged test is vacuous without an intermediate assertion (Phase 5 §1).
- 🔵 minor/medium: The backstop boundary is untested, and `fresh()` relies on underflow (Phase 5 §1).
- 🔵 minor/medium: `minisign_or_skip!` can silently pass red runs (Phases 4–6). Use `mise exec --`.
- 🔵 minor/medium: The cross-launcher tests are characterisation tests. Pin `FlockProbe` and assert the holder's inheritance (Phase 6).
- 🔵 suggestion/low: The floor unit test may pass before the change by chance (Phase 2 §4).
- 🔵 suggestion/medium: Three hand-written observed-held helpers (Phases 1–3).
- 🔵 suggestion/low: `executor_runtime` departs from the `executor_preflight` lane convention without documenting it (Phase 2 §2).

### Correctness

**Summary**: The logic holds, and the line citations are accurate. One API does not exist (`fcntl_dupfd`). The remaining risks concern lease lifetimes and the tests.

**Strengths**: A correct fd 3 diagnosis. The predicate matches the table. The `flock` semantics are correct on local filesystems. `_tree_leases` carries the leases across `exec`. Accurate citations. Lease failures are skipped rather than escalated.

**Findings**:
- 🟡 major/high: `rustix::io::fcntl_dupfd` does not exist on Unix in rustix 1.1.4 (Phase 2 §4). Use `fcntl_dupfd_cloexec` with `clear_cloexec`.
- 🟡 major/medium: Inheritable test leases leak into other tests' holders (Phase 6 §2).
- 🔵 minor/medium: `kill(pid, 0)` cannot see a zombie exit (Phase 2 §2; Phase 6 §2).
- 🔵 minor/medium: Lock, then dup, then close loses the lock under POSIX-emulated `flock` (Phase 2 §4). Reorder.
- 🔵 minor/medium: The cold-path lease is taken without re-checking, and the hit path does not refresh the claim (`resolver.rs:584-590`) (Phase 3 §2).
- 🔵 minor/high: Deferring `Unknown` superseded generations to `reap_orphans` orphans their sidecars (Phase 5 §2).
- 🔵 minor/high: The `probe_liveness(` grep criterion cannot pass (Phase 4).
- 🔵 minor/high: The unleasable test passes before the change, and the fake launcher records no arguments (Phase 3 §1).

### Safety

**Summary**: The plan closes the three data-loss routes, and the new rules fail safe. The main risks are in the harness: an unvalidated `killpg` target and lease leaks across tests.

**Strengths**: `== Free` in `reap_superseded`, and an unreadable mtime keeps the tree. The floor inside `tree-lease`, plus the re-plan gate. Per-test tempdirs. The fixture is excluded from staging. `reap_orphans` keeps its backstop.

**Findings**:
- 🟡 major/high: `killpg` targets an unvalidated pid (Phase 2 §2).
- 🟡 major/medium: In-process inheritable leases leak into concurrent holders (Phase 6 §2).
- 🔵 minor/medium: `kill(pid, 0)` waits never end for zombies, and a panic in `Drop` aborts the binary (Phase 2 §2; Phase 6 §2).
- 🔵 minor/medium: The cold-path lease is taken without lease-then-recheck (Phase 3 §2).
- 🔵 suggestion/medium: Proceeding unleased leaves no trace (Phase 3 §2).
- 🔵 suggestion/low: The manual `prune --older-than 0` step could hit a real shared cache (Manual Testing Steps).

### Portability

**Summary**: The plan targets macOS and glibc Linux with POSIX primitives. One API claim is wrong (`fcntl_dupfd`). Some environment assumptions should be made explicit.

**Strengths**: A floor instead of platform-specific `close_range`/`closefrom`. The non-unix stub now returns an error instead of panicking. Treating `Unknown` as not-free closes a hole on NFS, SMB and FUSE. The stubs follow `spawn_properties.rs` conventions. The fixture is never staged.

**Findings**:
- 🔴 critical/high: `rustix::io::fcntl_dupfd` does not exist on macOS or Linux in rustix 1.1.4 (Phase 2 §4).
- 🟡 major/medium: The warm red run assumes the runner leaks no inheritable descriptors (Phase 2 §1, §3).
- 🔵 minor/medium: Lock before dup-and-close depends on `flock` owning the lock per open file description (Phase 2 §4).
- 🔵 minor/medium: `kill(pid, 0)` waits depend on init reaping orphans (Phase 2 §2).
- 🔵 minor/high: The executor runtime tests fail rather than skip on musl and NixOS (Phases 2–3).
- 🔵 minor/medium: A crate-level `RawFd` constant breaks the non-unix stub (Phase 2 §4).
- 🔵 suggestion/high: `lsof` is absent on minimal Linux (manual steps).

### Standards

**Summary**: The library-crate checklist is followed completely. The main problem is the proposed edit to an accepted ADR. The rest are smaller convention slips.

**Strengths**: The checklist is covered item by item, including the consent-runner alternation. The manifest mirrors `process-probe`. `LeaseError` implements `Display` and `Error` by hand. The probe-pair shape matches. Test names describe behaviour. A comment is removed in favour of a name. The exemption reason is specific.

**Findings**:
- 🟡 major/high: The plan edits ADR-0069, which is accepted and immutable (Phase 1 §2).
- 🔵 minor/medium: The fixture name and path depart from `<crate>-fixture`, and the cited precedent contradicts the rule (Phase 2 §1).
- 🔵 minor/medium: `libc` in the launcher diverges from its `rustix`-only convention (Phase 6 §2).
- 🔵 minor/medium: A trait called "the port" sits in an outbound module (Phase 4 §1).
- 🔵 minor/medium: `tree-lease` is declared as both a normal and a dev dependency of design-cli (Phase 2 §1–2).
- 🔵 suggestion/medium: The release-staging check is already answerable from `tasks/build.py:39-64` (Phase 2 §1).
- 🔵 suggestion/low: The public-API exemption rests on the weaker precedent (Phase 1 §2).
- 🔵 suggestion/medium: The awkward possessive in a test name (Phase 3 §2).

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-10-12T22:39:29+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🔴 **Portability + Correctness**: `rustix::io::fcntl_dupfd` does not exist on macOS or Linux — Resolved
- 🟡 **Safety + Test Coverage + Correctness**: Inheritable in-process leases leak into concurrent holders — Partially resolved (the long-lived holder is now a fixture; transient `materialise` leases can still reach `minisign` children)
- 🟡 **Code Quality**: `TreeLeases.held` fails `warnings = "deny"` — Resolved
- 🟡 **Safety**: `killpg` targets an unvalidated pid — Resolved
- 🟡 **Test Coverage + Correctness**: The unleasable-tree test passes before the change; the fake launcher records no arguments — Resolved
- 🟡 **Test Coverage**: The cold-path red fails on a missing lease file — Resolved
- 🟡 **Test Coverage**: Post-`killpg` "free" assertions race the exit — Resolved
- 🟡 **Portability**: The warm red run depends on the runner's inherited descriptors — Resolved
- 🟡 **Code Quality**: Three variants of the liveness policy under a misleading name — Resolved
- 🟡 **Standards**: Edit to the accepted ADR-0069 — Resolved
- 🔵 **Portability + Correctness**: Lock-then-dup ordering — Resolved
- 🔵 **Architecture + Safety + Correctness**: The cold-path lease skips lease-then-recheck — Partially resolved (the re-check exists, but a gone tree is still spawned on, and the race is narrowed rather than closed)
- 🔵 **Correctness**: Orphaned sidecars under deferred `Unknown` — Resolved
- 🔵 **Correctness**: The `probe_liveness(` grep cannot pass — Resolved
- 🔵 **Test Coverage**: Seam tests red only by compilation; `materialise_with_waiter` untested — Resolved
- 🔵 **Test Coverage**: The aged-`Unknown` row is not red; reap-once-aged is vacuous — Resolved
- 🔵 **Test Coverage**: The backstop boundary is untested; `fresh()` depends on underflow — Partially resolved (boundary pinned; `fresh()` still sits before the mtime)
- 🔵 **Test Coverage**: `minisign_or_skip!` silent passes — Resolved
- 🔵 **Test Coverage**: Cross-launcher tests must pin `FlockProbe` and assert inheritance — Resolved
- 🔵 **Safety / Portability**: `kill(pid, 0)` zombie waits — Resolved
- 🔵 **Portability**: musl/NixOS hosts fail rather than skip — Resolved
- 🔵 **Portability**: A crate-level `RawFd` constant breaks the non-unix stub — Resolved
- 🔵 **Architecture**: The floor is coupled to `IDENTITY_FD` only implicitly — Resolved
- 🔵 **Code Quality**: The `RefCell` side channel and anonymous tuple — Resolved
- 🔵 **Code Quality**: `lease_failure`, repeated `map_err`, `LeaseError` drops its source — Resolved
- 🔵 **Code Quality / Standards**: Fixture-only dependency, declared twice — Resolved
- 🔵 **Standards**: Fixture name departs from `<crate>-fixture` — Partially resolved (the qualifier precedes `-fixture`, against the plan's own stated rule)
- 🔵 **Standards**: `libc` in the launcher — Resolved
- 🔵 **Standards**: `LivenessProbe` called a port — Resolved
- 🔵 All first-pass suggestions — Resolved

### New Issues Introduced

- 🟡 **Correctness** (also Architecture, Safety): A `TreeGone` tree is skipped but still handed to the daemon. A gone driver surfaces as `ENOENT` → `LoaderUnresolvable`, a sticky host-condition marker. The re-check should change the outcome: re-run `cache ensure` once, otherwise downgrade non-stickily with `ArtifactUnavailable`.
- 🔵 **Safety**: `TreeLeases` deletes a possibly-held lease file when `is_dir()` returns false, which also happens on any stat error. Only `NotFound` should count as gone, and the file should be unlinked only if `observed_held` reads free after this process has released its lease.
- 🔵 **Safety**: The age backstop counts from the generation's extraction mtime. The rename at `resolver.rs:324` does not refresh it, so an `Unknown` superseded generation older than an hour is reaped on the next `reap_orphans` pass, and the "deferral" gives no grace.
- 🔵 **Safety + Correctness**: The plan and the 0334 correction say the re-check "covers" the race. The reclaimers' probe-then-remove window, and a multi-second `remove_dir_all`, remain open, so the wording should say "narrows".
- 🔵 **Portability**: Closing every descriptor listed in `/dev/fd` includes the listing's own directory descriptor. Closing it through `OwnedFd` aborts in debug builds, so the raw `rustix::io::close` should be used, or that descriptor skipped.
- 🔵 **Portability + Correctness**: `Signal::Term` does not exist in rustix 1.1.4. The constant is `Signal::TERM` (verified at `src/signal.rs:87`), and the pid needs `Pid::from_raw`.
- 🔵 **Code Quality**: `Ensured.leases` is an `Option`, but `PinnedResolution.pinned_trees` is not, and `TreeLeases` has no empty constructor.
- 🔵 **Code Quality**: `clear_cloexec` cannot move "unchanged", because `LeaseError` needs a path; the `failure`/`open_lease_file` helpers are unnamed.
- 🔵 **Code Quality + Standards**: `SkipCause` has no `Display`, and the warning line would print the lease path twice.
- 🔵 **Test Coverage**: Phase 5 claims the rule and boundary unit tests fail before item 3, but only the sidecar test can.
- 🔵 **Test Coverage + Correctness**: The "no inheritable lease while spawning" rule does not hold inside `tree_resolution`.
- 🔵 **Standards**: Phase 1's public-API exemption reason cites a constant and an assert that arrive only in Phase 2.
- 🔵 Suggestions:
  - The `runtime_or_skip!` guard should panic under `CI`.
  - No test pins the order of re-check after lease.
  - `observed_held` duplicates `probe_liveness`; build the latter over the former.
  - Lease-then-recheck now exists twice with different rules.
  - The fixtures should define their behaviour when `/dev/fd` is unreadable.
  - `DaemonGroup`'s `Drop` re-signals after `terminate`.
  - Make the drop-before-lock ordering structural.
  - Share one inherited-descriptor closer.
  - Put fixture comments on the `[[bin]]` entries.

### Assessment

The revision resolved every critical finding and 8 of 9 majors. The descriptor floor now compiles and survives POSIX-emulated `flock`, the harness is isolated and safe, and the red-first claims are accurate. One new major remains. The cold-path re-check detects a reclaimed tree but does not act on it, so a reclaim race surfaces as a sticky `LoaderUnresolvable`. The rest are snippet-level compile slips (`Signal::TERM`, `clear_cloexec`'s signature, the `Option` mismatch) and overclaimed wording about the race and the backstop. The plan is acceptable to implement, but a short third edit covering the `TreeGone` outcome, the safe lease-file removal, the backstop wording and the snippet fixes would leave little for implementation to discover.

## Re-Review (Pass 3) — 2026-10-12T22:57:47+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Correctness** (also Architecture, Safety): A `TreeGone` tree is still handed to the daemon — Resolved
- 🔵 **Safety**: `TreeLeases` deletes a possibly-held lease file on any stat error — Resolved
- 🔵 **Safety**: The age backstop counts from extraction for superseded generations — Resolved (`restart_backstop`)
- 🔵 **Safety + Correctness**: The re-check "covers" the race — Resolved (now "narrows"; window accepted)
- 🔵 **Portability**: Closing the `/dev/fd` listing's own descriptor aborts in debug builds — Partially resolved (see the new major below)
- 🔵 **Portability + Correctness**: `Signal::Term` — Resolved
- 🔵 **Code Quality**: `Option` mismatch between `Ensured` and `PinnedResolution` — Resolved
- 🔵 **Code Quality**: `clear_cloexec` signature and unnamed helpers — Resolved
- 🔵 **Code Quality + Standards**: `SkipCause` has no `Display`; the path is printed twice — Resolved
- 🔵 **Test Coverage**: Phase 5 red claims — Resolved
- 🔵 **Test Coverage + Correctness**: Inheritable leases inside `tree_resolution` — Resolved (bounded free-poll; rule narrowed)
- 🔵 **Test Coverage**: `fresh()` before the mtime — Resolved
- 🔵 **Standards**: Fixture qualifier placement — Resolved
- 🔵 **Standards**: Phase 1 exemption reason runs ahead of Phase 2 — Resolved
- 🔵 Suggestions: `runtime_or_skip!` under CI, `Drop` re-signalling, structural drop-before-lock, unreadable `/dev/fd`, `[[bin]]` comments — Resolved
- ⚪ Accepted, not re-raised: `probe_liveness` over `observed_held`; one shared lease-then-recheck; a test that pins re-check ordering; a shared descriptor closer

### New Issues Introduced

- 🟡 **Portability + Correctness**: `rustix::io::close` panics on `EBADF` in Linux debug builds. The `linux_raw` backend's `decode_void` runs `debug_assert_eq!(ignore, 0)` (verified at `src/backend/linux_raw/reg.rs:165-168`). Both fixtures close the listing's already-closed descriptor, so they abort on Linux CI and pass on macOS. `close` is also `unsafe` and requires a valid descriptor. Fix: open `/dev/fd` through `rustix::fs::open` + `Dir` and skip its own number, or close only numbers for which `fcntl_getfd` succeeds. Correct the Key Discovery.
- 🟡 **Code Quality** (also Architecture, Test Coverage, Correctness): the `TreeGone` retry is inlined into an `ensure_runtime` of 69 lines (clippy `too_many_lines` risk), and it leaves three things unspecified:
  - whether a `Failed` second ensure goes through `classify_cause`;
  - which leases are dropped;
  - that every path and lease must come from the second `resolved`.

  Extract `ensure_and_pin` as a loop of at most two attempts, and test the second ensure failing.
- 🔵 **Correctness**: A tree whose lease cannot be opened because `trees/` itself is gone is classified `Unleasable`, which brings back the sticky `LoaderUnresolvable`. Stat the tree on a lease failure too.
- 🔵 **Safety + Correctness**: `reclaim_unclaimed`'s `Unknown` gate still counts from extraction, so a pointed-at generation gets no grace. Record this as a limitation, or measure from the last sign of use.
- 🔵 **Correctness**: `restart_backstop` also runs for `Held` superseded generations, which the end state does not describe.
- 🔵 **Safety**: `TreeLeases` creates and unlinks a lease path read from subprocess stdout without checking that it is the tree's `.lease` sibling.
- 🔵 **Portability**: `restart_backstop` fails silently on exactly the network filesystems that yield `Unknown`. Use `utimensat` by path and report failures, or record the reset as best-effort.
- 🔵 **Safety + Portability**: The mode-`000` test leaves an undeletable temp directory. Restore the mode in a guard.
- 🔵 **Code Quality**:
  - Existing `Ensured { browser_tree }` literal at `executor.rs:643` stops compiling.
  - `unclaimed_is_reclaimable` trips `missing_const_for_fn`.
  - `error.fmt(formatter)` needs `Display` in scope; use `write!`.
  - `# Errors` docs are required by `missing_errors_doc`.
- 🔵 **Test Coverage**:
  - `restart_backstop`'s use of the injected clock is unpinned.
  - No test stops a re-ensure on `Unleasable`.
  - The full-backstop test lacks a "still exists after repair" assertion.
- 🔵 **Architecture**: Stray-lease cleanup lives in `design-adapters`, while the launcher's own `acquire` miss leaks a lone `.lease` that `reap_orphans` never collects.
- 🔵 **Safety**: `DaemonGroup`'s failure-path `Drop` signals without re-checking `getpgid`.
- 🔵 **Standards**: The design-cli harness adds `libc` although `rustix` is now a normal dependency.
- 🔵 Suggestions:
  - `TreeLeases::hold` returns an anonymous tuple.
  - One age concept goes by two names.

### Assessment

Every pass-2 issue is resolved or deliberately accepted. The behavioural design is now sound: a reclaim race is retried and then downgraded without a marker, lease files are unlinked only when nobody holds them, and deferred `Unknown` generations get a real backstop. Two majors remain, and both can be fixed locally. The descriptor closer panics on Linux debug builds, an issue the lens confirmed against the rustix source. The retry also needs to be extracted with its second-failure path specified. The minors are mostly snippet-level lint and compile details plus two honest-limitation statements, covering the unclaimed backstop and the best-effort reset. The plan is acceptable to implement once the two majors are addressed. The minors can be settled during implementation, since `mise run check` will surface the lint ones.

## Re-Review (Pass 4) — 2026-10-12T23:09:17+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Portability + Correctness**: `rustix::io::close` panics on `EBADF` on Linux — Partially resolved (the listing's own number is skipped, but the constructor is unspecified; see below)
- 🟡 **Code Quality** (also Architecture, Test Coverage, Correctness): The `TreeGone` retry is inlined and under-specified — Resolved (`ensure_and_pin`; a failed second ensure is classified like a first; every path comes from one attempt)
- 🔵 **Correctness**: Missing `trees/` classified `Unleasable` — Resolved
- 🔵 **Safety + Correctness**: `reclaim_unclaimed` `Unknown` gate counts from extraction — Resolved (recorded limitation)
- 🔵 **Correctness**: `restart_backstop` runs for `Held` — Resolved (`Unknown` only, plus a regression guard)
- 🔵 **Safety**: Lease path from stdout is not validated — Resolved (sibling check)
- 🔵 **Portability**: Best-effort reset on network filesystems — Resolved (`utimensat` by path; recorded)
- 🔵 **Safety + Portability**: Mode-`000` cleanup — Resolved (restore guard), but see below
- 🔵 **Code Quality**: Literal at `executor.rs:643`, `missing_const_for_fn`, `write!`, `# Errors` — Resolved
- 🔵 **Test Coverage**: Injected-clock pin, re-ensure on `Unleasable`, intermediate assertion — Resolved
- 🔵 **Architecture**: Lone `.lease` from the launcher's `acquire` miss — Resolved (recorded)
- 🔵 **Safety**: `Drop` re-signals without a `getpgid` re-check — Resolved, but see below
- 🔵 **Standards**: `libc` in the design-cli harness — Resolved
- 🔵 Suggestions: anonymous tuple, two names for age — Resolved

### New Issues Introduced

- 🟡 **Portability + Correctness**: `Dir::read_from` opens a second descriptor through `openat(fd, ".")`, which I verified at `rustix-1.1.4/src/backend/linux_raw/fs/dir.rs:77-79`. That descriptor is listed and is not the recorded one, so it is closed twice and the Linux debug abort returns. The procedure must name `Dir::new(owned_fd)`.
- 🟡 **Code Quality + Correctness**: `TreeLeases::hold` cannot build a `tree_lease::LeaseError` for a lease path outside the layout, because the constructor and fields are private. Give `design-adapters` its own `Unleasable { OutsideTreeLayout, Lease(LeaseError) }` with a `Display`.
- 🟡 **Safety + Test Coverage**: `an_unreadable_tree_is_not_listed_gone` is unreachable under the sibling rule. A directory that blocks the tree's stat also blocks its sibling lease, so the tree is `unleasable`, never leased. Restate the test as "listed unleasable, not gone, nothing removed", and mark the "stat fails, lease taken" branch as defence in depth.
- 🔵 **Safety**: `DaemonGroup`'s lease-gated `Drop` never kills a daemon that holds no lease, which is exactly the red-run case. Gate it on `getpgid(pid) == pid` alone, or on a captured `process_probe` start time.
- 🔵 **Code Quality**: `reap_superseded`'s three-arm match leaves `superseded_is_reclaimable` without a production caller, so `dead_code` fires. Return a disposition (`Reclaim | Defer | Keep`) instead. `is_reclaimable_generation`'s `path` parameter is now unused.
- 🔵 **Architecture**: The `.lease` sibling rule is duplicated from `layout.rs` with nothing tying the two together. Add a launcher test that `cache ensure`'s printed lease equals tree + `.lease`, or share a `tree_lease::lease_path_of`.
- 🔵 **Architecture**: The non-sticky `TreesReclaimed` verdict lives in the shell. Consider a domain verdict beside `classify_cause`.
- 🔵 Suggestions:
  - "hold", "pin" and "lease" name one concept.
  - `PinnedResolution.pinned_trees: TreeLeases` versus `PinnedTrees`.
  - Sketch `PinnedTrees` and `EnsureFailure`.

### Assessment

Every pass-3 issue is resolved, partially resolved or recorded as an accepted limitation, and standards found nothing new. Three majors meet the REVISE threshold, but each needs only a small, local edit:
- name `Dir::new`;
- give `design-adapters` its own `Unleasable` reason;
- restate one unreachable test.

No finding questions the design. The remaining minors are naming and wiring details that implementation would settle anyway. One more targeted edit should bring the plan to APPROVE or COMMENT without another full lens pass.

## Re-Review (Pass 5) — 2026-10-12T23:16:07+00:00

**Verdict:** APPROVE

Lenses re-run: correctness, safety, test-coverage, code-quality. These are the lenses that raised the pass-4 majors. Standards was clean in pass 4. The architecture and portability items from pass 4 were each re-assessed by at least one of the lenses above.

### Previously Identified Issues

- 🟡 **Portability + Correctness**: `Dir::read_from` opens a second descriptor — Resolved (`Dir::new` on the recorded descriptor, verified on both backends)
- 🟡 **Code Quality + Correctness**: `LeaseError` cannot be built in `design-adapters` — Resolved (`Unleasable { OutsideTreeLayout, Lease }`)
- 🟡 **Safety + Test Coverage**: The mode-`000` test is unreachable — Resolved (restated as listed unleasable, not gone)
- 🔵 **Safety**: Lease-gated `Drop` skips red-run daemons — Resolved (gated on `getpgid` only)
- 🔵 **Code Quality**: `superseded_is_reclaimable` has no production caller; `path` parameter unused — Resolved (`SupersededDisposition`)
- 🔵 **Architecture**: Sibling rule untied to `layout.rs` — Resolved (`every_sealed_tree_carries_its_lease_beside_it`)
- 🔵 Suggestions: `PinnedTrees`/`EnsureFailure` sketches; one verb per side — Resolved
- ⚪ Deliberately left: the reclaim race's non-sticky verdict stays in the shell rather than the domain

### New Issues Introduced

All three were minor, and all were fixed in the plan after this pass. No lens has re-run since those fixes.

- 🔵 **Test Coverage**: The mode-`000` test's file assertion could not fail — Fixed (removed; the listing assertions remain)
- 🔵 **Test Coverage**: The "never created" half of the outside-layout rule was untested — Fixed (`a_lease_path_outside_the_tree_layout_is_never_created`)
- 🔵 **Correctness**: `tree_leases.rs` called `rustix` without a dependency — Fixed (`rustix` added to design-adapters' `[dev-dependencies]`)

### Assessment

Safety and code-quality returned no findings. Correctness and test-coverage returned only the three minors above, which are now fixed in the plan. Across five passes, every critical and major finding is resolved or recorded as an accepted limitation. The plan is ready for implementation.
