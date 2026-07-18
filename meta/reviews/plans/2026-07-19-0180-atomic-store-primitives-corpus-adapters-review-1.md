---
type: plan-review
id: "2026-07-19-0180-atomic-store-primitives-corpus-adapters-review-1"
title: "Plan Review: Atomic-Store Primitives in corpus-adapters"
date: "2026-07-19T00:58:24+00:00"
author: Toby Clemson
producer: review-plan
status: complete
target: "plan:2026-07-19-0180-atomic-store-primitives-corpus-adapters"
parent: "plan:2026-07-19-0180-atomic-store-primitives-corpus-adapters"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [correctness, architecture, code-quality, test-coverage, safety, compatibility, portability, standards]
review_number: 1
review_pass: 2
tags: [rust, corpus, corpus-adapters, atomic-store, jsonl, store]
last_updated: "2026-07-19T09:16:13+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Atomic-Store Primitives in corpus-adapters

**Verdict:** REVISE

The plan is a genuinely well-researched hexagonal port: it derives `StoreError`'s
placement from the cargo-pup constraint rather than asserting it, keeps the lock
private, centralises the escaper as a single parity seam, translates the bash
EXIT-trap/ceiling/PID-reclaim machinery into RAII guards with injectable ceiling
and liveness predicates, and is test-conscious throughout. However, one **critical
data-loss defect** — `remove_by_key` reads the file *before* acquiring the lock,
silently dropping concurrent appends — was independently flagged by two lenses,
and a cluster of majors covers a live acceptance-criterion contradiction (AC-7
`user_value`), the load-bearing anchored-prefix parity being duplicated and
untested, an internally contradictory `FileStore` definition across phases,
undefined load-bearing helpers, and clippy conventions that would fail the very
`cli:check` gate the plan claims to pass. These are all fixable without
restructuring; the plan should be revised before implementation.

### Cross-Cutting Themes

- **`remove_by_key` defeats its own lock** (flagged by: correctness, safety) —
  the read happens before `lock::acquire`, so a concurrent `append_record` that
  commits between the read and the atomic write is silently overwritten. This is
  the single most serious finding and the one the lock exists to prevent.
- **The AC-7 `user_value` contract is contradictory** (flagged by: compatibility,
  test-coverage, correctness) — the plan and research decided presence-based
  emission, but the work item's AC-7 text still reads "only when
  `outcome=edited`". The golden test would encode a rule that literally
  contradicts the criterion it discharges, and nothing enforces a valid
  outcome/`user_value` pairing.
- **The anchored-prefix parity is load-bearing but fragile** (flagged by:
  code-quality, test-coverage) — the `{"transformation_key":"<esc>",` opener is
  written twice (in `compose_record` and `remove_prefix`) with no shared
  constant, and no test exercises the false-match protection (`"foo"` must not
  remove `"foobar"`). A drift or an over-broad match would delete unintended
  records with a green suite.
- **The plan's code would fail `cli:check`** (flagged by: standards,
  code-quality) — `FileStore::new()` has no `Default` impl
  (`clippy::new_without_default`), and the public fallible free functions omit
  the `/// # Errors` sections the crate enforces (`missing_errors_doc`). Both are
  denied under `warnings = "deny"`, contradicting the Success Criteria.
- **The `FileStore` definition is internally inconsistent** (flagged by:
  code-quality, architecture) — Phase 2 ships a unit struct with `const fn new()`;
  Phase 3 reads `self.lock`, adds a `LockOptions` field and a `with_lock_options`
  constructor never shown, and the phases are described as "independently
  mergeable" despite this being a public-API rewrite of a type shipped one phase
  earlier.
- **The clean-cutover premise is unguarded** (flagged by: safety, compatibility)
  — byte-parity and correctness scope-outs rest entirely on an invariant enforced
  in a *different* work item (0172), with no runtime guard, and `lockdir(path)`
  is never specified to match bash's sidecar so the two implementations may not
  even mutually exclude.

### Tradeoff Analysis

- **Enforce outcome↔user_value coupling here vs defer to 0172**: correctness
  wants the composer to reject invalid combinations (or model them in the type);
  the research (Q2) deliberately keeps the primitive presence-based and leaves
  cross-field policy to the consumer, matching the bash split. Recommendation:
  keep the primitive presence-based (as the research decided) **but** reword AC-7
  so the golden test and the criterion agree, and consider modelling
  `user_value` inside the `Edited` variant only if the team wants the invariant
  unrepresentable rather than consumer-enforced.
- **serde_json escaper vs hand-ported escaper**: compatibility notes serde_json
  permanently forecloses future bash byte-parity (diverges at `0x7F`), while the
  research (Q3) accepts it because AC-6/AC-7 are Rust→Rust. Recommendation: keep
  serde_json (already a dep, decided) but record that a hand-port is the
  fallback if clean-cutover ever fails.

### Findings

#### Critical

- 🔴 **Correctness / Safety**: `remove_by_key` reads the file before acquiring the
  lock (lost-update TOCTOU → silent data loss)
  **Location**: Phase 3 §3 — `RecordStore::remove_by_key`
  In `remove_by_key`, `fs::read_to_string(path)` into `existing` runs *before*
  `lock::acquire`, and the retained lines are computed from that pre-lock snapshot
  and written back under the lock. Any `append_record` that commits between the
  read and the write is silently overwritten. This regresses the bash source
  (which reads inside the locked subshell) and is inconsistent with the plan's own
  `append_record` (which reads after acquiring). Fix: acquire first, then read the
  authoritative content inside the critical section; keep only a cheap unlocked
  existence/empty fast-path.

#### Major

