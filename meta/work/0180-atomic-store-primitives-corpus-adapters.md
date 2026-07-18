---
type: work-item
id: "0180"
title: "Atomic-Store Primitives in corpus-adapters"
date: "2026-07-06T22:27:35+00:00"
author: Toby Clemson
producer: refine-work-item
status: ready
kind: task
priority: high
parent: "work-item:0166"
blocked_by: ["work-item:0179"]
relates_to: ["work-item:0172"]
external_id: PP-704
tags: [rust, config, corpus, store, crates, dedup]
last_updated: "2026-07-18T22:02:04+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0180: Atomic-Store Primitives in corpus-adapters

**Kind**: Task
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

Port the load-bearing atomic-store concurrency primitives — atomic file writes,
the mkdir-based lock with PID-owner reclaim and jittered back-off, and
canonical-order JSONL compose/remove — from the bash library into
`corpus-adapters`.

## Context

Child of 0166 — Shared config, corpus, and store Crates. 0166 folds the
atomic-store capability into `corpus-adapters` rather than a standalone `store`
crate (a later split stays open if a second consumer needs it independently).
These primitives are subtle: the mkdir lock exists because `flock` is not POSIX
and is absent on macOS, and the JSONL remover relies on the writer emitting a
well-defined canonical field order. They must be ported with the concurrency
semantics reasoned through for a Rust context.

## Requirements

- `atomic_write`: same-directory temp file + atomic rename, cleaning up the temp
  on interruption before the rename.
- The mkdir-based lock: `mkdir` as the exclusive-acquisition mutex, PID-owner
  reclaim of a dead holder, jittered exponential back-off, and the 300 s
  acquisition ceiling. Reason through the bash `$BASHPID`-absent fallback for
  Rust (moot there, but PID-reuse safety and the ceiling still apply).
