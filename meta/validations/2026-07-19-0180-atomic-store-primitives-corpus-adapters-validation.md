---
type: plan-validation
id: "2026-07-19-0180-atomic-store-primitives-corpus-adapters-validation"
title: "Validation Report: Atomic-Store Primitives in corpus-adapters"
date: "2026-07-19T17:14:03+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: pass
parent: "plan:2026-07-19-0180-atomic-store-primitives-corpus-adapters"
target: "plan:2026-07-19-0180-atomic-store-primitives-corpus-adapters"
tags: [rust, corpus, corpus-adapters, atomic-store, jsonl, store]
last_updated: "2026-07-19T17:14:03+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Atomic-Store Primitives in corpus-adapters

### Implementation Status

✓ Phase 1: Domain contracts in `corpus` — Fully implemented
✓ Phase 2: `atomic_write` and the `AtomicWrite` port — Fully implemented
✓ Phase 3: The lock, the JSONL composer, and the `RecordStore` port — Fully
implemented

All three phases landed as the sequence of commits the plan called for
(`338dcd37` domain contracts → `accc29a5` atomic write → `76753652`
lock/composer/store → `609bb999` the rustix follow-up), each independently
mergeable.

### Automated Verification Results

✓ Both crates compile and all tests pass: `cargo test -p corpus -p
corpus-adapters` → 130 passed across 8 suites
✓ Full single-component gate: `mise run cli:check` (rustfmt + clippy) exits 0
✓ pup domain-import guard: `mise run pup:check` exits 0
✓ cargo-deny admits the new deps (`tempfile`, `rand`, `rustix`, `libc`,
`serde_json`): `mise run deny:check` → "advisories ok, bans ok, licenses ok,
sources ok"

### Code Review Findings

#### Matches Plan:

- `StoreError` (all five variants, `#[non_exhaustive]`, `PartialEq`/`Eq`, the
  `Display` strings, and `From<StoreError> for kernel::Error`) is exactly the
  Phase 1 spec (`cli/corpus/src/store.rs`). All five `Display` variants are
  exercised, including `CrossFilesystem`, as AC required.
- `Record`/`Outcome` model matches (`cli/corpus/src/record.rs`):
  `proposed_value` required `String`, `user_value` `Option<String>`, `extras`
  as ordered `Vec<(String, String)>`, `Outcome::as_str` tokens.
- `corpus/Cargo.toml` carries `kernel` with the updated comment noting
  `StoreError`'s boundary mapping is its first use.
- `atomic_write` is the same `stage`/`persist`/`classify_persist_error` split,
  `pub(crate)`, `NamedTempFile`-in-parent with RAII cleanup, EXDEV fail-closed
  (`cli/corpus-adapters/src/store.rs`). The interruption-before-rename
  fault-injection tests (both seeded-destination-intact and fresh-path-absent)
  are present.
- The mkdir-lock (`cli/corpus-adapters/src/lock.rs`) implements the injectable
  `LockOptions` ceiling, single-winner rename-then-remove reclaim, the
  owner-write-failure release-not-orphan hardening, jittered back-off, and the
  `dead_owner` "missing/empty/unparseable → live" semantics. The
  `#[allow(clippy::struct_field_names)]` the plan anticipated is applied.
- The JSONL composer (`cli/corpus-adapters/src/jsonl.rs`) renders the exact
  canonical field order, routes both compose and remove through the single
  `record_opener`, validates the three required fields plus reserved/malformed
  extras keys, and emits `user_value` by presence. Golden-record and escape
  tests use hand-written literal expected bytes (including raw `0x7F`), not
  recomputed serde output.
- `RecordStore` (`append_record`/`remove_by_key`) acquires the lock before
  reading authoritative content, normalises the separator, uses the
  `<path>.lockdir` sidecar, and the black-box `tests/store.rs` covers the
  adversarial-key round-trip, the anchored-prefix false-match guard, the
  byte-level append shape, and the `std::thread::scope` concurrency guard that
  asserts the set of keys (not just the line count).

#### Deviations from Plan (all documented, all improvements):

- **Liveness probe uses `rustix` instead of `libc::kill`.** The plan's
  `process_is_alive` wrapped an `unsafe { libc::kill(pid, 0) }` FFI call; the
  implementation (commit `609bb999`) uses `rustix::process::test_kill_process`
  with `Pid`, eliminating the `unsafe` block. It preserves the exact
  semantics the plan specified (signalable or `EPERM` → alive; only `ESRCH`/
  other error → dead) and adds a `pid <= 0 → live` guard as extra hardening.
  `cli/Cargo.toml:46-54` documents the rationale (rustix already in the graph
  via `tempfile`; the `process` feature only adds the module).
- **`rustix` added as an explicit workspace dependency.** The plan's Phase 3
  dependency list named only `rand` + `serde_json`; the rustix swap adds one
  more caret-pinned dep. `cargo-deny` admits it (it was already transitively
  present), so no license/ban/graph regression.
- `libc` is retained (as the plan's Phase 2 specified) solely for the
  `libc::EXDEV` errno classification in `atomic_write`.

#### Potential Issues:

- None blocking. The residual mkdir-lock reclaim race and the
  no-fsync/no-permission-preservation durability trade are both explicitly
  called out in the plan's "Desired End State" and "What We're NOT Doing" and
  remain acceptable under the clean-cutover ruling. The O(n²) append cost for a
  large log is likewise a documented, accepted characteristic for the bounded
  session-log consumer.

### Manual Testing Required:

1. Cross-mount atomicity:
  - [ ] AC-2's genuine cross-mount rename remains inspection-only against the
    `file_driver.rs` precedent (not portable in CI); the errno branch is
    unit-covered via `classify_persist_error`.
2. Sustained contention (optional):
  - [ ] A longer hand-run soak of parallel `append_record` to one path,
    confirming no lost record and no partial line ever observable on disk. The
    automated `std::thread::scope` guard already covers the core case.

### Recommendations:

- No changes required before merge — the implementation is complete, tested,
  and green across the full gate.
- Honour the plan's sequencing constraint: 0172's atomic per-file cutover must
  be in place before `FileCorpusStore`/`RecordStore` is wired to any production
  session-log path (the bash↔Rust `0x7F` escape divergence means a Rust
  `remove_by_key` over a bash-written line could silently fail to match).
- When the `RecordStore` method shapes are first consumed (0170/0172/0168),
  re-confirm them against 0172's concrete access pattern — the plan flagged them
  as provisional until then.