- 🟡 **Compatibility / Test-coverage**: Live AC-7 contradiction — work item still
  mandates outcome-gated `user_value`, plan emits presence-based
  **Location**: Phase 3 composer tests (AC-7 golden record) vs work item AC-7
  The research Q2 follow-up (2026-07-19) concluded AC-7 must be reworded to "only
  when a `user_value` is supplied", but the work item text was never updated. The
  golden test would encode a rule that contradicts the criterion it claims to
  discharge. Reword work-item AC-7 before implementing the golden test.

- 🟡 **Correctness**: Composer permits invalid `outcome`/`user_value`
  combinations
  **Location**: Phase 1 (record types) / Phase 3 (`compose_record`)
  `Outcome` and `Option<String> user_value` are independent, so
  `Accepted + Some(user_value)` and `Edited + None` are both constructible and
  emit contract-violating lines. If the coupling is genuinely deferred to 0172
  (per research Q2), state that explicitly; otherwise enforce it in
  `compose_record` or model `user_value` inside the `Edited` variant.

- 🟡 **Code-quality / Test-coverage**: Anchored-prefix opener is duplicated and
  its false-match protection is untested
  **Location**: Phase 3 §2 — `compose_record` / `remove_prefix`; tests
  The `{"transformation_key":"<esc>",` opener is emitted independently in two
  places (the work item's "single most important parity requirement"), with no
  shared constant and no test that `remove_by_key("foo")` spares a `"foobar"`
  record. Extract a shared opener helper and add a false-match test.

- 🟡 **Code-quality / Architecture**: `FileStore` definition is contradictory
  across phases and rewrites a shipped public API
  **Location**: Phase 2 §1 vs Phase 3 §3
  Phase 2 ships `pub struct FileStore;` + `const fn new()`; Phase 3 reads
  `self.lock`, adds a `LockOptions` field and an unshown `with_lock_options`, and
  `const fn new()` cannot populate a field. Define `FileStore { lock: LockOptions }`
  with both constructors once (Phase 2), so no merged public surface is rewritten.

- 🟡 **Code-quality**: Load-bearing helpers `io()`, `show()`, and `lockdir()` are
  referenced but never defined
  **Location**: Phase 2 §1 (`stage`/`persist`) and Phase 3 §3
  `lockdir()` in particular encodes a design decision (how the lock path derives
  from the target, and how it avoids colliding with a real corpus file). Define
  all three, and call out the `lockdir` naming scheme — it also governs whether
  this port mutually excludes with the bash lock (see clean-cutover finding).

- 🟡 **Test-coverage**: The lock's core concurrency guarantee has no automated
  regression test
  **Location**: Phase 3 Manual Verification (concurrent appenders)
  Serialisation of concurrent `append_record` is only hand-verified; the
  automated lock tests all use an injected `is_alive` and never race two real
  writers on the read-modify-write. Add a `std::thread::scope` test spawning N
  appenders (with a small injected ceiling) that asserts exactly N well-formed
  lines — deterministic despite non-deterministic thread timing.

- 🟡 **Test-coverage**: The deliberate "unparseable owner → treat as live" branch
  is untested
  **Location**: Phase 3 §1 — `reclaim_if_stale`
  The plan calls this a hardening divergence from bash (AC-3), but the AC-4 test
  covers only missing/empty. A mutation reclaiming on a corrupt owner file would
  break a live lock uncaught. Add a garbage-owner case asserting the lock is
  treated as live.

- 🟡 **Safety**: Blind `remove_dir_all` reclaim can delete a freshly re-acquired
  lock, breaking mutual exclusion
  **Location**: Phase 3 §1 — `reclaim_if_stale` / `LockGuard::Drop`
  Read-decide-remove is not atomic with the subsequent `create_dir`: two waiters
  observing the same dead owner can have one reclaim+acquire and the other then
  `remove_dir_all` the live lockdir, leaving both believing they hold the lock —
  exactly under the OOM-under-CI scenario reclaim exists for. Make reclaim
  single-winner (rename-then-remove, or re-verify the dead PID immediately before
  removal), and add a two-waiter reclaim stress test.

- 🟡 **Safety / Code-quality**: Ignored owner-write failure leaves a permanently
  un-reclaimable lock with no diagnostic
  **Location**: Phase 3 §1 — owner write after `create_dir`
  `let _ = fs::write(lockdir.join("owner"), owner)` discards errors; a
  missing owner is treated as live forever (AC-4), so a failed owner write (or a
  SIGKILL in the `create_dir`→owner-write window) yields a lockdir that spins to
  the ceiling indefinitely. Propagate/log the failure and release the lockdir if
  the owner cannot be recorded; document the manual `.lockdir` recovery step.

- 🟡 **Safety / Compatibility**: Clean-cutover premise is unguarded and
  `lockdir(path)` is unspecified — a violation fails silently
  **Location**: Migration Notes / Phase 3 (lockdir naming)
  Byte-parity and correctness scope-outs rest on "no file written by both
  implementations", enforced only in 0172 with no runtime guard. If `lockdir`
  differs from bash's `${target}.lockdir`, the two implementations won't even
  mutually exclude. Make `lockdir` byte-identical to bash's as cheap
  defense-in-depth, add a sequencing constraint that 0172's cutover precedes
  wiring `RecordStore` to any production path, and state that `remove_by_key`
  must only run against Rust-written files until then.

- 🟡 **Portability**: All verification is darwin-only while the release target is
  linux-musl
  **Location**: Desired End State + Phase 2/3 Automated Verification
  `libc::kill` errno handling, `NamedTempFile` (rustix on musl), and the
  `EXDEV` branch are never exercised at runtime on the deployment platform — only
  the dependency graph is musl-checked. Add a success criterion that the suite
  runs green on the linux CI runner, and note musl runtime behaviour is
  unverified until the cross-compile build lands.

- 🟡 **Portability**: Integration tests over shared `std::env::temp_dir()` need
  per-test isolation
  **Location**: Testing Strategy — Integration Tests (`tests/store.rs`)
  `temp_dir()` is a shared world-writable `/tmp` on many linux runners and cargo
  runs tests in parallel; two tests resolving to the same path + sidecar lockdir
  would contend the real lock and flake with `LockTimeout` (only on linux CI,
  which the plan doesn't verify). Give each test a unique dir via
  `tempfile::tempdir()` (already a dep) or PID + `AtomicU64`.

- 🟡 **Standards / Code-quality**: `FileStore::new()` without a `Default` impl
  trips `clippy::new_without_default`
  **Location**: Phase 2 §1 — `FileStore`
  Warn-by-default under `warnings = "deny"`, so `cli:check` (asserted green in
  Success Criteria) would fail. Add `Default` alongside `new()`, and reconcile
  the `const fn` claim once the `LockOptions` field lands.

- 🟡 **Standards**: Public fallible free functions omit the required
  `/// # Errors` doc sections
  **Location**: Phase 2/3 — `atomic_write`, `compose_record`, `remove_prefix`
  `missing_errors_doc` (pedantic = warn) under `warnings = "deny"` denies the
  crate; every existing public fallible fn in the crate documents it. Add
  `/// # Errors` to each, or make `remove_prefix` non-`pub` if it is internal.

#### Minor

- 🔵 **Correctness**: `compose_record` drops the non-empty validation bash applies
  to `transformation_key` and `timestamp`
  **Location**: Phase 3 §2 — field validation
  An empty `transformation_key` yields an anchor prefix removable only by
  `remove_by_key("")` (which would then clobber every empty-key record). Reject
  empty `transformation_key`/`timestamp` to match the ported spec.

- 🔵 **Code-quality**: Non-`AlreadyExists` `create_dir` errors collapse to
  `NotWritable`, dropping the cause
  **Location**: Phase 3 §1 — `acquire_with`
  `ENOSPC`/`ENOTDIR` would surface as "not writable" with no detail. Route
  through `io(...)` or add a `detail` field.

- 🔵 **Code-quality**: Magic number `200` in the back-off guard diverges from the
  injectable `cap_ms` (default 256)
  **Location**: Phase 3 §1 — doubling guard
  Drive the guard off `opts.cap_ms` so there is one source of truth.

- 🔵 **Code-quality**: The `unsafe { libc::kill }` block lacks a `// SAFETY:`
  justification
  **Location**: Phase 3 §1 — `process_is_alive`
  Add the standard FFI SAFETY note (scalar args, no pointers).

- 🔵 **Code-quality**: Hand-rolled byte-by-byte JSON assembly ("quote soup") is
  hard to verify
  **Location**: Phase 3 §2 — `compose_record`
  Consider a small ordered writer over `(name, value)` pairs, keeping
  `schema_version` (bare number) and presence-based `user_value` as explicit
  cases.

- 🔵 **Test-coverage**: The interruption-seam test is near-tautological (guards
  the helper, not the public path)
  **Location**: Phase 2 — interruption before rename
  Seed a prior destination with known bytes and assert they are unchanged after
  dropping the staged temp; add an explicit single-rename replace test on the
  public `write`.

- 🔵 **Test-coverage / Portability**: `EXDEV → CrossFilesystem` mapping has zero
  automated coverage
  **Location**: Phase 2 Manual Verification
  A real cross-mount isn't portable, but the errno classification could be unit-
  covered via a tiny injectable seam mapping a raw errno to the variant. At
  minimum ensure the Phase 1 `Display`/kernel-mapping tests cover all five
  variants including `CrossFilesystem`.

- 🔵 **Test-coverage**: Appended on-disk bytes (LF vs CRLF, separator,
  non-terminated file) are only checked indirectly
  **Location**: Phase 3 — `append_record`
  The `.lines()`-based round-trip tolerates `\r\n`, a missing final newline, or a
  concatenated line. Add a direct byte assertion (`compose_record + "\n"`, LF
  only) and an append-to-unterminated-file case. (This repo has a recent
  mixed-line-ending sensitivity.)

- 🔵 **Safety**: No fsync trades durability for atomicity
  **Location**: What We're NOT Doing — no fsync
  Atomicity is preserved but a crash after rename can lose just-written data (and,
  without a parent-dir fsync, the renamed file). Defensible here; state the
  distinction explicitly as a conscious decision.

- 🔵 **Safety**: Stray temp files accumulate on SIGKILL with no cleanup sweep
  **Location**: Phase 2 §1
  `NamedTempFile`'s Drop doesn't run on SIGKILL. Low impact (atomicity preserved);
  document that orphan temps in the target dir are expected after a hard kill.

- 🔵 **Portability**: Direct `libc::kill` + mkdir-mutex make the module POSIX-only
  — flag the coupling
  **Location**: Phase 3 Overview
  Appropriate for the darwin+musl target set, but record it as a deliberate
  constraint. Optionally consider `std::io::ErrorKind::CrossesDevices` for the
  EXDEV comparison to reduce direct-libc surface.

- 🔵 **Architecture**: Domain invariant (`proposed_value` required) is enforced in
  the shell, not the domain type
  **Location**: Phase 1 (`Record`) vs Phase 3 (`compose_record`, AC-8)
  Consider a validating constructor/newtype on `Record` in `corpus` so field-level
  validity is unrepresentable-when-illegal.

- 🔵 **Architecture**: Dual public surface (free functions plus ports) lets
  consumers depend on concretions
  **Location**: Phase 2/3 — lib.rs re-exports
  Make the free `atomic_write`/`compose_record` `pub(crate)` and route cross-crate
  use through the ports, or document them as the canonical low-level API.

- 🔵 **Standards**: `FileStore` diverges from the `FileConfigStore` naming
  precedent
  **Location**: Phase 2 §1
  Rename to `FileCorpusStore` for symmetry, or justify the shorter name in the
  plan.

#### Suggestions

- 🔵 **Architecture / Compatibility**: The `AtomicWrite`/`RecordStore` ports and
  the `From<StoreError>` kernel mapping are declared with no current Rust consumer
  **Location**: Phase 1 — domain contracts
  Consider deferring the `RecordStore` trait and the kernel mapping to 0172 where
  the single consumer can shape them; sanity-check the trait shapes against 0172's
  concrete session-log access pattern before treating them as frozen. Keep
  `AtomicWrite` (broadly reused, low risk).

- 🔵 **Architecture**: Whole-file read-modify-write under lock is an
  unacknowledged O(n²) scalability tradeoff
  **Location**: Phase 3 / Performance Considerations
  Note the atomicity-for-O(n)-append tradeoff and the bounded-log assumption.

- 🔵 **Test-coverage**: The jitter bound (`1..=base`) is a deterministic property
  but untested
  **Location**: Phase 3 — `jitter_ms`
  Add a cheap loop asserting `jitter_ms(4) ∈ 1..=4` (no wall-clock, no flake).

- 🔵 **Compatibility**: serde_json escaper permanently forecloses future bash
  byte-parity
  **Location**: Key Discoveries / Phase 3 escaper
  Accepted as decided; document that a hand-ported escaper is the migration path
  if clean-cutover ever fails.

- 🔵 **Code-quality / Standards**: Inline comments in the lock code sit in tension
  with the low-comment convention
  **Location**: Phase 3 §1
  Keep the invariant-explaining comments (EPERM/unparseable as live) as
  doc-comments; drop the ones that merely paraphrase the adjacent branch.

- 🔵 **Standards**: Grouped `use std::fmt::{Display, Formatter}` differs from the
  `ConfigError` precedent's per-item imports
  **Location**: Phase 1 §1
  Cosmetic; match the mirrored module or leave deliberately.

### Strengths

- ✅ `StoreError`'s placement in the `corpus` domain is *derived* from the pup
  import rule rather than asserted, and it reproduces `ConfigError`'s hand-written
  taxonomy shape exactly (`#[non_exhaustive]`, matching derives, match-based
  `Display`, bare `impl Error`, `From<_> for kernel::Error::Failed`).
- ✅ The mkdir-lock back-off shape is preserved faithfully (base 4 ms doubling
  through to a 256 ms cap; the `<200` guard plus `.min(cap_ms)` reproduce the bash
  sequence; the ceiling check precedes the sleep on accumulated `waited_ms`).
- ✅ Reclaim is correctly hardened beyond bash — missing/unreadable, empty, and
  unparseable owner, plus `EPERM` from `kill(pid,0)`, are all treated as live,
  preserving the "never break a live lock" invariant.
- ✅ A single `escape_value` drives both compose and remove, and the anchored
  prefix ends in `",` so a shorter key cannot false-match a longer one — the
  load-bearing parity requirement is structurally met.
- ✅ The RAII `LockGuard` cleanly replaces the bash subshell EXIT trap
  (acquire-then-guard preserves "never release a lock you didn't hold"), and
  `NamedTempFile::new_in(parent)` preserves the same-directory atomic-rename
  invariant with `EXDEV` fail-closed.
- ✅ Strong testability seams: the injected `is_alive` predicate, the injectable
  ceiling, and the `stage`/`persist` split make the concurrency and interruption
  logic deterministically testable without real PIDs or 300 s waits; wall-clock
  timing is deliberately ungated to avoid flakiness.
- ✅ `proposed_value` absence is made unrepresentable via a required `String`, and
  emptiness is explicitly rejected — both halves of AC-8 covered.
- ✅ Dependency-compatibility reasoning is accurate: deny.toml is a denylist, the
  caret pins satisfy `wildcards = "deny"`, the `rand::rng()`/`random_range` calls
  match the pinned `rand = "0.9"` API, and only `tempfile` grows the graph.
- ✅ Tradeoffs are named explicitly — clean-cutover byte-parity scope-out (with
  the obligation propagated onto 0172), no-fsync, no-per-path-mutex, and the
  dependency-graph impact.

### Recommended Changes

1. **Move the `remove_by_key` read inside the lock** (addresses: the critical
   lost-update TOCTOU). Acquire first, then read the authoritative content in the
   critical section, mirroring `append_record`; keep only a cheap unlocked
   existence/empty fast-path. Add a concurrent append+remove regression test.

2. **Reconcile the AC-7 `user_value` contract** (addresses: live AC-7
   contradiction; invalid outcome/`user_value` combos). Reword work-item AC-7 to
   "only when a `user_value` is supplied" per research Q2, and add a golden case
   with `outcome=accepted` + a supplied `user_value` to pin the decoupling.
   Decide explicitly whether the outcome↔`user_value` coupling stays deferred to
   0172 (state it) or is enforced/modelled here.

3. **De-duplicate and test the anchored prefix** (addresses: duplicated opener;
   untested false-match). Extract a shared `{"transformation_key":"<esc>",`
   opener used by both `compose_record` and `remove_prefix`, and add a test that
   `remove_by_key("foo")` leaves a `"foobar"` record intact.

4. **Fix the `FileStore` definition and undefined helpers** (addresses:
   cross-phase inconsistency; missing `io`/`show`/`lockdir`). Define
   `FileStore { lock: LockOptions }` with `new()` + `with_lock_options` once in
   Phase 2; define `io`, `show`, and `lockdir`, and specify the `lockdir` naming
   scheme (ideally byte-identical to bash's `${target}.lockdir`).

5. **Harden the lock's failure paths** (addresses: reclaim race; ignored
   owner-write; unparseable-owner untested). Make reclaim single-winner
   (rename-then-remove); propagate/log the owner-write failure and release the
   lockdir if it can't be recorded; add the garbage-owner "treat as live" test.

6. **Close the clippy-gate gaps** (addresses: `new_without_default`; missing
   `# Errors`). Add `Default` for `FileStore` and `/// # Errors` to every public
   fallible free function, so `cli:check` genuinely passes.

7. **Strengthen test coverage and portability** (addresses: no automated
   concurrency test; darwin-only verification; shared-temp isolation; indirect
   byte checks; jitter bound; EXDEV variant). Add the multi-thread appender test,
   a linux-CI success criterion, per-test unique temp dirs, a direct
   appended-bytes (LF) assertion, and a `jitter_ms` bound test.

8. **Record the deferred/POSIX-only decisions** (addresses: clean-cutover guard;
   fsync durability; POSIX coupling; O(n²) append; serde_json foreclosure).
   Add a sequencing note that 0172's cutover precedes production wiring, and
   short notes on the durability-vs-atomicity tradeoff, the POSIX-only coupling,
   the whole-file append cost, and the hand-ported-escaper fallback.

---
*Review generated by /review-plan*

## Per-Lens Results

### Correctness

**Summary**: The port is largely faithful — the jittered back-off shape, the
ceiling-check ordering, the reclaim-if-stale liveness windows, the single-escaper
parity, and the anchored prefix are all correctly reasoned, and RAII replaces the
EXIT trap cleanly. The one serious defect is that `remove_by_key` reads the file
before acquiring the lock and rewrites that stale snapshot — a lost-update TOCTOU.
Two smaller divergences (presence-based `user_value` vs the `outcome=edited`
coupling, and dropped non-empty validation for `transformation_key`/`timestamp`)
let the composer emit contract-violating records.

**Strengths**:
- Back-off shape preserved exactly (4 ms doubling through 256 ms cap; ceiling
  before sleep on accumulated `waited_ms`).
- Reclaim liveness windows correctly ported and hardened (missing/empty/
  unparseable owner, EPERM all treated as live).
- Escape parity structurally guaranteed via the single `escape_value`; anchored
  prefix ends in `",` so no shorter-key false-match.
- `proposed_value` absence unrepresentable via required `String`, emptiness
  rejected — AC-8 met.
- `append_record` does its read-modify-write entirely inside the lock;
  `NamedTempFile::new_in(parent)` preserves same-dir atomicity.

**Findings**:
- 🔴 (high) **`remove_by_key` reads content before acquiring the lock** — Phase 3
  §3. The read into `existing` precedes `lock::acquire`; a concurrent
  `append_record` committing between read and write is discarded. Bash reads
  inside the lock; `append_record` here reads after acquiring — this is a
  remove-path regression that the planned tests (single-threaded remove +
  concurrent *appenders* only) would never catch. Acquire first, then read inside
  the critical section.
- 🟡 (medium) **Presence-based `user_value` lets the composer violate the
  outcome=edited invariant** — Phase 3 §2 / Phase 1. `Accepted + Some(user_value)`
  and `Edited + None` are both constructible. Enforce the coupling in
  `compose_record`, or model `user_value` inside the `Edited` variant, and update
  the golden test.
- 🔵 (medium) **Composer drops non-empty validation for `transformation_key`/
  `timestamp`** — Phase 3 §2. Bash rejects empty required fields; the Rust port
  checks only `proposed_value`. An empty `transformation_key` produces an anchor
  removable only by `remove_by_key("")`. Reject empty `transformation_key`/
  `timestamp`.

### Architecture

**Summary**: A well-reasoned hexagonal port — `StoreError` placement derived from
the pup constraint, the lock kept private, a centralised escaper seam, and RAII
guards with injectable ceiling/liveness. The main tensions are forward-looking:
two ports and a kernel mapping declared ahead of any consumer, a dual public
surface, a domain invariant enforced in the shell, and a Phase 3 rewrite of the
Phase 2 `FileStore` API. None blocking; the hexagon boundary and dependency
direction are sound.

**Strengths**:
- `StoreError` placement derived from the corpus pup rule, with the divergence
  from the `PatchError`-in-adapter precedent correctly explained.
- Lock kept private to the adapter with zero public surface, matching its zero
  external callers.
- Single `escape_value` centralises the byte-parity coupling.
- RAII `LockGuard` faithfully replaces the subshell EXIT trap; ceiling and
  back-off preserved; reclaim hardened.
- Functional-core/imperative-shell seam well cut (injectable ceiling, injected
  `is_alive`, `stage`/`persist` split).
- Tradeoffs explicitly acknowledged.

**Findings**:
- 🔵 (medium) **Two ports + kernel mapping declared ahead of any Rust consumer** —
  Phase 1. RecordStore has one anticipated consumer and isn't exercised as an
  internal abstraction (Phase 3 calls the concrete `atomic_write`). Consider
  deferring `RecordStore` + the `From` mapping to 0172.
- 🔵 (medium) **Dual public surface (free functions + ports)** — lib.rs
  re-exports. Consumers can bypass the port and couple to the concretion. Make
  the free functions `pub(crate)` or document them as the canonical low-level API.
- 🔵 (medium) **Domain invariant enforced in the shell, not the type** — Phase 1
  vs 3. `Record` permits an empty `proposed_value`; consider a validating
  constructor/newtype in `corpus`.
- 🔵 (medium) **Phase 3 mutates the public `FileStore` API Phase 2 ships** —
  Phase 2 vs 3. Introduce the `LockOptions` field and both constructors in Phase 2.
- 🔵 (low) **Whole-file read-modify-write under lock is an unacknowledged
  scalability tradeoff** — O(n) append, O(n²) log build. Note it in Performance
  Considerations.

### Code Quality

**Summary**: Well-structured and pattern-faithful with genuinely good testability
seams. The main maintainability risks are an internal `FileStore` inconsistency
across phases, several load-bearing helpers referenced but never defined, and the
hand-rolled byte-by-byte JSON composition whose anchored-prefix literal is
duplicated.

**Strengths**:
- Clean hexagonal separation mirroring `ConfigError`/`FileConfigStore`.
- Strong testability: injected `is_alive`, injectable ceiling, `stage`/`persist`
  fault-injection seam.
- Single shared `escape_value` for compose and remove.
- Well-modelled, `PartialEq`-comparable error variants; clean let-else guards in
  `reclaim_if_stale`.

**Findings**:
- 🔴 (high) **`FileStore` struct definition inconsistent across phases** — Phase 2
  unit struct + `const fn new()` vs Phase 3 `self.lock` + unshown
  `with_lock_options`. Won't compile as written.
- 🟡 (high) **`io()`, `show()`, `lockdir()` referenced but never defined** — the
  `lockdir` derivation is a real design decision (granularity, collision-avoidance).
- 🟡 (medium) **Anchored-prefix literal duplicated between `compose_record` and
  `remove_prefix`** — the flagged "single most important parity requirement" with
  no shared constant.
- 🔵 (medium) **Hand-rolled byte-by-byte JSON assembly is hard to verify** —
  consider a small ordered writer.
- 🔵 (medium) **Non-`AlreadyExists` `create_dir` errors collapse to `NotWritable`,
  dropping the cause** — route through `io()` or add a `detail` field.
- 🔵 (medium) **Owner-sentinel write failure silently swallowed** — a missing
  owner is live forever; propagate or fail acquisition.
- 🔵 (medium) **`FileStore::new()` has no `Default` impl** — `new_without_default`
  denied under `warnings = "deny"`.
- 🔵 (low) **Magic number `200` diverges from injectable `cap_ms`** — drive the
  guard off `opts.cap_ms`.
- 🔵 (low) **`unsafe { libc::kill }` block lacks a `// SAFETY:` comment**.
- 🔵 (low) **Inline comments sit in tension with the low-comment convention** —
  keep invariant-explaining ones as doc comments, drop paraphrasing ones.

### Test Coverage

**Summary**: Unusually test-conscious (failing-tests-first, deterministic lock
tests via injected predicate + ceiling, AC-6 round-trip guarding escaper parity,
AC-7 golden independent of the round-trip). But the load-bearing concurrency
guarantee is only a hand-run stress loop, the anchored-prefix false-match is
untested, and the deliberate unparseable-owner branch is untested — cheap gaps in
the subtlest, safety-critical paths.

**Strengths**:
- Deterministic lock tests via injected `is_alive` + small `ceiling_ms`, timing
  ungated.
- AC-6 adversarial round-trip correctly guards the escaper-parity trap.
- AC-7 golden kept independent of the round-trip.
- Explicit TDD ordering; tests return `Result`, use `?`; temp-dir isolation
  precedent cited.
- AC-8 "absent" half discharged structurally via required `String`.

**Findings**:
- 🔴 (high) **Lock's core concurrency guarantee has no automated regression test**
  — only a manual stress loop; add a `std::thread::scope` N-appender test.
- 🔴 (high) **Anchored-prefix false-match (`foo` vs `foobar`) never tested** — a
  dropped trailing comma or `contains` swap would delete records with a green
  suite.
- 🔴 (medium) **Unparseable-owner "treat as live" branch untested** — add a
  garbage-owner case.
- 🔵 (medium) **Interruption-seam test near-tautological** — seed prior bytes and
  assert unchanged; add a public single-rename replace test.
- 🔵 (high) **EXDEV→CrossFilesystem mapping has zero automated coverage** —
  ensure all five variants' `Display`/kernel mapping are tested; consider an
  injectable errno seam.
- 🔵 (medium) **Appended on-disk bytes (line-ending, separator, non-terminated
  file) only checked indirectly** — add a direct LF byte assertion.
- 🔵 (medium) **AC-7 golden encodes presence-based, contradicting the AC-7 text**
  — reword AC-7; add an `outcome=accepted` + supplied `user_value` case.
- 🔵 (medium) **Jitter bound (`1..=base`) untested** — add a cheap bound-check
  loop.

### Safety

**Summary**: Faithfully ports a subtle concurrency design and gets the core
atomicity invariant right (same-dir temp + single rename, RAII cleanup, EXDEV
fail-closed, all-or-nothing read, PID-reuse hardening). But `remove_by_key` reads
before locking (drops concurrent appends), the blind `remove_dir_all` reclaim can
delete a freshly re-acquired lock, and the byte-parity/correctness scope-out rests
on an external clean-cutover invariant with no runtime guard — all failing
silently (data loss) if violated.

**Strengths**:
- Atomicity preserved (same-dir `NamedTempFile` + single rename; Drop cleans the
  common failure path).
- `append_record` reads inside the lock with all-or-nothing `fs::read` — no
  truncation hazard.
- Reclaim hardened beyond bash (unparseable/EPERM treated as live).
- EXDEV fail-closed to a distinct error.
- Injectable ceiling + liveness for deterministic tests.
- Clean-cutover obligation propagated onto 0172.

**Findings**:
- 🔴 (high) **`remove_by_key` reads before the lock, silently losing concurrent
  appends** — Phase 3 §3.
- 🟡 (medium) **Blind `remove_dir_all` reclaim can delete a freshly re-acquired
  lock** — two waiters can both end up believing they hold it. Make reclaim
  single-winner.
- 🟡 (medium) **Clean-cutover invariant unguarded; `lockdir` naming unspecified**
  — make `lockdir` byte-identical to bash's for cross-impl exclusion; add a
  sequencing constraint.
- 🟡 (medium) **Ignored owner-write failure leaves a permanently un-reclaimable
  lock** — propagate/log and release if the owner can't be recorded.
- 🔵 (medium) **No fsync trades durability for atomicity** — defensible; state the
  distinction.
- 🔵 (low) **Stray temp files accumulate on SIGKILL** — document as expected.

### Compatibility

**Summary**: Ports a load-bearing on-disk contract and handles the
toolchain/dependency surface well (caret pins satisfy `wildcards = "deny"`,
`rand::rng()`/`random_range` match `rand = "0.9"`, `StoreError` sited to respect
pup). The central problem is the live AC-7 contradiction (work item still says
"only when outcome=edited"); the secondary is that the serde_json 0x7F divergence
and byte-parity scope-out rest on a clean-cutover invariant nothing in 0180
enforces.

**Strengths**:
- Accurate dependency-version reasoning (denylist, licenses covered, only
  `tempfile` grows the graph).
- Correct rand 0.9 API usage.
- `StoreError` respects the pup import rule; infra confined to the adapter.
- Canonical field order fully enumerated; single `escape_value` for compose +
  remove.
- POSIX-only libc usage aligns with the darwin+musl targets (no Windows target).

**Findings**:
- 🟡 (high) **Live AC-7 contradiction: work item mandates outcome-gated
  `user_value`, plan emits presence-based** — reword AC-7 per research Q2.
- 🟡 (medium) **Byte-parity scope-out depends on a clean-cutover invariant 0180
  doesn't enforce** — a Rust `remove_by_key` over a bash-written 0x7F key would
  silently fail to match. Add a sequencing constraint and a "Rust-written files
  only" note.
- 🔵 (medium) **serde_json permanently forecloses future bash byte-parity** —
  document the hand-port fallback.
- 🔵 (low) **Ports declared with no Rust consumer yet** — sanity-check against
  0172's access pattern before freezing.

### Portability

**Summary**: Sound foundations (same-dir temp for atomic rename, the proven
`libc::EXDEV` comparison, POSIX-only primitives, deny.toml graph-checked across
both musl triples). The main gap is that all stated verification is darwin-only
while the release target is linux-musl — the EXDEV branch, `libc::kill` errno
handling, and `NamedTempFile` are never exercised at runtime on the deployment
platform — and the integration tests' shared `temp_dir()` needs per-test
isolation.

**Strengths**:
- `NamedTempFile::new_in(dir)` mandated (never default `new()`) — the most
  portability-critical decision, made explicitly.
- `raw_os_error() == Some(libc::EXDEV)` portable by construction; already shipped
  in the visualiser server on the same targets.
- POSIX-only primitives, uniform across OS families.
- New deps graph-checked against both musl triples; `tempfile` musl-clean.
- LF line endings; `lines()` normalisation.

**Findings**:
- 🟡 (medium) **All verification is darwin-only while the target is linux-musl** —
  add a linux-CI success criterion.
- 🟡 (medium) **Integration tests over shared `temp_dir()` need per-test
  isolation** — use `tempfile::tempdir()` or PID + `AtomicU64`.
- 🔵 (high) **Cross-filesystem branch verified only by inspection** — consider an
  injectable errno seam for deterministic coverage.
- 🔵 (high) **Direct `libc::kill` + mkdir-mutex make the module POSIX-only** —
  flag the coupling; optionally use `ErrorKind::CrossesDevices`.

### Standards

**Summary**: Closely mirrors the `cli/` hexagon conventions — `StoreError`
matches `ConfigError`'s taxonomy exactly, corpus imports stay within the pup
allow-list, new deps respect deny.toml, and module wiring follows house style.
Two convention gaps would break the `warnings = "deny"` clippy gate the plan
claims to pass: `FileStore::new()` without `Default`, and public fallible free
functions missing `/// # Errors`.

**Strengths**:
- `StoreError` reproduces the `ConfigError` idiom faithfully.
- Corpus domain imports use single-item `crate::`-qualified paths inside the pup
  allow-list; infra confined to the adapter.
- Dependency additions respect deny.toml (caret pins, allowed licenses).
- Module wiring/re-export styles match each crate.
- Trait methods carry `/// # Errors`; rationale-style comments match convention.

**Findings**:
- 🟡 (high) **`FileStore::new()` without `Default` trips
  `clippy::new_without_default`** — would fail `cli:check`.
- 🟡 (high) **Public fallible free functions omit `/// # Errors`** —
  `missing_errors_doc` denies the crate.
- 🔵 (medium) **`FileStore` diverges from the `FileConfigStore` naming
  precedent** — rename or justify.
- 🔵 (low) **Grouped `use std::fmt::{Display, Formatter}` differs from the
  `ConfigError` per-item imports** — cosmetic; match or leave deliberately.

## Re-Review (Pass 2) — 2026-07-19

**Verdict:** APPROVE

Delta review of the revised plan across all 8 lenses. **Every round-1 finding is
resolved** (the critical data-loss bug and all 14 majors). Pass-2 agents surfaced
one genuine new correctness bug plus several clippy-gate predictions and an
under-characterised residual race; all were addressed in a follow-up edit within
this pass. What remains is minor observations and consciously-accepted tradeoffs.

### Previously Identified Issues

- 🔴 **Correctness / Safety**: `remove_by_key` read before lock (lost update) —
  **Resolved** (reads authoritative content inside the lock; unlocked no-op
  fast-path only).
- 🟡 **Compatibility / Test**: AC-7 `user_value` contract contradiction —
  **Resolved** (work item AC-7 reworded to presence-based; plan and work item
  agree).
- 🟡 **Correctness**: composer permits invalid `outcome`/`user_value` combos —
  **Resolved** (presence-based with the cross-field coupling explicitly deferred
  to 0172).
- 🟡 **Code-quality / Test**: anchored-prefix duplicated + false-match untested —
  **Resolved** (single `record_opener`; `foo`/`foobar` test added).
- 🟡 **Code-quality / Architecture**: `FileStore` inconsistent across phases —
  **Resolved** (`FileCorpusStore` fully and consistently defined; dead_code
  rationale for the Phase-3 field).
- 🟡 **Code-quality**: undefined `io`/`show`/`lockdir` — **Resolved** (all
  defined; `lockdir` byte-identical to bash).
- 🟡 **Test**: lock concurrency had no automated test — **Resolved**
  (`std::thread::scope` guard).
- 🟡 **Test**: unparseable-owner branch untested — **Resolved**.
- 🟡 **Safety**: blind `remove_dir_all` reclaim race — **Partially resolved**
  (single-winner rename-then-remove + per-attempt nonce; residual race now
  documented honestly and accepted under clean-cutover usage).
- 🟡 **Safety / Code-quality**: ignored owner-write failure — **Resolved**
  (releases + propagates `Io`).
- 🟡 **Safety / Compatibility**: clean-cutover unguarded, `lockdir` unspecified —
  **Resolved** (sequencing constraint, byte-identical lockdir, escaper fallback).
- 🟡 **Portability**: darwin-only verification — **Resolved** (linux-CI
  criterion; musl runtime deferral documented).
- 🟡 **Portability**: shared `temp_dir()` isolation — **Resolved**
  (`tempfile::tempdir()` per test).
- 🟡 **Standards / Code-quality**: `new_without_default` — **Resolved** (`Default`
  added).
- 🟡 **Standards**: free fns missing `/// # Errors` — **Resolved** (unexported).

### New Issues Introduced (Pass 2) — and their disposition

- 🔴→✅ **Correctness / Safety**: `append_record` glued a record onto a
  newline-less last line (failed the plan's own separator test) — **Fixed**:
  inserts a separating `\n` when the file is non-empty and unterminated.
- 🟡→✅ **Standards**: `pub(crate)` in a private module trips
  `clippy::redundant_pub_crate` — **Fixed**: `compose_record`/`remove_prefix` are
  plain `pub fn` (matching `lock::acquire`).
- 🟡→✅ **Code-quality**: `with_lock_options` is const-eligible
  (`missing_const_for_fn`, nursery) — **Fixed**: `pub const fn`.
- 🟡→✅ **Test**: concurrency test's small `ceiling_ms` could flake under CI load
  — **Fixed**: uses the default ceiling and asserts the key set, not just line
  count.
- 🟡→✅ **Correctness / Safety**: residual reclaim race mischaracterised
  ("sub-microsecond", "not data loss") and `discard_path` could wedge future
  reclaims — **Fixed**: honest rewording (ordinary scheduling gap; possible lost
  update; accepted-not-eliminated) + per-attempt nonce.
- 🟡→📝 **Code-quality**: `LockOptions` `_ms` suffix may trip
  `struct_field_names` — **Noted**: resolve with a scoped `#[allow]` if it fires.
- 🟡→📝 **Test**: `PermissionDenied`→`NotWritable` arm not behaviourally tested —
  **Noted**: inspection-only (a real unwritable dir is root-dependent/flaky); the
  Phase-1 five-variant test pins its Display/kernel mapping.
- 🔵→✅ **Correctness / Compatibility**: extras-key validation looser than bash
  `^[a-z][a-z0-9_]*$` — **Fixed**: tightened.
- 🔵→✅ **Test**: single-winner reclaim test assertion ambiguous — **Fixed**
  (bounded ceiling; assert one `Ok` + one `LockTimeout`).
- 🔵→✅ **Test**: `claim` owner-write-failure hardening untested — **Fixed** (test
  added: pre-create a dir at `<lockdir>/owner` → assert `Io` + lockdir removed).
- 🔵→✅ **Test**: escape/golden assertions could be tautological — **Fixed**
  (assert hand-written literal bytes incl. raw `0x7F`, not re-derived).
- 🔵 **Accepted / not changed** (minor observations or conscious tradeoffs):
  `lock.rs` re-hand-rolls `io`/`show` (duplication); pre-lock `create_dir_all` is
  load-bearing for the lockdir parent (reads redundant); `CrossFilesystem` /
  `LockTimeout` are filesystem-flavoured variants in the domain error; field-level
  `Record` validity lives in the composer not the domain type; `AtomicWrite` and
  `RecordStore` share one impl carrying lock-only state; musl **runtime** is not
  exercised until the cross-compile build lands; the empty-parent `"."` fallback
  couples to CWD; the provisional `RecordStore` port lacks a formal freeze
  checkpoint; doc-comments on private helpers slightly exceed the crate's
  near-comment-free norm.

### Assessment

The plan is now in good shape and ready for implementation. The round-1 critical
and every round-1 major are resolved; the one real bug Pass 2 surfaced (append
separator) is fixed and pinned by an existing test, and the predicted
`cli:check`/clippy failures (`redundant_pub_crate`, `missing_const_for_fn`, and
the noted `struct_field_names`) are handled so each phase can land green. The
sole non-eliminated risk — the residual mkdir-reclaim displacement race — is
inherent to the lock strategy (bash has it too), now characterised honestly, and
acceptable under the clean-cutover single-writer-per-file usage; a fully race-free
reclaim would require an atomic compare-and-remove the filesystem does not offer.
The remaining accepted items are minor and can be revisited when 0172's real
consumer lands.

**Approved for implementation (2026-07-19).** No critical or major findings
remain open; the accepted minors are documented tradeoffs, not blockers. Plan
status moved `draft` → `ready`.

---
*Re-review generated by /accelerator:review-plan*