- Canonical-order JSONL compose/remove: `transformation_key` first (so the
  remover's anchored-prefix match is well-defined), `schema_version` second,
  then the remaining fields in canonical order; the writer and remover must
  agree byte-for-byte on the escape rules.
- Record-field validation: port the documented spec, not the bash source's
  looser behaviour — treat `proposed_value` as required and reject a record whose
  `proposed_value` is empty or absent (the bash writer documents it required at
  `jsonl-common.sh:60` but omits it from the emptiness check at `:115-116`).

## Acceptance Criteria

- [ ] **AC-1** _(umbrella, discharged by AC-2…AC-8 below):_ `corpus-adapters`
      provides atomic writes (temp + rename), the mkdir-based lock with PID-owner
      reclaim and jittered back-off, and canonical-order JSONL compose/remove.
- [ ] **AC-2** `atomic_write` creates its temp file in the *target's* directory
      (not `$TMPDIR`) and replaces the destination via a single rename within one
      filesystem; an interruption before the rename leaves either the prior file
      intact or no file — never a partially-written destination. The mid-write
      interruption is provoked deterministically via a fault-injection seam
      between the temp write and the rename. (Temp handling uses
      `tempfile::NamedTempFile::new_in` per Technical Notes; the criterion is met
      by the observable same-directory location + single-rename outcome, not the
      specific API call.)
- [ ] **AC-3** A test covers contended-lock reclaim of a dead holder: the lock is
      reclaimed when the owner PID is gone, and never broken while a live owner
      holds it.
- [ ] **AC-4** A lock directory whose `owner` sentinel is absent or empty (a
      holder caught mid-acquisition, between `mkdir` and the owner write) is
      treated as *live* — not reclaimed — until the acquisition ceiling, matching
      `_atomic_lock_reclaim_if_stale`.
- [ ] **AC-5** Lock acquisition against a live, permanently-held lock returns a
      lock-timeout `StoreError` bounded by the acquisition ceiling — with the
      ceiling injectable so the test does not wait 300 s — and a non-writable
      target directory yields the writability pre-check early error rather than
      entering the back-off loop. (The exact back-off *shape* — 4 ms base doubling
      to a ~200–256 ms cap, jitter uniform `1..=base` — is specified in Technical
      Notes as design intent; wall-clock timing is deliberately not gated by a
      criterion, to avoid flaky timing tests.)
- [ ] **AC-6** A round-trip (compose then remove-by-key) succeeds for adversarial
      `transformation_key` values containing backslashes, double-quotes, and
      control characters — verifying escape parity, not just field order. Per the
      clean-cutover ruling (see Open Questions) this Rust→Rust round-trip is the
      full parity requirement; byte-compatibility with records already written by
      `jsonl-common.sh` is out of scope.
- [ ] **AC-7** A golden-record assertion pins the emitted canonical field order
      independently of the round-trip, matching `jsonl_compose_record`
      (`jsonl-common.sh:129-148`): `transformation_key`, `schema_version`,
      `outcome`, `proposed_value`, then `user_value` (only when
      `outcome=edited`), then `timestamp`, then author-declared extras in
      declaration order. This guards against the self-referential AC-6 round-trip
      passing while the emitted order silently diverges from the on-disk contract
      the visualiser reads.
- [ ] **AC-8** Composing/writing a record whose `proposed_value` is empty *or
      absent* is rejected with a validation error (covering both halves of the
      "required and non-empty" rule), enforcing the documented-required behaviour
      the bash source omits from its emptiness check.

## Open Questions

- **Resolved (2026-07-18): clean cutover on the JSONL escaper.** There is a clean
  cutover point — a given atomic-store JSONL file is written by either the bash
  primitives or the Rust port, not both concurrently — so bash-written and
  Rust-written records need not interoperate byte-for-byte on disk. A
  self-consistent Rust escaper therefore suffices: the Rust escaper need not be
  byte-compatible with records already written by `jsonl-common.sh`, and
  remove-by-key is only ever run against Rust-written records. AC-6's Rust→Rust
  adversarial round-trip (with AC-7 pinning the emitted field order) is the full
  parity requirement; no differential (`bash-parity`-gated) suite against
  bash-written records is in scope. This scope-out rests on a clean cutover the
  migrate consumer (0172) must enforce — see Dependencies.
- **Resolved (2026-07-18): `atomic_write` uses the `tempfile` crate.** Follow the
  in-repo prior art (`server/src/file_driver.rs`) with
  `tempfile::NamedTempFile::new_in(parent)` — accepting a new direct dependency
  (subject to the cargo-deny ban-lists 0162/0166 activate) in exchange for robust
  unique temp naming and RAII cleanup on unwind. The `new_in(parent)` form is
  mandatory: the default `new()` uses `$TMPDIR` and degrades the rename to a
  non-atomic cross-mount copy. This ruling concerns only `atomic_write`'s temp
  handling; the mkdir-lock still uses `std::fs::create_dir` + `libc::kill`, and
  the lock/reclaim logic is novel regardless.

## Dependencies

- Blocked by: 0179 (the `corpus-adapters` crate these primitives live in).
- Parent: 0166.
- Downstream consumers (captured story-level via 0166's `blocks: 0167–0173`
  rather than as direct `blocks` edges here): the canonical field-order JSONL
  contract and the `atomic_write`/lock port feed 0172 (the migrate session log —
  the one production JSONL caller), 0170 (work / work-item-sync), and the
  visualiser/0168 refactor, which reads `jsonl_compose_record`'s canonical field
  order. A divergent escaper would silently break these, so the field-order and
  escape contract is load-bearing beyond this crate.
- Toolchain coupling: landing the new `tempfile` and direct `libc` dependencies
  requires `cli/deny.toml`'s allow-list (activated by 0162/0166) to admit them.
- Clean-cutover constraint on 0172 (`relates_to: 0172`): the clean-cutover ruling
  (see Open Questions) scopes bash↔Rust byte-parity out on the premise that no
  JSONL file is written by both implementations concurrently. Enforcing that
  premise belongs to the migrate consumer (0172), which must cut a given
  session-log file over from the bash primitives to this port atomically — never
  interleaving a bash writer and a Rust writer against the same file. This
  obligation is propagated onto 0172 via the reciprocal `relates_to` edge and a
  note in 0172's Dependencies. If 0172 cannot guarantee the atomic cutover, the
  byte-parity scope-out (and AC-6/AC-7) must be revisited.

## Assumptions

- Folding the atomic-store into `corpus-adapters` (vs. a standalone `store`
  crate) is acceptable for now; the capability is still fully delivered.

## Technical Notes

**Size**: M — a faithful port of ~230 lines of subtle concurrency bash (three
primitives: atomic write, mkdir-lock with PID reclaim, canonical JSONL) behind a
driven-port trait with faked-port tests. No twin extraction and no greenfield
UI, so smaller than 0179; the concurrency subtlety (same-dir-temp atomicity, the
serde_json escaper parity trap, PID-reclaim) keeps it at M rather than S.

- Source bash: `scripts/atomic-common.sh:105-247` — `atomic_write` (`:16-32`),
  `_atomic_lock_acquire` (`:105-143`, RANDOM re-seed + jittered back-off +
  300 s ceiling), `_atomic_lock_reclaim_if_stale` (`:157-163`, `kill -0` owner
  check), the subshell-owned EXIT trap for release (`:200-209`, `:233-246`),
  and the anchored-prefix `index($0,p)!=1` remove (`:243-244`, prefix passed via
  `ENVIRON`).
- Source bash: `scripts/jsonl-common.sh:66-149` — `jsonl_json_escape`
  (backslash-first escape ordering) and `jsonl_compose_record` (canonical field
  order, `transformation_key` first, `schema_version` second).
- Same-directory temp invariant (the easiest thing to break): `mktemp` creates
  the temp *in the target's dir* (`atomic-common.sh:26`) because `rename(2)` is
  atomic only within one filesystem. In Rust use `NamedTempFile::new_in(parent)`
  — the default `new()` uses `$TMPDIR` and silently degrades `mv` to a
  non-atomic cross-mount copy.
- `serde_json` escaper mismatch risk: `serde_json`'s string serialiser is not
  guaranteed byte-identical to the hand-rolled `jsonl_json_escape`
  (`jsonl-common.sh:21-50`, backslash-first ordering, control chars as
  `\u00XX`). Records written one way and prefix-matched another will silently
  fail to match — port one escaper and drive **both** compose and remove-by-key
  through it. This is the single most important parity requirement.
- Anchored-prefix, not substring: `atomic_jsonl_remove_by_key` drops lines where
  the `{"transformation_key":"…",` prefix starts at byte 0
  (`atomic-common.sh:243-244`, awk `index($0,p)!=1`). Rust: retain where
  `!line.starts_with(&prefix)`.
- Back-off shape to preserve exactly: writability pre-check early-error
  (`:110-115`); base 4 ms doubling to a ~200–256 ms cap; jitter uniform
  `1..=base`; 300 s acquisition ceiling (`:129`). `rm -rf` (not `rmdir`) on
  release/reclaim because the lockdir carries the `owner` sentinel (written
  `:141`, read `:159`; `rm -rf` at `:148`/`:162`).
- PID-reclaim window: `_atomic_lock_reclaim_if_stale` (`:157-163`) treats a
  missing/empty `owner` file as **live** (holder mid-acquisition, between the
  `mkdir` and the owner write) — preserve that window. `kill -0` →
  `libc::kill(pid,0)`; PID-reuse only degrades to spin-wait, never breaks a live
  lock.
- Port-trait design (Model 1 — each sub-binary wires its own adapters at its own
  composition root, per ADR-0053/0166): define an `AtomicStore`/`RecordStore`
  driven-port
  trait in the `corpus` domain (mirroring `launch/core.rs:173-189`
  `ResolveBinary`/`ExecBinary`), implement it in `corpus-adapters`, and map a
  rich `StoreError` (lock-timeout, not-writable, io) into `kernel::Error::Failed`
  via `From` (precedent `launch/core.rs:167-171`; kernel taxonomy
  `kernel/src/lib.rs:9-15`). Faked-port isolation tests mirror
  `FixedResolver`/`RecordingExec` (ADR-0053).
- Dependency decision (resolved — see Open Questions): `atomic_write` uses
  `tempfile::NamedTempFile::new_in` (new direct dep) for robust unique temp
  naming and RAII cleanup, mirroring `server/src/file_driver.rs`. The mkdir-lock
  uses `std::fs::create_dir` + `libc::kill` for liveness regardless (mkdir *is*
  the mutex). `fd-lock`/`fs2` remain rejected — they reintroduce the `flock` the
  bash deliberately avoids and drop PID-reclaim. Both `tempfile` and the direct
  `libc` dep are subject to the cargo-deny ban-lists 0162/0166 activate, so
  landing this port requires `cli/deny.toml`'s allow-list to admit them (`rand`
  0.9 is already present; `libc` today is transitive only, via rustls/ring).
- Parity surface: `atomic_write` has many callers (config-common, the migration
  engine, `work-item-sync-*`, and the `jira_atomic_write_json` /
  `linear_atomic_write_json` validate-then-write wrappers). The JSONL primitives
  have one production caller (the migrate session log, `interactive-lib.sh`), but
  `jsonl_compose_record`'s canonical field order is the JSONL contract the
  visualiser reads. Existing Rust atomic-write prior art:
  `server/src/file_driver.rs`.
- Bash-isms that fall away (do not port literally): the `$BASHPID`-absent
  spin-only fallback (`std::process::id()` always exists), the RANDOM lockstep
  re-seed, the `ENVIRON`-vs-`awk -v` backslash hazard, and the EXIT-trap
  string-building (→ RAII drop guard, which also gives the subshell-trap
  isolation for free). Validation nuance: `proposed_value` is documented required
  (`jsonl-common.sh:60`) but omitted from the emptiness check (`:115-116`) — port
  to the documented spec: treat `proposed_value` as required and reject empty
  values.
- In-workspace sync atomic-write precedents (closer than the async server
  driver): `config-adapters/src/store.rs:58-80` writes a temp under
  `.accelerator/tmp/` named `config-{pid}-{counter}.tmp` then `fs::write` +
  `fs::rename`, cleaning the temp on failure — no `tempfile` crate, no fsync,
  no `EXDEV` branch; `launcher/src/launch/outbound/resolve/cache.rs:112-127`
  (`write_then_rename`) is the same temp-in-dir-then-rename shape with an
  `ETXTBSY`-avoidance note. Both illustrate the temp-in-target-dir + rename shape
  to preserve; the chosen `tempfile::NamedTempFile::new_in` path must keep that
  same-dir invariant (its default `new()` would use `$TMPDIR` and break
  atomicity).
- First fallible corpus port: the `corpus` domain crate is dependency-free
  (`cli/corpus/Cargo.toml` empty `[dependencies]`) and its existing ports —
  `Clock` (`corpus/src/metadata.rs:13-16`) and `IdScanner`
  (`corpus/src/work_item_id.rs:15-17`) — are both infallible. A store port
  returning `Result<_, StoreError>` is the first fallible port here, so where
  `StoreError` lives (kernel-free `corpus` domain vs. `corpus-adapters`) is a
  live decision. Closest structural analog: the `vcs` pair declares
  `RepoRoot`/`VcsProbe` in `cli/vcs/src/lib.rs:41-55` with a `facts(...)`
  composer over `&dyn` ports.
- Persistence seam already exists: `corpus-adapters/src/patcher.rs:48-94`
  (`patch_status`) and `assemble.rs:31-60` produce the exact new bytes to
  persist as pure `Result<Vec<u8>, _>` transforms with no I/O — the
  `atomic_write` port is the missing persistence half, wired downstream of
  them.
- Sync, not async: the whole `cli/` workspace is synchronous `std::fs`, so
  port the bash semantics synchronously. `server/src/file_driver.rs` is
  async/tokio and layers a per-path `tokio::Mutex` + SHA-256 etag optimistic
  concurrency (not `flock`) — that layer is out of scope for this parity
  port; take only its same-dir-temp + rename + `EXDEV`-fail-closed shape.
- Faked-port tests live corpus-local: `FakeClock`
  (`corpus-adapters/tests/metadata.rs`) and inline `DigitRunScanner`
  (`corpus/src/work_item_id.rs:141-153`) are the nearest doubles; the richer
  `FixedResolver`/`RecordingExec` examples are in `launcher` `core.rs:261-301`.
  Add the store round-trip and dead-holder-reclaim tests to the existing
  `corpus-adapters/tests/` suite (`metadata.rs`, `doc_type_single_source.rs`,
  `parity.rs`, `common/mod.rs`), which already has a `bash-parity` feature
  gate for differential suites.

## Drafting Notes

- Verified every source-bash and Rust line reference in Technical Notes against
  the current tree during this review (2026-07-18); all confirmed. Corrected one
  imprecise citation (the owner-sentinel lines: `owner` is written at `:141` and
  read at `:159`, not the `rm -rf` line `:162`).
- 0179 is `done` and created `corpus-adapters`; 0166 locked "fold into
  `corpus-adapters`, no standalone `store` crate." Recorded the
  `blocked_by: 0179` edge in frontmatter accordingly (0179 already asserts the
  matching `blocks: 0180`).
- The atomic-write dependency recommendation diverges from existing repo prior
  art (`file_driver.rs` uses `tempfile`); surfaced as an Open Question rather
  than silently endorsing either path.
- Review 1 (2026-07-18) resolved both Open Questions: **clean cutover** on the
  JSONL escaper (a self-consistent Rust round-trip is the full parity
  requirement, no `bash-parity` differential suite) and **`tempfile` crate** for
  `atomic_write`, following the `file_driver.rs` prior art. Added acceptance
  criteria for the lock acquisition ceiling / not-writable early error and a
  fault-injection seam for the interruption invariant; reframed the umbrella
  criterion as AC-1 and demoted the round-trip criterion's "single shared escaper"
  clause to Technical-Notes guidance; recorded the downstream JSONL/atomic-store
  consumers and the `tempfile`/`libc` cargo-deny coupling in Dependencies; glossed
  "Model 1" and made the `proposed_value` validation intent concrete.
- Review 1 pass 2 (2026-07-18) added acceptance criteria for the canonical
  field-order golden assertion (AC-7), the empty/absent-`owner` reclaim window
  (AC-4), and the `proposed_value`-required rejection (AC-8); numbered the
  acceptance criteria explicitly (AC-1…AC-8) and fixed the stale round-trip
  references; recorded the clean-cutover constraint on 0172 in Dependencies; and
  noted the back-off shape as ungated design intent.
- Review 1 pass 3 (2026-07-18) propagated the clean-cutover obligation onto 0172
  (reciprocal `relates_to` edge + a note in 0172's Dependencies), added the
  `proposed_value`-required validation as a Requirements bullet, enumerated AC-7's
  full canonical field order against `jsonl-common.sh:129-148`, and extended AC-8
  to the absent-key case. Verdict moved REVISE → REVISE → COMMENT across the three
  passes.

## References

- Parent: `meta/work/0166-shared-config-corpus-store-crates.md`
- ADRs: ADR-0045, ADR-0053
